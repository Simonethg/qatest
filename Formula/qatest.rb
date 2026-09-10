class Qatest < Formula
  desc "Agent runtime for QA"
  homepage "https://github.com/Simonethg/qatest"
  version "0.1.0"
  license "Apache-2.0"

  on_macos do
    on_arm do
      url "https://github.com/Simonethg/qatest/releases/download/v0.1.0/qatest-macos-aarch64"
      sha256 "3093d8708f45771311da8ad857ec033834192e5095d3a8649e429b5b6d4e17d2"
    end
    on_intel do
      url "https://github.com/Simonethg/qatest/releases/download/v0.1.0/qatest-macos-x86_64"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/Simonethg/qatest/releases/download/v0.1.0/qatest-linux-aarch64"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
    on_intel do
      url "https://github.com/Simonethg/qatest/releases/download/v0.1.0/qatest-linux-x86_64"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
  end

  def install
    bin.install "qatest-macos-aarch64" => "qatest" if Hardware::CPU.arm? && OS.mac?
    bin.install "qatest-macos-x86_64" => "qatest" if Hardware::CPU.intel? && OS.mac?
    bin.install "qatest-linux-aarch64" => "qatest" if Hardware::CPU.arm? && OS.linux?
    bin.install "qatest-linux-x86_64" => "qatest" if Hardware::CPU.intel? && OS.linux?
  end

  test do
    system "#{bin}/qatest", "--version"
  end
end
