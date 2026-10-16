import Foundation
import SQLite3
import SwiftData
import Testing
@testable import FinanzasKit

/// Minimal copy of Actual's schema (only the columns the importer reads).
private let fixtureSQL = """
CREATE TABLE accounts (id TEXT PRIMARY KEY, name TEXT, offbudget INTEGER DEFAULT 0, closed INTEGER DEFAULT 0, tombstone INTEGER DEFAULT 0, sort_order REAL);
CREATE TABLE category_groups (id TEXT PRIMARY KEY, name TEXT, is_income INTEGER DEFAULT 0, sort_order REAL, tombstone INTEGER DEFAULT 0, hidden BOOLEAN NOT NULL DEFAULT 0);
CREATE TABLE categories (id TEXT PRIMARY KEY, name TEXT, is_income INTEGER DEFAULT 0, cat_group TEXT, sort_order REAL, tombstone INTEGER DEFAULT 0, hidden BOOLEAN NOT NULL DEFAULT 0);
CREATE TABLE category_mapping (id TEXT PRIMARY KEY, transferId TEXT);
CREATE TABLE payees (id TEXT PRIMARY KEY, name TEXT, tombstone INTEGER DEFAULT 0, transfer_acct TEXT);
CREATE TABLE payee_mapping (id TEXT PRIMARY KEY, targetId TEXT);
CREATE TABLE transactions (id TEXT PRIMARY KEY, isParent INTEGER DEFAULT 0, isChild INTEGER DEFAULT 0, acct TEXT, category TEXT, amount INTEGER, description TEXT, notes TEXT, date INTEGER, tombstone INTEGER DEFAULT 0, cleared INTEGER DEFAULT 1, parent_id TEXT, schedule TEXT);
CREATE TABLE rules (id TEXT PRIMARY KEY, conditions TEXT, actions TEXT, tombstone INTEGER DEFAULT 0);
CREATE TABLE schedules (id TEXT PRIMARY KEY, rule TEXT, completed INTEGER DEFAULT 0, posts_transaction INTEGER DEFAULT 0, tombstone INTEGER DEFAULT 0, name TEXT);
CREATE TABLE schedules_next_date (id TEXT PRIMARY KEY, schedule_id TEXT, local_next_date INTEGER, local_next_date_ts INTEGER, base_next_date INTEGER, base_next_date_ts INTEGER, tombstone INTEGER DEFAULT 0);
CREATE TABLE zero_budgets (id TEXT PRIMARY KEY, month INTEGER, category TEXT, amount INTEGER DEFAULT 0, carryover INTEGER DEFAULT 0);

INSERT INTO accounts VALUES ('a1','Banco',0,0,0,1), ('a2','Inversión',1,0,0,2), ('a3','Borrada',0,0,1,3);
INSERT INTO category_groups VALUES ('g1','Gastos',0,1,0,0), ('g2','Ingresos',1,2,0,0);
INSERT INTO categories VALUES ('c1','Comida',0,'g1',1,0,0), ('c2','Sueldo',1,'g2',2,0,0), ('cOld','Súper',0,'g1',3,1,0);
INSERT INTO category_mapping VALUES ('c1','c1'), ('c2','c2'), ('cOld','c1');
INSERT INTO payees VALUES ('p1','Oxxo',0,NULL), ('p2','Gym',0,NULL), ('pOld','OXXO',1,NULL), ('pT','',0,'a2');
INSERT INTO payee_mapping VALUES ('p1','p1'), ('p2','p2'), ('pOld','p1'), ('pT','pT');
INSERT INTO transactions VALUES
  ('t1',0,0,'a1','c2',2500000,'p1','quincena',20260915,0,1,NULL,NULL),
  ('t2',0,0,'a1','cOld',-12550,'pOld','',20260920,0,0,NULL,NULL),
  ('t3',0,0,'a1','c1',-999,'p1','',20260921,1,1,NULL,NULL),
  ('tp',1,0,'a1',NULL,-30000,'p1','split',20260922,0,1,NULL,NULL),
  ('tc1',0,1,'a1','c1',-10000,'p1','',20260922,0,1,'tp',NULL),
  ('tc2',0,1,'a1','c1',-20000,'p1','',20260922,0,1,'tp',NULL),
  ('t4',0,0,'a1',NULL,-50000,'pT','',20260923,0,1,NULL,NULL),
  ('t5',0,0,'a1','c1',-2500,'p2','',20261001,0,1,NULL,'s1');
INSERT INTO rules VALUES
  ('r1','[{"op":"is","field":"description","value":"p2"},{"op":"is","field":"acct","value":"a1"},{"op":"isapprox","field":"date","value":{"start":"2026-05-11","interval":1,"frequency":"monthly","patterns":[],"skipWeekend":true,"weekendSolveMode":"before","endMode":"never","endOccurrences":1,"endDate":"2026-04-12"}},{"op":"is","field":"amount","value":-2500}]','[]',0),
  ('r2','[{"op":"is","field":"acct","value":"a1"},{"op":"is","field":"date","value":{"start":"2026-01-10","interval":2,"frequency":"weekly","patterns":[],"skipWeekend":false,"weekendSolveMode":"after","endMode":"after_n_occurrences","endOccurrences":3,"endDate":"2026-01-01"}},{"op":"isbetween","field":"amount","value":{"num1":-1000,"num2":-2000}}]','[]',0),
  ('r3','[{"op":"is","field":"date","value":"2026-12-24"},{"op":"is","field":"amount","value":-80000}]','[]',0);
INSERT INTO schedules_next_date VALUES
  ('n1','s1',20261011,5,20261011,5,0),
  ('n2','s2',20260101,1,20260207,2,0);
INSERT INTO zero_budgets VALUES
  ('b1',202609,'c1',30000,0), ('b2',202609,'cOld',5000,0), ('b3',202610,'c1',20000,1),
  ('b4',202609,'c2',0,0), ('b5',202609,'cGone',100,0);
INSERT INTO schedules VALUES ('s1','r1',0,1,0,'Gym'), ('s2','r2',1,0,0,'Quincenal'), ('s3','r3',0,0,0,'Regalo'), ('sDead','r1',0,0,1,'Viejo');
"""

