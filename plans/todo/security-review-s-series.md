# Security review worklist: S-series

Status: todo. Opened 2026-10-14 by the first security review (empty tree to
`e24f453`), logged in `docs/logs/security-review-log.md`. The second review
(2026-10-15, `e24f453..a9445d9`) added S-005 to S-007, all low and none
blocking v0.0.1. The third review (2026-10-16, `a9445d9..5393f67`) added S-008
and S-009.

IDs are unique across all worklists and continue from the highest S-number used.
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
- **Status:** open.

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
- **Status:** open.

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
  GitHub-side copies are removed.

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
