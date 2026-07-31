#!/usr/bin/env bash
set -euo pipefail

# ------------------------------------------------------------
# OpenCalc Linux build script
#
# Builds OpenCalc and its native Rust HLP Viewer companion in release mode,
# then places the runtime files in:
#
#   build-linux/
#
# The output contains:
#   OpenCalc
#   hlp-viewer
#   calc.tooltip
#   Help/*.HLP
#   Help/*.CNT
#
# An existing OpenCalc.cfg is user data and is preserved across rebuilds.
# Rust-HLP-Viewer is cloned only when its source directory is absent.
# ------------------------------------------------------------

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

BUILD_DIR="$SCRIPT_DIR/build-linux"
CARGO_OUTPUT="$SCRIPT_DIR/target/release/OpenCalc"
HLP_VIEWER_URL="https://github.com/NullMagic2/Rust-HLP-Viewer"
HLP_VIEWER_DIR="$SCRIPT_DIR/Rust-HLP-Viewer"
HLP_VIEWER_ROOT_MANIFEST="$HLP_VIEWER_DIR/Cargo.toml"
HLP_VIEWER_MANIFEST="$HLP_VIEWER_DIR/viewer/Cargo.toml"
HLP_VIEWER_OUTPUT="$HLP_VIEWER_DIR/target/release/hlp-viewer"

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

require_executable() {
    require_file "$1"
    if [[ ! -x "$1" ]]; then
        echo "ERROR: Required executable is not executable:" >&2
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

HLP_VIEWER_ROOT_MANIFEST_BACKUP=""

restore_hlp_workspace_manifest() {
    if [[ -n "$HLP_VIEWER_ROOT_MANIFEST_BACKUP" && -f "$HLP_VIEWER_ROOT_MANIFEST_BACKUP" ]]; then
        cp -- "$HLP_VIEWER_ROOT_MANIFEST_BACKUP" "$HLP_VIEWER_ROOT_MANIFEST"
        rm -f -- "$HLP_VIEWER_ROOT_MANIFEST_BACKUP"
        HLP_VIEWER_ROOT_MANIFEST_BACKUP=""
    fi
}

trap restore_hlp_workspace_manifest EXIT

hlp_workspace_manifest_is_valid() {
    cargo metadata \
        --no-deps \
        --format-version 1 \
        --manifest-path "$HLP_VIEWER_ROOT_MANIFEST" \
        >/dev/null 2>&1
}

prepare_hlp_workspace_manifest() {
    if hlp_workspace_manifest_is_valid; then
        return 0
    fi

    echo "The current Rust-HLP-Viewer workspace Cargo.toml is invalid."
    echo "Recovering the newest valid workspace manifest from Git history..."

    require_command git
    require_command mktemp

    if ! git -C "$HLP_VIEWER_DIR" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
        echo "ERROR: Rust-HLP-Viewer is not a Git checkout, so its invalid" >&2
        echo "       workspace manifest cannot be recovered automatically:" >&2
        echo "       $HLP_VIEWER_ROOT_MANIFEST" >&2
        return 1
    fi

    if [[ "$(git -C "$HLP_VIEWER_DIR" rev-parse --is-shallow-repository)" == "true" ]]; then
        echo "Downloading Rust-HLP-Viewer history needed for manifest recovery..."
        git -C "$HLP_VIEWER_DIR" fetch --quiet --unshallow origin
    fi

    HLP_VIEWER_ROOT_MANIFEST_BACKUP="$(mktemp "${TMPDIR:-/tmp}/opencalc-hlp-cargo-backup.XXXXXX")"
    local candidate
    candidate="$(mktemp "${TMPDIR:-/tmp}/opencalc-hlp-cargo-candidate.XXXXXX")"
    cp -- "$HLP_VIEWER_ROOT_MANIFEST" "$HLP_VIEWER_ROOT_MANIFEST_BACKUP"

    local revision
    while IFS= read -r revision; do
        [[ -n "$revision" ]] || continue
        if ! git -C "$HLP_VIEWER_DIR" show "$revision:Cargo.toml" > "$candidate" 2>/dev/null; then
            continue
        fi

        cp -- "$candidate" "$HLP_VIEWER_ROOT_MANIFEST"
        if hlp_workspace_manifest_is_valid; then
            echo "Using valid Rust-HLP-Viewer workspace manifest from commit:"
            echo "  $revision"
            rm -f -- "$candidate"
            return 0
        fi
    done < <(git -C "$HLP_VIEWER_DIR" rev-list HEAD -- Cargo.toml)

    rm -f -- "$candidate"
    restore_hlp_workspace_manifest
    echo "ERROR: No valid Rust-HLP-Viewer workspace Cargo.toml was found" >&2
    echo "       in the repository history." >&2
    return 1
}

require_command cargo
require_command install
require_command pkg-config
require_command find

if ! pkg-config --atleast-version=4.10 gtk4; then
    echo "ERROR: GTK 4.10 or newer development files are required." >&2
    echo "       On Debian/Ubuntu, install: libgtk-4-dev pkg-config" >&2
    exit 1
fi

if [[ -f "$HLP_VIEWER_MANIFEST" ]]; then
    echo "Using existing Rust-HLP-Viewer source:"
    echo "  $HLP_VIEWER_DIR"
elif [[ -e "$HLP_VIEWER_DIR" ]]; then
    echo "ERROR: Rust-HLP-Viewer exists but is not a valid source checkout:" >&2
    echo "       $HLP_VIEWER_DIR" >&2
    echo "       Expected: $HLP_VIEWER_MANIFEST" >&2
    exit 1
else
    require_command git
    echo "Downloading Rust-HLP-Viewer..."
    git clone "$HLP_VIEWER_URL" "$HLP_VIEWER_DIR"
    require_file "$HLP_VIEWER_MANIFEST"
fi

prepare_hlp_workspace_manifest

echo "Building Rust-HLP-Viewer for Linux..."
cargo build \
    --release \
    --manifest-path "$HLP_VIEWER_MANIFEST" \
    --target-dir "$HLP_VIEWER_DIR/target"
require_executable "$HLP_VIEWER_OUTPUT"
restore_hlp_workspace_manifest

echo "Building OpenCalc for Linux with native GTK4..."
cargo build --release
require_executable "$CARGO_OUTPUT"

echo "Preparing build-linux..."
# Match the Windows packaging behavior: OpenCalc.cfg is user data beside the
# executable, so clean the packaged output while leaving that one file intact.
mkdir -p "$BUILD_DIR"
find "$BUILD_DIR" -mindepth 1 -maxdepth 1 \
    ! -name 'OpenCalc.cfg' -exec rm -rf -- {} +
mkdir -p "$BUILD_DIR/Help"

install -m 0755 "$CARGO_OUTPUT" "$BUILD_DIR/OpenCalc"
install -m 0755 "$HLP_VIEWER_OUTPUT" "$BUILD_DIR/hlp-viewer"
copy_required "$SCRIPT_DIR/calc.tooltip" \
    "$BUILD_DIR/calc.tooltip"
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

require_executable "$BUILD_DIR/OpenCalc"
require_executable "$BUILD_DIR/hlp-viewer"

echo
echo "Linux build completed successfully:"
echo "  $BUILD_DIR"
echo
echo "Included files:"
find "$BUILD_DIR" -type f -printf '  %P\n' | LC_ALL=C sort
