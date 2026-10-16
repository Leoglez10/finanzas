import Foundation

/// Months as `yyyymm` integers (same encoding Actual uses for `zero_budgets.month`).
public enum MonthKey {
    public static func of(_ date: Date) -> Int {
        let c = Calendar.current.dateComponents([.year, .month], from: date)
        return c.year! * 100 + c.month!
    }
    public static func next(_ m: Int) -> Int { m % 100 == 12 ? (m / 100 + 1) * 100 + 1 : m + 1 }
    public static func previous(_ m: Int) -> Int { m % 100 == 1 ? (m / 100 - 1) * 100 + 12 : m - 1 }
    public static func firstDay(_ m: Int) -> Date {
        Calendar.current.date(from: DateComponents(year: m / 100, month: m % 100, day: 1))!
    }
}

/// Envelope budgeting with Actual's rules:
/// - toBudget = income + last month's toBudget − last month's overspending − assigned this month
/// - a category's leftover carries forward; overspending resets it to 0 (and is taken from toBudget)
///   unless the category has carryover set for that month.
public struct EnvelopeBudget<ID: Hashable & Sendable>: Sendable {
    public struct CategoryInfo: Sendable { public var id: ID; public var isIncome: Bool
        public init(id: ID, isIncome: Bool) { self.id = id; self.isIncome = isIncome } }
    public struct Key: Hashable, Sendable { public var month: Int; public var category: ID
        public init(month: Int, category: ID) { self.month = month; self.category = category } }
    public struct Row: Equatable, Sendable { public var budgeted: Int; public var activity: Int; public var available: Int }
    public struct Month: Sendable { public var income: Int; public var budgeted: Int; public var toBudget: Int; public var rows: [ID: Row] }

    let categories: [CategoryInfo]
    let budgeted: [Key: Int]
    let carryover: Set<Key>
    let activity: [Key: Int]

    public init(categories: [CategoryInfo], budgeted: [Key: Int], carryover: Set<Key> = [], activity: [Key: Int]) {
        self.categories = categories
        self.budgeted = budgeted
        self.carryover = carryover
        self.activity = activity
    }

    public func month(_ target: Int) -> Month {
        let first = (budgeted.keys.map(\.month) + activity.keys.map(\.month)).min() ?? target
        var carry: [ID: Int] = [:]
        var prevToBudget = 0, prevOverspent = 0
        var m = min(first, target)
        while true {
            var rows: [ID: Row] = [:]
            var income = 0, assigned = 0, overspent = 0
            for c in categories {
                let key = Key(month: m, category: c.id)
                let act = activity[key] ?? 0
                if c.isIncome {
                    income += act
                    rows[c.id] = Row(budgeted: 0, activity: act, available: 0)
                    continue
                }
                let b = budgeted[key] ?? 0
                assigned += b
                let available = (carry[c.id] ?? 0) + b + act
                rows[c.id] = Row(budgeted: b, activity: act, available: available)
                if available >= 0 || carryover.contains(key) { carry[c.id] = available }
                else { carry[c.id] = 0; overspent -= available }
            }
            let toBudget = income + prevToBudget - prevOverspent - assigned
            if m >= target { return Month(income: income, budgeted: assigned, toBudget: toBudget, rows: rows) }
            prevToBudget = toBudget
            prevOverspent = overspent
            m = MonthKey.next(m)
        }
    }
}
