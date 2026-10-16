import Foundation
import SwiftData

/// Short, stable text ids for models ("Account/p3": entity + SQLite primary key),
/// so MCP clients can refer to rows across calls and processes.
public enum ModelID {
    public static func string(_ id: PersistentIdentifier) -> String {
        // PersistentIdentifier exposes its primary key only through its Codable form.
        let data = (try? JSONEncoder().encode(id)) ?? Data()
        let json = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any]
        let key = (json?["implementation"] as? [String: Any])?["primaryKey"] as? String ?? "unsaved"
        return "\(id.entityName)/\(key)"
    }

    /// Returns nil for malformed, deleted or other-type ids.
    @MainActor
    public static func find<T: PersistentModel>(_ type: T.Type, _ text: String, in context: ModelContext) throws -> T? {
        guard text.hasPrefix("\(String(describing: T.self))/") else { return nil }
        // ponytail: linear scan of one entity; fine for personal data, index the key if it grows.
        return try context.fetch(FetchDescriptor<T>()).first { string($0.persistentModelID) == text }
    }
}

/// Detects writes made by the MCP helper process (another container on the same store file),
/// using SwiftData's persistent history and the transaction author.
@MainActor
public struct ExternalChanges {
    public static let mcpAuthor = "mcp"
    private let context: ModelContext
    private var token: DefaultHistoryToken?

    public init(context: ModelContext) {
        self.context = context
        token = (try? context.fetchHistory(HistoryDescriptor<DefaultHistoryTransaction>()))?.last?.token
    }

    /// True when the MCP wrote something since the previous poll.
    public mutating func poll() throws -> Bool {
        var descriptor = HistoryDescriptor<DefaultHistoryTransaction>()
        if let token { descriptor.predicate = #Predicate { $0.token > token } }
        let transactions = try context.fetchHistory(descriptor)
        guard let last = transactions.last else { return false }
        token = last.token
        return transactions.contains { $0.author == Self.mcpAuthor }
    }
}
