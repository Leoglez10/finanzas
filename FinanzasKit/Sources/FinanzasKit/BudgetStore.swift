import Foundation
import SwiftData

extension BudgetAmount {
    /// Upserts the amount assigned to `category` in `month`.
    @MainActor
    public static func set(_ amount: Int, for category: Category, month: Int, in context: ModelContext) throws {
        if let existing = (category.budgetAmounts ?? []).first(where: { $0.month == month }) {
            existing.amount = amount
        } else {
            context.insert(BudgetAmount(month: month, amount: amount, category: category))
        }
    }
}

extension EnvelopeBudget where ID == PersistentIdentifier {
    /// Builds the budget from the store. Only on-budget, categorized entries count, as in Actual.
    @MainActor
    public static func load(from context: ModelContext) throws -> EnvelopeBudget {
        let categories = try context.fetch(FetchDescriptor<Category>())
        var budgeted: [Key: Int] = [:]
        var carryover: Set<Key> = []
        for b in try context.fetch(FetchDescriptor<BudgetAmount>()) {
            guard let id = b.category?.persistentModelID else { continue }
            let key = Key(month: b.month, category: id)
            budgeted[key, default: 0] += b.amount
            if b.carryover { carryover.insert(key) }
        }
        var activity: [Key: Int] = [:]
        // ponytail: aggregates every entry on each call; fine for thousands of rows, cache per month if it grows.
        for e in try context.fetch(FetchDescriptor<LedgerEntry>()) {
            guard let category = e.category, let account = e.account, !account.offBudget else { continue }
            activity[Key(month: MonthKey.of(e.date), category: category.persistentModelID), default: 0] += e.amount
        }
        return EnvelopeBudget(
            categories: categories.map { .init(id: $0.persistentModelID, isIncome: $0.group?.isIncome ?? false) },
            budgeted: budgeted, carryover: carryover, activity: activity)
    }
}
