# Decision records

Each significant design choice is recorded here using
[MADR](https://adr.github.io/madr/). Records are never deleted; a reversed
decision gets a new record that supersedes the old one.

| # | Decision | Status |
| --- | --- | --- |
| [0001](0001-rust-only-mvp.md) | Rust-only MVP | Accepted |
| [0002](0002-root-commits-to-everything.md) | The file root commits to every parameter | Accepted |
| [0003](0003-threat-model.md) | Threat model: four adversaries, no anonymity in v1 | Accepted |
| [0004](0004-capability-model.md) | Capability model: verify ⊂ read ⊂ write | Accepted for files; channels and keyring proposed |
| [0005](0005-storage-reciprocity.md) | Why strangers store your shards: swaps, pins, donors, leases | Proposed |
| [0006](0006-erasure-code.md) | Erasure code: Leopard-RS high-rate, checked against Leopard | Accepted |
| [0007](0007-no-multiformats-cids.md) | Links keep their own version byte, not multiformats CIDs | Accepted |
| [0008](0008-crate-names-and-publishing.md) | Crate names (`neoblast-*`) and publishing | Accepted |

## How a question becomes a decision

1. **Frame** the question in a new `NNNN-<slug>.md`: what it blocks, and the fixed constraints (free, solo, publishing first).
2. **Retrieve** two or three projects that already faced it. Read their design docs first, code second, then their issue trackers for "v2" or "redesign".
3. **Check licences** before copying anything. MIT, Apache-2.0, BSD and CC0 code may be used with attribution. GPL, AGPL and TGPPL projects (Tahoe-LAFS, Hyphanet, Storj's server, Peergos) are for ideas only.
4. **Spike** for at most two evenings in throwaway code that answers one measurable question.
5. **Decide** using a rule written down *before* the spike's result is known.
6. **Propagate** to `spec/`. If bytes change, that is a new format: a new magic, suite or link version byte and a new vector file, since the v1 vectors are frozen.

## The blast-radius rule for dependencies

- **Large** (1,000+ reverse dependencies, or core ecosystem infrastructure): use freely; pin via `Cargo.lock`; keep `cargo-deny` clean.
- **Medium** (100–1,000, or backed by a large organisation but pre-1.0): use behind our own types; pin exact versions.
- **Small** (fewer than 100, one maintainer, stale, or churning pre-1.0): question it. Can we vendor it (under ~2,000 lines)? Does its output reach the wire or disk format (then the spec must define those bytes and tests must cross-check)? Can a large-radius crate do the job?

Currently flagged small: `reed-solomon-simd` (its parity bytes are part of the format; cross-checked against Leopard-RS by decision 0006), and later `mainline`, `pkarr` and `bao-tree`.
