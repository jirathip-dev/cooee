# Native motion (#246): frame pairs from the iPhone simulator

Each PNG puts consecutive simulator screenshots (about 0.3 s apart) side by side, taken from the app running against
`apple/e2e/serve.sh` (iPhone 17 Pro, iOS 27.2, light). The loading frames were taken with that server paused, so the
load stays open long enough to see.

| File | Shows |
| --- | --- |
| `working-pill-modern.png`, `working-pill-scene.png` | The working pill: the persona's horse beside `Working` and Stop, its legs swapping gait frames |
| `loading-modern.png`, `loading-scene.png` | A chat loading: the pulsing bar under the header with a small walking horse, the big sprite and `saddling up`; the last Modern frame is the send button's spinner until the server takes the message |
| `big-sprite-empty-modern.png` | An empty chat's big sprite (`no horses out`) |
| `scene-walk.png` | Scene: Coder, on the trail, walks with a gait in the farm strip; Conductor grazes |
| `rows-needs-modern.png`, `rows-needs-scene.png` | A needs-you row on Home pulses its avatar and fades its red tint |

Not filmed: the done hop and the failed shake. The e2e agent cannot fail a chat and leaves finished chats idle, and
the queue's failed rows are below the fold until after their 2.4 s moment. Their poses are covered by
`apple/Tests/CooeeCoreTests/MotionTests.swift`.
