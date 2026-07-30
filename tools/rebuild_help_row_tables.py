#!/usr/bin/env python3
"""Rebuild OpenCalc reference grids as one WinHelp table record per visual row.

Classic WinHelp tables are independent vertical column flows, not HTML-style row
layouts.  A single multi-row record therefore drifts as soon as either column
wraps.  This generator emits a separate two-cell type-0x23 record for every
header/data row.  The enclosing record height is then the maximum of those two
cells, so the following row always begins below both columns.
"""
from __future__ import annotations

from pathlib import Path
from dataclasses import replace
import argparse

import rebuild_help_reference_tables as h
import repad_help_tables as content

NBSP = "\u00a0"


def padded(text: str) -> str:
    return f"{NBSP}{text}{NBSP}"


def build_pair(
    template: h.Record,
    left: str,
    right: str,
    identity: str,
    widths: tuple[int, int],
    *,
    header: bool,
) -> h.Record:
    header_left, header_right, key_cell, description_cell = h.parse_table_templates(template)
    left_payload, right_payload = (
        (header_left, header_right) if header else (key_cell, description_cell)
    )
    left_text = padded(left)
    right_text = padded(right)
    ld1 = bytearray()
    ld1.extend(h.encode_signed_long(16))
    ld1.extend(h.encode_unsigned_short(h.table_topic_length((left_text, right_text), [])))
    ld1.extend(bytes([2, 1]))
    ld1.extend(h.p16(widths[0]) + h.p16(0))
    ld1.extend(h.p16(widths[1]) + h.p16(0))
    ld2 = bytearray()

    def append_cell(column: int, payload: bytes, text: str) -> None:
        encoded = text.encode("cp1252")
        ld1.extend(h.pi16(column))
        ld1.append(0x20)
        ld1.extend(h.encode_signed_long(len(payload)))
        ld1.extend(h.encode_unsigned_short(0))
        ld1.extend(payload)
        ld2.extend(b"\0" + encoded + b"\0")

    append_cell(0, left_payload, left_text)
    append_cell(1, right_payload, right_text)
    ld1.extend(h.pi16(-1))
    return h.Record(identity, None, None, 0, 0x23, bytes(ld1), bytes(ld2), 0)


def build_stack(
    template: h.Record,
    headers: tuple[str, str],
    rows: list[tuple[str, str]],
    widths: tuple[int, int],
    prefix: str,
) -> list[h.Record]:
    result = [build_pair(template, headers[0], headers[1], f"{prefix}:header", widths, header=True)]
    result.extend(
        build_pair(template, left, right, f"{prefix}:row:{index}", widths, header=False)
        for index, (left, right) in enumerate(rows)
    )
    return result


def table_after_heading(records: list[h.Record], heading: str) -> int:
    for index, record in enumerate(records[:-1]):
        if heading in h.record_text(record) and records[index + 1].record_type == 0x23:
            return index + 1
    raise ValueError(f"table after heading not found: {heading}")


LANGUAGE = {
    "CALC_EN.HLP": {
        "code": "en",
        "operator_headers": ("Operator", "Description"),
        "key_headers": ("Key", "Description"),
        "headings": ("Basic and editing keys", "Scientific keys", "Function and control shortcuts"),
    },
    "CALC_PT-BR.HLP": {
        "code": "pt",
        "operator_headers": ("Operador", "Descrição"),
        "key_headers": ("Tecla", "Descrição"),
        "headings": ("Teclas básicas e de edição", "Teclas científicas", "Atalhos de função e controle"),
    },
    "CALC_ES.HLP": {
        "code": "es",
        "operator_headers": ("Operador", "Descripción"),
        "key_headers": ("Tecla", "Descripción"),
        "headings": ("Teclas básicas y de edición", "Teclas científicas", "Atajos de función y control"),
    },
}


def cell_count(record: h.Record) -> int:
    data = record.ld1
    _size, pos = h.decode_signed_long(data, 0)
    _length, pos = h.decode_unsigned_short(data, pos)
    column_count = data[pos]
    table_type = data[pos + 1]
    pos += 2
    if table_type == 0:
        pos += 2
    pos += column_count * 4
    count = 0
    while True:
        column = h.i16(data, pos)
        pos += 2
        if column == -1:
            return count
        record_type = data[pos]
        pos += 1
        payload_size, pos = h.decode_signed_long(data, pos)
        if record_type > 0x10:
            _topic_length, pos = h.decode_unsigned_short(data, pos)
        pos += payload_size
        count += 1


