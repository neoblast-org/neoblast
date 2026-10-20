# Code-review worklist (W-series, part 2)

Status: completed 2026-10-20: every item is fixed, the last in the
campaign-lock PR after #21 (W-020, W-021). Opened 2026-10-20 by the fourth
`/review-since` (`5393f67..9bf18dc`, the v0.0.2 candidate), logged in
`docs/logs/code-review-log.md`. Part 1, W-001 to W-014, is closed
(`plans/completed/code-review-w-series.md`). All items are low or cleanup;
none blocks v0.0.2. Those under `crates/` are fixed before the v0.0.2
campaign, since a later change would restart it.

IDs are unique across all worklists and continue from the highest W-number used.

## Success criteria

1. W-015: `seal` makes a new store in a directory that can be written to but
   not read (mode 0333), and succeeds. A regression test does it.
2. W-016: a seal into a new store syncs the store's own directory once for all
   the segment directories made in it, plus once for the manifest, whatever
   the number of segments; the W-007 test counts it.
3. W-017: `heal_stream` calls `finish` at the end of every run, also one that
   fails, and `ShardSink::finish`'s doc says when it is called.
4. W-018: an error about a directory that could not be made or synced names
   that directory, not a shard.
5. W-019: `tree::segment_root` returns `None` for more than 65,536 hashes, in
   debug and release builds, instead of overflowing.
6. Each fix has a regression test that fails without it, and the golden
   vectors are unchanged.
7. W-020 to W-023 are fixed, or left open with the reason.

## Findings

### W-015 (low): seal into a new store under an unreadable parent failed

