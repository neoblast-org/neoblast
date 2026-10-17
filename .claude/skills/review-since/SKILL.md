---
name: review-since
description: Run an incremental code review (or security review with the "security" arg) covering only the commits since the last review recorded in docs/logs/, then append the new entry to the log. Use when asked to "review since the last review", "review what's new", or run an incremental /code-review or /security-review from the logged baseline.
---

# Review Since

Reviews only what changed since the last logged review, instead of re-reviewing the
whole branch, and keeps `docs/logs/` as the single source of truth for what
has been reviewed up to which commit.

Two logs, one mechanism:

| Arg | Log | Review to run |
|-----|-----|---------------|
| *(none)* | `docs/logs/code-review-log.md` | the `code-review` skill |
| `security` | `docs/logs/security-review-log.md` | diff-based review (see step 4) |

## The baseline contract

Each log's header carries exactly ONE line beginning with the marker `<!-- baseline -->`:

    <!-- baseline --> **Next review diffs from:** `<full SHA>` (`<branch>`, recorded <date>)

That line is the machine-owned frontier — the only place "reviewed up to which commit"
lives. Its branch field names the branch the log tracks; its date is when the line was
last moved. Every field is rewritten together at step 6. Entries record history and
never carry the frontier; log prose must never start a line with the marker, so the
anchored grep in step 1 finds exactly one line.

A review logged by hand follows the same steps 5–6: prepend the entry AND move the
line — an entry without moving the line is invisible to the next incremental run.
Side-branch reviews are the exception: see step 5.

## Protocol

0. **Parse args.** An arg of 7–40 hex characters is intended as a base-override SHA:
   - `git cat-file -t <arg>^{commit}` must report `commit` — if it fails (typo,
     unfetched object, ambiguous prefix), stop and tell the user; never silently
     reclassify a hex-shaped arg as pass-through.
   - If it is all digits and six or fewer characters, it may be a PR number — stop and
     ask which was meant before proceeding.
   Anything not hex-shaped passes through to the underlying review skill (e.g. an
   effort level: `/review-since high`).

1. **Find the baseline.** `grep '^<!-- baseline -->' <log>` — anchored. Exactly one
   line must match; if the log, the line, or its SHA is missing — or more than one
   line matches — stop and ask the user. Never guess a baseline.

2. **Validate.**
   - `git cat-file -e <SHA>^{commit}` — if the logged SHA no longer exists in this
     clone (history rewrite, gc, shallow clone), stop and ask; never guess a
     replacement.
   - `git branch --show-current` must equal the baseline line's branch field — if
     not, stop and ask: running from another branch would record its tip as this
     log's frontier.
   - `git merge-base --is-ancestor <SHA> HEAD` — if the SHA exists but is not an
     ancestor (rebase), use `git merge-base <SHA> HEAD` as the base and say so in
     both the report and the log entry.
   - **Apply any override from step 0 now:** the override becomes `<base>` in every
     later step, and it must be the logged SHA or an ancestor of it (check with
     `git merge-base --is-ancestor <override> <logged SHA>`). An override at or
     behind the frontier only widens the range, so advancing to TIP stays sound; an
     override ahead of the frontier would mark the skipped commits reviewed when they
     were not — stop and ask instead. Note the override in the entry.

3. **Pin the tip, then check the range.** Resolve `TIP=$(git rev-parse HEAD)` ONCE,
   now. Every later step — rev-list, diff, review target, entry, new baseline — uses
   this TIP verbatim, never a re-resolved HEAD: commits landing mid-review must not
   be absorbed unreviewed into the recorded frontier.
   - **First review from the root commit.** When `<base>` is the branch's root commit
     (`git rev-list --max-parents=0 TIP` lists it) and the log has no entries yet, the
     first review covers the full history, and `<base>..TIP` would leave out the root
     commit's own changes. Use the empty tree as the base instead:
     `EMPTY=$(git hash-object -t tree /dev/null)`, then review `git diff $EMPTY TIP`
     (it contains everything in `<base>..TIP` plus the root commit, so don't review
     both). The range is never empty in this case, even when TIP is the root commit.
     Count scope with `git diff --shortstat $EMPTY TIP` and `git rev-list --count
     TIP`, and say in the entry that the review started from the empty tree.
   - `git rev-list <base>..TIP` empty and `git status --porcelain` clean → report
     "nothing new since <base> (<its date>)" and stop. No entry for an empty range.
   - Range empty but uncommitted changes exist → say so and offer to review the
     pending changes instead. Such a review is report-only: NO entry and NO baseline
     move (only committed ranges are logged), and say that in the report.
   - Range non-empty with uncommitted changes also present → state in the report and
     the entry that the review covers only the committed range.

