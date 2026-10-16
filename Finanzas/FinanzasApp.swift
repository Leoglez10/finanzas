import AppKit
import FinanzasKit
import SwiftData
import SwiftUI

@main
struct FinanzasApp: App {
    // nil when the store cannot be opened; the window then explains why instead of crashing.
    @State private var container: ModelContainer?
    @State private var mcpChanges: ExternalChanges?
    private let openError: String?
    private let tick = Timer.publish(every: 3, on: .main, in: .common).autoconnect()
    private let hourly = Timer.publish(every: 3_600, on: .main, in: .common).autoconnect()

    init() {
        do {
            let container = try FinanzasStore.onDisk()
            _container = State(initialValue: container)
            _mcpChanges = State(initialValue: ExternalChanges(context: container.mainContext))
            openError = nil
        } catch {
            _container = State(initialValue: nil)
            _mcpChanges = State(initialValue: nil)
            openError = error.localizedDescription
        }
    }

    var body: some Scene {
        WindowGroup {
            if let container {
                RootView()
                    .onReceive(tick) { _ in reloadIfMCPWrote() }
                    .onReceive(hourly) { _ in runAutomations() }
                    .task { await AutomationRunner.run(container.mainContext) }
                    .modelContainer(container)
            } else {
                StoreErrorView(message: openError ?? "")
            }
        }
        .commands { ImportFromActualCommand(container: container) }

        Settings {
            if let container { SettingsView().modelContainer(container) }
        }
    }

    private func runAutomations() {
        guard let container else { return }
        Task { await AutomationRunner.run(container.mainContext) }
    }

    /// The MCP helper writes from its own process; the main context would not see those rows,
    /// so a fresh container is opened when its history shows MCP-authored changes.
    /// While a sheet is open its picked models belong to the current container, so the reload waits.
    private func reloadIfMCPWrote() {
        guard let current = container,
              !NSApp.windows.contains(where: { $0.attachedSheet != nil }),
              (try? mcpChanges?.poll()) == true else { return }
        current.mainContext.saveOrAlert()
        guard let fresh = try? FinanzasStore.onDisk() else { return }
        container = fresh
        mcpChanges = ExternalChanges(context: fresh.mainContext)
        runAutomations()
    }
}

private struct StoreErrorView: View {
    let message: String

    var body: some View {
        ContentUnavailableView {
            Label("No se pudo abrir la base de datos", systemImage: "externaldrive.badge.exclamationmark")
        } description: {
            Text("\(message)\n\(FinanzasStore.storeURL.path)").textSelection(.enabled)
        } actions: {
            Button("Mostrar en Finder") {
                NSWorkspace.shared.activateFileViewerSelecting([FinanzasStore.storeURL.deletingLastPathComponent()])
            }
        }
        .frame(minWidth: 480, minHeight: 320)
    }
}

enum SidebarItem: Hashable {
    case overview, budget, upcoming, allAccounts
    case account(PersistentIdentifier)
}

struct RootView: View {
    @Environment(\.modelContext) private var context
    @Environment(\.undoManager) private var undoManager
    @Query(sort: \Account.sortOrder) private var accounts: [Account]
    @Query private var schedules: [Schedule]
    @State private var selection: SidebarItem? = .overview
    @State private var addingAccount = false
    @State private var newAccountName = ""

    var body: some View {
        NavigationSplitView {
            VStack(alignment: .leading, spacing: 0) {
                HStack(spacing: 12) {
                    Image(systemName: "chart.pie.fill")
                        .font(.title2).foregroundStyle(.white)
                        .padding(10)
                        .background(FinanceTheme.hero, in: RoundedRectangle(cornerRadius: 12))
                        .accessibilityHidden(true)
                    VStack(alignment: .leading, spacing: 3) {
                        Text("Finanzas").font(.title2.bold())
                        Text("Tu dinero, en orden").font(.caption).foregroundStyle(.secondary)
                    }
                }
                .padding(20)
                List(selection: $selection) {
                Label("Resumen", systemImage: "square.grid.2x2").tag(SidebarItem.overview)
                Label("Presupuesto", systemImage: "envelope").tag(SidebarItem.budget)
                Label("Próximos pagos", systemImage: "calendar").tag(SidebarItem.upcoming)

                Section {
                    AccountRow(name: "Todas", icon: "tray.full", balance: openAccounts.reduce(0) { $0 + $1.balance })
                        .tag(SidebarItem.allAccounts)
                    ForEach(openAccounts) { account in
                        AccountRow(name: account.name, icon: account.offBudget ? "chart.line.uptrend.xyaxis" : "creditcard",
                                   balance: account.balance)
                            .tag(SidebarItem.account(account.persistentModelID))
                    }
                    Button { newAccountName = ""; addingAccount = true } label: {
                        Label("Nueva cuenta", systemImage: "plus.circle.fill")
                    }
                    .buttonStyle(.borderless)
                        .foregroundStyle(FinanceTheme.petroleum)
                } header: {
                    Text("Cuentas")
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
                .listStyle(.sidebar)
                .scrollContentBackground(.hidden)
            }
            .background(FinanceTheme.card)
            .navigationSplitViewColumnWidth(min: 240, ideal: 270)
            .alert("Nueva cuenta", isPresented: $addingAccount) {
                TextField("Nombre", text: $newAccountName)
                Button("Crear", action: addAccount)
                    .disabled(newAccountName.trimmingCharacters(in: .whitespaces).isEmpty)
                Button("Cancelar", role: .cancel) {}
            }
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
            case .overview:
                OverviewView()
            case .budget:
                BudgetView()
            case .upcoming:
                UpcomingView()
            }
        }
        .tint(FinanceTheme.petroleum)
        // Edit ▸ Undo (⌘Z) reverts deletes and edits; re-run when an MCP reload swaps the context.
        .task(id: ObjectIdentifier(context)) { context.undoManager = undoManager }
    }

    private var scheduleSignature: [String] {
        schedules.map { "\($0.persistentModelID.hashValue)|\($0.effectiveNextDate?.timeIntervalSince1970 ?? 0)|\($0.autoPost)|\($0.amount)|\($0.name)" }
    }

    private func addAccount() {
        let name = newAccountName.trimmingCharacters(in: .whitespaces)
        guard !name.isEmpty else { return }
        let account = Account(name: name, sortOrder: (accounts.map(\.sortOrder).max() ?? 0) + 1)
        context.insert(account)
        context.saveOrAlert()
        selection = .account(account.persistentModelID)
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
                .foregroundStyle(balance < 0 ? FinanceTheme.coral : Color.secondary)
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
