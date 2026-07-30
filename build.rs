#!/usr/bin/env python3
"""Regenerate OpenCalc WinHelp lookup tables with matched one-line rows.

WinHelp's legacy table records do not automatically equalize independently
encoded cells when one side wraps.  This tool therefore uses concise one-line
copy, explicit nonbreaking-space cell padding, and stable column geometry.
"""
from __future__ import annotations

from dataclasses import replace
from pathlib import Path
import argparse

import rebuild_help_reference_tables as h

NBSP = "\u00a0"


def padded(text: str) -> str:
    return f"{NBSP}{text}{NBSP}"


OPERATOR_ROWS = {
    "en": [
        ("+", "Addition; unary plus."),
        ("-", "Subtraction; unary minus."),
        ("*", "Multiplication."),
        ("×", "Alias for *."),
        ("/", "Division."),
        ("÷", "Alias for /."),
        ("^", "Exponentiation."),
        ("**", "Alternate power syntax."),
        ("%", "Postfix percent; divide by 100."),
        ("!", "Factorial."),
        ("mod", "Integer remainder."),
        ("and", "Bitwise AND."),
        ("or", "Bitwise OR."),
        ("xor", "Bitwise XOR."),
        ("lsh", "Left shift."),
        ("root", "x root y = x^(1/y)."),
        ("(", "Open grouped expression."),
        (")", "Close grouped expression."),
        ("=", "Optional trailing equals."),
    ],
    "pt": [
        ("+", "Adição; mais unário."),
        ("-", "Subtração; menos unário."),
        ("*", "Multiplicação."),
        ("×", "Alias de *."),
        ("/", "Divisão."),
        ("÷", "Alias de /."),
        ("^", "Exponenciação."),
        ("**", "Sintaxe alternativa de potência."),
        ("%", "Porcentagem pós-fixa; divide por 100."),
        ("!", "Fatorial."),
        ("mod", "Resto inteiro."),
        ("and", "E bit a bit."),
        ("or", "OU bit a bit."),
        ("xor", "XOR bit a bit."),
        ("lsh", "Deslocamento à esquerda."),
        ("root", "x root y = x^(1/y)."),
        ("(", "Abre expressão agrupada."),
        (")", "Fecha expressão agrupada."),
        ("=", "Igual final opcional."),
    ],
    "es": [
        ("+", "Suma; más unario."),
        ("-", "Resta; menos unario."),
        ("*", "Multiplicación."),
        ("×", "Alias de *."),
        ("/", "División."),
        ("÷", "Alias de /."),
        ("^", "Exponenciación."),
        ("**", "Sintaxis alternativa de potencia."),
        ("%", "Porcentaje posfijo; divide entre 100."),
        ("!", "Factorial."),
        ("mod", "Resto entero."),
        ("and", "AND bit a bit."),
        ("or", "OR bit a bit."),
        ("xor", "XOR bit a bit."),
        ("lsh", "Desplazamiento a la izquierda."),
        ("root", "x root y = x^(1/y)."),
        ("(", "Abre expresión agrupada."),
        (")", "Cierra expresión agrupada."),
        ("=", "Igual final opcional."),
    ],
}

