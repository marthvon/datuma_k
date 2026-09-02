# datuma_k

A data contract (`*.dtct`) plus templates (`*.ngin`) that generate source. Declare the shape once; each platform gets its own types, validation, and UI instead of copying fields by hand.

An API backend and a React client both need the same records: capacity 1–500, `starts_at` a datetime, title required. Without a shared contract those rules are rewritten in Pydantic, Zod, forms, and serializers, and they drift. datuma_k keeps the shape in `*.dtct`. Shared attributes (`min`, `max`, `local`, `required`) mean the same thing everywhere. ngin turns them into *different* code on purpose: Pydantic `Field(ge=, le=)` on Python, Zod `.min().max()` on TypeScript.

## Install

Prebuilt binaries (see [`release/README.md`](release/README.md)):

| File | Platform |
| --- | --- |
| `datuma_k-macos-aarch64` | macOS Apple Silicon |
| `datuma_k-macos-x86_64` | macOS Intel |
| `datuma_k-linux-aarch64` | Linux arm64 |
| `datuma_k-linux-x86_64` | Linux x86_64 |
| `datuma_k-windows-x86_64.exe` | Windows x86_64 |

Rename the file for your platform to `datuma_k` (or `datuma_k.exe` on Windows) and put it on `PATH`.

Or build from source:

```bash
cargo build --release
```

The binary is `target/release/datuma_k`.

## Quick start

```bash
datuma_k start myapp
cd myapp
```

`start` creates `data/`, `engine/`, `definition/`, a `.env`, `data/keywords.md`, and a starter `definition/cases.def.ngin`. The `.env` is:

```
ROOT_DIRECTORY=.
DTCT_DIRECTORY=data
NGIN_DIRECTORY=engine
DEF_DIRECTORY=definition
```

Add `*.dtct` under `data/`, `*.ngin` under `engine/`, and extra `*.def.ngin` under `definition/` as needed. Document every contract name in `data/keywords.md`. Then generate from **this** project directory (not `$HOME` or `/` — if a command asks, type `yes`):

```bash
datuma_k run
```

## Commands

```
datuma_k start <project-name>
datuma_k check
datuma_k catalog [--trait NAME] [--model NAME] [--field NAME] [--attribute NAME] [--type NAME]
datuma_k preview
datuma_k run
```

`start` takes a single directory name (no `/` or `\`). `check`, `catalog`, and `preview` print JSON and do not write generated files. `catalog` filters match `dk` include filters. `run` commits planned files through dkcache.

`check` also requires `data/keywords.md`: every model, trait, type, attribute, and field name needs a table row with `kind`, `description`, `purpose`, and `platforms` (`api_server`, `web_frontend`, `mobile_frontend`). See [`mcp/resources/keywords.md`](mcp/resources/keywords.md).

Agents: install the MCP in [`mcp/`](mcp/README.md) instead of wrapping these commands by hand.

## What `run` does

1. Load directory paths from `.env` (`ROOT_DIRECTORY`, `DTCT_DIRECTORY`, `NGIN_DIRECTORY`, `DEF_DIRECTORY`).
2. Parse every `*.dtct`, `*.def.ngin`, and `*.ngin` under those trees.
3. Plan generated files from the templates.
4. Commit them through **dkcache**.

A `.dkcache` file sits next to generated output. Text **between** generated spans is kept across runs. Edits **inside** a generated snippet are overwritten the next time you run.

## Example

[`tests/example`](tests/example) is a FastAPI + Vite React app generated from one contract (`Event` and `Venue`). How to generate and start those servers: [`tests/example/README.md`](tests/example/README.md).

`cargo test` regenerates the example and checks the emitted files. It does not start uvicorn or npm.

## Developers

| Crate area | Role |
| --- | --- |
| `src/core` | Language, parser, interpreter |
| `src/dtct` | Data contracts (`*.dtct`) |
| `src/ngin` | Templates (`*.ngin`, `*.def.ngin`) |
| `src/dkcache` | Reconcile and commit generated files |
| `src/project` | Load, plan, catalog, check, preview |

```bash
cargo test
```

Parser internals and module notes: [`docs/README.md`](docs/README.md).

MCP (stdio, calls this binary): [`mcp/README.md`](mcp/README.md).
