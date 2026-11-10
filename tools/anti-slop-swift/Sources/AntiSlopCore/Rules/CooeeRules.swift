import Foundation
import SwiftSyntax

/// cooee: no comments in Swift code, except `///` on a `public` or `open`
/// declaration, a one-line `// SAFETY:` directly above a force unwrap or
/// `try!`, and `// TODO(#N)` (CI checks that #N is an open issue).
public final class NoCommentsRule: SlopRule {
    override public class var id: String { "no-comments" }
    override public class var summary: String {
        "Rejects comments except /// on public or open API, // SAFETY: above a force unwrap or try!, and // TODO(#N)."
    }

    private var forcedLines: Set<Int> = []
    private var pending: [(position: AbsolutePosition, line: Int, text: String)] = []

    public required init(
        fileName: String,
        sourceText: String,
        converter: SourceLocationConverter
    ) {
        super.init(fileName: fileName, sourceText: sourceText, converter: converter)
    }

    public override func visitPost(_ node: ForceUnwrapExprSyntax) {
        forcedLines.insert(line(of: node.exclamationMark))
    }

    public override func visitPost(_ node: TryExprSyntax) {
        guard node.questionOrExclamationMark?.tokenKind == .exclamationMark else { return }
        forcedLines.insert(line(of: node))
    }

    public override func visit(_ token: TokenSyntax) -> SyntaxVisitorContinueKind {
        check(token.leadingTrivia, from: token.position, token: token, leading: true)
        check(token.trailingTrivia, from: token.endPositionBeforeTrailingTrivia, token: token, leading: false)
        return .visitChildren
    }

    public override func visitPost(_ node: SourceFileSyntax) {
        for comment in pending where !forcedLines.contains(comment.line + 1) {
            report(
                at: comment.position,
                message: "A // SAFETY: comment must sit on the line directly above a force unwrap or try!."
            )
        }
    }

    private func check(_ trivia: Trivia, from start: AbsolutePosition, token: TokenSyntax, leading: Bool) {
        var position = start
        for piece in trivia {
            verdict(piece, at: position, token: token, leading: leading)
            position = position.advanced(by: piece.sourceLength.utf8Length)
        }
    }

    private func verdict(_ piece: TriviaPiece, at position: AbsolutePosition, token: TokenSyntax, leading: Bool) {
        switch piece {
        case .lineComment(let text):
            lineComment(text, at: position, leading: leading)
        case .docLineComment:
            guard !(leading && Self.documentsPublicAPI(token)) else { return }
            report(at: position, message: "A /// doc comment is allowed only on a public or open declaration.")
        case .blockComment, .docBlockComment:
            report(at: position, message: "Block comments are not allowed; say it in names, types and tests.")
        default:
            return
        }
    }

    private func lineComment(_ text: String, at position: AbsolutePosition, leading: Bool) {
        if text.range(of: #"^// TODO\(#[0-9]+\)"#, options: .regularExpression) != nil { return }
        if position.utf8Offset == 0 && text.hasPrefix("// swift-tools-version:") { return }
        if leading && text.hasPrefix("// SAFETY: ") {
            let line = line(at: position)
            pending.append((position, line, text))
            return
        }
        report(
            at: position,
            message: "Comments are not allowed: say it in names, types and tests (only ///, // SAFETY: and // TODO(#N))."
        )
    }

    static func documentsPublicAPI(_ token: TokenSyntax) -> Bool {
        var node = token.parent
        while let current = node {
            if let decl = current.asProtocol(DeclSyntaxProtocol.self) {
                guard current.firstToken(viewMode: .sourceAccurate) == token else { return false }
                return isPublic(Syntax(decl))
            }
            node = current.parent
        }
        return false
    }

    static func isPublic(_ decl: Syntax) -> Bool {
        if let modifiers = decl.asProtocol(WithModifiersSyntax.self)?.modifiers,
            modifiers.contains(where: { ["public", "open"].contains($0.name.text) && $0.detail == nil })
        {
            return true
        }
        guard decl.is(EnumCaseDeclSyntax.self) || enclosingType(of: decl)?.is(ProtocolDeclSyntax.self) == true,
            let parent = enclosingType(of: decl)
        else { return false }
        return isPublic(parent)
    }

    static func enclosingType(of decl: Syntax) -> Syntax? {
        var node = decl.parent
        while let current = node {
            if current.is(MemberBlockSyntax.self) { return current.parent }
            if current.is(CodeBlockSyntax.self) || current.is(SourceFileSyntax.self) { return nil }
            node = current.parent
        }
        return nil
    }
}

/// cooee: the imports of a file (and of each `#if` clause) are sorted by
/// module name.
public final class SortedImportsRule: SlopRule {
    override public class var id: String { "sorted-imports" }
    override public class var summary: String { "Requires each file's imports to be sorted by module name." }

