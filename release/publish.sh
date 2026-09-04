#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

PACKAGING="$ROOT/release/packaging"
PUBLISH_DIR="$ROOT/release/.publish"
SCRATCH="$PUBLISH_DIR/scratch"
ENV_FILE="$ROOT/release/publish.env"

GH_OWNER="marthvon"
GH_REPO="marthvon/datuma_k"
HOMEBREW_REPO="marthvon/homebrew-datuma_k"
SCOOP_REPO="marthvon/scoop-datuma_k"
COPR_PROJECT="marthvon/datuma-k"
COPR_CHROOTS=(
  fedora-43-x86_64
  fedora-43-aarch64
  fedora-44-x86_64
  fedora-44-aarch64
  fedora-rawhide-x86_64
  fedora-rawhide-aarch64
)
AUR_PACKAGE="datuma-k"
AUR_REMOTE="ssh://aur@aur.archlinux.org/datuma-k.git"
REPO_HOMEPAGE="https://github.com/marthvon/datuma_k"
WINGET_UPSTREAM="microsoft/winget-pkgs"
WINGET_ID="datuma-k.datuma-k"

BINARIES="datuma_k-macos-aarch64 datuma_k-macos-x86_64 datuma_k-linux-aarch64 datuma_k-linux-x86_64 datuma_k-linux-musl-aarch64 datuma_k-linux-musl-x86_64 datuma_k-windows-x86_64.exe"

DRY_RUN="${DRY_RUN:-0}"
SKIP_GITHUB=0
ONLY=""
SETUP_ONLY=0
WINGET_FORK="${WINGET_FORK:-marthvon/winget-pkgs}"
WINGET_SKIP="${WINGET_SKIP:-0}"
GIT_TERMINAL_PROMPT=0
export GIT_TERMINAL_PROMPT

load_env() {
  if [[ -f "$ENV_FILE" ]]; then
    set -a
    # shellcheck disable=SC1090
    . "$ENV_FILE"
    set +a
  fi
}

usage() {
  cat <<'EOF'
Usage: release/publish.sh [options]

Build .deb/.rpm/.apk from release/<version>/ binaries, upload to GitHub Releases,
and refresh Homebrew, AUR, Scoop, WinGet, and Copr packaging channels.

Options:
  --dry-run          Render manifests and print actions; no upload or push
  --skip-github      Update packaging repos only (no GitHub Release upload)
  --only CHANNELS    Comma list: github,homebrew,aur,scoop,winget,copr
  --setup            Create/clone sibling repos and print remaining manual steps
  -h, --help         Show this help

Environment (release/publish.env or the shell): see release/publish.env.example.

Typical:
  ./scripts/release.sh
  ./release/publish.sh
  ./release/publish.sh --dry-run
  ./release/publish.sh --only github,homebrew
  ./release/publish.sh --setup
EOF
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --dry-run)
        DRY_RUN=1
        shift
        ;;
      --skip-github)
        SKIP_GITHUB=1
        shift
        ;;
      --only)
        if [[ $# -lt 2 ]]; then
          echo "--only requires a comma-separated channel list" >&2
          exit 1
        fi
        ONLY="$2"
        shift 2
        ;;
      --only=*)
        ONLY="${1#--only=}"
        shift
        ;;
      --setup)
        SETUP_ONLY=1
        shift
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      *)
        echo "unknown argument: $1" >&2
        usage >&2
        exit 1
        ;;
    esac
  done
  ONLY="$(printf '%s' "$ONLY" | tr -d ' ')"
}

validate_only() {
  local ch
  local old_ifs="$IFS"
  if [[ -z "$ONLY" ]]; then
    return 0
  fi
  IFS=,
  for ch in $ONLY; do
    case "$ch" in
      github|homebrew|aur|scoop|winget|copr) ;;
      *)
        IFS="$old_ifs"
        die "unknown channel: $ch (github,homebrew,aur,scoop,winget,copr)"
        ;;
    esac
  done
  IFS="$old_ifs"
}

log() { printf '%s\n' "$*"; }
err() { printf '%s\n' "$*" >&2; }
die() { err "$*"; exit 1; }

have() { command -v "$1" >/dev/null 2>&1; }

