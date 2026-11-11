import CooeeCore
import Foundation
import Testing

internal struct PluginsTests {
    @Test internal func aPluginRowSaysWhatItMayDoInPlainWords() throws {
        let json = """
            {"name": "drover", "version": "0.4.0", "commit": "b3f8ce2e61", "url": "https://x/d", "listed": false,
             "state": "running", "places": [{"type": "header-strip", "id": "queue", "persona": "conductor",
             "label": "Queue", "screen": "/screens/queue"}, {"type": "chat-cards", "id": "", "persona": "conductor",
             "label": "", "screen": ""}, {"type": "menu-row", "id": "", "persona": "", "label": "drover",
             "screen": "/screens/fleets"}], "claims": ["drover"], "permissions": {"places": ["header-strip"], "push": true,
             "actions": ["drive", "drop"], "destructive": ["drop"], "network": []}}
            """
        let row = try JSONDecoder().decode(PluginRow.self, from: Data(json.utf8))
        #expect(row.id == "drover" && row.source == "https://x/d")
        #expect(
            row.may == [
                "Queue chip on the header strip", "cards in its chats", "drover row in Plugins",
                "sessions labelled source=drover in its Workflow sheet", "notifications",
            ]
        )
        #expect(row.permissions.destructive == ["drop"])
        let listed = PluginRow(name: "hello", version: "1", commit: "c", places: [], permissions: PluginPermissions())
        #expect(listed.source == "cooee's index" && listed.may.isEmpty)
    }

    @Test internal func aPlaceKeepsItsIdAsItsKey() throws {
        let json = #"{"plugin": "drover", "id": "queue", "label": "Queue", "screen": "/screens/queue"}"#
        let place = try JSONDecoder().decode(Place.self, from: Data(json.utf8))
        #expect(place == Place(plugin: "drover", key: "queue", label: "Queue", screen: "/screens/queue"))
        #expect(place.id == "drover/screens/queue")
    }

    @Test internal func theOfflineLineSaysWhoAndHowOld() {
        let now = Date(timeIntervalSince1970: 10_000)
        #expect(
            Offline.line(who: "Mac unreachable", title: "drover's Queue", asOf: now.addingTimeInterval(-300), now: now)
                == "Mac unreachable · drover's Queue as of 5m ago · actions are off until it's back"
        )
        #expect(
            Offline.line(who: "drover isn't answering", title: "Queue", asOf: now, now: now)
                == "drover isn't answering · Queue as of now · actions are off until it's back"
        )
        #expect(
            Offline.line(who: "Mac unreachable", title: "Queue", asOf: nil, now: now)
                == "Mac unreachable · actions are off until it's back"
        )
    }
}
