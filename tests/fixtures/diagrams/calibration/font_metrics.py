#!/usr/bin/env python3
"""Print the Noto Sans cap height and units per em from its TrueType tables."""
import pathlib
import struct

font = pathlib.Path(__file__).resolve().parents[3] / "fonts/NotoSans-Regular.ttf"
data = font.read_bytes()
count = struct.unpack_from(">H", data, 4)[0]
tables = {}
for index in range(count):
    tag, _, offset, _ = struct.unpack_from(">4sIII", data, 12 + 16 * index)
    tables[tag] = offset
units = struct.unpack_from(">H", data, tables[b"head"] + 18)[0]
cap = struct.unpack_from(">h", data, tables[b"OS/2"] + 88)[0]
print(f"cap height: {cap}, units per em: {units}, ratio: {cap / units:.3f}")
