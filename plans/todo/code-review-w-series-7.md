# Code-review worklist (W-series, part 7)

Status: todo. Opened 2026-11-08 by the thirteenth `/review-since`
(`0572592..281bd1a`, #46 and #49), logged in `docs/logs/code-review-log.md`
after the v0.0.3 campaign (`plans/completed/v0.0.3.md`, step 3) had ended; this
paragraph first said during it (W-064). W-064 to W-066 were added by step
5's reviews (`281bd1a..05bbd95`, #50 and #51). Part 6, W-050 to W-055, is
closed (`plans/completed/code-review-w-series-6.md`). All items are low or
cleanup, and none blocks v0.0.3. W-058 and W-060 change `crates/`, and so
every target's `code_id`, which may not change between the campaign and the
tag; they wait until after v0.0.3, with the code of decision 0011. W-065 and
W-066 change files the release commit may not, and wait too. The others are
fixed in `96efd53` (#50), and W-064 in `9ccdaad` (#52), the commit `v0.0.3`
tags.

IDs are unique across all worklists and continue from the highest W-number used.

## Success criteria

1. W-056: decision 0011 explains its bound by the high-rate algorithm alone,
   the only one `neoblast-erasure` runs, and no longer says case A ran the
   low-rate one; the bound still holds for every k and n the header allows.
2. W-057: decision 0011 no longer says the manifest alone can pass the
   default budget.
3. W-058: removing the terminal check from `link_or_stdin` makes a test
   fail.
4. W-059: `spec/header.md` states the largest segment's shards exactly.
5. W-060: both ways of giving no link end in one error message.
6. W-061: the README says what `umask 077` does not do for a file that
   already exists.
7. W-062 and W-063: decision 0011 says k = 2 is the smallest k, and claims
   for its default no more than its estimate supports.
8. W-064: the plans say when the thirteenth reviews ran, and the next log
   entries correct the older ones.
9. W-065: decision 0011 says how much the codec works on when decoding.
10. W-066: the README's quick-start refuses to write the read link into a
    file that already exists.
11. Every test passes, and the golden vectors are unchanged.

## Findings

### W-056 (low): decision 0011 explains its bound by a rate the codec never runs

- Where: `decisions/0011-memory-budget.md`, "Context and problem statement"
  and "Method" (`281bd1a`, #49).
- What: it gives `reed-solomon-simd`'s own choice between the high-rate and
  low-rate algorithms, and says case A ran the low-rate one. But
  `neoblast-erasure` always builds `HighRateEncoder` and `HighRateDecoder`,
  and `spec/header.md` forbids the crate's choice of rate, as it would give
  other parity. The bound W = `next_pow2(next_pow2(n − k) + k)` still holds:
  the high-rate decoder works on exactly W shards and its encoder on fewer.
- Remediation: explain the bound by the high-rate algorithm alone, and
  correct the sentence on case A.
- Verified: by a fresh agent, by reading `rate_high.rs` and computing both
  work counts for every 2 ≤ k ≤ 64, k < n ≤ 128, with no case over W; case
  A's measurements and its 2080 MiB estimate fit the high-rate algorithm.
- Status: fixed in `96efd53` (#50), in the decision. The held code of
  decision 0011 had the same wording in two comments, corrected there.

### W-057 (cleanup): decision 0011 says the manifest alone can pass the budget

- Where: `decisions/0011-memory-budget.md`, "Decision outcome".
- What: "at very small segments the manifest (n × 32 bytes a segment) can
  itself pass the budget". A store reads no manifest over
  `MAX_MANIFEST_LEN`, 256 MiB, and `seal` makes none, so the estimate of a
  file with tiny segments stays near 288 MiB, under the 1 GiB default.
- Remediation: say the manifest term is at most 256 MiB, so only a lower
  `--max-memory` refuses a file for it.
- Verified: by a fresh agent, by reading `store.rs` and `file.rs`.
- Status: fixed in `96efd53` (#50).

### W-058 (low): no test fails when the terminal check is removed

- Where: `crates/neoblast/src/main.rs`, `link_or_stdin` (W-050, `3b93b8d`,
  #46).
- What: W-050's worklist says the check is not tested in CI, "which has no
  terminal". But `script` makes a pseudo-terminal without one, and a
  parameter for the result of `is_terminal()` would let a unit test reach
  the branch. A mutant that disables the check passes every test.
- Remediation: a test that fails when the check is removed, by a unit test
  of the branch or the CLI under `script`.
- Verified: by a fresh agent, with that mutant and `cargo test -p
  neoblast`; the tip binary under `script -q /dev/null` refused at once.
- Status: open. It changes `crates/`, so it waits until after v0.0.3.

### W-059 (cleanup): the spec's largest segment is 64 bytes a shard short

- Where: `spec/header.md`, "Check the codeword" (`3b93b8d`, #46).
- What: "at most 128 × 32 MiB (k = 2, 64 MiB segments)"; a 64 MiB segment
  at k = 2 has shards of 33,554,496 bytes, 32 MiB and 64 bytes, with the
  tag and the alignment. Decision 0011 rounds the same way.
- Remediation: the exact figure, and "about 4 GiB".
- Verified: by a fresh agent, from `layout::shard_len`.
- Status: fixed in `96efd53` (#50), in the spec and the decision.

### W-060 (cleanup): two messages for giving no link

- Where: `crates/neoblast/src/main.rs`, `link_or_stdin` (W-050).
- What: a terminal on stdin gets "no link: give it as an argument, or on
  stdin from a file or a pipe"; an empty file or pipe gets "no link: give
  it as an argument, or on stdin". Both mean the same, so they should not
  drift apart.
- Remediation: one message for both.
- Verified: by a fresh agent, by reading.
- Status: open. It changes `crates/`, so it waits until after v0.0.3.

### W-061 (low): the README's `umask` does not protect a link file that already exists

- Where: `README.md`, "What works today" (W-051, `3b93b8d`, #46).
- What: "`umask 077` makes it readable by you alone". A redirect into a
  file that exists empties it but keeps its mode, so a `photo.link` left
  readable by others from an earlier run stays so, with the new read link
  in it. The security review found the same.
- Remediation: say that it applies to a new file, and to remove an old one
  first.
- Verified: by both reviews and a fresh agent, from POSIX `open(2)`: a
  file's mode is set only when it is made.
- Status: fixed in `96efd53` (#50). Documentation only.

### W-062 (cleanup): decision 0011 says "up to k = 2"

- Where: `decisions/0011-memory-budget.md`, "Context and problem statement".
- What: k = 2 is the smallest k the header allows, not the largest.
- Remediation: "down to k = 2".
- Verified: by a fresh agent, against `spec/header.md`.
- Status: fixed in `96efd53` (#50).

### W-063 (cleanup): decision 0011's first "Good" claims more than its estimate supports

- Where: `decisions/0011-memory-budget.md`, "Consequences".
- What: "a link can no longer make a reader or healer that uses the CLI
  run out of memory by default". The estimate rests on one run of each case
  on one machine, as the second "Bad" says, and a machine with less than
  1 GiB free can still run out.
- Remediation: say what the default bounds, and that it holds as far as
  the estimate does.
- Verified: by a fresh agent, by reading.
- Status: fixed in `96efd53` (#50).

### W-064 (low): the thirteenth reviews say they ran during the campaign

- Where: both thirteenth log entries, this worklist's status paragraph,
  `plans/completed/v0.0.3.md` step 5, and `96efd53`'s message (#50).
- What: they say the reviews ran "during the v0.0.3 campaign". The
  records show it ran from 2026-11-06 19:13 to 2026-11-07 19:13 UTC; the
  reviews ran on 2026-11-08. Nothing rests on it: the plan's steps hold
  whenever they ran, and their fixes changed nothing in `code_id`.
- Remediation: correct the plans; the next log entries correct the old
  ones, which are not rewritten, and the commit message stays.
- Verified: by a fresh agent, from the records and the entries' dates.
- Status: fixed in `9ccdaad` (#52), the commit `v0.0.3` tags, in this
  worklist, the plan and the fourteenth log entries.

### W-065 (cleanup): decision 0011 says the codec works on "about as many again" shards

- Where: `decisions/0011-memory-budget.md`, "Context and problem statement".
- What: true for an intact open or heal, whose encoder works on k rounded
  up to a multiple of P, n at 2-of-128; but its decoder works on W, up to
  2n (256 at 2-of-128), as case A's "open, last k" shows (1518 MiB,
  about 3n shards). The paragraph after states W exactly, and the
  estimate, 2n + W, bounds it, so nothing else is wrong.
- Remediation: "as many again, or twice as many to decode".
- Verified: by a fresh agent, from `rate_high.rs` and the decision's
  table.
- Status: open. `decisions/` is not among the files the release commit may
  change, so it waits until after v0.0.3.

### W-066 (low): the README's quick-start still writes into a link file that exists

- Where: `README.md`, "What works today" (W-061, `96efd53`, #50).
- What: W-061 says in prose to remove an old `photo.link` first; the
  snippet people copy still writes into one that exists, keeping its mode,
  or through a symlink to one.
- Remediation: `set -C` in the snippet's subshell, so the redirect
  refuses a file that exists, and `seal` does not run.
- Verified: by a fresh agent under `sh`, bash 3.2, zsh 5.9 and dash: with
  `set -C` a new file is made with mode 0600, and an existing file, a
  symlink to one and a dangling symlink are each refused and left as they
  were; a symlink to a device such as `/dev/null` is still written.
- Status: open. `README.md` is not among the files the release commit may
  change, so it waits until after v0.0.3.

## Not tracked

Checked by the same reviews and left as they are, with the reason:

- The library's `open_manifest` and `heal_manifest` stay unbounded:
  decision 0011 chose this, and lists a limit in the library among its
  considered options.
- W-052's comment overstates what is freed: refuted, it says what is freed
  and when, not the peak.
- The newest log entries say "the PR after #45": they were written in that
  PR, and entries are not rewritten; the thirteenth entries name it.
- Step 5's review also claimed that `spec/header.md`'s "already needs as
  much" is wrong (refuted: W ≥ n for every k and n, so it is a true lower
  bound); that its figure leaves out n = 128 (refuted: the 128 is n's
  maximum, in place); that the plan's step 3 leaves out #49 (refuted:
  `code_id` is unchanged, and `met` on #50's commit covers it); and that
  v0.0.2 changed the README's status line in the commit it tags (refuted:
  it was `7c06619`, the campaign's records, as here).