private func makeFixture() throws -> URL {
    let url = FileManager.default.temporaryDirectory.appending(path: "actual-fixture-\(UUID().uuidString).sqlite")
    var db: OpaquePointer?
    #expect(sqlite3_open(url.path, &db) == SQLITE_OK)
    defer { sqlite3_close(db) }
    var error: UnsafeMutablePointer<CChar>?
    let rc = sqlite3_exec(db, fixtureSQL, nil, nil, &error)
    #expect(rc == SQLITE_OK, Comment(rawValue: error.map { String(cString: $0) } ?? ""))
    return url
}

private func day(_ y: Int, _ m: Int, _ d: Int) -> Date {
    Calendar.current.date(from: DateComponents(year: y, month: m, day: d))!
}

@MainActor
@Test func importsLiveRowsWithMappingsResolved() throws {
    let container = try FinanzasStore.inMemory()
    let context = container.mainContext
    let summary = try ActualImporter.importDatabase(at: try makeFixture(), into: context)

    #expect(summary == .init(accounts: 2, categories: 2, payees: 2, entries: 6, schedules: 3, budgets: 2))

    let accounts = try context.fetch(FetchDescriptor<Account>(sortBy: [SortDescriptor(\.sortOrder)]))
    #expect(accounts.map(\.name) == ["Banco", "Inversión"])
    #expect(accounts[1].offBudget)
    // 25,000.00 - 125.50 - 100 - 200 (split children, parent skipped) - 500 (transfer) - 25
    #expect(accounts[0].balance == 2_500_000 - 12_550 - 10_000 - 20_000 - 50_000 - 2_500)

    let groups = try context.fetch(FetchDescriptor<CategoryGroup>(sortBy: [SortDescriptor(\.sortOrder)]))
    #expect(groups.map(\.name) == ["Gastos", "Ingresos"])
    #expect(groups[1].isIncome)
    #expect(groups[0].categories?.map(\.name) == ["Comida"])

    let entries = try context.fetch(FetchDescriptor<LedgerEntry>())
    let merged = try #require(entries.first { $0.amount == -12_550 })
    #expect(merged.category?.name == "Comida")   // cOld mapped to c1
    #expect(merged.payee?.name == "Oxxo")        // pOld mapped to p1
    #expect(merged.cleared == false)
    #expect(merged.date == day(2026, 9, 20))
    let transfer = try #require(entries.first { $0.amount == -50_000 })
    #expect(transfer.payee == nil && transfer.category == nil)
    let gymEntry = try #require(entries.first { $0.amount == -2_500 })
    #expect(gymEntry.schedule?.name == "Gym")
}

