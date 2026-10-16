import Foundation
import SwiftData
import Testing
@testable import FinanzasKit

private func day(_ y: Int, _ m: Int, _ d: Int) -> Date {
    Calendar.current.date(from: DateComponents(year: y, month: m, day: d))!
}

/// Seeded store: Banco (on-budget), Inversión (off-budget), Gastos/Comida, Ingresos/Sueldo, schedule Gym.
@MainActor
private final class Seed {
    let container: ModelContainer
    let context: ModelContext
    let bank = Account(name: "Banco", sortOrder: 1), invest = Account(name: "Inversión", offBudget: true, sortOrder: 2)
    let expenses = CategoryGroup(name: "Gastos", sortOrder: 1), incomeGroup = CategoryGroup(name: "Ingresos", isIncome: true, sortOrder: 2)
    let food: FinanzasKit.Category, salary: FinanzasKit.Category
    let oxxo = Payee(name: "Oxxo")
    let gym: Schedule

    init() throws {
        container = try FinanzasStore.inMemory()
        context = container.mainContext
        food = Category(name: "Comida", group: expenses, sortOrder: 1)
        salary = Category(name: "Sueldo", group: incomeGroup, sortOrder: 1)
        for m in [bank, invest] as [Account] { context.insert(m) }
        context.insert(expenses); context.insert(incomeGroup); context.insert(food); context.insert(salary); context.insert(oxxo)
        context.insert(LedgerEntry(date: day(2026, 9, 1), amount: 1_000_000, account: bank, category: salary, cleared: true))
        context.insert(LedgerEntry(date: day(2026, 9, 5), amount: -12_550, account: bank, category: food, payee: oxxo, notes: "súper"))
        context.insert(LedgerEntry(date: day(2026, 9, 6), amount: -50_000, account: invest, category: food))
        context.insert(LedgerEntry(date: day(2026, 10, 2), amount: -3_000, account: bank))
        gym = Schedule(name: "Gym", amount: -55_000, recurrence: .init(start: day(2026, 5, 11), frequency: .monthly), account: bank)
        gym.nextDate = day(2026, 10, 11)
        context.insert(gym)
        try BudgetAmount.set(30_000, for: food, month: 202609, in: context)
        try context.save()
    }

    func call(_ name: String, _ args: [String: Any] = [:]) throws -> Any {
        try FinanzasTools.call(name, arguments: args, context: context, today: day(2026, 10, 15))
    }
    func id(_ m: some PersistentModel) -> String { ModelID.string(m.persistentModelID) }
}

private func dict(_ v: Any) -> [String: Any] { v as! [String: Any] }
private func list(_ v: Any) -> [[String: Any]] { v as! [[String: Any]] }

@MainActor @Test func everySpecHasAnObjectSchema() throws {
    let names = FinanzasTools.specs.map(\.name)
    #expect(names == ["list_accounts", "get_transactions", "list_categories", "get_budget_month", "spending_by_category",
                      "add_transaction", "update_transaction", "delete_transaction", "set_budget_amount",
                      "list_schedules", "skip_schedule", "get_upcoming"])
    for spec in FinanzasTools.specs {
        let schema = try JSONSerialization.jsonObject(with: Data(spec.inputSchemaJSON.utf8)) as? [String: Any]
        #expect(schema?["type"] as? String == "object", "\(spec.name)")
    }
}

@MainActor @Test func listAccountsReturnsDecimalBalances() throws {
    let s = try Seed()
    let accounts = list(try s.call("list_accounts"))
    #expect(accounts.map { $0["name"] as? String } == ["Banco", "Inversión"])
    #expect(accounts[0]["balance"] as? Double == 9_844.5)
    #expect(accounts[1]["offBudget"] as? Bool == true)
    #expect(accounts[0]["id"] as? String == s.id(s.bank))
}