    public required init(
        fileName: String,
        sourceText: String,
        converter: SourceLocationConverter
    ) {
        super.init(fileName: fileName, sourceText: sourceText, converter: converter)
    }

    public override func visitPost(_ node: CodeBlockItemListSyntax) {
        let imports = node.compactMap { $0.item.as(ImportDeclSyntax.self) }
        for (previous, next) in zip(imports, imports.dropFirst())
        where Self.name(of: next) < Self.name(of: previous) {
            report(
                next,
                message: "`import \(Self.name(of: next))` comes after `import \(Self.name(of: previous))`: sort the imports."
            )
        }
    }

    static func name(of node: ImportDeclSyntax) -> String {
        node.path.map(\.name.text).joined(separator: ".")
    }
}

/// cooee: each module imports only what its layer allows. Core (models,
/// Foundation only) → API (client, SSE) → Features (TCA reducers) → UI
/// (SwiftUI views, no networking) → App (wiring only). The table mirrors
/// apple/Package.swift's target dependencies.
public final class LayerImportsRule: SlopRule {
    override public class var id: String { "layer-imports" }
    override public class var summary: String {
        "Requires each cooee module to import only what its layer allows (apple/Package.swift's table)."
    }

    public static let layers: [String: Set<String>] = [
        "CooeeCore": ["Foundation"],
        "CooeeAPI": ["CooeeCore", "Foundation", "Network", "Security"],
        "CooeeFeatures": ["ComposableArchitecture", "CooeeAPI", "CooeeCore", "Foundation"],
        "CooeeUI": [
            "AppKit", "ComposableArchitecture", "CooeeCore", "CooeeFeatures", "CoreImage", "SwiftUI", "UIKit",
            "UniformTypeIdentifiers", "VisionKit",
        ],
        "CooeeApp": ["ComposableArchitecture", "CooeeAPI", "CooeeCore", "CooeeFeatures", "CooeeUI", "SwiftUI"],
        "Cooee": ["CooeeApp", "CooeeUI", "SwiftUI"],
        "UITests": ["XCTest"],
    ]

    static let testImports: Set<String> = ["Foundation", "Testing", "XCTest"]

    public required init(
        fileName: String,
        sourceText: String,
        converter: SourceLocationConverter
    ) {
        super.init(fileName: fileName, sourceText: sourceText, converter: converter)
    }

    public override func visitPost(_ node: SourceFileSyntax) {
        guard Self.module(of: fileName) == nil else { return }
        report(node, message: "This file is in no cooee layer (Sources/<module>, Tests/<module>Tests, Cooee, UITests).")
    }

    public override func visitPost(_ node: ImportDeclSyntax) {
        guard let module = Self.module(of: fileName), let allowed = Self.allowed(in: module) else { return }
        let imported = node.path.first?.name.text ?? ""
        guard !allowed.contains(imported) else { return }
        report(
            node.path,
            message: "\(module) may not import \(imported); its layer allows \(allowed.sorted().joined(separator: ", "))."
        )
    }

    public static func allowed(in module: String) -> Set<String>? {
        if let own = layers[module] { return own }
        guard module.hasSuffix("Tests") else { return nil }
        let tested = String(module.dropLast("Tests".count))
        return layers[tested].map { $0.union(testImports).union([tested]) }
    }

