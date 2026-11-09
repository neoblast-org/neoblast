# Code Review Log

<!-- baseline --> **Next review diffs from:** `05bbd95e1238de51c81808a6ffc2f7ce0ea14870` (`main`, recorded 2026-11-08)

A running record of code-review events: branch/pre-PR reviews, review worklists, and
remediation programmes. Newest entry first. The baseline marker line above is the
machine-owned frontier the next incremental review (`/review-since`) starts from.
Security reviews are logged separately in `security-review-log.md`.

**Starting point:** no review has been logged here yet. The baseline is `main`'s
first commit, so the first `/review-since` covers the full history, including that
first commit (the skill's step 3 explains how).

**Not logged here:** the second-AI review of an individual change before it is
committed. Its outcome belongs in the commit message or the change's plan doc. This log
tracks the *distinct* review events: a branch reviewed before a PR, a review that
produced a standing worklist, or a remediation programme working through a review's
findings.

**How to append:** `/review-since` does it automatically, and its skill file
(`.claude/skills/review-since/SKILL.md`) is the sole definition of the mechanics —
this header does not restate them. Logging a frontier-moving review by hand means
following that protocol's steps 5–6 yourself: prepend a full entry AND move the
baseline marker line to the reviewed tip; a frontier-moving entry's Base must equal
the previous one's Tip unless the entry explains why not (an override, or a
merge-base fallback). Side-branch pre-PR reviews are logged as entries marked
"frontier unchanged (side branch)" and never move the line.

## Entries

### 2026-11-08: `281bd1a..05bbd95` (#50 and #51)

