import Foundation
import SwiftData
import Testing
@testable import FinanzasKit

@Test(arguments: [
    ("125.50", 12_550), ("$1,250.5", 125_050), (" 80 ", 8_000), ("0.07", 7), ("-40", 4_000), ("12.345", 1_235),
])
func parsesTypedAmountsToPositiveCents(input: String, cents: Int) {
    #expect(Money.cents(from: input) == cents)
}

@Test(arguments: ["", "abc", "1.2.3"])
func rejectsInvalidAmounts(input: String) {
    #expect(Money.cents(from: input) == nil)
}

@MainActor
@Test func payeeLookupIsCaseInsensitiveAndCreatesWhenMissing() throws {
    let container = try FinanzasStore.inMemory()
    let context = container.mainContext
    let oxxo = Payee(name: "Oxxo")
    context.insert(oxxo)

    #expect(try Payee.findOrCreate(named: "  OXXO ", in: context) === oxxo)
    let created = try #require(try Payee.findOrCreate(named: "Uber", in: context))
    #expect(created.name == "Uber")
    #expect(try Payee.findOrCreate(named: "   ", in: context) == nil)
    #expect(try context.fetchCount(FetchDescriptor<Payee>()) == 2)
}

@MainActor
@Test func payeeNamedFindsExistingWithoutCreating() throws {
    let container = try FinanzasStore.inMemory()
    let context = container.mainContext
    let oxxo = Payee(name: "Oxxo")
    context.insert(oxxo)

    #expect(try Payee.named(" oxxo ", in: context) === oxxo)
    #expect(try Payee.named("Uber", in: context) == nil)
    #expect(try Payee.named("  ", in: context) == nil)
    #expect(try context.fetchCount(FetchDescriptor<Payee>()) == 1)
}

@Test(arguments: [
    ("500", 20_000, 50_000), ("+100", 20_000, 30_000), ("-50", 20_000, 15_000), ("- 300", 20_000, 0), ("+$1,000.50", 0, 100_050),
])
func budgetInputSetsOrAdjustsTheAssignedAmount(input: String, current: Int, result: Int) {
    #expect(Money.budgeted(from: input, current: current) == result)
}

@Test func budgetInputRejectsInvalidText() {
    #expect(Money.budgeted(from: "+abc", current: 20_000) == nil)
}
