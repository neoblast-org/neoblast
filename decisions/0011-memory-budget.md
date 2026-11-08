# 0011: The CLI refuses files over a memory budget

- Status: accepted; the code lands after v0.0.3, as it changes `crates/`
- Date: 2026-11-07
- Settles: review finding S-037

## Context and problem statement

The header lets a publisher choose k, n and the segment size, down to k = 2
and up to n = 128 and 64 MiB segments. Every operation works one segment at
a time, but one segment's shards can then be 128 of just over 32 MiB, about
4 GiB, and the erasure codec works on about as many again. Since decision
0010, an intact `open` or `heal` rebuilds all n shards, so a publisher can
make every reader and every healer of a file need gigabytes (S-037). Seal,
opening from parity and healing after losses always did.

Checking the rebuilt shards one at a time does not help: the codec's work
buffers have the same scale. `neoblast-erasure` always runs the high-rate
algorithm of `reed-solomon-simd` 3.1.0, never the crate's own choice of rate
(`spec/header.md`). With P = `next_pow2(n − k)`, its encoder works on k
rounded up to a multiple of P shards, which is less than P + k, and its
decoder on W = `next_pow2(P + k)` (`rate_high.rs`), so W bounds both for
every k and n the header allows (review W-056). Only a limit bounds it.

The maintainer chose a limit in the CLI, with an option to raise it, and to
leave the library as it is: a program that embeds the library sets its own.

The rule, written before any measurement:

- **The default budget is 1 GiB of working memory per operation**, so that one
  operation fits a 2 GiB VPS or a 4 GiB phone with room to spare.
- `seal`, `open` and `heal` refuse a header whose estimated working memory is
  over the budget, unless `--max-memory` raises it. `seal` refuses too, so a
  publisher does not by default make a file that readers refuse.
- The estimate must be at least the working memory measured for every
  operation in every case below, as decision 0009 measures it (the peak minus
  the same operation on a 1 KB file), and far below the budget at the
  defaults.

How the estimate is built was not fixed in advance; the next sections say
which parts were chosen after which results.

## Method

Release build of `3b93b8d` on the reference Mac (Apple M5, macOS, arm64),
`/usr/bin/time -l`, one run each; decision 0009 saw single runs of one
operation differ by up to 8 MiB. For each case, a random file is sealed;
then it is opened with every shard present and from the last k shards of
each segment, and healed with nothing missing and from the last k shards.
Cases A to E are one segment long; F, at the defaults, is 1 GiB, 1,056
segments with padding. The same operations on a 1 KB file at the defaults peak at 11.4
to 11.6 MB, the fixed cost. Every case ran the high-rate algorithm, the only
one `neoblast-erasure` uses; this record first said A ran the low-rate one
(review W-056).

## Findings

Working memory in MiB (peak minus 11.6 MB):

| Case | k-of-n | Segment | Shard | seal | open | open, last k | heal | heal, last k |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| A | 2-of-128 | 8 MiB | 4 MiB | 1028 | 1036 | 1518 | 1018 | 1534 |
| B | 20-of-50 | 16 MiB | 0.8 MiB | 67 | 85 | 126 | 85 | 110 |
| C | 64-of-128 | 64 MiB | 1 MiB | 195 | 264 | 330 | 263 | 324 |
| D | 2-of-3 | 64 MiB | 32 MiB | 163 | 231 | 297 | 231 | 297 |
| E | 20-of-50 | 1 MiB | 51 KiB | 9 | 10 | 17 | 8 | 18 |
| F | 20-of-50 | 1 MiB | 51 KiB | 8 | 11 | 16 | 16 | 19 |

The first estimate tried, `shard_len × (2n + W)`, covered A to D (1.04 to
1.34 times the largest operation) but not E, the defaults (8 MiB against
18), where buffers that do not scale with the shards outweigh them. So, after
seeing A to E, two terms were added: the manifest's length, which `open` and
`heal` hold and which grows with the file, and a constant of 32 MiB, decision
0009's budget for one operation's working memory at the defaults, rather than
a figure fitted to E. Then, before F was run:

```text
P        = next_power_of_two(n − k)
W        = next_power_of_two(P + k)
estimate = shard_len × (2n + W) + manifest_len + 32 MiB
```

For A to E it is 2080, 163, 416, 352 and 40 MiB, 1.19 to 2.23 times the
largest operation. `shard_len` is the first segment's, the largest. For F,
with its 1.6 MiB manifest, it is 42 MiB, 2.21 times the largest operation
(heal from the last k shards, 19 MiB), so it held for a file of many
segments too.

## Decision outcome

**The CLI estimates each file's working memory and refuses one over the
budget.** The default is 1 GiB; `--max-memory <MiB>` on `seal`, `open` and
`heal` raises or lowers it. The error names the estimate and the option.
At 20-of-50, a file with segments of 64 MiB, the header's largest, comes to
about 557 MiB plus its manifest, so it passes. The manifest term is at
most 256 MiB, the largest manifest a store reads (`MAX_MANIFEST_LEN`), so
at the default budget it never refuses a file by itself; only a lower
`--max-memory` can (review W-057). Of the cases above only A is refused by
default. The library's functions do not check: a program that embeds it
applies its own limit, as `spec/header.md` already advises (S-037).

No bytes change, and no file becomes unreadable: a refused file opens with a
higher `--max-memory`.

### Consequences

- Good: by default, a link can no longer make a reader or healer that uses
  the CLI need more than about 1 GiB, whichever shards it holds, as far as
  the estimate holds (see the second "Bad"); a machine with less free than
  that is still exposed (review W-063).
- Good: the bound is stated, and covers every operation, not just the
  intact ones S-037 found.
- Bad: legal v1 files with extreme parameters need the option to open.
- Bad: the estimate rests on one run of each case on one machine and
  allocator; the 32 MiB constant is a stated allowance, not a measured part,
  and a platform whose allocator keeps more could exceed it.
- Bad: library users are not protected unless they check.

### Revisit

At the Phase 2 gate, with decision 0009's budget: the hardware storage nodes
run on, and how many operations they run at once.

## Considered options

- **Check the rebuilt shards one at a time.** Saves this crate's copies,
  but the codec's work buffer keeps the same scale.
- **A limit in the library** (`open_manifest` and `heal_manifest` take one).
  Protects every caller, at the cost of an API change; left until there is a
  second caller.
- **Accept, with documentation only.** The docs already give the bound; it
  leaves every reader exposed by default.
