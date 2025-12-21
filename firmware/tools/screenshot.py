#!/usr/bin/env python3
"""
SpoolBuddy Screenshot Capture Tool

Captures screenshots from the ESP32 display via serial output.
Triple-tap the top-left corner of the display to trigger a screenshot.

Usage:
    python screenshot.py /dev/ttyUSB0
    python screenshot.py /dev/ttyUSB0 --output screenshots/
    python screenshot.py /dev/ttyUSB0 --watch  # Continuous mode

Requirements:
    pip install pyserial pillow
"""

import argparse
import re
import sys
import time
from datetime import datetime
from pathlib import Path

try:
    import serial
except ImportError:
    print("Error: pyserial not installed. Run: pip install pyserial")
    sys.exit(1)

try:
    from PIL import Image
except ImportError:
    print("Error: Pillow not installed. Run: pip install pillow")
    sys.exit(1)


def rgb565_to_rgb888(rgb565: int) -> tuple:
    """Convert RGB565 (16-bit) to RGB888 (24-bit) tuple."""
    r = ((rgb565 >> 11) & 0x1F) << 3
    g = ((rgb565 >> 5) & 0x3F) << 2
    b = (rgb565 & 0x1F) << 3
    # Expand to full range
    r = r | (r >> 5)
    g = g | (g >> 6)
    b = b | (b >> 5)
    return (r, g, b)


def parse_screenshot_line(line: str) -> tuple:
    """Parse a screenshot row line. Returns (row_num, pixel_data) or (None, None)."""
    # Format: R000:AABBCCDD... (4 hex chars per pixel)
    match = re.match(r'R(\d{3}):([0-9A-Fa-f]+)', line)
    if match:
        row_num = int(match.group(1))
        hex_data = match.group(2)
        return row_num, hex_data
    return None, None


def capture_screenshot(ser, timeout=30) -> dict:
    """
    Capture a screenshot from serial output.
    Returns dict with 'width', 'height', 'format', 'rows' or None.
    """
    print("Waiting for screenshot data...")
    print("(Triple-tap top-left corner of display to trigger)")

    start_time = time.time()
    in_screenshot = False
    width = 0
    height = 0
    fmt = ""
    rows = {}

    while time.time() - start_time < timeout:
        try:
            line = ser.readline().decode('utf-8', errors='ignore').strip()
        except Exception as e:
            continue

        if not line:
            continue

        # Check for screenshot markers
        if 'SCREENSHOT_BEGIN' in line:
            # Parse header: SCREENSHOT_BEGIN:800x480:RGB565
            match = re.search(r'SCREENSHOT_BEGIN:(\d+)x(\d+):(\w+)', line)
            if match:
                width = int(match.group(1))
                height = int(match.group(2))
                fmt = match.group(3)
                in_screenshot = True
                rows = {}
                print(f"Receiving screenshot: {width}x{height} {fmt}")
            continue

        if 'SCREENSHOT_END' in line:
            if in_screenshot:
                print(f"Screenshot complete: {len(rows)} rows received")
                return {
                    'width': width,
                    'height': height,
                    'format': fmt,
                    'rows': rows
                }
            in_screenshot = False
            continue

        if 'SCREENSHOT_ERROR' in line:
            print(f"Screenshot error: {line}")
            in_screenshot = False
            continue

        # Parse row data
        if in_screenshot:
            row_num, hex_data = parse_screenshot_line(line)
            if row_num is not None:
                rows[row_num] = hex_data
                # Progress indicator
                if row_num % 50 == 0:
                    print(f"  Row {row_num}/{height}...")
        else:
            # Print other serial output
            print(f"[serial] {line}")

    print("Screenshot capture timed out")
    return None


def screenshot_to_image(screenshot: dict) -> Image.Image:
    """Convert screenshot data to PIL Image."""
    width = screenshot['width']
    height = screenshot['height']
    rows = screenshot['rows']

    img = Image.new('RGB', (width, height), color=(0, 0, 0))
    pixels = img.load()

    for y in range(height):
        if y not in rows:
            print(f"Warning: Missing row {y}")
            continue

        hex_data = rows[y]
        # Each pixel is 4 hex chars (2 bytes RGB565)
        for x in range(width):
            offset = x * 4
            if offset + 4 <= len(hex_data):
                rgb565 = int(hex_data[offset:offset+4], 16)
                pixels[x, y] = rgb565_to_rgb888(rgb565)

    return img


def main():
    parser = argparse.ArgumentParser(
        description='Capture screenshots from SpoolBuddy display via serial'
    )
    parser.add_argument('port', help='Serial port (e.g., /dev/ttyUSB0, COM3)')
    parser.add_argument('--baud', type=int, default=921600, help='Baud rate (default: 921600)')
    parser.add_argument('--output', '-o', default='.', help='Output directory for screenshots')
    parser.add_argument('--watch', '-w', action='store_true', help='Watch mode: capture continuously')
    parser.add_argument('--timeout', '-t', type=int, default=60, help='Timeout in seconds (default: 60)')

    args = parser.parse_args()

    output_dir = Path(args.output)
    output_dir.mkdir(parents=True, exist_ok=True)

    print(f"Opening serial port {args.port} at {args.baud} baud...")

    try:
        ser = serial.Serial(args.port, args.baud, timeout=1)
    except serial.SerialException as e:
        print(f"Error opening serial port: {e}")
        sys.exit(1)

    print("Connected! Triple-tap top-left corner of display to capture screenshot.")
    print("Press Ctrl+C to exit.\n")

    try:
        while True:
            screenshot = capture_screenshot(ser, timeout=args.timeout)

            if screenshot:
                img = screenshot_to_image(screenshot)

                # Generate filename with timestamp
                timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
                filename = output_dir / f'screenshot_{timestamp}.png'

                img.save(filename)
                print(f"Saved: {filename}")

                # Also save as 'latest.png' for easy access
                latest = output_dir / 'latest.png'
                img.save(latest)
                print(f"Saved: {latest}\n")

            if not args.watch:
                break

    except KeyboardInterrupt:
        print("\nExiting...")
    finally:
        ser.close()


if __name__ == '__main__':
    main()
