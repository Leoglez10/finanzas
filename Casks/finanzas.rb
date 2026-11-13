# Homebrew cask for the macOS app. This repository is its own tap:
#   brew tap leoglez10/finanzas https://github.com/Leoglez10/finanzas
#   brew trust leoglez10/finanzas
#   brew install --cask finanzas
# .github/workflows/release.yml rewrites `version` and `sha256` after each release.
cask "finanzas" do
  version "0.2.1"
  sha256 "2879439e610287d2a61ef73ab028258d5e3c90976bb9250930b53c7f4f41eb0c"

  url "https://github.com/Leoglez10/finanzas/releases/download/v#{version}/Finanzas-macos-arm64.dmg"
  name "Finanzas"
  desc "Personal finance app with accounts, budget, scheduled payments and an MCP server"
  homepage "https://github.com/Leoglez10/finanzas"

  livecheck do
    url :url
    strategy :github_latest
  end

  depends_on arch: :arm64
  depends_on :macos

  app "Finanzas.app"
  binary "#{appdir}/Finanzas.app/Contents/MacOS/finanzas-mcp"

  # The app is ad-hoc signed, not notarized: without this, Gatekeeper blocks the first launch.
  postflight_steps do
    run "/usr/bin/xattr", args:           ["-dr", "com.apple.quarantine", "{{appdir}}/Finanzas.app"],
                          writable_paths: "{{appdir}}/Finanzas.app"
  end

  # No `zap`: ~/Library/Application Support/Finanzas holds the user's financial data.
end
