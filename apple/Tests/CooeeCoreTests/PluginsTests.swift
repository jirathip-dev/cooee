import CooeeCore
import Foundation
import Testing

internal struct PluginsTests {
    @Test internal func aPlaceKeepsItsIdAsItsKey() throws {
        let json = #"{"plugin": "drover", "id": "queue", "label": "Queue", "screen": "/screens/queue"}"#
        let place = try JSONDecoder().decode(Place.self, from: Data(json.utf8))
        #expect(place == Place(plugin: "drover", key: "queue", label: "Queue", screen: "/screens/queue"))
        #expect(place.id == "drover/screens/queue")
        let counted = #"{"plugin": "drover", "id": "queue", "label": "Queue", "screen": "/q", "count": 7}"#
        #expect(try JSONDecoder().decode(Place.self, from: Data(counted.utf8)).count == 7)
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
