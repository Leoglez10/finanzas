import Charts
import FinanzasKit
import SwiftData
import SwiftUI

/// "octubre de 2026" → "Octubre de 2026" (`capitalized` would also capitalize "De").
func monthTitle(_ month: Int) -> String {
    let text = MonthKey.firstDay(month).formatted(.dateTime.month(.wide).year().locale(Locale(identifier: "es_MX")))
    return text.prefix(1).uppercased() + text.dropFirst()
}

private func shortMonth(_ month: Int) -> String {
    let text = MonthKey.firstDay(month).formatted(.dateTime.month(.abbreviated).locale(Locale(identifier: "es_MX")))
    return text.prefix(1).uppercased() + text.dropFirst().replacingOccurrences(of: ".", with: "")
}

struct OverviewView: View {
    @Environment(\.modelContext) private var context
    @Query(sort: \LedgerEntry.date, order: .reverse) private var entries: [LedgerEntry]
    @Query private var accounts: [Account]
    @Query private var categories: [Category]
    // Observed only so the view recomputes when assigned amounts change.
    @Query private var amounts: [BudgetAmount]
    @State private var month = MonthKey.of(.now)
    @State private var range = 6

    var body: some View {
        let budget = try? EnvelopeBudget.load(from: context)
        let current = budget.map { MonthSummary($0, month: month) }
        let previous = budget.map { MonthSummary($0, month: MonthKey.previous(month)) }

        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                cards(current, previous)
                HStack(alignment: .top, spacing: 16) {
                    if let budget { cashFlow(budget) }
                    spendingByCategory(current)
                        .frame(width: 320)
                }
                HStack(alignment: .top, spacing: 16) {
                    recentEntries
                    budgetProgress(current, rows: budget?.month(month).rows ?? [:]).frame(width: 320)
                }
            }
            .padding(20)
        }
        .navigationTitle("Resumen")
        .navigationSubtitle(monthTitle(month))
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

    // MARK: Cards

    private func cards(_ current: MonthSummary<PersistentIdentifier>?, _ previous: MonthSummary<PersistentIdentifier>?) -> some View {
        let income = current?.income ?? 0, expenses = current?.expenses ?? 0
        return LazyVGrid(columns: [GridItem(.adaptive(minimum: 200), spacing: 16)], spacing: 16) {
            StatCard(title: "Saldo total", icon: "building.columns",
                     value: Money.format(accounts.filter { !$0.closed }.reduce(0) { $0 + $1.balance }),
                     note: "\(accounts.filter { !$0.closed }.count) cuentas abiertas")
            StatCard(title: "Ingresos del mes", icon: "arrow.down.circle", value: Money.format(income),
                     change: relativeChange(income, from: previous?.income ?? 0), higherIsGood: true)
            StatCard(title: "Gastos del mes", icon: "arrow.up.circle", value: Money.format(expenses),
                     change: relativeChange(expenses, from: previous?.expenses ?? 0), higherIsGood: false)
            StatCard(title: "Tasa de ahorro", icon: "leaf",
                     value: current?.savingsRate.map { $0.formatted(.percent.precision(.fractionLength(1))) } ?? "—",
                     note: current?.savingsRate == nil ? "Sin ingresos este mes" : "\(Money.format(income - expenses)) ahorrados")
        }
    }

    // MARK: Cash flow

    private struct FlowPoint: Identifiable {
        let month: String, kind: String, amount: Double
        var id: String { month + kind }
    }

    private func cashFlow(_ budget: EnvelopeBudget<PersistentIdentifier>) -> some View {
        let months = (0..<range).reversed().map { offset in
            (0..<offset).reduce(month) { m, _ in MonthKey.previous(m) }
        }
        let summaries = months.map { ($0, MonthSummary(budget, month: $0)) }
        let points = summaries.flatMap { m, s in
            [FlowPoint(month: shortMonth(m), kind: "Ingresos", amount: Double(s.income) / 100),
             FlowPoint(month: shortMonth(m), kind: "Gastos", amount: Double(s.expenses) / 100)]
        }
        let averageNet = summaries.reduce(0) { $0 + $1.1.income - $1.1.expenses } / max(summaries.count, 1)

        return GroupBox {
            Chart(points) { point in
                BarMark(x: .value("Mes", point.month), y: .value("Monto", point.amount))
                    .foregroundStyle(by: .value("Tipo", point.kind))
                    .position(by: .value("Tipo", point.kind))
                    .cornerRadius(3)
            }
            .chartForegroundStyleScale(["Ingresos": Color.green, "Gastos": Color.red])
            .chartYAxis {
                AxisMarks { value in
                    AxisGridLine()
                    AxisValueLabel { if let v = value.as(Double.self) { Text(v, format: .currency(code: "MXN").precision(.fractionLength(0))) } }
                }
            }
            .frame(height: 240)
            Divider()
            HStack {
                Text("Flujo neto promedio: ").foregroundStyle(.secondary) + Text(Money.format(averageNet)).bold()
                Spacer()
            }
            .font(.callout)
        } label: {
            HStack {
                Text("Ingresos vs gastos").font(.headline)
                Spacer()
                Picker("Rango", selection: $range) {
                    Text("6M").tag(6)
                    Text("12M").tag(12)
                }
                .pickerStyle(.segmented)
                .labelsHidden()
                .fixedSize()
            }
        }
    }

    // MARK: Spending by category

    private func spendingByCategory(_ summary: MonthSummary<PersistentIdentifier>?) -> some View {
        let total = summary?.expenses ?? 0
        let top = (summary?.spending ?? [:]).sorted { $0.value > $1.value }.prefix(6)
        return GroupBox {
            VStack(alignment: .leading, spacing: 12) {
                Text("Total: \(Money.format(total))").foregroundStyle(.secondary)
                if top.isEmpty {
                    Text("Sin gastos este mes").foregroundStyle(.secondary).frame(maxWidth: .infinity, minHeight: 120)
                }
                ForEach(top, id: \.key) { id, cents in
                    let share = total > 0 ? Double(cents) / Double(total) : 0
                    VStack(alignment: .leading, spacing: 4) {
                        HStack {
                            Text(name(of: id))
                            Spacer()
                            Text(Money.format(cents)).monospacedDigit().bold()
                            Text(share, format: .percent.precision(.fractionLength(1)))
                                .monospacedDigit().foregroundStyle(.secondary).frame(width: 48, alignment: .trailing)
                        }
                        ProgressView(value: min(max(share, 0), 1))
                    }
                }
            }
            .padding(.top, 4)
        } label: {
            Text("Gasto por categoría").font(.headline)
        }
    }

    // MARK: Recent entries

    private var recentEntries: some View {
        GroupBox {
            Grid(alignment: .leading, horizontalSpacing: 16, verticalSpacing: 10) {
                GridRow {
                    Text("Descripción"); Text("Categoría"); Text("Cuenta"); Text("Fecha")
                    Text("Monto").gridColumnAlignment(.trailing)
                }
                .font(.caption.bold()).foregroundStyle(.secondary)
                Divider()
                ForEach(entries.prefix(8)) { entry in
                    GridRow {
                        Text(entry.payee?.name ?? (entry.notes.isEmpty ? "—" : entry.notes)).lineLimit(1)
                        Text(entry.category?.name ?? "Sin categoría").foregroundStyle(.secondary).lineLimit(1)
                        Text(entry.account?.name ?? "").foregroundStyle(.secondary).lineLimit(1)
                        Text(entry.date, format: dayFormat).monospacedDigit().foregroundStyle(.secondary)
                        Text(Money.format(entry.amount)).monospacedDigit()
                            .foregroundStyle(entry.amount < 0 ? Color.primary : Color.green)
                    }
                }
            }
            .padding(.top, 4)
        } label: {
            Text("Movimientos recientes").font(.headline)
        }
    }

    // MARK: Budget progress

    private func budgetProgress(_ summary: MonthSummary<PersistentIdentifier>?,
                                rows budgetRows: [PersistentIdentifier: EnvelopeBudget<PersistentIdentifier>.Row]) -> some View {
        let spent = summary?.expenses ?? 0, assigned = summary?.budgeted ?? 0
        let ratio = assigned > 0 ? Double(spent) / Double(assigned) : 0
        let top = budgetRows.filter { $0.value.budgeted > 0 }.sorted { $0.value.budgeted > $1.value.budgeted }.prefix(5)
        return GroupBox {
            VStack(alignment: .leading, spacing: 12) {
                HStack(alignment: .firstTextBaseline) {
                    Text(Money.format(spent)).font(.title.bold()).monospacedDigit()
                    Spacer()
                    Text("de \(Money.format(assigned))").foregroundStyle(.secondary)
                }
                ProgressView(value: min(ratio, 1)).tint(ratio > 1 ? .red : .accentColor)
                HStack {
                    Text(assigned > 0 ? ratio.formatted(.percent.precision(.fractionLength(0))) + " usado" : "Sin asignar")
                        .foregroundStyle(.secondary)
                    Spacer()
                    if assigned > 0 {
                        Text(spent > assigned ? "\(Money.format(spent - assigned)) excedido" : "\(Money.format(assigned - spent)) restante")
                            .foregroundStyle(spent > assigned ? .red : .green)
                    }
                }
                .font(.callout)
                if !top.isEmpty {
                    Divider()
                    ForEach(top, id: \.key) { id, row in
                        HStack {
                            Text(name(of: id))
                            Spacer()
                            Text("\(Money.format(-row.activity)) / \(Money.format(row.budgeted))")
                                .monospacedDigit().foregroundStyle(row.available < 0 ? .red : .secondary)
                        }
                    }
                }
            }
            .padding(.top, 4)
        } label: {
            Text("Presupuesto del mes").font(.headline)
        }
    }

    private func name(of id: PersistentIdentifier) -> String {
        categories.first { $0.persistentModelID == id }?.name ?? "—"
    }
}

private struct StatCard: View {
    let title: String
    let icon: String
    let value: String
    var change: Double? = nil
    var higherIsGood = true
    var note: String? = nil

    var body: some View {
        GroupBox {
            VStack(alignment: .leading, spacing: 8) {
                Text(value).font(.title.bold()).monospacedDigit().lineLimit(1).minimumScaleFactor(0.6)
                if let change {
                    let good = (change >= 0) == higherIsGood
                    Label(change.formatted(.percent.precision(.fractionLength(1)).sign(strategy: .always())) + " vs mes anterior",
                          systemImage: change >= 0 ? "arrow.up.right" : "arrow.down.right")
                        .font(.callout)
                        .foregroundStyle(good ? .green : .red)
                } else {
                    Text(note ?? "Sin datos del mes anterior").font(.callout).foregroundStyle(.secondary)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.top, 4)
        } label: {
            Label(title, systemImage: icon).foregroundStyle(.secondary)
        }
    }
}
