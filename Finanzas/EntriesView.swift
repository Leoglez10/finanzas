import FinanzasKit
import SwiftData
import SwiftUI

// ObjC also exports a `Category` type; pin the model one for the app target.
typealias Category = FinanzasKit.Category

let dayFormat = Date.FormatStyle.dateTime.day(.twoDigits).month(.twoDigits).year().locale(Locale(identifier: "es_MX"))

/// Ledger for one account, or every account when `account` is nil.
struct EntriesView: View {
    let account: Account?
    @Environment(\.modelContext) private var context
    @Query private var entries: [LedgerEntry]
    @State private var selection = Set<PersistentIdentifier>()
    @State private var search = ""
    @State private var showingNewEntry = false
    @State private var editingEntry: LedgerEntry?

    init(account: Account?) {
        self.account = account
        let sort = [SortDescriptor(\LedgerEntry.date, order: .reverse)]
        if let id = account?.persistentModelID {
            _entries = Query(filter: #Predicate<LedgerEntry> { $0.account?.persistentModelID == id }, sort: sort)
        } else {
            _entries = Query(sort: sort)
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            FinanceHeading(
                title: account?.name ?? "Todas las cuentas",
                subtitle: "Tu historial de movimientos", icon: "creditcard")
            HStack(alignment: .firstTextBaseline) {
                VStack(alignment: .leading, spacing: 6) {
                    Text("Saldo de los movimientos").font(.subheadline).foregroundStyle(.secondary)
                    Text(Money.format(entries.reduce(0) { $0 + $1.amount }))
                        .font(.system(.largeTitle, design: .rounded, weight: .bold)).monospacedDigit()
                }
                Spacer()
                Text("\(entries.count) movimientos")
                    .font(.callout).foregroundStyle(.secondary)
            }
            .padding(20)
            .financeCard()
            ledger
        }
        .padding(24)
        .background(FinanceTheme.canvas)
        .tint(FinanceTheme.petroleum)
        .searchable(text: $search, prompt: "Buscar comercio, categoría o nota")
        .navigationTitle(account?.name ?? "Todas las cuentas")
        .navigationSubtitle(Money.format(entries.reduce(0) { $0 + $1.amount }))
        .toolbar {
            Button("Nuevo movimiento", systemImage: "plus") { showingNewEntry = true }
                .keyboardShortcut("n")
        }
        .sheet(isPresented: $showingNewEntry) { NewEntrySheet(defaultAccount: account) }
        .sheet(item: $editingEntry) { NewEntrySheet(defaultAccount: account, editing: $0) }
    }

    private var ledger: some View {
        Table(filtered, selection: $selection) {
            TableColumn("Fecha") { Text($0.date, format: dayFormat).monospacedDigit() }
                .width(min: 80, ideal: 90, max: 100)
            if account == nil {
                TableColumn("Cuenta") { Text($0.account?.name ?? "") }
            }
            TableColumn("Comercio") { Text($0.payee?.name ?? "") }
            TableColumn("Categoría") { Text($0.category?.name ?? "").foregroundStyle(.secondary) }
            TableColumn("Notas") { Text($0.notes).foregroundStyle(.secondary) }
            TableColumn("Monto") { entry in
                Text(Money.format(entry.amount)).monospacedDigit()
                    .foregroundStyle(entry.amount < 0 ? FinanceTheme.coral : FinanceTheme.emerald)
                    .frame(maxWidth: .infinity, alignment: .trailing)
            }
            .width(min: 90, ideal: 110)
        }
        .contextMenu(forSelectionType: PersistentIdentifier.self) { ids in
            if ids.count == 1 { Button("Editar") { edit(ids) } }
            Button("Eliminar", role: .destructive) { delete(ids) }
        } primaryAction: { ids in
            edit(ids)
        }
        .onDeleteCommand { delete(selection) }
        .clipShape(RoundedRectangle(cornerRadius: 14))
        .overlay {
            RoundedRectangle(cornerRadius: 14).strokeBorder(FinanceTheme.border, lineWidth: 1)
                .allowsHitTesting(false)
        }
        .overlay {
            if entries.isEmpty {
                ContentUnavailableView("Sin movimientos", systemImage: "tray",
                                       description: Text("Agrega uno con ⌘N o importa desde Actual en Archivo."))
            }
        }
    }

    private var filtered: [LedgerEntry] {
        let q = search.trimmingCharacters(in: .whitespaces)
        guard !q.isEmpty else { return entries }
        return entries.filter {
            [$0.payee?.name, $0.category?.name, $0.notes].contains { $0?.localizedStandardContains(q) == true }
        }
    }

    /// Double-click opens the whole entry in the editor; every column is editable there.
    private func edit(_ ids: Set<PersistentIdentifier>) {
        guard ids.count == 1, let id = ids.first else { return }
        editingEntry = context.model(for: id) as? LedgerEntry
    }

    private func delete(_ ids: Set<PersistentIdentifier>) {
        for id in ids { if let entry = context.model(for: id) as? LedgerEntry { context.delete(entry) } }
        selection.subtract(ids)
        context.saveOrAlert()
    }
}

struct NewEntrySheet: View {
    let defaultAccount: Account?
    var editing: LedgerEntry? = nil
    @Environment(\.modelContext) private var context
    @Environment(\.dismiss) private var dismiss
    @Query(sort: \Account.sortOrder) private var accounts: [Account]
    @Query(sort: \CategoryGroup.sortOrder) private var groups: [CategoryGroup]

