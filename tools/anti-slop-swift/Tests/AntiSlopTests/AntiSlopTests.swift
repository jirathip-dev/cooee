import XCTest

@testable import AntiSlopCore

final class AntiSlopTests: XCTestCase {

    private func lint(_ source: String, _ ruleType: SlopRule.Type) -> [Violation] {
        AntiSlop.lint(source: source, fileName: "test.swift", rules: [ruleType])
    }

    private func assertViolation(
        _ source: String,
        _ ruleType: SlopRule.Type,
        line expectedLine: Int? = nil,
        file: StaticString = #filePath,
        line: UInt = #line
    ) {
        let violations = lint(source, ruleType)
        XCTAssertFalse(violations.isEmpty, "expected a violation", file: file, line: line)
        XCTAssertEqual(
            violations.first?.ruleID,
            ruleType.id,
            file: file,
            line: line
        )
        if let expectedLine {
            XCTAssertEqual(violations.first?.line, expectedLine, file: file, line: line)
        }
    }

    private func assertClean(
        _ source: String,
        _ ruleType: SlopRule.Type,
        file: StaticString = #filePath,
        line: UInt = #line
    ) {
        let violations = lint(source, ruleType)
        XCTAssertTrue(
            violations.isEmpty,
            "expected no violations, got \(violations)",
            file: file,
            line: line
        )
    }

    func testChainedForcedCastIsRejected() {
        assertViolation("let x = value as! Foo as! Bar", NoChainedTypeCastsRule.self)
    }

    func testChainedConditionalCastIsRejected() {
        assertViolation("let x = value as? Foo as? Bar", NoChainedTypeCastsRule.self)
    }

    func testSingleCastIsClean() {
        assertClean("let x = value as! Foo", NoChainedTypeCastsRule.self)
        assertClean("let x = value as? Foo", NoChainedTypeCastsRule.self)
    }

    func testForcedCastWithoutSafetyCommentIsRejected() {
        assertViolation(
            "let x = value as! Foo",
            RequireSafetyCommentForForcedCastRule.self
        )
    }

    func testForcedCastWithSafetyCommentIsClean() {
        assertClean(
            """
            // SAFETY: parseUser validated this payload above.
            let x = value as! Foo
            """,
            RequireSafetyCommentForForcedCastRule.self
        )
    }

    func testSafetyCommentBeyondBlankLineDoesNotCount() {
        assertViolation(
            """
            // SAFETY: too far away.

            let x = value as! Foo
            """,
            RequireSafetyCommentForForcedCastRule.self
        )
    }

    func testForceTryWithoutSafetyCommentIsRejected() {
        assertViolation("let x = try! decode()", NoForceTryRule.self)
    }

    func testForceTryWithSafetyCommentIsClean() {
        assertClean(
            """
            // SAFETY: bundled fixture is validated in unit tests.
            let x = try! decode()
            """,
            NoForceTryRule.self
        )
    }

    func testPlainTryIsClean() {
        assertClean("let x = try decode()", NoForceTryRule.self)
    }

    func testForceUnwrapWithoutSafetyCommentIsRejected() {
        assertViolation("let x = optionalValue!", NoForceUnwrapRule.self)
    }

    func testForceUnwrapWithSafetyCommentIsClean() {
        assertClean(
            """
            // SAFETY: invariant enforced by configure() before this point.
            let x = optionalValue!
            """,
            NoForceUnwrapRule.self
        )
    }

    func testGuardLetIsClean() {
        assertClean(
            """
            guard let x = optionalValue else { return }
            _ = x
            """,
            NoForceUnwrapRule.self
        )
    }

    func testNotEqualsOperatorIsNotForceUnwrap() {
        assertClean("if a != b {}", NoForceUnwrapRule.self)
    }

    func testAnyParameterIsRejected() {
        assertViolation("func handle(input: Any) {}", NoAnyParametersRule.self)
    }

    func testAnyObjectParameterIsRejected() {
        assertViolation("func handle(input: AnyObject) {}", NoAnyParametersRule.self)
    }

    func testAnyErrorParameterIsExempt() {
        assertClean("func finish(error: (any Error)?) {}", NoAnyParametersRule.self)
        assertClean("func finish(error: any Error) {}", NoAnyParametersRule.self)
    }

    func testConcreteParameterIsClean() {
        assertClean("struct User {}; func handle(user: User) {}", NoAnyParametersRule.self)
    }

    func testAnyReturnIsRejected() {
        assertViolation("func load() -> Any { 1 }", NoAnyReturnsRule.self)
    }

