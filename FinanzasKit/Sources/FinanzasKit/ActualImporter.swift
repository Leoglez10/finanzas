import Foundation
import SQLite3
import SwiftData

/// One-shot import of an Actual Budget `db.sqlite` into an empty store.
/// Mirrors Actual's own views: merged payees/categories resolve through `*_mapping`,
/// split parents are skipped (their children carry the amounts), tombstoned rows are ignored.
public enum ActualImporter {
    public struct Summary: Equatable, Sendable {
        public var accounts = 0, categories = 0, payees = 0, entries = 0, schedules = 0, budgets = 0
    }

    public enum ImportError: Error, Equatable {
        case storeNotEmpty
        case budgetsNotEmpty
        case cannotOpen(String)
        case query(String)
    }

    @MainActor
    public static func importDatabase(at url: URL, into context: ModelContext) throws -> Summary {
        guard try context.fetchCount(FetchDescriptor<Account>()) == 0,
              try context.fetchCount(FetchDescriptor<LedgerEntry>()) == 0 else { throw ImportError.storeNotEmpty }

        let db = try SQLiteReader(url: url)
        var summary = Summary()
        do {
            var accounts: [String: Account] = [:]
            try db.each("SELECT id, name, offbudget, closed, sort_order FROM accounts WHERE IFNULL(tombstone,0)=0") { r in
                let a = Account(name: r.text(1), offBudget: r.int(2) != 0, closed: r.int(3) != 0, sortOrder: r.double(4))
                context.insert(a)
                accounts[r.text(0)] = a
            }

            var groups: [String: CategoryGroup] = [:]
            try db.each("SELECT id, name, is_income, sort_order FROM category_groups WHERE IFNULL(tombstone,0)=0") { r in
                let g = CategoryGroup(name: r.text(1), isIncome: r.int(2) != 0, sortOrder: r.double(3))
                context.insert(g)
                groups[r.text(0)] = g
            }

            var categories: [String: Category] = [:]
            try db.each("SELECT id, name, hidden, cat_group, sort_order FROM categories WHERE IFNULL(tombstone,0)=0") { r in
                let c = Category(name: r.text(1), group: groups[r.text(3)], hidden: r.int(2) != 0, sortOrder: r.double(4))
                context.insert(c)
                categories[r.text(0)] = c
            }

            // Transfer payees ("Transfer : X") are not real payees; their entries import without one.
            var payees: [String: Payee] = [:]
            try db.each("SELECT id, name FROM payees WHERE IFNULL(tombstone,0)=0 AND transfer_acct IS NULL") { r in
                let p = Payee(name: r.text(1))
                context.insert(p)
                payees[r.text(0)] = p
            }
            var payeeMapping: [String: String] = [:]
            try db.each("SELECT id, targetId FROM payee_mapping") { r in payeeMapping[r.text(0)] = r.text(1) }

            var schedules: [String: Schedule] = [:]
            try db.each("""
                SELECT s.id, s.name, s.completed, s.posts_transaction, r.conditions
                FROM schedules s JOIN rules r ON r.id = s.rule
                WHERE IFNULL(s.tombstone,0)=0
                """) { r in
                let rule = ScheduleRule(conditionsJSON: r.text(4))
                guard let recurrence = rule.recurrence else { return }
                let payee = rule.payeeID.flatMap { payees[payeeMapping[$0] ?? $0] }
                let name = r.isNull(1) ? (payee?.name ?? "Sin nombre") : r.text(1)
                let s = Schedule(name: name, amount: rule.amount, recurrence: recurrence,
                                 account: rule.accountID.flatMap { accounts[$0] }, payee: payee,
                                 autoPost: r.int(3) != 0)
                s.completed = r.int(2) != 0
                context.insert(s)
                schedules[r.text(0)] = s
            }

            try db.each("""
                SELECT t.acct, cm.transferId, IFNULL(t.amount,0), pm.targetId, IFNULL(t.notes,''), t.date, t.cleared, t.schedule
                FROM transactions t
                LEFT JOIN category_mapping cm ON cm.id = t.category
                LEFT JOIN payee_mapping pm ON pm.id = t.description
                LEFT JOIN transactions parent ON t.isChild = 1 AND parent.id = t.parent_id
                WHERE IFNULL(t.tombstone,0)=0 AND IFNULL(t.isParent,0)=0
                  AND t.date IS NOT NULL AND t.acct IS NOT NULL
                  AND (IFNULL(t.isChild,0)=0 OR IFNULL(parent.tombstone,1)=0)
                """) { r in
                guard let account = accounts[r.text(0)], let date = Self.date(yyyymmdd: r.int(5)) else { return }
                context.insert(LedgerEntry(date: date, amount: r.int(2), account: account,
                                           category: categories[r.text(1)], payee: payees[r.text(3)],
                                           notes: r.text(4), cleared: r.int(6) != 0,
                                           schedule: schedules[r.text(7)]))
                summary.entries += 1
            }

            summary.budgets = try importBudgetRows(from: db, categories: categories, into: context)
            summary.accounts = accounts.count
            summary.categories = categories.count
            summary.payees = payees.count
            summary.schedules = schedules.count
            try context.save()
        } catch {
            context.rollback()
            throw error
        }
        return summary
    }

