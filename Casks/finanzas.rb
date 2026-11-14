# Homebrew cask for the macOS app. This repository is its own tap:
#   brew tap leoglez10/finanzas https://github.com/Leoglez10/finanzas
#   brew trust leoglez10/finanzas
#   brew install --cask finanzas
# .github/workflows/release.yml rewrites `version` and `sha256` after each release.
cask "finanzas" do
  version "0.3.0"
  sha256 "972d344967decfa038fb9f92c3cf74d6e1873e64e0b2ce167ee8e8c6826ba5be"

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
