#!/bin/bash
#
# SpoolBuddy Firmware Release Script
#
# Builds the firmware, creates the OTA binary, and makes it available for updates.
#
# Usage:
#   ./release-firmware.sh [version]
#
# If version is provided, updates Cargo.toml before building.
# Otherwise, uses the current version from Cargo.toml.
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIRMWARE_DIR="$SCRIPT_DIR/firmware"
RELEASES_DIR="$FIRMWARE_DIR/releases"
CARGO_TOML="$FIRMWARE_DIR/Cargo.toml"
TARGET_DIR="$FIRMWARE_DIR/target/xtensa-esp32s3-espidf/release"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
echo_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
echo_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Convert shorthand version to Cargo semver format
# 0.1.0b2 -> 0.1.0-beta.2, 0.1.0a1 -> 0.1.0-alpha.1, 0.1.0rc1 -> 0.1.0-rc.1
to_cargo_version() {
    local v="$1"
    echo "$v" | sed -E 's/^([0-9]+\.[0-9]+\.[0-9]+)a([0-9]+)$/\1-alpha.\2/' \
              | sed -E 's/^([0-9]+\.[0-9]+\.[0-9]+)b([0-9]+)$/\1-beta.\2/' \
              | sed -E 's/^([0-9]+\.[0-9]+\.[0-9]+)rc([0-9]+)$/\1-rc.\2/'
}

# Convert Cargo semver to short format for filenames
# 0.1.0-beta.2 -> 0.1.0b2
to_short_version() {
    local v="$1"
    echo "$v" | sed -E 's/-alpha\./a/' \
              | sed -E 's/-beta\./b/' \
              | sed -E 's/-rc\./rc/'
}

# Get current version from Cargo.toml
get_version() {
    grep '^version = ' "$CARGO_TOML" | head -1 | sed 's/version = "\(.*\)"/\1/'
}

# Update version in Cargo.toml
set_version() {
    local new_version="$1"
    local cargo_version=$(to_cargo_version "$new_version")
    sed -i "s/^version = \".*\"/version = \"$cargo_version\"/" "$CARGO_TOML"
    echo_info "Updated Cargo.toml version to $cargo_version"
}

# Main
main() {
    cd "$FIRMWARE_DIR"

    # Handle version argument
    if [ -n "$1" ]; then
        set_version "$1"
    fi

    CARGO_VERSION=$(get_version)
    VERSION=$(to_short_version "$CARGO_VERSION")
    BINARY_NAME="spoolbuddy-$VERSION.bin"
    OUTPUT_PATH="$RELEASES_DIR/$BINARY_NAME"

    echo_info "Building firmware version $VERSION..."

    # Create releases directory if needed
    mkdir -p "$RELEASES_DIR"

    # Build firmware
    echo_info "Running cargo build --release..."
    cargo build --release --jobs 14

    if [ ! -f "$TARGET_DIR/spoolbuddy-firmware" ]; then
        echo_error "Build failed - ELF file not found"
        exit 1
    fi

    # Create binary image
    echo_info "Creating OTA binary..."
    espflash save-image --chip esp32s3 \
        "$TARGET_DIR/spoolbuddy-firmware" \
        "$OUTPUT_PATH"

    if [ ! -f "$OUTPUT_PATH" ]; then
        echo_error "Failed to create binary"
        exit 1
    fi

    # Get file size
    SIZE=$(du -h "$OUTPUT_PATH" | cut -f1)

    # Verify ESP32 magic byte
    MAGIC=$(head -c 1 "$OUTPUT_PATH" | od -An -tx1 | tr -d ' ')
    if [ "$MAGIC" != "e9" ]; then
        echo_error "Invalid ESP32 binary (magic byte: 0x$MAGIC, expected 0xe9)"
        exit 1
    fi

    # Delete old versions
    for old_file in "$RELEASES_DIR"/spoolbuddy-*.bin; do
        if [ -f "$old_file" ] && [ "$old_file" != "$OUTPUT_PATH" ]; then
            echo_info "Removing old version: $(basename "$old_file")"
            rm -f "$old_file"
        fi
    done

    echo ""
    echo_info "Firmware release ready!"
    echo "  Version: $VERSION"
    echo "  File:    $OUTPUT_PATH"
    echo "  Size:    $SIZE"
    echo ""
    echo_info "The firmware is now available for OTA updates."
    echo_info "Devices will download it when they check for updates."
}

# Only run main if script is executed directly (not sourced)
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
