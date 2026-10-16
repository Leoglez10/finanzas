import FinanzasKit
import SwiftData
import SwiftUI

struct UpcomingView: View {
    @Environment(\.modelContext) private var context
    @Query(sort: \Schedule.name) private var schedules: [Schedule]
    @Query private var entries: [LedgerEntry]  // observed so projected balances refresh
    @State private var showingNewSchedule = false

    var body: some View {
        let items = Upcoming.items(schedules: schedules)
        let overdue = items.filter(\.overdue), coming = items.filter { !$0.overdue }
        List {
            if !overdue.isEmpty {
                Section("Vencidos") { ForEach(overdue, id: \.id) { row($0) } }
            }
            Section("Próximos 30 días") {
                if coming.isEmpty { Text("Nada programado").foregroundStyle(.secondary) }
                ForEach(coming, id: \.id) { row($0) }
            }
            Section("Pagos programados") {
                ForEach(schedules) { s in
                    LabeledContent {
                        Text(Money.format(s.amount)).monospacedDigit()
                    } label: {
                        Text(s.name)
                        Text(s.completed ? "Terminado" : describe(s)).foregroundStyle(.secondary)
                    }
                    .contextMenu { Button("Eliminar", role: .destructive) { context.delete(s); persist() } }
                }
            }
        }
        .navigationTitle("Próximos pagos")
        .toolbar {
            Button("Nuevo pago programado", systemImage: "plus") { showingNewSchedule = true }
                .keyboardShortcut("n")
        }
        .sheet(isPresented: $showingNewSchedule) { NewScheduleSheet() }
    }

    private func row(_ item: Upcoming.Item) -> some View {
        HStack {
            Text(item.date, format: dayFormat).monospacedDigit()
                .foregroundStyle(item.overdue ? .red : .secondary)
                .frame(width: 90, alignment: .leading)
            VStack(alignment: .leading) {
                Text(item.schedule.name)
                Text([item.schedule.payee?.name, item.schedule.account?.name].compactMap { $0 }.joined(separator: " · "))
                    .font(.caption).foregroundStyle(.secondary)
            }
            Spacer()
            VStack(alignment: .trailing) {
                Text(Money.format(item.schedule.amount)).monospacedDigit()
                if let balance = item.projectedBalance {
                    Text("Saldo: \(Money.format(balance))").font(.caption).monospacedDigit()
                        .foregroundStyle(balance < 0 ? .red : .secondary)
                }
            }
            // Only the pending occurrence can be acted on; later ones follow after it moves.
            HStack {
                if item.date == item.schedule.effectiveNextDate {
                    Button("Registrar") { item.schedule.post(in: context); persist() }
                    Button("Saltar") { item.schedule.skip(); persist() }
                }
            }
            .frame(width: 170, alignment: .trailing)
        }
    }

    // Saved right away so a posted payment survives a quit before autosave runs.
    private func persist() { try? context.save() }

    private func describe(_ s: Schedule) -> String {
        guard let r = s.recurrence else { return "" }
        let unit = switch r.frequency {
        case .daily: r.interval == 1 ? "Diario" : "Cada \(r.interval) días"
        case .weekly: r.interval == 1 ? "Semanal" : "Cada \(r.interval) semanas"
        case .monthly: r.interval == 1 ? "Mensual" : "Cada \(r.interval) meses"
        case .yearly: r.interval == 1 ? "Anual" : "Cada \(r.interval) años"
        }
        let next = s.effectiveNextDate.map { " · próximo \($0.formatted(dayFormat))" } ?? ""
        return unit + next + (s.autoPost ? " · automático" : "")
    }
}

private extension Upcoming.Item {
    var id: String { "\(schedule.persistentModelID.hashValue)-\(date.timeIntervalSince1970)" }
}

struct NewScheduleSheet: View {
    @Environment(\.modelContext) private var context
    @Environment(\.dismiss) private var dismiss
    @Query(sort: \Account.sortOrder) private var accounts: [Account]
    @Query(sort: \CategoryGroup.sortOrder) private var groups: [CategoryGroup]

    @State private var name = ""
    @State private var isExpense = true
    @State private var amountText = ""
    @State private var account: Account?
    @State private var payeeName = ""
    @State private var category: Category?
    @State private var frequency = Recurrence.Frequency.monthly
    @State private var interval = 1
    @State private var start = Date.now
    @State private var skipWeekend = false
    @State private var autoPost = false

    var body: some View {
        Form {
            TextField("Nombre", text: $name)
            Picker("Tipo", selection: $isExpense) {
                Text("Gasto").tag(true)
                Text("Ingreso").tag(false)
            }
            .pickerStyle(.segmented)
            TextField("Monto", text: $amountText, prompt: Text("0.00"))
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
            Picker("Se repite", selection: $frequency) {
                Text("Diario").tag(Recurrence.Frequency.daily)
                Text("Semanal").tag(Recurrence.Frequency.weekly)
                Text("Mensual").tag(Recurrence.Frequency.monthly)
                Text("Anual").tag(Recurrence.Frequency.yearly)
            }
            Stepper("Cada \(interval)", value: $interval, in: 1...52)
            DatePicker("Primera fecha", selection: $start, displayedComponents: .date)
            Toggle("Si cae en fin de semana, pasar al lunes", isOn: $skipWeekend)
            Toggle("Registrar automáticamente", isOn: $autoPost)
        }
        .formStyle(.grouped)
        .frame(width: 400)
        .toolbar {
            ToolbarItem(placement: .cancellationAction) { Button("Cancelar") { dismiss() } }
            ToolbarItem(placement: .confirmationAction) { Button("Guardar", action: save).disabled(!canSave) }
        }
        .onAppear { account = accounts.first { !$0.closed } }
    }

    private var canSave: Bool {
        !name.trimmingCharacters(in: .whitespaces).isEmpty && Money.cents(from: amountText) != nil && account != nil
    }

    private func save() {
        guard let cents = Money.cents(from: amountText) else { return }
        let rule = Recurrence(start: Calendar.current.startOfDay(for: start), frequency: frequency, interval: interval,
                              skipWeekend: skipWeekend, weekendMode: .after)
        let payee = try? Payee.findOrCreate(named: payeeName, in: context)
        context.insert(Schedule(name: name.trimmingCharacters(in: .whitespaces), amount: isExpense ? -cents : cents,
                                recurrence: rule, account: account, category: category, payee: payee ?? nil,
                                autoPost: autoPost))
        try? context.save()
        dismiss()
    }
}
