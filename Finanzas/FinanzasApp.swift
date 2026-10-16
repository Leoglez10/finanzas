import FinanzasKit
import SwiftData
import SwiftUI

@main
struct FinanzasApp: App {
    let container: ModelContainer

    init() {
        do { container = try FinanzasStore.onDisk() }
        catch { fatalError("Could not open the data store: \(error)") }
    }

    var body: some Scene {
        WindowGroup {
            RootView()
        }
        .modelContainer(container)
    }
}

enum Section: String, CaseIterable, Identifiable {
    case accounts = "Accounts", budget = "Budget", upcoming = "Upcoming"
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
