import FinanzasKit
import Foundation
import MCP
import SwiftData

// Stdio MCP server bundled inside Finanzas.app. It opens the same store as the app and signs its
// writes with the "mcp" author so the app can notice them and refresh.

let container = try FinanzasStore.onDisk()

let tools = try FinanzasTools.specs.map { spec in
    Tool(name: spec.name, description: spec.description,
         inputSchema: try JSONDecoder().decode(Value.self, from: Data(spec.inputSchemaJSON.utf8)),
         annotations: .init(readOnlyHint: spec.readOnly, destructiveHint: spec.destructive))
}

let server = Server(name: "finanzas", version: "1.0.0", capabilities: .init(tools: .init(listChanged: false)))

await server.withMethodHandler(ListTools.self) { _ in .init(tools: tools) }

await server.withMethodHandler(CallTool.self) { params in
    let name = params.name
    let arguments = try JSONEncoder().encode(params.arguments ?? [:])
    do {
        let text = try await MainActor.run { () throws -> String in
            let args = try JSONSerialization.jsonObject(with: arguments) as? [String: Any] ?? [:]
            // A fresh context per call reads what the app wrote since the last call.
            let context = ModelContext(container)
            context.author = ExternalChanges.mcpAuthor
            let result = try FinanzasTools.call(name, arguments: args, context: context)
            let data = try JSONSerialization.data(withJSONObject: result, options: [.prettyPrinted, .sortedKeys, .fragmentsAllowed])
            return String(decoding: data, as: UTF8.self)
        }
        return .init(content: [.text(text: text)], isError: false)
    } catch {
        return .init(content: [.text(text: "Error: \(error.localizedDescription)")], isError: true)
    }
}

try await server.start(transport: StdioTransport())
await server.waitUntilCompleted()
