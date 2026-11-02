# Code-review worklist (W-series, part 5)

Status: completed 2026-11-02: every item is fixed, W-044 to W-048 in
`e711e5a` (#42) and W-049 in the PR after #42. Opened 2026-11-02 by the
eleventh `/review-since`
(`8c34bca..805a5ed`, #37 to #41), logged in
`docs/logs/code-review-log.md`. Part 4, W-028 to W-043, is closed
(`plans/completed/code-review-w-series-4.md`). All items are low or
cleanup, and none is a weakness CI can be made to miss. W-046 changes
`crates/`, and so every target's `code_id`; the others are outside it.

IDs are unique across all worklists and continue from the highest W-number used.

## Success criteria

1. W-044: `object.sh tree` fails a top-level name with any control byte
   before its dot, whether `ls-tree` quotes it in octal or as a letter
   escape (`\a \b \t \n \v \f \r`), with cases and a mutant.
2. W-045: which allowed-signers file the signature step reads (`main`'s,
   or none when `main` holds none) is decided in `.github/checks/`, with a
   case for each branch on repositories git makes, and a mutant for each.
3. W-046: a test asserts that an extra shard `restore` returns is ignored,
   and each failing `regenerate` case differs from a passing one in one
   way only.
4. W-047: the UTF-8 locale is looked up in one place.
5. W-048: the UTC step either goes, with `utc.awk` and its cases and
   mutants, or says that after `5393f67` the identity step already implies
   it.
6. W-049: `CLAUDE.md`'s line on the blast-radius rule agrees with
   `decisions/README.md`.
7. Each fix that can be reached has a regression test that fails without
   it, and the golden vectors are unchanged.

## Findings

### W-044 (low): a control byte that `ls-tree` writes as a letter escape hides a dot-name

- Where: `.github/checks/tree.awk`, the `gsub` lines before the dot-name
  and `.mailmap` rules (S-031, `8c34bca`; ASCII-only dot-names,
  `1f30277`).
- What: only octal escapes are removed before the rules, but `ls-tree`
  quotes BEL, BS, TAB, LF, VT, FF and CR as `\a \b \t \n \v \f \r`. On
  trees made with `git mktree` and run through `object.sh tree` as the
  hygiene step does, `"\a.mailmap"`, `"\t.mailmap"`, `"\a.github"` and
  `"\a.ma\304\261lmap"` pass, while `"\001.mailmap"`, `"\177.mailmap"`
  and `".ma\304\261lmap"` fail. The header says a name with an invisible
  character before the dot fails closed. No file system is known to drop
  a leading control byte, and git reads `.mailmap` only by its exact name,
  so nothing known gets a mailmap this way.
- Remediation: remove `\\[abtnvfr]` with the octal escapes, and fail a
  dot-name that held one, as for octal; cases for `\a.mailmap`,
  `\t.mailmap` and `\a.ma\304\261lmap`, and a mutant.
- Verified: by a fresh agent, on crafted trees.
- Status: fixed in `e711e5a` (#42). `tree.awk` removes `\\[abtnvfr]`
  with the octal escapes and fails a dot-name that held one. For each of
  the seven letters, `test.sh` passes a name ending in `docs.mailmap`
  after it, and fails `.mailmap` and a quoted `.github` after it; it also
  fails `"\a.ma\304\261lmap"`, and a tree git makes with a `.mailmap`
  after a BEL. Seven new mutants keep every letter escape, BEL's alone or
  CR's alone, and pass a dot-name with any letter escape, with BEL's alone
  or with CR's alone; the three ASCII mutants are updated.

### W-045 (low): the signature step's choice of allowed-signers file has no test

- Where: `.github/workflows/hygiene.yml`, "Commits and tags are signed
  with the project's key" (S-026, `9f7839a`).
- What: the step reads `main`'s file, the checkout's while `main` has
  never held one, and fails when `main` has dropped it. That choice is
  written only in the workflow, and `mutate.sh` lists it as untested. A
  change that always read the checkout's file would let a pull request's
  own key count, and nothing would fail. W-034 moved the other steps'
  programs into `.github/checks/` for this reason.
- Remediation: a script in `.github/checks/` that writes the file to use,
  run by the step, with cases in `test.sh` on repositories git makes:
  `main` holding the file, never having held it, and having dropped it;
  and a mutant for each branch.
- Verified: by a fresh agent.
- Status: fixed in `e711e5a` (#42). `.github/checks/main-signers.sh`
  prints the file as `main` holds it, from `refs/remotes/origin/main`
  alone, read with `git show-ref --verify` (S-035), and both workflows run
  it. The checkout's file is no longer read while `main` has never held
  one: `main` has held it since `9f7839a` (#40), so that branch could not
  be reached, and `main` without the file fails as before. `test.sh` has 9
  cases on bare repositories git makes: `main` holding the file, beside a
  tag or a branch named `origin/main` or a tag named
  `refs/remotes/origin/main`, passes; no such file, a directory of that
  name, and no `main` with only one of those tags, fail; and an argument
  is a usage error. `mutate.sh` has 6 mutants: `git rev-parse` of the
  short name and of the full name, a missing file passing, `git show` for
  `cat-file blob`, any arguments, and a usage error exiting 1. Not tested:
  the workflows' `if !` around the script. A symlink or an empty file
  there prints a target or nothing, and every signature then fails.

### W-046 (cleanup): W-029's restore test does not pin down an extra shard

- Where: `crates/neoblast-erasure/src/lib.rs`,
  `w029_a_broken_restore_is_a_codec_error` (`06bda62`).
- What: the doc comment says an extra shard that `restore` returns is
  ignored, but no test calls `reconstruct_data` with `Fault::Extra`: a
  fresh agent made any extra index a `Codec` error, and every test passed.
  The test's `regenerate` cases use a codec that breaks `restore` and
  `encode` at once, so they differ from a passing case in two ways; the
  `reconstruct_data` loop and `w029_broken_parity_is_a_codec_error`
  already cover what they catch. Ignoring extras is not new: the code
  before W-029 did too.
- Remediation: assert that `reconstruct_data` with `Fault::Extra` is `Ok`
  and returns the data, and give the test codec a mode that breaks only
  `restore`, for the `regenerate` cases.
- Verified: by a fresh agent, by mutation.
- Status: fixed in `e711e5a` (#42). The test codec is
  `Broken(Part, Fault)`, breaking `encode` or `restore` alone.
  `w029_a_broken_restore_is_a_codec_error` asserts the exact data from
  `reconstruct_data` and `regenerate` for the real codec, for an extra
  shard past the data, and for other bytes returned for a data shard held;
  and a `Codec` error from both for a dropped, long and short shard; the
  parity test breaks only `encode`. Making an extra shard an error,
  preferring a restored shard to one held, dropping the length check,
  making a missing shard `NotEnough` and leaving parity unchecked each
  fail a test. Changes `crates/`, so every target's `code_id`.

### W-047 (cleanup): the UTF-8 locale is looked up twice

- Where: `.github/checks/mutate.sh` line 51 and `test.sh` line 94 (W-040,
  `1f30277`).
- What: the same `locale -a | grep … | head -n 1` is in both. A mutant
  run passes its result to `test.sh` in `CHECKS_UTF8`, so each run agrees
  with itself, but widening one copy alone would make a plain `test.sh`
  run use another locale than the mutants do. Criterion 11 of part 4 asked
  for one lookup.
- Remediation: keep the lookup in one place, for example a mode of
  `test.sh` that prints it, which `mutate.sh` calls.
- Verified: by a fresh agent.
- Status: fixed in `e711e5a` (#42). `test.sh --utf8-locale` prints the
  locale, and both a plain `test.sh` run and `mutate.sh` use it.

### W-048 (cleanup): past `5393f67`, the UTC step checks nothing the identity step does not

- Where: `.github/checks/utc.awk` and the hygiene step "Maintainer
  commits are recorded in UTC" (W-034, `6f9a2e9`).
- What: after `5393f67`, the identity step requires the author and the
  committer to be the project, both in `+0000`, which implies what
  `utc.awk` checks. The UTC step also reads the 11 commits up to
  `5393f67`, whose hashes are fixed, so it can fail no commit the identity
  step passes.
- Remediation: drop the step, `utc.awk` and its cases and mutants; or keep
  them and say in the step that they only re-check fixed history.
- Verified: by a fresh agent.
- Status: fixed in `e711e5a` (#42), by the second remedy: the step's
  comment says that after `5393f67` the identity step implies it, and
  that it also checks `5393f67` and the commits before it. The step,
  `utc.awk`, its cases and its mutants stay.

### W-049 (cleanup): CLAUDE.md states the blast-radius rule more strictly than its source

- Where: `CLAUDE.md`, "Dependencies follow the blast-radius rule", against
  `decisions/README.md` (S-024, `805a5ed`).
- What: `CLAUDE.md` says a crate with few dependents needs a written
  reason, an exact pin and a wrapper trait. The rule it defers to asks for
  a small crate to be questioned, and since S-024 lists `ctutils` as a
  minimum that nothing calls, with no pin and no wrapper. The summary does
  not allow that exception.
- Remediation: the maintainer's call: reword the line to match the README,
  or name the exception for a minimum that nothing calls.
- Verified: by a fresh agent.
- Status: fixed in the PR after #42, by the second remedy, as the
  maintainer chose: `CLAUDE.md` keeps the rule and says that a minimum
  version of a crate nothing calls needs only the written reason, as
  `decisions/README.md` records for `ctutils`. Documentation only.

## Not tracked

Checked by the same review and left as they are, with the reason:

- A commit pushed to `main` that adds a key and is signed with it passes
  the run on that push, which reads the pushed file: the required
  `hygiene` check (S-014) must pass on the commit first, and the runs on a
  pull request or a branch read `main` as it was. Only an edit of
  `hygiene.yml` gets around that, which S-014 already records.
- The UTC and identity steps run once per commit: about 25 ms a commit
  locally for 45 commits, as accepted for the header step in part 4; one
  `--batch` stream would do for all of them when the history grows.
- The `ctutils` floor check would fail if 0.4.3 were yanked: `--precise`
  still selects a yanked version (`cargo help update`, 1.97.0), so only
  its removal from the index would, and a raised floor fails the step
  loudly by design, naming S-024.
- A codec that leaves a data shard out now stops `heal` with `Codec`,
  where it was a segment's `Unrecoverable`: that is W-027's rule for a
  broken codec, and `ReedSolomonSimd` returns no such result.
