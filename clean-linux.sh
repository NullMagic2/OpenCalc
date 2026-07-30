#!/usr/bin/env bash
set -euo pipefail

# ------------------------------------------------------------
# OpenCalc Linux cleanup script
#
# Removes Cargo output and the packaged Linux release directory while
# leaving all source files, Help files, and companion viewer binaries intact.
# ------------------------------------------------------------

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "Cleaning OpenCalc Linux build output..."
rm -rf -- "$SCRIPT_DIR/target" "$SCRIPT_DIR/build-linux"
rm -f -- "$SCRIPT_DIR/OpenCalc"

# WSL/Windows downloads can leave NTFS alternate-stream marker files behind
# after a project is copied into the Linux filesystem.
find "$SCRIPT_DIR" -type f -name '*:Zone.Identifier' -delete

echo "Linux build output removed."
