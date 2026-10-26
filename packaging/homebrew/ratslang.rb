# Maintainer: stelzo <stelzo@steado.de>
# Template file. CI fills in the version and one checksum per downloaded file before publishing to the tap.
class Ratslang < Formula
  desc "Configuration language for building physical systems in Rust: CLI and language server"
  homepage "https://stelzo.codeberg.page/ratslang/"
  version "VERSION_PLACEHOLDER"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    on_arm do
      url "https://codeberg.org/stelzo/ratslang/releases/download/v#{version}/ratslang-aarch64-apple-darwin"
      sha256 "RATSLANG_SHA256_PLACEHOLDER"

      resource "ratslang-lsp" do
        url "https://codeberg.org/stelzo/ratslang/releases/download/v#{version}/ratslang-lsp-aarch64-apple-darwin"
        sha256 "RATSLANG_LSP_SHA256_PLACEHOLDER"
      end
    end
  end

  def install
    bin.install "ratslang-aarch64-apple-darwin" => "ratslang"
    resource("ratslang-lsp").stage { bin.install "ratslang-lsp-aarch64-apple-darwin" => "ratslang-lsp" }
  end

  test do
    system bin/"ratslang", "--version"
    (testpath/"rat.rl").write("rat.mood = CURIOUS\n")
    system bin/"ratslang", "check", testpath/"rat.rl"
  end
end
