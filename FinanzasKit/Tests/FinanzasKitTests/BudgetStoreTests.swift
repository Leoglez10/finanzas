import Foundation
import SwiftData
import Testing
@testable import FinanzasKit

private func day(_ y: Int, _ m: Int, _ d: Int) -> Date {
    Calendar.current.date(from: DateComponents(year: y, month: m, day: d))!
}

@MainActor
@Test func loadsBudgetFromStoreIgnoringOffBudgetAndUncategorized() throws {
    let container = try FinanzasStore.inMemory()
    let context = container.mainContext
    let bank = Account(name: "Banco"), invest = Account(name: "Inversión", offBudget: true)
    let expenses = CategoryGroup(name: "Gastos"), incomeGroup = CategoryGroup(name: "Ingresos", isIncome: true)
    let food = Category(name: "Comida", group: expenses), salary = Category(name: "Sueldo", group: incomeGroup)
    context.insert(bank); context.insert(invest); context.insert(expenses)
    context.insert(incomeGroup); context.insert(food); context.insert(salary)
    context.insert(LedgerEntry(date: day(2026, 9, 1), amount: 100_000, account: bank, category: salary))
    context.insert(LedgerEntry(date: day(2026, 9, 5), amount: -12_000, account: bank, category: food))
    context.insert(LedgerEntry(date: day(2026, 9, 6), amount: -99_999, account: invest, category: food))  // off-budget
    context.insert(LedgerEntry(date: day(2026, 9, 7), amount: -5_000, account: bank))                     // uncategorized
    try BudgetAmount.set(30_000, for: food, month: 202609, in: context)
    try context.save()

    let month = try EnvelopeBudget.load(from: context).month(202609)
    #expect(month.rows[food.persistentModelID] == .init(budgeted: 30_000, activity: -12_000, available: 18_000))
    #expect(month.income == 100_000)
    #expect(month.toBudget == 70_000)
}

@MainActor
@Test func settingABudgetUpdatesInsteadOfDuplicating() throws {
    let container = try FinanzasStore.inMemory()
    let context = container.mainContext
    let food = Category(name: "Comida")
    context.insert(food)
    try BudgetAmount.set(10_000, for: food, month: 202609, in: context)
    try BudgetAmount.set(25_000, for: food, month: 202609, in: context)
    try BudgetAmount.set(5_000, for: food, month: 202610, in: context)

    let amounts = try context.fetch(FetchDescriptor<BudgetAmount>(sortBy: [SortDescriptor(\.month)]))
    #expect(amounts.map(\.amount) == [25_000, 5_000])
}
