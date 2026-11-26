# 0003: Desks go/no-go: Apple `container`, not Lume (#168)

Status: proposed, 2026-11-21. Spike K0, run on the Mini (macOS 27.2, arm64, 12 CPUs) with lume 0.6.1 and Apple
`container` 1.7.1 (system service running).

## Question

Can a desk (a full desktop machine a persona drives) run on the Mini without reaching the LAN or the tailnet, and
with which provider: Lume or Apple's `container`? And is swag its own repo or a crate inside cooee?

## What was measured

No desk image was cached, so each provider pulled one. Sizes are the registry's compressed layer totals; times are
wall clock on the Mini's link.

| | Lume | Apple `container` |
| --- | --- | --- |
| Desk image | `trycua/ubuntu-noble-vanilla` (Ubuntu 24.04.1, `ubuntu-desktop`, `graphical.target`) | `linuxserver/webtop` (XFCE in the browser and VNC) |
| Download | 3.91 GB in 114 s | 0.96 GB in 164 s |
| On disk | 8.2 GB (20 GB sparse disk) | image only, no disk file |
| Start | IP after 13 s, SSH after about 2 min | `run -d` returns at once, the desktop answers HTTPS within about 2 min |
| Idle memory | 1.1 GB used of 4 GB | 964 MiB of 4 GB |
| macOS desk | yes: `macos-sequoia-vanilla` 17.8 GB (40 GB disk), `macos-tahoe-vanilla` 21.0 GB (100 GB disk), not pulled: 32 GB was free | no, Linux only |
| Network modes | `nat` (default), `bridged`; no isolated mode | `default` (NAT), `network create --internal` (host-only) |

Reach, probed from inside each desk (TCP connect with a 3 to 4 s timeout, and ping):

| Target | Lume `nat` | `container` default | `container --internal` |
| --- | --- | --- | --- |
| LAN router (HTTP, DNS, ping) | reached | reached | blocked |
| Internet (`1.1.1.1:443`) | reached | reached | blocked |
| Another tailnet machine (ping; the host reaches it) | blocked | blocked | blocked |
| The host's own LAN and tailnet addresses (SSH) | reached | reached | reached |
| Tailscale's MagicDNS address (DNS port) | reached | reached | reached |

On `--internal`, a desk reaches only the host's listeners bound to every address: 7 of the host's 33 TCP listeners
(screen sharing, AirPlay, `rapportd`, a DNS forwarder, a Python server, and Lume's own VM API). Loopback-only
listeners stayed closed. Other tailnet machines are out of reach in every mode because the Tailscale app does not route
traffic forwarded from the VM network.

## Decision

- **Go for Linux desks on Apple `container`, on an `--internal` network only.** It is the only tested mode that blocks
  the LAN, the internet and other tailnet machines without changing the host, and the image is a quarter of Lume's.
- **No-go for Lume as the provider.** It has no isolated mode: its NAT reaches the LAN router and the internet, and
  blocking that needs a packet-filter rule on the host. Its API listens on every address, so a desk can reach it.
- **No-go for macOS desks for now.** Only Lume runs them, the images are 18 to 21 GB against 32 GB free, and they have
  the same reach problem. Revisit when a task needs macOS and the host has the disk.
- **swag is not the desks' home.** The name is taken: `jirathip-dev/swag` is now the blocks theme package. Desks are
  one crate inside cooee (`cooee-desk`), which runs the `container` CLI through `runner::command`. A separate repo
  buys nothing for one provider behind one CLI.

## Conditions before the first desk ships

- Desks run only on an `--internal` network that cooee creates; the default network is never used.
- What a desk may reach on the host is a list: the host's other listeners bind to loopback, or the desk crate refuses
  to start while one is bound to every address. Lume's API is the first to close.
- Any internet a desk needs goes through a host-side proxy with an allowlist that refuses private, link-local and
  tailnet (CGNAT range) addresses, not through NAT.
- A test starts a desk and repeats this probe: the LAN router, the internet and a tailnet machine stay unreachable.

## Consequences

- Linux desks only. A macOS-only task waits for the revisit above.
- One provider and one CLI, so there is no provider trait until a second backend (Orgo, e2b) is real.
- The spike left nothing behind: the VM, containers, network and images were deleted.