    func testConcreteReturnIsClean() {
        assertClean("func load() -> Int { 1 }", NoAnyReturnsRule.self)
        assertClean("struct User {}; func load() -> User { User() }", NoAnyReturnsRule.self)
    }

    func testAnyTypealiasIsRejected() {
        assertViolation("typealias Payload = Any", NoAnyTypeAliasesRule.self)
    }

    func testAnyObjectTypealiasIsRejected() {
        assertViolation("typealias Delegate = AnyObject", NoAnyTypeAliasesRule.self)
    }

    func testConcreteTypealiasIsClean() {
        assertClean("typealias Handler = () -> Void", NoAnyTypeAliasesRule.self)
    }

    func testAnyDictionaryValueIsRejected() {
        assertViolation("let metadata: [String: Any] = [:]", NoAnyDictionaryValueRule.self)
    }

    func testConcreteDictionaryValueIsClean() {
        assertClean("let metadata: [String: Int] = [:]", NoAnyDictionaryValueRule.self)
    }

    func testShapeInTypeNameIsRejected() {
        assertViolation("struct UserShape {}", NoShapeInSymbolNamesRule.self)
        assertViolation("enum MessageShape {}", NoShapeInSymbolNamesRule.self)
        assertViolation("func shapePayload() {}", NoShapeInSymbolNamesRule.self)
        assertViolation("let eventShape = 1", NoShapeInSymbolNamesRule.self)
    }

    func testShapeInsideLongerWordIsRejectedCaseInsensitively() {
        assertViolation("struct ShapesProvider {}", NoShapeInSymbolNamesRule.self)
    }

    func testNonShapeSymbolsAreClean() {
        assertClean("struct User {}", NoShapeInSymbolNamesRule.self)
        assertClean("func process() {}", NoShapeInSymbolNamesRule.self)
        assertClean("let count = 1", NoShapeInSymbolNamesRule.self)
    }

    func testValueForKeyIsRejected() {
        assertViolation("let x = owner.value(forKey: \"name\")", NoKeyValueCodingRule.self)
    }

    func testSetValueForKeyIsRejected() {
        assertViolation("owner.setValue(1, forKey: \"count\")", NoKeyValueCodingRule.self)
    }

    func testPerformSelectorIsRejected() {
        assertViolation("owner.perform(#selector(Foo.bar))", NoKeyValueCodingRule.self)
    }

    func testSetNilValueForKeyWithUnlabeledArgumentIsRejected() {
        assertViolation("owner.setNilValueForKey(\"count\")", NoKeyValueCodingRule.self)
    }

    func testTypedPropertyAccessIsClean() {
        assertClean("let x = user.name", NoKeyValueCodingRule.self)
        assertClean("let x = dict.value(forKey2: \"k\")", NoKeyValueCodingRule.self)
    }

    func testTypedSelectorDispatchIsClean() {
        assertClean("handler.perform([request])", NoKeyValueCodingRule.self)
    }

    func testSelectorPerformIsStillRejected() {
        assertViolation("owner.perform(#selector(Foo.bar))", NoKeyValueCodingRule.self)
    }

    func testSetValueWithoutKeyLabelIsClean() {
        assertClean("owner.setValue(1)", NoKeyValueCodingRule.self)
        assertClean("owner.setValue(1, forKeyPath2: \"k\")", NoKeyValueCodingRule.self)
    }

    func testStringDescribingTypeOfIsRejected() {
        assertViolation(
            "let name = String(describing: type(of: value))",
            NoRuntimeTypeSniffingRule.self
        )
    }

    func testStringReflectingTypeOfIsRejected() {
        assertViolation(
            "let name = String(reflecting: type(of: value))",
            NoRuntimeTypeSniffingRule.self
        )
    }

    func testOrdinaryDescribingIsClean() {
        assertClean("let name = String(describing: value)", NoRuntimeTypeSniffingRule.self)
        assertClean("let n = type(of: value)", NoRuntimeTypeSniffingRule.self)
    }

    func testMultipleRulesReportTogether() {
        let violations = AntiSlop.lint(
            source: """
                struct ResponseShape {}
                let metadata: [String: Any] = [:]
                let x = try! run()
                """,
            fileName: "slop.swift",
            rules: SlopRule.allRules
        )
        let ids = Set(violations.map(\.ruleID))
        XCTAssertEqual(
            ids,
            [
                "no-shape-in-symbol-names", "no-any-dictionary-value", "no-force-try", "no-any", "explicit-access",
                "layer-imports",
            ]
        )
    }
}
