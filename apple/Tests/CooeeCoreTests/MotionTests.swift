@testable import CooeeCore
import Foundation
import Testing

internal struct MotionTests {
    private let kitchen = Cast(
        coats: ["head", "pastry", "line", "sous"],
        names: ["Head chef", "Pastry chef", "Line cook", "Sous chef"]
    )
    private let ids = ["chief", "coder", "res", "am", "cond", "t1", "t2", "t3"]

    @Test internal func aPersonaKeepsItsSpriteAndPersonasDiffer() {
        let picks = ids.map { Cast.farm.pick($0) }
        #expect(picks == ids.map { Cast.farm.pick($0) })
        #expect(picks.allSatisfy(Cast.farm.coats.contains))
        #expect(Set(picks).count >= 3)
    }

    @Test internal func aThemeSwitchRepicksFromTheNewCastWithTheSameSeed() {
        let picks = ids.map { kitchen.pick($0) }
        #expect(picks.allSatisfy(kitchen.coats.contains))
        #expect(picks == ids.map { kitchen.pick($0) })
        #expect(Set(picks).count >= 2)
    }

    @Test internal func anOverrideWinsInItsOwnThemeOnly() {
        var settings = Settings()
        let auto = Cast.farm.pick("coder")
        let other = Cast.farm.coats.first { $0 != auto } ?? ""
        settings.sprite("coder", other, in: "farm")
        let farm = settings.sprites(in: "farm")
        #expect(Cast.farm.pick("coder", override: farm["coder"]) == other)
        #expect(kitchen.pick("coder", override: settings.sprites(in: "kitchen")["coder"]) == kitchen.pick("coder"))
        #expect(kitchen.pick("coder", override: other) == kitchen.pick("coder"))
        settings.sprite("coder", nil, in: "farm")
        #expect(Cast.farm.pick("coder", override: settings.sprites(in: "farm")["coder"]) == auto)
    }

    @Test internal func motionIsOnByDefaultAndOffWithReduceMotionOrTheSwitch() throws {
        var settings = try JSONDecoder().decode(Settings.self, from: Data(#"{"scheme":"dark","style":"scene"}"#.utf8))
        #expect(settings.moves(reduceMotion: false))
        #expect(!settings.moves(reduceMotion: true))
        settings.motion = false
        #expect(!settings.moves(reduceMotion: false))
    }

    @Test internal func eachRowStateMapsToItsMotion() {
        let states: [LaneState] = [.working, .needs, .done, .merged, .fail, .idle]
        #expect(states.map(Motion.init) == [.walk, .pulse, .hop, .hop, .shake, .still])
        let tones = ["working", "needs", "warn", "ok", "done", "fail", "bad", "mute", "info"]
        #expect(tones.map(Motion.init(tone:)) == [.walk, .pulse, .pulse, .hop, .hop, .shake, .shake, .still, .still])
    }

    @Test internal func doneHopsNeedsPulsesAndTintsFailShakesThenTheyRest() {
        #expect(Motion.hop.pose(at: 0.4) == Pose(yOffset: -4))
        let needs = Motion.pulse.pose(at: 0.4)
        #expect(needs.scale == 1.25 && needs.tint > 0.1 && needs.tint < 0.18)
        #expect(Motion.shake.pose(at: 0.1).xOffset == -2 && Motion.shake.pose(at: 0.3).xOffset == 2)
        for motion in [Motion.hop, .pulse, .shake] {
            let after = motion.pose(at: Motion.moment + 0.1)
            #expect(after == Pose(frame: after.frame))
        }
    }

    @Test internal func theWalkSwapsGaitFramesAndTheLoopsRun() {
        #expect([0.1, 0.5, 0.9].map { Motion.walk.pose(at: $0).frame } == [0, 1, 0])
        #expect([0.1, 0.7, 1.3].map { Motion.still.pose(at: $0).frame } == [0, 1, 0])
        #expect(Motion.loading.pose(at: 0).tint == 0 && Motion.loading.pose(at: 0.6).tint == 1)
        #expect(Motion.spin.pose(at: 0.25).angle == 90)
    }

    @Test internal func terminalFiguresHaveTwoFrames() {
        let states: [LaneState] = [.working, .needs, .done, .idle, .fail, .merged]
        #expect(states.map { $0.figure(frame: 0) } == states.map(\.figure))
        #expect(states.allSatisfy { $0.figure(frame: 1) != $0.figure })
    }
}
