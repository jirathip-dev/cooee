import CooeeCore
@testable import CooeeUI
import SwiftUI

public struct SignInSheet: View {
    @Environment(\.look) private var look

    public var body: some View {
        ZStack(alignment: .top) {
            EntryPage(connector: Story.notionSigningIn)
            Dim()
            WebSheet().padding(.top, 10)
        }
    }

    public init() {}
}

internal struct WebSheet: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let dark = look.palette.dark
        let chrome = dark ? Color(white: 0.11) : Color(white: 0.97)
        VStack(spacing: 0) {
            HStack {
                Text("Cancel").font(.system(size: 17)).foregroundStyle(Color.blue)
                Spacer()
                HStack(spacing: 4) {
                    Image(systemName: "lock.fill").font(.system(size: 11))
                    Text("notion.com").font(.system(size: 15, weight: .semibold))
                }
                .foregroundStyle(dark ? Color.white : .black)
                Spacer()
                Image(systemName: "safari").font(.system(size: 17)).foregroundStyle(Color.blue)
            }
            .padding(.horizontal, 16).frame(height: 52).background(chrome)
            ServicePage().frame(maxHeight: .infinity, alignment: .top)
        }
        .background(Color.white)
        .clipShape(UnevenRoundedRectangle(topLeadingRadius: 12, topTrailingRadius: 12))
        .environment(\.colorScheme, .light)
    }
}

internal struct ServicePage: View {
    internal var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            Text("Notion").font(.system(size: 26, weight: .bold)).padding(.top, 28)
            Text("Cooee wants to access your workspace").font(.system(size: 21, weight: .semibold))
            VStack(alignment: .leading, spacing: 10) {
                grant("doc.text.magnifyingglass", "Read pages and databases you share with it")
                grant("pencil", "Create and edit pages")
                grant("person.2", "See the names of people in the workspace")
            }
            HStack {
                Text("Guy's workspace").font(.system(size: 15))
                Spacer()
                Image(systemName: "chevron.down").font(.system(size: 12, weight: .semibold))
            }
            .padding(.horizontal, 14).frame(height: 44)
            .overlay(RoundedRectangle(cornerRadius: 8).strokeBorder(Color(white: 0.85)))
            Text("Allow access").font(.system(size: 16, weight: .semibold)).foregroundStyle(.white)
                .frame(maxWidth: .infinity).frame(height: 46).background(Color.black, in: .rect(cornerRadius: 8))
            Text("Redirects to mini.tail3f2a.ts.net").font(.system(size: 12)).foregroundStyle(Color(white: 0.45))
                .frame(maxWidth: .infinity)
        }
        .foregroundStyle(Color(white: 0.1)).padding(.horizontal, 24)
    }

    private func grant(_ symbol: String, _ text: String) -> some View {
        HStack(alignment: .top, spacing: 10) {
            Image(systemName: symbol).font(.system(size: 14)).frame(width: 20).foregroundStyle(Color(white: 0.35))
            Text(text).font(.system(size: 15))
        }
    }
}

public struct SignedIn: View {
    @Environment(\.look) private var look

    public var body: some View {
        let palette = look.palette
        ZStack(alignment: .bottom) {
            EntryPage(connector: Story.notionSignedIn)
            Dim()
            VStack(alignment: .leading, spacing: 14) {
                HStack(spacing: 8) {
                    Glyph(symbol: "checkmark.circle.fill", ascii: "[x]", size: 18).foregroundStyle(palette.green)
                    Text("Notion is connected as main").ui(17, .semibold, look)
                }
                Text("Attach it to a persona").ui(13, .medium, look).foregroundStyle(palette.mute)
                HStack(alignment: .top, spacing: 14) {
                    ForEach(Fixtures.personas.prefix(5)) { persona in
                        VStack(spacing: 6) {
                            FigureView(persona: persona, size: 52)
                            Text(persona.api.name).ui(11, .regular, look).lineLimit(1)
                        }
                        .frame(width: 58)
                    }
                }
                Text(look.mono ? "[not now]" : "Not now").ui(15, .regular, look).foregroundStyle(palette.link)
                    .frame(maxWidth: .infinity)
            }
            .padding(20).padding(.bottom, 24).frame(maxWidth: .infinity, alignment: .leading)
            .background(sheetFill, in: .rect(cornerRadius: look.mono ? 0 : 28))
            .overlay { if look.mono { Rectangle().strokeBorder(palette.edge, lineWidth: 1) } }
        }
    }

    private var sheetFill: Color { look.mono ? look.palette.background : look.palette.card }

    public init() {}
}

public struct TokenSheet: View {
    @Environment(\.look) private var look

    public var body: some View {
        let palette = look.palette
        ZStack(alignment: .bottom) {
            EntryPage(connector: Story.github)
            Dim()
            VStack(alignment: .leading, spacing: 14) {
                HStack {
                    Text(look.mono ? "# sign in to github · main" : "Sign in to GitHub · main").ui(17, .semibold, look)
                    Spacer()
                    RoundButton(symbol: "xmark", ascii: "x", size: 32)
                }
                Text("Make a fine-grained token at github.com › Settings › Developer settings, with read access to the repositories your personas use.")
                    .ui(13, .regular, look).foregroundStyle(palette.mute).fixedSize(horizontal: false, vertical: true)
                HStack {
                    Text(String(repeating: "•", count: 22)).ui(17, .regular, look)
                    Spacer()
                    ActionPill(title: "Paste")
                }
                .padding(.horizontal, 14).frame(height: 50)
                .background(palette.background.opacity(0.6), in: .rect(cornerRadius: look.mono ? 0 : 12))
                .overlay { RoundedRectangle(cornerRadius: look.mono ? 0 : 12).strokeBorder(palette.link, lineWidth: 1.5) }
                HStack(alignment: .top, spacing: 8) {
                    Glyph(symbol: "lock.fill", ascii: "*", size: 12).foregroundStyle(palette.green)
                    Text("Goes straight to your Mac's keychain. No file, chat or log keeps it.")
                        .ui(12, .regular, look).foregroundStyle(palette.mute)
                }
                Primary(title: "Save")
            }
            .padding(20).padding(.bottom, 24)
            .background(look.mono ? palette.background : palette.card, in: .rect(cornerRadius: look.mono ? 0 : 28))
            .overlay { if look.mono { Rectangle().strokeBorder(palette.edge, lineWidth: 1) } }
        }
    }

    public init() {}
}