4. **Run the review over `<base>..TIP`.**
   - Code path: invoke the `code-review` skill with the range as its target (it
     accepts one). Confirm the review's stated scope matches `<base>..TIP` before
     logging it. A first review from the empty tree (step 3) can't be given to the
     skill as a range, so review that diff directly, as the security path does.
   - Security path: the stock `security-review` skill takes no range (it reviews the
     branch's pending changes), so always review the range diff directly.
   - Diff-based reviews are never run from one monolithic dump: `git diff --stat
     <base>..TIP` first; beyond a few thousand changed lines, fan the diff out per
     file or directory to subagents and consolidate. A full-history first review is
     almost always that size.
   - Findings from any review you run yourself must be adversarially verified by a
     fresh subagent before they enter the log, whatever the range size.
   - Re-check open items: every worklist doc linked from this log's entries that is
     not yet closed (still under `plans/todo/` or `plans/blocked/` — a linked path
     that no longer exists has moved folders, so look it up by filename), plus any
     items the newest entry marks "open". Carry each item's status (retired or still
     open) into the new entry, citing it by its worklist-unique ID.

5. **Append the entry** at the TOP of the log's entries (newest first), matching the
   existing entry format: review date, a range table (Base and Tip rows), SHAs with
   commit dates and subjects, commit/file counts with the counting method stated (e.g.
   "+N/−M"), the findings summary and outcome, the status of carried-over open
   items, and a pointer to any worklist/plan doc the findings produced. Never start
   a line inside an entry with the baseline marker. On the first entry, delete the
   log's `_No entries yet._` placeholder.
   **Side-branch reviews** (a pre-PR review of a branch other than the one the log
   tracks) are logged as an entry titled with the branch name and marked "frontier
   unchanged (side branch)" — step 6 is skipped for them, and the continuity check
   applies only between frontier-moving entries.

6. **Advance the baseline.** Rewrite the log's single marker line in full — new TIP
   SHA, the branch, and today's date. This is the only permitted edit to existing
   log text; entries are never rewritten (corrections to an older entry's claims are
   recorded in the new entry, not edited in place). Continuity check before writing:
   the new entry's Base must equal the line's old SHA, or the entry must explain why
   not (an override, or a merge-base fallback).

7. **Hand back, don't commit.** Report the findings to the user and leave the log
   edit (and any worklist doc) uncommitted — the user commits.

## Notes

- The baseline advances only when a review actually ran over a committed range ending
  at the recorded SHA; never append a frontier-moving entry for a review that was not
  performed.
- Findings worth tracking beyond the log entry go to a worklist doc under
  `plans/todo/`, linked from the entry — that is where open re-check items live, so
  step 4 can find them. An open worklist that is waiting on something outside the
  code lives in `plans/blocked/` and is still re-checked. A worklist is closed when
  every item is fixed or accepted with its reasoning recorded; it then moves to
  `plans/completed/`, and the move is noted in the next entry, never by editing the
  old entry's link.
- **Findings of medium severity or higher stay private until fixed**
  (`SECURITY.md`, "Our own findings"; S-006). Once committed, the log and the
  worklists are public, so for such a finding the entry names only its ID, its
  severity and whether it blocks the release, never where it is or how to
  trigger it. Hand its details to the user for a private draft advisory
  instead; its worklist entry follows once the fix is on `main`. To re-check
  it in step 4, ask the user for the advisory's details. Its ID counts as used
  for the ID rule below, though so far it appears only in a log. Low findings
  may be public before they are fixed.
- Worklist format: once a worklist of the same kind exists under `plans/`, follow
  the closest one. Until then: security worklists group findings by severity, each
  with file:line and a remediation-status line naming the fixing commit; code
  worklists put success criteria first. Either way, every finding gets an ID that is
  unique across all worklists — W-series for code, S-series for security,
  continuing from the highest ID already used (e.g. `S-001`, severity stated beside
  it) — and a status (open / fixed / won't fix) with the commit that resolved it, so
  step 4 can carry it unambiguously.