channel_wanted() {
  local name="$1"
  if [[ "$name" == "github" && "$SKIP_GITHUB" == "1" ]]; then
    return 1
  elif [[ -z "$ONLY" ]]; then
    return 0
  else
    case ",$ONLY," in
      *",$name,"*) return 0 ;;
      *) return 1 ;;
    esac
  fi
}

skip_or_die() {
  local channel="$1"
  local reason="$2"
  if [[ -n "$ONLY" ]]; then
    die "$channel: $reason"
  else
    log "$channel: skipped ($reason)"
    return 1
  fi
}

read_version() {
  VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -n 1)"
  if [[ -z "$VERSION" ]]; then
    die "could not read version from Cargo.toml"
  fi
  OUT="$ROOT/release/$VERSION"
  BASE_URL="https://github.com/marthvon/datuma_k/releases/download/v${VERSION}"
  TAG="v$VERSION"
}

sha256_of() {
  local name="$1"
  local hash=""
  hash="$(awk -v f="$name" '$2 == f { print $1; found=1 } END { if (!found) exit 1 }' "$OUT/SHA256SUMS")"
  if [[ -z "$hash" ]]; then
    die "no SHA256SUMS entry for $name"
  fi
  printf '%s\n' "$hash"
}

load_checksums() {
  SHA256_MACOS_AARCH64="$(sha256_of datuma_k-macos-aarch64)"
  SHA256_MACOS_X86_64="$(sha256_of datuma_k-macos-x86_64)"
  SHA256_LINUX_AARCH64="$(sha256_of datuma_k-linux-aarch64)"
  SHA256_LINUX_X86_64="$(sha256_of datuma_k-linux-x86_64)"
  SHA256_WINDOWS_X86_64="$(sha256_of datuma_k-windows-x86_64.exe)"
  SHA256_WINDOWS_X86_64_UPPER="$(printf '%s' "$SHA256_WINDOWS_X86_64" | tr '[:lower:]' '[:upper:]')"
}

render_template() {
  local src="$1"
  local dest="$2"
  mkdir -p "$(dirname "$dest")"
  sed \
    -e "s|@VERSION@|${VERSION}|g" \
    -e "s|@BASE_URL@|${BASE_URL}|g" \
    -e "s|@REPO_HOMEPAGE@|${REPO_HOMEPAGE}|g" \
    -e "s|@SHA256_LINUX_X86_64@|${SHA256_LINUX_X86_64}|g" \
    -e "s|@SHA256_LINUX_AARCH64@|${SHA256_LINUX_AARCH64}|g" \
    -e "s|@SHA256_MACOS_X86_64@|${SHA256_MACOS_X86_64}|g" \
    -e "s|@SHA256_MACOS_AARCH64@|${SHA256_MACOS_AARCH64}|g" \
    -e "s|@SHA256_WINDOWS_X86_64_UPPER@|${SHA256_WINDOWS_X86_64_UPPER}|g" \
    -e "s|@SHA256_WINDOWS_X86_64@|${SHA256_WINDOWS_X86_64}|g" \
    -e "s|@NFPM_ARCH@|${NFPM_ARCH:-}|g" \
    -e "s|@NFPM_BIN@|${NFPM_BIN:-}|g" \
    -e "s|@LICENSE_FILE@|${LICENSE_FILE}|g" \
    -e "s|@CHANGELOG_DATE@|${CHANGELOG_DATE}|g" \
    "$src" > "$dest"
}

validate_artifacts() {
  local name
  if [[ ! -d "$OUT" ]]; then
    die "missing $OUT — run ./scripts/release.sh first"
  elif [[ ! -f "$OUT/SHA256SUMS" ]]; then
    die "missing $OUT/SHA256SUMS"
  fi
  for name in $BINARIES; do
    if [[ ! -f "$OUT/$name" ]]; then
      die "missing artifact $OUT/$name — run ./scripts/release.sh first"
    fi
  done
  load_checksums
  if have git && git -C "$ROOT" rev-parse "$TAG" >/dev/null 2>&1; then
    log "git tag $TAG exists"
  else
    log "warning: git tag $TAG not found locally (GitHub can still create the release)"
  fi
}

need_gh() {
  if have gh; then
    if gh auth status >/dev/null 2>&1 || [[ -n "${GH_TOKEN:-}" ]]; then
      return 0
    else
      return 1
    fi
  else
    return 1
  fi
}

