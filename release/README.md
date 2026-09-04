# datuma_k releases

Binaries are built into `release/<version>/` by [`scripts/release.sh`](../scripts/release.sh). That directory is gitignored; run the script locally.

```sh
./scripts/release.sh
./release/publish.sh              # packages, GitHub Release, packaging channels
./release/publish.sh --dry-run    # render manifests, no upload or push
./release/publish.sh --only github,homebrew
./release/publish.sh --setup      # create/clone sibling repos only
```

Requires a current Rust toolchain on the host. Linux and Windows artifacts need Docker (Linux `amd64` on Apple Silicon uses QEMU and is slow). If Docker is missing or the daemon is down, macOS binaries are still produced.

Package id is `datuma-k`; the installed command is `datuma_k`.

## 1.0.0 artifacts

| File | Platform |
| --- | --- |
| `datuma_k-macos-aarch64` | macOS Apple Silicon |
| `datuma_k-macos-x86_64` | macOS Intel |
| `datuma_k-linux-aarch64` | Linux arm64 (glibc) |
| `datuma_k-linux-x86_64` | Linux x86_64 (glibc) |
| `datuma_k-linux-musl-aarch64` | Alpine arm64 (musl) |
| `datuma_k-linux-musl-x86_64` | Alpine x86_64 (musl) |
| `datuma_k-windows-x86_64.exe` | Windows x86_64 |
| `SHA256SUMS` | checksums of the binaries above |

`publish.sh` also writes:

| File | Notes |
| --- | --- |
| `datuma-k_<version>_amd64.deb` | from the Linux x86_64 binary (nfpm) |
| `datuma-k_<version>_arm64.deb` | from the Linux arm64 binary |
| `datuma-k-<version>-1.x86_64.rpm` | same |
| `datuma-k-<version>-1.aarch64.rpm` | same |
| `datuma-k_<version>_x86_64.apk` | Alpine musl x86_64 |
| `datuma-k_<version>_aarch64.apk` | Alpine musl aarch64 |

## publish.sh

After `./scripts/release.sh`, `./release/publish.sh` validates the binaries plus `SHA256SUMS`, builds the Linux packages, uploads everything to the GitHub Release `v<version>`, and updates downstream channels when credentials are present.

| Flag | Effect |
| --- | --- |
| `--dry-run` | Render manifests under `release/.publish/scratch/`, print actions, no git push / no upload |
| `--skip-github` | Packaging repos only |
| `--only CHANNELS` | Comma list: `github,homebrew,aur,scoop,winget,copr` |
| `--setup` | Create/clone sibling repos and print remaining manual steps, then exit |

Copy [`publish.env.example`](publish.env.example) to `release/publish.env` (gitignored) if you need tokens.

### Prerequisites

| Tool | Needed for |
| --- | --- |
| `gh` (authenticated) | GitHub Release, Homebrew tap, Scoop bucket, WinGet fork/PR |
| `nfpm` | `.deb` / `.rpm` / `.apk` |
| `git`, `ssh` | AUR |
| `makepkg` | optional; used to generate AUR `.SRCINFO` when present |
| `copr-cli` | Fedora Copr channel (`copr` in `--only`; skipped if missing) |

`GH_TOKEN` in `publish.env` is unused until `gh` is on PATH. On macOS:

```bash
brew install gh
```

Then `gh auth login`, or keep `GH_TOKEN` set. Confirm with `gh --version` and `gh auth status`.

Install nfpm (needed only for GitHub `.deb`/`.rpm`/`.apk` uploads). Go is **not** required.

On macOS (preferred):

```bash
brew install nfpm
nfpm --version
```

