# Getting started

Install the binary, create a project, write a contract and a template, then generate.

## Install

The command is `datuma_k`. The package name is `datuma-k`. The Homebrew tap and Scoop bucket use `datuma_k` (underscore) — `brew tap marthvon/datuma-k` clones a repo that does not exist.

### macOS (and Linux with Homebrew)

Homebrew 6 will not load an untrusted third-party tap, so `brew tap` then `brew install datuma-k` never finds the formula. Use the fully qualified name (taps, trusts, and installs):

```bash
brew install marthvon/datuma_k/datuma-k
```

If you already ran `brew tap marthvon/datuma_k` and install still fails, `brew untap marthvon/datuma_k` first — a tap cloned before the formula was published stays empty.

### Fedora

```bash
sudo dnf copr enable marthvon/datuma-k
sudo dnf install datuma-k
```

### Debian / Ubuntu

There is no apt repository.

```bash
# x86_64
curl -fsSL -o /tmp/datuma-k.deb https://github.com/marthvon/datuma_k/releases/download/v1.0.0/datuma-k_1.0.0_amd64.deb && sudo apt install /tmp/datuma-k.deb && rm -f /tmp/datuma-k.deb

# arm64
curl -fsSL -o /tmp/datuma-k.deb https://github.com/marthvon/datuma_k/releases/download/v1.0.0/datuma-k_1.0.0_arm64.deb && sudo apt install /tmp/datuma-k.deb && rm -f /tmp/datuma-k.deb
```

### Alpine

The `.apk` is a musl build. Alpine Docker images are root and often have no `curl` and no `sudo`.

```sh
apk add --no-cache curl

# x86_64
curl -fsSL -o /tmp/datuma-k.apk https://github.com/marthvon/datuma_k/releases/download/v1.0.0/datuma-k_1.0.0_x86_64.apk && apk add --allow-untrusted /tmp/datuma-k.apk && rm -f /tmp/datuma-k.apk

# aarch64
curl -fsSL -o /tmp/datuma-k.apk https://github.com/marthvon/datuma_k/releases/download/v1.0.0/datuma-k_1.0.0_aarch64.apk && apk add --allow-untrusted /tmp/datuma-k.apk && rm -f /tmp/datuma-k.apk
```

`--allow-untrusted` is required because the package is unsigned. Prefix with `sudo` if you are not root. If an older glibc `.apk` is already installed, `apk del datuma-k` first, then install again.

The command is `datuma_k` (underscore), at `/usr/bin/datuma_k`.

### Windows

```powershell
scoop bucket add datuma_k https://github.com/marthvon/scoop-datuma_k
scoop install datuma-k
```

### Direct binary

Download a binary or `.rpm` from the [latest GitHub release](https://github.com/marthvon/datuma_k/releases/latest). Rename the binary to `datuma_k` (or `datuma_k.exe` on Windows) and put it on `PATH`. On Fedora without Copr:

```bash
sudo dnf install ./datuma-k-*-1.x86_64.rpm   # or aarch64
```

### From source

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
