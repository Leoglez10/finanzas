import Foundation
import Testing
@testable import FinanzasKit

private typealias Budget = EnvelopeBudget<String>
private let cats = [Budget.CategoryInfo(id: "salary", isIncome: true),
                    Budget.CategoryInfo(id: "food", isIncome: false),
                    Budget.CategoryInfo(id: "fun", isIncome: false)]
private func k(_ month: Int, _ cat: String) -> Budget.Key { .init(month: month, category: cat) }

@Test func monthKeysStepAcrossYears() {
    #expect(MonthKey.next(202612) == 202701)
    #expect(MonthKey.previous(202701) == 202612)
    #expect(MonthKey.of(Calendar.current.date(from: DateComponents(year: 2026, month: 9, day: 30))!) == 202609)
}

@Test func assignedMinusSpendingIsAvailable() {
    let b = Budget(categories: cats,
                   budgeted: [k(202609, "food"): 30_000],
                   activity: [k(202609, "salary"): 100_000, k(202609, "food"): -10_000])
    let m = b.month(202609)
    #expect(m.rows["food"] == .init(budgeted: 30_000, activity: -10_000, available: 20_000))
    #expect(m.rows["salary"] == .init(budgeted: 0, activity: 100_000, available: 0))
    #expect(m.income == 100_000)
    #expect(m.toBudget == 70_000)
}

@Test func leftoverAndUnassignedMoneyCarryForward() {
    let b = Budget(categories: cats,
                   budgeted: [k(202609, "food"): 30_000],
                   activity: [k(202609, "salary"): 100_000, k(202609, "food"): -10_000])
    let next = b.month(202610)
    #expect(next.rows["food"]?.available == 20_000)
    #expect(next.toBudget == 70_000)
}

@Test func overspendingResetsCategoryAndReducesNextMonthToBudget() {
    let b = Budget(categories: cats,
                   budgeted: [k(202609, "fun"): 10_000],
                   activity: [k(202609, "salary"): 50_000, k(202609, "fun"): -15_000])
    #expect(b.month(202609).rows["fun"]?.available == -5_000)
    let next = b.month(202610)
    #expect(next.rows["fun"]?.available == 0)
    #expect(next.toBudget == 40_000 - 5_000)
}

@Test func carryoverKeepsOverspendingInTheCategory() {
    let b = Budget(categories: cats,
                   budgeted: [k(202609, "fun"): 10_000],
                   carryover: [k(202609, "fun")],
                   activity: [k(202609, "salary"): 50_000, k(202609, "fun"): -15_000])
    let next = b.month(202610)
    #expect(next.rows["fun"]?.available == -5_000)
    #expect(next.toBudget == 40_000)
}

@Test func overAssigningMakesToBudgetNegativeAndItCarries() {
    let b = Budget(categories: cats,
                   budgeted: [k(202609, "food"): 80_000, k(202610, "food"): 10_000],
                   activity: [k(202609, "salary"): 50_000])
    #expect(b.month(202609).toBudget == -30_000)
    #expect(b.month(202610).toBudget == -40_000)
    #expect(b.month(202610).rows["food"]?.available == 90_000)
}

@Test func emptyMonthBeforeAnyDataIsZero() {
    let b = Budget(categories: cats, budgeted: [:], activity: [k(202609, "salary"): 1_000])
    #expect(b.month(202601).toBudget == 0)
    #expect(b.month(202601).rows["food"]?.available == 0)
}

@Test(arguments: [
    (50_000, -45_000, 5_000, 0.9),    // 450 of 500 spent
    (50_000, -60_000, -10_000, 1.0),  // overspent caps at full
    (0, 0, 0, 0.0),                   // nothing assigned or spent
    (0, 0, 20_000, 0.0),              // carried-over funds, nothing spent
    (30_000, 10_000, 40_000, 0.0),    // a refund is not spending
] as [(Int, Int, Int, Double)])
func spentShareIsSpendingOverAvailableFunds(budgeted: Int, activity: Int, available: Int, share: Double) {
    let row = EnvelopeBudget<String>.Row(budgeted: budgeted, activity: activity, available: available)
    #expect(row.spentShare == share)
}
