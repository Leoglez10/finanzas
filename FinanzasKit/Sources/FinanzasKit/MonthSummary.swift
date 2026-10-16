import Foundation

/// Income, spending and savings for one month, read from the envelope budget.
public struct MonthSummary<ID: Hashable & Sendable>: Sendable {
    public var income: Int
    /// Positive cents spent across expense categories; a refund lowers its own category but never below zero.
    public var expenses: Int
    public var budgeted: Int
    /// Positive cents spent per expense category; categories with no net spending are left out.
    public var spending: [ID: Int]

    public init(_ budget: EnvelopeBudget<ID>, month: Int) {
        let m = budget.month(month)
        let incomeIDs = Set(budget.categories.filter(\.isIncome).map(\.id))
        income = m.income
        budgeted = m.budgeted
        spending = m.rows.filter { !incomeIDs.contains($0.key) && $0.value.activity < 0 }.mapValues { -$0.activity }
        expenses = spending.values.reduce(0, +)
    }

    /// Share of income kept this month; nil when there was no income.
    public var savingsRate: Double? { income > 0 ? Double(income - expenses) / Double(income) : nil }
}

/// Relative change from the previous value; nil when there is nothing to compare with.
public func relativeChange(_ current: Int, from previous: Int) -> Double? {
    previous == 0 ? nil : Double(current - previous) / Double(abs(previous))
}
