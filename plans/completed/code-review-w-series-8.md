# Code-review worklist (W-series, part 8)

Status: completed 2026-11-09: every item is fixed in the PR after #55.
Opened 2026-11-09 by the fifteenth `/review-since` (`05bbd95..6c2e947`, #52
to #55), logged in `docs/logs/code-review-log.md`. Part 7, W-056 to W-066,
is closed (`plans/completed/code-review-w-series-7.md`). All items are low
or cleanup, and none blocks anything; all but W-067 are in the memory
budget of `05b7838` (#54).

IDs are unique across all worklists and continue from the highest W-number used.

## Success criteria

1. W-067: the README says that a failed `seal` leaves an empty
   `photo.link`, which must be removed before a retry.
2. W-068: when `seal` refuses a file over the budget, it advises a smaller
   `--n` or `--segment-size`, not only a higher `--max-memory`.
3. W-069: an estimate that overflows is refused whatever the budget, and a
   test reaches that branch.
4. W-070: `check_params`'s doc says which message a file over the budget
   gets.
5. W-071: the estimate's test uses decision 0011's one-segment cases and
   pins its figures, in MiB.
6. Every test passes, and the golden vectors are unchanged.

## Findings

### W-067 (low): a failed `seal` in the README's quick-start leaves an empty link file

- Where: `README.md`, "What works today" (W-066, `05b7838`, #54).
- What: the shell creates `photo.link` before `seal` runs, so a `seal` that
  fails, such as one over `--max-memory`, leaves it empty, and with
  `set -C` every retry then fails with "file exists". Before W-066 the
  retry wrote over it.
- Remediation: say so where the README says to remove an old one.
- Verified: by a fresh agent under `sh`, `bash` and `zsh`, with `false` in
  place of `seal`.
- Status: fixed in the PR after #55.

### W-068 (cleanup): `seal`'s refusal advises only a higher budget

- Where: `crates/neoblast/src/main.rs`, `within_budget` (`05b7838`, #54).
- What: `seal`, `open` and `heal` gave one message, ending "raise
  --max-memory to allow it". For `seal` that leads a publisher to make a
  file that every reader and healer refuses by default, which decision
  0011 has `seal` refuse to prevent.
- Remediation: for `seal`, advise a smaller `--n` or `--segment-size`.
- Verified: by a fresh agent, by reading.
- Status: fixed in the PR after #55: `within_budget` takes the advice;
  `seal` gives `SHRINK_THE_FILE`, `open` and `heal` `RAISE_THE_BUDGET`.
  `s037_seal_open_and_heal_keep_to_the_memory_budget` and
  `the_budget_is_in_mib_and_inclusive` check both.

### W-069 (cleanup): an estimate that overflows passes the largest budget

- Where: `crates/neoblast/src/main.rs`, `within_budget` (`05b7838`, #54).
- What: an estimate that overflowed became `u64::MAX` MiB, which
  `--max-memory 18446744073709551615` allows, though the comments said no
  budget does. No v1 header overflows, so it cannot happen, and the branch
  had no test.
- Remediation: refuse an overflowed estimate whatever the budget.
- Verified: by a fresh agent, from the header's limits and clap's parsing.
- Status: fixed in the PR after #55: `check_budget` refuses `None`, and
  `an_estimate_that_overflowed_is_refused` reaches it.

### W-070 (cleanup): `check_params`'s doc reads as if a file over the budget gets W-009's hint

- Where: `crates/neoblast/src/main.rs`, `check_params` (`05b7838`, #54).
- What: "gets the same hint (review W-009). So is a file over the memory
  budget"; it gets `within_budget`'s message.
- Remediation: name the message it gets.
- Verified: by a fresh agent, who found the wording ambiguous rather than
  wrong.
- Status: fixed in the PR after #55.

### W-071 (cleanup): the estimate's test does not use decision 0011's cases

- Where: `crates/neoblast/src/main.rs`,
  `working_memory_is_decision_0011s_estimate` (`05b7838`, #54).
- What: its cases A to D are files as long as a segment, which the 8-byte
  length prefix makes two segments, not the decision's one; and it
  recomputes the expected value with the code's own helpers, so the
  decision's figures are not pinned.
- Remediation: one-segment cases, and the estimate in MiB as literals.
- Verified: by a fresh agent, who recomputed every case: rounded up, the
  estimates are 2081, 164, 417, 353 and 41 MiB, the decision's 2080, 163,
  416, 352 and 40 rounded down.
- Status: fixed in the PR after #55: cases A to E are one segment long,
  each case pins its estimate in MiB, and the test checks the segment
  counts.

## Not tracked

Checked by the same review and left as they are, with the reason:

- A `seal` from a pipe holds its input without counting it: decision 0011
  says so among its "Bad" consequences, and counting it would refuse files
  that readers accept, while not stopping the read.
- `open` and `heal` read the manifest before the check: the security
  review's S-038, documented in decision 0011.
- The CHANGELOG leaves out the rustix bump: no earlier release lists a
  lock-only bump that is not a security fix.
- The limit lives in the CLI, not the library: decision 0011 chose this,
  and lists a limit in the library among its considered options.
