# Code-review worklist (W-series, part 6)

Status: completed 2026-11-03: every item is fixed, or won't be fixed with
its reason, in the PR after #45. Opened 2026-11-03 by the twelfth
`/review-since` (`805a5ed..0572592`, #42 to #45), logged in
`docs/logs/code-review-log.md`, as step 2 of `plans/todo/v0.0.3.md`. Part
5, W-044 to W-049, is closed (`plans/completed/code-review-w-series-5.md`).
All items are low or cleanup, and none blocks v0.0.3. W-050, W-052, W-054
and W-055 change `crates/`, and so every target's `code_id`; they land
before the v0.0.3 campaign.

IDs are unique across all worklists and continue from the highest W-number used.

## Success criteria

1. W-050: with the link left out or `-` and stdin a terminal, `open` and
   `heal` refuse at once with a plain error, instead of waiting with no
   prompt; a file or a pipe still works.
2. W-051: the README's quick-start writes the read link to a file only the
   user can read, and says why.
3. W-052: `open` frees a segment's parity once it is checked, and each data
   shard once it is copied into the output, with its comment saying so.
4. W-053: decided, with the reason recorded.
5. W-054: `heal` hashes each rebuilt shard once.
6. W-055: the code and the changelog say which copy of the key is not wiped.
7. Every test passes, and each change in behaviour has a test or says why it
   cannot have one; the golden vectors are unchanged.

## Findings

### W-050 (low): with no link and a terminal on stdin, `open` and `heal` wait silently

- Where: `crates/neoblast/src/main.rs`, `link_or_stdin` (S-003, `ef16ebb`).
- What: the link became optional, so clap no longer reports it missing. At
  a terminal, `read_until` waits with nothing on screen, and a link pasted
  there is shown as it is typed, which S-003 meant to avoid.
- Remediation: refuse when stdin is a terminal, as before S-003 clap did.
- Verified: by a fresh agent, by reading.
- Status: fixed in the PR after #45. With the link left out or `-`,
  `link_or_stdin` refuses a terminal on stdin with "no link: give it as an
  argument, or on stdin from a file or a pipe"; both `--help`s say "a file
  or a pipe". Checked by hand under a pseudo-terminal (`script`) for `open`
  with the link left out and `heal -`: both refuse at once and write
  nothing; a pipe still opens the file. Not tested in CI, which has no
  terminal; the pipe cases of `s003_open_and_heal_read_the_link_from_stdin`
  still pass.

### W-051 (low): the README's quick-start writes the read link to a file others may read

- Where: `README.md`, "What works today" (S-003, `ef16ebb`).
- What: `neoblast seal … >photo.link` creates the file with the user's
  umask, usually readable by every local user, who can then decrypt the
  file. That outlasts the command line it replaced.
- Remediation: `(umask 077; …)`, and a sentence saying why.
- Verified: by a fresh agent, by reading (`seal` prints the link, and
  nothing sets the file's mode).
- Status: fixed in the PR after #45. Documentation only.

### W-052 (cleanup): `open` keeps the rebuilt parity while it fills the output

- Where: `crates/neoblast/src/stream.rs`, `open_manifest` (S-002, `ecc91cb`).
- What: the n rebuilt shards stay alive while the output buffer is
  allocated and filled, though the parity is needed only for the check;
  the comment reads as if they were freed.
- Remediation: drop the parity after the check, and each data shard as it
  is copied.
- Verified: by a fresh agent, by reading.
- Status: fixed in the PR after #45: `all.truncate(k)` after the check, and
  the data shards are consumed as they are copied; the comment says so. It
  keeps behaviour, so every open test covers it, among them
  `s002_shards_that_are_not_one_codeword_are_refused` and the CLI round
  trips. The memory saved is not tested.

### W-053 (cleanup): `regenerate` copies the data shards an intact open used to move

- Where: `crates/neoblast-erasure/src/lib.rs`, `reconstruct_data`, which
  `regenerate` calls (S-002, `ecc91cb`).
- What: every held data shard is cloned into the result; the fast path
  that S-002 removed moved them.
- Remediation: a `regenerate` that takes the shards by value.
- Verified: by a fresh agent, by reading.
- Status: won't fix. It is one copy of the segment's data, small next to
  the encode and the hashing that S-002 added, and decision 0010's
  measurements include it. Removing it would change `neoblast-erasure`'s
  API for a cleanup.

### W-054 (cleanup): `heal` hashes each missing rebuilt shard twice

- Where: `crates/neoblast/src/stream.rs`, `heal_segments` (S-002, `ecc91cb`).
- What: `first_mismatch` hashes every shard not held, the missing ones
  among them, and the loop that stores the missing ones hashes them again.
- Remediation: hash again only after a mismatch.
- Verified: by a fresh agent, by reading.
- Status: fixed in the PR after #45: with no mismatch in the segment, the
  loop stores the missing shards without hashing them again. It keeps
  behaviour: the heal round trips cover the case with no mismatch, and
  `s011_an_inconsistent_segment_does_not_stop_heal` and
  `s019_a_shard_of_the_wrong_length_counts_as_missing` the case with one,
  where a missing shard that differs is still not stored.

### W-055 (low): the key's last copy on the way out of `decode` is not wiped

- Where: `crates/neoblast-format/src/caps.rs`, `decode_32`, and
  `CHANGELOG.md` (S-003, `ef16ebb`).
- What: the key leaves the wiped buffer as a `[u8; 32]` returned by value;
  `FileReadCap` wipes the copy it holds, but one the compiler leaves on the
  stack is not wiped, and the changelog did not say so.
- Remediation: say so; returning `Zeroizing<[u8; 32]>` would not remove the
  copies a move makes either.
- Verified: by a fresh agent, by reading.
- Status: fixed in the PR after #45, in `decode_32`'s doc and the
  changelog. Documentation only.

## Not tracked

Checked by the same review and left as they are, with the reason:

- No test covers heal storing the matching shards of an inconsistent
  segment: refuted, a mutant that skips the segment fails
  `s019_a_shard_of_the_wrong_length_counts_as_missing`.
- Removing the fallback to the checkout's `allowed_signers` breaks a
  fork's CI: refuted for a fork whose `main` holds the file, as
  `fetch-depth: 0` fetches `refs/remotes/origin/main`; failing a `main`
  without it is W-045's intent, recorded there.
