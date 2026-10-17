# Security policy

Neoblast is experimental and has **not** been audited. Treat it as a research
prototype.

## Reporting a vulnerability

Please report privately. Do not open a public issue.

- Preferred: GitHub's **Report a vulnerability** button on the repository's
  Security tab (private advisory).
- Email: **security@neoblast.dev**

Include what you found, how to reproduce it, and its impact as you see it. You
will get an acknowledgement within 7 days. We aim to fix confirmed issues and
publish an advisory within 90 days, and we credit reporters unless you ask us not to.

## Our own findings

Findings from the maintainers' own reviews and fuzzing are kept private until
fixed too, if they are of medium severity or higher:

- Their details go into a private draft advisory on GitHub. The public review
  log names only the finding's ID, its severity and whether it blocks the
  release.
- The fix's pull request and commit say what changed, not how to exploit it.
  The fix and its test are public while that pull request is open, so it is
  merged promptly. The worklist entry with the details follows once the fix is
  on `main`.
- A release that ships with such a finding unfixed publishes its advisory with
  the release.

Findings of low severity may be public before they are fixed.

## Scope

In scope: the byte formats, cryptography, erasure coding and healing, and the
`neoblast` CLI. The threat model in `spec/threat-model.md` says what v1 does and
does not try to protect. Lack of publisher or reader anonymity is a documented
limitation, not a vulnerability.