    public static func module(of path: String) -> String? {
        let parts = path.split(separator: "/").map(String.init)
        for (index, part) in parts.enumerated().dropLast() {
            if part == "Sources" || part == "Tests" {
                return index + 2 < parts.count ? parts[index + 1] : nil
            }
            if part == "Cooee" || part == "UITests" { return part }
        }
        return nil
    }
}

/// cooee: `Any` and `AnyObject` are not allowed as a type, anywhere.
public final class NoAnyRule: SlopRule {
    override public class var id: String { "no-any" }
    override public class var summary: String { "Rejects `Any` and `AnyObject` used as a type." }

    public required init(
        fileName: String,
        sourceText: String,
        converter: SourceLocationConverter
    ) {
        super.init(fileName: fileName, sourceText: sourceText, converter: converter)
    }

    public override func visitPost(_ node: IdentifierTypeSyntax) {
        guard node.name.text == "Any" || node.name.text == "AnyObject" else { return }
        report(node, message: "`\(node.name.text)` throws the type away: name the real type, or introduce one.")
    }

    public override func visitPost(_ node: MemberTypeSyntax) {
        guard node.name.text == "Any" || node.name.text == "AnyObject" else { return }
        report(node, message: "`\(node.trimmedDescription)` throws the type away: name the real type, or introduce one.")
    }
}

/// cooee: every top-level and member declaration states its access level.
/// Extensions (swift format forbids access on them), enum cases, protocol
/// requirements, deinitializers and local declarations are exempt.
public final class ExplicitAccessRule: SlopRule {
    override public class var id: String { "explicit-access" }
    override public class var summary: String {
        "Requires every top-level and member declaration to state its access level."
    }

    static let levels: Set<String> = ["private", "fileprivate", "internal", "package", "public", "open"]

    public required init(
        fileName: String,
        sourceText: String,
        converter: SourceLocationConverter
    ) {
        super.init(fileName: fileName, sourceText: sourceText, converter: converter)
    }

    public override func visitPost(_ node: StructDeclSyntax) { check(node, node.structKeyword) }
    public override func visitPost(_ node: ClassDeclSyntax) { check(node, node.classKeyword) }
    public override func visitPost(_ node: EnumDeclSyntax) { check(node, node.enumKeyword) }
    public override func visitPost(_ node: ActorDeclSyntax) { check(node, node.actorKeyword) }
    public override func visitPost(_ node: ProtocolDeclSyntax) { check(node, node.protocolKeyword) }
    public override func visitPost(_ node: FunctionDeclSyntax) { check(node, node.funcKeyword) }
    public override func visitPost(_ node: VariableDeclSyntax) { check(node, node.bindingSpecifier) }
    public override func visitPost(_ node: InitializerDeclSyntax) { check(node, node.initKeyword) }
    public override func visitPost(_ node: SubscriptDeclSyntax) { check(node, node.subscriptKeyword) }
    public override func visitPost(_ node: TypeAliasDeclSyntax) { check(node, node.typealiasKeyword) }

    private func check(_ node: some WithModifiersSyntax, _ keyword: TokenSyntax) {
        guard Self.needsAccess(Syntax(node)) else { return }
        guard !node.modifiers.contains(where: { Self.levels.contains($0.name.text) && $0.detail == nil }) else { return }
        report(keyword, message: "State this declaration's access level (private, internal, public, …).")
    }

    static func needsAccess(_ decl: Syntax) -> Bool {
        var node = decl.parent
        while let current = node {
            if current.is(SourceFileSyntax.self) { return true }
            if current.is(MemberBlockSyntax.self) { return !(current.parent?.is(ProtocolDeclSyntax.self) ?? false) }
            if current.is(CodeBlockSyntax.self) || current.is(AccessorBlockSyntax.self)
                || current.is(ClosureExprSyntax.self)
            {
                return false
            }
            node = current.parent
        }
        return false
    }
}
