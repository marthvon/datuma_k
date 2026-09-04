class DatumaK < Formula
  desc "Data contract plus templates that generate source"
  homepage "@REPO_HOMEPAGE@"
  version "@VERSION@"
  license "AGPL-3.0-only"

  on_macos do
    on_arm do
      url "@BASE_URL@/datuma_k-macos-aarch64"
      sha256 "@SHA256_MACOS_AARCH64@"
    end
    on_intel do
      url "@BASE_URL@/datuma_k-macos-x86_64"
      sha256 "@SHA256_MACOS_X86_64@"
    end
  end

  on_linux do
    on_arm do
      url "@BASE_URL@/datuma_k-linux-aarch64"
      sha256 "@SHA256_LINUX_AARCH64@"
    end
    on_intel do
      url "@BASE_URL@/datuma_k-linux-x86_64"
      sha256 "@SHA256_LINUX_X86_64@"
    end
  end

  def install
    bin.install Dir["datuma_k*"].first => "datuma_k"
  end

  test do
    output = shell_output("#{bin}/datuma_k 2>&1", 1)
    assert_match "usage: datuma_k", output
  end
end
