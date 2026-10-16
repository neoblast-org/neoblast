# Code-review worklist (W-series)

Status: todo. Opened 2026-10-14 by the first `/review-since` (empty tree to
`e24f453`). Neither item blocked v0.0.1 on its own, but both are fixed in the
pre-release code change that S-001 required (#6), since that change
restarts the fuzz campaign anyway (`plans/completed/phase-1-finish.md`, A0).

2026-10-15: the second `/review-since` (`e24f453..a9445d9`) added W-003 to
W-012, all low or cleanup, none blocking v0.0.1. Every one except W-012 needs a
change under `crates/`, and `fuzz/campaign.sh` takes the `code_id` from the
whole `crates/` tree, tests included, so they wait for v0.0.2. W-012 is in
`bench/` and can land during the campaign.

2026-10-16: the third `/review-since` (`a9445d9..5393f67`, #8 to #10) added
W-013 and W-014, both outside `crates/`; its CHANGELOG findings are S-009.

## Success criteria

1. W-001: a manifest file whose declared size is consistent with its own header
   but far larger than any plausible file (for example a sparse 16 TiB
   `manifest.nbl`) makes `neoblast open` and `neoblast heal` fail with a clean
   error. Peak memory stays bounded, not proportional to the declared size. A
   regression test plants such a file (sparse, so the test stays cheap). Seal
   refuses, before writing anything, a file whose manifest would exceed the cap.
2. W-002: sealing a file whose segment count would not fit in a `u32` reports
   that the file is too large for the chosen segment size. It does not report
   that the segment size is out of range when that size is in range. A unit
   test in `neoblast-format` covers it.
3. Each fix has a regression test (CLAUDE.md), and the golden vectors are
   unchanged.
4. W-003, W-004: with a directory or a planted `*.tmp` name at one missing
   shard's path, `neoblast heal` still rebuilds every other missing shard in
   every segment, exits non-zero, and names the path it could not write.
5. W-005: `seal` into a non-empty `--out` without `--force` refuses before
   reading any input, also from a pipe; `--force` still keeps the old manifest
   when the new one would be too large.
6. W-006: `seal_stream` refuses parameters whose manifest would exceed
   `MAX_MANIFEST_LEN` before writing any shard.
7. W-007: seal and heal sync each file before renaming it into place, and
   each parent directory (the store root and the segment directories) after.
8. W-008: a test fails if `write_shard`'s symlinked-directory guard is removed.
9. W-009: `--k 0` reports invalid k/n, and too many segments reports the
   `--segment-size` hint.
10. W-010, W-011: the cap is checked before the manifest is encoded, and one
    `scratch` helper lives in `tests/common/mod.rs`.
11. W-012: `bench/heal-check.sh` prints seal's or heal's error when either
    fails.

## Findings

### W-001 (low): a planted manifest can make open and heal allocate without bound

- Where: `crates/neoblast/src/store.rs:50-72` (`DirStore::read_manifest`; doc comment at :47-49).
- What: the size check compares the file's length with the size its *own*
  header declares, then reads all of it into memory. Only after that does
  `Manifest::decode` check the bytes against the link's root. A valid header
  with `segment_count` near `u32::MAX` and `n = 128` declares about 2^44 bytes.
  A sparse file of that size costs the attacker nothing, and the CLI tries to
  read it into RAM. The doc comment says "a planted oversized manifest is
  refused before it is read", which holds only for a file that disagrees with
  its own header.
- Impact: denial of service (memory exhaustion or allocation abort) for anyone
  running `open` or `heal` on a directory someone else prepared. No integrity
  or confidentiality impact: a wrong manifest is still rejected by the root
  check.
- Remediation: cap the manifest size before reading, with an explicit override
  for larger files, and correct the doc comment. Hashing while streaming is not
  enough: it still reads up to 16 TiB before failing, so it misses criterion 1's
  clean error and the cheap sparse-file test.
- Verified (fresh agent, 2026-10-14): confirmed at `e24f453` with the release
  CLI. A sparse manifest declaring 2 GiB made `heal` peak at 2.5 GB before the
  root check failed; a sparse 15.9 TiB one grew memory by about 0.5 GB/s until
  killed. `Manifest::decode` (`manifest.rs:64-87`) makes a second full copy.
- Status: fixed in `9a542da` (#6).

### W-002 (low): wrong error when a file has too many segments

- Where: `crates/neoblast-format/src/header.rs:104-105` (`Header::new`).
- What: when `layout::segment_count` returns `None` because the count does not
  fit in a `u32`, the error is `HeaderError::SegmentSize(segment_size)`. Its
  message says the segment size is outside `64..=67108864`, even for an
  in-range size. Example: `--segment-size 64` with a 300 GiB input.
- Impact: misleading CLI message only. The header is correctly refused.
- Remediation: return the existing `HeaderError::Layout` when the size is in
  range but the count overflows. A new variant would change the public API of
  `neoblast-format`, whose `HeaderError` is not `#[non_exhaustive]`.
- Verified (fresh agent, 2026-10-14): confirmed; `seal` of a sparse 300 GiB file
  with `--segment-size 64` printed "segment size 64 outside 64..=67108864".
- Status: fixed in `9a542da` (#6). The message is now "layout fields are
  inconsistent", which is correct but unhelpful; see W-009.

## Second review (2026-10-15, `e24f453..a9445d9`)

Each finding below was checked by a fresh agent at `a9445d9`: reproduced, cut
by mutation, or (W-006, W-007, W-010, W-011) confirmed by reading the code.

### W-003 (low): one failed shard write ends the whole heal

- Where: `crates/neoblast/src/stream.rs:237` (`sink.put(segment, index, bytes)?`).
- What: an unrecoverable segment is skipped and reported at the end
  (`stream.rs:220-226`), but a failed write returns at once. A directory planted
  at `seg-000000/000.shard` makes every heal fail with "Is a directory", and
  later segments are never repaired. The doc comments at `stream.rs:194-199` and
  `file.rs:151-152` say one bad peer cannot block repair; the store's supplier is
  that peer here.
- Impact: denial of local repair, visible in the exit code. No shard is lost
  and `open` still works from any k good shards.
- Remediation: record the first write error and continue, like `first_failure`
  for reads; put the path in the error.
- Verified: reproduced with the release CLI. Pre-existing (`e24f453` has the
  same `?`).
- Status: fixed in #13: a failed write is recorded and heal moves on, and the
  error names the path; a segment found unrecoverable later still outranks it.
  Regression tests: `w003_an_unwritable_shard_does_not_stop_heal` and
  `w003_a_lost_segment_outranks_a_failed_write`.

### W-004 (low): planted temporary-file names make heal's writes fail

- Where: `crates/neoblast/src/store.rs:178-180` (`replace_file`).
- What: the temporary file is `<path>.<pid>.tmp`, opened with `create_new`. A
  supplier who plants every PID's name for the shard heal will write first
  (about 100k empty files on macOS) makes the write fail with "File exists";
  through W-003 the whole heal stops.
- Impact: as W-003. Nothing is followed or truncated.
- Remediation: a random suffix with a retry on `AlreadyExists`; W-003's fix
  also stops it ending the heal.
- Verified: reproduced with the release CLI. Introduced in #6.
- Status: fixed in #13: temporary names carry a random 64-bit suffix, retried
  on `AlreadyExists`. Regression test: `w004_planted_temporary_names_do_not_stop_heal`.

### W-005 (low): seal reads a whole pipe before refusing a non-empty store

- Where: `crates/neoblast/src/main.rs:91-111` (`run`, the `Command::Seal` arm).
- What: #6 moved the `is_empty`/`--force` check after reading piped input, so
  that `--force` removes the old manifest only once the new one is known to fit.
  Only `remove_manifest` needs to wait; the refusal does not.
- Impact: `head -c 200000000 /dev/zero | neoblast seal /dev/stdin --out <store>`
  peaked at 202.5 MB before refusing (2.3 MB at `e24f453`); an endless pipe runs
  out of memory.
- Remediation: refuse a non-empty store without `--force` first, then read and
  run `check_manifest_fits`, then `remove_manifest`.
- Verified: reproduced on both commits. Introduced in #6.
- Status: open (v0.0.2).

### W-006 (low): the library checks the manifest cap only after writing every shard

- Where: `crates/neoblast/src/stream.rs:52-59` (`seal_stream_with_key`);
  `crates/neoblast/src/store.rs:129-132` (`write_manifest`) and `144-152`
  (`checked_len`).
- What: `check_manifest_fits` is private to `main.rs`. A library caller of
  `seal_stream` into a `DirStore` writes every shard, then `write_manifest`
  refuses the manifest. `manifest_len` and `MAX_MANIFEST_LEN` are public, but
  nothing makes callers use them.
- Impact: wasted work and a store with no manifest; the CLI is not affected.
- Remediation: refuse in `seal_stream` (or a public `Params` check) before the
  first shard.
- Verified: confirmed by reading; `write_sealed` already checks first. New in #6.
- Status: open (v0.0.2).

### W-007 (low): store writes are not synced before rename

- Where: `crates/neoblast/src/store.rs:182` (`replace_file`).
- What: no `sync_all` before the rename (`write_atomically`, `main.rs:190`, has
  one), and no directory sync after. After a power loss shortly after `seal`
  prints the link, shards or the manifest can be empty.
- Impact: a user who deleted the original right after sealing could lose it.
  Heal rebuilds a torn shard on its next run; a torn manifest is not rebuilt.
- Remediation: sync the temporary file before renaming, and the store root and
  segment directories after; until then, `CHANGELOG.md` lists it as a known issue.
- Verified: confirmed by reading. Pre-existing (`e24f453` used `fs::write`).
- Status: fixed in #13: each file is synced before its rename, each segment
  directory once after its last write (`ShardSink::finish`), and each new
  directory into its parent. Test: `store::tests::w007_every_write_is_synced`.
  Sealing 256 MiB on the reference Mac went from about 2-4 s to 14.5 s, almost
  all of it one full flush (`F_FULLFSYNC`) per shard.

### W-008 (low): the symlinked-segment-directory test never reaches the write guard

- Where: `crates/neoblast/tests/review_regressions.rs:228-251`
  (`s001_heal_refuses_a_symlinked_segment_directory`).
- What: reads through the symlinked directory fail first, so the segment is
  unrecoverable and heal never writes. With `write_shard`'s guard
  (`store.rs:158`) disabled, every test still passes. The comment at lines
  236-237 says heal would write the fourth shard there, which it would not.
  The guard is reachable by `seal --force` into a store with a planted segment
  symlink (without it, 4 shards are written outside) and by `heal_stream` with
  a separate sink.
- Impact: none today; a regression in the guard would go unnoticed.
- Remediation: a regression test through `seal_stream` or a separate sink; fix
  the comment.
- Verified: mutation run. Test gap only; the code is correct.
- Status: fixed in #13: `w008_seal_never_writes_through_a_symlinked_segment_directory`
  fails with the guard removed, and the S-001 test's comment is corrected.

### W-009 (low): unhelpful errors for `--k 0` and too many segments

- Where: `crates/neoblast-format/src/header.rs:110` (`Header::new`);
  `crates/neoblast/src/main.rs:151-162` (`check_manifest_fits`).
- What: `Header::new` computes the layout before checking k/n, so `--k 0` says
  "layout fields are inconsistent" (pre-existing). A 300 GiB file with
  `--segment-size 64` now gets the same message from `check_manifest_fits`
  instead of the `--segment-size` hint, and the "invalid parameters:" prefix
  is gone.
- Impact: message only; headers from a peer go through `decode`, which checks
  k/n first.
- Remediation: in `main.rs`, check k/n before `Header::new` and map
  `HeaderError::Layout` to the hint.
- Verified: reproduced with the release CLI on both commits.
- Status: open (v0.0.2).

### W-010 (cleanup): `write_manifest` encodes before checking the cap

- Where: `crates/neoblast/src/store.rs:118,131,144` (`checked_len`).
- What: the manifest is encoded and then measured; `manifest_len(header)` gives
  the same answer first.
- Impact: a wasted encoding, for library callers only. The CLI cannot reach
  it, since `check_manifest_fits` runs before.
- Remediation: check `manifest_len(header)` before encoding.
- Verified: confirmed by reading; `encode` is exactly `manifest_len` bytes.
  Introduced in #6.
- Status: open (v0.0.2).

### W-011 (cleanup): duplicated test helper

- Where: `crates/neoblast/tests/review_regressions.rs:171-177` and
  `crates/neoblast/tests/cli.rs:16-21` (`scratch`).
- What: the same scratch-directory helper in two test files, although
  `tests/common/mod.rs` exists for shared helpers.
- Impact: the copies can drift apart.
- Verified: confirmed by reading. Introduced in #6.
- Remediation: move it to `tests/common/mod.rs` and add `mod common;` to
  `cli.rs`.
- Status: fixed in #13: `scratch` lives in `tests/common/mod.rs`.

### W-012 (low): `bench/heal-check.sh` fails silently

- Where: `bench/heal-check.sh:34,42`.
- What: seal's stderr goes to `/dev/null`, and heal's to a file the exit trap
  deletes, so a failure prints only the table header. `bench/segment-size.sh`
  prints the captured output and a message.
- Remediation: follow `segment-size.sh` (`|| { cat …; exit 1; }`).
- Verified: reproduced with a stub binary.
- Status: fixed in #10 (seal and heal errors are printed; a missing
  `/usr/bin/time` or an unsupported OS is reported). Checked by forcing a seal
  failure and with a stub binary; `bench/` has no test harness, so criterion
  3's regression test applies to fixes under `crates/` only.

## Third review (2026-10-16, `a9445d9..5393f67`)

Each finding below was checked by a fresh agent at `5393f67`.

### W-013 (cleanup): `bench/heal-check.sh` assumes the default 20-of-50

- Where: `bench/heal-check.sh:42` (seal without `--k`/`--n`) and `:51`
  (deletes `range(30)` shards).
- What: the script seals with the default parameters but deletes a fixed
  n − k = 30 shards. `bench/segment-size.sh` pins `k=20 n=50` and derives the
  deletions from them. The two scripts also repeat the same OS check, time
  check, build step and RSS parsing.
- Impact: if the defaults change, heal-check fails, or tests less than the
  worst case.
- Remediation: pass `--k 20 --n 50` and derive the deletions; optionally move
  the shared steps into a sourced `bench/lib.sh`.
- Verified: confirmed by reading. From #5.
- Status: fixed in #12 (20-of-50 pinned and the deletions derived from it;
  the shared steps stay duplicated, as in `segment-size.sh`).

### W-014 (low): `fuzz/campaign.sh summary` counts overlapping runs twice

- Where: `fuzz/campaign.sh:49` (sums `seconds_run` over every clean or stopped
  record with the same `code_id`).
- What: two stopped campaigns that ran at the same time on 2026-10-15 overlap
  by about 1.3 h per target, and `summary` adds both. It shows 27.2 h where
  about 25.9 h of wall-clock time ran. Nothing stops two campaigns starting at
  once.
- Impact: the hours shown can overstate the fuzzing done. The v0.0.1 gate is
  unaffected, since each target's clean 24 h run meets it alone.
- Remediation: merge overlapping intervals per target (or count only clean
  runs toward the gate), say which in `fuzz/README.md`, and refuse to start a
  second campaign while one is running.
- Verified: confirmed against the records and `campaign.sh`.
- Status: fixed in #12 (overlapping runs count once, 25.9 h for v0.0.1; a
  second campaign on the same clone is refused).
