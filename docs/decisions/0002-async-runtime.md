# 0002: Tokio, through Topcoat (#2)

Status: accepted, 2026-11-05.

## Context

The engineering standards say: few, boring dependencies, and an async runtime or web framework only with an ADR.
herdsman's server runs without one. ADR 0001 (#10) chose Topcoat for the web UI, and the owner confirmed it.

## Decision

`cooee-server` uses Topcoat (`=0.11.0`, default features off, only `serve`) as its web framework, and so Tokio as its
runtime: Topcoat is a tokio-rs project and its server is Tokio's. The runtime is built inside `cooee_server::serve`, so
`cooee-core` and the binary stay synchronous and Tokio-free.

Why it is needed, not just convenient: Topcoat's live regions hold one WebSocket per open tab and re-render on every
change (ADR 0001), and M1 streams ACP sessions from shed into them. Many long-lived connections with streaming is the
case an async runtime is for; a thread per connection would be the alternative and buys nothing here.

## Consequences

- One runtime for HTTP, WebSockets and (M1) shed's ACP client.
- Topcoat is pre-1.0: pinned exactly, and its upgrades are their own PRs (ADR 0001's risks).
- Topcoat 0.11 needs Rust 1.99, so that is the MSRV (`rust-toolchain.toml`).
- Topcoat's default features are off; each one we turn on later is a line in `Cargo.toml` and a reason in its PR.