@MainActor
@Test func importsSchedulesWithTheirRecurrence() throws {
    let container = try FinanzasStore.inMemory()
    let context = container.mainContext
    _ = try ActualImporter.importDatabase(at: try makeFixture(), into: context)
    let schedules = Dictionary(uniqueKeysWithValues: try context.fetch(FetchDescriptor<Schedule>()).map { ($0.name, $0) })

    let gym = try #require(schedules["Gym"])
    #expect(gym.amount == -2_500 && gym.autoPost && !gym.completed)
    #expect(gym.payee?.name == "Gym" && gym.account?.name == "Banco")
    #expect(gym.recurrence == Recurrence(start: day(2026, 5, 11), frequency: .monthly,
                                         skipWeekend: true, weekendMode: .before))

    #expect(gym.nextDate == day(2026, 10, 11))           // local == base timestamp → local
    let biweekly = try #require(schedules["Quincenal"])
    #expect(biweekly.nextDate == day(2026, 2, 7))        // timestamps differ → base
    #expect(biweekly.amount == -1_500)  // isbetween → midpoint
    #expect(biweekly.completed)
    #expect(biweekly.recurrence?.endDate == day(2026, 2, 7))  // 3rd occurrence: Jan 10, 24, Feb 7

    let oneOff = try #require(schedules["Regalo"])
    #expect(oneOff.recurrence?.occurrences(from: day(2026, 1, 1), through: day(2027, 12, 31)) == [day(2026, 12, 24)])
}

@MainActor
@Test func refusesToImportIntoANonEmptyStore() throws {
    let container = try FinanzasStore.inMemory()
    let context = container.mainContext
    context.insert(Account(name: "Existente"))
    try context.save()

    #expect(throws: ActualImporter.ImportError.storeNotEmpty) {
        try ActualImporter.importDatabase(at: try makeFixture(), into: context)
    }
    #expect(try context.fetchCount(FetchDescriptor<Account>()) == 1)
}

@MainActor
@Test func importsBudgetAmountsMergingMappedCategories() throws {
    let container = try FinanzasStore.inMemory()
    let context = container.mainContext
    _ = try ActualImporter.importDatabase(at: try makeFixture(), into: context)
    let amounts = try context.fetch(FetchDescriptor<BudgetAmount>(sortBy: [SortDescriptor(\.month)]))
    #expect(amounts.map(\.month) == [202609, 202610])
    #expect(amounts.map(\.amount) == [35_000, 20_000])  // cOld merged into c1
    #expect(amounts.map(\.carryover) == [false, true])
    #expect(amounts.allSatisfy { $0.category?.name == "Comida" })
}

@MainActor
@Test func importsBudgetsAloneIntoAnExistingStoreByCategoryName() throws {
    let container = try FinanzasStore.inMemory()
    let context = container.mainContext
    let fixture = try makeFixture()
    _ = try ActualImporter.importDatabase(at: fixture, into: context)
    try context.delete(model: BudgetAmount.self)
    try context.save()

    #expect(try ActualImporter.importBudgets(at: fixture, into: context) == 2)
    #expect(try context.fetch(FetchDescriptor<BudgetAmount>()).map(\.amount).sorted() == [20_000, 35_000])
    #expect(throws: ActualImporter.ImportError.budgetsNotEmpty) {
        try ActualImporter.importBudgets(at: fixture, into: context)
    }
    #expect(try context.fetchCount(FetchDescriptor<BudgetAmount>()) == 2)
}

@MainActor
@Test func importsScheduleDatesIntoAnExistingStoreByName() throws {
    let container = try FinanzasStore.inMemory()
    let context = container.mainContext
    let fixture = try makeFixture()
    _ = try ActualImporter.importDatabase(at: fixture, into: context)
    for s in try context.fetch(FetchDescriptor<Schedule>()) { s.nextDate = nil }
    let extra = Schedule(name: "Solo aquí", amount: -1, recurrence: .init(start: day(2026, 1, 1), frequency: .monthly))
    context.insert(extra)
    try context.save()

    #expect(try ActualImporter.importScheduleDates(at: fixture, into: context) == 2)
    let byName = Dictionary(uniqueKeysWithValues: try context.fetch(FetchDescriptor<Schedule>()).map { ($0.name, $0) })
    #expect(byName["Gym"]?.nextDate == day(2026, 10, 11))
    #expect(byName["Quincenal"]?.nextDate == day(2026, 2, 7))
    #expect(byName["Solo aquí"]?.nextDate == nil)
}

/// Opt-in check against a real Actual export: `ACTUAL_DB=/path/db.sqlite swift test`.
@MainActor
@Test(.enabled(if: ProcessInfo.processInfo.environment["ACTUAL_DB"] != nil))
func importsRealActualDatabase() throws {
    let container = try FinanzasStore.inMemory()
    let url = URL(filePath: ProcessInfo.processInfo.environment["ACTUAL_DB"]!)
    let summary = try ActualImporter.importDatabase(at: url, into: container.mainContext)
    print("Real import:", summary)
    #expect(summary.accounts > 0 && summary.entries > 0)
}
