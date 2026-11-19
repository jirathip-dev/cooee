#if os(iOS)
    import XCTest

    internal final class LiveReliabilityUITests: LiveUITestCase {
        @MainActor
        internal func testTheComposerSitsOnTheKeyboardAndTheThreadKeepsItsPlace() throws {
            let app = try launchHome()
            let lines = (1...40).map { "line \($0)" } + ["the last line"]
            send(app, "fake says: \(lines.joined(separator: " / "))")
            let last = app.descendants(matching: .any).matching(NSPredicate(format: "label BEGINSWITH 'line 1'"))
                .firstMatch
            XCTAssert(last.waitForExistence(timeout: 30), "no long reply")
            let keyboard = app.keyboards.firstMatch
            if keyboard.exists { app.swipeDown() }
            XCTAssert(keyboard.waitForNonExistence(timeout: 5), "the keyboard stayed up")
            let attach = app.buttons["Attach"].firstMatch
            let gap = settled(attach).minY - last.frame.maxY
            keep("keyboard-down")
            app.textFields.matching(NSPredicate(format: "label BEGINSWITH 'Message '")).firstMatch.tap()
            XCTAssert(keyboard.waitForExistence(timeout: 10), "no keyboard came up")
            let bar = settled(attach)
            keep("keyboard-up")
            XCTAssert((0...56).contains(keyboard.frame.minY - bar.maxY), "the composer is off the keyboard: \(bar)")
            XCTAssertEqual(bar.minY - last.frame.maxY, gap, accuracy: 2, "the thread lost its place")
        }

        @MainActor
        internal func testAPluginSheetClosesWithNoBlankFrame() throws {
            let app = try launchHome()
            let queue = droverChat(app, newChat(app))
            app.buttons["Events"].firstMatch.tap()
            XCTAssert(labelled(app, "merged").waitForExistence(timeout: 20))
            app.buttons["Done"].firstMatch.tap()
            let start = Date()
            var frames = 0
            while Date().timeIntervalSince(start) < 1.5 {
                let shot = XCUIScreen.main.screenshot()
                frames += 1
                if Self.blank(shot.image) { keep("sheet-close-blank", shot) }
                XCTAssertFalse(Self.blank(shot.image), "frame \(frames) of the sheet closing is blank")
            }
            XCTAssert(queue.waitForExistence(timeout: 5) && queue.isHittable, "the chat is not back")
            XCTAssert(frames > 2, "only \(frames) frames caught")
        }

        @MainActor
        private func settled(_ element: XCUIElement) -> CGRect {
            var frame = element.frame
            for _ in 0..<20 {
                RunLoop.current.run(until: Date().addingTimeInterval(0.1))
                guard element.frame != frame else { return frame }
                frame = element.frame
            }
            return frame
        }

        private static func blank(_ image: UIImage) -> Bool {
            let (width, height) = (32, 64)
            var pixels = [UInt8](repeating: 0, count: width * height * 4)
            let drawn = pixels.withUnsafeMutableBytes { bytes in
                guard let picture = image.cgImage,
                    let context = CGContext(
                        data: bytes.baseAddress, width: width, height: height, bitsPerComponent: 8,
                        bytesPerRow: width * 4, space: CGColorSpaceCreateDeviceRGB(),
                        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)
                else { return false }
                context.draw(picture, in: CGRect(x: 0, y: 0, width: width, height: height))
                return true
            }
            guard drawn else { return false }
            let body = Array(pixels.dropFirst(width * 4 * 6))
            let first = Array(body.prefix(4))
            let off = stride(from: 0, to: body.count, by: 4).filter { index in
                (0..<3).contains { abs(Int(body[index + $0]) - Int(first[$0])) > 12 }
            }
            return off.count < width * height / 100
        }
    }
#endif
