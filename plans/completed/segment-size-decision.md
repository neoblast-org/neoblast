# Segment-size decision (ROADMAP open item 2)

Status: completed 2026-10-17: decision 0009 applies the rule to runs at
1 GiB and 256 MiB and sets the default to 1 MiB, the only size within it.
Criterion 4: seal, open and open from parity grow by at most about the
manifest; heal's RSS steps up 5 MiB between 256 MiB and 1 GiB and no further
at 2 GiB, while its live heap grows only by the manifest (decision 0009,
finding 2).

Opened for v0.0.2 (step 6 of `plans/completed/v0.0.2.md`). The heal memory fix
on `exp/heal-reuse` lowered heal's peak but did not stop it growing with the
file (`plans/completed/phase-1-finish.md`, A3 results), so it and this decision
are not in v0.0.1. At 4 MiB segments, heal on a 1 GiB file already exceeds the
32 MiB rule, with or without that fix.

2026-10-16: what grew is found and fixed (#15, #16): heal now grows with the
file only by its manifest, to the byte (`bench/heal-heap.sh`; Step 5 results
in `plans/completed/v0.0.2.md`). Its working memory by RSS is still high at 4 MiB
segments, mostly memory the system allocator keeps after blocks are freed.

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
   file size. (Heal's live heap grows only by the manifest, shown exactly by
   `bench/heal-heap.sh` in #16; the step 6 RSS run must still confirm this
   for every operation.) (The first reference runs showed heal growing on macOS, from
   10.9 to 24.7 MiB at 1 MiB segments; that must be fixed before the decision.)

## Precedent to match

`decisions/0006-erasure-code.md` (rule fixed first, method, findings, outcome).
