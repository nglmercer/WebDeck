/** Inspect the built Windows binaries; a GUI subsystem must not allocate a console. */
import assert from "node:assert/strict";
import { closeSync, openSync, readSync } from "node:fs";
import path from "node:path";

function subsystem(file: string): number {
  const fd = openSync(file, "r");
  function readAt(offset: number, length: number): Buffer {
    const buffer = Buffer.alloc(length);
    assert.equal(
      readSync(fd, buffer, 0, length, offset),
      length,
      `${file}: truncated PE header`,
    );
    return buffer;
  }
  try {
    const dos = readAt(0, 64);
    assert.equal(
      dos.toString("ascii", 0, 2),
      "MZ",
      `${file}: not a Windows executable`,
    );
    const pe = dos.readUInt32LE(0x3c);
    assert.equal(
      readAt(pe, 4).toString("ascii"),
      "PE\0\0",
      `${file}: invalid PE signature`,
    );
    const optional = readAt(pe + 24, 70);
    assert.ok(
      [0x10b, 0x20b].includes(optional.readUInt16LE(0)),
      `${file}: unsupported PE header`,
    );
    return optional.readUInt16LE(68);
  } finally {
    closeSync(fd);
  }
}

for (const [name, expected] of [
  ["webdeck", 2],
  ["webdeck-qr", 2],
  ["webdeckctl", 3],
] as const) {
  const file = path.join("target", "debug", `${name}.exe`);
  assert.equal(
    subsystem(file),
    expected,
    `${file}: unexpected Windows subsystem`,
  );
  console.log(`${file}: ${expected === 2 ? "Windows GUI" : "console CLI"}`);
}
