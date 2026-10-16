import Testing
@testable import FinanzasKit

@Test func monthSummaryTakesIncomeAndSpendingFromTheBudgetRows() {
    let budget = EnvelopeBudget<String>(
        categories: [.init(id: "salary", isIncome: true), .init(id: "food", isIncome: false), .init(id: "gym", isIncome: false)],
        budgeted: [.init(month: 202_610, category: "food"): 30_000],
        activity: [.init(month: 202_610, category: "salary"): 100_000,
                   .init(month: 202_610, category: "food"): -25_000,
                   .init(month: 202_610, category: "gym"): -15_000])
    let summary = MonthSummary(budget, month: 202_610)
    #expect(summary.income == 100_000)
    #expect(summary.expenses == 40_000)
    #expect(summary.savingsRate == 0.6)
    #expect(summary.spending["food"] == 25_000)
    #expect(summary.spending["salary"] == nil)
}

@Test func savingsRateIsNilWithoutIncome() {
    let budget = EnvelopeBudget<String>(categories: [.init(id: "food", isIncome: false)], budgeted: [:],
                                        activity: [.init(month: 202_610, category: "food"): -5_000])
    #expect(MonthSummary(budget, month: 202_610).savingsRate == nil)
}

@Test(arguments: [(110, 100, 0.1), (90, 100, -0.1), (5, 0, nil)] as [(Int, Int, Double?)])
func changeIsRelativeToThePreviousMonth(current: Int, previous: Int, change: Double?) {
    #expect(relativeChange(current, from: previous) == change)
}

@Test func aRefundedCategoryDoesNotMakeSpendingNegative() {
    let budget = EnvelopeBudget<String>(
        categories: [.init(id: "extras", isIncome: false), .init(id: "apps", isIncome: false)], budgeted: [:],
        activity: [.init(month: 202_609, category: "extras"): 9_800, .init(month: 202_609, category: "apps"): -4_900])
    #expect(MonthSummary(budget, month: 202_609).expenses == 4_900)
}

@MainActor
@Test func uncategorizedIncomeCountsOnlyThisMonthsOnBudgetDepositsWithoutCategory() throws {
    let container = try FinanzasStore.inMemory()
    let context = container.mainContext
    let checking = Account(name: "Nómina"), savings = Account(name: "Inversión", offBudget: true)
    let salary = Category(name: "Sueldo")
    [checking, savings].forEach(context.insert)
    context.insert(salary)
    let oct = MonthKey.firstDay(202_610), sep = MonthKey.firstDay(202_609)
    let entries = [
        LedgerEntry(date: oct, amount: 250_000, account: checking),                    // counted
        LedgerEntry(date: oct, amount: 10_000, account: checking, category: salary),   // categorized
        LedgerEntry(date: oct, amount: -5_000, account: checking),                     // expense
        LedgerEntry(date: oct, amount: 40_000, account: savings),                      // off budget
        LedgerEntry(date: sep, amount: 30_000, account: checking),                     // other month
    ]
    entries.forEach(context.insert)
    #expect(uncategorizedIncome(entries, month: 202_610) == 250_000)
}
