---
name: security-review-since
description: Run an incremental security review covering only the commits since the last security review recorded in docs/logs/security-review-log.md, then append the new entry to the log. Use when asked to "security-review since the last review" or run /security-review incrementally from the logged baseline.
---

# Security Review Since

Alias for the `review-since` skill with the `security` argument. Follow the protocol
in `.claude/skills/review-since/SKILL.md` exactly, with:

- **Log:** `docs/logs/security-review-log.md`
- **Review to run:** the security path of that protocol's step 4 (a diff-based
  security review of `<base>..TIP` — the stock `security-review` skill takes no
  range, so the range diff is reviewed directly)
- Args behave as that protocol's step 0 describes.

Do not duplicate or restate the protocol here — if the two files ever disagree,
`review-since/SKILL.md` is the source of truth.