    /// Imports only the monthly assigned amounts into a store that already has the categories
    /// (matched by group and category name). Returns how many amounts were created.
    @MainActor
    public static func importBudgets(at url: URL, into context: ModelContext) throws -> Int {
        guard try context.fetchCount(FetchDescriptor<BudgetAmount>()) == 0 else { throw ImportError.budgetsNotEmpty }
        let db = try SQLiteReader(url: url)
        let byName = Dictionary(try context.fetch(FetchDescriptor<Category>()).map { ("\($0.group?.name ?? "")/\($0.name)", $0) },
                                uniquingKeysWith: { first, _ in first })
        var categories: [String: Category] = [:]
        try db.each("""
            SELECT c.id, IFNULL(g.name,'') || '/' || c.name FROM categories c
            LEFT JOIN category_groups g ON g.id = c.cat_group
            WHERE IFNULL(c.tombstone,0)=0
            """) { r in categories[r.text(0)] = byName[r.text(1)] }
        do {
            let count = try importBudgetRows(from: db, categories: categories, into: context)
            try context.save()
            return count
        } catch {
            context.rollback()
            throw error
        }
    }

    /// Merged categories resolve through `category_mapping`, so several Actual rows can sum into one.
    @MainActor
    private static func importBudgetRows(from db: SQLiteReader, categories: [String: Category],
                                         into context: ModelContext) throws -> Int {
        struct Row { var amount = 0; var carryover = false }
        var rows: [String: Row] = [:]  // key: "month|categoryID"
        try db.each("""
            SELECT b.month, cm.transferId, IFNULL(b.amount,0), IFNULL(b.carryover,0)
            FROM zero_budgets b LEFT JOIN category_mapping cm ON cm.id = b.category
            WHERE IFNULL(b.amount,0) != 0 OR IFNULL(b.carryover,0) != 0
            """) { r in
            guard categories[r.text(1)] != nil else { return }
            let key = "\(r.int(0))|\(r.text(1))"
            rows[key, default: Row()].amount += r.int(2)
            if r.int(3) != 0 { rows[key, default: Row()].carryover = true }
        }
        for (key, row) in rows {
            let parts = key.split(separator: "|", maxSplits: 1)
            context.insert(BudgetAmount(month: Int(parts[0])!, amount: row.amount,
                                        category: categories[String(parts[1])], carryover: row.carryover))
        }
        return rows.count
    }

    static func date(yyyymmdd v: Int) -> Date? {
        Calendar.current.date(from: DateComponents(year: v / 10_000, month: v / 100 % 100, day: v % 100))
    }

    static func date(iso: String) -> Date? {
        let p = iso.split(separator: "-").compactMap { Int($0) }
        guard p.count == 3 else { return nil }
        return Calendar.current.date(from: DateComponents(year: p[0], month: p[1], day: p[2]))
    }
}

