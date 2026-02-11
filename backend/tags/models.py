"""Tag data models."""

from enum import StrEnum

from pydantic import BaseModel


class TagType(StrEnum):
    """Type of NFC tag."""

    SPOOLEASE_V1 = "SpoolEaseV1"
    SPOOLEASE_V2 = "SpoolEaseV2"
    BAMBULAB = "Bambu Lab"
    OPENPRINTTAG = "OpenPrintTag"
    OPENSPOOL = "OpenSpool"
    OPENTAG3D = "OpenTag3D"
    UNKNOWN = "Unknown"


class NfcTagType(StrEnum):
    """Physical NFC tag type."""

    NTAG = "NTAG"  # NTAG213/215/216
    MIFARE_CLASSIC_1K = "MifareClassic1K"
    MIFARE_CLASSIC_4K = "MifareClassic4K"
    UNKNOWN = "Unknown"


class SpoolEaseTagData(BaseModel):
    """Parsed data from a SpoolEase tag (V1 or V2)."""

    version: int = 2  # 1 or 2
    tag_id: str  # Base64-encoded UID
    spool_id: str | None = None
    material: str | None = None
    material_subtype: str | None = None
    color_code: str | None = None  # RGBA hex (e.g., "FF0000FF")
    color_name: str | None = None
    brand: str | None = None
    weight_label: int | None = None  # Advertised weight in grams
    weight_core: int | None = None  # Empty spool weight
    weight_new: int | None = None  # Actual weight when full
    slicer_filament_code: str | None = None  # e.g., "GFL99"
    slicer_filament_name: str | None = None
    note: str | None = None
    encode_time: int | None = None  # Unix timestamp
    added_time: int | None = None  # Unix timestamp


class BambuLabTagData(BaseModel):
    """Parsed data from a Bambu Lab RFID tag."""

    tag_id: str  # Hex-encoded UID
    material_variant_id: str | None = None  # e.g., "A00-G1"
    material_id: str | None = None  # e.g., "GFA00"
    filament_type: str | None = None  # e.g., "PLA"
    detailed_filament_type: str | None = None  # e.g., "PLA Basic"
    color_rgba: str | None = None  # e.g., "FF0000FF"
    color_rgba2: str | None = None  # Secondary color for multi-color
    spool_weight: int | None = None  # Empty spool weight in grams
    # Raw block data for reference
    blocks: dict[int, bytes] | None = None


class OpenPrintTagData(BaseModel):
    """Parsed data from an OpenPrintTag."""

    tag_id: str  # Base64-encoded UID
    material_name: str | None = None
    material_type: str | None = None  # e.g., "PLA", "PETG"
    brand_name: str | None = None
    primary_color: str | None = None  # RGBA hex
    secondary_colors: list[str] | None = None
    nominal_weight: int | None = None  # Advertised weight
    actual_weight: int | None = None  # Real weight when full
    empty_weight: int | None = None  # Empty spool weight


class OpenSpoolTagData(BaseModel):
    """Parsed data from an OpenSpool tag.

    OpenSpool uses JSON in NDEF MIME records (application/json).
    Format: {"protocol": "openspool", "version": "1.0", "type": "PLA", ...}
    """

    tag_id: str  # Base64-encoded UID
    version: str = "1.0"  # Protocol version
    material_type: str | None = None  # e.g., "PLA", "PETG"
    color_hex: str | None = None  # RGB hex without alpha (e.g., "FFAABB")
    brand: str | None = None
    min_temp: int | None = None  # Minimum print temperature
    max_temp: int | None = None  # Maximum print temperature


class TagReadResult(BaseModel):
    """Result of reading an NFC tag."""

    uid: str  # Hex-encoded UID
    uid_base64: str  # Base64-encoded UID (for SpoolEase compatibility)
    nfc_type: NfcTagType
    tag_type: TagType

    # Parsed data (one of these will be set based on tag_type)
    spoolease_data: SpoolEaseTagData | None = None
    bambulab_data: BambuLabTagData | None = None
    openprinttag_data: OpenPrintTagData | None = None
    openspool_data: OpenSpoolTagData | None = None
    opentag3d_data: dict | None = None  # Uses OpenTag3DTagData from opentag3d module

    # Raw data
    ndef_message: bytes | None = None  # Raw NDEF for NTAG
    mifare_blocks: dict[int, bytes] | None = None  # Raw blocks for Mifare

    # For matching to existing spools
    matched_spool_id: str | None = None


class SpoolFromTag(BaseModel):
    """Spool data extracted from any tag type, normalized for database."""

    tag_id: str
    tag_type: str
    material: str | None = None
    subtype: str | None = None
    color_name: str | None = None
    rgba: str | None = None
    brand: str | None = None
    label_weight: int | None = None
    core_weight: int | None = None
    weight_new: int | None = None
    slicer_filament: str | None = None
    note: str | None = None
    data_origin: str | None = None