def rebuild_manual(path: Path) -> dict[str, int]:
    cfg = LANGUAGE[path.name]
    language = cfg["code"]
    data = bytearray(path.read_bytes())
    entries = {entry.name: entry for entry in h.parse_directory(data)}
    topic_entry = entries["|TOPIC"]
    original_records, _ = h.parse_topic_records(h.internal_file(data, topic_entry.file_offset))

    operator_index = next(
        index for index, record in enumerate(original_records)
        if record.record_type == 0x23
        and (("Operator" in h.record_text(record) and "Description" in h.record_text(record))
             or ("Operador" in h.record_text(record) and "Descrição" in h.record_text(record))
             or ("Operador" in h.record_text(record) and "Descripción" in h.record_text(record)))
    )
    basic_index = table_after_heading(original_records, cfg["headings"][0])
    scientific_index = table_after_heading(original_records, cfg["headings"][1])
    control_index = table_after_heading(original_records, cfg["headings"][2])

    # Safe to run repeatedly: a row-safe manual already has a two-cell table
    # immediately after each target heading, rather than one large multi-cell
    # independent-column record.
    target_indices = (operator_index, basic_index, scientific_index, control_index)
    if all(cell_count(original_records[index]) == 2 for index in target_indices):
        crossing = [
            record.old_pos for record in original_records
            if record.old_pos is not None
            and (record.old_pos - h.TOPIC_HEADER_SIZE) % h.TOPIC_DATA_SIZE + h.TOPIC_LINK_HEADER_SIZE > h.TOPIC_DATA_SIZE
        ]
        if crossing:
            raise ValueError(f"{path}: crossing TOPICLINK headers: {crossing}")
        return {
            "records": len(original_records),
            "topics": sum(record.record_type == 0x21 for record in original_records),
            "blocks": h.internal_file(data, topic_entry.file_offset).used // h.TOPIC_BLOCK_SIZE,
            "row_tables": sum(record.record_type == 0x23 and cell_count(record) == 2 for record in original_records),
            "bytes": len(data),
        }

    specifications = {
        operator_index: (cfg["operator_headers"], content.OPERATOR_ROWS[language], (120, 620), "operators"),
        basic_index: (cfg["key_headers"], content.BASIC_ROWS[language], (155, 585), "basic"),
        scientific_index: (cfg["key_headers"], content.SCIENTIFIC_ROWS[language], (135, 605), "scientific"),
        control_index: (cfg["key_headers"], content.CONTROL_ROWS[language], (165, 575), "control"),
    }

    records: list[h.Record] = []
    generated_ids: list[str] = []
    for index, record in enumerate(original_records):
        spec = specifications.get(index)
        if spec is None:
            records.append(record)
            continue
        headers, rows, widths, name = spec
        stack = build_stack(record, headers, rows, widths, f"new:{language}:{name}")
        first = stack[0]
        first.identity = record.identity
        first.old_index = record.old_index
        first.old_pos = record.old_pos
        first.old_size = record.old_size
        stack[-1].gap_after = record.gap_after
        generated_ids.extend(item.identity for item in stack)
        records.extend(stack)

    h.assign_positions(records)
    h.patch_topic_positions(records)
    translate = h.topic_offset_translator(original_records, records)
    h.patch_topic_offsets(records, translate)
    h.patch_navigation_streams(data, entries, translate)
    h.patch_system_contents(data, entries["|SYSTEM"], translate)
    topic_stream = h.rebuild_topic_stream(records)
    rebuilt_bytes = h.replace_topic_file(data, topic_entry, topic_stream)
    path.write_bytes(rebuilt_bytes)

    checked_entries = {entry.name: entry for entry in h.parse_directory(rebuilt_bytes)}
    checked_topic = h.internal_file(rebuilt_bytes, checked_entries["|TOPIC"].file_offset)
    checked_records, _ = h.parse_topic_records(checked_topic)
    crossing = [
        record.old_pos for record in checked_records
        if record.old_pos is not None
        and (record.old_pos - h.TOPIC_HEADER_SIZE) % h.TOPIC_DATA_SIZE + h.TOPIC_LINK_HEADER_SIZE > h.TOPIC_DATA_SIZE
    ]
    if crossing:
        raise ValueError(f"{path}: crossing TOPICLINK headers: {crossing}")

    # Every generated table record must contain exactly one two-cell visual row.
    generated = [record for record in records if record.identity in generated_ids or record.identity.startswith(f"new:{language}:")]
    bad = [record.identity for record in generated if record.record_type == 0x23 and cell_count(record) != 2]
    if bad:
        raise ValueError(f"{path}: non-two-cell generated rows: {bad}")

    return {
        "records": len(checked_records),
        "topics": sum(record.record_type == 0x21 for record in checked_records),
        "blocks": checked_topic.used // h.TOPIC_BLOCK_SIZE,
        "row_tables": sum(record.record_type == 0x23 and cell_count(record) == 2 for record in checked_records),
        "bytes": len(rebuilt_bytes),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("help_directory", nargs="?", default="Help")
    args = parser.parse_args()
    directory = Path(args.help_directory)
    for filename in LANGUAGE:
        result = rebuild_manual(directory / filename)
        print(f"{filename}: {result}")


if __name__ == "__main__":
    main()
