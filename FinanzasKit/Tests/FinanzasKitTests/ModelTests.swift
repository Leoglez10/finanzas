import Foundation
import SwiftData
import Testing
@testable import FinanzasKit

@MainActor
@Test func accountBalanceSumsItsEntries() throws {
    let container = try FinanzasStore.inMemory()
    let context = container.mainContext
    let checking = Account(name: "Checking")
    let savings = Account(name: "Savings", offBudget: true)
    context.insert(checking)
    context.insert(savings)
    context.insert(LedgerEntry(date: .now, amount: 100_050, account: checking))
    context.insert(LedgerEntry(date: .now, amount: -2_550, account: checking))
    context.insert(LedgerEntry(date: .now, amount: 900, account: savings))
    try context.save()

    let accounts = try context.fetch(FetchDescriptor<Account>(sortBy: [SortDescriptor(\.name)]))
    #expect(accounts.map(\.balance) == [97_500, 900])
}

@MainActor
@Test func scheduleRoundTripsItsRecurrence() throws {
    let container = try FinanzasStore.inMemory()
    let context = container.mainContext
    let rule = Recurrence(start: .now, frequency: .monthly, skipWeekend: true)
    context.insert(Schedule(name: "Gym", amount: -2_500, recurrence: rule))
    try context.save()

    let schedule = try #require(try context.fetch(FetchDescriptor<Schedule>()).first)
    #expect(schedule.recurrence == rule)
}
