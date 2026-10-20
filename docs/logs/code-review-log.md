# Code Review Log

<!-- baseline --> **Next review diffs from:** `9bf18dccf6e6603b10518b37852a1cda1a2b99bb` (`main`, recorded 2026-10-20)

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
