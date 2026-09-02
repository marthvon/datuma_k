# datuma_k releases

Binaries are built into `release/<version>/` by [`scripts/release.sh`](../scripts/release.sh). That directory is gitignored; run the script locally.

```sh
./scripts/release.sh
```

Requires a current Rust toolchain on the host. Linux and Windows artifacts need Docker (Linux `amd64` on Apple Silicon uses QEMU and is slow). If Docker is missing or the daemon is down, macOS binaries are still produced.

## 1.0.0 artifacts

| File | Platform |
| --- | --- |
| `datuma_k-macos-aarch64` | macOS Apple Silicon |
| `datuma_k-macos-x86_64` | macOS Intel |
| `datuma_k-linux-aarch64` | Linux arm64 |
| `datuma_k-linux-x86_64` | Linux x86_64 |
| `datuma_k-windows-x86_64.exe` | Windows x86_64 |
| `SHA256SUMS` | checksums of the binaries above |
