# Code Review Log

<!-- baseline --> **Next review diffs from:** `57d6ed43503ab1e0421fa43c20c23cc699bfe0ed` (`main`, recorded 2026-10-14)

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

_No entries yet._
