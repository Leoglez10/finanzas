# Homebrew cask for the macOS app. This repository is its own tap:
#   brew tap leoglez10/finanzas https://github.com/Leoglez10/finanzas
#   brew trust leoglez10/finanzas
#   brew install --cask finanzas
# .github/workflows/release.yml rewrites `version` and `sha256` after each release.
cask "finanzas" do
  version "0.6.1"
  sha256 "e1199623b10e633040b8fcbcbcb62c80ad9359b05d104dd6e1b5b520d37f4066"

  url "https://github.com/Leoglez10/finanzas/releases/download/v#{version}/Finanzas-macos-arm64.dmg"
  name "Finanzas"
  desc "Personal finance app with accounts, budget, scheduled payments and an MCP server"
  homepage "https://github.com/Leoglez10/finanzas"

  livecheck do
    url :url
    strategy :github_latest
  end

  # The app updates itself from GitHub Releases; `brew upgrade` leaves it alone unless run with --greedy.
  auto_updates true

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
