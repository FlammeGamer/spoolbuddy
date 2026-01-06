from fastapi import APIRouter, HTTPException
from fastapi.responses import Response
from pydantic import BaseModel
import asyncio
import io
import logging
import re
import tempfile
import zipfile
from pathlib import Path
from PIL import Image

from db import get_db
from services.bambu_ftp import download_file_try_paths_async
from models import (
    Printer,
    PrinterCreate,
    PrinterUpdate,
    PrinterWithStatus,
    AmsFilamentSettingRequest,
    AssignSpoolRequest,
    SetCalibrationRequest,
)

logger = logging.getLogger(__name__)
router = APIRouter(prefix="/printers", tags=["printers"])

# Reference to printer manager (set by main.py)
_printer_manager = None

# Cache for cover images: {serial: {(subtask_name, plate_num): bytes}}
_cover_cache: dict[str, dict[tuple[str, int], bytes]] = {}

# Cover image size for ESP32 display (must match EEZ design: 70x70)
COVER_SIZE = (70, 70)


def resize_cover_image(image_data: bytes) -> bytes:
    """Resize a PNG image to COVER_SIZE and convert to raw RGB565 for ESP32 display.

    Args:
        image_data: Original PNG bytes

    Returns:
        Raw RGB565 pixel data (no header, just pixels)
    """
    img = Image.open(io.BytesIO(image_data))
    # Convert to RGB (no alpha needed for RGB565)
    if img.mode != "RGB":
        img = img.convert("RGB")
    img = img.resize(COVER_SIZE, Image.LANCZOS)

    # Convert to RGB565 (16-bit: 5 bits red, 6 bits green, 5 bits blue)
    pixels = img.load()
    width, height = img.size
    rgb565_data = bytearray(width * height * 2)

    idx = 0
    for y in range(height):
        for x in range(width):
            r, g, b = pixels[x, y]
            # Convert 8-bit RGB to RGB565
            r5 = (r >> 3) & 0x1F
            g6 = (g >> 2) & 0x3F
            b5 = (b >> 3) & 0x1F
            rgb565 = (r5 << 11) | (g6 << 5) | b5
            # Little-endian for ESP32
            rgb565_data[idx] = rgb565 & 0xFF
            rgb565_data[idx + 1] = (rgb565 >> 8) & 0xFF
            idx += 2

    return bytes(rgb565_data)


def set_printer_manager(manager):
    """Set the printer manager reference."""
    global _printer_manager
    _printer_manager = manager


@router.get("", response_model=list[PrinterWithStatus])
async def list_printers():
    """Get all printers with connection status and live state."""
    db = await get_db()
    printers = await db.get_printers()

    # Get connection statuses
    statuses = _printer_manager.get_connection_statuses() if _printer_manager else {}

    result = []
    for printer in printers:
        connected = statuses.get(printer.serial, False)
        gcode_state = None
        print_progress = None
        subtask_name = None
        mc_remaining_time = None
        cover_url = None
        ams_units = []
        tray_now = None
        tray_now_left = None
        tray_now_right = None
        active_extruder = None
        stg_cur = -1
        stg_cur_name = None

        # Get live state if connected
        if connected and _printer_manager:
            state = _printer_manager.get_state(printer.serial)
            if state:
                gcode_state = state.gcode_state
                print_progress = state.print_progress
                subtask_name = state.subtask_name
                mc_remaining_time = state.mc_remaining_time
                ams_units = state.ams_units
                tray_now = state.tray_now
                tray_now_left = state.tray_now_left
                tray_now_right = state.tray_now_right
                active_extruder = state.active_extruder
                stg_cur = state.stg_cur
                stg_cur_name = state.stg_cur_name
                # Add cover URL if printing
                if gcode_state in ("RUNNING", "PAUSE", "PAUSED") and subtask_name:
                    cover_url = f"/api/printers/{printer.serial}/cover"

        result.append(PrinterWithStatus(
            **printer.model_dump(),
            connected=connected,
            gcode_state=gcode_state,
            print_progress=print_progress,
            subtask_name=subtask_name,
            mc_remaining_time=mc_remaining_time,
            cover_url=cover_url,
            ams_units=ams_units,
            tray_now=tray_now,
            tray_now_left=tray_now_left,
            tray_now_right=tray_now_right,
            active_extruder=active_extruder,
            stg_cur=stg_cur,
            stg_cur_name=stg_cur_name,
        ))

    return result


