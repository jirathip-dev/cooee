# ADR: push for self-hosters: content-free pushes, one delivery seam, a relay later (#349)

## Summary

Decision: two cheap choices now, the relay itself later. (1) A push may be content-free: it names the thread and
why (needs you, done, failed) and no message text; the app fetches the text from the owner's own server over the
tailnet when it wakes. Content-free is an option for direct sends and the only form a relayed send takes. (2) Every
push leaves through the one worker #344 built; a relay is a second sender behind it, chosen per device
subscription. The trigger to build the relay is the first outside user who self-hosts cooee with the TestFlight
or App Store app.

## Question

The App Store app's APNs topic belongs to cooee's key. Someone else who runs the cooee server cannot hold that key,
so their server cannot push to their phone. How do we keep the way open to a relay without designing it now, and
without a relay ever seeing message text?

## Constraints found in the code

1. `crates/cooee-server/src/push/apns.rs` signs an APNs JWT with the server's own `.p8` key and posts
   `{"aps": {"alert": {"title", "body"}, "thread-id"}, "url", ...}` straight to APNs.
2. A `Note` (`push/note.rs`) carries `title` (the thread's title), `body` (one line, up to 120 characters, with
   the agent's text in it), `tag` (the thread id), `url` and `item`.
3. `Push::send_to` (`push/worker.rs`) queues every note for one worker; `Push::deliver` (`push/send.rs`) walks the
   paired devices' `Sub`s and builds one post per sub: APNs when `Sub.apns` is set, else encrypted web push. That
   per-sub branch is already the seam a relay sender joins.
4. Web push is end-to-end encrypted to the browser's key (`push/crypto.rs`), so the push service never reads it.
   APNs payloads are readable by whoever holds the key or forwards the payload, which a relay would.
5. T3 Code and OpenClaw (as reported by the 2026-11-13 survey, not rechecked) both need a hosted relay for
   background push to a store-built app; OpenClaw's relay uses App Attest and never hands the gateway the raw token.

## Options

| | Choice | For | Against |
|---|---|---|---|
| Content A | Relay forwards today's title and body | No app change | A third party reads every approval and agent line |
| Content B | Encrypt the payload to the app, relay forwards ciphertext | Text still on the lock screen | A key exchange and a notification service extension of our own |
| Content **C** | **Content-free: thread id, item id, kind; the app fetches the text** | The relay sees nothing worth reading; no crypto design | The text shows after the fetch, not in the banner |
| Seam A | A relay path beside the worker | Quick | Two ways to send, two throttles |
| Seam **B** | **A third sender kind in `deliver`, picked by the device's sub** | One queue, one throttle, one log | `Sub` gains a field when the relay lands |

## Decisions

1. **Content-free pushes.** A content-free push carries the thread id, the item id and the kind (`needs`, `done`,
   `fail`) and a generic line ("A thread needs you"), never the thread title, the agent's line or a tool name. The
   app, on tap or in a notification service extension, fetches the item from the owner's server over the tailnet
   and shows it. Direct APNs and web push keep today's text by default, with content-free as an owner setting;
   a relayed send is always content-free and the relay sender builds only that form, so no setting can leak text
   through it.
2. **One delivery seam.** The relay is a sender inside `Push::deliver` next to APNs and web push, behind the same
   worker queue, chosen per `Sub` (a device registered through the relay gets a relay sub instead of an APNs
   token). Nothing else in the server learns that a relay exists.
3. **Trigger.** Nothing is built until the first outside self-hoster needs background push to the store app. Then
   one issue builds decision 1's content-free form and the relay sender, with its tests (below).

## Decided later, at the trigger

1. Who runs the relay and what it costs.
2. Device attestation (App Attest) so the relay forwards only to real installs.
3. Whether the relay is the same service as #74's connection relay.

## Tests owed at the trigger

The issue's second criterion applies when decision 1 is built, not by this note, because no relayed push exists
yet to test:

1. A relayed push's payload, captured at a fake relay, carries none of the item's text, the thread title or the
   tool name: only ids, the kind and the generic line.
2. The app's reducer, given that push, fetches the item and shows its text (a TestStore test with the threads
   client's fetch).

## Consequences

- No code changes now; `push/` stays as #344 left it.
- A relay never needs its own encryption design, and leaking content through it needs a code change, not a setting.
- Content-free banners say less on the lock screen; owners who send direct keep the text unless they opt out.
