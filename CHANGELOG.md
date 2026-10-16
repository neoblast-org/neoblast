# Changelog

All four crates (`neoblast`, `neoblast-format`, `neoblast-crypto`,
`neoblast-erasure`) share one version (decision 0008). The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/) (0.0.x: anything may change except
the frozen v1 byte format).

## [Unreleased]

### Added

- `Params::header` plans a file's header and refuses parameters whose
  manifest would be larger than a store reads, with the new
  `Error::ManifestTooLarge`.
- `ShardSink::finish`, called once after the last shard of a successful run;
  the default does nothing. `DirStore` uses it to sync the last segment
  directory, so a caller that uses `put` directly calls it too.

### Changed

- `seal`, `seal_with_key` and `seal_stream` refuse parameters whose manifest
  would be larger than 256 MiB (`MAX_MANIFEST_LEN`) before they write a shard,
  whatever the sink. Before, `seal_stream` into a `DirStore` wrote every shard
  and then left the store with no manifest (review W-006).
- `Header::new` reports bad k or n as `HeaderError::Params` even when the
  segment size or layout would also fail; the bytes of valid headers are
  unchanged.
- `heal_stream` no longer stops at a shard its sink cannot store: it goes on,
  then returns the first unrecoverable segment or, failing that, the first
  write error.
- `DirStore::write_sealed` writes the shards before the manifest, as a
  streaming seal does.

### Fixed

- Rejecting a planted manifest costs little more than its own size: the root
  is computed from the raw bytes and compared before anything else is built.
  A 264 MB manifest that matches no link now peaks `heal` at 267 MB instead of
  up to 862 MB, and an accepted manifest keeps its hashes in one flat list
  (review S-005).
- `seal` refuses a non-empty `--out` given without `--force` before it reads
  the input, so a piped input is no longer read into memory first (review
  W-005).
- `seal --k 0` reports invalid k/n, and a file that needs too many segments
  gets the hint to use a larger `--segment-size` or a smaller `--n` (review
  W-009).
- A store directory from someone else can no longer stop `heal` part-way. A
  shard that cannot be written is skipped, every other one is still rebuilt,
  and `heal` then fails naming the path. Temporary files get random names, so
  names planted in advance no longer block a write (reviews W-003 and W-004).
- `seal` and `heal` sync every shard and the manifest before renaming them into
  place, and on Unix the directories after, so what they report as written
  survives a power loss. On macOS each shard costs a full flush: sealing
  256 MiB took about 14.5 s on an Apple M5, up from 2–4 s (review W-007).

## [0.0.1] - 2026-10-16

The offline core (Phase 1). Experimental and **not audited**: do not rely on it
to protect anything yet. It keeps content unreadable to storage hosts; it does
not hide who published or read it ([`spec/threat-model.md`](spec/threat-model.md)).

### Added

- The `neoblast` CLI: `seal` a file into shards, `open` it with the read link,
  and `heal` missing shards with only the verify link. All three work one
  segment at a time.
- The v1 byte format, now frozen ([`spec/header.md`](spec/header.md)):
  - a 32-byte header (magic `NBL\x01`, suite `0x01`) that the file root commits
    to, together with every shard (decision 0002);
  - Padmé padding and segmented XChaCha20-Poly1305 with the header as
    associated data;
  - Leopard-RS high-rate Reed-Solomon over GF(2^16), byte-for-byte compatible
    with the Leopard C library (decision 0006);
  - a BLAKE3 hash tree, the file root and the verify manifest.
- Capability links `nbl://f/…#<read key>` and `nbl://fv/…`, with a version
  byte instead of multiformats CIDs (decisions 0004 and 0007).
- Defaults: 20-of-50 shards over 4 MiB segments.
- Golden vectors (`spec/test-vectors/`), pinned by hash in CI; Leopard
  fixtures; property tests that drop any n − k shards; and five fuzz targets,
  each run for 24 hours without a crash before release (`fuzz/campaigns/`).

### Security

- A store directory is treated as hostile, since it may come from someone else.
  `open` and `heal` read only regular files, so they don't read through a
  planted symlink or block on a FIFO. Shards and manifests are written to a new
  file and renamed into place, so a planted symlink is replaced, not written
  through. A segment directory that is a symlink is neither read nor written.
  The store directory itself is yours to choose and may be a symlink (review
  S-001).
- A manifest larger than 256 MiB is refused before it is read, and `seal`
  refuses a file that would need one: about 640 GiB at the default parameters,
  less with smaller segments (review W-001).

### Known issues

- A publisher can build shards so that readers who decode from different
  subsets get different content; nothing checks that all n shards agree
  (review S-002). Treat a link as naming the publisher's file, not one fixed
  plaintext, until this is closed.
- `neoblast open` takes the read link as an argument, so it can show in `ps`
  and shell history, and decoded copies of the key are not wiped from memory
  (review S-003).
- Rejecting a planted manifest near the 256 MiB limit can take up to about
  820 MiB of memory before the clean error (review S-005).
- A store directory from someone else can stop `heal` part-way: one planted
  directory, or planted temporary-file names, at a shard's path ends the run,
  so later segments are not repaired. No shard is lost, and `open` still works
  from any k good shards (reviews W-003, W-004 and S-009).
- `seal` reads a whole piped input into memory before it refuses a non-empty
  `--out` given without `--force` (review W-005).
- `seal` and `heal` do not sync shards or the manifest to disk before they
  report success, so a power loss soon after can leave them empty. Run `sync`
  before deleting the original or other copies (review W-007).
- Memory has only been measured on macOS (Apple M5, `bench/results/` and
  `bench/heal-check.sh`). There, `heal`'s working memory grows with the file
  size: between a 256 MiB and a 1 GiB file it went from about 11 to 20 MiB with
  1 MiB segments, and from about 36 to 43 MiB with the default 4 MiB segments,
  above the 32 MiB target. A fix that reuses the codec's buffers lowered the
  peak by up to 5 MiB but did not stop the growth; it is for a later release.
- The default segment size is not final (ROADMAP open item 2) and may change.
  Files sealed with any segment size stay readable, because the size is
  recorded in the header.

[Unreleased]: https://github.com/neoblast-org/neoblast/compare/v0.0.1...HEAD
[0.0.1]: https://github.com/neoblast-org/neoblast/releases/tag/v0.0.1
