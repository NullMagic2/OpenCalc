#!/usr/bin/env python3
"""Verify that shared OpenCalc modules do not absorb OS-specific frontend code."""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "src"

UI_BACKENDS = [SRC / "ui" / name for name in ("windows.rs", "linux.rs", "other.rs")]
PLATFORM_BACKENDS = [
    SRC / "platform" / name for name in ("windows.rs", "linux.rs", "other.rs")
]
LOCALE_BACKENDS = [
    SRC / "locale" / name for name in ("windows.rs", "linux.rs", "other.rs")
]

PUB_SUPER_FN = re.compile(r"(?m)^pub\(super\)\s+fn\s+([A-Za-z_][A-Za-z0-9_]*)")
PUB_FN = re.compile(r"(?m)^pub\s+fn\s+([A-Za-z_][A-Za-z0-9_]*)")


def fail(message: str) -> None:
    raise AssertionError(message)


def function_set(path: Path, pattern: re.Pattern[str]) -> set[str]:
    return set(pattern.findall(path.read_text(encoding="utf-8")))


def require_same_interface(paths: list[Path], pattern: re.Pattern[str], label: str) -> None:
    baseline = function_set(paths[0], pattern)
    for path in paths[1:]:
        current = function_set(path, pattern)
        if current != baseline:
            missing = sorted(baseline - current)
            extra = sorted(current - baseline)
            fail(f"{label} mismatch in {path}: missing={missing}, extra={extra}")


def check_shared_file(path: Path, forbidden: tuple[str, ...]) -> None:
    text = path.read_text(encoding="utf-8")
    body = "\n".join(text.splitlines()[24:])
    body = re.sub(r"/\*.*?\*/", "", body, flags=re.DOTALL)
    body = re.sub(r"//.*", "", body)
    for token in forbidden:
        if token in body:
            fail(f"{path} contains platform-specific code outside its selector header: {token}")


def main() -> int:
    for obsolete in (SRC / "ui.rs", SRC / "platform.rs", SRC / "locale.rs"):
        if obsolete.exists():
            fail(f"obsolete mixed-platform module still exists: {obsolete}")

    require_same_interface(UI_BACKENDS, PUB_SUPER_FN, "UI frontend facade")
    require_same_interface(PLATFORM_BACKENDS, PUB_FN, "platform integration facade")
    require_same_interface(LOCALE_BACKENDS, PUB_SUPER_FN, "numeric locale facade")

    for path in UI_BACKENDS + PLATFORM_BACKENDS + LOCALE_BACKENDS:
        if 'target_os = ' in path.read_text(encoding="utf-8"):
            fail(f"backend contains target selection that belongs in mod.rs: {path}")

    check_shared_file(
        SRC / "ui" / "mod.rs",
        ("SetWindowTheme", "MessageDialog", "StaticBox", "gtk_", "WM_MOUSEACTIVATE"),
    )
    check_shared_file(
        SRC / "platform" / "mod.rs",
        ("MessageBoxW", "gtk_", "GetDpiForWindow", "OpenClipboard"),
    )
    check_shared_file(
        SRC / "locale" / "mod.rs",
        ("GetLocaleInfoEx", "localeconv", "setlocale"),
    )

    print("PASS: shared UI, platform, and locale modules are target-neutral.")
    print("PASS: Windows, Linux, and fallback backends expose matching facades.")
    print("PASS: obsolete mixed-platform source files are absent.")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as error:
        print(f"FAIL: {error}", file=sys.stderr)
        raise SystemExit(1)