@router.get("/{serial}", response_model=PrinterWithStatus)
async def get_printer(serial: str):
    """Get a single printer."""
    db = await get_db()
    printer = await db.get_printer(serial)
    if not printer:
        raise HTTPException(status_code=404, detail="Printer not found")

    connected = _printer_manager.is_connected(serial) if _printer_manager else False
    return PrinterWithStatus(**printer.model_dump(), connected=connected)


@router.post("", response_model=Printer, status_code=201)
async def create_printer(printer: PrinterCreate):
    """Create or update a printer."""
    db = await get_db()
    return await db.create_printer(printer)


@router.put("/{serial}", response_model=Printer)
async def update_printer(serial: str, printer: PrinterUpdate):
    """Update an existing printer."""
    db = await get_db()
    updated = await db.update_printer(serial, printer)
    if not updated:
        raise HTTPException(status_code=404, detail="Printer not found")
    return updated


@router.delete("/{serial}", status_code=204)
async def delete_printer(serial: str):
    """Delete a printer."""
    # Disconnect first
    if _printer_manager:
        await _printer_manager.disconnect(serial)

    db = await get_db()
    if not await db.delete_printer(serial):
        raise HTTPException(status_code=404, detail="Printer not found")


@router.post("/{serial}/connect", status_code=204)
async def connect_printer(serial: str):
    """Connect to a printer."""
    if not _printer_manager:
        raise HTTPException(status_code=500, detail="Printer manager not available")

    db = await get_db()
    printer = await db.get_printer(serial)
    if not printer:
        raise HTTPException(status_code=404, detail="Printer not found")

    if not printer.ip_address or not printer.access_code:
        raise HTTPException(status_code=400, detail="Printer missing IP address or access code")

    try:
        await _printer_manager.connect(
            serial=printer.serial,
            ip_address=printer.ip_address,
            access_code=printer.access_code,
            name=printer.name,
        )
    except Exception as e:
        logger.error(f"Failed to connect to {serial}: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.post("/{serial}/disconnect", status_code=204)
async def disconnect_printer(serial: str):
    """Disconnect from a printer."""
    if _printer_manager:
        await _printer_manager.disconnect(serial)


class AutoConnectRequest(BaseModel):
    auto_connect: bool


@router.post("/{serial}/auto-connect", response_model=Printer)
async def set_auto_connect(serial: str, request: AutoConnectRequest):
    """Set auto-connect setting."""
    db = await get_db()
    printer = await db.get_printer(serial)
    if not printer:
        raise HTTPException(status_code=404, detail="Printer not found")

    return await db.update_printer(serial, PrinterUpdate(auto_connect=request.auto_connect))


@router.post("/{serial}/ams/{ams_id}/tray/{tray_id}/filament", status_code=204)
async def set_filament(serial: str, ams_id: int, tray_id: int, filament: AmsFilamentSettingRequest):
    """Set filament information for an AMS slot.

    Args:
        serial: Printer serial number
        ams_id: AMS unit ID (0-3 for regular AMS, 128-135 for AMS-HT, 254/255 for external)
        tray_id: Tray ID within AMS (0-3 for regular AMS, 0 for HT/external)
        filament: Filament settings
    """
    if not _printer_manager:
        raise HTTPException(status_code=500, detail="Printer manager not available")

    if not _printer_manager.is_connected(serial):
        raise HTTPException(status_code=400, detail="Printer not connected")

    success = _printer_manager.set_filament(
        serial=serial,
        ams_id=ams_id,
        tray_id=tray_id,
        tray_info_idx=filament.tray_info_idx,
        tray_type=filament.tray_type,
        tray_color=filament.tray_color,
        nozzle_temp_min=filament.nozzle_temp_min,
        nozzle_temp_max=filament.nozzle_temp_max,
    )

    if not success:
        raise HTTPException(status_code=500, detail="Failed to set filament")


@router.post("/{serial}/ams/{ams_id}/tray/{tray_id}/assign", status_code=204)
async def assign_spool_to_tray(serial: str, ams_id: int, tray_id: int, request: AssignSpoolRequest):
    """Assign a spool from inventory to an AMS slot.

    Looks up the spool data, sends filament settings to the printer,
    and persists the assignment for usage tracking.

    Args:
        serial: Printer serial number
        ams_id: AMS unit ID (0-3 for regular AMS, 128-135 for AMS-HT, 254/255 for external)
        tray_id: Tray ID within AMS (0-3 for regular AMS, 0 for HT/external)
        request: Assignment request with spool_id
    """
    if not _printer_manager:
        raise HTTPException(status_code=500, detail="Printer manager not available")

    if not _printer_manager.is_connected(serial):
        raise HTTPException(status_code=400, detail="Printer not connected")

    # Look up spool from database
    db = await get_db()
    spool = await db.get_spool(request.spool_id)
    if not spool:
        raise HTTPException(status_code=404, detail="Spool not found")

    # Convert spool color (RGBA format) - ensure it's 8 hex chars
    tray_color = spool.rgba or "FFFFFFFF"
    if len(tray_color) == 6:
        tray_color = tray_color + "FF"  # Add alpha if missing

    # Determine temperature range based on material
    temp_ranges = {
        "PLA": (190, 230),
        "PETG": (220, 260),
        "ABS": (240, 270),
        "ASA": (240, 270),
        "TPU": (200, 230),
        "PA": (260, 290),
        "PC": (260, 280),
        "PVA": (190, 210),
    }
    material = (spool.material or "").upper()
    temp_min, temp_max = temp_ranges.get(material, (190, 250))

    # Build tray_info_idx - this is typically a manufacturer preset ID
    # For generic filaments, we use empty string
    tray_info_idx = ""

    success = _printer_manager.set_filament(
        serial=serial,
        ams_id=ams_id,
        tray_id=tray_id,
        tray_info_idx=tray_info_idx,
        tray_type=spool.material or "",
        tray_color=tray_color,
        nozzle_temp_min=temp_min,
        nozzle_temp_max=temp_max,
    )

    if not success:
        raise HTTPException(status_code=500, detail="Failed to assign spool")

    # Persist assignment for usage tracking
    await db.assign_spool_to_slot(request.spool_id, serial, ams_id, tray_id)

    logger.info(f"Assigned spool {spool.id} ({spool.material}) to {serial} AMS {ams_id} tray {tray_id}")


@router.delete("/{serial}/ams/{ams_id}/tray/{tray_id}/assign", status_code=204)
async def unassign_spool_from_tray(serial: str, ams_id: int, tray_id: int):
    """Remove spool assignment from an AMS slot.

    This only removes the tracking assignment, not the filament setting on the printer.
    """
    db = await get_db()
    await db.unassign_slot(serial, ams_id, tray_id)
    logger.info(f"Unassigned spool from {serial} AMS {ams_id} tray {tray_id}")


@router.get("/{serial}/assignments")
async def get_slot_assignments(serial: str):
    """Get all spool-to-slot assignments for a printer.

    Returns list of assignments with spool info.
    """
    db = await get_db()
    return await db.get_slot_assignments(serial)


@router.post("/{serial}/ams/{ams_id}/tray/{tray_id}/reset", status_code=204)
async def reset_slot(serial: str, ams_id: int, tray_id: int):
    """Reset/clear an AMS slot to trigger RFID re-read.

    Clears the slot filament settings, causing the printer to re-read
    the RFID tag on the next operation.

    Args:
        serial: Printer serial number
        ams_id: AMS unit ID (0-3 for regular AMS, 128-135 for AMS-HT, 254/255 for external)
        tray_id: Tray ID within AMS (0-3 for regular AMS, 0 for HT/external)
    """
    if not _printer_manager:
        raise HTTPException(status_code=500, detail="Printer manager not available")

    if not _printer_manager.is_connected(serial):
        raise HTTPException(status_code=400, detail="Printer not connected")

    success = _printer_manager.reset_slot(serial=serial, ams_id=ams_id, tray_id=tray_id)

    if not success:
        raise HTTPException(status_code=500, detail="Failed to reset slot")


@router.post("/{serial}/ams/{ams_id}/tray/{tray_id}/calibration", status_code=204)
async def set_calibration(serial: str, ams_id: int, tray_id: int, request: SetCalibrationRequest):
    """Set calibration profile (k-value) for an AMS slot.

    Args:
        serial: Printer serial number
        ams_id: AMS unit ID
        tray_id: Tray ID within AMS
        request: Calibration settings (cali_idx, filament_id, nozzle_diameter)
    """
    if not _printer_manager:
        raise HTTPException(status_code=500, detail="Printer manager not available")

    if not _printer_manager.is_connected(serial):
        raise HTTPException(status_code=400, detail="Printer not connected")

    success = _printer_manager.set_calibration(
        serial=serial,
        ams_id=ams_id,
        tray_id=tray_id,
        cali_idx=request.cali_idx,
        filament_id=request.filament_id,
        nozzle_diameter=request.nozzle_diameter,
    )

    if not success:
        raise HTTPException(status_code=500, detail="Failed to set calibration")


@router.get("/{serial}/calibrations")
async def get_calibrations(serial: str, nozzle_diameter: str = "0.4"):
    """Get available calibration profiles (K-profiles) for a printer.

    Returns list of calibration profiles with cali_idx, name, k_value, filament_id.
    Uses async request with retry logic to reliably fetch profiles from printer.
    """
    import logging
    logger = logging.getLogger(__name__)

    if not _printer_manager:
        raise HTTPException(status_code=500, detail="Printer manager not available")

    if not _printer_manager.is_connected(serial):
        logger.warning(f"[API] get_calibrations: printer {serial} not connected")
        raise HTTPException(status_code=400, detail="Printer not connected")

    # Use async method with retry logic
    calibrations = await _printer_manager.get_kprofiles(serial, nozzle_diameter)
    logger.info(f"[API] get_calibrations({serial}): returning {len(calibrations)} K-profiles")
    return calibrations


@router.get("/{serial}/cover")
async def get_printer_cover(serial: str):
    """Get the cover image for the current print job.

    Downloads the 3MF file from the printer via FTP and extracts the thumbnail.
    Results are cached per print job.
    """
    db = await get_db()
    printer = await db.get_printer(serial)
    if not printer:
        raise HTTPException(status_code=404, detail="Printer not found")

    if not _printer_manager:
        raise HTTPException(status_code=500, detail="Printer manager not available")

    if not _printer_manager.is_connected(serial):
        raise HTTPException(status_code=400, detail="Printer not connected")

    state = _printer_manager.get_state(serial)
    if not state:
        raise HTTPException(status_code=404, detail="Printer state not available")

    # Get subtask_name (the 3MF filename)
    subtask_name = state.subtask_name
    if not subtask_name:
        raise HTTPException(status_code=404, detail="No active print job")

    # Extract plate number from gcode_file (e.g., "plate_1.gcode" -> 1)
    plate_num = 1
    gcode_file = getattr(state, "gcode_file", None)
    if gcode_file:
        match = re.search(r"plate_(\d+)\.gcode", gcode_file)
        if match:
            plate_num = int(match.group(1))

    # Check cache
    cache_key = (subtask_name, plate_num)
    if serial in _cover_cache and cache_key in _cover_cache[serial]:
        return Response(content=_cover_cache[serial][cache_key], media_type="application/octet-stream")

    # Build 3MF filename
    filename = subtask_name
    if not filename.endswith(".3mf"):
        filename = filename + ".gcode.3mf"

    # Possible paths on printer
    remote_paths = [
        f"/{filename}",
        f"/cache/{filename}",
        f"/model/{filename}",
        f"/data/{filename}",
    ]

    logger.info(f"Downloading cover for '{filename}' from {printer.ip_address}")

    # Download 3MF file
    data = await download_file_try_paths_async(
        printer.ip_address,
        printer.access_code,
        remote_paths,
        timeout=30.0,
    )

    if not data:
        raise HTTPException(
            status_code=404,
            detail=f"Could not download 3MF file '{filename}' from printer"
        )

    # Extract thumbnail from 3MF (ZIP file)
    try:
        zf = zipfile.ZipFile(io.BytesIO(data), "r")
    except zipfile.BadZipFile:
        raise HTTPException(status_code=500, detail="Downloaded file is not a valid 3MF/ZIP")

    try:
        # Try common thumbnail paths
        thumbnail_paths = [
            f"Metadata/plate_{plate_num}.png",
            "Metadata/plate_1.png",
            "Metadata/thumbnail.png",
            f"Metadata/plate_{plate_num}_small.png",
            "Metadata/plate_1_small.png",
            "Thumbnails/thumbnail.png",
        ]

        for thumb_path in thumbnail_paths:
            try:
                image_data = zf.read(thumb_path)
                # Convert to raw RGB565 for ESP32 display
                rgb565_data = resize_cover_image(image_data)
                logger.info(f"Converted cover to RGB565: {len(rgb565_data)} bytes")
                # Cache result
                if serial not in _cover_cache:
                    _cover_cache[serial] = {}
                _cover_cache[serial][cache_key] = rgb565_data
                return Response(content=rgb565_data, media_type="application/octet-stream")
            except KeyError:
                continue

        # Try any PNG in Metadata folder
        for name in zf.namelist():
            if name.startswith("Metadata/") and name.endswith(".png"):
                image_data = zf.read(name)
                # Convert to raw RGB565 for ESP32 display
                rgb565_data = resize_cover_image(image_data)
                logger.info(f"Converted cover to RGB565: {len(rgb565_data)} bytes")
                if serial not in _cover_cache:
                    _cover_cache[serial] = {}
                _cover_cache[serial][cache_key] = rgb565_data
                return Response(content=rgb565_data, media_type="application/octet-stream")

        raise HTTPException(status_code=404, detail="No thumbnail found in 3MF file")
    finally:
        zf.close()
