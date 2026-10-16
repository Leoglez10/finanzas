import Foundation
import SwiftData

extension Schedule {
    /// The next unpaid occurrence, or nil when the schedule is finished.
    public var effectiveNextDate: Date? {
        guard !completed, let r = recurrence else { return nil }
        return nextDate ?? r.next(onOrAfter: r.start)
    }

    /// Skips the pending occurrence without recording a movement.
    public func skip() { advance() }

    /// Records the pending occurrence as a movement linked to this schedule, then advances.
    @MainActor @discardableResult
    public func post(in context: ModelContext) -> LedgerEntry {
        let entry = LedgerEntry(date: effectiveNextDate ?? Calendar.current.startOfDay(for: .now), amount: amount,
                                account: account, category: category, payee: payee, schedule: self)
        context.insert(entry)
        advance()
        return entry
    }

    private func advance() {
        guard let r = recurrence, let current = effectiveNextDate else { return }
        let dayAfter = Calendar.current.date(byAdding: .day, value: 1, to: current)!
        if let next = r.next(onOrAfter: dayAfter) { nextDate = next } else { completed = true }
    }
}

public enum Upcoming {
    public struct Item {
        public let schedule: Schedule
        public let date: Date
        public let overdue: Bool
        /// Account balance after this and every earlier listed payment of the same account.
        public let projectedBalance: Int?
    }

    /// Pending occurrences up to `today + days`, including overdue ones, oldest first.
    public static func items(schedules: [Schedule], today: Date = .now, days: Int = 30) -> [Item] {
        let cal = Calendar.current
        let today = cal.startOfDay(for: today)
        let horizon = cal.date(byAdding: .day, value: days, to: today)!
        let occurrences = schedules.flatMap { s -> [(Schedule, Date)] in
            guard let next = s.effectiveNextDate, let r = s.recurrence else { return [] }
            return r.occurrences(from: next, through: horizon).map { (s, $0) }
        }
        .sorted { ($0.1, $0.0.name) < ($1.1, $1.0.name) }

        var running: [PersistentIdentifier: Int] = [:]
        return occurrences.map { schedule, date in
            var projected: Int?
            if let account = schedule.account {
                let balance = (running[account.persistentModelID] ?? account.balance) + schedule.amount
                running[account.persistentModelID] = balance
                projected = balance
            }
            return Item(schedule: schedule, date: date, overdue: date < today, projectedBalance: projected)
        }
    }
}
