import FinanzasKit
import SwiftData
import SwiftUI

@main
struct FinanzasApp: App {
    let container: ModelContainer

    init() {
        do { container = try FinanzasStore.onDisk() }
        catch { fatalError("No se pudo abrir la base de datos: \(error)") }
    }

    var body: some Scene {
        WindowGroup {
            RootView()
        }
        .modelContainer(container)
        .commands { ImportFromActualCommand(container: container) }
    }
}

enum Section: String, CaseIterable, Identifiable {
    case accounts = "Cuentas", budget = "Presupuesto", upcoming = "Próximos pagos"
    var id: Self { self }
    var icon: String {
        switch self {
        case .accounts: "creditcard"
        case .budget: "envelope"
        case .upcoming: "calendar"
        }
    }
}

struct RootView: View {
    @State private var section: Section? = .accounts

    var body: some View {
        NavigationSplitView {
            List(Section.allCases, selection: $section) { item in
                Label(item.rawValue, systemImage: item.icon)
            }
            .navigationSplitViewColumnWidth(min: 180, ideal: 200)
        } detail: {
            // Screens land in T3–T5.
            ContentUnavailableView(section?.rawValue ?? "", systemImage: section?.icon ?? "questionmark")
        }
    }
}
