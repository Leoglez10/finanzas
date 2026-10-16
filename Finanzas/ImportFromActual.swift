import AppKit
import FinanzasKit
import SwiftData
import SwiftUI

/// File › Importar desde Actual… — accepts Actual's `db.sqlite` or its export zip / server `.blob`.
struct ImportFromActualCommand: Commands {
    /// nil when the store failed to open; the command is then disabled.
    let container: ModelContainer?

    var body: some Commands {
        CommandGroup(replacing: .importExport) {
            Button("Importar desde Actual…") { run() }.disabled(container == nil)
        }
    }

    @MainActor private func run() {
        guard let container else { return }
        let panel = NSOpenPanel()
        panel.title = "Selecciona el db.sqlite o el respaldo .zip de Actual"
        panel.allowsMultipleSelection = false
        guard panel.runModal() == .OK, let url = panel.url else { return }

        let alert = NSAlert()
        do {
            let summary = try ActualImporter.importDatabase(at: try databaseURL(for: url), into: container.mainContext)
            alert.messageText = "Importación completa"
            alert.informativeText = """
                \(summary.accounts) cuentas, \(summary.categories) categorías, \(summary.payees) comercios, \
                \(summary.entries) movimientos y \(summary.schedules) pagos programados.
                """
        } catch ActualImporter.ImportError.storeNotEmpty {
            alert.alertStyle = .warning
            alert.messageText = "Ya tienes datos"
            alert.informativeText = "La importación solo funciona en una base vacía para no duplicar movimientos."
        } catch {
            alert.alertStyle = .critical
            alert.messageText = "No se pudo importar"
            alert.informativeText = "\(error)"
        }
        alert.runModal()
    }

    /// Returns a SQLite file, unzipping Actual's export (zip containing `db.sqlite`) when needed.
    private func databaseURL(for url: URL) throws -> URL {
        let header = try FileHandle(forReadingFrom: url).read(upToCount: 16) ?? Data()
        if header.starts(with: Data("SQLite format 3".utf8)) { return url }

        let dir = FileManager.default.temporaryDirectory.appending(path: "actual-import-\(UUID().uuidString)")
        let ditto = Process()
        ditto.executableURL = URL(filePath: "/usr/bin/ditto")
        ditto.arguments = ["-x", "-k", url.path, dir.path]
        try ditto.run()
        ditto.waitUntilExit()
        let db = dir.appending(path: "db.sqlite")
        guard ditto.terminationStatus == 0, FileManager.default.fileExists(atPath: db.path) else {
            throw CocoaError(.fileReadCorruptFile, userInfo: [NSLocalizedDescriptionKey: "El archivo no es un respaldo de Actual."])
        }
        return db
    }
}
