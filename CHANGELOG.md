# Changelog

All four crates (`neoblast`, `neoblast-format`, `neoblast-crypto`,
`neoblast-erasure`) share one version (decision 0008). The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/) (0.0.x: anything may change except
the frozen v1 byte format).

## [0.0.1] - unreleased

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
- Memory has only been measured on macOS (Apple M5, `bench/results/`). There,
  `heal`'s working memory grows with the file size: with 1 MiB segments it went
  from 10.9 to 24.7 MiB between a 256 MiB and a 1 GiB file. At the default
  4 MiB segments, `heal`'s peak was 53.7 MiB (256 MiB file) and 47.4 MiB
  (1 GiB file). A fix that reuses the codec's
  buffers is being measured.
- The default segment size is not final (ROADMAP open item 2) and may change.
  Files sealed with any segment size stay readable, because the size is
  recorded in the header.

[0.0.1]: https://github.com/neoblast-org/neoblast/releases/tag/v0.0.1
