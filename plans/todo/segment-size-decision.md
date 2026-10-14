# Segment-size decision (ROADMAP open item 2)

Status: todo. Waiting on the reference run and one clarification.

## Success criteria

1. `bench/segment-size.sh` runs on the reference Mac (1 GiB file; 1, 4, 8 and
   16 MiB segments), and its record is committed in `bench/results/`.
2. A decision record applies the rule written in ROADMAP item 2 ("largest size
   keeping peak RAM ≤ 32 MiB per operation") to those numbers, and changes
   `Params::default()` only if the rule says so.
3. The record states which memory figure the rule uses. The benchmark reports
   both total process peak and working memory (total minus the fixed ~11 MiB
   for the program and codec tables), and the cloud numbers put 4 MiB on
   different sides of the line depending on which is meant. The maintainer
   picks the reading; we do not pick it after seeing the reference numbers.

## Precedent to match

`decisions/0006-erasure-code.md` (rule fixed first, method, findings, outcome).
