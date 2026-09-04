# yaml-language-server: $schema=https://aka.ms/winget-manifest.installer.1.9.0.schema.json
PackageIdentifier: datuma-k.datuma-k
PackageVersion: @VERSION@
InstallerLocale: en-US
InstallerType: portable
Commands:
  - datuma_k
Installers:
  - Architecture: x64
    InstallerType: portable
    InstallerUrl: @BASE_URL@/datuma_k-windows-x86_64.exe
    InstallerSha256: @SHA256_WINDOWS_X86_64_UPPER@
    Commands:
      - datuma_k
ManifestType: installer
ManifestVersion: 1.9.0
