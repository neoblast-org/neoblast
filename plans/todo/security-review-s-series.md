# Security review worklist: S-series

Status: todo. Opened 2026-10-14 by the first security review (empty tree to
`e24f453`), logged in `docs/logs/security-review-log.md`.

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
- **Status:** fixed in #6 (`fix/hostile-store`). Regression tests: the `s001_*` tests in
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
- **Status:** open.
