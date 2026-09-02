#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
if [[ -z "$VERSION" ]]; then
  echo "could not read version from Cargo.toml" >&2
  exit 1
fi

OUT="$ROOT/release/$VERSION"
DOCKER_IMAGE="rust:1.92-bookworm"
mkdir -p "$OUT"

copy_bin() {
  local src="$1"
  local dest="$2"
  cp "$src" "$dest"
  chmod +x "$dest"
}

echo "building macOS aarch64 (native)"
cargo build --release
copy_bin "$ROOT/target/release/datuma_k" "$OUT/datuma_k-macos-aarch64"

echo "building macOS x86_64"
rustup target add x86_64-apple-darwin
cargo build --release --target x86_64-apple-darwin
copy_bin "$ROOT/target/x86_64-apple-darwin/release/datuma_k" "$OUT/datuma_k-macos-x86_64"

docker_ok=1
if ! command -v docker >/dev/null 2>&1; then
  echo "docker not found; skipping Linux and Windows" >&2
  docker_ok=0
elif ! docker info >/dev/null 2>&1; then
  echo "docker daemon is not running; skipping Linux and Windows" >&2
  docker_ok=0
fi

if [[ "$docker_ok" -eq 1 ]]; then
  docker_linux() {
    local platform="$1"
    local artifact="$2"
    echo "building $artifact via docker ($platform)"
    docker run --rm \
      --platform "$platform" \
      -v "$ROOT:/src" \
      -e CARGO_TARGET_DIR=/tmp/dk-target \
      -w /src \
      "$DOCKER_IMAGE" \
      bash -c "cargo build --release && cp /tmp/dk-target/release/datuma_k /src/release/${VERSION}/${artifact}"
    chmod +x "$OUT/$artifact"
  }

  docker_linux linux/arm64 datuma_k-linux-aarch64
  docker_linux linux/amd64 datuma_k-linux-x86_64

  echo "building datuma_k-windows-x86_64.exe via docker"
  docker run --rm \
    --platform linux/amd64 \
    -v "$ROOT:/src" \
    -e CARGO_TARGET_DIR=/tmp/dk-target \
    -w /src \
    "$DOCKER_IMAGE" \
    bash -c "apt-get update && apt-get install -y gcc-mingw-w64-x86-64 && rustup target add x86_64-pc-windows-gnu && cargo build --release --target x86_64-pc-windows-gnu && cp /tmp/dk-target/x86_64-pc-windows-gnu/release/datuma_k.exe /src/release/${VERSION}/datuma_k-windows-x86_64.exe"
fi

echo "writing SHA256SUMS"
(
  cd "$OUT"
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 datuma_k-*
  else
    sha256sum datuma_k-*
  fi
) > "$OUT/SHA256SUMS"

echo "release artifacts in $OUT"
ls -l "$OUT"
