# Neoblast

Censorship-resistant storage: a file is encrypted on the owner's machine,
erasure-coded into shards, and every byte is committed to one root hash. Storage
nodes hold only ciphertext and can verify and heal shards without the key.
Experimental and not audited.

Read before changing anything:

- `ROADMAP.md`: phases, gates and the open items with their decision rules.
- `spec/`: the byte formats. The spec, not the code, is the source of truth.
- `decisions/`: why things are the way they are (MADR records).
- `spec/threat-model.md`: what v1 does and does not protect.

## Layout

- `crates/neoblast-format`: header, layout rules, capability links. Pure, no I/O, fuzzed.
- `crates/neoblast-crypto`: key derivation and segmented XChaCha20-Poly1305.
- `crates/neoblast-erasure`: Reed-Solomon behind the `ErasureCode` trait.
- `crates/neoblast`: the main crate and the `neoblast` CLI (`seal` / `open` / `heal`).
  All four crates are published together; see `RELEASING.md` (decision 0008).
- `spec/test-vectors/`: golden vectors that lock the byte format.
- `fuzz/`: `cargo-fuzz` targets, `campaign.sh` and run records in `fuzz/campaigns/`. `sim/`: durability model.

## Commands

- `cargo fmt --all` · `cargo clippy --all-targets -- -D warnings`
- `cargo test --all` · `cargo deny check`
- `NBL_UPDATE_VECTORS=1 cargo test -p neoblast --test golden_vectors`: only for an
  intended format change that has a decision record and a spec update.
- `cargo +nightly fuzz run <target>` · `fuzz/campaign.sh summary` · `python3 sim/durability.py`

Keep fmt, clippy, tests, `cargo deny` and the golden vectors green.

## Conventions

- **Parameters come only from a verified header.** Never trust k, n, lengths or
  indices sent by a peer; take them from a header checked against the file root.
- **Parsers never panic.** No `unwrap`, `expect`, `panic` or unchecked indexing
  outside tests. `unsafe` is forbidden in first-party crates.
- **Byte-format changes** need a decision record, a `spec/` update and regenerated
  golden vectors, in the same change.
- **Dependencies follow the blast-radius rule** in `decisions/README.md`. A crate
  with few dependents needs a written reason, an exact pin and a wrapper trait.
- **Secrets** appear only after `#` in links, never in logs or `Debug` output.
- **No kill switch.** Nothing may let anyone delete another party's shards.
- **Every fixed bug gets a regression test** (see `crates/neoblast/tests/review_regressions.rs`).

## Working agreement — every plan & change

Apply these three steps to every plan and every change, not just large ones:

1. **Define the success criteria first.** Before implementing, state explicit,
   checkable criteria for a great result — a mini definition-of-done: what must be
   true, which edge cases are covered, and the quality bar. Plans lead with these;
   changes restate the ones they satisfy. Vague goals ("make it work") are not
   criteria; "a forged k > n is rejected by `Header::decode` without panicking" is.
2. **Match a past example.** Find the closest existing example — a sibling module in
   the same crate, a test in `crates/*/tests/`, a record in `decisions/`, a file in
   `spec/` — and mirror its structure, naming and idiom instead of inventing a new
   shape. Say what you're matching.
3. **Have a second AI check the final output.** Before declaring done, run an
   independent review pass (a separate agent or subagent) that checks the result
   against the criteria in (1) and the example in (2), and reports gaps. Fix what it
   finds; call out anything deliberately left.

## Commits and privacy (this repository is public)

- Commits are made as `neoblast maintainers <dev@neoblast.dev>`, with UTC
  timestamps, signed, and signed off (DCO).
- Never put personal names, personal email addresses, local file-system paths or
  links to AI or tool sessions into commits, pull-request text, code or docs,
  including session-link trailers in commit messages.
- Leave changes uncommitted unless asked; the maintainer reviews and commits.

## Reviews

- `/review-since` and `/security-review-since` review only the commits since the last
  logged review. Logs live in `docs/logs/`; worklists in `plans/todo/`, `plans/blocked/`
  and `plans/completed/`.
- Plans submitted for approval must contain success criteria and a named precedent;
  `.claude/hooks/require-plan-criteria.mjs` enforces this.
