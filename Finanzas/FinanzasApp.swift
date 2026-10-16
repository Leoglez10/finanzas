import AppKit
import FinanzasKit
import SwiftData
import SwiftUI

@main
struct FinanzasApp: App {
    @State private var container: ModelContainer
    @State private var mcpChanges: ExternalChanges
    private let tick = Timer.publish(every: 3, on: .main, in: .common).autoconnect()
    private let hourly = Timer.publish(every: 3_600, on: .main, in: .common).autoconnect()

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
                .onReceive(hourly) { _ in runAutomations() }
                .task { await AutomationRunner.run(container.mainContext) }
        }
        .modelContainer(container)
        .commands { ImportFromActualCommand(container: container) }

        Settings {
            SettingsView().modelContainer(container)
        }
    }

    private func runAutomations() { Task { await AutomationRunner.run(container.mainContext) } }

    /// The MCP helper writes from its own process; the main context would not see those rows,
    /// so a fresh container is opened when its history shows MCP-authored changes.
    /// While a sheet is open its picked models belong to the current container, so the reload waits.
    private func reloadIfMCPWrote() {
        guard !NSApp.windows.contains(where: { $0.attachedSheet != nil }),
              (try? mcpChanges.poll()) == true else { return }
        container.mainContext.saveOrAlert()
        guard let fresh = try? FinanzasStore.onDisk() else { return }
        container = fresh
        mcpChanges = ExternalChanges(context: fresh.mainContext)
        runAutomations()
    }
}

enum SidebarItem: Hashable {
    case budget, upcoming, allAccounts
    case account(PersistentIdentifier)
}

struct RootView: View {
    @Environment(\.modelContext) private var context
    @Query(sort: \Account.sortOrder) private var accounts: [Account]
    @Query private var schedules: [Schedule]
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
            // Posting, skipping or editing a schedule changes what is due and what to remind about.
            .onChange(of: scheduleSignature) { Task { await AutomationRunner.run(context) } }
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

    private var scheduleSignature: [String] {
        schedules.map { "\($0.persistentModelID.hashValue)|\($0.effectiveNextDate?.timeIntervalSince1970 ?? 0)|\($0.autoPost)|\($0.amount)|\($0.name)" }
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

extension ModelContext {
    /// Saves now; a failure (for example a conflict with an MCP write) is shown instead of being lost silently.
    @MainActor func saveOrAlert() {
        do { try save() } catch {
            rollback()
            let alert = NSAlert()
            alert.alertStyle = .warning
            alert.messageText = "No se pudo guardar el cambio"
            alert.informativeText = "Puede que la IA haya modificado los mismos datos. Revisa y vuelve a intentarlo.\n\n\(error.localizedDescription)"
            alert.runModal()
        }
    }
}
