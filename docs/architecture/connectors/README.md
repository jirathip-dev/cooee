# Connectors (#497)

## Summary

Decision: a connector is one small reviewed file in cooee's repo (who runs the server, its exact address, how to sign in, which tools read and which write). The phone adds it and opens the service's own sign-in page; the answer comes back to the Mac over the tailnet; the Mac keeps the tokens in its keychain and makes every call itself, so no token reaches an agent, a file or a log. Read tools run, write tools ask with a card, per persona and account.
Slices: A the catalogue and the registry's new keys; B the Mac makes the calls (the older streaming kind included); C the read/write gate and its card; D sign-in, refresh and sign-out; E the phone's screens; F read-only mail and calendar.
Open questions: (1) Google mail and calendar need your own Google sign-in client, made once on Google's console: OK, or app passwords instead? (2) Sign-in adds one address that answers without a paired device (tailnet only, useless without a one-time code): OK? (3) Which services first: Linear, Notion and GitHub are my picks.

## Read in this order

1. [`adr.md`](adr.md): the question, what the code and the spec say, the options, the twelve decisions, the hot
   files, the six impl issues, the open questions, the `.drover/arch.toml` lines.
2. [`callers.md`](callers.md): the phone's calls from "add" to "attached", the app's reducer, a catalogue entry's
   file, the proxy, cooee answering a call, the files afterwards, the card and the Settings row. Written first.
3. [`shapes.md`](shapes.md): the entry, the registry's three sources, a pick's account, accounts and credentials,
   a call's types, the API table.
4. [`boundaries.md`](boundaries.md): which module may call which, what holds each rule, the statecharts of an
   account and of a tool call.

## Where the issue's points are answered

| The issue asks | ADR decision |
|---|---|
| The catalogue, pinned; what "not reviewed" means | 2, 3 |
| Sign-in from the phone: the sheet, the callback, the tokens, refresh, revoke; the token case | 5, 6, 7 |
| The default allowlist and #72 | 8 |
| Per persona; a room with two accounts | 9, 10 |
| `sse` and streamable HTTP through the proxy | 4 |
| What leaves the Mac, per connector | 11 |
| D2: mail and calendar read-only, not the browser | 12 |
| The impl issues, in order | "The impl issues, in order" |

## What was checked, and what was not

- Read in the code at this branch's base (cf2a6be46): the registry, the proxy and its grant, the gate's open paths,
  the secrets store, the cards, a room's asks, the three `curl` call sites, the line caps.
- Read on 2026-11-17: MCP's 2025-11-25 authorization and transport pages (redirect URIs, discovery, registration
  order, PKCE, `resource`, streamable HTTP, the `sse` fallback).
- Not checked: any live service. No entry is listed until its lane has signed in to the real server; Google's
  terms for a personal client are slice F's first step ([ADR, consequences](adr.md)).
