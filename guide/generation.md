# Generation

`datuma_k run` turns contracts and templates into files. The other commands inspect the same project without writing generated output (except `start`, which scaffolds).

## Project layout

`.env` in the current working directory:

| Key | Default role |
| --- | --- |
| `ROOT_DIRECTORY` | Project root (output paths, default `.`) |
| `DTCT_DIRECTORY` | `*.dtct` tree (default `data`) |
| `NGIN_DIRECTORY` | `*.ngin` templates (default `engine`) |
| `DEF_DIRECTORY` | `*.def.ngin` helpers (default `definition`) |

Relative directory values join `ROOT_DIRECTORY`. Run commands from the project directory so this `.env` is the one that loads.

If the cwd is `/`, `$HOME`, or `$HOME`’s parent, the CLI asks you to type `yes`. Without a TTY it refuses.

## What `run` does

1. Load directory paths from `.env`.
2. Parse every `*.dtct`, `*.def.ngin`, and `*.ngin` under those trees.
3. Seed template scope with env vars, `ROOT_DIRECTORY`, and `dk`.
4. Execute every definition file, then walk each template to plan output files.
5. Commit the plan through **dkcache**.

```bash
datuma_k run
```

No arguments. Parse or runtime errors stop the run; nothing is “best effort” per file.

## dkcache

A `.dkcache` file sits next to generated output (JSON: version, per-file trees of host and frame nodes).

| Edit | Next `run` |
| --- | --- |
| Text **between** generated spans | Kept |
| Text **inside** a generated span | Overwritten |
| Model / file that disappeared from the plan | Deleted |
| Empty `+=` after all guards fail | Region cut |

Put generated files in a `generated/` folder (or similar). Keep handwritten glue — routers, HTTP clients, auth — **outside** those spans, or in files templates never open.

Older outputs used inline fence markers (`/*@dk^…@*/`). If disk still has those and the cache is empty, commit strips the markers and rebuilds a tree.

## CLI

```
datuma_k start <project-name>
datuma_k check
datuma_k catalog [--trait NAME] [--model NAME] [--field NAME] [--attribute NAME] [--type NAME]
datuma_k preview
datuma_k run
```

`start` takes a single directory name (no `/` or `\`). `check`, `catalog`, and `preview` print JSON and do not write generated files.

### `start`

Creates `data/`, `engine/`, `definition/`, `.env`, `data/keywords.md`, and `definition/cases.def.ngin`. See [Getting started](getting-started.md).

### `check`

Loads and plans the project the same way `run` would, then compares every contract name to `data/keywords.md`. Prints `{ "ok": …, "diagnostics": … }`. Non-zero exit when `ok` is false.

`run` does **not** require the keyword table. `check` does: missing file, undocumented name, unused documented name, wrong `kind`, or empty description / purpose / platforms all fail. Table format: [Contracts](contracts.md#keyword-table).

### `catalog`

Lists models, traits, types, attributes, and fields from the contracts. Optional `--trait` / `--model` / `--field` / `--attribute` / `--type` flags are include filters, one per dimension — the same rule as [`dk.trait("…")`](querying.md). Unknown names yield an empty list, not an error.

### `preview`

Plans every template and prints the flattened file contents as JSON. Does not create those files and does not update `.dkcache`. Use it to see what `run` would write.

### `run`

Commits the plan. This is the command that touches generated source.

## Example project

[`tests/example`](../tests/example) is a FastAPI + Vite React app from one contract (`Event`, `Venue`). Generate from that directory:

```bash
cargo run --manifest-path ../../Cargo.toml -- run
```

How to start the servers: [`tests/example/README.md`](../tests/example/README.md). `cargo test` regenerates the example and asserts the emitted files; it does not start uvicorn or npm.

Agents that should call this binary over stdio: [`mcp/README.md`](../mcp/README.md).
