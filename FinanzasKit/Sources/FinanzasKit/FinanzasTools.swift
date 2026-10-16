import Foundation
import SwiftData

/// MCP tool definitions and their implementation, JSON in / JSON out, independent of the MCP SDK.
/// Names and arguments match the earlier actual-mcp server; amounts are decimal currency units.
public enum FinanzasTools {
    public struct Spec: Sendable {
        public let name: String
        public let description: String
        public let inputSchemaJSON: String
        public let readOnly: Bool
        public let destructive: Bool
    }

    public struct ToolError: Error, Equatable, LocalizedError {
        public let message: String
        public init(_ message: String) { self.message = message }
        public var errorDescription: String? { message }
    }

    private static func schema(_ properties: String = "", required: [String] = []) -> String {
        let req = required.map { "\"\($0)\"" }.joined(separator: ",")
        return #"{"type":"object","properties":{"# + properties + #"},"required":["# + req + "]}"
    }
    private static let dateProp = #"{"type":"string","description":"YYYY-MM-DD"}"#
    private static let monthProp = #"{"type":"string","description":"YYYY-MM"}"#
    private static let amountProp = #"{"type":"number","description":"Decimal currency units, max 2 decimals; negative = outflow"}"#

    public static let specs: [Spec] = [
        Spec(name: "list_accounts", description: "List all accounts with id, name, off-budget/closed flags and current balance.",
             inputSchemaJSON: schema(), readOnly: true, destructive: false),
        Spec(name: "get_transactions",
             description: "Get transactions in a date range (newest first), optionally for one account and/or category. Amounts: negative = outflow.",
             inputSchemaJSON: schema(#""accountId":{"type":"string","description":"Account id; all accounts if omitted"},"startDate":"# + dateProp
                                     + #","endDate":"# + dateProp
                                     + #","categoryId":{"type":"string","description":"Only transactions in this category"},"limit":{"type":"integer","minimum":1,"maximum":1000,"description":"Max rows (default 100, max 1000)"}"#,
                                     required: ["startDate", "endDate"]),
             readOnly: true, destructive: false),
        Spec(name: "list_categories", description: "List category groups with their categories (ids, hidden and income flags).",
             inputSchemaJSON: schema(), readOnly: true, destructive: false),
        Spec(name: "get_budget_month",
             description: "Envelope budget for a month: money left to budget, income, and per category the assigned, spent and available amounts.",
             inputSchemaJSON: schema(#""month":"# + monthProp, required: ["month"]), readOnly: true, destructive: false),
        Spec(name: "spending_by_category",
             description: "Total outflows per category in a date range, largest first. Inflows are excluded, and so are off-budget accounts unless includeOffBudget is true.",
             inputSchemaJSON: schema(#""startDate":"# + dateProp + #","endDate":"# + dateProp
                                     + #","includeOffBudget":{"type":"boolean","description":"Also count off-budget accounts (default false)"}"#,
                                     required: ["startDate", "endDate"]),
             readOnly: true, destructive: false),
        Spec(name: "add_transaction",
             description: "Add a transaction to an account. The payee is given by name: an existing payee is reused (case-insensitive), otherwise it is created. Cleared by default. Returns the transaction.",
             inputSchemaJSON: schema(#""accountId":{"type":"string","description":"Account id (see list_accounts)"},"date":"# + dateProp
                                     + #","amount":"# + amountProp
                                     + #","payee":{"type":"string","description":"Payee name"},"categoryId":{"type":"string","description":"Category id (see list_categories)"},"notes":{"type":"string"},"cleared":{"type":"boolean","description":"Mark as cleared (default true)"}"#,
                                     required: ["accountId", "date", "amount"]),
             readOnly: false, destructive: false),
        Spec(name: "update_transaction",
             description: "Change fields of an existing transaction; only the fields given are changed, at least one is required. The payee is given by name (reused case-insensitively, created if new).",
             inputSchemaJSON: schema(#""id":{"type":"string","description":"Transaction id (see get_transactions)"},"date":"# + dateProp
                                     + #","amount":"# + amountProp
                                     + #","payee":{"type":"string","description":"Payee name (non-empty)"},"categoryId":{"type":"string","description":"Category id"},"notes":{"type":"string","description":"Empty string clears the notes"},"cleared":{"type":"boolean"}"#,
                                     required: ["id"]),
             readOnly: false, destructive: false),
        Spec(name: "delete_transaction", description: "Permanently delete a transaction by id. Returns the deleted transaction.",
             inputSchemaJSON: schema(#""id":{"type":"string","description":"Transaction id (see get_transactions)"}"#, required: ["id"]),
             readOnly: false, destructive: true),
        Spec(name: "set_budget_amount",
             description: "Set the amount assigned to an expense category for a month. Income categories cannot be budgeted. Returns the resulting assigned amount.",
             inputSchemaJSON: schema(#""month":"# + monthProp
                                     + #","categoryId":{"type":"string","description":"Expense category id (see list_categories)"},"amount":{"type":"number","minimum":0,"description":"Decimal currency units, 0 or more, max 2 decimals"}"#,
                                     required: ["month", "categoryId", "amount"]),
             readOnly: false, destructive: false),
        Spec(name: "list_schedules",
             description: "List scheduled payments with id, name, payee, account, amount, frequency, interval, next date, completed and auto-post flags.",
             inputSchemaJSON: schema(), readOnly: true, destructive: false),
        Spec(name: "skip_schedule",
             description: "Skip the next occurrence of a scheduled payment once, or with `through` keep skipping every occurrence dated on or before that day (max 500 skips per call). Completed schedules cannot be skipped.",
             inputSchemaJSON: schema(#""id":{"type":"string","description":"Schedule id (see list_schedules)"},"through":{"type":"string","description":"YYYY-MM-DD; skip all occurrences up to and including this date"}"#,
                                     required: ["id"]),
             readOnly: false, destructive: false),
        Spec(name: "get_upcoming",
             description: "Upcoming scheduled payments from overdue ones through the next `days` days (default 30), oldest first, with each account's projected balance after the payment.",
             inputSchemaJSON: schema(#""days":{"type":"integer","minimum":0,"maximum":365,"description":"Days ahead (default 30)"}"#),
             readOnly: true, destructive: false),
    ]

    /// Runs a tool. Writes are saved before returning. The result is JSONSerialization-compatible.
    @MainActor
    public static func call(_ name: String, arguments: [String: Any], context: ModelContext, today: Date = .now) throws -> Any {
        let a = Args(values: arguments)
        let result: Any
        switch name {
        case "list_accounts":
            result = try context.fetch(FetchDescriptor<Account>(sortBy: [SortDescriptor(\.sortOrder)])).map(json)
        case "get_transactions":
            let (start, end) = try a.range()
            let account = try a.optionalString("accountId").map { try a.model(Account.self, "accountId", $0, context) }
            let category = try a.optionalString("categoryId").map { try a.model(Category.self, "categoryId", $0, context) }
            let limit = try a.int("limit", default: 100, in: 1...1000)
            let matching = try entries(from: start, through: end, context: context).filter {
                (account == nil || $0.account === account) && (category == nil || $0.category === category)
            }
            result = ["total": matching.count, "truncated": matching.count > limit,
                      "transactions": matching.prefix(limit).map(json)] as [String: Any]
        case "list_categories":
            result = try context.fetch(FetchDescriptor<CategoryGroup>(sortBy: [SortDescriptor(\.sortOrder)])).map { g in
                ["id": id(g), "name": g.name, "isIncome": g.isIncome,
                 "categories": sorted(g).map { ["id": id($0), "name": $0.name, "hidden": $0.hidden] }] as [String: Any]
            }
        case "get_budget_month":
            let month = try a.month("month")
            let budget = try EnvelopeBudget.load(from: context).month(month)
            let groups = try context.fetch(FetchDescriptor<CategoryGroup>(sortBy: [SortDescriptor(\.sortOrder)]))
            result = ["month": a.string(month), "toBudget": dec(budget.toBudget), "income": dec(budget.income),
                      "budgeted": dec(budget.budgeted),
                      "groups": groups.map { g in
                          ["id": id(g), "name": g.name, "isIncome": g.isIncome,
                           "categories": sorted(g).map { c in
                               let row = budget.rows[c.persistentModelID]
                               return ["id": id(c), "name": c.name, "budgeted": dec(row?.budgeted ?? 0),
                                       "spent": dec(row?.activity ?? 0), "available": dec(row?.available ?? 0)] as [String: Any]
                           }] as [String: Any]
                      }] as [String: Any]
        case "spending_by_category":
            let (start, end) = try a.range()
            let includeOff = try a.bool("includeOffBudget") ?? false
            var totals: [String: (id: Any, name: String, cents: Int)] = [:]
            for e in try entries(from: start, through: end, context: context) where e.amount < 0 {
                guard includeOff || !(e.account?.offBudget ?? false) else { continue }
                let key = e.category.map(id) ?? ""
                let current = totals[key] ?? (e.category.map(id) ?? NSNull(), e.category?.name ?? "Sin categoría", 0)
                totals[key] = (current.id, current.name, current.cents - e.amount)
            }
            let rows = totals.values.sorted { $0.cents > $1.cents }
            result = ["startDate": a.values["startDate"]!, "endDate": a.values["endDate"]!,
                      "total": dec(rows.reduce(0) { $0 + $1.cents }),
                      "categories": rows.map { ["categoryId": $0.id, "category": $0.name, "spent": dec($0.cents)] as [String: Any] }
            ] as [String: Any]
        case "add_transaction":
            let account = try a.model(Account.self, "accountId", a.requiredString("accountId"), context)
            let category = try a.optionalString("categoryId").map { try a.model(Category.self, "categoryId", $0, context) }
            let payee = try a.optionalString("payee").flatMap { try Payee.findOrCreate(named: $0, in: context) }
            let entry = LedgerEntry(date: try a.date("date"), amount: try a.cents("amount"), account: account,
                                    category: category, payee: payee, notes: try a.optionalString("notes") ?? "",
                                    cleared: try a.bool("cleared") ?? true)
            context.insert(entry)
            try context.save()  // ids are temporary until saved
            result = json(entry)
        case "update_transaction":
            let entry = try a.model(LedgerEntry.self, "id", a.requiredString("id"), context)
            let fields = ["date", "amount", "payee", "categoryId", "notes", "cleared"]
            guard fields.contains(where: { a.values[$0] != nil }) else {
                throw ToolError("Provide at least one field to change (date, amount, payee, categoryId, notes, cleared)")
            }
            if let name = try a.optionalString("payee") {
                guard let payee = try Payee.findOrCreate(named: name, in: context) else { throw ToolError("Payee must not be empty") }
                entry.payee = payee
            }
            if a.values["date"] != nil { entry.date = try a.date("date") }
            if a.values["amount"] != nil { entry.amount = try a.cents("amount") }
            if let c = try a.optionalString("categoryId") { entry.category = try a.model(Category.self, "categoryId", c, context) }
            if let notes = try a.optionalString("notes") { entry.notes = notes }
            if let cleared = try a.bool("cleared") { entry.cleared = cleared }
            try context.save()  // a newly created payee needs its permanent id
            result = json(entry)
        case "delete_transaction":
            let entry = try a.model(LedgerEntry.self, "id", a.requiredString("id"), context)
            let snapshot = json(entry)
            context.delete(entry)
            result = ["deleted": snapshot]
        case "set_budget_amount":
            let month = try a.month("month")
            let category = try a.model(Category.self, "categoryId", a.requiredString("categoryId"), context)
            let cents = try a.cents("amount")
            guard cents >= 0 else { throw ToolError("amount must be 0 or more") }
            guard !(category.group?.isIncome ?? false) else { throw ToolError("Income categories cannot be budgeted") }
            try BudgetAmount.set(cents, for: category, month: month, in: context)
            result = ["month": a.string(month), "categoryId": id(category), "category": category.name, "budgeted": dec(cents)]
                as [String: Any]
        case "list_schedules":
            result = try context.fetch(FetchDescriptor<Schedule>(sortBy: [SortDescriptor(\.name)])).map(json)
        case "skip_schedule":
            let schedule = try a.model(Schedule.self, "id", a.requiredString("id"), context)
            guard let previous = schedule.effectiveNextDate else { throw ToolError("Schedule is completed") }
            var skipped = 0
            if a.values["through"] != nil {
                let through = try a.date("through")
                while skipped < 500, let next = schedule.effectiveNextDate, next <= through { schedule.skip(); skipped += 1 }
            } else {
                schedule.skip(); skipped = 1
            }
            result = ["id": id(schedule), "name": schedule.name, "previousNextDate": day(previous),
                      "nextDate": schedule.effectiveNextDate.map(day) ?? NSNull(), "completed": schedule.completed,
                      "skipped": skipped] as [String: Any]
        case "get_upcoming":
            let days = try a.int("days", default: 30, in: 0...365)
            let schedules = try context.fetch(FetchDescriptor<Schedule>())
            result = Upcoming.items(schedules: schedules, today: today, days: days).map { item in
                ["scheduleId": id(item.schedule), "name": item.schedule.name, "date": day(item.date),
                 "amount": dec(item.schedule.amount), "accountId": item.schedule.account.map(id) ?? NSNull(),
                 "account": item.schedule.account?.name ?? NSNull(), "overdue": item.overdue,
                 "projectedBalance": item.projectedBalance.map(dec) ?? NSNull()] as [String: Any]
            }
        default:
            throw ToolError("Unknown tool: \(name)")
        }
        if let spec = specs.first(where: { $0.name == name }), !spec.readOnly { try context.save() }
        return result
    }

    // MARK: - JSON shapes

    private static func id(_ m: some PersistentModel) -> String { ModelID.string(m.persistentModelID) }
    private static func dec(_ cents: Int) -> Double { Double(cents) / 100 }
    private static func day(_ d: Date) -> String {
        let c = Calendar.current.dateComponents([.year, .month, .day], from: d)
        return String(format: "%04d-%02d-%02d", c.year!, c.month!, c.day!)
    }
    private static func sorted(_ g: CategoryGroup) -> [Category] { (g.categories ?? []).sorted { $0.sortOrder < $1.sortOrder } }

    private static func json(_ a: Account) -> [String: Any] {
        ["id": id(a), "name": a.name, "offBudget": a.offBudget, "closed": a.closed, "balance": dec(a.balance)]
    }

    private static func json(_ e: LedgerEntry) -> [String: Any] {
        ["id": id(e), "date": day(e.date), "amount": dec(e.amount),
         "accountId": e.account.map(id) ?? NSNull(), "account": e.account?.name ?? NSNull(),
         "payee": e.payee?.name ?? NSNull(), "categoryId": e.category.map(id) ?? NSNull(),
         "category": e.category?.name ?? NSNull(), "notes": e.notes, "cleared": e.cleared,
         "scheduleId": e.schedule.map(id) ?? NSNull()]
    }

    private static func json(_ s: Schedule) -> [String: Any] {
        ["id": id(s), "name": s.name, "amount": dec(s.amount), "payee": s.payee?.name ?? NSNull(),
         "accountId": s.account.map(id) ?? NSNull(), "account": s.account?.name ?? NSNull(),
         "frequency": s.recurrence?.frequency.rawValue ?? NSNull(), "interval": s.recurrence?.interval ?? 1,
         "nextDate": s.effectiveNextDate.map(day) ?? NSNull(), "completed": s.completed, "autoPost": s.autoPost]
    }

    @MainActor
    private static func entries(from start: Date, through end: Date, context: ModelContext) throws -> [LedgerEntry] {
        try context.fetch(FetchDescriptor<LedgerEntry>(
            predicate: #Predicate { $0.date >= start && $0.date <= end },
            sortBy: [SortDescriptor(\.date, order: .reverse)]))
    }

    // MARK: - Argument parsing

    struct Args {
        let values: [String: Any]

        /// JSON booleans arrive as CFBoolean; plain 0/1 numbers also cast to Bool, so `is Bool` can't tell them apart.
        static func isBoolean(_ v: Any) -> Bool {
            (v as AnyObject) is NSNumber && CFGetTypeID(v as CFTypeRef) == CFBooleanGetTypeID()
        }
        private func present(_ key: String) -> Any? { values[key].flatMap { $0 is NSNull ? nil : $0 } }

        func optionalString(_ key: String) throws -> String? {
            guard let v = present(key) else { return nil }
            guard let s = v as? String else { throw ToolError("\(key): expected a string") }
            return s
        }
        func requiredString(_ key: String) throws -> String {
            guard let s = try optionalString(key), !s.isEmpty else { throw ToolError("\(key): required") }
            return s
        }
        func bool(_ key: String) throws -> Bool? {
            guard let v = present(key) else { return nil }
            guard Self.isBoolean(v), let b = v as? Bool else { throw ToolError("\(key): expected true or false") }
            return b
        }

        func int(_ key: String, default value: Int, in range: ClosedRange<Int>) throws -> Int {
            guard let raw = present(key) else { return value }
            guard !Self.isBoolean(raw), let d = (raw as? NSNumber)?.doubleValue, d == d.rounded(),
                  range.contains(Int(d)) else {
                throw ToolError("\(key): expected an integer from \(range.lowerBound) to \(range.upperBound)")
            }
            return Int(d)
        }

        func date(_ key: String) throws -> Date {
            guard let v = present(key) else { throw ToolError("\(key): required") }
            guard let s = v as? String, s.wholeMatch(of: /\d{4}-\d{2}-\d{2}/) != nil, let d = ActualImporter.date(iso: s),
                  FinanzasTools.day(d) == s else { throw ToolError("\(key): expected YYYY-MM-DD") }
            return d
        }

        func range() throws -> (Date, Date) {
            let start = try date("startDate"), end = try date("endDate")
            guard start <= end else { throw ToolError("startDate must be on or before endDate") }
            return (start, end)
        }

        func month(_ key: String) throws -> Int {
            guard let s = present(key) as? String, let m = s.wholeMatch(of: /(\d{4})-(0[1-9]|1[0-2])/) else {
                throw ToolError("\(key): expected YYYY-MM")
            }
            return Int(m.1)! * 100 + Int(m.2)!
        }
        func string(_ month: Int) -> String { String(format: "%04d-%02d", month / 100, month % 100) }

        /// Decimal currency units → cents, rejecting more than 2 decimals.
        func cents(_ key: String) throws -> Int {
            guard let v = present(key) else { throw ToolError("\(key): required") }
            guard !Self.isBoolean(v), let n = v as? NSNumber else { throw ToolError("\(key): expected a number") }
            // Go through the shortest decimal text so 89.9 stays 89.9 rather than 89.8999….
            guard var value = Decimal(string: "\(n.doubleValue)", locale: Locale(identifier: "en_US_POSIX")) else {
                throw ToolError("\(key): expected a number")
            }
            value *= 100
            var rounded = Decimal()
            NSDecimalRound(&rounded, &value, 0, .plain)
            guard rounded == value else { throw ToolError("\(key): at most 2 decimals") }
            return NSDecimalNumber(decimal: rounded).intValue
        }

        @MainActor
        func model<T: PersistentModel>(_ type: T.Type, _ key: String, _ text: String, _ context: ModelContext) throws -> T {
            guard let m = try ModelID.find(type, text, in: context) else { throw ToolError("\(key): not found") }
            return m
        }
    }
}
