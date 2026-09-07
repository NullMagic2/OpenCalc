#!/usr/bin/env python3
"""Correct Exp and PI behavior in OpenCalc's compiled WinHelp manuals.

The HLP files are uncompressed WinHelp 4 containers.  Replacing topic records
changes TOPICOFFSET positions, so use the shared HLP repacker rather than a raw
byte substitution.
"""
from __future__ import annotations

from dataclasses import replace
from pathlib import Path
import argparse

import rebuild_help_reference_tables as hlp

CONTENT = {
    "CALC_EN.HLP": {
        "old_exp": "In this Rust implementation the Exp button applies the exponential function e^x to the displayed value. Scientific notation in pasted text is entered directly with e/E, for example 1e-3.",
        "exp": "Exp enters scientific notation; it does not calculate e^x. In Decimal mode it opens a signed three-digit exponent field (for example 12.e+003). After C, Exp starts from 1.e+000. The upper positive exponent is +289.",
        "old_pi": "PI places the value of pi in the display. The pasted-expression parser accepts pi, pi(), and the Greek pi character as the same constant.",
        "pi": "PI is available only in Decimal mode. PI displays pi; Inv+PI displays 2*pi and consumes Inv. Pasted expressions still accept pi, pi(), and the Greek pi character as the same constant.",
        "old_note": "• The Exp button computes e^x in this implementation; e/E inside a pasted number is the scientific-notation marker.",
        "note": "• Exp enters a decimal scientific-notation exponent; e/E inside a pasted number remains the scientific-notation marker.",
    },
    "CALC_PT-BR.HLP": {
        "old_exp": "Nesta implementação Rust, Exp aplica a função exponencial e^x ao valor exibido. Em texto colado, a notação científica usa e/E diretamente, por exemplo  1e-3 .",
        "exp": "Exp digita notação científica; não calcula e^x. No modo decimal, abre um expoente sinalizado de três dígitos (por exemplo 12.e+003). Após C, Exp começa em 1.e+000. O limite positivo é +289.",
        "old_pi": "PI coloca o valor de pi no visor. O analisador de expressões coladas aceita pi, pi() e o caractere grego pi como a mesma constante.",
        "pi": "PI só está disponível no modo decimal. PI exibe pi; Inv+PI exibe 2*pi e desativa Inv. Expressões coladas continuam aceitando pi, pi() e o caractere grego pi.",
        "old_note": "• Exp calcula e^x nesta implementação; e/E dentro de um número colado marca notação científica.",
        "note": "• Exp digita o expoente da notação científica decimal; e/E dentro de um número colado continua marcando notação científica.",
    },
    "CALC_ES.HLP": {
        "old_exp": "En esta implementación Rust, Exp aplica la función exponencial e^x al valor mostrado. En texto pegado, la notación científica usa e/E directamente, por ejemplo  1e-3 .",
        "exp": "Exp introduce notación científica; no calcula e^x. En modo decimal abre un exponente con signo de tres dígitos (por ejemplo 12.e+003). Después de C, Exp empieza en 1.e+000. El límite positivo es +289.",
        "old_pi": "PI coloca el valor de pi en la pantalla. El analizador de expresiones pegadas acepta pi, pi() y el carácter griego pi como la misma constante.",
        "pi": "PI solo está disponible en modo decimal. PI muestra pi; Inv+PI muestra 2*pi y desactiva Inv. Las expresiones pegadas siguen aceptando pi, pi() y el carácter griego pi.",
        "old_note": "• Exp calcula e^x en esta implementación; e/E dentro de un número pegado marca notación científica.",
        "note": "• Exp introduce el exponente de la notación científica decimal; e/E dentro de un número pegado sigue marcando notación científica.",
    },
}


def visible_text(text: str) -> bytes:
    return b"\0" + text.encode("cp1252") + b"\0"


def update_manual(path: Path) -> dict[str, int]:
    authored = CONTENT[path.name]
    original = bytearray(path.read_bytes())
    entries = {entry.name: entry for entry in hlp.parse_directory(original)}
    topic_entry = entries["|TOPIC"]
    topic = hlp.internal_file(original, topic_entry.file_offset)
    original_records, _ = hlp.parse_topic_records(topic)
    records = list(original_records)

    pairs = [(authored["old_exp"], authored["exp"]), (authored["old_pi"], authored["pi"]), (authored["old_note"], authored["note"])]
    replaced = 0
    for old, new in pairs:
        old_index = next((i for i, r in enumerate(records) if hlp.record_text(r) == old), None)
        new_index = next((i for i, r in enumerate(records) if hlp.record_text(r) == new), None)
        if old_index is not None:
            records[old_index] = replace(records[old_index], ld2=visible_text(new))
            replaced += 1
        elif new_index is None:
            raise ValueError(f"{path}: could not locate topic text: {old!r}")

    if replaced:
        hlp.assign_positions(records)
        hlp.patch_topic_positions(records)
        translate = hlp.topic_offset_translator(original_records, records)
        hlp.patch_topic_offsets(records, translate)
        hlp.patch_navigation_streams(original, entries, translate)
        hlp.patch_system_contents(original, entries["|SYSTEM"], translate)
        topic_stream = hlp.rebuild_topic_stream(records)
        rebuilt = hlp.replace_topic_file(original, topic_entry, topic_stream)
        path.write_bytes(rebuilt)
    else:
        rebuilt = bytes(original)

    check_entries = {entry.name: entry for entry in hlp.parse_directory(rebuilt)}
    checked_topic = hlp.internal_file(rebuilt, check_entries["|TOPIC"].file_offset)
    checked, _ = hlp.parse_topic_records(checked_topic)
    visible = [hlp.record_text(record) for record in checked]
    for _, new in pairs:
        if new not in visible:
            raise ValueError(f"{path}: missing corrected topic: {new!r}")
    return {"replaced": replaced, "bytes": len(rebuilt), "records": len(checked)}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("help_directory", nargs="?", default="Help")
    args = parser.parse_args()
    directory = Path(args.help_directory)
    for filename in CONTENT:
        print(filename, update_manual(directory / filename))


if __name__ == "__main__":
    main()