BASIC_ROWS = {
    "en": [
        ("0-9", "Enter a decimal digit."),
        ("A-F", "Enter a hexadecimal digit in Hex."),
        (".", "Enter the decimal separator."),
        (",", "Enter the decimal separator."),
        ("+", "Addition."),
        ("-", "Subtraction."),
        ("*", "Multiply; type another * for **."),
        ("**", "Exponentiation."),
        ("×", "Alias for *."),
        ("/", "Division."),
        ("÷", "Alias for /."),
        ("%", "Percent (Standard); Mod (Scientific)."),
        ("=", "Calculate the result."),
        ("Enter", "Calculate the result."),
        ("Backspace", "Delete the last digit (Back)."),
        ("Left", "Delete the last digit (Back)."),
        ("Delete", "Clear the current entry (CE)."),
        ("Esc", "Clear the calculation (C)."),
        ("F9", "Toggle the sign (+/-)."),
        ("(", "Open parenthesis (Scientific)."),
        (")", "Close parenthesis (Scientific)."),
        ("@", "sqrt (Standard); x^2 (Scientific)."),
    ],
    "pt": [
        ("0-9", "Digita um algarismo decimal."),
        ("A-F", "Digita um algarismo hexadecimal em Hex."),
        (".", "Digita o separador decimal."),
        (",", "Digita o separador decimal."),
        ("+", "Adição."),
        ("-", "Subtração."),
        ("*", "Multiplica; digite outro * para **."),
        ("**", "Exponenciação."),
        ("×", "Alias de *."),
        ("/", "Divisão."),
        ("÷", "Alias de /."),
        ("%", "Porcentagem (Padrão); Mod (Científico)."),
        ("=", "Calcula o resultado."),
        ("Enter", "Calcula o resultado."),
        ("Backspace", "Apaga o último dígito (Back)."),
        ("Esquerda", "Apaga o último dígito (Back)."),
        ("Delete", "Limpa a entrada atual (CE)."),
        ("Esc", "Limpa o cálculo (C)."),
        ("F9", "Alterna o sinal (+/-)."),
        ("(", "Abre parêntese (Científico)."),
        (")", "Fecha parêntese (Científico)."),
        ("@", "sqrt (Padrão); x^2 (Científico)."),
    ],
    "es": [
        ("0-9", "Introduce un dígito decimal."),
        ("A-F", "Introduce un dígito hexadecimal en Hex."),
        (".", "Introduce el separador decimal."),
        (",", "Introduce el separador decimal."),
        ("+", "Suma."),
        ("-", "Resta."),
        ("*", "Multiplica; escriba otro * para **."),
        ("**", "Exponenciación."),
        ("×", "Alias de *."),
        ("/", "División."),
        ("÷", "Alias de /."),
        ("%", "Porcentaje (Estándar); Mod (Científico)."),
        ("=", "Calcula el resultado."),
        ("Enter", "Calcula el resultado."),
        ("Retroceso", "Borra el último dígito (Back)."),
        ("Izquierda", "Borra el último dígito (Back)."),
        ("Supr", "Borra la entrada actual (CE)."),
        ("Esc", "Borra el cálculo (C)."),
        ("F9", "Alterna el signo (+/-)."),
        ("(", "Abre paréntesis (Científico)."),
        (")", "Cierra paréntesis (Científico)."),
        ("@", "sqrt (Estándar); x^2 (Científico)."),
    ],
}

SCIENTIFIC_ROWS = {
    "en": [
        ("!", "Factorial."), ("#", "Cube (x^3)."), ("r", "Reciprocal (1/x)."),
        ("s", "Sine."), ("o", "Cosine."), ("t", "Tangent."),
        ("n", "Natural logarithm (ln)."), ("l", "Base-10 logarithm (log)."),
        ("m", "DMS conversion."), ("x", "Exponent entry (Exp)."),
        ("y", "Power (x^y)."), ("p", "Insert pi."), ("i", "Toggle Inv."),
        ("h", "Toggle Hyp."), ("v", "Toggle F-E."), ("&", "Bitwise AND."),
        ("|", "Bitwise OR."), ("^", "Bitwise XOR in direct input."),
        ("<", "Left shift (Lsh)."), ("~", "Bitwise NOT."), (";", "Integer part (Int)."),
    ],
    "pt": [
        ("!", "Fatorial."), ("#", "Cubo (x^3)."), ("r", "Recíproco (1/x)."),
        ("s", "Seno."), ("o", "Cosseno."), ("t", "Tangente."),
        ("n", "Logaritmo natural (ln)."), ("l", "Logaritmo decimal (log)."),
        ("m", "Conversão DMS."), ("x", "Entrada de expoente (Exp)."),
        ("y", "Potência (x^y)."), ("p", "Insere pi."), ("i", "Alterna Inv."),
        ("h", "Alterna Hyp."), ("v", "Alterna F-E."), ("&", "E bit a bit."),
        ("|", "OU bit a bit."), ("^", "XOR na digitação direta."),
        ("<", "Desloca à esquerda (Lsh)."), ("~", "NÃO bit a bit."), (";", "Parte inteira (Int)."),
    ],
    "es": [
        ("!", "Factorial."), ("#", "Cubo (x^3)."), ("r", "Recíproco (1/x)."),
        ("s", "Seno."), ("o", "Coseno."), ("t", "Tangente."),
        ("n", "Logaritmo natural (ln)."), ("l", "Logaritmo decimal (log)."),
        ("m", "Conversión DMS."), ("x", "Entrada de exponente (Exp)."),
        ("y", "Potencia (x^y)."), ("p", "Inserta pi."), ("i", "Alterna Inv."),
        ("h", "Alterna Hyp."), ("v", "Alterna F-E."), ("&", "AND bit a bit."),
        ("|", "OR bit a bit."), ("^", "XOR en entrada directa."),
        ("<", "Desplaza a la izquierda (Lsh)."), ("~", "NOT bit a bit."), (";", "Parte entera (Int)."),
    ],
}

