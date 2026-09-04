pkgname=datuma-k
pkgver=@VERSION@
pkgrel=1
pkgdesc="Data contract plus templates that generate source"
arch=('x86_64' 'aarch64')
url="@REPO_HOMEPAGE@"
license=('AGPL-3.0-only')
provides=('datuma_k')
conflicts=('datuma_k')
options=('!strip' '!debug')
source_x86_64=("${pkgname}-${pkgver}-x86_64::@BASE_URL@/datuma_k-linux-x86_64")
source_aarch64=("${pkgname}-${pkgver}-aarch64::@BASE_URL@/datuma_k-linux-aarch64")
sha256sums_x86_64=('@SHA256_LINUX_X86_64@')
sha256sums_aarch64=('@SHA256_LINUX_AARCH64@')

package() {
  install -Dm755 "${pkgname}-${pkgver}-${CARCH}" "${pkgdir}/usr/bin/datuma_k"
}
