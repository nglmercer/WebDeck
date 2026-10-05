"""Render the WebDeck diamond mark without third-party build dependencies."""
import struct
import sys
import zlib

size = 1024
rows = bytearray()
for y in range(size):
    rows.append(0)
    for x in range(size):
        distance = abs(x - size / 2) + abs(y - size / 2)
        rows.extend((240, 20, 125, 255) if 230 <= distance <= 320 else (17, 19, 26, 255))


def chunk(kind, payload):
    return struct.pack('!I', len(payload)) + kind + payload + struct.pack('!I', zlib.crc32(kind + payload))


png = b'\x89PNG\r\n\x1a\n'
png += chunk(b'IHDR', struct.pack('!2I5B', size, size, 8, 6, 0, 0, 0))
png += chunk(b'IDAT', zlib.compress(rows)) + chunk(b'IEND', b'')
with open(sys.argv[1], 'wb') as output:
    output.write(png)
