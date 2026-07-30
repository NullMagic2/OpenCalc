#!/usr/bin/env python3
"""Rebuild OpenCalc's compiled WinHelp reference tables in place.

This tool intentionally targets the uncompressed Windows 95 HLP manuals shipped
with OpenCalc. It replaces the grouped operator/keyboard tables with true
operator-or-key / description lookup tables, repacks |TOPIC, translates all
TOPICOFFSET navigation references, and shifts later HLP streams when |TOPIC
requires additional physical blocks.
"""
from __future__ import annotations

from dataclasses import dataclass, replace
from pathlib import Path
import argparse
import math
import struct
from typing import Iterable

HLP_MAGIC = 0x00035F3F
TOPIC_HEADER_SIZE = 12
TOPIC_LINK_HEADER_SIZE = 21
TOPIC_BLOCK_SIZE = 4096
TOPIC_DATA_SIZE = 4084
BTREE_HEADER_SIZE = 38


def u16(data: bytes | bytearray, off: int) -> int:
    return struct.unpack_from("<H", data, off)[0]


def i16(data: bytes | bytearray, off: int) -> int:
    return struct.unpack_from("<h", data, off)[0]


def u32(data: bytes | bytearray, off: int) -> int:
    return struct.unpack_from("<I", data, off)[0]


def i32(data: bytes | bytearray, off: int) -> int:
    return struct.unpack_from("<i", data, off)[0]


def p16(value: int) -> bytes:
    return struct.pack("<H", value)


def pi16(value: int) -> bytes:
    return struct.pack("<h", value)


def p32(value: int) -> bytes:
    return struct.pack("<I", value)


def pi32(value: int) -> bytes:
    return struct.pack("<i", value)