/// The parts of an Actual schedule rule (`rules.conditions` JSON) this app models.
struct ScheduleRule {
    var payeeID: String?
    var accountID: String?
    var amount = 0
    var recurrence: Recurrence?

    init(conditionsJSON: String) {
        let conditions = (try? JSONSerialization.jsonObject(with: Data(conditionsJSON.utf8))) as? [[String: Any]] ?? []
        for c in conditions {
            let value = c["value"]
            switch c["field"] as? String {
            case "description": payeeID = value as? String
            case "acct": accountID = value as? String
            case "amount":
                if let n = value as? Int { amount = n }
                else if let range = value as? [String: Any], let a = range["num1"] as? Int, let b = range["num2"] as? Int {
                    amount = (a + b) / 2  // "isbetween": plan for the midpoint
                }
            case "date":
                if let iso = value as? String, let d = ActualImporter.date(iso: iso) {
                    recurrence = Recurrence(start: d, frequency: .monthly, endDate: d)  // one-off
                } else if let config = value as? [String: Any] {
                    recurrence = Self.recurrence(config)
                }
            default: break
            }
        }
    }

    // ponytail: Actual's `patterns` (e.g. "2nd Tuesday") are ignored; add when a schedule needs them.
    private static func recurrence(_ c: [String: Any]) -> Recurrence? {
        guard let start = (c["start"] as? String).flatMap(ActualImporter.date(iso:)),
              let frequency = (c["frequency"] as? String).flatMap(Recurrence.Frequency.init(rawValue:)) else { return nil }
        var r = Recurrence(start: start, frequency: frequency, interval: c["interval"] as? Int ?? 1,
                           skipWeekend: c["skipWeekend"] as? Bool ?? false,
                           weekendMode: c["weekendSolveMode"] as? String == "before" ? .before : .after)
        switch c["endMode"] as? String {
        case "on_date":
            r.endDate = (c["endDate"] as? String).flatMap(ActualImporter.date(iso:))
        case "after_n_occurrences":
            // Actual counts raw (un-shifted) occurrences.
            var raw = r
            raw.skipWeekend = false
            let n = max(1, c["endOccurrences"] as? Int ?? 1)
            var last = start
            for _ in 1..<n {
                guard let next = raw.next(onOrAfter: Calendar.current.date(byAdding: .day, value: 1, to: last)!) else { break }
                last = next
            }
            r.endDate = last
        default: break
        }
        return r
    }
}

/// Read-only row iteration over a SQLite file.
final class SQLiteReader {
    private var db: OpaquePointer?

    init(url: URL) throws {
        guard sqlite3_open_v2(url.path, &db, SQLITE_OPEN_READONLY, nil) == SQLITE_OK else {
            let message = db.map { String(cString: sqlite3_errmsg($0)) } ?? "unknown"
            sqlite3_close(db)
            throw ActualImporter.ImportError.cannotOpen(message)
        }
    }

    deinit { sqlite3_close(db) }

    struct Row {
        let stmt: OpaquePointer
        func isNull(_ i: Int32) -> Bool { sqlite3_column_type(stmt, i) == SQLITE_NULL }
        func text(_ i: Int32) -> String { sqlite3_column_text(stmt, i).map { String(cString: $0) } ?? "" }
        func int(_ i: Int32) -> Int { Int(sqlite3_column_int64(stmt, i)) }
        func double(_ i: Int32) -> Double { sqlite3_column_double(stmt, i) }
    }

    func each(_ sql: String, _ body: (Row) throws -> Void) throws {
        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK, let stmt else {
            throw ActualImporter.ImportError.query(String(cString: sqlite3_errmsg(db)))
        }
        defer { sqlite3_finalize(stmt) }
        while true {
            switch sqlite3_step(stmt) {
            case SQLITE_ROW: try body(Row(stmt: stmt))
            case SQLITE_DONE: return
            default: throw ActualImporter.ImportError.query(String(cString: sqlite3_errmsg(db)))
            }
        }
    }
}
