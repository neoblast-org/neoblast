# 0003: Threat model

- Status: accepted
- Date: 2026-10-13

## Context and problem statement

"Uncensorable" means nothing until we say against whom. The choice drives transport, placement, bootstrap and what we promise users.

## Decision outcome

v1 defends against four adversaries: a **hosting provider**, an **ISP**, a **national firewall** and a **court order**. It keeps content available and unreadable. It does **not** provide publisher or reader anonymity. Details and the scenario tests live in `spec/threat-model.md`.

### Consequences

- Good: a scope one person can deliver and test.
- Bad: users who need anonymity must layer Tor themselves; the README and CLI must say so plainly.
- Design rule: no kill switch. No admin key, central index or server may exist that can delete content.
