# Getting started

Install the binary, create a project, write a contract and a template, then generate.

## Install

Prebuilt binaries (see [`release/README.md`](../release/README.md)):

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

## Create a project

```bash
datuma_k start myapp
cd myapp
```

`start` takes a single directory name (no `/` or `\`). It creates:

```
myapp/
  .env
  data/                 # *.dtct contracts, plus keywords.md
  engine/               # *.ngin templates
  definition/           # *.def.ngin helpers (cases.def.ngin is seeded)
```

The `.env` is:

```
ROOT_DIRECTORY=.
DTCT_DIRECTORY=data
NGIN_DIRECTORY=engine
DEF_DIRECTORY=definition
```

`definition/cases.def.ngin` ships case helpers (`snake_case`, `pascal_case`, …). `data/keywords.md` is an empty table header; fill it when you want [`datuma_k check`](generation.md#check) to pass. `run` does not require it.

## A tiny contract and template

`data/app.dtct`:

```
Item [Resource] {
  title: text_type<required>
}
```

`engine/hello.ngin`:

````ngin
|$ROOT_DIRECTORY/hello.txt>
```
@{
for (model in dk.models) {
  => model
}
}@
```
````

`|path>` opens an output file. The fenced block is the file body. `@{ … }@` runs the [scripting language](language.md). `=> model` writes the model name into that body. `dk.models` is the [query host](querying.md) over every contract.

## Generate

Run from **this** project directory, not `$HOME` or `/`. If a command asks you to confirm walking a dangerous directory, type `yes`.

```bash
datuma_k run
```

That writes `hello.txt` containing `Item`, plus a `.dkcache` next to it. Edits **between** generated spans survive later runs; edits **inside** a generated span are overwritten. See [Generation](generation.md).

## Editor highlighting

VS Code / Cursor cannot pick up TextMate grammars from workspace settings. Install the folder once:

1. Command Palette → **Extensions: Install from Location…**
2. Choose the [`syntaxes`](../syntaxes/README.md) directory (the folder that contains `package.json`).

After you edit the grammar JSON, **Developer: Reload Window**.

## Next

- [Contracts](contracts.md) — models, traits, fields, types, attributes
- [Templates](templates.md) — file rules, emit, loops, the example’s Pydantic / Zod pattern
- [`tests/example`](../tests/example) — FastAPI + React from one contract
