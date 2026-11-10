Closes #198

Raises `line_cap` in `ci/cap.toml` from 23,500 to 27,500 (`apple_cap` unchanged at 5,000) and records it in `docs/cap-history/2026-11-10-198.md`. Owner's pick: q1183.

## For humans

The repo's size limit goes up to 27,500 lines once, enough for all the open and queued work.
The Apple app's limit is unchanged.

## Evidence (head 8d71cc9531f5faac1bfcc64ef93faea0187bc9e5, base staging 0b3b591)

- `scripts/line-cap.sh` → exit 0: `non-test lines: 23398 / 27500`, `apple lines: 4014 / 5000`
- `ci/line-cap-test.sh` → exit 0: `line-cap test ok`
- `just` is not installed on this host, so `just cap` was run as its two commands above.

src/ net +0 (budget +0): config and docs only.
