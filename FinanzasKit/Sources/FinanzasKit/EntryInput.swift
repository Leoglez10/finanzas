import Foundation
import SwiftData

public enum Money {
    /// Parses what a person types ("$1,250.50", "80", "-40") into positive cents.
    /// The sign comes from the gasto/ingreso choice, not from the text.
    public static func cents(from text: String) -> Int? {
        let cleaned = text.filter { !" $,-".contains($0) }
        guard cleaned.wholeMatch(of: /\d+(\.\d+)?/) != nil,
              var value = Decimal(string: cleaned, locale: Locale(identifier: "en_US_POSIX")) else { return nil }
        value *= 100
        var rounded = Decimal()
        NSDecimalRound(&rounded, &value, 0, .plain)
        return NSDecimalNumber(decimal: rounded).intValue
    }

    /// Budget cell input: "500" sets the assigned amount, "+100" / "-50" adjust it; never below zero.
    public static func budgeted(from text: String, current: Int) -> Int? {
        let trimmed = text.trimmingCharacters(in: .whitespaces)
        guard let cents = cents(from: trimmed.filter { $0 != "+" }) else { return nil }
        switch trimmed.first {
        case "+": return current + cents
        case "-": return max(0, current - cents)
        default: return cents
        }
    }

    public static func format(_ cents: Int, currency: String = "MXN") -> String {
        (Decimal(cents) / 100).formatted(.currency(code: currency).locale(Locale(identifier: "es_MX")))
    }
}

extension Payee {
    /// Case-insensitive match on the trimmed name; inserts a new payee when none exists.
    @MainActor
    public static func findOrCreate(named rawName: String, in context: ModelContext) throws -> Payee? {
        let name = rawName.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !name.isEmpty else { return nil }
        // ponytail: in-memory scan; personal payee lists are small. Add a normalized-name field if it grows.
        if let existing = try context.fetch(FetchDescriptor<Payee>())
            .first(where: { $0.name.compare(name, options: [.caseInsensitive, .diacriticInsensitive]) == .orderedSame }) {
            return existing
        }
        let payee = Payee(name: name)
        context.insert(payee)
        return payee
    }
}
