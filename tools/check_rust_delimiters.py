#!/usr/bin/env python3
"""Lightweight lexical delimiter scan for Rust sources.

This is not a compiler. It catches truncated edits, unclosed strings/comments,
and unbalanced (), [], or {} while ignoring Rust comments and string literals.
"""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OPEN_TO_CLOSE = {"(": ")", "[": "]", "{": "}"}
CLOSE_TO_OPEN = {value: key for key, value in OPEN_TO_CLOSE.items()}


def scan(path: Path) -> None:
    text = path.read_text(encoding="utf-8")
    stack: list[tuple[str, int, int]] = []
    i = 0
    line = 1
    column = 1
    block_depth = 0

    def advance(count: int = 1) -> None:
        nonlocal i, line, column
        for _ in range(count):
            if i >= len(text):
                return
            if text[i] == "\n":
                line += 1
                column = 1
            else:
                column += 1
            i += 1

    while i < len(text):
        if block_depth:
            if text.startswith("/*", i):
                block_depth += 1
                advance(2)
            elif text.startswith("*/", i):
                block_depth -= 1
                advance(2)
            else:
                advance()
            continue

        if text.startswith("//", i):
            while i < len(text) and text[i] != "\n":
                advance()
            continue

        if text.startswith("/*", i):
            block_depth = 1
            advance(2)
            continue

        # Raw strings: r"...", r#"..."#, br##"..."##.
        raw_start = i
        if text.startswith("br", i):
            raw_start = i + 2
        elif text.startswith("r", i):
            raw_start = i + 1
        else:
            raw_start = -1
        if raw_start >= 0:
            hashes = 0
            j = raw_start
            while j < len(text) and text[j] == "#":
                hashes += 1
                j += 1
            if j < len(text) and text[j] == '"':
                opening_len = j - i + 1
                advance(opening_len)
                terminator = '"' + ("#" * hashes)
                end = text.find(terminator, i)
                if end < 0:
                    raise AssertionError(f"{path}:{line}:{column}: unclosed raw string")
                advance(end - i + len(terminator))
                continue

        if text[i] == '"':
            start_line, start_column = line, column
            advance()
            escaped = False
            while i < len(text):
                ch = text[i]
                if escaped:
                    escaped = False
                    advance()
                elif ch == "\\":
                    escaped = True
                    advance()
                elif ch == '"':
                    advance()
                    break
                else:
                    advance()
            else:
                raise AssertionError(
                    f"{path}:{start_line}:{start_column}: unclosed string literal"
                )
            continue

        # Character literals, including escaped forms. Lifetimes such as 'a do
        # not contain a closing quote immediately and are left as normal tokens.
        if text[i] == "'":
            j = i + 1
            if j < len(text) and text[j] == "\\":
                j += 2
                if j < len(text) and text[j - 1] == "u" and text[j] == "{":
                    j = text.find("}", j) + 1
            else:
                j += 1
            if 0 <= j < len(text) and text[j] == "'":
                advance(j - i + 1)
                continue

        ch = text[i]
        if ch in OPEN_TO_CLOSE:
            stack.append((ch, line, column))
        elif ch in CLOSE_TO_OPEN:
            if not stack or stack[-1][0] != CLOSE_TO_OPEN[ch]:
                raise AssertionError(f"{path}:{line}:{column}: unmatched {ch}")
            stack.pop()
        advance()

    if block_depth:
        raise AssertionError(f"{path}: unclosed block comment")
    if stack:
        ch, open_line, open_column = stack[-1]
        raise AssertionError(
            f"{path}:{open_line}:{open_column}: unclosed {ch}, expected {OPEN_TO_CLOSE[ch]}"
        )


def main() -> int:
    paths = sorted((ROOT / "src").rglob("*.rs"))
    for path in paths:
        scan(path)
    print(f"PASS: Rust lexical delimiter scan accepted {len(paths)} source files.")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as error:
        print(f"FAIL: {error}", file=sys.stderr)
        raise SystemExit(1)
