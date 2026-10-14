# 0005: Why strangers store your shards

- Status: proposed (Phase 2 uses donors and pinners only; swaps arrive in Phase 3)
- Date: 2026-10-13

## Context and problem statement

Self-healing assumes idle nodes with free disk. Neoblast is free: no token, no payment, no global ledger.

## Considered options

1. Payment or tokens (Filecoin, Storj, Sia). Rejected: Neoblast is free.
2. A global ledger. Rejected: heavy, and a single point of attack.
3. Pure altruism (IPFS pinning). Rejected alone: unpinned content vanishes.
4. Pairwise storage swaps with unforgeable claims (Samsara, SOSP 2003), plus pinners and donors. Chosen.

## Decision outcome

- **Swaps:** when B stores your shard, you hold an equal-sized claim for B, a pseudorandom block from B's keyed BLAKE3 stream that B can regenerate to check you.
- **Challenges:** both sides ask for random chunks every few hours. Failures are punished gradually (Samsara's schedule), so sleepers lose little and cheaters lose everything within days.
- **Leases:** every shard lives 30 days unless renewed by the owner, a pinner or a donor.
- **Newcomers** get about 1 GB free; beyond that they upload only what they have proven they store.
- **Default reserve:** 2× what you upload.

### Consequences

- Good: no money, no chain; content survives as long as someone cares.
- Bad: lease length, challenge interval and punishment schedule are guesses until the Phase 3 simulations (ROADMAP open items 5 and 8).
