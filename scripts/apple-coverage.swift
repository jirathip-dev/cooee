// Line coverage per layer from `swift test --enable-code-coverage`, as `llvm-cov export -summary-only` over every test
// bundle (scripts/apple-check.sh).
// Fails unless CooeeCore, CooeeAPI and CooeeFeatures each cover at least 95% of their lines, naming each layer's number.
// Usage: swift scripts/apple-coverage.swift <codecov.json>
import Foundation

struct Export: Decodable {
    struct Data: Decodable { let files: [File] }
    struct File: Decodable {
        let filename: String
        let summary: Summary
    }
    struct Summary: Decodable { let lines: Lines }
    struct Lines: Decodable {
        let count: Int
        let covered: Int
    }
    let data: [Data]
}

let minimum = 95.0
let layers = ["CooeeCore", "CooeeAPI", "CooeeFeatures"]
let path = CommandLine.arguments.dropFirst().first ?? ""
let export = try JSONDecoder().decode(Export.self, from: Data(contentsOf: URL(fileURLWithPath: path)))
var failed = false
for layer in layers {
    let files = export.data.flatMap(\.files).filter { $0.filename.contains("/apple/Sources/\(layer)/") }
    let count = files.map(\.summary.lines.count).reduce(0, +)
    let covered = files.map(\.summary.lines.covered).reduce(0, +)
    let percent = count == 0 ? 0 : Double(covered) * 100 / Double(count)
    let ok = percent >= minimum
    failed = failed || !ok
    print("\(ok ? "ok  " : "FAIL") \(layer): \(String(format: "%.1f", percent))% of \(count) lines (minimum \(Int(minimum))%)")
}
exit(failed ? 1 : 0)
