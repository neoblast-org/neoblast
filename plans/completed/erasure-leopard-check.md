# Leopard byte check (ROADMAP open item 7)

Status: completed 2026-10-14. Outcome recorded in decision 0006.

## Why now

Parity bytes are the only part of the Phase 1 format that could still change
because of a third-party crate. Settling it allows the golden vectors to be frozen.

## Success criteria

1. A yes/no answer, using the decision rule fixed in ROADMAP item 7 before the
   check: does `reed-solomon-simd` match Leopard-RS byte for byte?
2. The answer covers our real parameters: 20-of-50 with a full 4 MiB segment's
   shard size (209,728 bytes), plus the format's k and n extremes.
3. Every condition needed for a match (rate, alignment, k range) is enforced in
   code, so a caller cannot bypass it:
   - `HighRateEncoder` / `HighRateDecoder` are used explicitly;
   - `check()` rejects shard lengths that are not multiples of 64 and k < 2;
   - `Header` rejects k < 2;
   - `shard_len` always returns a multiple of 64.
4. A differential test runs in the normal `cargo test`:
   - it fails if any fixture byte changes (verified by corrupting one hash);
   - it decodes from Leopard's parity, as well as checking that encoding matches.
5. Fixtures can be regenerated from upstream by one script, Leopard is not
   vendored, and its licence is credited.
6. The format change is propagated in one change: spec, decision record,
   regenerated `phase1.json`, an updated regression test for issue 10, and the
   ROADMAP item closed.
7. fmt, clippy `-D warnings`, the full test suite and `cargo deny` stay green, and
   no new dependencies are added.

## Precedent matched

- Decision record: mirrors `decisions/0002-root-commits-to-everything.md` (MADR:
  context, outcome, consequences).
- Fixture test: mirrors `crates/neoblast/tests/golden_vectors.rs`, which reads a
  JSON file under `spec/test-vectors/` with serde and compares field by field.
- Spec text: mirrors the existing "Shards" section of `spec/header.md`.

## Results

- Match on all 66 committed cases, and on all 75 m ≤ k configurations of the
  wider exploratory sweep. Rules: high rate, 64-byte shards, k ≥ 2.
- m > k (including 20-of-50) matches Leopard's algorithm with its API argument
  check removed.
- `phase1.json`: only `shard_size` (and the hashes covering it) changed.
- Second review (independent agent): no blockers. Fixed:
  - the trait methods now enforce the rules too;
  - the fixture test checks the exact case list and build labels;
  - k = 2, m = 1 and n = 128 edge cases added (58 → 66);
  - the decision's counts and k = 1 wording corrected;
  - a pre-freeze exception noted where the docs required a version bump;
  - generator hardening.
- Deliberately left:
  - Leopard's newest commit, which did not build here; the pinned commit
    `22ddc78` is the reference;
  - hash-pinning the generator's `blake3` pip install, since its only output is
    the reviewed fixture diff.