@MainActor @Test func getTransactionsFiltersSortsAndTruncates() throws {
    let s = try Seed()
    let all = dict(try s.call("get_transactions", ["startDate": "2026-09-01", "endDate": "2026-09-30"]))
    #expect(all["total"] as? Int == 3)
    let rows = list(all["transactions"]!)
    #expect(rows.map { $0["date"] as? String } == ["2026-09-06", "2026-09-05", "2026-09-01"])
    let oxxoRow = rows[1]
    #expect(oxxoRow["amount"] as? Double == -125.5)
    #expect(oxxoRow["payee"] as? String == "Oxxo" && oxxoRow["category"] as? String == "Comida")
    #expect(oxxoRow["notes"] as? String == "súper" && oxxoRow["account"] as? String == "Banco")

    let byAccount = dict(try s.call("get_transactions", ["startDate": "2026-09-01", "endDate": "2026-10-31",
                                                         "accountId": s.id(s.bank), "limit": 2]))
    #expect(byAccount["total"] as? Int == 3 && byAccount["truncated"] as? Bool == true)
    #expect(list(byAccount["transactions"]!).count == 2)

    let byCategory = dict(try s.call("get_transactions", ["startDate": "2026-09-01", "endDate": "2026-09-30",
                                                          "categoryId": s.id(s.food)]))
    #expect(byCategory["total"] as? Int == 2)

    #expect(throws: FinanzasTools.ToolError("startDate must be on or before endDate")) {
        try s.call("get_transactions", ["startDate": "2026-10-01", "endDate": "2026-09-01"])
    }
    #expect(throws: FinanzasTools.ToolError("startDate: expected YYYY-MM-DD")) {
        try s.call("get_transactions", ["startDate": "1/9/2026", "endDate": "2026-09-01"])
    }
}

@MainActor @Test func listCategoriesGroupsWithFlags() throws {
    let s = try Seed()
    let groups = list(try s.call("list_categories"))
    #expect(groups.map { $0["name"] as? String } == ["Gastos", "Ingresos"])
    #expect(groups[1]["isIncome"] as? Bool == true)
    let cats = list(groups[0]["categories"]!)
    #expect(cats.map { $0["name"] as? String } == ["Comida"])
    #expect(cats[0]["id"] as? String == s.id(s.food) && cats[0]["hidden"] as? Bool == false)
}

@MainActor @Test func budgetMonthMatchesEnvelopeRules() throws {
    let s = try Seed()
    let m = dict(try s.call("get_budget_month", ["month": "2026-09"]))
    #expect(m["toBudget"] as? Double == 9_700 && m["income"] as? Double == 10_000 && m["budgeted"] as? Double == 300)
    let food = list(list(m["groups"]!)[0]["categories"]!)[0]
    #expect(food["budgeted"] as? Double == 300 && food["spent"] as? Double == -125.5 && food["available"] as? Double == 174.5)
    #expect(throws: FinanzasTools.ToolError("month: expected YYYY-MM")) { try s.call("get_budget_month", ["month": "2026-13"]) }
}

@MainActor @Test func setBudgetAmountValidatesAndUpserts() throws {
    let s = try Seed()
    let r = dict(try s.call("set_budget_amount", ["month": "2026-10", "categoryId": s.id(s.food), "amount": 450.25]))
    #expect(r["budgeted"] as? Double == 450.25)
    _ = try s.call("set_budget_amount", ["month": "2026-10", "categoryId": s.id(s.food), "amount": 100])
    #expect(try s.context.fetchCount(FetchDescriptor<BudgetAmount>()) == 2)
    #expect(throws: FinanzasTools.ToolError("amount must be 0 or more")) {
        try s.call("set_budget_amount", ["month": "2026-10", "categoryId": s.id(s.food), "amount": -1])
    }
    #expect(throws: FinanzasTools.ToolError("Income categories cannot be budgeted")) {
        try s.call("set_budget_amount", ["month": "2026-10", "categoryId": s.id(s.salary), "amount": 1])
    }
}

