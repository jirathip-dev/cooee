# cooee rules (impl and rev briefs carry these; each names its enforcer)

- Don't comment Swift code except `///` on public API, `// SAFETY:` above a `!` or `try!`, and `// TODO(#N)` on an open issue — anti-slop no-comments, apple-check TODO step
- Don't leave Swift imports unsorted or unused — anti-slop sorted-imports, swiftlint analyze unused_import
- Don't import across Swift layers (Core → API → Features → UI → App) beyond apple/Package.swift's table — anti-slop layer-imports
- Don't use `Any` or `AnyObject` as a Swift type — anti-slop no-any
- Don't leave a Swift declaration without its access level — anti-slop explicit-access
- Don't force unwrap, `try!` or `as!` in Swift without a `// SAFETY:` line — anti-slop no-force-unwrap, no-force-try, require-safety-comment-for-forced-cast
- Don't exceed Swift file 250, type 150, function 25 lines, complexity 8 or nesting 2 — SwiftLint
- Don't hand-format Swift — swift format lint --strict
- Don't keep app state outside a TCA `@Reducer`, or call side effects except through a Features `@Dependency` client — anti-slop layer-imports (URLSession only in CooeeAPI), reducer TestStore tests
- Don't let Swift warnings through: Swift 6, complete concurrency, warnings as errors — apple/Package.swift (COOEE_STRICT=1 in apple-check), Cooee.xcodeproj
- Don't drop CooeeCore, CooeeAPI or CooeeFeatures under 95% line coverage — scripts/apple-coverage.swift
- Don't leave unused Swift code — Periphery
- Don't change a design frame without its snapshot (apple/design/v1) — CooeeUITests SnapshotTests
- Don't name a provider (claude, anthropic, codex, openai, gemini, hermes) outside crates/cooee-core/src/providers/ and tests — scripts/neutral-core.sh
