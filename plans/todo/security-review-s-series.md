# Security review worklist: S-series

Status: todo. Opened 2026-10-14 by the first security review (empty tree to
`e24f453`), logged in `docs/logs/security-review-log.md`. The second review
(2026-10-15, `e24f453..a9445d9`) added S-005 to S-007, all low and none
blocking v0.0.1. The third review (2026-10-16, `a9445d9..5393f67`) added S-008
and S-009. The fourth (2026-10-20, `5393f67..9bf18dc`, the v0.0.2 candidate)
added S-010 to S-015, all low.

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
- **Release:** in step 7d, applied by hand.
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
- **Release:** in step 7d, applied by hand.
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
- **Release:** in step 7d (maintainer, settings).
- **Status:** fixed 2026-10-20, in the settings. The review found `main`'s
  ruleset already requires the check, but organisation admins may bypass it,
  so a `[skip ci]` push by an admin was not stopped. A second ruleset,
  `hygiene-required`, now requires only the `hygiene` check from GitHub
  Actions on the default branch, with no bypass; read back through the API.

### S-015 · Low · Tags are not checked for the project's identity

- **Where:** `.github/workflows/hygiene.yml`, `release.yml`; `RELEASING.md`
  tags with `git tag -s`, which takes the local identity and time zone.
- **Problem:** nothing checks a tag's tagger name, address or offset.
  `v0.0.1`'s tag is the project's, in UTC.
- **Fix:** check them in the hygiene job on tag pushes, printing only the tag.
- **Verified:** by a fresh agent.
- **Release:** in step 7d, applied by hand, before `v0.0.2` is tagged.
- **Status:** fixed in #21: the hygiene job checks
  every tag, and the Release workflow the tag it publishes, before anything
  is published; `RELEASING.md` tags in UTC and checks the tag before pushing
  it.
