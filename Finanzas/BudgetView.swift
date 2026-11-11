import FinanzasKit
import SwiftData
import SwiftUI

struct BudgetView: View {
    @Environment(\.modelContext) private var context
    @Query(sort: \CategoryGroup.sortOrder) private var groups: [CategoryGroup]
    // Observed only so the view recomputes when entries or amounts change.
    @Query private var entries: [LedgerEntry]
    @Query private var amounts: [BudgetAmount]
    @State private var month = MonthKey.of(.now)
    @State private var nameEdit: NameEdit?
    @State private var nameText = ""
    @State private var showHidden = false

    private enum NameEdit {
        case newGroup, renameGroup(CategoryGroup), newCategory(CategoryGroup), renameCategory(Category)
    }

    var body: some View {
        let budget = (try? EnvelopeBudget.load(from: context))?.month(month)
        VStack(spacing: 0) {
            VStack(alignment: .leading, spacing: 20) {
                FinanceHeading(title: "Dale un destino a tu dinero", subtitle: monthTitle(month))
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 190), spacing: 16)], spacing: 16) {
                    FinanceMetric(
                        title: "Disponible para asignar", value: Money.format(budget?.toBudget ?? 0),
                        icon: "tray.and.arrow.down",
                        color: (budget?.toBudget ?? 0) < 0 ? FinanceTheme.coral : FinanceTheme.emerald,
                        note: toBudgetHint(budget?.toBudget ?? 0))
                    FinanceMetric(
                        title: "Ingresos del mes", value: Money.format(budget?.income ?? 0),
                        icon: "arrow.down.circle", color: FinanceTheme.emerald)
                    FinanceMetric(title: "Asignado", value: Money.format(budget?.budgeted ?? 0), icon: "envelope.badge")
                }
                let uncategorized = uncategorizedIncome(entries, month: month)
                if uncategorized > 0 {
                    Label("Tienes \(Money.format(uncategorized)) en ingresos sin categoría. Dales una categoría de ingreso en Movimientos para poder repartirlos.",
                          systemImage: "exclamationmark.triangle.fill")
                        .foregroundStyle(FinanceTheme.amber)
                        .padding(12)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(FinanceTheme.amber.opacity(0.1), in: RoundedRectangle(cornerRadius: 12))
                }
                Label("Escribe cuánto asignas a cada categoría, o usa +100 / -50 para sumar o restar.",
                      systemImage: "keyboard")
                    .font(.callout).foregroundStyle(.secondary)
            }
            .padding(24)
            List {
            ForEach(groups.filter { !$0.isIncome }) { group in
                Section {
                    ForEach(visibleCategories(group)) { category in
                        let row = budget?.rows[category.persistentModelID]
                        BudgetRow(name: category.name, row: row) { cents in
                            try? BudgetAmount.set(cents, for: category, month: month, in: context)
                            context.saveOrAlert()
                        }
                        .id("\(month)-\(category.persistentModelID.hashValue)")
                        .opacity(category.hidden ? 0.5 : 1)
                        .contextMenu { categoryMenu(category) }
                    }
                } header: {
                    BudgetHeader(title: group.name) { edit(.newCategory(group)) }
                        .contextMenu { Button("Renombrar grupo") { edit(.renameGroup(group), name: group.name) } }
                }
            }
            ForEach(groups.filter(\.isIncome)) { group in
                Section {
                    ForEach(visibleCategories(group)) { category in
                        LabeledContent(category.name) {
                                Text(Money.format(budget?.rows[category.persistentModelID]?.activity ?? 0))
                                    .monospacedDigit().foregroundStyle(FinanceTheme.emerald)
                        }
                        .opacity(category.hidden ? 0.5 : 1)
                        .contextMenu { categoryMenu(category) }
                    }
                } header: {
                    HStack {
                        Text(group.name)
                        AddButton(help: "Nueva categoría") { edit(.newCategory(group)) }
                    }
                    .contextMenu { Button("Renombrar grupo") { edit(.renameGroup(group), name: group.name) } }
                }
            }
            }
            .listStyle(.inset)
            .scrollContentBackground(.hidden)
        }
        .background(FinanceTheme.canvas)
        .tint(FinanceTheme.petroleum)
        .navigationTitle(monthTitle(month))
        .alert(nameEditTitle, isPresented: Binding(get: { nameEdit != nil }, set: { if !$0 { nameEdit = nil } })) {
            TextField("Nombre", text: $nameText)
            Button("Guardar", action: saveName)
                .disabled(nameText.trimmingCharacters(in: .whitespaces).isEmpty)
            Button("Cancelar", role: .cancel) {}
        }
        .toolbar {
            ToolbarItem {
                Button("Nuevo grupo", systemImage: "folder.badge.plus") { edit(.newGroup) }
            }
            ToolbarItem {
                Toggle("Mostrar ocultas", systemImage: "eye", isOn: $showHidden)
                    .help("Muestra las categorías ocultas; clic derecho ▸ Mostrar para recuperarlas")
            }
            ToolbarItemGroup {
                Button("Mes anterior", systemImage: "chevron.left") { month = MonthKey.previous(month) }
                    .keyboardShortcut(.leftArrow, modifiers: .command)
                Button("Hoy") { month = MonthKey.of(.now) }
                Button("Mes siguiente", systemImage: "chevron.right") { month = MonthKey.next(month) }
                    .keyboardShortcut(.rightArrow, modifiers: .command)
            }
        }
    }

    @ViewBuilder private func categoryMenu(_ category: Category) -> some View {
        Button("Renombrar") { edit(.renameCategory(category), name: category.name) }
        // Hide instead of delete, so past transactions keep their category.
        Button(category.hidden ? "Mostrar" : "Ocultar") { category.hidden.toggle(); context.saveOrAlert() }
    }

    private func edit(_ target: NameEdit, name: String = "") {
        nameText = name
        nameEdit = target
    }

    private var nameEditTitle: String {
        switch nameEdit {
        case .newGroup: "Nuevo grupo"
        case .renameGroup: "Renombrar grupo"
        case .newCategory(let group): "Nueva categoría en \(group.name)"
        case .renameCategory: "Renombrar categoría"
        case nil: ""
        }
    }

    private func saveName() {
        let name = nameText.trimmingCharacters(in: .whitespaces)
        guard !name.isEmpty, let target = nameEdit else { return }
        switch target {
        case .newGroup:
            let expenseGroups = groups.filter { !$0.isIncome }
            context.insert(CategoryGroup(name: name, sortOrder: (expenseGroups.map(\.sortOrder).max() ?? 0) + 1))
        case .renameGroup(let group):
            group.name = name
        case .newCategory(let group):
            let last = (group.categories ?? []).map(\.sortOrder).max() ?? 0
            context.insert(Category(name: name, group: group, sortOrder: last + 1))
        case .renameCategory(let category):
            category.name = name
        }
        context.saveOrAlert()
    }

    private func visibleCategories(_ group: CategoryGroup) -> [Category] {
        (group.categories ?? []).filter { showHidden || !$0.hidden }.sorted { $0.sortOrder < $1.sortOrder }
    }
}

