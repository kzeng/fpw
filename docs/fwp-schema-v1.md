# FWP Schema v1

`.fwp` files are JSON documents. Version 1 supports raw binary artifacts and address-aware Intel HEX image artifacts.

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

### delete

Sets an existing byte range to the erased value `0xFF` without shifting later offsets.

```json
{
  "id": "delete_old_metadata",
  "kind": "delete",
  "input": "firmware",
  "output": "cleaned",
  "range": { "offset": "0x100", "length": 16 }
}
```

Semantics:

- The range is half-open: `[offset, offset + length)`.
- Existing bytes in the range become `0xFF`.
- The output length is always the same as the input length.
- A range extending beyond EOF affects only existing bytes and does not extend the image.
- A range starting at or beyond EOF is a successful no-op.

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

### image-input

Reads an Intel HEX file as a sparse, absolute-address image.

```json
{
  "id": "gboot",
  "kind": "image-input",
  "name": "gboot",
  "path": "gboot.hex"
}
```

The `name` can be overridden with the existing CLI `--input name=path` option.

### image-extract

Copies an absolute-address range from an image. Missing addresses remain absent rather than becoming padding.

```json
{
  "id": "boot_bank_a",
  "kind": "image-extract",
  "input": "gboot",
  "output": "boot_a",
  "address": "0x08000000",
  "length": "0x2000"
}
```

### image-overlay

Overlays one or more sparse images onto a base image without relocating their addresses.

```json
{
  "id": "merge_images",
  "kind": "image-overlay",
  "base": "boot",
  "overlays": ["image_a", "image_b"],
  "output": "full_image",
  "overlap": "error"
}
```

`overlap` is `error` by default. Use `replace` only when later image data is intentionally allowed to overwrite existing addresses.

### image-patch

Writes hexadecimal bytes at an absolute address.

```json
{
  "id": "select_image_a",
  "kind": "image-patch",
  "input": "full_image",
  "output": "selected_image",
  "address": "0x0810C000",
  "data": "0000"
}
```

### image-to-binary

Flattens an explicit absolute-address range to a binary artifact.

```json
{
  "id": "make_bin",
  "kind": "image-to-binary",
  "input": "selected_image",
  "output": "firmware_bin",
  "address": "0x08000000",
  "length": "0x200000",
  "fill": "0xFF"
}
```

Sparse holes are filled with `fill`, which defaults to `0xFF`.

### image-output

Writes a sparse image as Intel HEX while preserving its start address.

```json
{
  "id": "write_hex",
  "kind": "image-output",
  "input": "selected_image",
  "name": "firmware_hex",
  "path": "out/firmware.hex",
  "recordSize": 16
}
```

`recordSize` must be between 1 and 255. The output path can be overridden with `--output name=path`.

### image-extract-string

Reads an ASCII string from an absolute image address. The default `null-space` trim mode removes leading and trailing NUL and space characters.

```json
{
  "id": "read_image_a_version",
  "kind": "image-extract-string",
  "input": "full_image",
  "output": "image_a_version",
  "address": "0x08010250",
  "length": 7,
  "trim": "null-space"
}
```

### assert-equal

Stops execution when two text artifacts differ.

```json
{
  "id": "validate_versions",
  "kind": "assert-equal",
  "left": "image_a_version",
  "right": "image_b_version",
  "message": "Image A and Image B firmware versions differ"
}
```

### image-insert-binary

Splits a binary artifact into configured source ranges and inserts them at absolute image addresses.

```json
{
  "id": "inject_dsp",
  "kind": "image-insert-binary",
  "base": "mcu_image",
  "input": "dsp",
  "output": "mcu_with_dsp",
  "maxLength": "0x93000",
  "parts": [
    {
      "sourceOffset": "0x0",
      "address": "0x08080000",
      "length": "0x80000"
    },
    {
      "sourceOffset": "0x80000",
      "address": "0x08180000",
      "length": "0x13000"
    }
  ]
}
```

If a part reaches the end of the input before its configured length, only the remaining bytes are inserted. Inputs larger than `maxLength` fail before the image is modified.

### nvr-generate

Reads NVR values from `.xlsx` or `.xlsm` and creates an artifact containing the bytes and `NVR-REG` metadata.

```json
{
  "id": "generate_nvr",
  "kind": "nvr-generate",
  "output": "nvr_block",
  "workbook": "default_nvr/config.xlsm",
  "page": 254,
  "bankStart": 8,
  "bankEnd": 9,
  "registerStart": 128,
  "registerEnd": 255,
  "baseAddress": "0x08002000",
  "versionSheet": "Cover",
  "versionCell": "E3",
  "ignoreMaskRule": false,
  "alternateBase": true,
  "sheets": [
    { "name": "8_254_Low", "bank": 8, "rowStart": 4, "rowEnd": 230, "dataColumn": 7 }
  ]
}
```

`dataColumn` is zero-based, so `7` is Excel column H. Rows with an empty register address in column A are skipped. Every mapped bank must yield exactly 128 bytes. An odd bank count is padded to a 256-byte archive boundary.

### nvr-inject-image

Writes the NVR block at its calculated Flash address. `mirrorOffset` is optional; `0x100000` mirrors it to Image B in the current Postbuild layout.

```json
{
  "id": "inject_nvr",
  "kind": "nvr-inject-image",
  "image": "full_image",
  "nvr": "nvr_block",
  "output": "image_with_nvr",
  "mirrorOffset": "0x100000"
}
```

### nvr-patch-registers

Creates a new NVR artifact with explicit register changes while retaining the original NVR header metadata.

```json
{
  "id": "patch_aa_settings",
  "kind": "nvr-patch-registers",
  "input": "nvr_block",
  "output": "aa_nvr_block",
  "patches": [
    { "bank": 1, "register": 144, "data": "00" },
    { "bank": 1, "register": 147, "data": "0000" }
  ]
}
```

Patch ranges must remain inside the selected block's effective `dataLen`.

### nvr-append-archive

Copies an existing archive and invokes an imgAr-compatible executable with file type `NVR-REG`.

```json
{
  "id": "append_nvr",
  "kind": "nvr-append-archive",
  "archive": "mcu_dsp_archive",
  "nvr": "nvr_block",
  "output": "mcu_dsp_nvr_archive",
  "tool": "../tools/imgAr.exe",
  "encryption": "enc0"
}
```

`encryption` must be `enc0` or `enc1`. A missing executable or non-zero exit status fails the workflow.

### imgAr-append

Creates or extends a device release archive using the packaged legacy imgAr tool.

```json
{
  "id": "archive_image_a",
  "kind": "imgar-append",
  "input": "image_a",
  "output": "archive_a",
  "tool": "../tools/imgAr.exe",
  "fileType": "IMG-A",
  "encryption": "enc0"
}
```

Omit `archive` for the first entry. Later entries reference the preceding binary archive:

```json
{
  "id": "archive_dsp",
  "kind": "imgar-append",
  "archive": "archive_ab",
  "input": "dsp",
  "output": "archive_ab_dsp",
  "tool": "../tools/imgAr.exe",
  "fileType": "DSP-N-A",
  "encryption": "enc0",
  "inputFileName": "dsp_vE000F200_ig1_A.bin"
}
```

`IMG-A` and `IMG-B` require image artifacts. `DSP-N-A` and `DSP-N-B` require binary artifacts of at least `0x7000` bytes. DSP `inputFileName` is mandatory because legacy imgAr parses the eight-digit version and `ig0/ig1` flag from fixed filename positions.
