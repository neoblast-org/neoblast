# Segment-size decision (ROADMAP open item 2)

Status: todo, for v0.0.2. The heal memory fix on `exp/heal-reuse` lowered heal's
peak but did not stop it growing with the file (`phase-1-finish.md`, A3
results), so it and this decision are not in v0.0.1. First find what still
grows. At 4 MiB segments, heal on a 1 GiB file already exceeds the 32 MiB rule,
with or without that fix.

## Success criteria

1. `bench/segment-size.sh` runs on the reference Mac (1 GiB file; 1, 4, 8 and
   16 MiB segments), and its record is committed in `bench/results/`.
2. A decision record applies the rule written in ROADMAP item 2 ("largest size
   keeping peak RAM ≤ 32 MiB per operation") to those numbers, and changes
   `Params::default()` only if the rule says so.
3. The record states which memory figure the rule uses. **Settled on
   2026-10-14, before the final run (the two earlier runs, 15:06Z and 15:19Z,
   had been seen): working memory** (the peak minus the same
   operation on a 1 KB file), which must be ≤ 32 MiB for seal, open, heal and
   open from parity. The fixed ~11 MiB (program and codec tables) is excluded
   because a long-running process pays it once for all its operations.
4. Runs at both 256 MiB and 1 GiB show working memory does not grow with the
   file size. (The first reference runs showed heal growing on macOS, from
   10.9 to 24.7 MiB at 1 MiB segments; that must be fixed before the decision.)

## Precedent to match

`decisions/0006-erasure-code.md` (rule fixed first, method, findings, outcome).