def encode_unsigned_short(value: int) -> bytes:
    if not 0 <= value <= 32767:
        raise ValueError(f"compressed unsigned short out of range: {value}")
    if value < 128:
        return bytes([value * 2])
    return bytes([(value % 128) * 2 + 1, value // 128])


def decode_unsigned_short(data: bytes | bytearray, off: int) -> tuple[int, int]:
    first = data[off]
    if first & 1 == 0:
        return first // 2, off + 1
    return first // 2 + data[off + 1] * 128, off + 2


def encode_signed_long(value: int) -> bytes:
    if -16384 <= value <= 16383:
        return p16((value + 16384) * 2)
    raw = (value + 1073741824) * 2 + 1
    return struct.pack("<I", raw & 0xFFFFFFFF)


def decode_signed_long(data: bytes | bytearray, off: int) -> tuple[int, int]:
    first = u16(data, off)
    if first & 1 == 0:
        return first // 2 - 16384, off + 2
    second = u16(data, off + 2)
    return first // 2 + second * 32768 - 1073741824, off + 4


@dataclass
class DirectoryEntry:
    name: str
    file_offset: int
    offset_field_abs: int


@dataclass
class InternalFile:
    offset: int
    reserved: int
    used: int
    flags: int
    content_start: int
    content: bytes


@dataclass
class Record:
    identity: str
    old_index: int | None
    old_pos: int | None
    old_size: int
    record_type: int
    ld1: bytes
    ld2: bytes
    gap_after: int = 0
    new_pos: int = 0

    @property
    def size(self) -> int:
        return TOPIC_LINK_HEADER_SIZE + len(self.ld1) + len(self.ld2)

    def bytes(self, previous: int, following: int) -> bytes:
        data_len1 = TOPIC_LINK_HEADER_SIZE + len(self.ld1)
        return (
            pi32(self.size)
            + pi32(len(self.ld2))
            + pi32(previous)
            + pi32(following)
            + pi32(data_len1)
            + bytes([self.record_type])
            + self.ld1
            + self.ld2
        )


def internal_file(data: bytes | bytearray, offset: int) -> InternalFile:
    reserved = u32(data, offset)
    used = u32(data, offset + 4)
    flags = data[offset + 8]
    start = offset + 9
    return InternalFile(offset, reserved, used, flags, start, bytes(data[start : start + used]))


def parse_directory(data: bytes | bytearray) -> list[DirectoryEntry]:
    directory_start = u32(data, 4)
    directory = internal_file(data, directory_start)
    content = directory.content
    if u16(content, 0) != 0x293B:
        raise ValueError("invalid HLP directory B-tree")
    page_size = u16(content, 4)
    root_page = i16(content, 26)
    levels = i16(content, 32)
    expected = u32(content, 34)

    def page_abs(page_number: int) -> int:
        return directory.content_start + BTREE_HEADER_SIZE + page_number * page_size

    page_number = root_page
    for _ in range(1, levels):
        page_number = i16(data, page_abs(page_number) + 4)

    entries: list[DirectoryEntry] = []
    visited: set[int] = set()
    while page_number >= 0:
        if page_number in visited:
            raise ValueError("directory leaf cycle")
        visited.add(page_number)
        base = page_abs(page_number)
        unused = u16(data, base)
        count = i16(data, base + 2)
        next_page = i16(data, base + 6)
        pos = base + 8
        logical_end = base + page_size - unused
        for _ in range(count):
            end = data.index(0, pos, logical_end)
            name = bytes(data[pos:end]).decode("latin1")
            pos = end + 1
            field = pos
            value = u32(data, field)
            pos += 4
            entries.append(DirectoryEntry(name, value, field))
        page_number = next_page
    if len(entries) != expected:
        raise ValueError(f"directory entries {len(entries)} != {expected}")
    return entries


def logical_read(blocks: list[bytes], position: int, length: int) -> bytes:
    adjusted = position - TOPIC_HEADER_SIZE
    block = adjusted // TOPIC_DATA_SIZE
    offset = adjusted % TOPIC_DATA_SIZE
    output = bytearray()
    while length:
        available = len(blocks[block]) - offset
        take = min(length, available)
        output.extend(blocks[block][offset : offset + take])
        length -= take
        block += 1
        offset = 0
    return bytes(output)


def parse_topic_records(topic: InternalFile) -> tuple[list[Record], list[tuple[int, int, int]]]:
    if topic.flags != 0 or topic.used % TOPIC_BLOCK_SIZE != 0:
        raise ValueError("this generator supports uncompressed 4096-byte |TOPIC blocks only")
    blocks: list[bytes] = []
    headers: list[tuple[int, int, int]] = []
    for start in range(0, topic.used, TOPIC_BLOCK_SIZE):
        chunk = topic.content[start : start + TOPIC_BLOCK_SIZE]
        headers.append(struct.unpack_from("<iii", chunk, 0))
        blocks.append(chunk[TOPIC_HEADER_SIZE:])
    candidates = sorted({header[1] for header in headers if header[1] >= 12})
    if not candidates:
        raise ValueError("no first TOPICLINK")
    position = candidates[0]
    raw_records: list[Record] = []
    seen: set[int] = set()
    index = 0
    while position >= 12:
        if position in seen:
            raise ValueError("TOPICLINK cycle")
        seen.add(position)
        block_offset = (position - 12) % TOPIC_DATA_SIZE
        if block_offset + TOPIC_LINK_HEADER_SIZE > TOPIC_DATA_SIZE:
            raise ValueError(f"TOPICLINK header crosses block at {position}")
        header = logical_read(blocks, position, TOPIC_LINK_HEADER_SIZE)
        block_size, data_len2, _previous, following, data_len1, record_type = struct.unpack(
            "<iiiiiB", header
        )
        raw = logical_read(blocks, position, block_size)
        ld1 = raw[TOPIC_LINK_HEADER_SIZE:data_len1]
        ld2 = raw[data_len1:block_size]
        raw_records.append(
            Record(
                identity=f"old:{index}",
                old_index=index,
                old_pos=position,
                old_size=block_size,
                record_type=record_type,
                ld1=ld1,
                ld2=ld2,
            )
        )
        index += 1
        if following <= 0:
            break
        position = following
    for current, following in zip(raw_records, raw_records[1:]):
        assert current.old_pos is not None and following.old_pos is not None
        current.gap_after = max(0, following.old_pos - (current.old_pos + current.old_size))
    return raw_records, headers


def parse_table_templates(record: Record) -> tuple[bytes, bytes, bytes, bytes]:
    data = record.ld1
    _size, pos = decode_signed_long(data, 0)
    _length, pos = decode_unsigned_short(data, pos)
    column_count = data[pos]
    table_type = data[pos + 1]
    pos += 2
    if table_type == 0:
        pos += 2
    pos += column_count * 4
    payloads: list[bytes] = []
    while True:
        column = i16(data, pos)
        pos += 2
        if column == -1:
            break
        record_type = data[pos]
        pos += 1
        payload_size, pos = decode_signed_long(data, pos)
        if record_type > 0x10:
            _topic_length, pos = decode_unsigned_short(data, pos)
        payloads.append(data[pos : pos + payload_size])
        pos += payload_size
    if len(payloads) < 4:
        raise ValueError("table template has fewer than four cells")
    return payloads[0], payloads[1], payloads[2], payloads[3]


def table_topic_length(headers: tuple[str, str], rows: list[tuple[str, str]]) -> int:
    total = sum(len(text.encode("cp1252")) + 1 for text in headers)
    total += sum(len(a.encode("cp1252")) + len(b.encode("cp1252")) + 2 for a, b in rows)
    return min(total, 32767)


def build_table(
    template: Record,
    headers: tuple[str, str],
    rows: list[tuple[str, str]],
    identity: str,
    widths: tuple[int, int] = (120, 620),
) -> Record:
    header_left, header_right, key_cell, description_cell = parse_table_templates(template)
    ld1 = bytearray()
    ld1.extend(encode_signed_long(16))
    ld1.extend(encode_unsigned_short(table_topic_length(headers, rows)))
    ld1.extend(bytes([2, 1]))  # two columns, absolute geometry
    ld1.extend(p16(widths[0]) + p16(0))
    ld1.extend(p16(widths[1]) + p16(0))
    ld2 = bytearray()

    def append_cell(column: int, payload: bytes, text: str) -> None:
        encoded = text.encode("cp1252")
        ld1.extend(pi16(column))
        ld1.append(0x20)
        ld1.extend(encode_signed_long(len(payload)))
        ld1.extend(encode_unsigned_short(0))
        ld1.extend(payload)
        # Every copied cell template starts with an empty string, changes font,
        # then consumes the visible string before its 0xFF terminator.
        ld2.extend(b"\0" + encoded + b"\0")

    append_cell(0, header_left, headers[0])
    append_cell(1, header_right, headers[1])
    for key, description in rows:
        append_cell(0, key_cell, key)
        append_cell(1, description_cell, description)
    ld1.extend(pi16(-1))
    return Record(identity, None, None, 0, 0x23, bytes(ld1), bytes(ld2), 0)


def clone_heading(template: Record, text: str, identity: str) -> Record:
    return Record(
        identity=identity,
        old_index=None,
        old_pos=None,
        old_size=0,
        record_type=template.record_type,
        ld1=template.ld1,
        ld2=b"\0" + text.encode("cp1252") + b"\0",
        gap_after=0,
    )


def record_text(record: Record) -> str:
    return " ".join(
        part.decode("cp1252", "replace") for part in record.ld2.split(b"\0") if part
    )


def parse_topic_length(ld1: bytes) -> int:
    _size, pos = decode_signed_long(ld1, 0)
    length, _pos = decode_unsigned_short(ld1, pos)
    return length


def assign_positions(records: list[Record]) -> None:
    cursor = 12
    for index, record in enumerate(records):
        if index == 0:
            cursor = 12
        offset = (cursor - 12) % TOPIC_DATA_SIZE
        if offset + TOPIC_LINK_HEADER_SIZE > TOPIC_DATA_SIZE:
            cursor += TOPIC_DATA_SIZE - offset
        record.new_pos = cursor
        cursor += record.size + record.gap_after


def patch_topic_positions(records: list[Record]) -> None:
    old_pairs = sorted(
        (record.old_pos, record.new_pos)
        for record in records
        if record.old_pos is not None
    )

    def translate_topic_pos(value: int) -> int:
        if value < 12:
            return value
        chosen: tuple[int, int] | None = None
        for old_position, new_position in old_pairs:
            if old_position <= value:
                chosen = (old_position, new_position)
            else:
                break
        if chosen is None:
            return value
        return chosen[1] + (value - chosen[0])

    headers = [record for record in records if record.record_type == 0x21]
    final_end = records[-1].new_pos + records[-1].size
    for index, record in enumerate(headers):
        if len(record.ld1) < 28:
            continue
        data = bytearray(record.ld1)
        next_header = headers[index + 1].new_pos if index + 1 < len(headers) else -1
        topic_end = next_header if next_header >= 12 else final_end
        struct.pack_into("<i", data, 0, topic_end - record.new_pos)
        for field in (16, 20):
            old = i32(data, field)
            if old >= 12:
                struct.pack_into("<i", data, field, translate_topic_pos(old))
        struct.pack_into("<i", data, 24, next_header)
        record.ld1 = bytes(data)


def build_anchors(records: Iterable[Record], use_old: bool) -> tuple[dict[str, int], dict[int, list[tuple[int, str]]]]:
    block_counts: dict[int, int] = {}
    by_identity: dict[str, int] = {}
    by_block: dict[int, list[tuple[int, str]]] = {}
    for record in records:
        if record.record_type not in (0x20, 0x23):
            continue
        position = record.old_pos if use_old else record.new_pos
        if position is None:
            continue
        block = (position - 12) // TOPIC_DATA_SIZE
        count = block_counts.get(block, 0)
        offset = block * 32768 + count
        by_identity[record.identity] = offset
        by_block.setdefault(block, []).append((offset, record.identity))
        block_counts[block] = count + parse_topic_length(record.ld1)
    return by_identity, by_block


def topic_offset_translator(original: list[Record], rebuilt: list[Record]):
    old_by_id, old_by_block = build_anchors(original, True)
    new_by_id, _new_by_block = build_anchors(rebuilt, False)

    def translate(value: int) -> int:
        if value < 0:
            return value
        block = value // 32768
        candidates = old_by_block.get(block, [])
        chosen: tuple[int, str] | None = None
        for offset, identity in candidates:
            if offset <= value:
                chosen = (offset, identity)
            else:
                break
        if chosen is None:
            return value
        old_anchor, identity = chosen
        new_anchor = new_by_id.get(identity)
        if new_anchor is None:
            return value
        return new_anchor + (value - old_anchor)

    return translate


def patch_topic_offsets(records: list[Record], translate) -> None:
    for record in records:
        data = bytearray(record.ld1)
        if record.record_type == 0x21 and len(data) >= 12:
            for field in (4, 8):
                value = i32(data, field)
                if value >= 0:
                    struct.pack_into("<i", data, field, translate(value))
        # Direct internal links and the two ordinary external-link forms used
        # by these manuals. Only patch a candidate when translation changes it.
        pos = 0
        while pos < len(data):
            opcode = data[pos]
            if opcode in (0xE0, 0xE1) and pos + 5 <= len(data):
                value = i32(data, pos + 1)
                changed = translate(value)
                if changed != value:
                    struct.pack_into("<i", data, pos + 1, changed)
                pos += 5
                continue
            if opcode in (0xEA, 0xEB, 0xEE, 0xEF) and pos + 8 <= len(data):
                total = i16(data, pos + 1)
                if total >= 5 and pos + 3 + total <= len(data):
                    value = i32(data, pos + 4)
                    changed = translate(value)
                    if changed != value:
                        struct.pack_into("<i", data, pos + 4, changed)
                    pos += 3 + total
                    continue
            pos += 1
        record.ld1 = bytes(data)


def rebuild_topic_stream(records: list[Record]) -> bytes:
    final_end = max(record.new_pos + record.size for record in records)
    block_count = max(1, math.ceil((final_end - 12) / TOPIC_DATA_SIZE))
    blocks = [bytearray(TOPIC_DATA_SIZE) for _ in range(block_count)]
    for index, record in enumerate(records):
        previous = records[index - 1].new_pos if index else -1
        following = records[index + 1].new_pos if index + 1 < len(records) else -1
        payload = record.bytes(previous, following)
        adjusted = record.new_pos - 12
        block = adjusted // TOPIC_DATA_SIZE
        offset = adjusted % TOPIC_DATA_SIZE
        remaining = memoryview(payload)
        while remaining:
            take = min(len(remaining), TOPIC_DATA_SIZE - offset)
            blocks[block][offset : offset + take] = remaining[:take]
            remaining = remaining[take:]
            block += 1
            offset = 0

    header_positions = [record.new_pos for record in records if record.record_type == 0x21]
    record_positions = [record.new_pos for record in records]
    output = bytearray()
    for block_index, block_data in enumerate(blocks):
        start = 12 + block_index * TOPIC_DATA_SIZE
        end = start + TOPIC_DATA_SIZE
        previous_links = [position for position in record_positions if position < start]
        in_block = [position for position in record_positions if start <= position < end]
        previous_headers = [position for position in header_positions if position < start]
        last_link = previous_links[-1] if previous_links else -1
        # A split table can fill most of a block. If no TOPICLINK header starts
        # here, advertise the next linked record; the traversal still follows
        # the prior record's Next pointer and never treats payload as a header.
        future = [position for position in record_positions if position >= start]
        first_link = in_block[0] if in_block else (future[0] if future else -1)
        last_header = previous_headers[-1] if previous_headers else 0
        output.extend(struct.pack("<iii", last_link, first_link, last_header))
        output.extend(block_data)
    return bytes(output)


def navigation_leaf_pages(data: bytes | bytearray, stream: InternalFile) -> list[int]:
    content = stream.content
    if u16(content, 0) != 0x293B:
        raise ValueError("invalid navigation B-tree")
    page_size = u16(content, 4)
    root = i16(content, 26)
    levels = i16(content, 32)

    def page_abs(number: int) -> int:
        return stream.content_start + BTREE_HEADER_SIZE + number * page_size

    page = root
    for _ in range(1, levels):
        page = i16(data, page_abs(page) + 4)
    result: list[int] = []
    visited: set[int] = set()
    while page >= 0:
        if page in visited:
            raise ValueError("navigation B-tree cycle")
        visited.add(page)
        result.append(page_abs(page))
        page = i16(data, page_abs(page) + 6)
    return result


def patch_navigation_streams(data: bytearray, entries: dict[str, DirectoryEntry], translate) -> None:
    for name in ("|CONTEXT", "|TopicId"):
        entry = entries.get(name)
        if entry is None:
            continue
        stream = internal_file(data, entry.file_offset)
        page_size = u16(stream.content, 4)
        for base in navigation_leaf_pages(data, stream):
            unused = u16(data, base)
            count = i16(data, base + 2)
            pos = base + 8
            logical_end = base + page_size - unused
            for _ in range(count):
                if name == "|CONTEXT":
                    pos += 4  # hash key
                    value = i32(data, pos)
                    struct.pack_into("<i", data, pos, translate(value))
                    pos += 4
                else:
                    value = i32(data, pos)
                    struct.pack_into("<i", data, pos, translate(value))
                    pos += 4
                    end = data.index(0, pos, logical_end)
                    pos = end + 1


def patch_system_contents(data: bytearray, entry: DirectoryEntry, translate) -> None:
    stream = internal_file(data, entry.file_offset)
    pos = stream.content_start + 12
    end = stream.content_start + stream.used
    while pos < end:
        record_type = u16(data, pos)
        size = u16(data, pos + 2)
        payload = pos + 4
        if record_type == 3 and size >= 4:
            value = i32(data, payload)
            struct.pack_into("<i", data, payload, translate(value))
        pos = payload + size


def shift_directory_offsets(data: bytearray, old_topic_end: int, delta: int) -> None:
    # The directory itself moved with the trailing streams, so update the HLP
    # header first and then rediscover the relocated leaf fields.
    struct.pack_into("<I", data, 4, u32(data, 4) + delta)
    struct.pack_into("<I", data, 12, u32(data, 12) + delta)
    for entry in parse_directory(data):
        if entry.file_offset >= old_topic_end:
            struct.pack_into("<I", data, entry.offset_field_abs, entry.file_offset + delta)


def replace_topic_file(data: bytearray, topic_entry: DirectoryEntry, topic_stream: bytes) -> bytearray:
    old = internal_file(data, topic_entry.file_offset)
    old_end = old.offset + old.reserved
    header = p32(len(topic_stream) + 9) + p32(len(topic_stream)) + bytes([old.flags])
    replacement = header + topic_stream
    delta = len(replacement) - old.reserved
    logical_size = u32(data, 12)
    rebuilt = bytearray(data[: old.offset] + replacement + data[old_end:logical_size])
    shift_directory_offsets(rebuilt, old_end, delta)
    return rebuilt


OPERATOR_ROWS = {
    "en": [
        ("+", "Addition; unary plus."),
        ("-", "Subtraction; unary minus starts a negative value."),
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
        ("=", "Optional trailing = sign."),
    ],
    "pt": [
        ("+", "Adição; sinal positivo unário."),
        ("-", "Subtração; o sinal unário inicia um valor negativo."),
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
        ("lsh", "Desloca bits à esquerda."),
        ("root", "x root y = x^(1/y)."),
        ("(", "Abre expressão agrupada."),
        (")", "Fecha expressão agrupada."),
        ("=", "Igual final opcional."),
    ],
    "es": [
        ("+", "Suma; signo positivo unario."),
        ("-", "Resta; el signo unario inicia un valor negativo."),
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
        ("lsh", "Desplaza bits a la izq."),
        ("root", "x root y = x^(1/y)."),
        ("(", "Abre expresión agrupada."),
        (")", "Cierra expresión."),
        ("=", "Igual final opcional."),
    ],
}

BASIC_ROWS = {
    "en": [
        ("0-9", "Enter numeric digits."),
        ("A-F", "Enter hexadecimal digits in Hex mode."),
        (". or ,", "Enter the decimal separator."),
        ("+", "Addition."),
        ("-", "Subtraction."),
        ("*", "Multiplication. Type * again for **."),
        ("**", "Exponentiation."),
        ("×", "Unicode alias for multiplication."),
        ("/", "Division."),
        ("÷", "Unicode alias for division."),
        ("%", "Percent in Standard Mode; Mod in Scientific Mode."),
        ("= or Enter", "Calculate the result."),
        ("Backspace or Left", "Delete the last digit (Back)."),
        ("Delete", "Clear the current entry (CE)."),
        ("Esc", "Clear the calculation (C)."),
        ("F9", "Toggle the sign (+/-)."),
        ("(", "Open a parenthesis in Scientific Mode."),
        (")", "Close a parenthesis in Scientific Mode."),
        ("@", "Square root in Standard Mode; x^2 in Scientific Mode."),
    ],
    "pt": [
        ("0-9", "Digita algarismos numéricos."),
        ("A-F", "Digita algarismos hexadecimais no modo Hex."),
        (". ou ,", "Digita o separador decimal."),
        ("+", "Adição."),
        ("-", "Subtração."),
        ("*", "Multiplicação. Digite * outra vez para **."),
        ("**", "Exponenciação."),
        ("×", "Alias Unicode de multiplicação."),
        ("/", "Divisão."),
        ("÷", "Alias Unicode de divisão."),
        ("%", "Porcentagem no Modo Padrão; Mod no Científico."),
        ("= ou Enter", "Calcula o resultado."),
        ("Backspace ou Esquerda", "Apaga o último dígito (Back)."),
        ("Delete", "Limpa a entrada atual (CE)."),
        ("Esc", "Limpa o cálculo (C)."),
        ("F9", "Alterna o sinal (+/-)."),
        ("(", "Abre parêntese no Modo Científico."),
        (")", "Fecha parêntese no Modo Científico."),
        ("@", "Raiz quadrada no Modo Padrão; x^2 no Modo Científico."),
    ],
    "es": [
        ("0-9", "Introduce dígitos numéricos."),
        ("A-F", "Introduce dígitos hexadecimales en modo Hex."),
        (". o ,", "Introduce el separador decimal."),
        ("+", "Suma."),
        ("-", "Resta."),
        ("*", "Multiplicación. Escriba * otra vez para **."),
        ("**", "Exponenciación."),
        ("×", "Alias Unicode de multiplicación."),
        ("/", "División."),
        ("÷", "Alias Unicode de división."),
        ("%", "Porcentaje en Modo Estándar; Mod en Modo Científico."),
        ("= o Enter", "Calcula el resultado."),
        ("Retroceso o Izq.", "Borra el último dígito (Back)."),
        ("Supr", "Borra la entrada actual (CE)."),
        ("Esc", "Borra el cálculo (C)."),
        ("F9", "Signo +/-."),
        ("(", "Abre un paréntesis en Modo Científico."),
        (")", "Cierra un paréntesis en Modo Científico."),
        ("@", "Raíz cuadrada en Modo Estándar; x^2 en Modo Científico."),
    ],
}

SCIENTIFIC_ROWS = {
    "en": [
        ("!", "Factorial."),
        ("#", "Cube the displayed value (x^3)."),
        ("r", "Reciprocal operation (1/x)."),
        ("s", "Sine."),
        ("o", "Cosine."),
        ("t", "Tangent."),
        ("n", "Natural logarithm (ln)."),
        ("l", "Base-10 logarithm (log)."),
        ("m", "Degrees-minutes-seconds conversion (dms)."),
        ("x", "Enter exponent notation (Exp)."),
        ("y", "Power operation (x^y)."),
        ("p", "Insert pi."),
        ("i", "Toggle Inv."),
        ("h", "Toggle Hyp."),
        ("v", "Toggle F-E."),
        ("&", "Bitwise AND."),
        ("|", "Bitwise OR."),
        ("^", "Bitwise XOR; it is not exponentiation in direct keyboard mode."),
        ("<", "Shift bits left (Lsh)."),
        ("~", "Bitwise NOT."),
        (";", "Integer part (Int)."),
    ],
    "pt": [
        ("!", "Fatorial."),
        ("#", "Eleva o valor exibido ao cubo (x^3)."),
        ("r", "Operação recíproca (1/x)."),
        ("s", "Seno."),
        ("o", "Cosseno."),
        ("t", "Tangente."),
        ("n", "Logaritmo natural (ln)."),
        ("l", "Logaritmo decimal (log)."),
        ("m", "Conversão graus-minutos-segundos (dms)."),
        ("x", "Digita notação exponencial (Exp)."),
        ("y", "Operação de potência (x^y)."),
        ("p", "Insere pi."),
        ("i", "Ativa ou desativa Inv."),
        ("h", "Ativa ou desativa Hyp."),
        ("v", "Ativa ou desativa F-E."),
        ("&", "E bit a bit."),
        ("|", "OU bit a bit."),
        ("^", "OU exclusivo bit a bit; não é exponenciação na digitação direta."),
        ("<", "Desloca bits à esquerda (Lsh)."),
        ("~", "NÃO bit a bit."),
        (";", "Parte inteira (Int)."),
    ],
    "es": [
        ("!", "Factorial."),
        ("#", "Eleva al cubo el valor mostrado (x^3)."),
        ("r", "Operación recíproca (1/x)."),
        ("s", "Seno."),
        ("o", "Coseno."),
        ("t", "Tangente."),
        ("n", "Logaritmo natural (ln)."),
        ("l", "Logaritmo decimal (log)."),
        ("m", "Conversión grados-minutos-segundos (dms)."),
        ("x", "Introduce notación exponencial (Exp)."),
        ("y", "Operación de potencia (x^y)."),
        ("p", "Inserta pi."),
        ("i", "Activa o desactiva Inv."),
        ("h", "Activa o desactiva Hyp."),
        ("v", "Activa o desactiva F-E."),
        ("&", "AND bit a bit."),
        ("|", "OR bit a bit."),
        ("^", "XOR bit a bit; no es exponenciación en la entrada directa."),
        ("<", "Desplaza bits a la izquierda (Lsh)."),
        ("~", "NOT bit a bit."),
        (";", "Parte entera (Int)."),
    ],
}

CONTROL_ROWS = {
    "en": [
        ("F2", "Select Degrees in decimal mode."),
        ("F4", "Select Grads in decimal mode."),
        ("F5", "Select Hex."),
        ("F6", "Select Radians in decimal mode; otherwise select Dec."),
        ("F7", "Select Oct."),
        ("F8", "Select Bin."),
        ("Insert", "Enter Statistics data (Dat) in Scientific Mode."),
        ("Ctrl+L", "Clear memory (MC)."),
        ("Ctrl+R", "Recall memory (MR)."),
        ("Ctrl+M", "Store in memory (MS)."),
        ("Ctrl+P", "Add to memory (M+)."),
        ("Ctrl+S", "Open Statistics (Sta)."),
        ("Ctrl+A", "Calculate average (Ave)."),
        ("Ctrl+T", "Calculate sum (Sum)."),
        ("Ctrl+D", "Calculate standard deviation (s)."),
        ("Ctrl+Insert", "Copy the display."),
        ("Shift+Insert", "Paste a complete expression."),
    ],
    "pt": [
        ("F2", "Seleciona Graus no modo decimal."),
        ("F4", "Seleciona Grads no modo decimal."),
        ("F5", "Seleciona Hex."),
        ("F6", "Seleciona Radianos no modo decimal; nas outras bases, seleciona Dec."),
        ("F7", "Seleciona Oct."),
        ("F8", "Seleciona Bin."),
        ("Insert", "Registra dados estatísticos (Dat) no Modo Científico."),
        ("Ctrl+L", "Limpa a memória (MC)."),
        ("Ctrl+R", "Recupera a memória (MR)."),
        ("Ctrl+M", "Armazena na memória (MS)."),
        ("Ctrl+P", "Soma à memória (M+)."),
        ("Ctrl+S", "Abre Estatística (Sta)."),
        ("Ctrl+A", "Calcula a média (Ave)."),
        ("Ctrl+T", "Calcula a soma (Sum)."),
        ("Ctrl+D", "Calcula o desvio padrão (s)."),
        ("Ctrl+Insert", "Copia o visor."),
        ("Shift+Insert", "Cola uma expressão completa."),
    ],
    "es": [
        ("F2", "Selecciona Grados en modo decimal."),
        ("F4", "Selecciona Grads en modo decimal."),
        ("F5", "Selecciona Hex."),
        ("F6", "Selecciona Radianes en modo decimal; en otras bases, selecciona Dec."),
        ("F7", "Selecciona Oct."),
        ("F8", "Selecciona Bin."),
        ("Insert", "Registra datos estadísticos (Dat) en Modo Científico."),
        ("Ctrl+L", "Borra la memoria (MC)."),
        ("Ctrl+R", "Recupera la memoria (MR)."),
        ("Ctrl+M", "Guarda en memoria (MS)."),
        ("Ctrl+P", "Suma a la memoria (M+)."),
        ("Ctrl+S", "Abre Estadística (Sta)."),
        ("Ctrl+A", "Calcula el promedio (Ave)."),
        ("Ctrl+T", "Calcula la suma (Sum)."),
        ("Ctrl+D", "Calcula la desviación estándar (s)."),
        ("Ctrl+Insert", "Copia la pantalla."),
        ("Shift+Insert", "Pega una expresión completa."),
    ],
}

LANGUAGE = {
    "CALC_EN.HLP": ("en", ("Operator", "Description"), ("Key", "Description"),
                    ("Basic and editing keys", "Scientific keys", "Function and control shortcuts")),
    "CALC_PT-BR.HLP": ("pt", ("Operador", "Descrição"), ("Tecla", "Descrição"),
                       ("Teclas básicas e de edição", "Teclas científicas", "Atalhos de função e controle")),
    "CALC_ES.HLP": ("es", ("Operador", "Descripción"), ("Tecla", "Descripción"),
                    ("Teclas básicas y de edición", "Teclas científicas", "Atajos de función y control")),
}


def rebuild_manual(path: Path) -> dict[str, int]:
    language, operator_headers, key_headers, headings = LANGUAGE[path.name]
    original_bytes = bytearray(path.read_bytes())
    if u32(original_bytes, 0) != HLP_MAGIC:
        raise ValueError(f"{path}: not a Windows HLP")
    directory_list = parse_directory(original_bytes)
    entries = {entry.name: entry for entry in directory_list}
    topic_entry = entries["|TOPIC"]
    topic = internal_file(original_bytes, topic_entry.file_offset)
    original_records, _headers = parse_topic_records(topic)

    # The migration is intentionally safe to run on an already rebuilt manual.
    # In that case, validate the expected lookup tables and leave the bytes alone.
    existing_text = b"\0".join(record.ld2 for record in original_records)
    factorial_marker = {
        "en": b"Factorial operation.",
        "pt": "Operação fatorial.".encode("cp1252"),
        "es": "Operación factorial.".encode("cp1252"),
    }[language]
    heading_markers = [heading.encode("cp1252") for heading in headings]
    if factorial_marker in existing_text and all(marker in existing_text for marker in heading_markers):
        stale = (b"Key groups", b"Grupos de teclas")
        if any(marker in existing_text for marker in stale):
            raise ValueError(f"{path}: mixed old/new keyboard tables")
        crossing = [
            record.old_pos for record in original_records
            if record.old_pos is not None
            and (record.old_pos - 12) % TOPIC_DATA_SIZE + TOPIC_LINK_HEADER_SIZE > TOPIC_DATA_SIZE
        ]
        if crossing:
            raise ValueError(f"{path}: crossing TOPICLINK headers {crossing}")
        return {
            "records": len(original_records),
            "topics": sum(record.record_type == 0x21 for record in original_records),
            "topic_blocks": topic.used // TOPIC_BLOCK_SIZE,
            "bytes": len(original_bytes),
        }

    operator_index = next(
        index for index, record in enumerate(original_records)
        if record.record_type == 0x23 and any(marker in record_text(record) for marker in (
            "Operator groups", "Grupos de operadores"
        ))
    )
    keyboard_index = next(
        index for index, record in enumerate(original_records)
        if record.record_type == 0x23 and (
            "Shortcut" in record_text(record) or "Atalho" in record_text(record) or "Atajo" in record_text(record)
        ) and any(key in record_text(record) for key in ("Ctrl+Ins", "Shift+Ins"))
    )
    heading_index = keyboard_index - 1
    heading_template = original_records[heading_index]
    keyboard_template = original_records[keyboard_index]

    records: list[Record] = []
    for index, record in enumerate(original_records):
        if index == operator_index:
            rebuilt = build_table(
                keyboard_template,
                operator_headers,
                OPERATOR_ROWS[language],
                identity=record.identity,
                widths=(75, 675),
            )
            rebuilt.old_index = record.old_index
            rebuilt.old_pos = record.old_pos
            rebuilt.old_size = record.old_size
            rebuilt.gap_after = record.gap_after
            records.append(rebuilt)
            continue
        if index == heading_index:
            replacement = replace(
                record,
                ld2=b"\0" + headings[0].encode("cp1252") + b"\0",
            )
            records.append(replacement)
            continue
        if index == keyboard_index:
            basic = build_table(record, key_headers, BASIC_ROWS[language], record.identity, widths=(95, 645))
            basic.old_index = record.old_index
            basic.old_pos = record.old_pos
            basic.old_size = record.old_size
            basic.gap_after = 0
            records.append(basic)
            records.append(clone_heading(heading_template, headings[1], f"new:{language}:scientific-heading"))
            records.append(build_table(keyboard_template, key_headers, SCIENTIFIC_ROWS[language], f"new:{language}:scientific-table", widths=(95, 645)))
            records.append(clone_heading(heading_template, headings[2], f"new:{language}:control-heading"))
            control = build_table(keyboard_template, key_headers, CONTROL_ROWS[language], f"new:{language}:control-table", widths=(95, 645))
            control.gap_after = record.gap_after
            records.append(control)
            continue
        # Remove only the seven compiler-padding display records immediately
        # after the former grouped keyboard table.
        if keyboard_index < index <= keyboard_index + 7 and record.record_type == 0x20 and record.size == 24 and not record.ld2:
            continue
        records.append(record)

    assign_positions(records)
    patch_topic_positions(records)
    translate = topic_offset_translator(original_records, records)
    patch_topic_offsets(records, translate)

    # Patch streams that live before |TOPIC while their absolute addresses are
    # still unchanged.
    patch_navigation_streams(original_bytes, entries, translate)
    patch_system_contents(original_bytes, entries["|SYSTEM"], translate)

    topic_stream = rebuild_topic_stream(records)
    rebuilt_bytes = replace_topic_file(original_bytes, topic_entry, topic_stream)
    path.write_bytes(rebuilt_bytes)

    # Structural postconditions.
    rebuilt_entries = {entry.name: entry for entry in parse_directory(rebuilt_bytes)}
    rebuilt_topic = internal_file(rebuilt_bytes, rebuilt_entries["|TOPIC"].file_offset)
    checked_records, _ = parse_topic_records(rebuilt_topic)
    all_text = b"\0".join(record.ld2 for record in checked_records)
    for forbidden in (b"Key groups", b"Grupos de teclas"):
        if forbidden in all_text:
            raise ValueError(f"{path}: stale grouped-key heading remains")
    expected = {
        "en": (b"\0!\0", b"Factorial operation."),
        "pt": (b"\0!\0", "Operação fatorial.".encode("cp1252")),
        "es": (b"\0!\0", "Operación factorial.".encode("cp1252")),
    }[language]
    if not all(part in all_text for part in expected):
        raise ValueError(f"{path}: key/description factorial row missing")
    crossing = [
        record.new_pos for record in records
        if (record.new_pos - 12) % TOPIC_DATA_SIZE + 21 > TOPIC_DATA_SIZE
    ]
    if crossing:
        raise ValueError(f"{path}: crossing TOPICLINK headers {crossing}")
    return {
        "records": len(checked_records),
        "topics": sum(record.record_type == 0x21 for record in checked_records),
        "topic_blocks": rebuilt_topic.used // TOPIC_BLOCK_SIZE,
        "bytes": len(rebuilt_bytes),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("help_directory", nargs="?", default="Help")
    args = parser.parse_args()
    directory = Path(args.help_directory)
    for filename in LANGUAGE:
        path = directory / filename
        result = rebuild_manual(path)
        print(f"{filename}: {result}")


if __name__ == "__main__":
    main()
