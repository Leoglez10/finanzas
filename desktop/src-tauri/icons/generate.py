"""Generate the source icon for pnpm tauri icon."""
import struct
import zlib
from pathlib import Path

size = 1024
rows = []
for y in range(size):
    row = bytearray()
    for x in range(size):
        # A block-letter F, centered on the petroleum background.
        white = (270 <= x < 375 and 200 <= y < 824) or (375 <= x < 755 and 200 <= y < 305) or (375 <= x < 670 and 450 <= y < 555)
        row.extend((255, 255, 255, 255) if white else (21, 94, 117, 255))
    rows.append(b"\0" + row)

def chunk(kind, data):
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))

png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0))
png += chunk(b"IDAT", zlib.compress(b"".join(rows), 9)) + chunk(b"IEND", b"")
Path(__file__).with_name("source.png").write_bytes(png)