CONTROL_ROWS = {
    "en": [
        ("F2", "Select Degrees."), ("F4", "Select Grads."), ("F5", "Select Hex."),
        ("F6", "Select Radians; otherwise Dec."), ("F7", "Select Oct."), ("F8", "Select Bin."),
        ("Insert", "Enter Statistics data (Dat)."), ("Ctrl+L", "Clear memory (MC)."),
        ("Ctrl+R", "Recall memory (MR)."), ("Ctrl+M", "Store in memory (MS)."),
        ("Ctrl+P", "Add to memory (M+)."), ("Ctrl+S", "Open Statistics (Sta)."),
        ("Ctrl+A", "Calculate average (Ave)."), ("Ctrl+T", "Calculate sum (Sum)."),
        ("Ctrl+D", "Calculate deviation (s)."), ("Ctrl+Insert", "Copy the display."),
        ("Shift+Insert", "Paste a complete expression."),
    ],
    "pt": [
        ("F2", "Seleciona Graus."), ("F4", "Seleciona Grads."), ("F5", "Seleciona Hex."),
        ("F6", "Seleciona Radianos; nas bases, Dec."), ("F7", "Seleciona Oct."), ("F8", "Seleciona Bin."),
        ("Insert", "Registra dados (Dat)."), ("Ctrl+L", "Limpa a memória (MC)."),
        ("Ctrl+R", "Recupera a memória (MR)."), ("Ctrl+M", "Armazena na memória (MS)."),
        ("Ctrl+P", "Soma à memória (M+)."), ("Ctrl+S", "Abre Estatística (Sta)."),
        ("Ctrl+A", "Calcula a média (Ave)."), ("Ctrl+T", "Calcula a soma (Sum)."),
        ("Ctrl+D", "Calcula o desvio (s)."), ("Ctrl+Insert", "Copia o visor."),
        ("Shift+Insert", "Cola uma expressão completa."),
    ],
    "es": [
        ("F2", "Selecciona Grados."), ("F4", "Selecciona Grads."), ("F5", "Selecciona Hex."),
        ("F6", "Selecciona Radianes; en bases, Dec."), ("F7", "Selecciona Oct."), ("F8", "Selecciona Bin."),
        ("Insert", "Registra datos (Dat)."), ("Ctrl+L", "Borra la memoria (MC)."),
        ("Ctrl+R", "Recupera la memoria (MR)."), ("Ctrl+M", "Guarda en memoria (MS)."),
        ("Ctrl+P", "Suma a la memoria (M+)."), ("Ctrl+S", "Abre Estadística (Sta)."),
        ("Ctrl+A", "Calcula el promedio (Ave)."), ("Ctrl+T", "Calcula la suma (Sum)."),
        ("Ctrl+D", "Calcula la desviación (s)."), ("Ctrl+Insert", "Copia la pantalla."),
        ("Shift+Insert", "Pega una expresión completa."),
    ],
}