Or download a prebuilt binary from [nfpm releases](https://github.com/goreleaser/nfpm/releases) and put it on PATH.

Only if you already have a Go toolchain:

```bash
go install github.com/goreleaser/nfpm/v2/cmd/nfpm@latest
export PATH="$HOME/go/bin:$PATH"
```

Install Copr support with one of:

```bash
pip install copr-cli
```

On Fedora: `sudo dnf install copr-cli`.

If `pip` is not on PATH:

```bash
python3 -m pip install --user copr-cli
```

On macOS, add the user scripts dir to PATH (adjust `3.x` to your Python version):

```bash
export PATH="$HOME/Library/Python/3.x/bin:$PATH"
```

On Homebrew Python you may get `externally-managed-environment` (PEP 668). Recommended on macOS:

```bash
brew install pipx
pipx install copr-cli
pipx ensurepath
```

Restart the shell, then `copr-cli --version`.

Optional channels are skipped with a message when credentials are missing. `--only` on a channel that cannot run is an error.

Re-running for the same version is safe: GitHub assets are uploaded with `--clobber`; packaging repos get a new commit only when files changed.

### One-time setup

`./release/publish.sh --setup` (or the first real publish) creates and clones GitHub sibling repos under `release/.publish/`. Homebrew and Scoop repos are forced **public** (private taps/buckets cannot be installed). `git push` uses `gh auth git-credential` on those clones so you do not need `gh auth setup-git`.

| Repo / project | User install |
| --- | --- |
| `marthvon/homebrew-datuma_k` | `brew tap marthvon/datuma_k && brew install datuma-k` |
| `marthvon/scoop-datuma_k` | `scoop bucket add datuma_k https://github.com/marthvon/scoop-datuma_k && scoop install datuma-k` |
| AUR `datuma-k` | `yay -S datuma-k` |
| PR → `microsoft/winget-pkgs` | `winget install datuma-k` (after Microsoft merges) |
| Copr `marthvon/datuma-k` | `sudo dnf copr enable marthvon/datuma-k && sudo dnf install datuma-k` |
| GitHub Release `.deb` | `curl … -o /tmp/datuma-k.deb && sudo apt install /tmp/datuma-k.deb && rm -f /tmp/datuma-k.deb` |
| GitHub Release `.apk` | musl; `apk add curl` on Docker Alpine, then curl to `/tmp` and `apk add --allow-untrusted` |

This does **not** submit to official Debian, Fedora, or Arch archives.

**AUR** cannot be created with `gh`. Register at [aur.archlinux.org](https://aur.archlinux.org/account/register/), add an SSH key, then `ssh aur@aur.archlinux.org` should print a welcome (no shell). Set `AUR_SSH_KEY` if the key is not in ssh-agent. Authenticated clone of `ssh://aur@aur.archlinux.org/datuma-k.git` creates the empty package repo; the script pushes `PKGBUILD` and `.SRCINFO` after that.

**Copr:** register at [accounts.fedoraproject.org](https://accounts.fedoraproject.org/), then install `copr-cli`. On macOS with Homebrew Python:

```bash
brew install pipx
pipx install copr-cli
pipx ensurepath
```

Elsewhere: `pip install copr-cli`, `sudo dnf install copr-cli` (Fedora), or `python3 -m pip install --user copr-cli`. Download the API config from [copr.fedorainfracloud.org/api](https://copr.fedorainfracloud.org/api/) into `~/.config/copr`, or set `COPR_LOGIN`, `COPR_USERNAME`, and `COPR_API_TOKEN` in `release/publish.env` (Copr issues a login id and a token, not a token alone). The script creates or updates project `datuma-k` with currently supported Fedora + rawhide chroots (`fedora-43` / `fedora-44` / `fedora-rawhide`, x86_64 and aarch64) and `--follow-fedora-branching on` so new Fedora branches get chroots automatically. EOL Fedora is not a target. If that fails (unknown chroot names, etc.), set those chroots in the [Copr web UI](https://copr.fedorainfracloud.org/coprs/marthvon/datuma-k/) and re-run.

**WinGet:** the script forks `microsoft/winget-pkgs` to `WINGET_FORK` (default `marthvon/winget-pkgs`), sparse-clones it, writes v1.9 manifests under `manifests/d/datuma-k/datuma-k/<version>/`, and opens a PR. Merge is a Microsoft review. Set `WINGET_SKIP=1` to ignore this channel. The clone is large even with sparse checkout; first fork takes a while.

### Limits

- WinGet is a PR; `winget install` works only after merge.
- Official distro archives are out of scope.
- crates.io, npm, PyPI, VS Marketplace, and a self-hosted apt repo are out of scope.
