# FPW Architecture

FPW is designed as a CLI-first, local-first tool for Windows and Linux.

## Layers

```text
fpw-cli
  commands, argument parsing, local web server

fpw-core
  workflow schema, validation, execution, reports, importers

fpw-web
  browser UI served by `fpw web`
```

The CLI and WebUI must share `fpw-core`. Workflow execution logic must not be duplicated in TypeScript.

## Platform Rules

MVP must support Windows and Linux.

Avoid platform-specific assumptions in core behavior:

- No PowerShell dependency.
- No `.bat`-only output.
- No Windows registry dependency.
- No Tauri-only desktop shell.
- Use Rust path APIs rather than string-only path manipulation.

Local FPW config is stored in the platform user config directory by default. Tests, CI, and portable use can override this with `FPW_CONFIG_HOME`.

## CLI Commands

MVP commands:

```bash
fpw validate workflow.fwp
fpw preview workflow.fwp
fpw run workflow.fwp --input firmware=app.bin --output image=out.bin
fpw config
fpw web
fpw import-ffc workflow.ffc --output workflow.fwp
fpw recent list
```

`fpw run` is the canonical execution path. WebUI execution should call the same core path through local APIs.

## Local Web Server

`fpw web` starts a local HTTP server.

If `web/dist` exists, static assets are served from that directory. Otherwise the CLI serves a built-in fallback page so the command remains useful before the WebUI is built.

Initial routes:

- `GET /`: WebUI shell.
- `GET /api/health`: service health.
- `GET /api/recent-projects`: local recent project list.

Planned API routes:

- `POST /api/workflows/validate`
- `POST /api/workflows/preview`
- `POST /api/workflows/run`
- `POST /api/workflows/import/ffc`
- `POST /api/recent-projects`

## Workflow Execution

MVP executes steps in array order. This intentionally avoids graph scheduling complexity in v1.

The execution engine maintains an in-memory artifact map:

```text
artifact name -> bytes
```

`input` loads bytes into the map. Processing steps read artifacts and produce new artifacts. `output` writes artifacts to disk.

## Reports

Every `fpw run` writes JSON and TXT reports by default.

Default directory:

```text
fpw-reports/
```

The report includes:

- FPW version
- workflow path
- workflow SHA256
- command
- start/end time
- duration
- step status
- input/output file SHA256

## Deferred Architecture

Deferred features should not distort the MVP core:

- Node canvas editor
- Graph execution
- RSA/AES/HMAC steps
- Intel HEX / Motorola SREC address-space model
- C/CMake project generation
- per-workflow executable generation
- multi-user server deployment
- role/permission model
