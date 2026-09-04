name: datuma-k
arch: "@NFPM_ARCH@"
platform: linux
version: "@VERSION@"
release: "1"
section: utils
priority: optional
maintainer: "mamertvonn <https://github.com/marthvon>"
vendor: marthvon
description: |
  A data contract (*.dtct) plus templates (*.ngin) that generate source.
  Declare the shape once; each platform gets its own types, validation, and UI.
homepage: "@REPO_HOMEPAGE@"
license: AGPL-3.0-only
contents:
  - src: "@NFPM_BIN@"
    dst: /usr/bin/datuma_k
    file_info:
      mode: 0755
  - src: "@LICENSE_FILE@"
    dst: /usr/share/doc/datuma-k/LICENSE.md
    file_info:
      mode: 0644
