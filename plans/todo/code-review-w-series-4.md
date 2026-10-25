# Code-review worklist (W-series, part 4)

Status: todo. Opened 2026-10-25 by the sixth `/review-since`
(`255353b..c3d405d`, #23, #25 and #26), logged in
`docs/logs/code-review-log.md`, while the v0.0.2 campaign (step 7d″) ran on
`c3d405d`. Part 3, W-024 to W-027,
is closed (`plans/completed/code-review-w-series-3.md`). All items are low or
cleanup; none blocks v0.0.2. W-028 to W-030 each change `crates/`, and so every target's
`code_id`, so they wait until after v0.0.2 is tagged. The seventh
`/review-since` (`c3d405d..118f710`, the review PR after #26) added W-031,
which is outside `code_id` and can land before the tag.

IDs are unique across all worklists and continue from the highest W-number used.

## Success criteria

1. W-028: making `full_sync` fall back after an `EIO`, by any edit, fails a
   test.
2. W-029: a codec result of the wrong count or length is a `Codec` error
   wherever `neoblast-erasure` returns shards, never a store's or a
   publisher's fault.
3. W-030: `review_regressions.rs` has one recording sink and one way to
   rebuild a manifest after changing a sealed file.
4. W-031: the tag object is checked by one awk program, which the hygiene
   job and the Release workflow both run.
5. Each fix that can be reached has a regression test that fails without it,
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

### W-031 (cleanup): the tag check is written out twice

- Where: `.github/workflows/hygiene.yml`, "Tags carry only the project's
  identity", and `release.yml`, "Tag is the project's, in UTC" (`118f710`,
  S-022).
- What: the same 11-line awk program is in both steps. A later change to
  one, such as S-026's signature check or S-025's control bytes, can miss
  the other, and the hygiene job and the release gate then disagree about
  what a tag may hold.
- Remediation: move it to a checked-in file, such as
  `.github/scripts/tag-object.awk`, run by both with `awk -f`; the same
  for the commit program if S-025 grows it.
- Verified: by a fresh agent; the two copies are byte-identical.
- Status: fixed in `14989d0`. Both workflows run
  `.github/checks/object.sh`, which runs `commit-object.awk` or
  `tag-object.awk`; the programs are the old ones line for line, with
  S-025's line added. An object of only a blank line passes, as it did
  before; git never writes one.
- Tests, after `14989d0`: the attribution check (S-013, S-018) moved to
  `message.awk`, run as `object.sh message`, as the old program line for
  line. `test.sh` runs 107 cases under every awk on PATH (mawk on
  GitHub's runner, BSD awk on macOS, gawk where installed): crafted commits, tags and messages, each
  differing from an allowed one in one way, and 14 objects git itself
  writes in a throwaway repository with a throwaway SSH key (a signed
  commit, tag and merge pass; a `mergetag`, an `encoding` header, a
  commit not in UTC, a tag of a tag, a two-line tag message, another
  tagger and a co-author, as a trailer or in prose, fail). CI fails if it
  cannot make those, and they cannot reach another repository through
  `GIT_DIR` and the like. `mutate.sh` holds 47 mutants, each bringing
  back a weakness a review closed; it fails if the checks as they are do
  not pass, or if a mutant survives or no longer applies. The hygiene job
  runs both. The mutants it cannot hold, as no
  test tells them apart, are listed in its header.

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
- From the seventh review (`c3d405d..118f710`):
  - No regression test for S-021 to S-023: no test exists for a workflow
    check, and S-012 to S-017 and S-022 were checked by hand on crafted
    objects, as here. Since W-031, the object checks of S-015, S-017,
    S-022 and S-025 have one (`.github/checks/test.sh`).
  - One `git cat-file` per commit in the header step: 18 commits today;
    one `--batch` stream would do when the history grows.
  - The header step's author and committer pattern repeats the identity
    step's rule: one reads git's parsed fields and the other the object as
    stored, on purpose.
  - Rejected: the log's and the worklist's counts for the sixth review
    were said not to add up; they do, as W-028 came from the security
    review.
  - Rejected: the CHANGELOG's `cargo update -p ctutils --precise 0.4.5`
    for a `rust-version` below 1.87 was said to be wrong; `ctutils` 0.4.3
    to 0.4.5 all need Rust 1.87, so resolver 3 keeps 0.4.2 without it.
  - Rejected: `verify`'s `deny` reads only the root lock file; that is the
    graph that is published, and CI's `deny` job did the same before the
    range.
