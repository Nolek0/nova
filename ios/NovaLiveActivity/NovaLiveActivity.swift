// Dynamic Island et écran verrouillé pendant que l'iPhone contrôle le Mac.

import ActivityKit
import SwiftUI
import WidgetKit

@main
struct NovaLiveActivityBundle: WidgetBundle {
    var body: some Widget {
        ControlLiveActivity()
    }
}

private let cyan = Color(red: 0.13, green: 0.83, blue: 1.0)
private let violet = Color(red: 0.60, green: 0.48, blue: 1.0)

struct ControlLiveActivity: Widget {
    var body: some WidgetConfiguration {
        ActivityConfiguration(for: ControlActivityAttributes.self) { context in
            // Écran verrouillé et bannière.
            HStack(spacing: 14) {
                Pulse()
                VStack(alignment: .leading, spacing: 2) {
                    Text("Vous contrôlez le Mac").font(.headline)
                    Text(context.attributes.computer).font(.subheadline).foregroundStyle(.secondary)
                }
                Spacer()
                Text(context.attributes.started, style: .timer)
                    .font(.title3.monospacedDigit().weight(.semibold))
                    .foregroundStyle(cyan)
                    .frame(maxWidth: 80, alignment: .trailing)
            }
            .padding(18)
            .activityBackgroundTint(Color.black.opacity(0.85))
            .activitySystemActionForegroundColor(cyan)
        } dynamicIsland: { context in
            DynamicIsland {
                DynamicIslandExpandedRegion(.leading) {
                    Image(systemName: "macbook.and.iphone")
                        .font(.title2)
                        .foregroundStyle(LinearGradient(colors: [cyan, violet], startPoint: .topLeading, endPoint: .bottomTrailing))
                        .padding(.leading, 6)
                }
                DynamicIslandExpandedRegion(.trailing) {
                    Text(context.attributes.started, style: .timer)
                        .font(.headline.monospacedDigit())
                        .foregroundStyle(cyan)
                        .frame(maxWidth: 70, alignment: .trailing)
                }
                DynamicIslandExpandedRegion(.center) {
                    Text("Vous contrôlez le Mac").font(.headline)
                }
                DynamicIslandExpandedRegion(.bottom) {
                    Text("\(context.attributes.computer) · \(context.state.detail)")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            } compactLeading: {
                Image(systemName: "macbook").foregroundStyle(cyan)
            } compactTrailing: {
                Text("Mac").font(.caption.weight(.semibold)).foregroundStyle(cyan)
            } minimal: {
                Image(systemName: "macbook").foregroundStyle(cyan)
            }
            .keylineTint(cyan)
        }
    }
}

private struct Pulse: View {
    var body: some View {
        ZStack {
            Circle().fill(cyan.opacity(0.25)).frame(width: 44, height: 44)
            Image(systemName: "macbook.and.iphone").font(.title3).foregroundStyle(cyan)
        }
    }
}