- Where: `crates/neoblast/src/store.rs`, `create_dirs` and `sync_dir` (#13).
- What: making a store syncs the new directory into its parent, which means
  opening the parent, and that needs read permission. A shared drop box
  (mode 0733, as macOS's `~/Public/Drop Box`) can be written to but not read,
  so a seal into a new store there failed with "Permission denied", reported
  against the first shard. v0.0.1 sealed there.
- Remediation: a parent outside the store that cannot be read is not synced.
- Verified: by a fresh agent, from the code.
- Status: fixed in #20. Regression tests:
  `w015_seal_into_an_unreadable_parent`, and
  `w015_an_unreadable_store_directory_is_not_skipped` for the store's own.

### W-016 (low): a new seal paid two full flushes per segment

- Where: `crates/neoblast/src/store.rs`, `create_dirs` (#13); the claim in its
  module doc, the CHANGELOG and #18.
- What: each new segment directory synced the store's directory at once, and
  the previous segment directory was synced as the writes moved on: two
  `F_FULLFSYNC`s per segment on macOS. An estimate, not measured: about
  1,040 more flushes for 1 GiB at 1 MiB segments, at the 4.5 ms one cost in
  #18's measurements, is about 4.6 s of a 33 s seal.
- Remediation: a directory that new directories were made in is synced once,
  at the end of the run.
- Verified: by a fresh agent, from the code.
- Status: fixed in #20. Regression test: the count in
  `store::tests::w007_every_write_is_synced`.

### W-017 (cleanup): `finish` did not run on every failed heal

- Where: `crates/neoblast/src/stream.rs`, `ShardSink::finish`'s doc and
  `heal_manifest`'s early returns (#13).
- What: the doc said `finish` runs after a successful run, but heal also
  called it after failed ones, except on the paths that returned early, which
  left the last directory unsynced.
- Remediation: heal calls `finish` at the end of every run past its root
  check, and the doc says so; `DirStore::write_sealed` and `write_rebuilt`
  sync what they wrote before returning an error too. The early return for
  an error that means a bug (not a bad store) has no test, as no store can
  cause it.
- Verified: by a fresh agent, from the code.
- Status: fixed in #20. Regression test: the `finish`
  check in `s011_an_inconsistent_segment_does_not_stop_heal`.

### W-018 (cleanup): a failed directory sync was reported as "cannot write"

- Where: `crates/neoblast/src/store.rs`, `write_shard` and
  `write_manifest_bytes` (#13).
- What: an error making or syncing a directory was reported as
  `cannot write <shard or manifest>`.
- Remediation: such errors read `cannot create <dir>` or `cannot sync <dir>`.
- Verified: by a fresh agent (partly: the segment directory's own sync was
  already named).
- Status: fixed in #20. Regression test:
  `w018_a_directory_that_cannot_be_made_is_named`.

### W-019 (cleanup): `tree::segment_root` overflowed its shard index

- Where: `crates/neoblast/src/tree.rs`, `segment_root_of` (predates v0.0.1).
- What: 65,536 or more hashes overflowed the 16-bit index: a panic in debug
  builds, a wrong root in release builds. A verified header has at most 128
  shards, so only an outside caller of the public function could reach it.
- Remediation: `None` beyond 65,536 hashes.
- Verified: by a fresh agent, with a test program.
- Status: fixed in #20. Regression test:
  `tree::tests::w019_too_many_shard_hashes_have_no_root`.

### W-020 (low): `campaign.sh` can run two campaigns after a stale lock

- Where: `fuzz/campaign.sh`, the lock (#12).
- What: taking over a stale lock is not atomic, so two starts at once can both
  run; and a campaign killed with SIGKILL leaves its fuzzers running while its
  lock looks stale.
- Impact: CPU contention and shared corpora; a crash an orphan finds could be
  recorded against the new run, which fails safe. One maintainer runs it.
- Remediation: remove a stale lock and retry the atomic create; refuse to
  start while a fuzzer for the same targets is running.
- Verified: by a fresh agent, from the script.
- Status: fixed in the campaign-lock PR after #21, before the v0.0.2
  campaign. The lock is now a `flock` on a descriptor the fuzzers inherit, so
  it is taken atomically, held until the last fuzzer exits even if the script
  is killed, and freed with nothing to delete. The build and the other
  commands that could leave a server running (git, rustc) run without the
  descriptor. Tested on the lock alone, with a child process in place of a
  fuzzer, on macOS (bash 3.2) and, by a fresh agent, on Linux (bash 5.2): a
  second start, and one after a SIGKILL while the child still ran, were
  refused; once it exited, a start worked; of 20 starts at once, one ran. The
  whole script is checked on the fuzzing Mac before the 24 h run.

### W-021 (cleanup): `code_id` does not cover the compiler

- Where: `fuzz/campaign.sh`, `code_id` (predates v0.0.1).
- What: the hours add up across runs with the same sources whatever nightly
  built them; each record names its compiler, but `summary` does not compare
  them, and the `cargo-fuzz` version is not recorded.
- Remediation: have `summary` warn when the counted records name different
  compilers, and record `cargo-fuzz`'s version.
- Verified: by a fresh agent.
- Status: fixed in the campaign-lock PR after #21: records name the
  `cargo-fuzz` version and the compiler, both read once after the build, and
  `summary` warns, per target, when the counted
  runs on the current code name more than one compiler. The hours still add
  up; the nightly is still left alone while a campaign runs, and step 7d pins
  CI to the compiler the records name.

### W-022 (cleanup): the CHANGELOG's new timings had no record

- Where: `CHANGELOG.md`, [Unreleased], the W-007 entry (#18).
- What: sealing 1 GiB in about 33 s and healing it in about 24 s came from the
  measurements in #18's description, not a committed record.
- Remediation: the entry says where they come from.
- Status: fixed in #20.

### W-023 (cleanup): the README said memory does not grow with the file

- Where: `README.md` (#17).
- What: heal's allocated memory grows only by the manifest, but its measured
  peak steps up about 5 MiB between a 256 MiB and a 1 GiB file (decision 0009,
  finding 2).
- Remediation: say "the memory they allocate" and mention the step.
- Status: fixed in #20.
