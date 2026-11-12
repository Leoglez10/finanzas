# Homebrew cask for the macOS app. This repository is its own tap:
#   brew tap leoglez10/finanzas https://github.com/Leoglez10/finanzas
#   brew trust leoglez10/finanzas
#   brew install --cask finanzas
# .github/workflows/release.yml rewrites `version` and `sha256` after each release.
cask "finanzas" do
  version "0.1.0"
  sha256 "c4155d9082647e3b3c7d635eba2f49b23dc7b95fc8accce4fa8918433f480075"

  url "https://github.com/Leoglez10/finanzas/releases/download/v#{version}/Finanzas-macos-arm64.dmg"
  name "Finanzas"
  desc "Personal finance app with accounts, budget, scheduled payments and an MCP server"
  homepage "https://github.com/Leoglez10/finanzas"

  livecheck do
    url :url
    strategy :github_latest
  end

  depends_on arch: :arm64
  depends_on macos: ">= :big_sur"

  app "Finanzas.app"
  binary "#{appdir}/Finanzas.app/Contents/MacOS/finanzas-mcp"

  # The app is ad-hoc signed, not notarized: without this, Gatekeeper blocks the first launch.
  postflight do
    system_command "/usr/bin/xattr", args: ["-dr", "com.apple.quarantine", "#{appdir}/Finanzas.app"]
  end

  # No `zap`: ~/Library/Application Support/Finanzas holds the user's financial data.
end
