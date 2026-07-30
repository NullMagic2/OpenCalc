#!/usr/bin/env python3
"""Update the graph-field syntax topic in OpenCalc's compiled HLP manuals.

The operation keeps the topic/context identifiers intact while repacking
|TOPIC and translating affected WinHelp offsets. It documents the graph-only
implicit-multiplication and power notation accepted by buildfix72.
"""
from __future__ import annotations

from dataclasses import replace
from pathlib import Path
import argparse

import rebuild_help_reference_tables as hlp

CONTENT = {
    "CALC_EN.HLP": {
        "heading": "Graph field syntax",
        "description": (
            "Graph mode accepts implicit multiplication (2x), Unicode superscripts (x²), "
            "and powers written with ^ or **. Enter an expression, y = expression, "
            "f(x) = expression, or one equation."
        ),
        "examples": ("2x² + 2x + 2", "2x^2 + 2x + 2", "2x**2 + 2x + 2"),
        "note": (
            "Equations are plotted as left minus right; roots solve the equation. "
            "x is graph-only and is not valid in Calculator paste. Only one function "
            "or equation is accepted."
        ),
    },
    "CALC_PT-BR.HLP": {
        "heading": "Sintaxe do campo de gráfico",
        "description": (
            "O gráfico aceita multiplicação implícita (2x), sobrescritos Unicode (x²) "
            "e potências com ^ ou **. Digite uma expressão, y = expressão, "
            "f(x) = expressão ou uma equação."
        ),
        "examples": ("2x² + 2x + 2", "2x^2 + 2x + 2", "2x**2 + 2x + 2"),
        "note": (
            "Equações são plotadas como lado esquerdo menos lado direito; as raízes "
            "resolvem a equação. x só é válido no gráfico e não ao colar na Calculadora. "
            "Só uma função ou equação é aceita."
        ),
    },
    "CALC_ES.HLP": {
        "heading": "Sintaxis del campo de gráfico",
        "description": (
            "El gráfico acepta multiplicación implícita (2x), superíndices Unicode (x²) "
            "y potencias con ^ o **. Use una expresión, y = expresión, f(x) = expresión "
            "o una ecuación."
        ),
        "examples": ("2x² + 2x + 2", "2x^2 + 2x + 2", "2x**2 + 2x + 2"),
        "note": (
            "Las ecuaciones se grafican como lado izquierdo menos lado derecho; las raíces "
            "son las soluciones. x solo funciona en el gráfico y no al pegar en la "
            "Calculadora. Se acepta una función o ecuación."
        ),
    },
}


def visible_text(text: str) -> bytes:
    return b"\0" + text.encode("cp1252") + b"\0"


def update_manual(path: Path) -> dict[str, int]:
    authored = CONTENT[path.name]
    original_bytes = bytearray(path.read_bytes())
    directory_list = hlp.parse_directory(original_bytes)
    entries = {entry.name: entry for entry in directory_list}
    topic_entry = entries["|TOPIC"]
    topic = hlp.internal_file(original_bytes, topic_entry.file_offset)
    original_records, _headers = hlp.parse_topic_records(topic)

    heading_index = next(
        index
        for index, record in enumerate(original_records)
        if hlp.record_text(record) == authored["heading"]
    )
    target_indices = range(heading_index + 1, heading_index + 6)
    records = list(original_records)
    replacements = (
        authored["description"],
        authored["examples"][0],
        authored["examples"][1],
        authored["examples"][2],
        authored["note"],
    )
    for index, text in zip(target_indices, replacements):
        records[index] = replace(records[index], ld2=visible_text(text))

    hlp.assign_positions(records)
    hlp.patch_topic_positions(records)
    translate = hlp.topic_offset_translator(original_records, records)
    hlp.patch_topic_offsets(records, translate)
    hlp.patch_navigation_streams(original_bytes, entries, translate)
    hlp.patch_system_contents(original_bytes, entries["|SYSTEM"], translate)
    topic_stream = hlp.rebuild_topic_stream(records)
    rebuilt_bytes = hlp.replace_topic_file(original_bytes, topic_entry, topic_stream)
    path.write_bytes(rebuilt_bytes)

    rebuilt_entries = {entry.name: entry for entry in hlp.parse_directory(rebuilt_bytes)}
    rebuilt_topic = hlp.internal_file(rebuilt_bytes, rebuilt_entries["|TOPIC"].file_offset)
    checked, _ = hlp.parse_topic_records(rebuilt_topic)
    text = b"\0".join(record.ld2 for record in checked)
    for expected in replacements:
        if expected.encode("cp1252") not in text:
            raise ValueError(f"{path}: missing updated graph text: {expected}")
    crossings = [
        record.old_pos
        for record in checked
        if record.old_pos is not None
        and (record.old_pos - 12) % hlp.TOPIC_DATA_SIZE + hlp.TOPIC_LINK_HEADER_SIZE
        > hlp.TOPIC_DATA_SIZE
    ]
    if crossings:
        raise ValueError(f"{path}: crossing TOPICLINK headers: {crossings}")
    return {
        "records": len(checked),
        "topics": sum(record.record_type == 0x21 for record in checked),
        "topic_blocks": rebuilt_topic.used // hlp.TOPIC_BLOCK_SIZE,
        "bytes": len(rebuilt_bytes),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("help_directory", nargs="?", default="Help")
    args = parser.parse_args()
    directory = Path(args.help_directory)
    for filename in CONTENT:
        print(filename, update_manual(directory / filename))


if __name__ == "__main__":
    main()
