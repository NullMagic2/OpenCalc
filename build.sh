#!/usr/bin/env bash
set -euo pipefail

# ------------------------------------------------------------
# OpenCalc Linux build script
#
# Builds OpenCalc in release mode and places the runtime files in:
#
#   build-linux/
#
# The output contains:
#   OpenCalc
#   calc.tooltip
#   Help/*.HLP
#   Help/*.CNT
#
# A native extensionless "hlp-viewer" is copied when present.
# The Windows-only "hlp-viewer.exe" is intentionally excluded.
# ------------------------------------------------------------

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

BUILD_DIR="$SCRIPT_DIR/build-linux"
CARGO_OUTPUT="$SCRIPT_DIR/target/release/OpenCalc"

require_command() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "ERROR: Required command not found: $1" >&2
        return 1
    fi
}

require_file() {
    if [[ ! -f "$1" ]]; then
        echo "ERROR: Required file is missing:" >&2
        echo "       $1" >&2
        return 1
    fi
}

copy_required() {
    local source="$1"
    local destination="$2"

    require_file "$source"
    install -m 0644 "$source" "$destination"
}

require_command cargo
require_command install

echo "Building OpenCalc for Linux..."
cargo build --release

require_file "$CARGO_OUTPUT"

echo "Preparing build-linux..."
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR/Help"

install -m 0755 "$CARGO_OUTPUT" "$BUILD_DIR/OpenCalc"
copy_required "$SCRIPT_DIR/calc.tooltip" "$BUILD_DIR/calc.tooltip"

copy_required "$SCRIPT_DIR/Help/CALC_EN.HLP" \
    "$BUILD_DIR/Help/CALC_EN.HLP"
copy_required "$SCRIPT_DIR/Help/CALC_EN.CNT" \
    "$BUILD_DIR/Help/CALC_EN.CNT"

copy_required "$SCRIPT_DIR/Help/CALC_PT-BR.HLP" \
    "$BUILD_DIR/Help/CALC_PT-BR.HLP"
copy_required "$SCRIPT_DIR/Help/CALC_PT-BR.CNT" \
    "$BUILD_DIR/Help/CALC_PT-BR.CNT"

copy_required "$SCRIPT_DIR/Help/CALC_ES.HLP" \
    "$BUILD_DIR/Help/CALC_ES.HLP"
copy_required "$SCRIPT_DIR/Help/CALC_ES.CNT" \
    "$BUILD_DIR/Help/CALC_ES.CNT"

if [[ -f "$SCRIPT_DIR/hlp-viewer" ]]; then
    install -m 0755 "$SCRIPT_DIR/hlp-viewer" "$BUILD_DIR/hlp-viewer"
    echo "Included native hlp-viewer."
else
    echo "Note: no native hlp-viewer was found."
    echo "      Help files were included, but opening them requires an"
    echo "      extensionless Linux hlp-viewer beside OpenCalc."
fi

echo
echo "Linux build completed successfully:"
echo "  $BUILD_DIR"
echo
echo "Included files:"
find "$BUILD_DIR" -type f -printf '  %P\n' | LC_ALL=C sort
