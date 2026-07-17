# FWP Schema v1

`.fwp` files are JSON documents. Version 1 is intentionally small and focused on raw `.bin` workflows.

## Top-Level Shape

```json
{
  "schemaVersion": 1,
  "name": "example",
  "description": "Optional human-readable text",
  "steps": []
}
```

## Path Rules

- Relative paths are resolved relative to the `.fwp` file directory.
- CLI `--input name=path` and `--output name=path` override workflow paths.
- Reports are written to `fpw-reports/` in the current execution directory unless `--report-dir` is provided.

## Step Rules

All steps have:

```json
{
  "id": "unique_step_id",
  "kind": "input"
}
```

Step ids must be unique. Steps run in array order for MVP.

## Step Kinds

### input

Declares a named input.

```json
{
  "id": "firmware",
  "kind": "input",
  "name": "firmware",
  "path": "examples/app.bin"
}
```

CLI override:

```bash
--input firmware=path/to/app.bin
```

### output

Writes an artifact to a named output.

```json
{
  "id": "write_image",
  "kind": "output",
  "input": "patched",
  "name": "image",
  "path": "out/image.bin"
}
```

CLI override:

```bash
--output image=path/to/out.bin
```

### fill

Writes repeated bytes into a binary buffer.

```json
{
  "id": "fill_gap",
  "kind": "fill",
  "input": "firmware",
  "output": "filled",
  "offset": "0x100",
  "length": 16,
  "value": "0xFF"
}
```

Semantics:

- Offset and length use half-open range semantics: `[offset, offset + length)`.
- If the range extends past EOF, the buffer is extended.
- Holes are filled with `0xFF`.

### insert

Overwrites bytes from one artifact into another at an offset.

```json
{
  "id": "insert_digest",
  "kind": "insert",
  "base": "filled",
  "insert": "digest",
  "output": "patched",
  "offset": "0x200"
}
```

Semantics:

- MVP behavior is overwrite, not shifting insertion.
- If the write extends past EOF, the buffer is extended.
- Holes are filled with `0xFF`.

### merge

Merges binary artifacts at explicit offsets.

```json
{
  "id": "merge_boot_app",
  "kind": "merge",
  "output": "image",
  "parts": [
    { "input": "boot", "offset": "0x0" },
    { "input": "app", "offset": "0x1000" }
  ]
}
```

Semantics:

- Overlapping ranges are errors in MVP.
- Holes are filled with `0xFF`.

### crc32

Computes IEEE CRC-32 and writes the 4-byte result into a binary buffer.

```json
{
  "id": "write_crc",
  "kind": "crc32",
  "input": "image",
  "output": "image_crc",
  "range": { "offset": "0x0", "length": 4096 },
  "writeOffset": "0xFFC",
  "endian": "little"
}
```

Defaults:

- poly: `0x04C11DB7`
- init: `0xFFFFFFFF`
- xorout: `0xFFFFFFFF`
- refin/refout: `true`
- endian: `little`

### sha256

Computes SHA-256 and emits the digest as a 32-byte artifact.

```json
{
  "id": "digest",
  "kind": "sha256",
  "input": "image_crc",
  "output": "digest",
  "range": { "offset": "0x0", "length": 4096 }
}
```

MVP behavior:

- Produces a digest artifact.
- Does not write the digest back to the input buffer.
