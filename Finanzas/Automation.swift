import FinanzasKit
import SwiftData
import SwiftUI
import UserNotifications

/// Keys for the automation settings (Ajustes).
enum AutomationSettings {
    static let remindersEnabled = "avisosActivos"
    static let daysBefore = "diasAviso"
    static let hour = 9
}

/// Posts due automatic payments and reschedules payment reminders. Runs at launch, hourly,
/// after MCP reloads and whenever schedules change.
@MainActor
enum AutomationRunner {
    static func run(_ context: ModelContext) async {
        let schedules = (try? context.fetch(FetchDescriptor<Schedule>())) ?? []
        if !Automations.postDue(schedules, in: context).isEmpty { context.saveOrAlert() }
        await reschedule(schedules)
    }

    static func reschedule(_ schedules: [Schedule]) async {
        let center = UNUserNotificationCenter.current()
        let pending = await center.pendingNotificationRequests().map(\.identifier).filter { $0.hasPrefix(Reminders.idPrefix) }
        center.removePendingNotificationRequests(withIdentifiers: pending)

        let defaults = UserDefaults.standard
        guard defaults.object(forKey: AutomationSettings.remindersEnabled) as? Bool ?? true,
              (try? await center.requestAuthorization(options: [.alert, .sound])) == true else { return }
        let days = defaults.object(forKey: AutomationSettings.daysBefore) as? Int ?? 2
        for r in Reminders.plan(Upcoming.items(schedules: schedules, days: 60), daysBefore: days, hour: AutomationSettings.hour) {
            let content = UNMutableNotificationContent()
            content.title = r.title
            content.body = r.body
            content.sound = .default
            let when = Calendar.current.dateComponents([.year, .month, .day, .hour, .minute], from: r.fireDate)
            try? await center.add(UNNotificationRequest(identifier: r.id, content: content,
                                                        trigger: UNCalendarNotificationTrigger(dateMatching: when, repeats: false)))
        }
    }
}

struct SettingsView: View {
    @AppStorage(AutomationSettings.remindersEnabled) private var remindersEnabled = true
    @AppStorage(AutomationSettings.daysBefore) private var daysBefore = 2
    @Environment(\.modelContext) private var context

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            FinanceHeading(title: "Ajustes", subtitle: "Tus recordatorios, a tu ritmo")
                .padding(24)
            Form {
                Section {
                    Label("Avisos de pagos", systemImage: "bell.badge")
                        .font(.headline).foregroundStyle(FinanceTheme.petroleum)
                Toggle("Avisarme antes de cada pago", isOn: $remindersEnabled)
                Stepper("Avisar \(daysBefore) \(daysBefore == 1 ? "día" : "días") antes, a las 9:00",
                        value: $daysBefore, in: 0...14)
                    .disabled(!remindersEnabled)
            }
            Section {
                    Label("Automatización", systemImage: "bolt.circle")
                        .font(.headline).foregroundStyle(FinanceTheme.petroleum)
                Text("Los pagos marcados como \"Registrar automáticamente\" se registran solos en su fecha al abrir la app y cada hora. Si un movimiento no tiene categoría, se usa la última categoría usada con ese comercio.")
                    .foregroundStyle(.secondary)
            }
            }
            .formStyle(.grouped)
            .scrollContentBackground(.hidden)
        }
        .frame(minWidth: 500, idealWidth: 540, minHeight: 400)
        .background(FinanceTheme.canvas)
        .tint(FinanceTheme.petroleum)
        .onChange(of: remindersEnabled) { Task { await AutomationRunner.run(context) } }
        .onChange(of: daysBefore) { Task { await AutomationRunner.run(context) } }
    }
}
