import SwiftUI

internal struct SignInAccount: Identifiable {
    internal enum Status {
        case signedIn(usage: String)
        case signedOut
        case signingIn(command: String, output: [String])
    }

    internal let id: String
    internal let who: String
    internal let folder: String
    internal var isDefault = false
    internal let status: Status
}

internal struct SignInProvider: Identifiable {
    internal let id: String
    internal var accounts: [SignInAccount] = []
    internal var install = ""
}

extension SignInProvider {
    internal static let fixtures: [SignInProvider] = [
        SignInProvider(
            id: "Claude Code",
            accounts: [
                SignInAccount(
                    id: "main", who: "guy@example.com", folder: "~/.claude", isDefault: true,
                    status: .signedIn(usage: "5h 12% · 7d 40% · read 5m")),
                SignInAccount(
                    id: "work", who: "work", folder: "~/.config/cooee/accounts/work",
                    status: .signingIn(
                        command: "claude /login",
                        output: ["Opening the browser to sign in…", "Paste the code from the sign-in page:"])),
            ]),
        SignInProvider(
            id: "Codex",
            accounts: [
                SignInAccount(
                    id: "main", who: "main login", folder: "~/.codex", isDefault: true,
                    status: .signedIn(usage: "5h — · 7d 8% · read 2m")),
                SignInAccount(id: "ops", who: "ops", folder: "~/.config/cooee/accounts/ops", status: .signedOut),
            ]),
        SignInProvider(id: "Hermes", install: "uv tool install hermes-agent"),
    ]
}

public struct SignInsView: View {
    @Environment(\.look) private var look
    private let providers = SignInProvider.fixtures
    private let phone: Bool

    public var body: some View {
        SettingsPage(title: "Sign-ins", phone: phone) {
            ForEach(providers) { provider in
                Group2(header: provider.id) {
                    if provider.accounts.isEmpty {
                        missing(provider)
                    } else {
                        ForEach(provider.accounts) { account(provider, $0) }
                        add
                    }
                }
            }
            Text(look.mono ? "-> where logins live" : "Where logins live").ui(14, .regular, look)
                .foregroundStyle(look.palette.link).padding(.leading, 16)
        }
    }

    private func account(_ provider: SignInProvider, _ account: SignInAccount) -> some View {
        let palette = look.palette
        return Line {
            HStack(spacing: 8) {
                Text(account.who).ui(16, .medium, look).lineLimit(1)
                if account.isDefault { Pill(title: "Default", tint: palette.mute) }
                Spacer(minLength: 6)
                trailing(account.status)
            }
            Text(account.folder).ui(12, .regular, look).foregroundStyle(palette.mute).lineLimit(1)
            detail(account.status)
        }
    }

    @ViewBuilder private func trailing(_ status: SignInAccount.Status) -> some View {
        let palette = look.palette
        switch status {
        case .signedIn: StateWord(text: "Signed in", tint: palette.green)
        case .signedOut: Pill(title: "Sign in…", primary: true)
        case .signingIn: StateWord(text: "Signing in", tint: palette.amber)
        }
    }

    @ViewBuilder private func detail(_ status: SignInAccount.Status) -> some View {
        let palette = look.palette
        switch status {
        case .signedIn(let usage):
            HStack {
                Text(usage).ui(13, .regular, look).foregroundStyle(palette.mute)
                Spacer()
                Pill(title: "Sign out", tint: palette.red)
            }
        case .signedOut: EmptyView()
        case .signingIn(let command, let output): LoginTerminal(command: command, output: output)
        }
    }

    private var add: some View {
        Line(divider: false) {
            HStack(spacing: 8) {
                if !look.mono { Image(systemName: "plus.circle.fill").foregroundStyle(look.palette.link) }
                Text(look.mono ? "+ account" : "Add an account").ui(15, .regular, look)
                    .foregroundStyle(look.palette.link)
            }
        }
    }

    private func missing(_ provider: SignInProvider) -> some View {
        Line(divider: false) {
            HStack {
                Text("Not installed").ui(15, .regular, look).foregroundStyle(look.palette.mute)
                Spacer()
                Pill(title: "Copy install")
            }
            Text(provider.install).ui(12, .regular, look).monospaced().foregroundStyle(look.palette.mute)
        }
    }

    public init(phone: Bool = true) {
        self.phone = phone
    }
}

internal struct LoginTerminal: View {
    @Environment(\.look) private var look
    internal let command: String
    internal let output: [String]

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 8) {
            Text("\(command) · shed terminal").ui(12, .medium, look).foregroundStyle(palette.mute)
            VStack(alignment: .leading, spacing: 3) {
                ForEach(output, id: \.self) { Text($0) }
            }
            .font(.system(size: 12, design: .monospaced)).foregroundStyle(palette.foreground)
            .frame(maxWidth: .infinity, alignment: .leading).padding(10)
            .background(Color.black.opacity(palette.dark ? 0.35 : 0.08), in: .rect(cornerRadius: look.mono ? 0 : 8))
            HStack(spacing: 8) {
                Text("Paste the code").ui(14, .regular, look).foregroundStyle(palette.mute)
                    .padding(.horizontal, 10).frame(maxWidth: .infinity, minHeight: 32, alignment: .leading)
                    .overlay(
                        RoundedRectangle(cornerRadius: look.mono ? 0 : 8).strokeBorder(palette.edge, lineWidth: 1))
                Pill(title: "Send", primary: true)
                Pill(title: "Cancel", tint: palette.mute)
            }
        }
        .padding(.top, 4)
    }
}