private struct AddButton: View {
    let help: String
    let action: () -> Void
    var body: some View {
        Button(action: action) { Image(systemName: "plus.circle.fill") }
            .buttonStyle(.borderless)
            .foregroundStyle(FinanceTheme.petroleum)
            .accessibilityLabel(help)
            .help(help)
    }
}

private struct BudgetHeader: View {
    let title: String
    let onAdd: () -> Void
    var body: some View {
        HStack {
            Text(title).font(.headline).foregroundStyle(FinanceTheme.petroleum)
            AddButton(help: "Nueva categoría", action: onAdd)
            Spacer()
            Group {
                Text("Asignado").frame(width: 110, alignment: .trailing)
                Text("Gastado").frame(width: 110, alignment: .trailing)
                Text("Disponible").frame(width: 110, alignment: .trailing)
            }
        }
    }
}

private struct BudgetRow: View {
    let name: String
    let row: EnvelopeBudget<PersistentIdentifier>.Row?
    let onAssign: (Int) -> Void
    @State private var text = ""
    @FocusState private var focused: Bool

    var body: some View {
        HStack(spacing: 16) {
            VStack(alignment: .leading, spacing: 6) {
                Text(name)
                ProgressView(value: row?.spentShare ?? 0)
                    .tint((row?.available ?? 0) < 0 ? FinanceTheme.coral : FinanceTheme.petroleum)
                    .frame(maxWidth: 280)
                    .accessibilityLabel("Gastado \(Int((row?.spentShare ?? 0) * 100))%")
            }
            Spacer()
            TextField("Asignado", text: $text)
                .multilineTextAlignment(.trailing)
                .monospacedDigit()
                .frame(width: 110)
                .textFieldStyle(.roundedBorder)
                .help("Escribe un monto, o +100 / -50 para sumar o restar")
                .focused($focused)
                .onSubmit(commit)
                // Tab or a click elsewhere commits too, so a typed amount is never silently dropped.
                .onChange(of: focused) { if !focused { commit() } }
            Text(Money.format(row?.activity ?? 0)).monospacedDigit().foregroundStyle(.secondary)
                .frame(width: 110, alignment: .trailing)
            Text(Money.format(row?.available ?? 0)).monospacedDigit()
                .padding(.horizontal, 6).padding(.vertical, 2)
                .background(availableColor.opacity(0.18), in: Capsule())
                .foregroundStyle(availableColor)
                .frame(width: 110, alignment: .trailing)
        }
        .padding(.vertical, 8)
        .onAppear { text = Money.format(row?.budgeted ?? 0) }
        .onChange(of: row?.budgeted) { text = Money.format(row?.budgeted ?? 0) }
    }

    private func commit() {
        let current = row?.budgeted ?? 0
        guard text != Money.format(current) else { return }
        if let cents = Money.budgeted(from: text, current: current) { onAssign(cents) }
        text = Money.format(current)
    }

    private var availableColor: Color {
        switch row?.available ?? 0 {
        case ..<0: FinanceTheme.coral
        case 0: .secondary
        default: FinanceTheme.emerald
        }
    }
}
