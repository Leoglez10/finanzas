// Prints the window number of the first on-screen window owned by a process named like argv[1].
import CoreGraphics
import Foundation
let owner = CommandLine.arguments[1].lowercased()
let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]] ?? []
for w in list {
    let name = (w[kCGWindowOwnerName as String] as? String ?? "").lowercased()
    let layer = w[kCGWindowLayer as String] as? Int ?? 1
    if name.contains(owner) && layer == 0, let id = w[kCGWindowNumber as String] as? Int { print(id); exit(0) }
}
exit(1)
