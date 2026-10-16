import AppKit
import FinanzasKit
import SwiftData
import SwiftUI

/// Isolated presentation probe: never initializes FinanzasApp or its on-disk store.
@main
struct DesignCheck {
    @MainActor static func main() throws {
        let app = NSApplication.shared
        app.setActivationPolicy(.prohibited)
        let output = URL(filePath: CommandLine.arguments[1])
        let container = try FinanzasStore.inMemory()
        let context = container.mainContext
        let account = Account(name: "Cuenta de prueba con nombre largo")
        let group = CategoryGroup(name: "Gastos esenciales")
        let category = FinanzasKit.Category(name: "Servicios y suministros del hogar", group: group)
        let payee = Payee(name: "Comercio de prueba")
        context.insert(account)
        context.insert(group)
        context.insert(category)
        context.insert(payee)
        context.insert(LedgerEntry(date: .now, amount: 250000, account: account))
        context.insert(LedgerEntry(date: .now, amount: -45000, account: account,
                                   category: category, payee: payee, notes: "Movimiento sintético"))
        context.insert(BudgetAmount(month: MonthKey.of(.now), amount: 50000, category: category))
        let overdue = Calendar.current.date(byAdding: .day, value: -3, to: .now)!
        context.insert(Schedule(name: "Pago de prueba con una descripción extensa", amount: -12000,
                                recurrence: Recurrence(start: overdue, frequency: .monthly),
                                account: account, category: category, payee: payee, autoPost: false))
        try context.save()
        let entryCount = try context.fetchCount(FetchDescriptor<LedgerEntry>())
        let scheduleCount = try context.fetchCount(FetchDescriptor<Schedule>())
        let defaults = UserDefaults(suiteName: "Finanzas.DesignCheck.\(UUID().uuidString)")!

        for scheme in [ColorScheme.light, .dark] {
            for width in [CGFloat(760), CGFloat(1160)] {
                try render(BudgetView(), name: "budget", width: width, height: 800, scheme: scheme,
                           container: container, defaults: defaults, output: output)
                try render(EntriesView(account: nil), name: "ledger", width: width, height: 800, scheme: scheme,
                           container: container, defaults: defaults, output: output)
                try render(UpcomingView(), name: "upcoming", width: width, height: 800, scheme: scheme,
                           container: container, defaults: defaults, output: output)
            }
            try render(OverviewView(), name: "overview", width: 1160, height: 900, scheme: scheme,
                       container: container, defaults: defaults, output: output)
            try render(NewEntrySheet(defaultAccount: account), name: "entry", width: 520, height: 680,
                       scheme: scheme, container: container, defaults: defaults, output: output)
            try render(NewScheduleSheet(), name: "schedule", width: 540, height: 850,
                       scheme: scheme, container: container, defaults: defaults, output: output)
            try render(SettingsView(), name: "settings", width: 540, height: 520, scheme: scheme,
                       container: container, defaults: defaults, output: output)
        }
        guard try context.fetchCount(FetchDescriptor<LedgerEntry>()) == entryCount,
              try context.fetchCount(FetchDescriptor<Schedule>()) == scheduleCount else {
            fatalError("Rendering changed the in-memory records")
        }
        print("PASS: 20 view renders, light/dark and compact/wide; in-memory record counts unchanged.")
    }

    @MainActor private static func render<V: View>(
        _ view: V, name: String, width: CGFloat, height: CGFloat, scheme: ColorScheme,
        container: ModelContainer, defaults: UserDefaults, output: URL
    ) throws {
        let content = view.modelContainer(container).defaultAppStorage(defaults).environment(\.colorScheme, scheme)
        let host = NSHostingView(rootView: content)
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: width, height: height),
                              styleMask: [.borderless], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.appearance = NSAppearance(named: scheme == .dark ? .darkAqua : .aqua)
        window.contentView = host
        host.frame = NSRect(x: 0, y: 0, width: width, height: height)
        host.layoutSubtreeIfNeeded()
        // A single bounded run-loop turn lets SwiftData queries and native controls lay out.
        RunLoop.current.run(until: Date(timeIntervalSinceNow: 0.1))
        host.layoutSubtreeIfNeeded()
        guard let bitmap = host.bitmapImageRepForCachingDisplay(in: host.bounds) else {
            fatalError("No bitmap for \(name)")
        }
        host.cacheDisplay(in: host.bounds, to: bitmap)
        guard let png = bitmap.representation(using: .png, properties: [:]), !png.isEmpty else {
            fatalError("Empty render for \(name)")
        }
        let file = "\(name)-\(scheme == .dark ? "dark" : "light")-\(Int(width)).png"
        try png.write(to: output.appending(path: file))
        print("Rendered \(file)")
        window.close()
    }
}
