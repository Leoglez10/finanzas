import Foundation
import Testing
@testable import FinanzasKit

private func day(_ y: Int, _ m: Int, _ d: Int) -> Date {
    Calendar.current.date(from: DateComponents(year: y, month: m, day: d))!
}

@Test func monthlyOccurrencesWithinRange() {
    let r = Recurrence(start: day(2026, 4, 28), frequency: .monthly)
    #expect(r.occurrences(from: day(2026, 10, 1), through: day(2026, 12, 31))
        == [day(2026, 10, 28), day(2026, 11, 28), day(2026, 12, 28)])
}

@Test func monthEndClampsWithoutDrift() {
    let r = Recurrence(start: day(2026, 1, 31), frequency: .monthly)
    #expect(r.occurrences(from: day(2026, 1, 1), through: day(2026, 4, 30))
        == [day(2026, 1, 31), day(2026, 2, 28), day(2026, 3, 31), day(2026, 4, 30)])
}

@Test func intervalAndWeekly() {
    let r = Recurrence(start: day(2026, 10, 1), frequency: .weekly, interval: 2)
    #expect(r.occurrences(from: day(2026, 10, 1), through: day(2026, 10, 31))
        == [day(2026, 10, 1), day(2026, 10, 15), day(2026, 10, 29)])
}

@Test func weekendMovesAfterOrBefore() {
    // 2026-11-28 is a Saturday.
    let after = Recurrence(start: day(2026, 11, 28), frequency: .monthly, skipWeekend: true, weekendMode: .after)
    let before = Recurrence(start: day(2026, 11, 28), frequency: .monthly, skipWeekend: true, weekendMode: .before)
    #expect(after.next(onOrAfter: day(2026, 11, 1)) == day(2026, 11, 30))
    #expect(before.next(onOrAfter: day(2026, 11, 1)) == day(2026, 11, 27))
}

@Test func endDateStopsOccurrences() {
    let r = Recurrence(start: day(2026, 10, 5), frequency: .monthly, endDate: day(2026, 11, 5))
    #expect(r.occurrences(from: day(2026, 10, 1), through: day(2027, 1, 31)) == [day(2026, 10, 5), day(2026, 11, 5)])
    #expect(r.next(onOrAfter: day(2026, 11, 6)) == nil)
}

@Test func weekendShiftNeverRepeatsADate() {
    // 2026-10-17/18 are Sat/Sun: both shift onto Monday 19 (after) or Friday 16 (before).
    let after = Recurrence(start: day(2026, 10, 16), frequency: .daily, skipWeekend: true, weekendMode: .after)
    #expect(after.occurrences(from: day(2026, 10, 16), through: day(2026, 10, 20))
        == [day(2026, 10, 16), day(2026, 10, 19), day(2026, 10, 20)])
    let before = Recurrence(start: day(2026, 10, 15), frequency: .daily, skipWeekend: true, weekendMode: .before)
    #expect(before.occurrences(from: day(2026, 10, 15), through: day(2026, 10, 19))
        == [day(2026, 10, 15), day(2026, 10, 16), day(2026, 10, 19)])
}
