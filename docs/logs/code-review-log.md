# Code Review Log

<!-- baseline --> **Next review diffs from:** `e24f4538e88320993d7cd5497239f3bf8a776786` (`main`, recorded 2026-10-14)

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