    @State private var isExpense = true
    @State private var amountText = ""
    @State private var account: Account?
    @State private var payeeName = ""
    @State private var category: Category?
    @State private var date = Date.now
    @State private var notes = ""
    @FocusState private var amountFocused: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            FinanceHeading(
                title: editing == nil ? "Nuevo movimiento" : "Editar movimiento",
                subtitle: "Registra los detalles de tu ingreso o gasto", icon: "square.and.pencil"
            )
            .padding(24)
            Form {
                Section("Monto y tipo") {
            Picker("Tipo", selection: $isExpense) {
                Text("Gasto").tag(true)
                Text("Ingreso").tag(false)
            }
            .pickerStyle(.segmented)
                    TextField("Monto", text: $amountText, prompt: Text("0.00"))
                        .font(.title2.monospacedDigit())
                        .foregroundStyle(isExpense ? FinanceTheme.coral : FinanceTheme.emerald)
                        .focused($amountFocused)
                }
                Section("Cuenta y categoría") {
            Picker("Cuenta", selection: $account) {
                ForEach(accounts.filter { !$0.closed }) { Text($0.name).tag(Optional($0)) }
            }
            TextField("Comercio", text: $payeeName)
            Picker("Categoría", selection: $category) {
                Text("Sin categoría").tag(Category?.none)
                ForEach(groups) { group in
                    Section(group.name) {
                        ForEach((group.categories ?? []).filter { !$0.hidden }.sorted { $0.sortOrder < $1.sortOrder }) {
                            Text($0.name).tag(Optional($0))
                        }
                    }
                }
            }
                }
                Section("Fecha y notas") {
                    DatePicker("Fecha", selection: $date, displayedComponents: .date)
                    TextField("Notas", text: $notes)
                }
            }
            .formStyle(.grouped)
            .scrollContentBackground(.hidden)
        }
        .frame(minWidth: 480, idealWidth: 520, minHeight: 570)
        .background(FinanceTheme.canvas)
        .tint(FinanceTheme.petroleum)
        .toolbar {
            ToolbarItem(placement: .cancellationAction) { Button("Cancelar") { dismiss() } }
            ToolbarItem(placement: .confirmationAction) { Button("Guardar", action: save).disabled(!canSave) }
        }
        .navigationTitle(editing == nil ? "Nuevo movimiento" : "Editar movimiento")
        .onAppear {
            account = defaultAccount ?? accounts.first { !$0.closed }
            if let editing {
                isExpense = editing.amount <= 0
                amountText = Money.format(abs(editing.amount))
                account = editing.account
                payeeName = editing.payee?.name ?? ""
                category = editing.category
                date = editing.date
                notes = editing.notes
            }
            amountFocused = true
        }
    }

    private var canSave: Bool { Money.cents(from: amountText) != nil && account != nil }

    private func save() {
        guard let cents = Money.cents(from: amountText), let account else { return }
        let payee = try? Payee.findOrCreate(named: payeeName, in: context)
        if let editing {
            editing.date = Calendar.current.startOfDay(for: date)
            editing.amount = isExpense ? -cents : cents
            editing.account = account
            editing.payee = payee ?? nil
            editing.category = category
            editing.notes = notes
            context.saveOrAlert()
            dismiss()
            return
        }
        let entry = LedgerEntry(date: Calendar.current.startOfDay(for: date), amount: isExpense ? -cents : cents,
                                account: account, category: category, payee: payee ?? nil, notes: notes)
        context.insert(entry)
        entry.applyPayeeRule()
        context.saveOrAlert()
        dismiss()
    }
}
