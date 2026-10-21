# Code-review worklist (W-series, part 3)

Status: completed 2026-10-21: every item is fixed in the review-fix PR after
#22, before the v0.0.2 campaign. Opened 2026-10-21 by the fifth
`/review-since` (`9bf18dc..255353b`, #20 to #22), logged in
`docs/logs/code-review-log.md`. Part 2, W-015 to W-023, is closed
(`plans/completed/code-review-w-series-2.md`). All items are low or cleanup;
none blocks v0.0.2.

IDs are unique across all worklists and continue from the highest W-number used.

## Success criteria

1. W-024: removing the W-017 sync after a failed write from
   `DirStore::write_sealed` or `DirStore::write_rebuilt` fails a test, and so
   does letting a failed write outrank a lost or inconsistent segment in heal.
2. W-025: a `DirStore` sealed into again after a failed `seal_stream` works.
3. W-026: no directory left to `Unsynced` is dropped unsynced.
4. W-027: a codec that returns fewer than n shards is a hard error again.
5. Each fix that can be reached has a regression test that fails without it,
   and the golden vectors are unchanged.

## Findings

### W-024 (low): three W-017 fixes had no test

- Where: `crates/neoblast/src/store.rs`, `write_sealed` and `write_rebuilt`;
  `crates/neoblast/src/stream.rs`, `heal_segments`'s ranking (#20).
- What: #20 said each fix had a regression test that fails without it. A
  fresh agent put back the old code (return before syncing) in
  `write_sealed`, `write_rebuilt` and `heal_manifest` together, and every
  test still passed: S-011's test reaches `finish` either way. No test
  checked that a lost or inconsistent segment outranks a write that failed
  before it.
- Remediation: tests for each.
- Verified: by a fresh agent, by mutation.
- Status: fixed. Regression tests:
  `store::tests::w024_a_failed_write_still_syncs_what_was_written` and
  `w024_an_inconsistent_segment_outranks_a_failed_write`; each fails when
  its fix is reverted. The early return in `heal_manifest` cannot be reached
  once S-019 is fixed, so it has no test.

### W-025 (low): a `DirStore` used again after a failed seal kept stale state

- Where: `crates/neoblast/src/stream.rs`, `seal_stream_with_key` (the stale
  `last` predates #20, which added `made_in`).
- What: a seal that failed part-way returned without calling
  `ShardSink::finish`, so the store still held the directories it had not
  synced. Sealing into the same `DirStore` value again tried to sync them,
  and failed if they were gone. The CLI makes a new store for each command,
  so it was not affected.
- Remediation: `seal_stream` calls `finish` at the end of every run past its
  parameter check, as `heal_manifest` does; the run's own error comes first.
- Verified: by a fresh agent, which reproduced it.
- Status: fixed. Regression test:
  `w025_a_store_can_be_sealed_into_again_after_a_failed_seal`.

### W-026 (cleanup): `write_manifest_bytes` threw a list of directories away

- Where: `crates/neoblast/src/store.rs`, `write_manifest_bytes` (#20).
- What: it passed a temporary `Unsynced` to `create_dirs` and dropped it.
  Nothing is left in it for the store's own root today, but a change to
  `create_dirs` could leave a directory there unsynced, with no error.
- Remediation: sync what is in it before the manifest is written.
- Verified: by a fresh agent, which found nothing left in it today.
- Status: fixed. No test: nothing can reach it.

### W-027 (cleanup): a short codec result counted as a publisher's mismatch

- Where: `crates/neoblast/src/stream.rs`, `heal_segments` (#20).
- What: if `regenerate` returned fewer than n shards, each missing one was
  reported as `RebuildMismatch` and heal went on, as for a publisher's
  inconsistent segment (S-011). Before #20 it stopped heal. It means a
  broken codec, which should not look like hostile input.
- Remediation: return an error at once, as before.
- Verified: by a fresh agent; `regenerate` always returns n shards today.
- Status: fixed. No test: nothing can reach it.

## Not tracked

Checked by the same review and left as they are, with the reason:

- `heal_segments` returns its results through two out-parameters: style; the
  ranking is correct and commented.
- A failed write or final sync is not mentioned when a lost segment is
  reported: by design and documented (`heal_stream`); the command still
  fails, and running heal again rewrites what was lost.
- Heal rebuilds from the first k shards that match: part of S-002, noted
  there.
- The hygiene job's attribution step runs `git log` once per commit: under a
  second for the 14 commits since `5393f67`.