@MainActor @Test func spendingByCategoryExcludesInflowsAndOffBudget() throws {
    let s = try Seed()
    let r = dict(try s.call("spending_by_category", ["startDate": "2026-09-01", "endDate": "2026-10-31"]))
    #expect(r["total"] as? Double == 155.5)
    #expect(list(r["categories"]!).map { $0["category"] as? String } == ["Comida", "Sin categoría"])
    let withOff = dict(try s.call("spending_by_category", ["startDate": "2026-09-01", "endDate": "2026-10-31",
                                                           "includeOffBudget": true]))
    #expect(list(withOff["categories"]!).first?["spent"] as? Double == 625.5)
}

@MainActor @Test func addTransactionReusesPayeeAndDefaultsCleared() throws {
    let s = try Seed()
    let t = dict(try s.call("add_transaction", ["accountId": s.id(s.bank), "date": "2026-10-14", "amount": -89.9,
                                                 "payee": "OXXO", "categoryId": s.id(s.food), "notes": "café"]))
    #expect(t["amount"] as? Double == -89.9 && t["cleared"] as? Bool == true && t["payee"] as? String == "Oxxo")
    #expect(try s.context.fetchCount(FetchDescriptor<Payee>()) == 1)
    #expect(s.bank.balance == 984_450 - 8_990)
    #expect(throws: FinanzasTools.ToolError("amount: at most 2 decimals")) {
        try s.call("add_transaction", ["accountId": s.id(s.bank), "date": "2026-10-14", "amount": 1.234])
    }
    #expect(throws: FinanzasTools.ToolError("accountId: not found")) {
        try s.call("add_transaction", ["accountId": "nope", "date": "2026-10-14", "amount": 1])
    }
}

@MainActor @Test func updateTransactionChangesOnlyGivenFields() throws {
    let s = try Seed()
    let added = dict(try s.call("add_transaction", ["accountId": s.id(s.bank), "date": "2026-10-14", "amount": -10]))
    let id = added["id"] as! String
    let t = dict(try s.call("update_transaction", ["id": id, "notes": "corregido", "payee": "Uber"]))
    #expect(t["notes"] as? String == "corregido" && t["payee"] as? String == "Uber" && t["amount"] as? Double == -10)
    #expect(throws: FinanzasTools.ToolError("Provide at least one field to change (date, amount, payee, categoryId, notes, cleared)")) {
        try s.call("update_transaction", ["id": id])
    }
    #expect(throws: FinanzasTools.ToolError("Payee must not be empty")) {
        try s.call("update_transaction", ["id": id, "payee": "  "])
    }
}

@MainActor @Test func deleteTransactionReturnsTheDeletedRow() throws {
    let s = try Seed()
    let added = dict(try s.call("add_transaction", ["accountId": s.id(s.bank), "date": "2026-10-14", "amount": -10]))
    let r = dict(try s.call("delete_transaction", ["id": added["id"]!]))
    #expect(dict(r["deleted"]!)["amount"] as? Double == -10)
    #expect(try s.context.fetchCount(FetchDescriptor<LedgerEntry>()) == 4)
    #expect(throws: FinanzasTools.ToolError("id: not found")) { try s.call("delete_transaction", ["id": added["id"]!]) }
}

@MainActor @Test func schedulesListSkipAndUpcoming() throws {
    let s = try Seed()
    let schedules = list(try s.call("list_schedules"))
    #expect(schedules.first?["nextDate"] as? String == "2026-10-11")
    #expect(schedules.first?["frequency"] as? String == "monthly" && schedules.first?["amount"] as? Double == -550)

    let upcoming = list(try s.call("get_upcoming", ["days": 30]))
    #expect(upcoming.map { $0["date"] as? String } == ["2026-10-11", "2026-11-11"])
    #expect(upcoming.map { $0["overdue"] as? Bool } == [true, false])
    #expect(upcoming.map { $0["projectedBalance"] as? Double } == [9_294.5, 8_744.5])

    let once = dict(try s.call("skip_schedule", ["id": s.id(s.gym)]))
    #expect(once["skipped"] as? Int == 1 && once["nextDate"] as? String == "2026-11-11")
    let through = dict(try s.call("skip_schedule", ["id": s.id(s.gym), "through": "2027-01-31"]))
    #expect(through["skipped"] as? Int == 3 && through["nextDate"] as? String == "2027-02-11")
}

