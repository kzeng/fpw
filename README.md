# FPW - Firmware Packaging Workflow

[中文说明](README-CN.md) | [User Manual / 用户手册](User-Manual.md) | [FWP Schema](docs/fwp-schema-v1.md)

FPW is a local-first firmware packaging workflow tool for repeatable raw binary image processing.

- `fpw` is the primary CLI workflow runner.
- `fpw web` starts a local WebUI for creating, managing, previewing, and running workflows.
- `.fwp` files are versioned JSON workflow definitions that can run with or without the WebUI.
- Windows and Linux are the target platforms.

## Features

- Ordered raw `.bin` processing with `input`, `output`, `fill`, `insert`, `merge`, `crc32`, and `sha256` steps.
- JSON and TXT execution reports containing the command, timing, step status, and file hashes.
- Five-stage WebUI authoring wizard and a managed workflow library.
- Create, open, save, duplicate, archive, and import workflow operations.
- English and Simplified Chinese WebUI, with English as the first-use default.
- Run Preview with both an execution plan and a copyable CLI command.
- Best-effort FirmwareFlow `.ffc` import for supported steps.

## Quick Start on Windows

Prerequisites:

- Rust toolchain with Cargo
- Node.js and npm

Build the WebUI and release executable from the repository root:

```powershell
Set-Location web
npm install
npm run build
Set-Location ..
cargo build --release -p fpw-cli
```

Validate, preview, and run an example:

```powershell
.\target\release\fpw.exe validate examples\fill-crc-sha.fwp
.\target\release\fpw.exe preview examples\fill-crc-sha.fwp
.\target\release\fpw.exe run examples\fill-crc-sha.fwp
```

Start the WebUI:

```powershell
.\target\release\fpw.exe web --host 127.0.0.1 --port 4769
```

Open `http://127.0.0.1:4769/`. Keep the terminal open while using the WebUI.

## CLI

```text
fpw validate <workflow.fwp>
fpw preview <workflow.fwp>
fpw run <workflow.fwp> [--input name=path] [--output name=path] [--report-dir path]
fpw config [--output workflow.fwp]
fpw web [--host 127.0.0.1] [--port 4769]
fpw import-ffc <workflow.ffc> --output <workflow.fwp>
fpw recent list
fpw recent add <workflow.fwp>
```

Use repeated `--input` and `--output` options to override named paths:

```powershell
.\target\release\fpw.exe run workflows\release.fwp `
  --input firmware=C:\firmware\app.bin `
  --output image=C:\firmware\release.bin `
  --report-dir C:\firmware\reports
```

Paths declared inside a workflow are resolved relative to the `.fwp` file. CLI input/output override paths and the report directory are resolved from the current process directory when relative.

## WebUI Workflow

The WebUI separates the user journey into three task areas:

1. Manage `.fwp` files in the workflow library.
2. Create or edit a workflow through the five-stage wizard.
3. Select a saved workflow, preview it, and run it.

Run Preview displays a copyable `fpw run` command matching the selected workflow and current path overrides. The command can be executed in another terminal where `fpw` is available. If it is not on `PATH`, replace `fpw` with `.\target\release\fpw.exe`.

Web-managed workflows are stored under `workflows/` by default. Archived files move to `workflows/.trash/` instead of being permanently deleted.

Environment overrides:

- `FPW_WORKFLOW_HOME`: managed workflow library directory.
- `FPW_CONFIG_HOME`: local state directory, including recent projects.

## Reports

Every successful `fpw run` writes JSON and TXT reports. The default directory is `fpw-reports/` under the current process directory.

Reports contain:

- FPW version and workflow identity
- Workflow SHA256
- Reproducible command arguments
- Start time, end time, and duration
- Per-step status and errors
- Input/output file size and SHA256

## Repository Layout

```text
crates/fpw-core/   Workflow model, validation, execution, and reports
crates/fpw-cli/    CLI and local Web server
web/               React/Vite WebUI
docs/              Schema and architecture notes
examples/          Example workflows and binary fixtures
workflows/         Default WebUI-managed workflow library
```

## Development

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
Set-Location web
npm run build
```

Deferred features and implementation history are tracked in [dev_log.md](dev_log.md).
