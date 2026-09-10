class Qat < Formula
  desc "Agent runtime for QA"
  homepage "https://github.com/Simonethg/qat"
  version "0.1.0"
  license "Apache-2.0"

  on_macos do
    on_arm do
      url "https://github.com/Simonethg/qat/releases/download/v0.1.0/qat-macos-aarch64"
      sha256 "eb5cc657efb9483e1d08e65962ad6c18fcfec562c35f4a5635b46427a4cdcefc"
    end
    on_intel do
      url "https://github.com/Simonethg/qat/releases/download/v0.1.0/qat-macos-x86_64"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/Simonethg/qat/releases/download/v0.1.0/qat-linux-aarch64"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
    on_intel do
      url "https://github.com/Simonethg/qat/releases/download/v0.1.0/qat-linux-x86_64"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
  end

  def install
    bin.install "qat-macos-aarch64" => "qat" if Hardware::CPU.arm? && OS.mac?
    bin.install "qat-macos-x86_64" => "qat" if Hardware::CPU.intel? && OS.mac?
    bin.install "qat-linux-aarch64" => "qat" if Hardware::CPU.arm? && OS.linux?
    bin.install "qat-linux-x86_64" => "qat" if Hardware::CPU.intel? && OS.linux?
  end

  test do
    system "#{bin}/qat", "--version"
  end
end
