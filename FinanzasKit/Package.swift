// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "FinanzasKit",
    platforms: [.macOS(.v15)],
    products: [.library(name: "FinanzasKit", targets: ["FinanzasKit"])],
    targets: [
        .target(name: "FinanzasKit"),
        .testTarget(name: "FinanzasKitTests", dependencies: ["FinanzasKit"]),
    ]
)
