# 0009: Default segment size: 1 MiB

- Status: accepted
- Date: 2026-10-17
- Settles: ROADMAP open item 2, for now (see "Revisit")

## Context and problem statement

`seal`, `open` and `heal` work one segment at a time, so the segment size sets
how much memory each operation needs. It also sets how many shards a file
becomes and how large each one is. Every file records its own segment size in
its header, so the default affects only files sealed from now on; it is not a
format change. Until now the default was 4 MiB, a placeholder.

Decision rule, written in ROADMAP item 2 before the final run: **the largest
size keeping peak RAM ≤ 32 MiB per operation.** The figure it uses was fixed
by the maintainer on 2026-10-14, after two earlier runs had been seen and
before the final one: **working memory**, the peak minus the same operation
on a 1 KB file, which must be ≤ 32 MiB for seal, open, heal and open from
parity. The fixed cost (about 3 MiB for the program and 8 MiB for the erasure
library's multiplication table) is excluded, because a long-running process
pays it once for all its operations. The candidates are the sizes ROADMAP
item 2 names, 1, 4, 8 and 16 MiB, measured on a 1 GiB file (the plan's
criterion 1), with a 256 MiB file to check growth.

## Method

`bench/segment-size.sh` on the reference Mac (Apple M5, macOS, arm64) at
commit `d56d51a`, the code after heal's memory fix (#16). Each run seals a
random file at 20-of-50, then measures peak resident memory with
`/usr/bin/time` for:

- seal;
- open with every shard present;
- heal after losing 30 shards in every segment;
- open after losing all 20 data shards (open from parity, the worst case).

The records are in `bench/results/`:

- `20261016T230715Z-segment-size.json`: 1 GiB file, all four sizes;
- `20261017T035658Z-segment-size.json`: 256 MiB file, all four sizes;
- six more runs at 1 MiB only, three per file size, to measure how much the
  figures vary between runs (`20261017T040521Z` to `20261017T042702Z`).

A one-off check of heal is not committed: the CLI heal at 1 MiB segments on
256 MiB, 1 GiB and 2 GiB files, one run each, and `bench/heal-heap/` with its
sink pointed at the real store, so that its exact heap count includes writing
the shards. What is reproducible from this repository is the heap count
without writes, `bench/heal-heap.sh "1024 2048" 1`.

## Findings

Working memory in MiB, 1 GiB file (256 MiB file in brackets):

| Segment | Seal | Open | Heal | Open from parity | Within 32 MiB |
| --- | --- | --- | --- | --- | --- |
| 1 MiB | 8.6 (8.1) | 5.1 (3.9) | 23.0 (18.0) | 11.2 (8.9) | yes |
| 4 MiB | 16.6 (25.7) | 14.0 (19.8) | **41.8 (50.3)** | **34.0** (28.4) | no |
| 8 MiB | **41.5 (36.2)** | 26.0 (26.6) | **71.4 (75.7)** | **67.1 (52.3)** | no |
| 16 MiB | **85.3 (91.2)** | **44.8 (39.1)** | **149.3 (141.9)** | **120.8 (104.6)** | no |

1. **Only 1 MiB is within the rule**, at both file sizes. Heal is the
   operation that decides it: at 4 MiB it is over the limit by 10 to 18 MiB.
2. **At 1 MiB, working memory grows with the file only by the manifest, plus
   one step in heal.** Medians of four runs, 256 MiB to 1 GiB: seal 8.1 to
   7.9 MiB, open 4.2 to 5.4, open from parity 9.5 to 11.2, heal 17.8 to 23.0.
   Open and open from parity grow by about the manifest (1.2 MiB between
   these files). Heal grows by 5.2 MiB; all four 1 GiB runs gave 23.0 or
   23.1 MiB, and three of the four 256 MiB runs 17.6 to 18.0, so that is not
   noise. Two checks suggest a step rather than continuing growth:
   - its exact live heap grows by the manifest and nothing more: by 2 bytes
     beyond it from 256 MiB to 1 GiB while writing the rebuilt shards, and by
     0 bytes beyond it from 1 GiB to 2 GiB (counted without writes);
   - its peak RSS was 28.4, 34.1 and 31.8 MiB at 256 MiB, 1 GiB and 2 GiB:
     no higher at 2 GiB than at 1 GiB.

   So the extra RSS looks like memory the system allocator keeps while heal
   writes, not data. These are single runs, and the fourth 256 MiB run put
   heal at 9.7 MiB, so single runs can differ by about 8 MiB; the decision
   rests on the 1 GiB figures, which did not vary.
3. **The manifest makes larger files cost more at 1 MiB.** Every operation
   holds it: about 1.6 MiB per GiB of file at 1 MiB segments, 0.4 MiB at
   4 MiB. Projected from those exact sizes, heal at 1 MiB reaches 32 MiB of
   working memory at a file of about 6 to 7 GiB, and above about 17 GiB it
   uses more than heal at 4 MiB would. The rule is applied at 1 GiB, as
   fixed; larger files are a known limit of this decision (see "Revisit").
4. **Much of heal's peak at 4 MiB is not live data.** Heal's peak live heap
   on a 1 GiB file, counted exactly by `bench/heal-heap.sh` on the same code,
   is 29.7 MiB, codec table included. Its peak RSS here was 52.8 MiB. Apart
   from the program itself (about 3 MiB), the gap is mostly memory the system
   allocator keeps after blocks are freed, which RSS counts.
5. **Costs of 1 MiB against 4 MiB**, for a 1 GiB file (padded to 1,056 MiB,
   so 1,056 or 264 segments):

   | | 1 MiB | 4 MiB |
   | --- | --- | --- |
   | Shard files | 52,800 | 13,200 |
   | Shard size | 52,480 bytes | 209,728 bytes |
   | Manifest (`n × 32` bytes per segment) | 1.6 MiB (0.16%) | 0.4 MiB (0.04%) |
   | Largest file `seal` accepts (manifest ≤ 256 MiB) | about 162 GiB | about 648 GiB |
   | Holder uptime needed at 20-of-50 (`sim/durability.py`) | 71.4% | 69.6% |

   More segments mean more chances for one to be unreadable, hence the higher
   uptime needed (independent failures assumed, as in the model). A larger
   file can still be sealed with a larger `--segment-size`, which `seal`'s
   error suggests.
6. **Times are not part of the rule, and they are noisy.** Opening from
   parity, which writes nothing, took a median of 13.0 s at 1 MiB on the
   1 GiB file (four runs, 9.7 to 23.0 s) against 8.3 s at 4 MiB (one run);
   on the 256 MiB file, 2.0 s against 0.76 s. So decoding is slower with the
   smaller shards, by about 1.6 to 2.6 times. Open with every shard present
   showed no difference (median 10.2 s, 7.3 to 13.0, against 10.1 s). Seal
   and heal are dominated by the full flush to disk of every shard file, four
   times as many at 1 MiB; at 1 GiB and 1 MiB they ranged from 245 to 277 s
   and from 152 to 191 s. A cheaper per-file sync is a separate change.

## Decision outcome

**1 MiB.** `Params::default()` changes from 4 MiB to 1 MiB segments; k and n
stay 20-of-50. `--segment-size` still chooses any size the format allows, and
files sealed at 4 MiB open and heal as before.

The v1 golden vectors are unchanged. Their `default-20of50` case was built
from `Params::default()`; it now names its parameters, 20-of-50 with 4 MiB
segments, so the vector keeps the default it was frozen with.

### Consequences

- Good: every operation fits the 32 MiB rule on a 1 GiB file, with 9 MiB to
  spare for the largest (heal).
- Good: no format change and no new code; the decision can be reversed for new
  files by changing one constant.
- Bad: four times as many shards, each a quarter the size. On disk that is
  more files and more flushes. On the network (Phase 2) it matters if shards
  are announced one by one: the count of announcements grows with it, 13,200
  or 52,800 for a 1 GiB file. If a holder announces once per file, for its
  shard of every segment, as BitTorrent announces a torrent rather than its
  pieces, a file is n network objects at any segment size.
- Bad: the manifest is four times larger (0.16% of the file), so memory grows
  faster with the file (finding 3), and the largest file `seal` accepts with
  the defaults falls from about 648 GiB to about 162 GiB.
- Bad: opening from parity is slower, and 20-of-50 needs 71.4% holder uptime
  instead of 69.6% for a 1 GiB file.

### Revisit

The 32 MiB limit has no stated basis: ROADMAP item 2 names no device or
workload behind it, and the working-memory difference here (23 against
42 MiB) fits any phone, desktop or small VPS. It was nonetheless fixed before
the result, so it is applied as written. Revisit at the Phase 2 gate, when
these are known:

- the hardware storage nodes actually run on, and how many heals they run at
  once, which gives a memory budget with a reason behind it;
- the file sizes that budget must hold for: past about 6 GiB the manifest
  alone pushes heal at 1 MiB over 32 MiB, unless operations stop holding the
  whole manifest;
- the second test ROADMAP item 2 asks for, announcing once per file instead of
  once per shard, which decides how much the number of shards costs on the
  network.

A new budget then gets a new record that supersedes this one.

## Considered options

- **Keep 4 MiB and cut heal's memory to fit** (reusing the codec's buffers,
  the approach on `exp/heal-reuse`). Two operations are over the limit at
  1 GiB, heal by about 10 MiB and open from parity by 2 MiB, and most of the
  excess is memory the allocator keeps, which buffer reuse may not remove.
  Not attempted: new code with an uncertain result, for a limit that may
  itself change.
- **Keep 4 MiB with a different memory allocator.** Heal's live heap at 4 MiB
  (29.7 MiB with the 8 MiB codec table) would be within the limit; the excess
  is the system allocator's. Not attempted: a new dependency that every
  operation runs on, under the blast-radius rule, for the same uncertain
  limit.
- **Restate the limit, for example to 48 MiB, and keep 4 MiB.** Rejected for
  now: it would change the rule after seeing the numbers, which fixing it in
  advance was meant to prevent. The doubt about the limit is recorded under
  "Revisit" instead.
- **A size between 1 and 4 MiB, such as 2 MiB.** Not among the candidates
  fixed in advance. Heal at 2 MiB would likely land near the limit, where
  single runs differing by about 8 MiB would make the result hard to judge.
