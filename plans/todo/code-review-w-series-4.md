# Code-review worklist (W-series, part 4)

Status: todo. Opened 2026-10-25 by the sixth `/review-since`
(`255353b..c3d405d`, #23, #25 and #26), logged in
`docs/logs/code-review-log.md`, while the v0.0.2 campaign (step 7d″) ran on
`c3d405d`. Part 3, W-024 to W-027,
is closed (`plans/completed/code-review-w-series-3.md`). All items are low or
cleanup; none blocks v0.0.2. Each changes `crates/`, and so every target's
`code_id`, so they wait until after v0.0.2 is tagged.

IDs are unique across all worklists and continue from the highest W-number used.

## Success criteria

1. W-028: making `full_sync` fall back after an `EIO`, by any edit, fails a
   test.
2. W-029: a codec result of the wrong count or length is a `Codec` error
   wherever `neoblast-erasure` returns shards, never a store's or a
   publisher's fault.
3. W-030: `review_regressions.rs` has one recording sink and one way to
   rebuild a manifest after changing a sealed file.
4. Each fix that can be reached has a regression test that fails without it,
   and the golden vectors are unchanged.

## Findings

### W-028 (low): S-016's test checks the predicate, not the fix

- Where: `crates/neoblast/src/store.rs`, `full_sync` and
  `s016_a_failed_full_sync_is_not_hidden` (#23).
- What: the test checks only `falls_back`. A fresh agent changed the arm
  for an error that does not fall back to run the plain `fsync` anyway
  (`Err(e) if !falls_back(e) => Ok(retry_on_intr(|| rustix::fs::fsync(file))?)`):
  clippy and every test still passed, with S-016's bug back. Deleting the
  arm outright is caught, but only by clippy's dead-code lint.
- Remediation: pass the full sync's result and the plain `fsync` into a
  helper, and test that an `EIO` is returned without the `fsync` running and
  that `ENOTSUP` runs it. A real `EIO` cannot be made in a test.
- Verified: by two fresh agents, by mutation.
- Status: open.

### W-029 (cleanup): a codec's result is checked only for its count, and only in heal

- Where: `crates/neoblast-erasure/src/lib.rs`, `regenerate` and
  `reconstruct_data`; `crates/neoblast/src/stream.rs`, `heal_segments` (#23,
  W-027).
- What: a codec whose `restore` returns too few shards makes
  `reconstruct_data` report `NotEnough` with enough shards, which heal
  reports as `Unrecoverable { have: 2, need: 2 }`, blaming the store. n
  shards of the wrong length pass W-027's count check and are reported as
  `RebuildMismatch`, blaming the publisher. W-027's message says "too few"
  while it checks `!= n`.
- Remediation: check the count and length of every codec result in
  `neoblast-erasure` and return `Codec`; heal's own check can then go.
- Verified: by a fresh agent, with fake codecs in a copy. `ReedSolomonSimd`
  is the only codec and heal names it directly; the proptest and the
  `erasure_restore` fuzz target assert full equality, so nothing reaches it
  today.
- Status: open.

### W-030 (cleanup): two copies of the same test helpers

- Where: `crates/neoblast/tests/review_regressions.rs`: `Recorder` and
  `sealed_as` (#23) repeat the `Sink` and the manifest rebuild in
  `s011_an_inconsistent_segment_does_not_stop_heal`.
- Remediation: `s011` uses `Recorder::new(false)` and `sealed_as`.
- Verified: by a fresh agent.
- Status: open.

## Not tracked

Checked by the same review and left as they are, with the reason:

- `seal_stream` calls `ShardSink::finish` after a failed run (W-025): the
  documented meaning is "make every shard stored so far durable", not a
  commit; `finish` is new in 0.0.2, `heal_manifest` already did the same,
  and the sync is bounded by the last segment's directories.
- An `EIO` from an `F_FULLFSYNC` that a file system does not offer would now
  fail (S-016): xnu's default answer is `ENOTSUP`, Apple's smbfs answers
  `ENOTSUP` for directories, the open-source FUSE kext passes the request
  to the daemon's fsync, and probes on APFS, HFS+, FAT32, exFAT, FSKit and
  devfs gave no `EIO`. Closed-source macFUSE could not be checked; a report
  of one would reopen it.
- `write_manifest_bytes`'s `Unsynced` can never hold anything: W-026 says so;
  it guards a later change to `create_dirs`.
- The CHANGELOG dates 0.0.2 and claims a campaign, which ran on `86b8a67`,
  not `c3d405d`: step 7d‴ re-dates it and re-checks that sentence against
  7d″'s records.
