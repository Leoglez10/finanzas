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
                    }
                } header: {
                    BudgetHeader(title: group.name)
                }
            }
            ForEach(groups.filter(\.isIncome)) { group in
                Section(group.name) {
                    ForEach(visibleCategories(group)) { category in
                        LabeledContent(category.name) {
                            Text(Money.format(budget?.rows[category.persistentModelID]?.activity ?? 0)).monospacedDigit()
                        }
                    }
                }
            }
        }
        .navigationTitle(monthTitle)
        .toolbar {
            ToolbarItemGroup {
                Button("Mes anterior", systemImage: "chevron.left") { month = MonthKey.previous(month) }
                    .keyboardShortcut(.leftArrow, modifiers: .command)
                Button("Hoy") { month = MonthKey.of(.now) }
                Button("Mes siguiente", systemImage: "chevron.right") { month = MonthKey.next(month) }
                    .keyboardShortcut(.rightArrow, modifiers: .command)
            }
        }
    }

    /// "octubre de 2026" → "Octubre de 2026" (`capitalized` would also capitalize "De").
    private var monthTitle: String {
        let text = MonthKey.firstDay(month).formatted(.dateTime.month(.wide).year().locale(Locale(identifier: "es_MX")))
        return text.prefix(1).uppercased() + text.dropFirst()
    }

    private func visibleCategories(_ group: CategoryGroup) -> [Category] {
        (group.categories ?? []).filter { !$0.hidden }.sorted { $0.sortOrder < $1.sortOrder }
    }
}

private struct BudgetHeader: View {
    let title: String
    var body: some View {
        HStack {
            Text(title)
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
                .onSubmit {
                    if let cents = Money.cents(from: text) { onAssign(cents) }
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