ensure_github_repo() {
  local repo="$1"
  local desc="$2"
  if [[ "$DRY_RUN" == "1" ]]; then
    log "would ensure github repo $repo exists"
    return 0
  elif gh repo view "$repo" >/dev/null 2>&1; then
    log "github repo $repo exists"
  else
    log "creating github repo $repo"
    gh repo create "$repo" --public --description "$desc" --add-readme
  fi
}

ensure_github_repo_public() {
  local repo="$1"
  local vis=""
  if [[ "$DRY_RUN" == "1" ]]; then
    log "would ensure $repo is public"
    return 0
  fi
  vis="$(gh repo view "$repo" --json isPrivate --jq .isPrivate 2>/dev/null || true)"
  if [[ "$vis" == "true" ]]; then
    log "making $repo public (install channels cannot use a private repo)"
    gh repo edit "$repo" --visibility public --accept-visibility-change-consequences
  fi
}

git_use_gh_credentials() {
  local dir="$1"
  if [[ ! -d "$dir/.git" ]]; then
    return 0
  fi
  git -C "$dir" config --local --unset-all credential.helper 2>/dev/null || true
  git -C "$dir" config --local --unset-all credential.https://github.com.helper 2>/dev/null || true
  git -C "$dir" config --local credential.https://github.com.helper ""
  git -C "$dir" config --local --add credential.https://github.com.helper "!gh auth git-credential"
}

