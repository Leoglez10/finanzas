import FinanzasKit
import SwiftData
import SwiftUI

@main
struct FinanzasApp: App {
    @State private var container: ModelContainer
    @State private var mcpChanges: ExternalChanges
    private let tick = Timer.publish(every: 3, on: .main, in: .common).autoconnect()

    init() {
        let container: ModelContainer
        do { container = try FinanzasStore.onDisk() }
        catch { fatalError("No se pudo abrir la base de datos: \(error)") }
        _container = State(initialValue: container)
        _mcpChanges = State(initialValue: ExternalChanges(context: container.mainContext))
    }

    var body: some Scene {
        WindowGroup {
            RootView()
                .onReceive(tick) { _ in reloadIfMCPWrote() }
        }
        .modelContainer(container)
        .commands { ImportFromActualCommand(container: container) }
    }

    /// The MCP helper writes from its own process; the main context would not see those rows,
    /// so a fresh container is opened when its history shows MCP-authored changes.
    private func reloadIfMCPWrote() {
        guard (try? mcpChanges.poll()) == true, let fresh = try? FinanzasStore.onDisk() else { return }
        container = fresh
        mcpChanges = ExternalChanges(context: fresh.mainContext)
    }
}

enum SidebarItem: Hashable {
    case budget, upcoming, allAccounts
    case account(PersistentIdentifier)
}

struct RootView: View {
    @Environment(\.modelContext) private var context
    @Query(sort: \Account.sortOrder) private var accounts: [Account]
    @State private var selection: SidebarItem? = .allAccounts

    var body: some View {
        NavigationSplitView {
            List(selection: $selection) {
                Label("Presupuesto", systemImage: "envelope").tag(SidebarItem.budget)
                Label("Próximos pagos", systemImage: "calendar").tag(SidebarItem.upcoming)

                Section("Cuentas") {
                    AccountRow(name: "Todas", icon: "tray.full", balance: openAccounts.reduce(0) { $0 + $1.balance })
                        .tag(SidebarItem.allAccounts)
                    ForEach(openAccounts) { account in
                        AccountRow(name: account.name, icon: account.offBudget ? "chart.line.uptrend.xyaxis" : "creditcard",
                                   balance: account.balance)
                            .tag(SidebarItem.account(account.persistentModelID))
                    }
                }
                if !closedAccounts.isEmpty {
                    Section("Cerradas") {
                        ForEach(closedAccounts) { account in
                            AccountRow(name: account.name, icon: "archivebox", balance: account.balance)
                                .tag(SidebarItem.account(account.persistentModelID))
                        }
                    }
                }
            }
            .navigationSplitViewColumnWidth(min: 200, ideal: 240)
        } detail: {
            switch selection {
            case .allAccounts, nil:
                EntriesView(account: nil)
            case .account(let id):
                if let account = context.model(for: id) as? Account {
                    EntriesView(account: account).id(id)
                }
            case .budget:
                BudgetView()
            case .upcoming:
                UpcomingView()
            }
        }
    }

    private var openAccounts: [Account] { accounts.filter { !$0.closed } }
    private var closedAccounts: [Account] { accounts.filter(\.closed) }
}

struct AccountRow: View {
    let name: String
    let icon: String
    let balance: Int

    var body: some View {
        LabeledContent {
            Text(Money.format(balance)).monospacedDigit()
                .foregroundStyle(balance < 0 ? .red : .secondary)
        } label: {
            Label(name, systemImage: icon)
        }
    }
}
