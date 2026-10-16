import Foundation
import SwiftData
import Testing
@testable import FinanzasKit

private func day(_ y: Int, _ m: Int, _ d: Int) -> Date {
    Calendar.current.date(from: DateComponents(year: y, month: m, day: d))!
}

@MainActor
private struct Fixture {
    let container: ModelContainer
    let context: ModelContext
    let bank = Account(name: "Banco"), cash = Account(name: "Cartera")
    let gym: Schedule, rent: Schedule, weekly: Schedule, done: Schedule

    init() throws {
        container = try FinanzasStore.inMemory()
        context = container.mainContext
        context.insert(bank); context.insert(cash)
        context.insert(LedgerEntry(date: day(2026, 10, 1), amount: 100_000, account: bank))
        context.insert(LedgerEntry(date: day(2026, 10, 1), amount: 5_000, account: cash))
        gym = Schedule(name: "Gym", amount: -55_000, recurrence: .init(start: day(2026, 5, 11), frequency: .monthly), account: bank)
        gym.nextDate = day(2026, 10, 11)  // overdue relative to "today" = Oct 15
        rent = Schedule(name: "Renta", amount: -30_000, recurrence: .init(start: day(2026, 1, 20), frequency: .monthly), account: bank)
        rent.nextDate = day(2026, 10, 20)
        weekly = Schedule(name: "Súper", amount: -1_000, recurrence: .init(start: day(2026, 10, 16), frequency: .weekly), account: cash)
        done = Schedule(name: "Viejo", amount: -1, recurrence: .init(start: day(2026, 1, 1), frequency: .monthly), account: bank)
        done.completed = true
        [gym, rent, weekly, done].forEach { context.insert($0) }
        try context.save()
    }
}

@MainActor
@Test func upcomingListsOccurrencesWithProjectedBalancePerAccount() throws {
    let f = try Fixture()
    let items = Upcoming.items(schedules: [f.gym, f.rent, f.weekly, f.done], today: day(2026, 10, 15), days: 14)

    #expect(items.map(\.schedule.name) == ["Gym", "Súper", "Renta", "Súper"])
    #expect(items.map(\.date) == [day(2026, 10, 11), day(2026, 10, 16), day(2026, 10, 20), day(2026, 10, 23)])
    #expect(items.map(\.overdue) == [true, false, false, false])
    // Banco: 1000.00 → -550 → -300 ; Cartera: 50.00 → -10 → -10
    #expect(items.map(\.projectedBalance) == [45_000, 4_000, 15_000, 3_000])
}

@MainActor
@Test func scheduleWithoutStoredNextDateStartsAtItsFirstOccurrence() throws {
    let f = try Fixture()
    #expect(f.weekly.nextDate == nil)
    #expect(f.weekly.effectiveNextDate == day(2026, 10, 16))
}

@MainActor
@Test func skipAdvancesToTheFollowingOccurrence() throws {
    let f = try Fixture()
    f.gym.skip()
    #expect(f.gym.nextDate == day(2026, 11, 11))
    #expect(try f.context.fetchCount(FetchDescriptor<LedgerEntry>()) == 2)
}

@MainActor
@Test func postingCreatesALinkedEntryAndAdvances() throws {
    let f = try Fixture()
    let payee = Payee(name: "Smart Fit"), category = Category(name: "Salud")
    f.context.insert(payee); f.context.insert(category)
    f.gym.payee = payee
    f.gym.category = category

    let entry = f.gym.post(in: f.context)
    #expect(entry.date == day(2026, 10, 11))
    #expect(entry.amount == -55_000)
    #expect(entry.account === f.bank && entry.payee === payee && entry.category === category)
    #expect(entry.schedule === f.gym)
    #expect(f.gym.nextDate == day(2026, 11, 11))
    #expect(f.bank.balance == 100_000 - 55_000)
}

@MainActor
@Test func passingTheLastOccurrenceCompletesTheSchedule() throws {
    let f = try Fixture()
    let once = Schedule(name: "Regalo", amount: -80_000,
                        recurrence: .init(start: day(2026, 12, 24), frequency: .monthly, endDate: day(2026, 12, 24)), account: f.bank)
    f.context.insert(once)
    once.skip()
    #expect(once.completed)
    #expect(Upcoming.items(schedules: [once], today: day(2026, 12, 1), days: 60).isEmpty)
}

@MainActor
@Test func dailyWeekendShiftedScheduleListsEachDateOnce() throws {
    let f = try Fixture()
    let daily = Schedule(name: "Diario", amount: -100,
                         recurrence: .init(start: day(2026, 10, 16), frequency: .daily, skipWeekend: true), account: f.cash)
    f.context.insert(daily)
    let items = Upcoming.items(schedules: [daily], today: day(2026, 10, 15), days: 5)
    #expect(items.map(\.date) == [day(2026, 10, 16), day(2026, 10, 19), day(2026, 10, 20)])
    #expect(items.map(\.projectedBalance) == [4_900, 4_800, 4_700])
}