LANGUAGE = {
    "CALC_EN.HLP": ("en", ("Operator", "Description"), ("Key", "Description")),
    "CALC_PT-BR.HLP": ("pt", ("Operador", "Descrição"), ("Tecla", "Descrição")),
    "CALC_ES.HLP": ("es", ("Operador", "Descripción"), ("Tecla", "Descripción")),
}


def table_after_heading(records: list[h.Record], heading: str) -> int:
    for i, record in enumerate(records[:-1]):
        if heading in h.record_text(record) and records[i + 1].record_type == 0x23:
            return i + 1
    raise ValueError(f"table after heading not found: {heading}")


def rebuild_manual(path: Path) -> dict[str, int]:
    language, operator_headers, key_headers = LANGUAGE[path.name]
    data = bytearray(path.read_bytes())
    entries = {entry.name: entry for entry in h.parse_directory(data)}
    topic_entry = entries["|TOPIC"]
    original_records, _ = h.parse_topic_records(h.internal_file(data, topic_entry.file_offset))

    operator_index = next(
        i for i, record in enumerate(original_records)
        if record.record_type == 0x23 and any(x in h.record_text(record) for x in ("Operator Description", "Operador Descrição", "Operador Descripción"))
    )
    headings = {
        "en": ("Basic and editing keys", "Scientific keys", "Function and control shortcuts"),
        "pt": ("Teclas básicas e de edição", "Teclas científicas", "Atalhos de função e controle"),
        "es": ("Teclas básicas y de edición", "Teclas científicas", "Atajos de función y control"),
    }[language]
    basic_index = table_after_heading(original_records, headings[0])
    scientific_index = table_after_heading(original_records, headings[1])
    control_index = table_after_heading(original_records, headings[2])

    replacements = {
        operator_index: (operator_headers, OPERATOR_ROWS[language], (88, 652)),
        basic_index: (key_headers, BASIC_ROWS[language], (125, 615)),
        scientific_index: (key_headers, SCIENTIFIC_ROWS[language], (100, 640)),
        control_index: (key_headers, CONTROL_ROWS[language], (130, 610)),
    }

    records: list[h.Record] = []
    for i, record in enumerate(original_records):
        if i not in replacements:
            records.append(record)
            continue
        headers, rows, widths = replacements[i]
        padded_headers = tuple(padded(x) for x in headers)
        padded_rows = [(padded(k), padded(d)) for k, d in rows]
        rebuilt = h.build_table(record, padded_headers, padded_rows, record.identity, widths=widths)
        rebuilt.old_index = record.old_index
        rebuilt.old_pos = record.old_pos
        rebuilt.old_size = record.old_size
        rebuilt.gap_after = record.gap_after
        records.append(rebuilt)

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
        rec.old_pos for rec in checked_records
        if rec.old_pos is not None
        and (rec.old_pos - h.TOPIC_HEADER_SIZE) % h.TOPIC_DATA_SIZE + h.TOPIC_LINK_HEADER_SIZE > h.TOPIC_DATA_SIZE
    ]
    if crossing:
        raise ValueError(f"{path}: crossing TOPICLINK headers: {crossing}")
    return {
        "records": len(checked_records),
        "topics": sum(rec.record_type == 0x21 for rec in checked_records),
        "blocks": checked_topic.used // h.TOPIC_BLOCK_SIZE,
        "bytes": len(rebuilt_bytes),
    }


def main() -> None:
    # Multi-row WinHelp tables are independent column flows and can drift when
    # either side wraps. Keep this historical content module, but route command-
    # line regeneration through the row-safe generator.
    from rebuild_help_row_tables import main as rebuild_row_tables_main
    rebuild_row_tables_main()


if __name__ == "__main__":
    main()
