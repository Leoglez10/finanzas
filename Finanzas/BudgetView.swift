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

    private enum NameEdit {
        case newGroup, renameGroup(CategoryGroup), newCategory(CategoryGroup), renameCategory(Category)
    }

    var body: some View {
        let budget = (try? EnvelopeBudget.load(from: context))?.month(month)
        List {
            Section {
                LabeledContent("Disponible para asignar") {
                    Text(Money.format(budget?.toBudget ?? 0)).font(.title2.bold()).monospacedDigit()
                        .foregroundStyle((budget?.toBudget ?? 0) < 0 ? .red : .green)
                }
                LabeledContent("Ingresos del mes", value: Money.format(budget?.income ?? 0))
                LabeledContent("Asignado", value: Money.format(budget?.budgeted ?? 0))
            }
            ForEach(groups.filter { !$0.isIncome }) { group in
                Section {
                    ForEach(visibleCategories(group)) { category in
                        let row = budget?.rows[category.persistentModelID]
                        BudgetRow(name: category.name, row: row) { cents in
                            try? BudgetAmount.set(cents, for: category, month: month, in: context)
                            context.saveOrAlert()
                        }
                        .id("\(month)-\(category.persistentModelID.hashValue)")
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
                            Text(Money.format(budget?.rows[category.persistentModelID]?.activity ?? 0)).monospacedDigit()
                        }
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
        // ponytail: hide instead of delete, so past transactions keep their category; no unhide UI yet.
        Button("Ocultar") { category.hidden = true; context.saveOrAlert() }
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
        (group.categories ?? []).filter { !$0.hidden }.sorted { $0.sortOrder < $1.sortOrder }
    }
}

private struct AddButton: View {
    let help: String
    let action: () -> Void
    var body: some View {
        Button(action: action) { Image(systemName: "plus.circle.fill") }
            .buttonStyle(.borderless)
            .foregroundStyle(Color.accentColor)
            .help(help)
    }
}

private struct BudgetHeader: View {
    let title: String
    let onAdd: () -> Void
    var body: some View {
        HStack {
            Text(title)
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

    var body: some View {
        HStack {
            Text(name)
            Spacer()
            TextField("Asignado", text: $text)
                .multilineTextAlignment(.trailing)
                .monospacedDigit()
                .frame(width: 110)
                .textFieldStyle(.roundedBorder)
                .help("Escribe un monto, o +100 / -50 para sumar o restar")
                .onSubmit {
                    if let cents = Money.budgeted(from: text, current: row?.budgeted ?? 0) { onAssign(cents) }
                    text = Money.format(row?.budgeted ?? 0)
                }
            Text(Money.format(row?.activity ?? 0)).monospacedDigit().foregroundStyle(.secondary)
                .frame(width: 110, alignment: .trailing)
            Text(Money.format(row?.available ?? 0)).monospacedDigit()
                .padding(.horizontal, 6).padding(.vertical, 2)
                .background(availableColor.opacity(0.18), in: Capsule())
                .foregroundStyle(availableColor)
                .frame(width: 110, alignment: .trailing)
        }
        .onAppear { text = Money.format(row?.budgeted ?? 0) }
        .onChange(of: row?.budgeted) { text = Money.format(row?.budgeted ?? 0) }
    }

    private var availableColor: Color {
        switch row?.available ?? 0 {
        case ..<0: .red
        case 0: .secondary
        default: .green
        }
    }
}