@MainActor @Test func unknownToolAndBadIdsAreErrors() throws {
    let s = try Seed()
    #expect(throws: FinanzasTools.ToolError("Unknown tool: nope")) { try s.call("nope") }
    #expect(throws: FinanzasTools.ToolError("id: not found")) { try s.call("skip_schedule", ["id": s.id(s.bank)]) }
    #expect(throws: FinanzasTools.ToolError("accountId: required")) {
        try s.call("add_transaction", ["date": "2026-10-14", "amount": 1])
    }
}

@MainActor @Test func writesFromAnotherContainerAreDetectedByAuthor() throws {
    let url = FileManager.default.temporaryDirectory.appending(path: "finanzas-\(UUID().uuidString)/F.store")
    try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
    let app = try FinanzasStore.onDisk(at: url)
    var watcher = ExternalChanges(context: app.mainContext)
    #expect(try watcher.poll() == false)

    let mcp = try FinanzasStore.onDisk(at: url)
    let mcpContext = ModelContext(mcp)
    mcpContext.author = ExternalChanges.mcpAuthor
    mcpContext.insert(Account(name: "Desde IA"))
    try mcpContext.save()

    app.mainContext.insert(Account(name: "Desde la app"))
    try app.mainContext.save()

    #expect(try watcher.poll() == true)
    #expect(try watcher.poll() == false)
}

@MainActor @Test func idsAreShortStableAndTyped() throws {
    let s = try Seed()
    let first = s.id(s.bank)
    #expect(first.wholeMatch(of: /Account\/p\d+/) != nil)
    #expect((0..<20).allSatisfy { _ in s.id(s.bank) == first })
    #expect(try ModelID.find(Account.self, first, in: s.context) === s.bank)
    #expect(try ModelID.find(Schedule.self, first, in: s.context) == nil)
    #expect(try ModelID.find(Account.self, "Account/p999", in: s.context) == nil)
}

/// Arguments as an MCP client sends them: parsed from JSON text (0 and 1 bridge oddly on Darwin).
@MainActor private func callJSON(_ s: Seed, _ name: String, _ json: String) throws -> Any {
    let args = try JSONSerialization.jsonObject(with: Data(json.utf8)) as! [String: Any]
    return try FinanzasTools.call(name, arguments: args, context: s.context, today: day(2026, 10, 15))
}

