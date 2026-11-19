# Pairing and push (M2)

## Today

- **Pairing:** QR or a short code, approved from a paired device with its passkey (#68) or with `cooee pair approve` on the server (#484).

## How it works

Everything but `POST /pair` (and `/pair/request`, `/pair/claim`, `/pair/cancel`, `/passkey/challenge`,
`/passkey/signin`) needs a paired device: any other read or write gets 401. `cooee pair` (or Settings › devices › pair a device) shows a six-digit code and its `/pair?code=` link as a
QR code, good for 10 minutes and once; a new one drops the open one.
Five wrong codes a minute from one address (the forwarded one behind `tailscale serve`) refuse codes for the rest
of the minute, and five misses drop the code. The device that enters it gets a random token as an `HttpOnly`,
`SameSite=Strict` cookie. A `devices.json` that can't be read gets 503, never taken for "not paired".
`<data>/devices.json` (0600) keeps only SHA-256 hashes of codes and tokens, and the passkeys' public keys. The other
way round, a new device asks to pair (`/pair/request`), shows a two-word check word and polls `/pair/claim`; every
paired device gets a push and a card, and approving needs a passkey with Face ID (`/pair/answer`); a decline tells the
new device who said no; on the server's own machine `cooee pair asks` lists the requests and `cooee pair approve|decline
<id>` answers one (#484: how a Mac running `cooee setup` approves its own Mac app or a phone; the simulator e2e's
`testAsksAndTheCLIApprovesIt` drives it). Requests last 5 minutes, at most 4 at once. Passkeys are WebAuthn, ES256 on the platform
authenticator, offered after pairing: the server checks the challenge (once, 5 minutes), the origin, the relying
party's hash, user verification and the signature. A device that lost its cookie signs back in with its passkey
(`/passkey/signin`) as a new device. `cooee devices` lists the devices and `cooee devices rm <id>` unpairs one (its
pushes stop too); Settings → devices does both from the phone and lists and removes passkeys.

Before that, every request's `Host` (and its `Origin`, when sent) must be localhost, the `bind` address or one of
`allowed_hosts`, or it gets 403, paired cookie or not. That blocks DNS rebinding. To reach cooee from the phone,
keep `bind` on 127.0.0.1, run `tailscale serve --bg 4747`, and put the machine's Tailscale name in `allowed_hosts`.

Web push (RFC 8291 aes128gcm, VAPID per RFC 8292, sent with `curl`): the VAPID key is a file, `<data>/vapid.key`,
made once with mode 0600 (a looser one is tightened to 0600 before use); cooee keeps nothing in the macOS keychain.
Before #99 the key sat there (service `cooee`): while there is no key file, `cooee serve` reads it once in the
background, 5 s at most, and copies it to the file; `cooee push migrate` does the same and prints the
`security delete-generic-password` command that removes the old item, which cooee leaves for the owner. A read that
times out (a locked keychain) makes a new key, and every subscribed device is asked, with the reason, to turn
alerts on again (`GET /push/reset`). The thread list's 🔔 subscribes;
subscriptions sit in `<data>/push.json` (0600), each tied to its device. A push goes out when an agent asks for an
approval, a turn finishes while no page has the thread open, or an agent errors. Each one is a line about the thread:
`{title, body, tag: <thread>, url: /t/<thread>, item}`. A tap opens the thread, and reading the thread past `item`
clears it.

APNs (#130): the native app registers its device token with `POST /push/apns`; it sits in `push.json` beside the web
subscriptions, tied to its device, so unpairing the device drops it. Every push above also goes to each token over
HTTP/2 (`curl --http2`), `{aps: {alert: {title, body}, thread-id: <thread>}, url, item}` with `apns-collapse-id`
the thread, signed with a provider JWT (ES256, `kid` the key id, `iss` the team, made again every 50 minutes). A 410
or `BadDeviceToken` drops the token. The key is the owner's APNs auth key: Apple Developer › Certificates, IDs &
Profiles › Keys, a new key with Apple Push Notifications service (APNs), downloaded once as `AuthKey_<id>.p8`. It
lives only in `<data>/apns.p8` (mode 0600, a looser one tightened before use; never the keychain), and `[apns]`
holds what isn't secret. Without either, nothing goes to APNs and `cooee doctor` warns; it never prints the key.

Live Activity (#662): the hub ranks one activity, asks › failed › running with the rest as `+N`, and sends
`apns-push-type: liveactivity` (topic `<topic>.push-type.liveactivity`) on a start, a state change, an ask or an end,
and a new last line at most once per 15 s. A start goes to the push-to-start token, the rest to the update token;
an end drops the update token and the island shows `replied` for 15 minutes.

## API

### Alerts, devices, reports

| Call | Body | Answer |
|---|---|---|
| `GET /push/key` | | `{key}`: the VAPID public key, base64url |
| `POST /push/subscribe` | `{endpoint, p256dh, auth}` (`PushSubscription.toJSON()`) | text |
| `POST /push/unsubscribe` | `{endpoint}` | text |
| `GET /push/endpoints` | | this device's endpoints and APNs tokens, `string[]` |
| `POST /push/test` | `{}` | text |
| `POST /push/apns` | `{token, env?}`: a native device's APNs token (hex); `env` `sandbox` or `production`, default `[apns] env` | text; 400 for a token that isn't hex |
| `DELETE /push/apns` | `{token}` | text |
| `POST /push/activity` | `{token, activity?, env?}`: the Live Activity push-to-start token (no `activity`) or the running activity's update token | text; 400 for a token that isn't hex |
| `GET /glance` | | `{at, asks, blocked, running}`, each row `{thread, title, persona, name, machine, line, since, lane}`: what the widgets show, as of `at` |
| `GET /devices` | | `{id, label, created, used, me, by}[]`: `by`, the device that approved its request (empty if none) |
| `POST /devices/remove` | `{id}` | text |
| `POST /devices/pair` | `{}` | `{code, link, svg, host}`: a new code (dropping the open one), its link and the link as a QR code |
| `GET /pair/asks` | | the requests to pair still waiting, `{id, word, label, at}[]` |
| `POST /pair/answer` | `{id, yes, passkey?}`: approving needs a passkey's `{id, client, auth, sig}` | text; 403 for a bad passkey, 410 when it's gone |
| `GET /passkeys` | | `{id, label, created, used, mine}[]`: `mine`, made on (or signed back in to) this device |
| `POST /passkeys` | `{id, key, client, auth}`: `navigator.credentials.create`'s id, `getPublicKey()` (P-256), client and authenticator data | text |
| `POST /passkeys/remove` | `{id}` | text |
| `POST /passkey/options` | `{}` | `{challenge, id}`: a challenge for this device's passkey ceremony (2 minutes, once), its passkey's credential id or `null` |
| `POST /passkey/register` | `{id, client, auth, key}` (base64url: the credential id, `clientDataJSON`, `getAuthenticatorData()`, `getPublicKey()`) | text; the device's first passkey, which unlocks it |
| `POST /passkey/unlock` | `{id, client, auth, sig}` (`authenticatorData`, the DER `signature`) | text; 403 with why when it doesn't check out |
| `POST /report` | `{note, path, target, html, look, viewport}` | text |

### Pairing (no cookie needed)

| Call | Body | Answer |
|---|---|---|
| `POST /pair` | a form: `code` (or `code` and `d2`…`d6`) | 303 to `/?paired` with the cookie, else to `/pair?bad=wrong&left=N`, `bad=expired` or `bad=limit&wait=S` (5 wrong codes a minute per address) |
| `POST /pair/request` | `{}` | `{word, claim}`: the check word; every paired device gets a push and the card |
| `POST /pair/claim` | `{claim}` | 200 with the cookie once approved, 202 while it waits, 403 declined (who said no, text), 410 when it's gone (5 minutes) |
| `POST /pair/cancel` | `{claim}` | text |
| `POST /passkey/challenge` | `{}` | `{challenge, ids}`: a challenge good once for 5 minutes, the passkeys' ids |
| `POST /passkey/signin` | a passkey's `{id, client, auth, sig}` (`navigator.credentials.get`, user verified) | 200 with the cookie, 403 why not |
