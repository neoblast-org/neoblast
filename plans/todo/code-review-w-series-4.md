# Code-review worklist (W-series, part 4)

Status: todo. Opened 2026-10-25 by the sixth `/review-since`
(`255353b..c3d405d`, #23, #25 and #26), logged in
`docs/logs/code-review-log.md`, while the v0.0.2 campaign (step 7d″) ran on
`c3d405d`. Part 3, W-024 to W-027,
is closed (`plans/completed/code-review-w-series-3.md`). All items are low or
cleanup; none blocks v0.0.2. W-028 to W-030 each change `crates/`, and so every target's
`code_id`, so they wait until after v0.0.2 is tagged. The seventh
`/review-since` (`c3d405d..118f710`, the review PR after #26) added W-031,
which is outside `code_id` and can land before the tag. The eighth
(`118f710..f2380e7`, #28, #30 and #31) added W-032 to W-034, all outside
`code_id`: W-032 and W-033 are fixed in #32 (`c618dea`), and W-034
waits until after v0.0.2. The ninth (`f2380e7..c618dea`, #32) added W-035
to W-037, also outside `code_id`, fixed before the tag.

IDs are unique across all worklists and continue from the highest W-number used.

## Success criteria

1. W-028: making `full_sync` fall back after an `EIO`, by any edit, fails a
   test.
2. W-029: a codec result of the wrong count or length is a `Codec` error
   wherever `neoblast-erasure` returns shards, never a store's or a
   publisher's fault.
3. W-030: `review_regressions.rs` has one recording sink and one way to
   rebuild a manifest after changing a sealed file.
4. W-031: the tag object is checked by one awk program, which the hygiene
   job and the Release workflow both run.
5. W-032: every rule of the object checks has a case in `test.sh` and a
   mutant in `mutate.sh` that the case kills.
6. W-033: every commit the worklists cite exists on `main`, and the plan
   names the commit `v0.0.2` tags.
7. W-034: the identity and UTC steps' programs are in `.github/checks/`,
   with cases and mutants.
8. W-035: `object.sh message` fails every message in which git folds a
   line into a `-by:` trailer, carriage returns included, and passes the
   messages it passes today without one.
9. W-036: `LC_ALL=C` is set in one place, and what tests its loss is
   stated as it is.
10. W-037: the commit `v0.0.2` tags changes nothing under `.github/` or in
    `code_id`, and both review logs cover every commit before it.
11. Each fix that can be reached has a regression test that fails without it,
    and the golden vectors are unchanged.

## Findings

### W-028 (low): S-016's test checks the predicate, not the fix

- Where: `crates/neoblast/src/store.rs`, `full_sync` and
  `s016_a_failed_full_sync_is_not_hidden` (#23).
- What: the test checks only `falls_back`. A fresh agent changed the arm
  for an error that does not fall back to run the plain `fsync` anyway
  (`Err(e) if !falls_back(e) => Ok(retry_on_intr(|| rustix::fs::fsync(file))?)`):
  clippy and every test still passed, with S-016's bug back. Deleting the
  arm outright is caught, but only by clippy's dead-code lint.
- Remediation: pass the full sync's result and the plain `fsync` into a
  helper, and test that an `EIO` is returned without the `fsync` running and
  that `ENOTSUP` runs it. A real `EIO` cannot be made in a test.
- Verified: by two fresh agents, by mutation.
- Status: open.

### W-029 (cleanup): a codec's result is checked only for its count, and only in heal

- Where: `crates/neoblast-erasure/src/lib.rs`, `regenerate` and
  `reconstruct_data`; `crates/neoblast/src/stream.rs`, `heal_segments` (#23,
  W-027).
- What: a codec whose `restore` returns too few shards makes
  `reconstruct_data` report `NotEnough` with enough shards, which heal
  reports as `Unrecoverable { have: 2, need: 2 }`, blaming the store. n
  shards of the wrong length pass W-027's count check and are reported as
  `RebuildMismatch`, blaming the publisher. W-027's message says "too few"
  while it checks `!= n`.
- Remediation: check the count and length of every codec result in
  `neoblast-erasure` and return `Codec`; heal's own check can then go.
- Verified: by a fresh agent, with fake codecs in a copy. `ReedSolomonSimd`
  is the only codec and heal names it directly; the proptest and the
  `erasure_restore` fuzz target assert full equality, so nothing reaches it
  today.
- Status: open.

### W-030 (cleanup): two copies of the same test helpers

- Where: `crates/neoblast/tests/review_regressions.rs`: `Recorder` and
  `sealed_as` (#23) repeat the `Sink` and the manifest rebuild in
  `s011_an_inconsistent_segment_does_not_stop_heal`.
- Remediation: `s011` uses `Recorder::new(false)` and `sealed_as`.
- Verified: by a fresh agent.
- Status: open.

### W-031 (cleanup): the tag check is written out twice

- Where: `.github/workflows/hygiene.yml`, "Tags carry only the project's
  identity", and `release.yml`, "Tag is the project's, in UTC" (`118f710`,
  S-022).
- What: the same 11-line awk program is in both steps. A later change to
  one, such as S-026's signature check or S-025's control bytes, can miss
  the other, and the hygiene job and the release gate then disagree about
  what a tag may hold.
- Remediation: move it to a checked-in file, such as
  `.github/scripts/tag-object.awk`, run by both with `awk -f`; the same
  for the commit program if S-025 grows it.
- Verified: by a fresh agent; the two copies are byte-identical.
- Status: fixed in `9b8cefe` (#30). Both workflows run
  `.github/checks/object.sh`, which runs `commit-object.awk` or
  `tag-object.awk`; the programs are the old ones line for line, with
  S-025's line added. An object of only a blank line passes, as it did
  before; git never writes one.
- Tests, in `642ebc0` (#30): the attribution check (S-013, S-018) moved to
  `message.awk`, run as `object.sh message`. Its rules are the old
  program's, but it exits non-zero when it finds a line, where the old one
  exited zero, and the hygiene step tests it with `if !`. `test.sh` runs
  107 cases under every awk on PATH (mawk on GitHub's runner, BSD awk on
  macOS, gawk where installed): crafted commits, tags and messages, each
  differing from an allowed one in one way, and 14 objects git itself
  writes in a throwaway repository with a throwaway SSH key (a signed
  commit, tag and merge pass; a `mergetag`, an `encoding` header, a
  commit not in UTC, a tag of a tag, a two-line tag message, another
  tagger and a co-author, as a trailer or in prose, fail). CI fails if it
  cannot make those, and they cannot reach another repository through
  `GIT_DIR` and the like. `mutate.sh` holds 47 mutants, each bringing
  back a weakness a review closed; it fails if the checks as they are do
  not pass, or if a mutant survives or no longer applies. The hygiene job
  runs both. The mutants it cannot hold, as no
  test tells them apart, are listed in its header. W-032 adds the rules
  that had no case.

### W-032 (cleanup): some rules of the object checks had no case

- Where: `.github/checks/tag-object.awk`, the `!t++`, `!n++` and `!g++`
  guards (S-022, `118f710`); `commit-object.awk`, the upper end of S-025's
  byte class (`9b8cefe`).
- What: a tag with two `type`, `tag` or `tagger` lines had no case, so
  turning `!t++` into `++t`, or the same for `n` and `g`, failed no test.
  The mutant for two object lines removed `!o++` whole, which fails every
  tag for another reason. No commit message held a byte above DEL, so
  widening `\177` to `\177-\377` would have refused every UTF-8 message
  with no test failing.
- Remediation: a case and a mutant for each rule.
- Verified: by a fresh agent, by mutation under mawk, gawk and busybox.
- Status: fixed in #32 (`c618dea`): cases for two `type`, `tag`
  and `tagger` lines and a UTF-8 message, and mutants that keep each guard
  counting (`++t`) or widen the class. With them, two smaller fixes:
  `tag-object.awk` compares the message to the tag's name as a string, as
  a tag named `1` passed with `1.0` as its message, and `object.sh` sets
  `LC_ALL=C` for `tr` too, which on macOS printed "Illegal byte sequence"
  on bytes outside UTF-8 (it failed closed; a Latin-1 case catches it on
  macOS). `test.sh` runs 120 cases and `mutate.sh` 57 mutants, with
  S-028's.

### W-033 (cleanup): the worklists cite commits that are not on `main`

- Where: W-031's status in this worklist and S-025's in
  `security-review-s-series.md` (#30); `plans/todo/v0.0.2.md`, rows 7d‴,
  7e and 8; `CHANGELOG.md`, the date of 0.0.2 (#28).
- What: `14989d0` and `7f8b3e9` are the branch's commits from before its
  rebase onto #28, which no branch holds; `main` has `9b8cefe` (the fix)
  and `642ebc0` (the tests). W-031 said `message.awk` moved line for line,
  though its exit status was reversed. The plan still named 7d‴ (`57b7b54`)
  as the commit `v0.0.2` tags, with four commits after it, and the
  CHANGELOG dated 0.0.2 to 2026-10-26, a day before any tag.
- Remediation: cite the commits on `main`; tag the review PR after #31;
  date 0.0.2 to the day it is tagged.
- Verified: by a fresh agent (`git cat-file -t` on each hash).
- Status: fixed in #32 (`c618dea`).

### W-034 (cleanup): two hygiene steps still hold their own awk

- Where: `.github/workflows/hygiene.yml`, "Maintainer commits are recorded
  in UTC" and "Commits carry only the project's identity" (S-008, before
  the range).
- What: their programs, such as the trailer `gsub` and the `NF != 6`
  guard, have no case and no mutant. CLAUDE.md asks for both for a new or
  changed check; these did not change, so they fall outside its letter
  but not its point.
- Remediation: move them to `.github/checks/`, with cases and mutants, as
  W-031 did for the others.
- Verified: by a fresh agent.
- Status: open; outside `code_id`, not needed for v0.0.2.

### W-035 (low): a carriage return hides a folded attribution line

- Where: `.github/checks/object.sh`, the message line, and `message.awk`'s
  fold rule (`c618dea`, S-028).
- What: git splits a message into lines only at `\n`, and folds a line
  that starts with a space, tab or carriage return into the trailer above
  it. `object.sh` first drops one CR before each newline and turns every
  other CR into a newline, so `message.awk` sees a blank line where git
  sees none, and its `by` flag is reset. `object.sh message` passes
  `Reviewed-by: <project>\r\r\n <other>`, `…\n\r<other>`, `…\n \r<other>`
  and `…\n\r <other>`, each of which `git interpret-trailers --parse`
  reads as one `Reviewed-by` line naming the other party. S-028's CRLF
  case uses a single CR, which the pre-pass was written for.
- Reach: none in CI. `commit-object.awk`'s control-byte class refuses a CR
  anywhere in a commit, and the hygiene job runs that check over the same
  commits; the gap is in `object.sh message` on its own, which `test.sh`
  claims to cover.
- Remediation: `message.awk` reads git's lines as they are: the fold rule
  treats CR as a blank (`/^[ \t\r]+[^ \t\r]/`), and the `-by:` rule runs on
  each part of a line split at CR (S-018). The pre-pass and `tr` go, and
  with them a process per commit. A case for each input above, and mutants
  for a fold rule that ignores CR and for a line not split at CR.
- Verified: by a fresh agent, with `git interpret-trailers --parse` and
  real commits through both hygiene steps, under macOS awk; a prototype
  passed `test.sh` and failed all four inputs.
- Status: fixed in the PR after #32. `message.awk` reads git's lines,
  folds on a space, tab or carriage return (git does not fold on `\v` or
  `\f`), and splits each line at a carriage return for the `-by:` rule;
  `object.sh` passes the message to it as it is. A `-by:` value runs on to
  the end of git's line, CRs and all, as git reads it: the second agent's
  review found `Reviewed-by: <project>\r <other>`, which the first version
  of this fix passed, and `…\r<other>`, which `c618dea` passed too.
  `test.sh` fails the four inputs above, the CR inside a line, and a commit
  git writes with a CR-folded line, and passes a line of blanks and a CR;
  mutants for a fold that ignores CR, a line not split at CR, a value that
  ends at a CR and a CR that counts replace the two `object.sh` ones. The
  24 commits since `5393f67` still pass.

### W-036 (cleanup): `LC_ALL=C` is set five times, and its loss is caught only on macOS

- Where: `.github/checks/object.sh`, the `export LC_ALL=C` and the four
  `LC_ALL=C awk` prefixes; `mutate.sh`'s header (`c618dea`, W-032).
- What: the export makes the prefixes do nothing, so removing any one of
  them fails no test. Removing the export fails `test.sh`'s Latin-1 case
  only with BSD `tr` under a UTF-8 locale: on GitHub's runner GNU `tr`
  reads bytes in any locale. `mutate.sh`'s header says the Latin-1 case
  catches it, without saying where.
- Remediation: keep the export and drop the prefixes; have the header say
  the case catches the export's loss only on macOS, and, where `locale -a`
  lists a UTF-8 locale, run that case under it so the result does not
  depend on the caller's.
- Verified: by a fresh agent, by mutation on a copy, under UTF-8 and C
  locales; the Linux side by reasoning.
- Status: fixed in the PR after #32. `object.sh` sets `LC_ALL=C` once;
  `test.sh` runs the Latin-1 case under `C.UTF-8` or `en_US.UTF-8` where
  `locale -a` lists one, and `mutate.sh` removes the export as a mutant
  only where `tr` refuses a byte outside UTF-8 (macOS's), its header
  saying why.

### W-037 (low): the amended release gate lets the tagged commit change the release checks

- Where: `plans/todo/v0.0.2.md`, criterion 10's amendment and row 8
  (`c618dea`).
- What: the amendment let the commit `v0.0.2` tags be the review PR that
  logs the last reviews, its own changes reviewed by an agent before merge
  and logged only by the next `/review-since`. Row 8 named `c618dea`,
  which changes `tag-object.awk` and `object.sh`; the Release workflow runs
  both, from that commit, on the tag, while neither review log covered
  them.
- Remediation: the tagged commit changes only `docs/logs/`, `plans/` and
  `CHANGELOG.md`, and logs the reviews of every commit before it; the
  amendment and row 8 say so. `c618dea` is not tagged.
- Verified: by a fresh agent, against `release.yml`.
- Status: fixed in the PR after #32: criterion 10's amendment and rows
  7e and 8 of `plans/todo/v0.0.2.md` say so.

## Not tracked

Checked by the same review and left as they are, with the reason:

- `seal_stream` calls `ShardSink::finish` after a failed run (W-025): the
  documented meaning is "make every shard stored so far durable", not a
  commit; `finish` is new in 0.0.2, `heal_manifest` already did the same,
  and the sync is bounded by the last segment's directories.
- An `EIO` from an `F_FULLFSYNC` that a file system does not offer would now
  fail (S-016): xnu's default answer is `ENOTSUP`, Apple's smbfs answers
  `ENOTSUP` for directories, the open-source FUSE kext passes the request
  to the daemon's fsync, and probes on APFS, HFS+, FAT32, exFAT, FSKit and
  devfs gave no `EIO`. Closed-source macFUSE could not be checked; a report
  of one would reopen it.
- `write_manifest_bytes`'s `Unsynced` can never hold anything: W-026 says so;
  it guards a later change to `create_dirs`.
- The CHANGELOG dates 0.0.2 and claims a campaign, which ran on `86b8a67`,
  not `c3d405d`: step 7d‴ re-dates it and re-checks that sentence against
  7d″'s records.
- From the seventh review (`c3d405d..118f710`):
  - No regression test for S-021 to S-023: no test exists for a workflow
    check, and S-012 to S-017 and S-022 were checked by hand on crafted
    objects, as here. Since W-031, the object checks of S-015, S-017,
    S-022 and S-025 have one (`.github/checks/test.sh`).
  - One `git cat-file` per commit in the header step: 18 commits today;
    one `--batch` stream would do when the history grows.
  - The header step's author and committer pattern repeats the identity
    step's rule: one reads git's parsed fields and the other the object as
    stored, on purpose.
  - Rejected: the log's and the worklist's counts for the sixth review
    were said not to add up; they do, as W-028 came from the security
    review.
  - Rejected: the CHANGELOG's `cargo update -p ctutils --precise 0.4.5`
    for a `rust-version` below 1.87 was said to be wrong; `ctutils` 0.4.3
    to 0.4.5 all need Rust 1.87, so resolver 3 keeps 0.4.2 without it.
  - Rejected: `verify`'s `deny` reads only the root lock file; that is the
    graph that is published, and CI's `deny` job did the same before the
    range.
- From the eighth review (`118f710..f2380e7`):
  - `mutate.sh` runs all of `test.sh` once per mutant: the hygiene step
    takes about 50 s on GitHub's runner, against about 1 min 40 s for CI
    and 4 min 30 s for CodeQL on the same push. Its first run of `test.sh`
    is what stops a broken environment from killing every mutant.
  - S-025's byte class lets through C1 controls written in UTF-8 (U+009B,
    U+0085): they hide nothing from git or the message check, and names in
    prose are not checked by design.
  - `commit-object.awk` does not require a tree, author or committer: git
    writes no such commit, `git log` fails the step on a commit with no
    tree, and the identity step fails one with no author or committer.
- From the ninth review (`f2380e7..c618dea`):
  - The fold rule fails a prose `-by:` line followed by an indented one,
    where git reads no trailer: like S-013's rule, it applies anywhere in
    the message, and fails closed.
  - The git-made folded case does not ask git whether it folds the line:
    if git stopped folding, the check would only be stricter than it needs
    to be.
  - `mutate.sh`'s ten new mutants make it about 40 s slower: as for the
    eighth review's item on its cost.
