// swift-tools-version:6.2
import PackageDescription

// NOVA AI — application native macOS.
// Chaque module a une responsabilité unique ; l'interface (NovaApp) ne parle
// aux modèles et aux outils qu'à travers NovaCore.
let package = Package(
    name: "NovaAI",
    defaultLocalization: "fr",
    platforms: [.macOS(.v26)],
    products: [
        .executable(name: "NovaAI", targets: ["NovaApp"]),
        .executable(name: "nova-apple-helper", targets: ["NovaAppleHelper"]),
    ],
    targets: [
        .target(
            name: "NovaStorage",
            linkerSettings: [.linkedLibrary("sqlite3")]
        ),
        .target(name: "NovaTools"),
        .target(name: "NovaModels"),
        .target(
            name: "NovaCore",
            dependencies: ["NovaStorage", "NovaTools", "NovaModels"]
        ),
        .executableTarget(
            name: "NovaApp",
            dependencies: ["NovaCore", "NovaStorage", "NovaTools", "NovaModels"]
        ),
        .executableTarget(
            name: "NovaAppleHelper",
            dependencies: ["NovaModels", "NovaTools"]
        ),
        .testTarget(name: "NovaToolsTests", dependencies: ["NovaTools"]),
        .testTarget(name: "NovaStorageTests", dependencies: ["NovaStorage"]),
        .testTarget(name: "NovaCoreTests", dependencies: ["NovaCore"]),
    ],
    swiftLanguageModes: [.v5]
)
