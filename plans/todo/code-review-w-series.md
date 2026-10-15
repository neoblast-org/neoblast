# Code-review worklist (W-series)

Status: todo. Opened 2026-10-14 by the first `/review-since` (empty tree to
`e24f453`). Neither item blocked v0.0.1 on its own, but both are fixed in the
pre-release code change that S-001 required (#6), since that change
restarts the fuzz campaign anyway (`plans/todo/phase-1-finish.md`, A0).

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
- Status: fixed in #6 (`fix/hostile-store`).

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
- Status: fixed in #6 (`fix/hostile-store`).