| | SHA | Date | Subject |
| --- | --- | --- | --- |
| Base | `281bd1a6b7d9513a635a9af14bbccd9ebc465df8` | 2026-11-07 | docs: decision 0011, a memory budget for the CLI (S-037) (#49) |
| Tip | `05bbd95e1238de51c81808a6ffc2f7ce0ea14870` | 2026-11-08 | fuzz: 24 h campaign on 3b93b8d; all five targets clean (#51) |

- **Scope:** 2 commits (`git rev-list --count 281bd1a..05bbd95`) and 17
  files, +555/−39 (`git diff --shortstat 281bd1a 05bbd95`): the
  thirteenth reviews' logs and fixes (#50), and the v0.0.3 campaign's
  records with the README's status line (#51). Of them, 10 files, +270,
  under `fuzz/`, all records, and none under `crates/` or `.github/` (the
  same command with `-- fuzz`, `-- crates` and `-- .github`). Base equals
  the previous entry's Tip. Only the committed range is covered. This is
  step 5 of `plans/todo/v0.0.3.md`, logged in the commit `v0.0.3` tags.
- **Method:** the `code-review` skill over the range, at high effort; a
  fresh agent then tried to refute each claim, from the records, the
  entries' dates, `rate_high.rs`, and `set -C` under four shells. Of the
  review's 6 claims, 3 are tracked and 3 rejected.
- **Findings:** 3 in the open worklist, none a bug: W-064 (low), the
  thirteenth reviews say they ran during the campaign, which had ended
  the day before; W-065 (cleanup), decision 0011 says the codec works on
  "about as many again" shards, when decoding takes up to twice as many;
  W-066 (low), the README's quick-start still writes into a link file
  that exists.
- **Corrections to the entry below:** it says it ran during the v0.0.3
  campaign; the campaign ran from 2026-11-06 19:13 to 2026-11-07 19:13
  UTC, and that review on 2026-11-08 (W-064).
- **Not tracked,** with the reasons in the worklist: the spec's "already
  needs as much" (a true lower bound), its figure without n = 128 (stated
  in place), the plan's step 3 without #49 (`code_id` unchanged), and the
  README's status line before the tag (as v0.0.2 did, in `7c06619`).
- **Checked, no finding:** the records' `code_id` is the one
  `fuzz/campaign.sh summary` computes at the Tip, which shows `met` ×5;
  nothing in `code_id` changed since `3b93b8d`.
- **Outcome:** no blocker. W-064 is fixed in the commit `v0.0.3` tags;
  W-065 and W-066 change files that commit may not, and wait until after
  v0.0.3. Worklist:
  [`plans/todo/code-review-w-series-7.md`](../../plans/todo/code-review-w-series-7.md).
- **Carried-over items:** W-056, W-057, W-059 and W-061 to W-063 fixed in
  `96efd53` (#50), which the entry below calls "the PR after #49"; W-058
  and W-060 open, after v0.0.3.

### 2026-11-08: `0572592..281bd1a` (#46 and #49)

| | SHA | Date | Subject |
| --- | --- | --- | --- |
| Base | `0572592f6469c003a8ba9f7c14ca9fd0ef910a42` | 2026-11-03 | chore: bump to 0.0.3; plan the release (#45) |
| Tip | `281bd1a6b7d9513a635a9af14bbccd9ebc465df8` | 2026-11-07 | docs: decision 0011, a memory budget for the CLI (S-037) (#49) |

- **Scope:** 2 commits (`git rev-list --count 0572592..281bd1a`) and 15
  files, +481/−37 (`git diff --shortstat 0572592 281bd1a`): the twelfth
  reviews' fixes and logs (#46), and decision 0011 (#49). Of them, 3
  files, +35/−21, under `crates/` and none under `.github/` (the same
  command with `-- crates` and `-- .github`). Base equals the previous
  entry's Tip. Only the committed range is covered. Run during the v0.0.3
  campaign (`plans/todo/v0.0.3.md`, step 3), so that step 5 reviews only
  what follows.
- **Method:** the `code-review` skill over the range, at high effort; a
  fresh agent then tried to refute each claim, reading the code at the
  Tip, reading `reed-solomon-simd`'s high-rate sizing and computing it for
  every k and n the header allows, and running a mutant of
  `link_or_stdin` on a copy. Of the review's 9 claims, 6 are tracked (W-061
  also raised by the security review) and 3 rejected; the security
  review's two wording points on decision 0011 are tracked here too, as
  W-062 and W-063.
- **Findings:** 8 in a new code worklist, none a bug:
  W-056 (low), decision 0011 explains its bound by a rate the codec never
  runs; W-057 (cleanup), it says the manifest alone can pass the budget;
  W-058 (low), no test fails when W-050's terminal check is removed;
  W-059 (cleanup), the spec's largest segment is 64 bytes a shard short;
  W-060 (cleanup), two messages for giving no link; W-061 (low), the
  README's `umask` does not protect a link file that already exists;
  W-062 (cleanup), "up to k = 2"; W-063 (cleanup), decision 0011's first
  "Good" claims more than its estimate supports.
- **Not tracked,** with the reasons in the worklist: the library left
  unbounded (decision 0011's choice), W-052's comment (it says what is
  freed, not the peak), and the twelfth entries' "the PR after #45",
  which is `3b93b8d` (#46).
- **Checked, no finding:** W-052 and W-054 keep behaviour: with no
  mismatch, `first_mismatch` has hashed every shard heal stores, and
  mutants of the new filter fail the S-011 and S-019 tests;
  `cargo test --all` (106 passed) at the Tip.
- **Outcome:** no blocker. W-056, W-057, W-059 and W-061 to W-063 are
  fixed in the PR after #49, outside `code_id`; W-058 and W-060 change
  `crates/` and wait until after v0.0.3. Worklist:
  [`plans/todo/code-review-w-series-7.md`](../../plans/todo/code-review-w-series-7.md).
- **Carried-over items:** W-050 to W-052, W-054 and W-055 fixed, and
  W-053 won't be, in `3b93b8d` (#46), which closed part 6.

### 2026-11-03: `805a5ed..0572592` (#42 to #45)

| | SHA | Date | Subject |
| --- | --- | --- | --- |
| Base | `805a5ed727555de816d98a76b8711e7be0f9e6b9` | 2026-11-02 | fix: neoblast-crypto requires ctutils 0.4.4 or later (S-024) (#41) |
| Tip | `0572592f6469c003a8ba9f7c14ca9fd0ef910a42` | 2026-11-03 | chore: bump to 0.0.3; plan the release (#45) |

- **Scope:** 5 commits (`git rev-list --count 805a5ed..0572592`) and 30
  files, +1275/−158 (`git diff --shortstat 805a5ed 0572592`): the
  eleventh reviews' fixes (#42); S-003 and W-049 (#43); S-002 (#44); and
  the 0.0.3 bump with the release plan (#45). Of them, 6 files, +189/−39,
  under `.github/` and 7 files, +422/−73, under `crates/` (the same
  command with `-- .github` and `-- crates`). Base equals the previous
  entry's Tip. Only the committed range is covered. This is step 2 of
  `plans/todo/v0.0.3.md`.
- **Method:** the `code-review` skill over the range, at high effort; a
  fresh agent then tried to refute each claim, reading the code at the
  Tip and running a mutant of `heal_segments` on a copy. Of the review's
  8 claims, 6 are tracked and 2 rejected.
- **Findings:** 6 in a new code worklist, none a weakness CI can be made
  to miss: W-050 (low), with no link and a terminal on stdin, `open` and
  `heal` wait silently; W-051 (low), the README's quick-start writes the
  read link to a file others may read; W-052 (cleanup), `open` keeps the
  rebuilt parity while it fills the output; W-053 (cleanup), `regenerate`
  copies the data shards an intact open used to move; W-054 (cleanup),
  `heal` hashes each missing rebuilt shard twice; W-055 (low), the key's
  last copy on the way out of `decode` is not wiped.
- **Not tracked,** with the reasons in the worklist: a missing test for
  heal storing an inconsistent segment's matching shards (a mutant fails
  the S-019 test), and the removed fallback to the checkout's signers
  file (W-045's intent).
- **Checked, no finding:** no correctness bug in the range;
  `cargo test --all` (106 passed) at the Tip; the bump changes only the
  `neoblast-*` versions.
- **Outcome:** no blocker. W-050 to W-052, W-054 and W-055 are fixed, and
  W-053 won't be, in the PR after #45, before the v0.0.3 campaign; W-050,
  W-052, W-054 and W-055 change `crates/`, and so `code_id`. Worklist:
  [`plans/completed/code-review-w-series-6.md`](../../plans/completed/code-review-w-series-6.md),
  closed in the same PR.
- **Carried-over items:** W-044 to W-048 fixed in `e711e5a` (#42) and
  W-049 in `5d2cdf6` (#43). Part 5 closed and moved to
  [`plans/completed/code-review-w-series-5.md`](../../plans/completed/code-review-w-series-5.md)
  in #43; the entry below still links to its old place.

### 2026-11-02: `8c34bca..805a5ed` (#37 to #41)

| | SHA | Date | Subject |
| --- | --- | --- | --- |
| Base | `8c34bcaf796de268b55a3d7ed7b46821c8959af8` | 2026-10-28 | fix: S-030 and S-031; log the security review of #32 and #33 (#36) |
| Tip | `805a5ed727555de816d98a76b8711e7be0f9e6b9` | 2026-11-02 | fix: neoblast-crypto requires ctutils 0.4.4 or later (S-024) (#41) |

- **Scope:** 8 commits (`git rev-list --count 8c34bca..805a5ed`) and 33
  files, +1346/−217 (`git diff --shortstat 8c34bca 805a5ed`): the v0.0.2
  release (#37); the plan's close-out, the action bumps, S-034, W-038 to
  W-043 and W-028 to W-030 (#38); W-034 (#39); S-026 and S-029 (#40); and
  S-024 (#41). Of them, 15 files, +597/−103, under `.github/` and 5 files,
  +220/−54, under `crates/` (the same command with `-- .github` and
  `-- crates`). Base equals the previous entry's Tip. Only the committed
  range is covered.
- **Method:** the `code-review` skill over the range, which also ran
  `test.sh`, `signed.sh` on every commit from `f2380e7` and every tag, and
  `neoblast-erasure`'s tests; two fresh agents then tried to refute each
  claim, one for `.github/` (trees made with `git mktree` run through
  `object.sh tree`, the steps timed, the ruleset of S-014 read) and one
  for `crates/` and `Cargo.toml` (four mutants of `neoblast-erasure` on a
  copy). Of the code review's 10 claims, 6 are tracked and 4 rejected.
- **Findings:** 6 in a new code worklist, none a weakness CI can be made
  to miss: W-044 (low), a control byte that `ls-tree` writes as a letter
  escape hides a dot-name; W-045 (low), the signature step's choice of
  allowed-signers file has no test; W-046 (cleanup), W-029's restore test
  does not pin down an extra shard; W-047 (cleanup), the UTF-8 locale is
  looked up twice; W-048 (cleanup), past `5393f67` the UTC step checks
  nothing the identity step does not; W-049 (cleanup), `CLAUDE.md` states
  the blast-radius rule more strictly than its source.
- **Not tracked,** with the reasons in the worklist: a key
  that authorizes itself on a push to `main` (the required check reads
  `main` as it was); the cost of the per-commit steps; the `ctutils`
  floor check after a yank; and `heal` stopping on a broken codec
  (W-027's rule).
- **Checked, no finding:** no correctness bug in the range; the 34
  commits since `5393f67` and every tag pass every check, `signed.sh`
  included; `test.sh` ran 239 cases with none wrong; the signature layout
  positions S-034 checks match SSHSIG.
- **Outcome:** no blocker. W-046 changes `crates/`, and so `code_id`;
  the others are outside it.
  Worklist:
  [`plans/todo/code-review-w-series-5.md`](../../plans/todo/code-review-w-series-5.md).
- **Carried-over items:** W-028 to W-030 and W-038 to W-043 fixed in
  `06bda62` and `1f30277` (#38), and W-034 in `6f9a2e9` (#39), which their
  statuses had as "the PR after #37" and "the PR after #38". Every item of
  part 4 is fixed, so it is closed and moved to
  [`plans/completed/code-review-w-series-4.md`](../../plans/completed/code-review-w-series-4.md);
  the entries below still link to its old place.

### 2026-10-28: `c618dea..8c34bca` (#33 and #36)

| | SHA | Date | Subject |
| --- | --- | --- | --- |
| Base | `c618dea9e15535f3de44ed34d4e1522d80e756ab` | 2026-10-27 | fix: review findings from #28, #30 and #31; log the eighth reviews (#32) |
| Tip | `8c34bcaf796de268b55a3d7ed7b46821c8959af8` | 2026-10-28 | fix: S-030 and S-031; log the security review of #32 and #33 (#36) |

- **Scope:** 2 commits (`git rev-list --count c618dea..8c34bca`) and 16
  files, +609/−98 (`git diff --shortstat c618dea 8c34bca`): W-035 to
  W-037's fixes and the code log of #32 (#33), and S-030 and S-031's
  fixes, `SECURITY.md` and the security log of #32 and #33 (#36), of which
  9 files, +267/−72, under `.github/` (the same command with `-- .github`)
  and none under `crates/`. Base equals the previous entry's Tip. Only the
  committed range is covered.
- **Method:** the `code-review` skill over the range, which also ran every
  check over the 26 commits since `5393f67` and every tag, `test.sh`, and
  mutants of its own; a fresh agent then tried to refute each claim,
  decoding a real signature, applying mutants to a copy of
  `.github/checks/` under mawk, gawk, busybox and macOS awk, and timing
  `mutate.sh` and `message.awk`. Of the code review's 7 claims, all are
  tracked, the two about `mutate.sh`'s locale together as W-040.
- **Findings:** 6 in the code worklist, none a weakness CI can reach:
  W-038 (low), the signature's magic and namespace have no case; W-039
  (cleanup), a signature with an empty body is not tested; W-040 (low),
  what `mutate.sh` says about the locale is not true of every awk; W-041
  (cleanup), two workflow comments predate `ssh-signature.awk`; W-042
  (low), `message.awk` takes a value in time that grows with the cube of a
  line's CRs; W-043 (cleanup), `mutate.sh` remakes git's objects for every
  mutant.
- **Checked, no finding:** no correctness bug in the range; the 26 commits
  since `5393f67` and every tag pass every check; `test.sh` ran 153 cases
  with none wrong under each awk; removing `LC_ALL=C` fails `test.sh` on
  macOS; on `8c34bca`'s CI the hygiene job ran every step, "No mailmap"
  among them, and `mutate.sh` killed all 77 mutants; nothing under
  `crates/`, so `code_id` is the same as on `c3d405d`.
- **Outcome:** no blocker. W-038 to W-043 change only `.github/` and
  wait until after v0.0.2, as no fix is needed for the release (W-037).
  Worklist:
  [`plans/todo/code-review-w-series-4.md`](../../plans/todo/code-review-w-series-4.md).
- **Carried-over items:** W-028 to W-030 and W-034 open, waiting until
  after v0.0.2; W-035 to W-037 fixed in `ee97e24` (#33), which their
  statuses had as "the PR after #32".

### 2026-10-27: `f2380e7..c618dea` (#32)

| | SHA | Date | Subject |
| --- | --- | --- | --- |
| Base | `f2380e7ff4ac4708d9dc8599915feb49397b4c98` | 2026-10-26 | ci: pin dtolnay/rust-toolchain to 683b588 (#31) |
| Tip | `c618dea9e15535f3de44ed34d4e1522d80e756ab` | 2026-10-27 | fix: review findings from #28, #30 and #31; log the eighth reviews (#32) |

- **Scope:** 1 commit (`git rev-list --count f2380e7..c618dea`) and 12
  files, +327/−34 (`git diff --shortstat f2380e7 c618dea`): the fixes for
  S-028, W-032 and W-033 and the eighth reviews' logs, of which 6 files,
  +55/−15, under `.github/` (the same command with `-- .github`) and none
  under `crates/`. Base equals the previous entry's Tip. Only the committed
  range is covered. The review of the release PR's own changes that
  `plans/todo/v0.0.2.md` criterion 10's amendment asks for.
- **Method:** the `code-review` skill over the range, which also ran
  `object.sh` over the 24 commits since `5393f67`, and `test.sh` and
  `mutate.sh`; a fresh agent then tried to refute each claim, with crafted
  messages, `git interpret-trailers --parse` on them, real commits run
  through both hygiene steps, and mutants applied to a copy of
  `.github/checks/`, under macOS awk only. Of the code review's 10 claims:
  - 7 are tracked: W-035 from three (the CR fold gap, its cause, and the
    pre-pass it needs), W-036 from three (the `LC_ALL=C` export, its
    prefixes and `mutate.sh`'s claim about them) and W-037.
  - 3 are not tracked: the fold rule outside git's trailer block, the
    git-made fold case not asking git, and `mutate.sh`'s cost. The
    worklist's "Not tracked" gives the reasons.
- **Findings:** 3 in the code worklist: W-035 (low), a carriage return
  hides a folded attribution line from `message.awk`; W-036 (cleanup),
  `LC_ALL=C` is set five times and its test runs only on macOS; W-037
  (low), the amended release gate lets the tagged commit change the
  checks the Release workflow runs on it.
- **Checked, no finding:** the 24 commits since `5393f67` pass both checks;
  `test.sh` ran 120 cases with none wrong and `mutate.sh` killed all 57
  mutants under macOS awk, and on `c618dea`'s CI under mawk; W-035 cannot
  pass the hygiene job, as `commit-object.awk` refuses a carriage return in
  any commit; nothing under `crates/`, so `code_id` is the same as on
  `c3d405d`.
- **Outcome:** no blocker, though W-037 changes what is tagged: `c618dea`
  is not. W-035 to W-037 are fixed in the next PR, and the commit that logs
  its reviews, which changes only logs, plans and the CHANGELOG, is the one
  `v0.0.2` tags. Worklist:
  [`plans/todo/code-review-w-series-4.md`](../../plans/todo/code-review-w-series-4.md).
- **Carried-over items:** W-028 to W-030 and W-034 open, waiting until
  after v0.0.2; W-032 and W-033 fixed in `c618dea` (#32).

### 2026-10-27: `118f710..f2380e7` (#28, #30 and #31)

| | SHA | Date | Subject |
| --- | --- | --- | --- |
| Base | `118f710bb1944b9c3817c10e8eae3b4274948bec` | 2026-10-25 | fix: review findings from #23, #25 and #26 outside code_id; log the sixth reviews (#27) |
| Tip | `f2380e7ff4ac4708d9dc8599915feb49397b4c98` | 2026-10-26 | ci: pin dtolnay/rust-toolchain to 683b588 (#31) |

- **Scope:** 5 commits (`git rev-list --count 118f710..f2380e7`) and 27
  files, +1077/−65 (`git diff --shortstat 118f710 f2380e7`): #28's ten
  campaign records and CHANGELOG date, #30's three commits (the seventh
  reviews' logs, `.github/checks/` with its tests, S-025's fix) and #31's
  action pin, of which 10 files, +567/−55, under `.github/` (the same
  command with `-- .github`) and none under `crates/`. Base equals the
  previous entry's Tip. Only the committed range is covered. The last part
  of step 7e; `f2380e7` is `main`'s tip.
- **Method:** the `code-review` skill at high effort over the range, which
  also ran `object.sh` over the 23 commits since `5393f67` and every tag,
  and `test.sh` and `mutate.sh`; a fresh agent then tried to refute each
  claim, applying mutants to a copy of `.github/checks/` under mawk, gawk
  and busybox. Of the code review's 9 claims:
  - 5 are tracked: W-032 (rules with no case), W-033 from three (the
    hashes that are not on `main`, "line for line", the stale plan rows and
    CHANGELOG date) and W-034.
  - 4 are not tracked: `mutate.sh`'s cost, its first run of `test.sh`,
    C1 controls in a message and commits with missing headers. The worklist's
    "Not tracked" gives the reasons; `mutate.sh`'s two share one.
- **Findings:** 3 in the code worklist, all cleanup: W-032, some rules of
  the object checks had no case; W-033, the worklists cite commits that are
  not on `main`, and the plan and CHANGELOG are stale; W-034, the identity
  and UTC steps still hold their own awk.
- **Checked, no finding:** the checks in `.github/checks/` are `118f710`'s
  programs, with S-025's line added and `message.awk`'s exit reversed
  where the hygiene step calls it; the 23 commits since `5393f67` and `v0.0.1` pass; on
  `f2380e7`'s CI, `test.sh` ran 321 cases under mawk, gawk and busybox
  with none wrong and `mutate.sh` killed all 47 mutants; #31 equals
  Dependabot's #24 and changes no job; nothing under `crates/`, so
  `code_id` is the same as on `c3d405d`.
- **Outcome:** no blocker. W-032 and W-033 are fixed in the review PR
  after #31, the checks applied by hand; W-034 waits until after v0.0.2.
  Worklist:
  [`plans/todo/code-review-w-series-4.md`](../../plans/todo/code-review-w-series-4.md).
- **Carried-over items:** W-028 to W-030 open, waiting until after v0.0.2;
  W-031 fixed in `9b8cefe` (#30), which its status had as `14989d0`
  (W-033).

### 2026-10-25: `c3d405d..118f710` (the review PR after #26)

| | SHA | Date | Subject |
| --- | --- | --- | --- |
| Base | `c3d405de4e4ff39c0132dd2f049f664396b22080` | 2026-10-25 | fix: ctutils 0.4.5 for RUSTSEC-2026-0385; MSRV 1.87 (#26) |
| Tip | `118f710bb1944b9c3817c10e8eae3b4274948bec` | 2026-10-25 | fix: review findings from #23, #25 and #26 outside code_id; log the sixth reviews |

- **Scope:** 1 commit (`git rev-list --count c3d405d..118f710`) and 13
  files, +423/−46 (`git diff --shortstat c3d405d 118f710`): CI, scripts and
  docs, none under `crates/`. Base equals the previous entry's Tip. Only
  the committed range is covered. Run while the second v0.0.2 campaign
  (step 7d″) runs on `c3d405d`; the rest of 7e's second part, `118f710`
  to 7d‴, follows once 7d‴ lands.
- **Method:** the `code-review` skill over the range; the new checks were
  run under mawk over the history and on crafted objects (details in the
  security log); a
  fresh agent then tried to refute each claim and verdict. Of the code
  review's 10 claims:
  - 4 are tracked: W-031 here, and three that are S-026 in the security
    log (an unsigned tag passes; an unsigned commit passes; `RELEASING.md`
    reads as if a signature were required).
  - 3 are rejected: the CHANGELOG's `--precise 0.4.5` is right, as 0.4.3
    to 0.4.5 all need Rust 1.87; the last entry's claim count adds up, as
    W-028 came from the security review; and `deny` on the root lock file
    only is the published graph's and predates the range in CI.
  - 3 are not tracked: no test for a workflow check, one `git cat-file`
    per commit, and the header step repeating the identity rule.

  The 3 rejected and the 3 not tracked are in the worklist's "Not
  tracked", with the reason.
- **Findings:** 1 in the code worklist: W-031 (cleanup), the tag check's
  awk program is written out twice, in `hygiene.yml` and `release.yml`.
- **Checked, no finding:** the new checks pass all 18 commits since
  `5393f67` and `v0.0.1`, and CI is green on `118f710`; every `python3`
  call has `-I`; `deny` in `verify` is pinned as in CI; nothing under
  `crates/`, so no `code_id` changes.
- **Outcome:** no blocker. W-031 is outside `code_id`, so it can land
  before the tag. Worklist:
  [`plans/todo/code-review-w-series-4.md`](../../plans/todo/code-review-w-series-4.md).
- **Carried-over items:** W-028 to W-030 open, waiting until after v0.0.2
  as the last entry said.

### 2026-10-25: `255353b..c3d405d` (#23, #25 and #26)

| | SHA | Date | Subject |
| --- | --- | --- | --- |
| Base | `255353b780c0a41de3d32259a2178d40d39691b5` | 2026-10-20 | fuzz: campaign lock held by the fuzzers; warn on mixed compilers (#22) |
| Tip | `c3d405de4e4ff39c0132dd2f049f664396b22080` | 2026-10-25 | fix: ctutils 0.4.5 for RUSTSEC-2026-0385; MSRV 1.87 (#26) |

- **Scope:** 3 commits (`git rev-list --count 255353b..c3d405d`) and 37
  files, +1096/−95 (`git diff --shortstat 255353b c3d405d`): the review-fix
  PR (#23), the first campaign's records (#25) and the `ctutils` fix (#26),
  of which 5 files, +303/−36, under `crates/` (the same command with
  `-- crates`). Base equals the previous entry's Tip. Only the committed
  range is covered. Run while the second v0.0.2 campaign (step 7d″) runs on
  `c3d405d`, so a fix under `crates/` waits unless it blocks; step 7e's
  second part reviews what follows.
- **Method:** the `code-review` skill, at high effort, over the range; two
  fresh agents read `crates/` and the rest for security (logged in the
  security log); a fresh agent then tried to refute each `crates/` claim,
  with mutations, fake codecs, probes and the xnu, smbfs and FUSE sources,
  and another each claim outside it. Of the code review's 8 claims, 2 were
  rejected (`finish` after a failed seal changes its meaning; an `EIO` from
  a full sync that is not offered) and 2 are not tracked, all with the
  reason, in the worklist's "Not tracked". One more, that the attribution
  check flags some prose, is in the security log's, and the bench scripts'
  Python is S-021 there.
- **Findings:** 3 in the code worklist, all low or cleanup:
  - W-028 (low): S-016's test checks the predicate, not `full_sync`; a
    mutant that brings the bug back passes clippy and every test (found by
    the security review).
  - W-029 (cleanup): a codec's result is checked only for its count, and
    only in heal; a wrong count or length is blamed on the store or the
    publisher. Unreachable with the only codec.
  - W-030 (cleanup): two copies of the same test helpers.
- **Checked, no finding:** #23's fixes do what part 3 of the worklist says,
  and each of its new tests fails when its fix is reverted (W-028 aside);
  `is_multiple_of` means the same as `%` for `SHARD_ALIGN` (64); each lock
  file changes only `ctutils`; no `unwrap`, `expect`, `panic`, unchecked
  indexing or `unsafe` outside tests; the tests, clippy and `cargo deny`
  pass.
- **Outcome:** no blocker, so nothing under `crates/` changes during the
  campaign. All three wait until after v0.0.2. Worklist:
  [`plans/todo/code-review-w-series-4.md`](../../plans/todo/code-review-w-series-4.md).
- **Carried-over items:** W-024 to W-027 fixed in #23, the review-fix PR the
  last entry named; part 3 of the worklist was closed there.

### 2026-10-21: `9bf18dc..255353b` (#20 to #22)

| | SHA | Date | Subject |
| --- | --- | --- | --- |
| Base | `9bf18dccf6e6603b10518b37852a1cda1a2b99bb` | 2026-10-17 | chore: bump to 0.0.2; keep medium findings private until fixed (S-006) (#19) |
| Tip | `255353b780c0a41de3d32259a2178d40d39691b5` | 2026-10-20 | fuzz: campaign lock held by the fuzzers; warn on mixed compilers (#22) |

- **Scope:** 3 commits (`git rev-list --count 9bf18dc..255353b`) and 18
  files, +909/−148 (`git diff --shortstat 9bf18dc 255353b`): the review-fix
  PR (#20), the hygiene and tag checks (#21) and the campaign lock (#22), of
  which 5 files, +379/−91, under `crates/` (the same command with
  `-- crates`). Base equals the previous entry's Tip. Only the committed
  range is covered. Run before the v0.0.2 campaign, so that a fix it calls
  for under `crates/` does not restart it; step 7e reviews only what follows.
- **Method:** the `code-review` skill, at high effort, over the range; two
  fresh agents read `crates/` and the rest for security (logged in the
  security log); a fresh agent then tried to refute each `crates/` claim,
  with probes and by reverting fixes, and another each claim outside it. Of
  the code review's 10 claims, one was rejected: `actions/checkout` keeps
  an annotated tag annotated on a tag push (its source at the pinned commit,
  and `v0.0.1`'s own runs, show it fetches the tag objects). Its Apple sync
  claim is S-016 in the security log.
- **Findings:** 4 in the code worklist, all low or cleanup:
  - W-024 (low): three of #20's W-017 fixes had no test that fails without
    them, against #20's own claim.
  - W-025 (low): a `DirStore` sealed into again after a failed seal kept
    directories to sync from that run.
  - W-026 (cleanup): `write_manifest_bytes` dropped a list of directories
    to sync; empty today.
  - W-027 (cleanup): a short codec result counted as a publisher's
    mismatch.
- **Checked, no finding:** #20's fixes do what part 2 of the worklist says;
  its new tests fail on `9bf18dc`; no `unwrap`, `expect`, `panic`, unchecked
  indexing or `unsafe` outside tests; heal writes only bytes that match the
  manifest; clippy and the tests pass. Four claims were left as they are,
  with the reason, in the worklist's "Not tracked".
- **Outcome:** no blocker. All four are fixed in the review-fix PR that
  follows, before the v0.0.2 campaign, with S-016 to S-020. Worklist:
  [`plans/completed/code-review-w-series-3.md`](../../plans/completed/code-review-w-series-3.md),
  closed at once.
- **Carried-over items:** W-015 to W-019, W-022 and W-023 fixed in #20,
  W-020 and W-021 in #22; part 2 of the worklist moved to
  `plans/completed/code-review-w-series-2.md` in #22.

### 2026-10-20: `5393f67..9bf18dc` (#11 to #19, the v0.0.2 candidate)

| | SHA | Date | Subject |
| --- | --- | --- | --- |
| Base | `5393f6704b5059c4c8ecfbb6adc01718c13c4455` | 2026-10-16 | docs: release v0.0.1 on 2026-10-16 (#10) |
| Tip | `9bf18dccf6e6603b10518b37852a1cda1a2b99bb` | 2026-10-17 | chore: bump to 0.0.2; keep medium findings private until fixed (S-006) (#19) |

- **Scope:** 11 commits (`git rev-list --count 5393f67..9bf18dc`) and 56 files,
  +3785/−306 (`git diff --shortstat 5393f67 9bf18dc`): #11 to #19, everything
  in v0.0.2 so far, of which 14 files, +1139/−201, under `crates/` (the same
  command with `-- crates`). Base equals the previous entry's Tip. Only the
  committed range is covered.
- **Method:** the `code-review` skill was not available in this session, so
  the range was reviewed directly, as the security path does: four fresh
  agents, one per area (store, stream and CLI; manifest, tree, header and the
  heap harness; a security pass over `crates/`; CI, scripts and docs), then
  two more fresh agents tried to refute each of their 18 claims. One was
  rejected (Dependabot's pull requests fail the hygiene job by design, as
  `hygiene.yml` says). Of the rest, 15 got IDs; three cleanups were fixed
  without one (S-005's entry named a renamed test; a completed plan named a
  moved worklist; S-008's entry got a note), and the released 0.0.1 entry's
  "about 640 GiB" (648 is right) is left as released.
- **Findings:** 9 in the code worklist, all low or cleanup; the security ones
  are S-010 to S-015 in the security log.
  - W-015 (low): a seal into a new store under a parent that can be written
    to but not read, such as a drop box, failed (a regression from #13).
  - W-016 (low): a new seal paid two full flushes per segment on macOS, not
    the one #18 claims.
  - W-017 (cleanup): heal skipped `finish` on some failures, and its doc
    did not match.
  - W-018 (cleanup): a failed directory sync was reported as "cannot write".
  - W-019 (cleanup): `tree::segment_root` overflowed its 16-bit index
    beyond 65,536 hashes (predates v0.0.1; unreachable through a header).
  - W-020 and W-021: `campaign.sh` can run two campaigns after a stale lock,
    and `code_id` ignores the compiler.
  - W-022 and W-023: CHANGELOG timings without a record, and the README's
    memory claim.
- **Checked, no finding:** the new tree builder gives the old root for every
  leaf count tried (1 to 600, and 17 larger ones up to 300,001); every flipped
  byte, truncation and extension of a manifest is rejected, the same by
  `decode` and `decode_owned`; W-003 to W-011's fixes do what their worklist
  says, and their tests assert it; no `unwrap`, `expect`, `panic`, unchecked
  indexing or `unsafe` outside tests; the golden vectors are byte-identical;
  decision 0009's numbers match its records.
- **Outcome:** no blocker. W-015 to W-019 and W-022, W-023 are fixed in the
  review-fix PR that follows, with S-010 and S-011, before the v0.0.2
  campaign; step 7e reviews that PR. W-020 and W-021 stay open. Worklist:
  [`plans/todo/code-review-w-series-2.md`](../../plans/todo/code-review-w-series-2.md).
- **Carried-over items:** W-003 to W-011 fixed in #13 and #14, W-013 and
  W-014 in #12; the part 1 worklist moved to
  `plans/completed/code-review-w-series.md` in #14.

### 2026-10-16: `a9445d9..5393f67` (#8, #9, #10)

| | SHA | Date | Subject |
| --- | --- | --- | --- |
| Base | `a9445d9a32dcafe4a8bd018e4280e03411ce6a12` | 2026-10-15 | docs: freeze the v1 format; changelog; first full code and security reviews (#7) |
| Tip | `5393f6704b5059c4c8ecfbb6adc01718c13c4455` | 2026-10-16 | docs: release v0.0.1 on 2026-10-16 (#10) |

- **Scope:** 3 commits (`git rev-list --count a9445d9..5393f67`) and 24 files,
  +783/−20 (`git diff --shortstat a9445d9 5393f67`): #8 (review logs and
  worklists), #9 (15 fuzz records) and #10 (CHANGELOG date, decision 0004,
  `bench/heal-check.sh`, plan). Nothing under `crates/`, the Cargo files, the
  fuzz targets or `.github/`. Base equals the previous entry's Tip. Only the
  committed range is covered.
- **Method:** the `code-review` skill over the range. A fresh agent then
  checked each of its 10 findings at `5393f67`: 9 confirmed, 1 rejected (the
  heal-check unsupported-OS exit is not a regression; it matches
  `segment-size.sh`).
- **Findings:** 9 confirmed: W-013 and W-014 are new; the rest are logged as
  S-007, S-008 and S-009, or fixed in the closing PR. None is in shipped code,
  so v0.0.1 is unaffected.
  - W-013 (cleanup): `bench/heal-check.sh` assumes the default 20-of-50 and
    repeats `segment-size.sh`'s steps; its time-format parse fails loudly.
  - W-014 (low): `fuzz/campaign.sh summary` counts overlapping runs twice
    (27.2 h shown for about 25.9 h); the gate is unaffected.
  - The CHANGELOG omissions (W-003, W-004, W-005, and `heal` in W-007) are
    logged as S-009 and fixed in the closing PR.
  - GitHub's merge metadata on the range's commits is S-007 and S-008.
  - W-012 has no regression test because `bench/` has no harness; its
    status now says so.
  - The Phase 1 plan's merge-method and tag records are corrected in the
    closing PR.
- **History note:** while the review ran, the last two commits of the range
  were re-made on `main` with the project's identity and the same files
  (S-007, S-008). The review read the earlier commits; their trees equal
  `36cb370` and `5393f67`, so the Tip and the baseline name `5393f67`.
- **Outcome:** no blocker. Worklist:
  [`plans/todo/code-review-w-series.md`](../../plans/todo/code-review-w-series.md).
- **Carried-over items:** W-003 to W-011 still open (no `crates/` change in
  the range); W-012 fixed in #10.

### 2026-10-15: `e24f453..a9445d9` (#5, #6, #7)

| | SHA | Date | Subject |
| --- | --- | --- | --- |
| Base | `e24f4538e88320993d7cd5497239f3bf8a776786` | 2026-10-14 | feat: streaming seal, open and heal; segment-size benchmark (#4) |
| Tip | `a9445d9a32dcafe4a8bd018e4280e03411ce6a12` | 2026-10-15 | docs: freeze the v1 format; changelog; first full code and security reviews (#7) |

- **Scope:** 3 commits (`git rev-list --count e24f453..a9445d9`) and 25 files,
  +942/−80 (`git diff --shortstat e24f453 a9445d9`). The code under review was
  #6's change to `crates/` (6 files, +335/−40), `bench/heal-check.sh` (#5) and
  the `vectors` CI job (#7); the rest is docs. Base equals the previous entry's
  Tip. Only the committed range is covered.
- **Method:** the `code-review` skill over the range, from eight angles, with
  duplicates removed. Each of its 10 findings was then checked by one of three
  fresh agents in separate worktrees: reproduced with release builds of both
  commits, cut by mutation, or (W-006, W-007, W-010, W-011) confirmed by
  reading the code. All 10 were confirmed.
- **Findings:** 10, all low or cleanup; none blocks v0.0.1. Details and
  remediation in the worklist.
  - W-003 (low): one failed shard write ends the whole heal
    (`crates/neoblast/src/stream.rs:237`); pre-existing.
  - W-004 (low): planted `<path>.<pid>.tmp` names make heal's writes fail
    (`crates/neoblast/src/store.rs:178`); from #6.
  - W-005 (low): `seal` reads a whole pipe before refusing a non-empty store
    (`crates/neoblast/src/main.rs:91`); from #6.
  - W-006 (low): the library checks the manifest cap only after writing every
    shard (`crates/neoblast/src/stream.rs:52`); from #6.
  - W-007 (low): store writes are not synced before rename
    (`crates/neoblast/src/store.rs:182`); pre-existing. Listed as a known issue
    in `CHANGELOG.md`.
  - W-008 (low): the symlinked-segment-directory regression test never reaches
    `write_shard`'s guard (`crates/neoblast/tests/review_regressions.rs:247`).
  - W-009 (low): `--k 0` and too many segments give unhelpful errors
    (`crates/neoblast-format/src/header.rs:110`).
  - W-010 (cleanup): `write_manifest` encodes before checking the cap
    (`crates/neoblast/src/store.rs:129-152`).
  - W-011 (cleanup): a duplicated test helper
    (`crates/neoblast/tests/review_regressions.rs:171` and `cli.rs:16`).
  - W-012 (low): `bench/heal-check.sh` fails silently (`bench/heal-check.sh:34,42`).
- **Checked and clean:** #6's S-001 fix reads only regular files and never
  writes through a planted symlink; the W-001 cap is checked before a manifest
  is read and before `seal` touches the store; `--force` keeps the old manifest
  when the new one would not fit; no `unwrap`, `expect`, `panic` or unchecked
  indexing outside tests; both v1 vector files are unchanged and pinned by the
  new `vectors` CI job.
- **Outcome:** no release blocker. W-003 to W-011 need `crates/` changes, and
  `fuzz/campaign.sh` takes `code_id` from the whole `crates/` tree, tests
  included, so they wait for v0.0.2. W-012 can land during the campaign.
  Worklist: [`plans/todo/code-review-w-series.md`](../../plans/todo/code-review-w-series.md).
- **Carried-over items:** W-001 fixed in `9a542da` (#6); W-002 fixed in
  `9a542da` (#6), with its follow-up message in W-009.

### 2026-10-14: first review, full history (empty tree to `e24f453`)

| | SHA | Date | Subject |
| --- | --- | --- | --- |
| Base | `4b825dc642cb6eb9a060e54bf8d69288fbee4904` (empty tree) | n/a | First review: started from the empty tree so the root commit `57d6ed4` is included (protocol step 3) |
| Tip | `e24f4538e88320993d7cd5497239f3bf8a776786` | 2026-10-14 | feat: streaming seal, open and heal; segment-size benchmark (#4) |

- **Scope:** 5 commits (`git rev-list --count e24f453`) and 107 files, +10454/−0
  (`git diff --shortstat <empty tree> e24f453`). The Rust sources reviewed were
  the four crates and `fuzz/` (45 files, +6084/−0), checked against `spec/header.md`,
  `spec/caps.md` and the CLAUDE.md conventions. Only the committed range is covered.
  Uncommitted doc edits in the working tree (spec wording, CHANGELOG, plans) were
  not reviewed.
- **Method:** a direct review of the range, crate by crate. The reviewer ran as a
  delegated agent that could not start further subagents. So the fan-out and the
  fresh-subagent verification in protocol step 4 were not used. Both findings were
  checked by re-reading every code path involved. Afterwards a fresh agent
  verified both against `e24f453`, reproducing each with the release CLI, and
  confirmed them as low; details in the worklist.
- **Findings:** 2 confirmed, both low, neither blocks v0.0.1. Both need a
  `crates/` change, so they wait for v0.0.2. Doing them before the tag would
  reset the fuzz hours. (Later the same day: once S-001 forced a new campaign,
  both were fixed before the release in #6; see the worklist.)
  - W-001 (low): `DirStore::read_manifest` (`crates/neoblast/src/store.rs:50`)
    reads a manifest of any size that matches its own header before the root
    check. A planted sparse file can make `open` and `heal` allocate up to
    about 16 TiB.
  - W-002 (low): `Header::new` (`crates/neoblast-format/src/header.rs:104`)
    reports a segment-count overflow as an out-of-range segment size.
- **Checked and clean:**
  - Header decoding validates every field and relation, and `Header` is only
    built through `new` or `decode`.
  - k, n and lengths come only from a root-checked manifest header.
  - No `unwrap`, `expect`, `panic` or unchecked indexing outside tests.
    `unsafe` is forbidden workspace-wide.
  - `FileReadCap` and `SegmentCipher` redact secrets in `Debug`, and the key
    appears only after `#`.
  - AEAD nonces and AAD match the spec.
  - Hash-tree domain separation holds, and odd nodes are promoted without
    self-pairing.
  - Heal ignores bad shards and never deletes any.
  - The fuzz campaign refuses a dirty tree and pins `code_id` at launch.
- **Outcome:** no release blocker. Worklist:
  [`plans/todo/code-review-w-series.md`](../../plans/todo/code-review-w-series.md).
- **Carried-over items:** none (first entry).
