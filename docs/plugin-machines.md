# First-party plugin: machines

## How it works

`crates/cooee-machines`, built into cooee: the health of the machines shed runs on. A `[[plugins]]` entry named
`machines` turns it on for that instance, and `cooee serve` runs its backend itself at the entry's `url`:

```toml
[[plugins]]
name = "machines"
label = "Machines"
url = "http://127.0.0.1:4801"     # a free port; cooee listens there for the plugin
module = "/tab.js"
token = "another-long-random-secret"
```

Once a minute it asks shed (never SSH itself): `shed machine ls`, then `shed machine stats` for this machine and
`shed --on <m> machine stats` for each added one (shed answers offline within 5 s). Each card shows online or offline
with when it was last seen, load per CPU, memory pressure, swap, free disk and shed's terminals by state; each number
has a 24 h sparkline, and a tap opens its chart. Only the samples newer than the last kept one are asked for. A
machine going offline, under 10 GB of free disk or at 90 % memory pressure or more sends one push (tag `machine`, a
tap opens `#<machine>`) when its set of problems changes, and the tab shows a dot while any machine has one.
