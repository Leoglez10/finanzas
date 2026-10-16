import Foundation
import SwiftData

// CloudKit-compatible on purpose (iCloud sync later): every attribute has a default,
// every relationship is optional with an explicit inverse, no unique constraints.
// Money is integer cents everywhere.

@Model public final class Account {
    public var name: String = ""
    public var offBudget: Bool = false
    public var closed: Bool = false
    public var sortOrder: Double = 0
    @Relationship(deleteRule: .cascade, inverse: \LedgerEntry.account) public var entries: [LedgerEntry]? = []
    @Relationship(deleteRule: .nullify, inverse: \Schedule.account) public var schedules: [Schedule]? = []

    public init(name: String, offBudget: Bool = false, closed: Bool = false, sortOrder: Double = 0) {
        self.name = name
        self.offBudget = offBudget
        self.closed = closed
        self.sortOrder = sortOrder
    }

    public var balance: Int { (entries ?? []).reduce(0) { $0 + $1.amount } }
}

@Model public final class CategoryGroup {
    public var name: String = ""
    public var isIncome: Bool = false
    public var sortOrder: Double = 0
    @Relationship(deleteRule: .cascade, inverse: \Category.group) public var categories: [Category]? = []

    public init(name: String, isIncome: Bool = false, sortOrder: Double = 0) {
        self.name = name
        self.isIncome = isIncome
        self.sortOrder = sortOrder
    }
}

@Model public final class Category {
    public var name: String = ""
    public var hidden: Bool = false
    public var sortOrder: Double = 0
    public var group: CategoryGroup?
    @Relationship(deleteRule: .nullify, inverse: \LedgerEntry.category) public var entries: [LedgerEntry]? = []
    @Relationship(deleteRule: .nullify, inverse: \Schedule.category) public var schedules: [Schedule]? = []
    @Relationship(deleteRule: .cascade, inverse: \BudgetAmount.category) public var budgetAmounts: [BudgetAmount]? = []

    public init(name: String, group: CategoryGroup? = nil, hidden: Bool = false, sortOrder: Double = 0) {
        self.name = name
        self.group = group
        self.hidden = hidden
        self.sortOrder = sortOrder
    }
}

@Model public final class Payee {
    public var name: String = ""
    @Relationship(deleteRule: .nullify, inverse: \LedgerEntry.payee) public var entries: [LedgerEntry]? = []
    @Relationship(deleteRule: .nullify, inverse: \Schedule.payee) public var schedules: [Schedule]? = []

    public init(name: String) { self.name = name }
}

@Model public final class LedgerEntry {
    public var date: Date = Date.distantPast
    public var amount: Int = 0
    public var notes: String = ""
    public var cleared: Bool = false
    public var account: Account?
    public var category: Category?
    public var payee: Payee?
    public var schedule: Schedule?

    public init(date: Date, amount: Int, account: Account?, category: Category? = nil,
                payee: Payee? = nil, notes: String = "", cleared: Bool = false, schedule: Schedule? = nil) {
        self.date = date
        self.amount = amount
        self.account = account
        self.category = category
        self.payee = payee
        self.notes = notes
        self.cleared = cleared
        self.schedule = schedule
    }
}

@Model public final class Schedule {
    public var name: String = ""
    public var amount: Int = 0
    public var autoPost: Bool = false
    public var completed: Bool = false
    /// Next unpaid occurrence; posting or skipping moves it forward. Nil = recurrence's first date.
    public var nextDate: Date?
    public var recurrenceData: Data = Data()
    public var account: Account?
    public var category: Category?
    public var payee: Payee?
    @Relationship(deleteRule: .nullify, inverse: \LedgerEntry.schedule) public var entries: [LedgerEntry]? = []

    public init(name: String, amount: Int, recurrence: Recurrence, account: Account? = nil,
                category: Category? = nil, payee: Payee? = nil, autoPost: Bool = false) {
        self.name = name
        self.amount = amount
        self.account = account
        self.category = category
        self.payee = payee
        self.autoPost = autoPost
        self.recurrence = recurrence
    }

    // Stored as JSON so the rule can evolve without a schema migration.
    public var recurrence: Recurrence? {
        get { try? JSONDecoder().decode(Recurrence.self, from: recurrenceData) }
        set { recurrenceData = (try? newValue.map { try JSONEncoder().encode($0) }) ?? Data() }
    }
}

/// Amount assigned to a category for one month (`yyyymm`). `carryover` keeps overspending in the category.
@Model public final class BudgetAmount {
    public var month: Int = 0
    public var amount: Int = 0
    public var carryover: Bool = false
    public var category: Category?

    public init(month: Int, amount: Int, category: Category?, carryover: Bool = false) {
        self.month = month
        self.amount = amount
        self.category = category
        self.carryover = carryover
    }
}

public enum FinanzasStore {
    public static let models: [any PersistentModel.Type] = [
        Account.self, CategoryGroup.self, Category.self, Payee.self, LedgerEntry.self, Schedule.self, BudgetAmount.self,
    ]

    /// `~/Library/Application Support/Finanzas/Finanzas.store` — explicit, because an unsandboxed
    /// app would otherwise share the generic `default.store` name with other apps.
    /// `FINANZAS_STORE` overrides it (tests and manual checks against a copy).
    public static var storeURL: URL {
        ProcessInfo.processInfo.environment["FINANZAS_STORE"].map { URL(filePath: $0) }
            ?? URL.applicationSupportDirectory.appending(path: "Finanzas/Finanzas.store")
    }

    public static func onDisk(at url: URL = storeURL) throws -> ModelContainer {
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        return try ModelContainer(for: Schema(models),
                                  configurations: ModelConfiguration(url: url, cloudKitDatabase: .none))
    }

    public static func inMemory() throws -> ModelContainer {
        try ModelContainer(for: Schema(models), configurations: ModelConfiguration(isStoredInMemoryOnly: true))
    }
}