@MainActor @Test func jsonZeroAndOneAreAmountsNotBooleans() throws {
    let s = try Seed()
    let food = s.id(s.food), bank = s.id(s.bank)
    let zero = dict(try callJSON(s, "set_budget_amount", #"{"month":"2026-09","categoryId":"\#(food)","amount":0}"#))
    #expect(zero["budgeted"] as? Double == 0)
    let one = dict(try callJSON(s, "add_transaction", #"{"accountId":"\#(bank)","date":"2026-10-14","amount":1}"#))
    #expect(one["amount"] as? Double == 1)
    let updated = dict(try callJSON(s, "update_transaction", #"{"id":"\#(one["id"]!)","amount":0}"#))
    #expect(updated["amount"] as? Double == 0)
    let fromFloat = dict(try callJSON(s, "add_transaction", #"{"accountId":"\#(bank)","date":"2026-10-14","amount":1.0}"#))
    #expect(fromFloat["amount"] as? Double == 1)
}

@MainActor @Test func wrongTypedArgumentsAreRejectedNotIgnored() throws {
    let s = try Seed()
    let bank = s.id(s.bank)
    let range = #""startDate":"2026-09-01","endDate":"2026-09-30""#
    #expect(throws: FinanzasTools.ToolError("accountId: expected a string")) {
        try callJSON(s, "get_transactions", "{\(range),\"accountId\":123}")
    }
    #expect(throws: FinanzasTools.ToolError("includeOffBudget: expected true or false")) {
        try callJSON(s, "spending_by_category", "{\(range),\"includeOffBudget\":\"yes\"}")
    }
    #expect(throws: FinanzasTools.ToolError("cleared: expected true or false")) {
        try callJSON(s, "add_transaction", #"{"accountId":"\#(bank)","date":"2026-10-14","amount":5,"cleared":"false"}"#)
    }
    #expect(throws: FinanzasTools.ToolError("cleared: expected true or false")) {
        try callJSON(s, "add_transaction", #"{"accountId":"\#(bank)","date":"2026-10-14","amount":5,"cleared":1}"#)
    }
    #expect(throws: FinanzasTools.ToolError("limit: expected an integer from 1 to 1000")) {
        try callJSON(s, "get_transactions", "{\(range),\"limit\":1.5}")
    }
    #expect(throws: FinanzasTools.ToolError("days: expected an integer from 0 to 365")) {
        try callJSON(s, "get_upcoming", #"{"days":true}"#)
    }
    #expect(throws: FinanzasTools.ToolError("amount: expected a number")) {
        try callJSON(s, "add_transaction", #"{"accountId":"\#(bank)","date":"2026-10-14","amount":"12.5"}"#)
    }
    #expect(throws: FinanzasTools.ToolError("through: expected YYYY-MM-DD")) {
        try callJSON(s, "skip_schedule", #"{"id":"\#(s.id(s.gym))","through":12}"#)
    }
    // Valid booleans still work.
    let t = dict(try callJSON(s, "add_transaction", #"{"accountId":"\#(bank)","date":"2026-10-14","amount":5,"cleared":false}"#))
    #expect(t["cleared"] as? Bool == false)
}

@MainActor @Test func hugeIntegersAreRejectedWithoutCrashing() throws {
    let s = try Seed()
    #expect(throws: FinanzasTools.ToolError("limit: expected an integer from 1 to 1000")) {
        try callJSON(s, "get_transactions", #"{"startDate":"2026-09-01","endDate":"2026-09-30","limit":1e300}"#)
    }
    #expect(throws: FinanzasTools.ToolError("days: expected an integer from 0 to 365")) {
        try callJSON(s, "get_upcoming", #"{"days":-1e300}"#)
    }
}

@MainActor @Test func absurdAmountsAreRejected() throws {
    let s = try Seed()
    #expect(throws: FinanzasTools.ToolError("amount: must be between -1000000000000 and 1000000000000")) {
        try callJSON(s, "add_transaction", #"{"accountId":"\#(s.id(s.bank))","date":"2026-10-14","amount":1e17}"#)
    }
    let big = dict(try callJSON(s, "add_transaction", #"{"accountId":"\#(s.id(s.bank))","date":"2026-10-14","amount":-999999999999.99}"#))
    #expect(big["amount"] as? Double == -999_999_999_999.99)
}

@MainActor @Test func addTransactionLearnsCategoryFromPayeeWhenNoneGiven() throws {
    let s = try Seed()
    let t = dict(try s.call("add_transaction", ["accountId": s.id(s.bank), "date": "2026-10-14", "amount": -20, "payee": "oxxo"]))
    #expect(t["category"] as? String == "Comida")
    let explicit = dict(try s.call("add_transaction", ["accountId": s.id(s.bank), "date": "2026-10-14", "amount": -20,
                                                       "payee": "Oxxo", "categoryId": s.id(s.salary)]))
    #expect(explicit["category"] as? String == "Sueldo")
}
