import CooeeCore
import Testing

internal struct MentionTests {
    private let members = [
        APIPersona(id: "chief", name: "Chief", agent: ""),
        APIPersona(id: "coder", name: "Coder", agent: ""),
        APIPersona(id: "res", name: "Researcher", agent: ""),
        APIPersona(id: "am", name: "Account Manager", agent: ""),
    ]

    @Test internal func membersReplyInMentionOrderOnceEachByNameOrId() {
        let text = "@Researcher then @coder, @res again, @AccountManager; mail me@chief.com and @nobody"
        #expect(Mentions.named(text, members) == ["res", "coder", "am"])
        #expect(Mentions.named("no one named", members).isEmpty)
    }

    @Test internal func theWordBeingTypedPicksTheFirstMemberNotYetNamed() {
        let draft = MentionDraft("@Coder check Q3 with @Re", members: members)
        #expect(draft.named == ["coder"] && draft.query == "re" && draft.match == "res")
        #expect(draft.order == ["coder", "res"])
        let done = MentionDraft("@Coder check Q3 ", members: members)
        #expect(done.query == nil && done.match == nil && done.order == ["coder"])
        let bare = MentionDraft("ask @", members: members)
        #expect(bare.query == "" && bare.match == nil)
        #expect(MentionDraft("@Co", members: members).match == "coder")
    }

    @Test internal func pickingAMemberReplacesTheWordBeingTypedWithItsToken() {
        #expect(Mentions.pick("Researcher", in: "@Coder check Q3 with @Re") == "@Coder check Q3 with @Researcher ")
        #expect(Mentions.pick("Account Manager", in: "ask ") == "ask @AccountManager ")
        let picked = Mentions.pick("Account Manager", in: "@Acc")
        #expect(Mentions.named(picked, members) == ["am"])
    }

    @Test internal func theLineSaysWhoRepliesInOrder() {
        #expect(Mentions.line([]) == nil)
        let two = String((Mentions.line(["Coder", "Researcher"]) ?? "").characters)
        #expect(two == "Coder, then Researcher reply, and anyone they @mention")
        let one = Mentions.line(["Coder"]) ?? ""
        #expect(String(one.characters) == "Coder replies, and anyone they @mention")
        let bold = one.runs.filter { $0.inlinePresentationIntent == .stronglyEmphasized }
        #expect(bold.map { String(one[$0.range].characters) } == ["Coder"])
    }

    @Test internal func aDraftNamingMembersQueuesThemUpNextAndWaitsTheOthers() {
        var round = RoomRound()
        let ids = members.map(\.id)
        round.queue(ids, named: MentionDraft("@Coder check Q3 with @Re", members: members).order)
        #expect(ids.map(round.state) == [.waiting, .upNext, .upNext, .waiting])
        round.queue(ids, named: [])
        #expect(ids.map(round.state) == [.upNext, .upNext, .upNext, .upNext])
    }
}