ensure_clone() {
  local repo="$1"
  local dir="$2"
  shift 2
  if [[ "$DRY_RUN" == "1" ]]; then
    log "would clone or update $repo into $dir"
    return 0
  elif [[ -d "$dir/.git" ]]; then
    git_use_gh_credentials "$dir"
    if git -C "$dir" rev-parse --verify HEAD >/dev/null 2>&1; then
      git -C "$dir" pull --ff-only || git -C "$dir" fetch origin
    else
      git -C "$dir" fetch origin || true
    fi
  else
    mkdir -p "$(dirname "$dir")"
    if [[ $# -gt 0 ]]; then
      gh repo clone "$repo" "$dir" -- "$@"
    else
      gh repo clone "$repo" "$dir"
    fi
    git_use_gh_credentials "$dir"
  fi
}

commit_and_push() {
  local dir="$1"
  local message="$2"
  local branch="${3:-}"
  if [[ "$DRY_RUN" == "1" ]]; then
    log "would commit and push in $dir: $message"
    return 0
  fi
  git_use_gh_credentials "$dir"
  git -C "$dir" add -A
  if git -C "$dir" diff --cached --quiet; then
    log "no new commit in $dir"
  else
    git -C "$dir" commit -m "$message"
  fi
  if ! git -C "$dir" rev-parse --verify HEAD >/dev/null 2>&1; then
    log "no commits to push in $dir"
    return 0
  fi
  if [[ -n "$branch" ]]; then
    git -C "$dir" push -u origin "$branch"
  else
    git -C "$dir" push -u origin HEAD
  fi
}

write_if_missing() {
  local dest="$1"
  if [[ -f "$dest" ]]; then
    return 0
  fi
  mkdir -p "$(dirname "$dest")"
  cat > "$dest"
}

nfpm_package() {
  local arch="$1"
  local bin="$2"
  local packager="$3"
  local target="$4"
  NFPM_ARCH="$arch"
  NFPM_BIN="$OUT/$bin"
  local cfg="$SCRATCH/nfpm-${arch}.yaml"
  render_template "$PACKAGING/nfpm/datuma-k.yaml.tpl" "$cfg"
  if [[ "$DRY_RUN" == "1" ]]; then
    log "would nfpm package --packager $packager --target $target"
    if have nfpm; then
      nfpm package --packager "$packager" --config "$cfg" --target "$target"
      log "built $target (dry-run keeps local packages)"
    fi
  else
    nfpm package --packager "$packager" --config "$cfg" --target "$target"
    log "built $target"
  fi
}

build_nfpm() {
  mkdir -p "$SCRATCH" "$OUT"
  if ! have nfpm; then
    if [[ "$DRY_RUN" == "1" ]]; then
      log "nfpm not on PATH; skipping package build in dry-run"
      return 0
    else
      die "nfpm not on PATH. Install: go install github.com/goreleaser/nfpm/v2/cmd/nfpm@latest
or download a binary from https://github.com/goreleaser/nfpm/releases"
    fi
  fi
  nfpm_package amd64 datuma_k-linux-x86_64 deb "$OUT/datuma-k_${VERSION}_amd64.deb"
  nfpm_package arm64 datuma_k-linux-aarch64 deb "$OUT/datuma-k_${VERSION}_arm64.deb"
  nfpm_package amd64 datuma_k-linux-x86_64 rpm "$OUT/datuma-k-${VERSION}-1.x86_64.rpm"
  nfpm_package arm64 datuma_k-linux-aarch64 rpm "$OUT/datuma-k-${VERSION}-1.aarch64.rpm"
  nfpm_package amd64 datuma_k-linux-musl-x86_64 apk "$OUT/datuma-k_${VERSION}_x86_64.apk"
  nfpm_package arm64 datuma_k-linux-musl-aarch64 apk "$OUT/datuma-k_${VERSION}_aarch64.apk"
}

publish_github() {
  local files=""
  local f
  if [[ "$DRY_RUN" != "1" ]] && ! need_gh; then
    skip_or_die github "gh not installed or not authenticated (GH_TOKEN or gh auth login)" || return 0
  fi
  build_nfpm
  for f in $BINARIES SHA256SUMS \
    "datuma-k_${VERSION}_amd64.deb" \
    "datuma-k_${VERSION}_arm64.deb" \
    "datuma-k-${VERSION}-1.x86_64.rpm" \
    "datuma-k-${VERSION}-1.aarch64.rpm" \
    "datuma-k_${VERSION}_x86_64.apk" \
    "datuma-k_${VERSION}_aarch64.apk"
  do
    if [[ -f "$OUT/$f" ]]; then
      files="$files $OUT/$f"
    fi
  done
  if [[ "$DRY_RUN" == "1" ]]; then
    log "would upload to GitHub Release $TAG:$files"
    if ! need_gh; then
      log "github: would need gh (GH_TOKEN or gh auth login)"
    fi
    return 0
  elif gh release view "$TAG" --repo "$GH_REPO" >/dev/null 2>&1; then
    log "uploading assets to existing release $TAG"
    # shellcheck disable=SC2086
    gh release upload "$TAG" $files --clobber --repo "$GH_REPO"
  else
    log "creating GitHub Release $TAG"
    # shellcheck disable=SC2086
    gh release create "$TAG" $files --title "$TAG" --generate-notes --repo "$GH_REPO"
  fi
}

setup_homebrew() {
  local dir="$PUBLISH_DIR/homebrew-datuma_k"
  ensure_github_repo "$HOMEBREW_REPO" "Homebrew tap for datuma_k"
  ensure_github_repo_public "$HOMEBREW_REPO"
  ensure_clone "$HOMEBREW_REPO" "$dir"
  if [[ "$DRY_RUN" == "1" ]]; then
    return 0
  fi
  mkdir -p "$dir/Formula"
  write_if_missing "$dir/README.md" <<EOF
# homebrew-datuma_k

    brew tap ${GH_OWNER}/datuma_k
    brew install datuma-k
EOF
}

publish_homebrew() {
  local dir="$PUBLISH_DIR/homebrew-datuma_k"
  local dest
  if [[ "$DRY_RUN" == "1" ]]; then
    dest="$SCRATCH/homebrew/datuma-k.rb"
    render_template "$PACKAGING/homebrew/datuma-k.rb.tpl" "$dest"
    log "rendered $dest"
    log "would create/clone $HOMEBREW_REPO into $dir, commit Formula/datuma-k.rb, push main"
    if ! need_gh; then
      log "homebrew: would need gh (authenticated)"
    fi
    return 0
  elif ! need_gh; then
    skip_or_die homebrew "gh not installed or not authenticated" || return 0
  else
    setup_homebrew
    dest="$dir/Formula/datuma-k.rb"
    render_template "$PACKAGING/homebrew/datuma-k.rb.tpl" "$dest"
    log "rendered $dest"
    commit_and_push "$dir" "datuma-k $VERSION"
  fi
}

write_srcinfo() {
  local dest="$1"
  local tab
  tab="$(printf '\t')"
  cat > "$dest" <<EOF
pkgbase = ${AUR_PACKAGE}
${tab}pkgdesc = Data contract plus templates that generate source
${tab}pkgver = ${VERSION}
${tab}pkgrel = 1
${tab}url = ${REPO_HOMEPAGE}
${tab}arch = x86_64
${tab}arch = aarch64
${tab}license = AGPL-3.0-only
${tab}provides = datuma_k
${tab}conflicts = datuma_k
${tab}options = !strip
${tab}options = !debug
${tab}source_x86_64 = ${AUR_PACKAGE}-${VERSION}-x86_64::${BASE_URL}/datuma_k-linux-x86_64
${tab}sha256sums_x86_64 = ${SHA256_LINUX_X86_64}
${tab}source_aarch64 = ${AUR_PACKAGE}-${VERSION}-aarch64::${BASE_URL}/datuma_k-linux-aarch64
${tab}sha256sums_aarch64 = ${SHA256_LINUX_AARCH64}

pkgname = ${AUR_PACKAGE}
EOF
}

setup_aur() {
  local dir="$PUBLISH_DIR/aur-datuma-k"
  local ssh_cmd="ssh -o BatchMode=yes -o ConnectTimeout=8 -o StrictHostKeyChecking=accept-new"
  if [[ -n "${AUR_SSH_KEY:-}" ]]; then
    if [[ ! -f "$AUR_SSH_KEY" ]]; then
      skip_or_die aur "AUR_SSH_KEY is not a file: $AUR_SSH_KEY" || return 1
    fi
    ssh_cmd="$ssh_cmd -i $AUR_SSH_KEY -o IdentitiesOnly=yes"
    export GIT_SSH_COMMAND="$ssh_cmd"
  fi
  if [[ "$DRY_RUN" == "1" ]]; then
    log "would clone $AUR_REMOTE into $dir"
    return 0
  elif [[ -d "$dir/.git" ]]; then
    git -C "$dir" pull --ff-only || git -C "$dir" fetch origin
    return 0
  elif git clone "$AUR_REMOTE" "$dir"; then
    log "cloned AUR $AUR_PACKAGE"
    return 0
  else
    skip_or_die aur "could not clone $AUR_REMOTE — create an AUR account, add an SSH key, then retry (see release/README.md)" || return 1
  fi
}

publish_aur() {
  local dir="$PUBLISH_DIR/aur-datuma-k"
  local dest_pkg dest_src
  if ! have git || ! have ssh; then
    skip_or_die aur "git and ssh are required" || return 0
  elif ! setup_aur; then
    return 0
  fi
  if [[ "$DRY_RUN" == "1" ]]; then
    dest_pkg="$SCRATCH/aur/PKGBUILD"
    dest_src="$SCRATCH/aur/.SRCINFO"
  else
    dest_pkg="$dir/PKGBUILD"
    dest_src="$dir/.SRCINFO"
  fi
  render_template "$PACKAGING/aur/PKGBUILD.tpl" "$dest_pkg"
  if [[ "$DRY_RUN" != "1" && -d "$dir" ]] && have makepkg; then
    (cd "$dir" && makepkg --printsrcinfo > "$dest_src")
  else
    write_srcinfo "$dest_src"
  fi
  log "rendered $dest_pkg and $dest_src"
  commit_and_push "$dir" "datuma-k $VERSION"
}

setup_scoop() {
  local dir="$PUBLISH_DIR/scoop-datuma_k"
  ensure_github_repo "$SCOOP_REPO" "Scoop bucket for datuma_k"
  ensure_github_repo_public "$SCOOP_REPO"
  ensure_clone "$SCOOP_REPO" "$dir"
  if [[ "$DRY_RUN" == "1" ]]; then
    return 0
  fi
  mkdir -p "$dir/bucket"
  write_if_missing "$dir/README.md" <<EOF
# scoop-datuma_k

    scoop bucket add datuma_k https://github.com/${SCOOP_REPO}
    scoop install datuma-k
EOF
}

publish_scoop() {
  local dir="$PUBLISH_DIR/scoop-datuma_k"
  local dest
  if [[ "$DRY_RUN" == "1" ]]; then
    dest="$SCRATCH/scoop/datuma-k.json"
    render_template "$PACKAGING/scoop/datuma-k.json.tpl" "$dest"
    log "rendered $dest"
    log "would create/clone $SCOOP_REPO into $dir, commit bucket/datuma-k.json, push main"
    if ! need_gh; then
      log "scoop: would need gh (authenticated)"
    fi
    return 0
  elif ! need_gh; then
    skip_or_die scoop "gh not installed or not authenticated" || return 0
  else
    setup_scoop
    dest="$dir/bucket/datuma-k.json"
    render_template "$PACKAGING/scoop/datuma-k.json.tpl" "$dest"
    log "rendered $dest"
    commit_and_push "$dir" "datuma-k $VERSION"
  fi
}

setup_winget() {
  local dir="$PUBLISH_DIR/winget-pkgs"
  if [[ "$WINGET_SKIP" == "1" ]]; then
    skip_or_die winget "WINGET_SKIP=1" || return 1
  elif [[ "$DRY_RUN" == "1" ]]; then
    log "would ensure fork $WINGET_FORK and sparse-clone into $dir"
    return 0
  elif ! need_gh; then
    skip_or_die winget "gh not installed or not authenticated" || return 1
  elif gh repo view "$WINGET_FORK" >/dev/null 2>&1; then
    log "winget fork $WINGET_FORK exists"
  else
    log "forking $WINGET_UPSTREAM to $WINGET_FORK"
    gh repo fork "$WINGET_UPSTREAM" --clone=false
  fi
  if [[ -d "$dir/.git" ]]; then
    git_use_gh_credentials "$dir"
    git -C "$dir" fetch origin
  else
    mkdir -p "$(dirname "$dir")"
    gh repo clone "$WINGET_FORK" "$dir" -- --filter=blob:none --sparse --depth 1
    git -C "$dir" sparse-checkout set manifests/d/datuma-k
    git_use_gh_credentials "$dir"
  fi
}

publish_winget() {
  local dir="$PUBLISH_DIR/winget-pkgs"
  local man_dir branch title existing
  local ver_tpl inst_tpl loc_tpl
  if ! setup_winget; then
    return 0
  fi
  branch="datuma-k-v$VERSION"
  man_dir="manifests/d/datuma-k/datuma-k/$VERSION"
  ver_tpl="datuma-k.datuma-k.yaml"
  inst_tpl="datuma-k.datuma-k.installer.yaml"
  loc_tpl="datuma-k.datuma-k.locale.en-US.yaml"
  if [[ "$DRY_RUN" == "1" ]]; then
    render_template "$PACKAGING/winget/$ver_tpl.tpl" "$SCRATCH/winget/$ver_tpl"
    render_template "$PACKAGING/winget/$inst_tpl.tpl" "$SCRATCH/winget/$inst_tpl"
    render_template "$PACKAGING/winget/$loc_tpl.tpl" "$SCRATCH/winget/$loc_tpl"
    log "rendered $SCRATCH/winget"
    log "would push $branch on $WINGET_FORK and open a PR against $WINGET_UPSTREAM"
    if ! need_gh; then
      log "winget: would need gh (authenticated)"
    fi
    return 0
  fi
  git -C "$dir" fetch origin master 2>/dev/null || git -C "$dir" fetch origin main
  if git -C "$dir" show-ref --verify --quiet refs/remotes/origin/master; then
    git -C "$dir" checkout -B "$branch" origin/master
  else
    git -C "$dir" checkout -B "$branch" origin/main
  fi
  mkdir -p "$dir/$man_dir"
  render_template "$PACKAGING/winget/$ver_tpl.tpl" "$dir/$man_dir/$ver_tpl"
  render_template "$PACKAGING/winget/$inst_tpl.tpl" "$dir/$man_dir/$inst_tpl"
  render_template "$PACKAGING/winget/$loc_tpl.tpl" "$dir/$man_dir/$loc_tpl"
  log "rendered $dir/$man_dir"
  git -C "$dir" add "$man_dir"
  if git -C "$dir" diff --cached --quiet; then
    log "no winget manifest changes"
  else
    if ls -d "$dir/manifests/d/datuma-k/datuma-k"/*/ >/dev/null 2>&1 && \
       find "$dir/manifests/d/datuma-k/datuma-k" -mindepth 1 -maxdepth 1 -type d ! -name "$VERSION" | grep -q .; then
      title="New version: ${WINGET_ID} version $VERSION"
    else
      title="New package: ${WINGET_ID} version $VERSION"
    fi
    git -C "$dir" commit -m "$title"
    git_use_gh_credentials "$dir"
    git -C "$dir" push -u origin "$branch"
    existing="$(gh pr list --repo "$WINGET_UPSTREAM" --head "${GH_OWNER}:${branch}" --json number --jq '.[0].number' 2>/dev/null || true)"
    if [[ -n "$existing" && "$existing" != "null" ]]; then
      log "winget PR already exists: https://github.com/${WINGET_UPSTREAM}/pull/${existing}"
    else
      gh pr create --repo "$WINGET_UPSTREAM" --head "${GH_OWNER}:${branch}" --title "$title" --body "$(cat <<EOF
Portable x64 installer for datuma-k ${VERSION}.

Built from ${REPO_HOMEPAGE}/releases/tag/${TAG}
EOF
)"
    fi
  fi
}

write_copr_config() {
  local dest="$PUBLISH_DIR/copr.conf"
  if [[ -n "${COPR_CONFIG:-}" ]]; then
    printf '%s\n' "$COPR_CONFIG"
    return 0
  elif [[ -n "${COPR_API_TOKEN:-}" && -n "${COPR_LOGIN:-}" ]]; then
    mkdir -p "$PUBLISH_DIR"
    cat > "$dest" <<EOF
[copr-cli]
login = ${COPR_LOGIN}
username = ${COPR_USERNAME:-$GH_OWNER}
token = ${COPR_API_TOKEN}
copr_url = ${COPR_URL:-https://copr.fedorainfracloud.org}
EOF
    printf '%s\n' "$dest"
    return 0
  elif [[ -f "${HOME}/.config/copr" ]]; then
    printf '%s\n' "${HOME}/.config/copr"
    return 0
  else
    return 1
  fi
}

copr_cli_cmd() {
  local cfg="$1"
  shift
  if [[ -n "$cfg" ]]; then
    copr-cli --config "$cfg" "$@"
  else
    copr-cli "$@"
  fi
}

copr_chroot_flag_array() {
  local c
  COPR_CHROOT_FLAGS=()
  for c in "${COPR_CHROOTS[@]}"; do
    COPR_CHROOT_FLAGS+=(--chroot "$c")
  done
}

verify_copr_chroots() {
  local cfg="$1"
  local available="" c
  available="$(copr_cli_cmd "$cfg" list-chroots 2>/dev/null || true)"
  if [[ -z "$available" ]]; then
    log "Copr list-chroots empty; using COPR_CHROOTS as-is"
    return 0
  fi
  for c in "${COPR_CHROOTS[@]}"; do
    if ! printf '%s\n' "$available" | grep -qx "$c"; then
      skip_or_die copr "Copr chroot $c is not available (copr-cli list-chroots)" || return 1
    fi
  done
  return 0
}

apply_copr_chroots() {
  local cfg="$1"
  copr_chroot_flag_array
  copr_cli_cmd "$cfg" modify datuma-k "${COPR_CHROOT_FLAGS[@]}" --follow-fedora-branching on
}

setup_copr() {
  local cfg=""
  if [[ "$DRY_RUN" == "1" ]]; then
    log "would ensure Copr project $COPR_PROJECT with chroots: ${COPR_CHROOTS[*]}"
    if ! have copr-cli; then
      log "copr: would need copr-cli (pip install copr-cli)"
    elif ! cfg="$(write_copr_config)"; then
      log "copr: would need ~/.config/copr or COPR_LOGIN+COPR_API_TOKEN"
    fi
    return 0
  elif ! have copr-cli; then
    skip_or_die copr "copr-cli not on PATH (pip install copr-cli)" || return 1
  elif ! cfg="$(write_copr_config)"; then
    skip_or_die copr "no Copr credentials; set COPR_LOGIN+COPR_API_TOKEN or place config at ~/.config/copr (https://copr.fedorainfracloud.org/api/)" || return 1
  elif ! verify_copr_chroots "$cfg"; then
    return 1
  elif copr_cli_cmd "$cfg" list 2>/dev/null | grep -Eq '(^|[[:space:]/])datuma-k([[:space:]]|$)'; then
    log "Copr project $COPR_PROJECT exists; syncing chroots"
    if apply_copr_chroots "$cfg"; then
      return 0
    else
      skip_or_die copr "copr-cli modify failed — enable Fedora 43/44 + rawhide chroots in the Copr web UI, then retry" || return 1
    fi
  else
    copr_chroot_flag_array
    if copr_cli_cmd "$cfg" create datuma-k \
        "${COPR_CHROOT_FLAGS[@]}" \
        --follow-fedora-branching on \
        --description "datuma_k packages" \
        --instructions "sudo dnf copr enable ${COPR_PROJECT} && sudo dnf install datuma-k"; then
      log "created Copr project $COPR_PROJECT"
      return 0
    else
      skip_or_die copr "copr-cli create failed — create marthvon/datuma-k in the Copr web UI, then retry" || return 1
    fi
  fi
}

publish_copr() {
  local cfg="" spec
  if ! setup_copr; then
    return 0
  fi
  spec="$SCRATCH/datuma-k-${VERSION}.spec"
  render_template "$PACKAGING/copr/datuma-k.spec.tpl" "$spec"
  log "rendered $spec"
  if [[ "$DRY_RUN" == "1" ]]; then
    log "would: copr-cli build $COPR_PROJECT $spec"
    return 0
  fi
  cfg="$(write_copr_config || true)"
  copr_cli_cmd "$cfg" build "$COPR_PROJECT" "$spec"
}

print_setup_notes() {
  cat <<EOF

One-time setup (if a channel was skipped):

  AUR
    1. Account at https://aur.archlinux.org/account/register/
    2. Add an SSH public key on https://aur.archlinux.org/account/
    3. ssh aur@aur.archlinux.org   # should print a welcome, no shell
    4. Optional: AUR_SSH_KEY=/path/to/key in release/publish.env
    5. First clone creates the empty datuma-k package if you are logged in:
         git clone ssh://aur@aur.archlinux.org/datuma-k.git

  Copr
    1. Download API config from https://copr.fedorainfracloud.org/api/
    2. Save as ~/.config/copr, or set COPR_LOGIN, COPR_USERNAME, COPR_API_TOKEN
    3. Fallback if copr-cli create fails: open
         https://copr.fedorainfracloud.org/coprs/${GH_OWNER}/datuma-k/
       create project datuma-k, then re-run this script

  WinGet
    Microsoft reviews PRs; merge is not automatic.
    Set WINGET_SKIP=1 to ignore this channel.
    Default fork: $WINGET_FORK (created from microsoft/winget-pkgs on first run)

Sibling GitHub repos (created when missing):
  $HOMEBREW_REPO
  $SCOOP_REPO
EOF
}

setup_all() {
  if [[ "$DRY_RUN" == "1" ]]; then
    log "would create $HOMEBREW_REPO and $SCOOP_REPO, clone them under $PUBLISH_DIR"
    log "would fork $WINGET_UPSTREAM to $WINGET_FORK (unless WINGET_SKIP=1)"
    log "would clone $AUR_REMOTE and ensure Copr project $COPR_PROJECT"
    print_setup_notes
    return 0
  elif ! need_gh; then
    die "--setup needs gh (authenticated) to create Homebrew/Scoop repos"
  fi
  log "== setup homebrew =="
  setup_homebrew
  log "== setup scoop =="
  setup_scoop
  log "== setup winget =="
  setup_winget || true
  log "== setup aur =="
  setup_aur || true
  log "== setup copr =="
  setup_copr || true
  print_setup_notes
}

main() {
  load_env
  parse_args "$@"
  DRY_RUN="${DRY_RUN:-0}"
  validate_only
  read_version
  LICENSE_FILE="$ROOT/LICENSE.md"
  CHANGELOG_DATE="$(date '+%a %b %d %Y')"
  mkdir -p "$PUBLISH_DIR" "$SCRATCH"

  log "version $VERSION"
  log "artifacts $OUT"

  if [[ "$SETUP_ONLY" == "1" ]]; then
    setup_all
    exit 0
  fi

  validate_artifacts

  if channel_wanted github; then
    log "== github =="
    publish_github
  fi
  if channel_wanted homebrew; then
    log "== homebrew =="
    publish_homebrew
  fi
  if channel_wanted aur; then
    log "== aur =="
    publish_aur
  fi
  if channel_wanted scoop; then
    log "== scoop =="
    publish_scoop
  fi
  if channel_wanted winget; then
    log "== winget =="
    publish_winget
  fi
  if channel_wanted copr; then
    log "== copr =="
    publish_copr
  fi

  log "done"
}

main "$@"
