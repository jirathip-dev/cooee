import Testing

@testable import AntiSlopCore

struct CooeeRuleTests {
    private func ids(_ source: String, _ rule: SlopRule.Type, file: String = "apple/Sources/CooeeCore/A.swift") -> [Int] {
        AntiSlop.lint(source: source, fileName: file, rules: [rule]).map(\.line).sorted()
    }

    @Test func commentsAreRejected() {
        #expect(ids("// what this does\nlet a = 1\n", NoCommentsRule.self) == [1])
        #expect(ids("let a = 1 // trailing\n", NoCommentsRule.self) == [1])
        #expect(ids("/* block */\nlet a = 1\n", NoCommentsRule.self) == [1])
        #expect(ids("/// internal doc\nstruct A {}\n", NoCommentsRule.self) == [1])
        #expect(ids("public struct A {\n    /// internal doc\n    var b = 1\n}\n", NoCommentsRule.self) == [2])
    }

    @Test func allowedComments() {
        let source = """
            // swift-tools-version: 6.2
            /// The server.
            public struct A {
                /// Its address.
                public var url = ""
                /// Opened.
                @MainActor open func go() {}
            }
            /// A state.
            public enum B {
                /// Working.
                case working
            }
            // TODO(#147) the next step
            let c = 1
            // SAFETY: the literal is a valid URL.
            let d = URL(string: "https://a")!
            // SAFETY: the literal is valid JSON.
            let e = try! decode()
            """
        #expect(ids(source, NoCommentsRule.self) == [])
    }

    @Test func safetyMustSitAboveAForcedOperation() {
        #expect(ids("// SAFETY: nothing forced\nlet a = 1\n", NoCommentsRule.self) == [1])
        #expect(ids("// SAFETY: too far\n\nlet a = b!\n", NoCommentsRule.self) == [1])
        #expect(ids("// TODO: no issue\nlet a = 1\n", NoCommentsRule.self) == [1])
    }

    @Test func importsAreSorted() {
        #expect(ids("import SwiftUI\nimport Foundation\n", SortedImportsRule.self) == [2])
        #expect(ids("import CooeeCore\n@testable import CooeeUI\nimport Testing\n", SortedImportsRule.self) == [])
        #expect(ids("#if os(iOS)\nimport UIKit\nimport SwiftUI\n#endif\n", SortedImportsRule.self) == [3])
    }

    @Test func layerImports() {
        let view = "apple/Sources/CooeeUI/HomeView.swift"
        #expect(ids("import CooeeAPI\nimport SwiftUI\n", LayerImportsRule.self, file: view) == [1])
        #expect(ids("import CooeeFeatures\nimport SwiftUI\n", LayerImportsRule.self, file: view) == [])
        #expect(ids("import SwiftUI\n", LayerImportsRule.self) == [1])
        #expect(ids("import Foundation\n", LayerImportsRule.self) == [])
        let test = "apple/Tests/CooeeAPITests/ClientTests.swift"
        #expect(ids("import Testing\n@testable import CooeeAPI\n", LayerImportsRule.self, file: test) == [])
        #expect(ids("import CooeeFeatures\n", LayerImportsRule.self, file: test) == [1])
        #expect(ids("let a = 1\n", LayerImportsRule.self, file: "apple/Other/A.swift") == [1])
    }

    @Test func moduleOfAPath() {
        #expect(LayerImportsRule.module(of: "apple/Sources/CooeeAPI/Client.swift") == "CooeeAPI")
        #expect(LayerImportsRule.module(of: "apple/Tests/CooeeUITests/Shots.swift") == "CooeeUITests")
        #expect(LayerImportsRule.module(of: "apple/Cooee/CooeeApp.swift") == "Cooee")
        #expect(LayerImportsRule.module(of: "apple/UITests/LiveUITests.swift") == "UITests")
        #expect(LayerImportsRule.module(of: "apple/Package.swift") == nil)
    }

    @Test func anyIsRejectedAsAType() {
        #expect(ids("let a: Any = 1\n", NoAnyRule.self) == [1])
        #expect(ids("let a: [String: Any] = [:]\n", NoAnyRule.self) == [1])
        #expect(ids("protocol P: AnyObject {}\n", NoAnyRule.self) == [1])
        #expect(ids("func f(_ a: Swift.AnyObject) {}\n", NoAnyRule.self) == [1])
        #expect(ids("let a: AnyShape = Circle()\nlet b: any Error = E()\n", NoAnyRule.self) == [])
    }

    @Test func accessIsExplicit() {
        let source = """
            struct A {
                var b = 1
                private(set) var c = 2
                public init() {}
                func d() { let local = 1 }
            }
            public enum E { case x }
            extension A { func f() {} }
            protocol P { func g() }
            internal let h = 1
            """
        #expect(ids(source, ExplicitAccessRule.self) == [1, 2, 3, 5, 8, 9])
    }
}
