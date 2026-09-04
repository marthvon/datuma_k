# datuma_k

A data contract (`*.dtct`) plus templates (`*.ngin`) that generate source. Declare the shape once; each platform gets its own types, validation, and UI instead of copying fields by hand.

An API backend and a React client both need the same records: capacity 1–500, `starts_at` a datetime, title required. Without a shared contract those rules are rewritten in Pydantic, Zod, forms, and serializers, and they drift. datuma_k keeps the shape in `*.dtct`. Shared attributes (`min`, `max`, `local`, `required`) mean the same thing everywhere. ngin turns them into *different* code on purpose: Pydantic `Field(ge=, le=)` on Python, Zod `.min().max()` on TypeScript.

**Guide:** [guide/README.md](guide/README.md) — getting started, contracts, templates, the scripting language, `dk` queries, and generation.

## Install

The command is `datuma_k`. The package name is `datuma-k`. The Homebrew tap and Scoop bucket use `datuma_k` (underscore) — `brew tap marthvon/datuma-k` clones a repo that does not exist.

More detail: [guide/getting-started.md](guide/getting-started.md).

**macOS (and Linux with Homebrew)** — Homebrew 6 ignores untrusted taps, so `brew install datuma-k` after a bare `brew tap` will not find it:

```bash
brew install marthvon/datuma_k/datuma-k
```

If that still fails after an earlier `brew tap`, run `brew untap marthvon/datuma_k` first.

**Fedora**

```bash
sudo dnf copr enable marthvon/datuma-k
sudo dnf install datuma-k
```

**Debian / Ubuntu** — no apt repo:

```bash
# x86_64
curl -fsSL -o /tmp/datuma-k.deb https://github.com/marthvon/datuma_k/releases/download/v1.0.0/datuma-k_1.0.0_amd64.deb && sudo apt install /tmp/datuma-k.deb && rm -f /tmp/datuma-k.deb

# arm64
curl -fsSL -o /tmp/datuma-k.deb https://github.com/marthvon/datuma_k/releases/download/v1.0.0/datuma-k_1.0.0_arm64.deb && sudo apt install /tmp/datuma-k.deb && rm -f /tmp/datuma-k.deb
```

**Alpine** — musl `.apk` (unsigned). On Docker Alpine, install `curl` first; skip `sudo` if you are root. If a previous glibc apk is installed, `apk del datuma-k` first.

```sh
apk add --no-cache curl
curl -fsSL -o /tmp/datuma-k.apk https://github.com/marthvon/datuma_k/releases/download/v1.0.0/datuma-k_1.0.0_x86_64.apk && apk add --allow-untrusted /tmp/datuma-k.apk && rm -f /tmp/datuma-k.apk
```

aarch64: `datuma-k_1.0.0_aarch64.apk`. The command is `datuma_k`.

**Windows**

```powershell
scoop bucket add datuma_k https://github.com/marthvon/scoop-datuma_k
scoop install datuma-k
```

**Direct binary** — download a binary or `.rpm` from the same release page, rename to `datuma_k` (or `datuma_k.exe` on Windows), and put it on `PATH`. On Fedora without Copr: `sudo dnf install ./datuma-k-*-1.x86_64.rpm`.

**From source:** `cargo build --release` → `target/release/datuma_k`.

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

## License

Copyright (C) 2026 mamertvonn

datuma_k (the engine, MCP, grammars, and helpers shipped with this repository) is licensed under the GNU Affero General Public License v3.0. See [`LICENSE.md`](LICENSE.md).

Your own `*.dtct` and `*.ngin` files, and the source `datuma_k run` generates from them, are yours. AGPL does not apply to that output just because you used the binary.

Distributing a modified datuma_k, including offering it as a network service, must stay under AGPL.
