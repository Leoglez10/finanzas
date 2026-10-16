import Foundation

/// Repeating date rule for scheduled payments. Stored as JSON on `Schedule`.
public struct Recurrence: Codable, Hashable, Sendable {
    public enum Frequency: String, Codable, CaseIterable, Sendable { case daily, weekly, monthly, yearly }
    public enum WeekendMode: String, Codable, Sendable { case before, after }

    public var start: Date
    public var frequency: Frequency
    public var interval: Int
    public var skipWeekend: Bool
    public var weekendMode: WeekendMode
    public var endDate: Date?

    public init(start: Date, frequency: Frequency, interval: Int = 1,
                skipWeekend: Bool = false, weekendMode: WeekendMode = .after, endDate: Date? = nil) {
        self.start = start
        self.frequency = frequency
        self.interval = max(1, interval)
        self.skipWeekend = skipWeekend
        self.weekendMode = weekendMode
        self.endDate = endDate
    }

    /// Adjusted occurrence dates within `from...through` (inclusive, day precision).
    public func occurrences(from: Date, through: Date) -> [Date] {
        let cal = Calendar.current
        let from = cal.startOfDay(for: from), through = cal.startOfDay(for: through)
        // A weekend shift moves a date by at most 2 days, so scan slightly past `through`.
        let scanLimit = cal.date(byAdding: .day, value: 3, to: through)!
        var result: [Date] = []
        var n = 0
        while let raw = rawOccurrence(n), raw <= scanLimit {
            if let endDate, raw > cal.startOfDay(for: endDate) { break }
            let date = adjusted(raw)
            // Weekend shifts can land several raw dates on one weekday; shifted dates stay ordered,
            // so comparing with the last kept date removes the repeats.
            if date >= from && date <= through && result.last != date { result.append(date) }
            n += 1
        }
        return result
    }

    public func next(onOrAfter date: Date) -> Date? {
        // ponytail: linear scan over a 2-year window; plenty for personal schedules.
        let limit = Calendar.current.date(byAdding: .year, value: 2, to: date)!
        return occurrences(from: date, through: limit).first
    }

    // Computed from `start` each time (not cumulatively) so month-end dates don't drift.
    private func rawOccurrence(_ n: Int) -> Date? {
        let cal = Calendar.current
        let base = cal.startOfDay(for: start)
        let steps = n * interval
        switch frequency {
        case .daily: return cal.date(byAdding: .day, value: steps, to: base)
        case .weekly: return cal.date(byAdding: .day, value: steps * 7, to: base)
        case .monthly: return cal.date(byAdding: .month, value: steps, to: base)
        case .yearly: return cal.date(byAdding: .year, value: steps, to: base)
        }
    }

    private func adjusted(_ date: Date) -> Date {
        guard skipWeekend else { return date }
        let cal = Calendar.current
        var d = date
        let step = weekendMode == .after ? 1 : -1
        while cal.isDateInWeekend(d) { d = cal.date(byAdding: .day, value: step, to: d)! }
        return d
    }
}
