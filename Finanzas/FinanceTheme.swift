import AppKit
import SwiftUI

/// Semantic colors shared by every screen. Signs and labels still carry financial meaning.
enum FinanceTheme {
    static let petroleum = adaptive(light: 0x155E75, dark: 0x67C6DF)
    static let emerald = adaptive(light: 0x147D59, dark: 0x6DDBAD)
    static let coral = adaptive(light: 0xBE493D, dark: 0xFF9A8F)
    static let amber = adaptive(light: 0x986000, dark: 0xF5C46B)
    static let canvas = adaptive(light: 0xF4F7FA, dark: 0x151D27)
    static let card = adaptive(light: 0xFFFFFF, dark: 0x1E2936)
    static let border = adaptive(light: 0xDEE7ED, dark: 0x354453)
    static let hero = Color(red: 21 / 255, green: 94 / 255, blue: 117 / 255)

    private static func adaptive(light: UInt32, dark: UInt32) -> Color {
        Color(
            nsColor: NSColor(name: nil) { appearance in
                let hex = appearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua ? dark : light
                return NSColor(
                    srgbRed: CGFloat((hex >> 16) & 0xFF) / 255,
                    green: CGFloat((hex >> 8) & 0xFF) / 255,
                    blue: CGFloat(hex & 0xFF) / 255, alpha: 1)
            })
    }
}

extension View {
    func financeCard() -> some View {
        background(FinanceTheme.card, in: RoundedRectangle(cornerRadius: 18))
            .overlay {
                RoundedRectangle(cornerRadius: 18)
                    .strokeBorder(FinanceTheme.border, lineWidth: 1)
                    .allowsHitTesting(false)
            }
    }
}

struct FinanceHeading: View {
    let title: String
    let subtitle: String
    let icon: String

    var body: some View {
        HStack(alignment: .center, spacing: 16) {
            VStack(alignment: .leading, spacing: 6) {
                Text(title).font(.largeTitle.bold()).accessibilityAddTraits(.isHeader)
                Text(subtitle).font(.callout).foregroundStyle(.secondary)
            }
            Spacer(minLength: 8)
            Image(systemName: icon)
                .font(.title2)
                .foregroundStyle(FinanceTheme.petroleum)
                .padding(12)
                .background(FinanceTheme.petroleum.opacity(0.1), in: RoundedRectangle(cornerRadius: 14))
                .accessibilityHidden(true)
        }
    }
}

struct FinanceMetric: View {
    let title: String
    let value: String
    let icon: String
    var color = FinanceTheme.petroleum

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Label(title, systemImage: icon)
                .font(.subheadline.weight(.medium)).foregroundStyle(.secondary)
            Text(value)
                .font(.system(.title, design: .rounded, weight: .bold))
                .monospacedDigit().foregroundStyle(color)
                .lineLimit(1).minimumScaleFactor(0.6)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(20)
        .financeCard()
    }
}

struct FinancePanelStyle: GroupBoxStyle {
    func makeBody(configuration: Configuration) -> some View {
        VStack(alignment: .leading, spacing: 18) {
            configuration.label.foregroundStyle(.primary)
            configuration.content.frame(maxWidth: .infinity, alignment: .leading)
        }
        .padding(20)
        .financeCard()
    }
}
