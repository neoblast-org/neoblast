# Security Review Log

<!-- baseline --> **Next review diffs from:** `57d6ed43503ab1e0421fa43c20c23cc699bfe0ed` (`main`, recorded 2026-10-14)

A running record of branch security reviews. Each entry records the exact commit range
reviewed; the baseline marker line above is the machine-owned frontier the next
incremental review starts from. Newest entry first. Code reviews are logged separately
in `code-review-log.md`.

**Starting point:** no security review has been logged here yet. The baseline is
`main`'s first commit, so the first `/security-review-since` covers the full history,
including that first commit (the review-since skill's step 3 explains how). Review it
against `spec/threat-model.md` and `spec/scenarios.md`.

**How to run the next review:** `/security-review-since` (or `/review-since
security`). The skill files (`.claude/skills/review-since/SKILL.md` and its alias)
are the sole definition of the mechanics — this header does not restate them. A
hand-run review must follow the same protocol steps, pinning the tip before reviewing
and doing both halves of the bookkeeping: prepend the entry AND move the baseline
marker line; a frontier-moving entry's Base must equal the previous one's Tip unless
the entry explains why not.

## Entries

_No entries yet._
