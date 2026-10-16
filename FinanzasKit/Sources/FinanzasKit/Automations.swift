import Foundation
import SwiftData

public enum Automations {
    /// Records every due occurrence (on or before `today`) of schedules marked automatic.
    @MainActor @discardableResult
    public static func postDue(_ schedules: [Schedule], today: Date = .now, in context: ModelContext) -> [LedgerEntry] {
        let today = Calendar.current.startOfDay(for: today)
        var posted: [LedgerEntry] = []
        for s in schedules where s.autoPost && !s.completed {
            var count = 0
            // Cap guards against a runaway loop (e.g. a daily rule left alone for years).
            while count < 500, let next = s.effectiveNextDate, next <= today {
                posted.append(s.post(in: context))
                count += 1
            }
        }
        return posted
    }
}

/// Local notification plan for upcoming payments (scheduling itself lives in the app).
public enum Reminders {
    public static let idPrefix = "pago-"

    public struct Reminder: Equatable, Sendable {
        public let id: String
        public let fireDate: Date
        public let title: String
        public let body: String
    }

    public static func plan(_ items: [Upcoming.Item], daysBefore: Int, hour: Int, now: Date = .now) -> [Reminder] {
        let cal = Calendar.current
        let dayText = Date.FormatStyle.dateTime.day(.twoDigits).month(.twoDigits).year().locale(Locale(identifier: "es_MX"))
        return items.compactMap { item in
            let base = cal.date(byAdding: .day, value: -daysBefore, to: item.date)!
            guard let fire = cal.date(bySettingHour: hour, minute: 0, second: 0, of: base), fire > now else { return nil }
            let s = item.schedule
            var body = "\(Money.format(s.amount)) el \(item.date.formatted(dayText))"
            if let account = s.account?.name { body += " (\(account))" }
            if s.autoPost { body += " · se registrará automáticamente" }
            let key = cal.dateComponents([.year, .month, .day], from: item.date)
            let id = "\(idPrefix)\(ModelID.string(s.persistentModelID))-" + String(format: "%04d%02d%02d", key.year!, key.month!, key.day!)
            return Reminder(id: id, fireDate: fire, title: "Pago próximo: \(s.name)", body: body)
        }
        .sorted { $0.fireDate < $1.fireDate }
    }
}

extension Payee {
    /// The category of this payee's most recent categorized entry (Actual's "learn categories").
    public var suggestedCategory: Category? {
        (entries ?? []).filter { $0.category != nil }.max { $0.date < $1.date }?.category
    }
}

extension LedgerEntry {
    /// Fills a missing category from the payee's history; an explicit category is kept.
    public func applyPayeeRule() {
        if category == nil { category = payee?.suggestedCategory }
    }
}
