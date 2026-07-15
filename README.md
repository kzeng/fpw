# FPW - Firmware Packaging Workflow

FPW is a local-first firmware packaging workflow tool.

The product shape is:

- `fpw` CLI as the primary executable workflow runner.
- `fpw web` as a local WebUI for editing, previewing, and running workflows.
- `.fwp` JSON workflow files that can be versioned, reviewed, and executed without the WebUI.

Target platforms: Windows and Linux.

## MVP Scope

The first version targets raw `.bin` processing only.

Supported steps:

- `input`
- `output`
- `fill`
- `insert`
- `merge`
- `crc32`
- `sha256`

Deferred features are tracked in [dev_log.md](dev_log.md).

## Planned CLI

```bash
fpw validate workflow.fwp
fpw preview workflow.fwp
fpw run workflow.fwp --input firmware=app.bin --output image=out.bin
fpw config
fpw web
fpw import-ffc workflow.ffc --output workflow.fwp
fpw recent list
```

## Repository Layout

```text
crates/fpw-core/   Workflow model, validation, execution, reports
crates/fpw-cli/    Command line interface
web/               Local React WebUI
docs/              Workflow schema and design notes
examples/          Example .fwp workflows and binary fixtures
```

## Current Implementation Status

This repository currently contains the initial project skeleton:

- Versioned `.fwp` schema documentation.
- Example workflows.
- Rust workspace with `fpw-core` and `fpw-cli`.
- Core execution path for MVP steps.
- JSON/TXT execution report generation.
- Minimal `fpw web` local HTTP entry.
- `fpw web` serves `web/dist` when built, with a built-in fallback page.
- React/Vite WebUI scaffold.

Rust tooling is required to build the CLI:

```bash
cargo build -p fpw-cli
```

For portable or CI runs, set `FPW_CONFIG_HOME` to control where local state such as recent projects is written.
