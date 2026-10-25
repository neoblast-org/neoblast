# Security review worklist: S-series

Status: todo. Opened 2026-10-14 by the first security review (empty tree to
`e24f453`), logged in `docs/logs/security-review-log.md`. The second review
(2026-10-15, `e24f453..a9445d9`) added S-005 to S-007, all low and none
blocking v0.0.1. The third review (2026-10-16, `a9445d9..5393f67`) added S-008
and S-009. The fourth (2026-10-20, `5393f67..9bf18dc`, the v0.0.2 candidate)
added S-010 to S-015, all low. The fifth (2026-10-21, `9bf18dc..255353b`,
#20 to #22) added S-016 to S-020, all low. The sixth (2026-10-25,
`255353b..c3d405d`, #23, #25 and #26) added S-021 to S-024, all low. The
seventh (2026-10-25, `c3d405d..118f710`, the review PR after #26) added S-025
and S-026, both low.

IDs are unique across all worklists and continue from the highest S-number used,
including one that so far appears only in a review log (`SECURITY.md`, "Our own
findings").
Each item has a status (open / fixed / won't fix) and, once fixed, the commit
that fixed it.

## Medium

### S-001 · Medium · `DirStore` writes through symlinks and opens special files in an untrusted shard directory

- **Where:** `crates/neoblast/src/store.rs:119-124` (`write_shard` uses
  `fs::write`, which follows symlinks), `crates/neoblast/src/store.rs:128-136`
  (`get` opens any entry type), reached from `crates/neoblast/src/main.rs:142`
  (`neoblast heal`).
- **Problem:** `ShardSource` treats shard bytes as untrusted, and `get` already
  defends against planted oversized files, so the directory is in the hostile
  model. But if `seg-NNNNNN/III.shard` is a symlink, `get` follows it, sees the
  wrong size and reports the shard missing; heal then rebuilds it and
  `fs::write` truncates and overwrites the symlink's target. Heal never
  decrypts, so a hostile publisher chooses the rebuilt bytes: they pick any data
  shard content, compute parity and the manifest with the public API, ship the
  directory with that shard replaced by a symlink (to `~/.bashrc`, say), and
  `neoblast heal` writes their bytes there (length a multiple of 64; padding
  with comment characters makes a script). A FIFO in place of a shard makes
  `File::open` block forever, so heal or open hangs.
- **Verified:** reproduced with the CLI on 2026-10-14: seal 2-of-4, replace
  `seg-000000/000.shard` with a symlink to a file outside the store, run
  `neoblast heal`. The outside file was overwritten with the rebuilt shard.
  A fresh agent then reproduced it independently (heal reported "rebuilt 1
  shard(s)", exit 0; the target now held the shard's bytes) and the FIFO hang
  (killed by a 5 s alarm). It also found the bug is wider: a dangling symlink
  makes heal create a file wherever the user can write, and a symlinked `seg-*`
  directory redirects every write, because `create_dir_all` follows it.
- **Fix:** in `DirStore`, refuse anything that `symlink_metadata` does not
  report as a regular file before reading (this also refuses FIFOs; `O_NOFOLLOW`
  would need the `libc` crate, which the crate does not use). Write shards to a temporary file in the segment
  directory and `rename` it into place (rename replaces a symlink, never its
  target). Also refuse a `seg-*` directory that is a symlink. Add a regression
  test to `crates/neoblast/tests/review_regressions.rs` for each case: a
  symlinked shard (target untouched), a dangling symlink (nothing created) and a
  symlinked `seg-*` directory.
- **Release:** blocks v0.0.1 (recommended). The fix is in `crates/`, so it
  changes every target's `code_id` and needs a new 24 h campaign. The
  alternative is to ship v0.0.1 with this listed as a known issue and fix it in
  v0.0.2; that is the maintainer's call.
- **Status:** fixed in `9a542da` (#6). Regression tests: the `s001_*` tests in
  `crates/neoblast/tests/review_regressions.rs`, each confirmed to fail on
  `e24f453`.

## Low

### S-002 · Low · A publisher can make different shard subsets decode to different content

- **Where:** `crates/neoblast/src/stream.rs:141-153` (open decodes from the
  first k shards that match their hashes), `crates/neoblast/src/stream.rs:218-236`
  (heal checks only the shards it rebuilds).
- **Problem:** the root commits to each shard's hash, but nothing checks that
  the n shards form one Reed-Solomon codeword. A publisher holds the read key,
  so they can encrypt two plaintexts under the same nonce, use one for the data
  shards and derive the parity from the other. With n − k ≥ k (the default
  20-of-50), a reader who gets only parity shards, for example because the data
  shards were censored, opens different content from a reader who gets the data
  shards. Any mixed subset fails authentication. The link alone does not pin a
  single plaintext.
- **Fix:** no format change is needed. Either check every parity shard against
  the manifest when sealing is verified and when healing (re-encode from the
  decoded data and compare all n hashes, then refuse the file as inconsistent),
  or state in `spec/threat-model.md` that a link pins shard bytes, not a unique
  plaintext against its own publisher.
- **Note (2026-10-21):** heal and open rebuild from the first k shards that
  match the manifest. In an inconsistent segment another k could rebuild a
  shard that matches, so heal reports `RebuildMismatch` (S-011) where a
  different choice would have repaired it. Only the manifest's author can
  cause this, and the fix above covers it.
- **Release:** not blocking.
- **Status:** open.

### S-003 · Low · The read key passes through argv and unwiped copies

- **Where:** `crates/neoblast/src/main.rs:45-47` (`neoblast open <link>` takes
  the read link as an argument), `crates/neoblast-format/src/caps.rs:120-124`
  (`decode` builds an uppercase `String` and a `Vec` of the key that are not
  zeroized).
- **Problem:** command-line arguments can be seen by other local users via `ps`
  and end up in shell history. That does not break the rule that secrets
  appear only after `#`, but it puts the key in more places than needed.
  `FileReadCap` itself is `ZeroizeOnDrop`.
- **Fix:** let `open` read the link from stdin or a file (keeping the argument
  for convenience, with a warning in `--help`), and decode the key fragment into
  `Zeroizing` buffers.
- **Release:** not blocking.
- **Status:** open.

### S-004 · Low · The nightly fuzz job installs an unpinned `cargo-fuzz`

- **Where:** `.github/workflows/fuzz.yml:23` (`cargo install cargo-fuzz --locked`).
- **Problem:** each night's run takes whatever `cargo-fuzz` version is newest,
  so it is not pinned to an exact version. (Verification found it is not the
  only one: the same job's `toolchain: nightly` floats too.) The job has
  `contents: read` and no secrets, so the exposure is small.
- **Fix:** `cargo install cargo-fuzz --version <exact> --locked`, with
  Dependabot or a manual bump.
- **Release:** not blocking (CI only; not in `code_id`).
- **Status:** fixed in #12: `cargo-fuzz` 0.13.2 and `nightly-2026-10-14`, the
  compiler of the v0.0.1 campaign.

### S-005 · Low · Rejecting a planted manifest can cost about 3.2 times its size in memory

- **Where:** `crates/neoblast/src/manifest.rs:76-84` (`Manifest::decode`
  builds a list of hashes per segment before comparing with the link's root at
  `:85`) and `:92-102` (`compute_root` collects the segment roots and the
  content-root leaves); reached from `open` and `heal` via `read_manifest`
  (`crates/neoblast/src/store.rs:86-88`) and `Manifest::decode` at
  `crates/neoblast/src/stream.rs:131` and `:206`.
- **Problem:** W-001's cap limits the manifest file to 256 MiB, but decoding
  allocates per segment before the root check. A store's supplier picks the
  header, so it can choose the smallest n (3): a sparse
  `manifest.nbl` just under the cap, a few KiB on disk, made `heal` peak at
  about 822 MiB before it failed with "manifest does not match the link's root".
  At the default 20-of-50 and 4 MiB segments the same size costs about 556 MiB.
  The `MAX_MANIFEST_LEN` doc comment suggests the cap bounds memory at 256 MiB.
- **Impact:** bounded memory use, under 1 GB whatever the header says, before
  a clean error. An honest file sealed with the same parameters costs the same,
  so the store's supplier cannot force more than the largest legitimate file.
  It matters more once peers send manifests over the network (Phase 2).
- **Fix:** compute the segment roots and the content root from the raw bytes
  and compare with the expected root before building any per-segment list, or
  keep the hashes flat; correct the doc comment; add a regression test.
- **Verified:** reproduced twice by a fresh agent with the release CLI at
  `a9445d9`.
- **Release:** not blocking (a `crates/` change; v0.0.2).
- **Status:** fixed in #15. `Manifest::decode` computes the root from the
  raw bytes, one segment at a time and one tree node per level, and compares
  it before listing any hash; a manifest that passes keeps its hashes in one
  flat list, and `read_manifest` reserves exactly the declared size. On the
  reference Mac the planted manifest above (264 MB) now peaks `heal` at
  267 MB, 1.01 times its size, against 630 to 862 MB before (3 runs each).
  Regression test: `manifest::tests::s005_the_root_is_compared_before_the_bytes_are_copied`
  (#16 renamed it, and `Manifest` now keeps its encoding instead of a flat
  hash list; it marks the copy, and the peak above covers everything else);
  `tree::tests::builder_matches_level_by_level_pairing` checks the new tree
  builder against the old pairing, and the golden vectors are unchanged.

### S-006 · Low · An unfixed medium finding's exploit details were public before its fix

- **Where:** `plans/todo/security-review-s-series.md`, S-001, published with
  #5 (`b3c3c63`), about 5.5 hours before the fix in #6 (`9a542da`).
- **Problem:** the worklist described S-001 step by step while `main` was
  still vulnerable. `SECURITY.md` asks outside reporters to report privately,
  but says nothing about the maintainers' own unfixed findings.
- **Impact:** none on users: nothing had been released. A later finding could
  be exposed the same way.
- **Fix:** keep open findings of medium severity or higher out of the public
  repo, in a private draft advisory, until the fix merges, and land the
  worklist entry with the fix. Say so in `SECURITY.md` and in the review
  skill. Low items that are deliberate known issues may stay public.
- **Verified:** by reading the history.
- **Release:** not blocking (process and docs).
- **Status:** fixed in the v0.0.2 version-bump PR: `SECURITY.md` ("Our own
  findings") and the `review-since` skill's notes, which the security review
  follows too. One change from the fix above: the worklist entry follows the
  fix's merge instead of landing with it, because every change is first a
  public pull request; that pull request says what changed, not how to
  exploit it.

### S-007 · Low · GitHub's squash merges replace the project's commit metadata

- **Where:** every squash merge on `main`.
- **Problem:** GitHub creates the merge commit itself, so it carries the
  merging account's name, GitHub as committer and signer, and the merger's
  local time-zone offset instead of UTC. The project's own commits are UTC,
  `neoblast maintainers` and signed, but those are not the ones on `main`.
  The offset can reveal a maintainer's time zone, which matters for a
  censorship-resistance project. The pre-push hook cannot catch it, because
  the merge never passes through a client.
- **Impact:** privacy. It also let a maintainer's personal account into
  commit metadata; see S-008.
- **Fix:** merge so that the project's own commits land on `main` unchanged
  (for example a fast-forward of a squashed, UTC-stamped commit), or confirm
  where GitHub takes the offset from and set it to UTC. Rewriting the existing merges would need a force-push
  to `main`, which is the maintainer's call; copies may already exist.
- **Verified:** by reading the commit metadata of all merges on `main`.
- **Release:** not blocking, but decide the merge method before tagging, since
  the release commit will be one of these merges.
- **Status:** open. #9 and #10 were re-made on `main` with the project's
  identity (`36cb370`, `5393f67`); the merges up to #8 keep GitHub's metadata.
  Merging by fast-forward avoids it from #11 on.

### S-008 · Medium · A maintainer's personal account reached public history

- **Where:** GitHub-side merge and branch-update commits made during the
  release.
- **Problem:** the GitHub CLI on the maintainer's machine was logged in to a
  personal account. Actions taken through it on GitHub put that account's name
  and address into commit metadata on `main` and onto GitHub's own records.
- **Impact:** it links a maintainer's identity to the project, which the
  threat model treats as sensitive (an author can be ordered or pressed).
- **Fix:** on 2026-10-16 the maintainer re-made the affected commits on
  `main` with the project's identity, moved the `v0.0.1` tag, re-created the
  release as `neoblast-dev`, and deleted the merged branches. Still to do:
  remove the remaining GitHub-side copies. A CI step (#12) now refuses any
  commit after `5393f67` whose author, committer, co-author or sign-off is not
  the project, in UTC; the pre-push hook cannot see merges made on GitHub.
- **Verified:** by reading the commit metadata and the remote refs.
- **Release:** v0.0.1 is tagged on the corrected commit.
- **Status:** fixed on `main`, and the CI check landed in #12; open until the
  GitHub-side copies are removed. This entry predates the rule in
  `SECURITY.md`, "Our own findings" (S-006), and stays as it is (maintainer's
  call, 2026-10-20).

### S-009 · Low · The 0.0.1 changelog left out that a hostile store can stop `heal`

- **Where:** `CHANGELOG.md`, 0.0.1, Security and Known issues.
- **Problem:** the Security section says a store directory is treated as
  hostile, but the known issues omitted W-003 and W-004 (one planted path
  ends the whole heal), W-005, and that `heal` also skips the disk sync
  (W-007).
- **Fix:** two known issues added (W-003 and W-004 together, and W-005) and
  W-007's widened to `heal`, in the Phase 1 closing PR; the GitHub release
  notes updated to match.
- **Verified:** by comparing the changelog with the worklists and
  `crates/neoblast/src/stream.rs`.
- **Release:** not blocking (docs).
- **Status:** fixed in the Phase 1 closing PR.

### S-010 · Low · On macOS, a volume without full syncs made every seal fail

- **Where:** `crates/neoblast/src/store.rs`, `sync_dir`, since #13.
- **Problem:** a file system that cannot sync directories was meant to be
  accepted as it is. On Apple systems std's `sync_all` is `F_FULLFSYNC`, and a
  file system without it answers `ENOTSUP`, which Rust reports as
  `Uncategorized`, not `Unsupported`; only the latter was accepted. devfs is
  one; some network and FUSE file systems likely are too.
- **Impact:** availability. A seal onto such a volume failed at its first
  shard and left partial shards and no manifest; v0.0.1 worked there. A heal
  wrote its shards but reported a sync error. It fails closed.
- **Fix:** on Apple systems `sync_dir` calls `F_FULLFSYNC` itself and falls
  back to a plain `fsync` on `ENOTSUP`, as SQLite does, and `ENOTSUP` is
  accepted by name.
- **Verified:** by a fresh agent, which reproduced the error kind on devfs.
- **Release:** fixed before the v0.0.2 campaign.
- **Status:** fixed in #20. Regression test:
  `store::tests::s010_a_directory_without_full_sync_is_accepted`.

### S-011 · Low · A publisher's inconsistent segment still stopped `heal` part-way

- **Where:** `crates/neoblast/src/stream.rs`, `heal_manifest`; the behaviour
  predates the range, the claim that it was gone (CHANGELOG, `heal_stream`'s
  doc) came with #13.
- **Problem:** whoever builds the manifest can commit to shards that are not
  one Reed-Solomon codeword (the S-002 setup). A shard rebuilt from the others
  then fails its hash, and heal returned at once: later segments were not
  repaired and the last directory was not synced. Only the manifest's author,
  in practice the publisher, can cause it.
- **Impact:** a publisher can block repair of their own file only.
- **Fix:** such a shard is skipped like an unwritable one, every other segment
  is still repaired, and the segment is reported at the end, ranking with an
  unrecoverable one.
- **Verified:** by a fresh agent, which ran a proof of concept.
- **Release:** fixed before the v0.0.2 campaign. S-002 itself stays open.
- **Status:** fixed in #20. Regression test:
  `s011_an_inconsistent_segment_does_not_stop_heal`.

### S-012 · Low · The hygiene job can print more than hashes

- **Where:** `.github/workflows/hygiene.yml`, steps 1 and 2 (before #12).
- **Problem:** the UTC step prints each failing commit's author and committer
  addresses and dates, and the session-link step the matching lines. A commit
  made as the project but committed elsewhere in local time fails here, before
  the hash-only identity step, and its committer address and offset reach the
  public Actions log.
- **Impact:** privacy. The same data is in the pushed commit, but the log
  repeats it.
- **Fix:** print only hashes in both steps.
- **Verified:** by a fresh agent, from the workflow.
- **Release:** in step 7c′ (#21), applied by hand.
- **Status:** fixed in #21: every step prints only
  hashes or tag names, and the UTC step runs with `pipefail`, so it fails
  closed.

### S-013 · Low · Attribution outside git's trailer block passes the hygiene job

- **Where:** `.github/workflows/hygiene.yml`, the identity step (#12).
- **Problem:** the step reads trailers as git parses them. A `Co-authored-by`
  line in an earlier paragraph, or followed by prose when there is no
  sign-off, is not a trailer, and other keys (`Co-developed-by`,
  `Reviewed-by`) are not checked.
- **Impact:** privacy: a name could reach public history. Criterion 1 names
  only `Co-authored-by` and `Signed-off-by` trailers, which are caught.
- **Fix:** also scan the whole message for `<key>-by:` lines that do not name
  the project, printing only hashes.
- **Verified:** by a fresh agent, in a scratch repository.
- **Release:** in step 7c′ (#21), applied by hand.
- **Status:** fixed in #21: a new step fails any
  `<key>-by:` line, anywhere in a message after `5393f67`, that does not name
  the project. Lines git does not read as trailers either (quoted, bulleted,
  `Reviewed by:`) are not checked.

### S-014 · Low · `[skip ci]` skips the hygiene job

- **Where:** GitHub Actions; `.github/workflows/hygiene.yml`.
- **Problem:** a head commit whose message contains `[skip ci]` or a similar
  marker runs no workflow, so a bad commit can be fast-forwarded to `main`
  unchecked. The next run catches it, after it is public.
- **Fix:** make the hygiene check required for `main` in a ruleset; a skipped
  required check blocks the push.
- **Verified:** by a fresh agent, from GitHub's documented behaviour; the
  repository's settings could not be seen.
- **Release:** in step 7c′ (maintainer, settings).
- **Status:** fixed 2026-10-20, in the settings. The review found `main`'s
  ruleset already requires the check, but organisation admins may bypass it,
  so a `[skip ci]` push by an admin was not stopped. A second ruleset,
  `hygiene-required`, now requires only the `hygiene` check from GitHub
  Actions on the default branch, with no bypass; read back through the API.
  The check is matched by its name and source, so a commit that changes
  `hygiene.yml` itself can pass it; changes under `.github/` are reviewed
  like any other (the fifth review).

### S-015 · Low · Tags are not checked for the project's identity

- **Where:** `.github/workflows/hygiene.yml`, `release.yml`; `RELEASING.md`
  tags with `git tag -s`, which takes the local identity and time zone.
- **Problem:** nothing checks a tag's tagger name, address or offset.
  `v0.0.1`'s tag is the project's, in UTC.
- **Fix:** check them in the hygiene job on tag pushes, printing only the tag.
- **Verified:** by a fresh agent.
- **Release:** in step 7c′ (#21), applied by hand, before `v0.0.2` is tagged.
- **Status:** fixed in #21: the hygiene job checks
  every tag, and the Release workflow the tag it publishes, before anything
  is published; `RELEASING.md` tags in UTC and checks the tag before pushing
  it. A tag of a tag passed these checks (S-017).

### S-016 · Low · On macOS, a failed full sync was reported as a success

- **Where:** `crates/neoblast/src/store.rs`, `full_sync`, since #20 (S-010's
  fix).
- **Problem:** a directory's `F_FULLFSYNC` that failed for any reason was
  followed by a plain `fsync`, as SQLite does. If the drive failed to flush
  its cache, `F_FULLFSYNC` returns `EIO`, and the plain `fsync` after it,
  which does not flush the cache, can succeed. Files there get only a plain
  `fsync` (#18), so shards still in the drive's cache were reported as
  written. Before #20, std's `sync_all` returned that error. S-010's entry
  and #20's commit describe a fallback on `ENOTSUP` only.
- **Impact:** durability: after a power loss, a seal or heal reported as
  complete can be missing shards. Needs a failing drive, not an attacker.
- **Fix:** an `EIO` from `F_FULLFSYNC` is returned. Every other error still
  falls back, since they vary by file system (`ENOTSUP` on devfs, others on
  network and FUSE file systems) and mean only that it is not offered.
- **Verified:** by two fresh agents, from rustix's and std's source and by
  probing APFS, HFS+, FAT32, exFAT and devfs (only devfs refused, with
  `ENOTSUP`).
- **Release:** fixed before the v0.0.2 campaign.
- **Status:** fixed in #23 (`86b8a67`). Regression test:
  `store::tests::s016_a_failed_full_sync_is_not_hidden`.

### S-017 · Low · A tag of a tag passed the tag checks

- **Where:** `.github/workflows/hygiene.yml`, the tag step, `release.yml`'s
  tag step and `RELEASING.md` step 2 (#21).
- **Problem:** the checks read only the outer tag. Re-tagging in place
  (`git tag -f -s v0.0.2 -m v0.0.2 v0.0.2`) makes a tag whose target is the
  old tag, which is pushed with it and keeps its own tagger, address and
  offset. A tag of a tree or blob passed too.
- **Impact:** privacy: an identity the checks exist to stop could reach
  GitHub inside a tag that passes them. Needs a maintainer's mistake.
- **Fix:** every check also requires the tag to point at a commit
  (`%(type)`; `%(*objecttype)` peels through to the commit and would not
  see it). `RELEASING.md` says to delete and re-make a bad tag, never
  `git tag -f` over it.
- **Verified:** by two fresh agents, in a scratch repository.
- **Release:** fixed before `v0.0.2` is tagged, applied by hand.
- **Status:** fixed in #23 (`86b8a67`). Tested in a scratch
  repository: a tag of a tag and a tag of a tree fail both checks; a project
  tag of a commit, and `v0.0.1`, pass.

### S-018 · Low · Some crafted attribution lines passed the hygiene job

- **Where:** `.github/workflows/hygiene.yml`, the attribution step (#21,
  S-013).
- **Problem:** a line that follows a lone carriage return was read as part
  of the line before, and a key with a Unicode hyphen (`Co‐authored‐by`)
  did not match. Either way the name is in the public message.
- **Impact:** privacy, for a crafted message only; tools write ASCII
  `-by:` lines ending in a newline, which were caught.
- **Fix:** a lone carriage return also ends a line, and any byte that is not
  a letter, digit, space or colon counts as the hyphen before `by:`, so
  `Standby:` and the like still pass.
- **Verified:** by two fresh agents, in a scratch repository.
- **Release:** fixed before `v0.0.2` is tagged, applied by hand.
- **Status:** fixed in #23 (`86b8a67`). Tested in a scratch
  repository: lone carriage returns, U+2010 and U+FF0D hyphens and a
  `Reviewed-by` naming someone else fail; `Standby:`, a CRLF sign-off by the
  project and a URL containing `-by:` pass; so does all of `main`'s history.

### S-019 · Low · A shard of the wrong length stopped `heal`

- **Where:** `crates/neoblast/src/stream.rs`, `verify_all` and
  `first_k_verified` (predates the range).
- **Problem:** a shard was checked against its hash but not its length; the
  length was left to the source, and `ShardSource::get` only "should" check
  it. A manifest can commit to a shard of another length. From a source
  that does not check, heal handed it to the decoder, whose error stopped
  the run: later segments were not repaired, and an earlier lost segment was
  not reported. `DirStore` checks the length, so the CLI was not affected.
- **Impact:** a publisher can stop repair of their own file only, through a
  library caller's source.
- **Fix:** a shard counts only if it is as long as the header says, whatever
  the source returns (parameters come only from a verified header).
- **Verified:** by a fresh agent, which ran a proof of concept.
- **Release:** fixed before the v0.0.2 campaign.
- **Status:** fixed in #23 (`86b8a67`). Regression test:
  `s019_a_shard_of_the_wrong_length_counts_as_missing`.

### S-020 · Low · `campaign.sh` ran Python with `fuzz/` on its module path

- **Where:** `fuzz/campaign.sh`, every `python3` call.
- **Problem:** `python3 -` puts the working directory, `fuzz/`, first on
  `sys.path`, so a stray `fuzz/json.py` would run in place of the module.
  A campaign refuses untracked files there, but `summary` does not check.
- **Impact:** code execution on the fuzzing machine for anyone who can
  already write into the clone. Hardening.
- **Fix:** run Python isolated (`-I`).
- **Verified:** by two fresh agents; `-I` tested with Python 3.9 (as on a
  fresh Mac) and 3.14.
- **Release:** not blocking (not in `code_id`).
- **Status:** fixed in #23 (`86b8a67`).

### S-021 · Low · The bench and vector scripts ran Python with the working directory on its module path

- **Where:** `bench/heal-check.sh`, `bench/heal-heap.sh`,
  `bench/segment-size.sh` and `tools/leopard-vectors/generate.sh`, every
  `python3` call (predates the range; S-020 fixed `campaign.sh` only).
- **Problem:** `python3 -`, `python3 -c` and `python3 -m venv` put the
  directory the script is run from first on `sys.path`, so a `json.py`,
  `pathlib.py` or `venv.py` there runs in place of the module. (`time` is
  built into the interpreter, so a `time.py` does not.)
- **Impact:** code execution for anyone who can already write into the
  directory the script runs from. Hardening, as S-020.
- **Fix:** `-I` on every call, the venv's own Python included, which now
  also runs `pip`; it still finds the venv's packages.
- **Verified:** by a fresh agent, with a planted `json.py` under the exact
  `-c` call on Python 3.9 and 3.14; a planted `venv.py` ran under
  `python3 -m venv` and not under `-I`.
- **Release:** not blocking (not in `code_id`).
- **Status:** fixed in the review PR after #26.

### S-022 · Low · An identity could ride inside a commit or tag object past the hygiene checks

- **Where:** `.github/workflows/hygiene.yml`, the identity and tag steps, and
  `release.yml`'s tag step (the commit side predates the range; the tag side
  is what S-017 left).
- **Problem:** the checks read git's parsed fields only. A merge of a signed
  tag writes a `mergetag` header holding that tag's tagger, a tag can carry
  another header (`git hash-object --literally`, which `fsck` accepts), and
  nothing read a tag's message. Each is public through `git cat-file -p`.
- **Impact:** privacy. It needs a maintainer to merge someone's signed tag,
  craft an object or write a name into a tag message. `main` has no merge
  commits, and `protect-main` requires a linear history except through its
  admin bypass.
- **Fix:** a new hygiene step allows a commit after `5393f67` only `tree`,
  `parent`, `author`, `committer` and `gpgsig` headers, with the author and
  committer read as stored, and a signature only as an SSH-armored block of
  base64. Both tag checks read the tag object as stored: only `object`,
  `type commit`, its name and the project's tagger in UTC, and as message
  only the tag's name and an SSH-armored block. A NUL becomes a control
  byte first, which fails every line, so it cannot hide the rest of a line
  from mawk. Nothing verifies
  the signature, so a block of base64 letters could still spell a name; an
  allowed-signers check would close that.
- **Verified:** by two fresh agents, in scratch repositories.
- **Release:** fixed before `v0.0.2` is tagged, applied by hand.
- **Status:** fixed in the review PR after #26. Tested on crafted objects
  under mawk, gawk and BSD awk: a `mergetag` or `encoding` header, another
  tag header, a name in a commit's or a tag's signature block or after it,
  a NUL in an author or tagger line, a name in a tag's message or after a
  lone carriage return, a tag whose name and message differ, and a tag of
  a tag all fail; all of `main`'s history, `v0.0.1`, and a tag next to a
  branch of the same name pass.

### S-023 · Low · A commit could reach `main`, and a tag be released, with `deny` failing

- **Where:** the merge rule in `plans/todo/v0.0.2.md`, and `release.yml`'s
  `verify` job (predates the range).
- **Problem:** `protect-main` requires `deny` among other checks, but the
  fast-forward push goes through its admin bypass, so only the maintainer's
  own wait enforces them. #25 was pushed 42 s after its CI began; `deny`
  then failed on RUSTSEC-2026-0385, published 2026-10-23, after the last
  green run. The Release workflow's `verify` job does not run `cargo deny`,
  so tagging #25 would have published with a lock file the advisory flags.
- **Impact:** a release built from a lock file with a known advisory, which
  `cargo install --locked` would install.
- **Fix:** `verify` runs `cargo-deny-action`, pinned as in CI, before the
  tests, and the merge rule waits for `gh pr checks <n> --watch --fail-fast`
  to succeed before the push.
- **Verified:** by two fresh agents, from the runs' times, the rulesets and
  advisory-db's history.
- **Release:** fixed before `v0.0.2` is tagged, applied by hand.
- **Status:** fixed in the review PR after #26.

### S-024 · Low · Library users can keep the `ctutils` that RUSTSEC-2026-0385 flags

- **Where:** the crates' `Cargo.toml` files set no minimum for `ctutils`,
  which `universal-hash` 0.6.1 asks for as `0.4`; `c3d405d` fixed this
  repository's lock files only.
- **Problem:** a program that uses `neoblast-crypto` as a library resolves
  `ctutils` itself. A lock file that already holds 0.4.2 or 0.4.3 keeps it.
  Where cargo honours a `rust-version` below 1.87 (resolver 3, the default
  from edition 2024), it picks 0.4.2, and `cargo update -p ctutils` keeps
  it; `--precise 0.4.5` moves it.
- **Impact:** in such a program, the tag comparison `poly1305` makes through
  `universal-hash` might not run in constant time. Probably not exploitable,
  since whether a tag is accepted is public anyway; not proven.
- **Fix:** the CHANGELOG tells library users how to update `ctutils` to
  0.4.4 or later. A minimum in `neoblast-crypto` would be a new direct
  dependency (the blast-radius rule in `decisions/README.md`) and changes
  `code_id`, so it waits until after v0.0.2.
- **Verified:** by two fresh agents, with `cargo generate-lockfile` for a
  downstream crate at each `rust-version`.
- **Release:** not blocking.
- **Status:** open. The CHANGELOG note is in the review PR after #26.

### S-025 · Low · A NUL in a commit message hides the rest of it from the message checks

- **Where:** `.github/workflows/hygiene.yml`: the identity step's trailers,
  the session-link `--grep`, the attribution step's `%B` (S-008, S-013 and
  S-018, before the range) and S-022's header step, which skips the message
  (`118f710`).
- **Problem:** git reads a commit message only up to its first NUL. A
  commit object (`git hash-object -t commit --literally`) whose message
  holds `hidden`, a NUL, then `Co-authored-by: Alice <a@example.org>` shows
  `%B` as `subject`, a blank line and `hidden`; its trailers are empty;
  `--grep=alice` finds nothing; and S-022's step passes, since after the
  blank line it reads nothing. `git fsck` only warns (`nulInCommit`). The
  whole message is public through `git cat-file -p`. S-022 says a NUL
  "cannot hide the rest of a line", which holds for headers only.
- **Impact:** privacy, as S-022. It needs a maintainer to craft a commit
  object and push it.
- **Fix:** in S-022's step, fail any control byte but a tab, a CR included,
  in the message
  as stored (`body && /[\001-\010\013-\037\177]/ { bad = 1 }` before
  `body { next }`). No commit since `5393f67` holds one.
- **Verified:** reproduced under mawk 1.3.4 in a scratch repository by a
  fresh agent, then again by the reviewer; GitHub's own acceptance of the
  push was not tested.
- **Release:** fixed before `v0.0.2` is tagged, applied by hand; not in
  `code_id`.
- **Status:** fixed in the change after `7f8b3e9`, in
  `.github/checks/commit-object.awk` (W-031). `.github/checks/test.sh`
  fails without it, and no commit since `5393f67` fails with it.

### S-026 · Low · Nothing checks that a commit or tag is signed, or by whom

- **Where:** `.github/workflows/hygiene.yml` and `release.yml`'s `verify`
  job; `RELEASING.md` step 2 (predates the range; S-022's fix names it as
  what is left).
- **Problem:** CLAUDE.md says commits are signed and `RELEASING.md` says the
  tag is SSH-signed, but S-022's checks allow a signature without requiring
  one, and nothing runs `git verify-commit` or `git verify-tag`. An unsigned
  tag, made with `git tag -a` in place of `-s`, passes both workflows; so
  does a block of base64 letters that spells a name. `RELEASING.md` says the
  checks refuse a tag with "anything but its name and its signature as
  message", which reads as if a signature were required.
- **Impact:** a released tag that is unsigned, or signed by another key,
  with no check failing. Pushing a tag needs write access and publishing
  needs approval in the `release` environment, so this is about provenance
  and the documented guarantee, not about who can release.
- **Fix:** commit the project's SSH public key as an allowed-signers file,
  and have `verify` (and the hygiene tag step) run `git -c
  gpg.ssh.allowedSignersFile=<file> verify-tag`; until then, `RELEASING.md`
  says only that a signature, if there is one, must be an SSH block.
- **Verified:** by the reviewer, with an unsigned tag and an unsigned commit
  under both checks; confirmed by a fresh agent.
- **Release:** not blocking; the tag step in `RELEASING.md` makes a signed
  tag.
- **Status:** open.
