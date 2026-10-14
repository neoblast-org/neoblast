# 0004: Capability model

- Status: proposed (files implemented in Phase 1; channels in Phase 2)
- Date: 2026-10-13

## Context and problem statement

Only the master-key holder could decrypt or search, so nothing could be published. Publishing needs shareable links, and storage nodes need just enough to verify and heal.

## Considered options

1. Tahoe-LAFS-style nested capabilities (verify ⊂ read ⊂ write). Chosen.
2. One key per user, shared out of band. Rejected: sharing one file would expose all of them.
3. Public-key encryption per reader. Deferred: needs reader identities, which conflicts with publishing to anyone.

## Decision outcome

- **Files** get a fresh random 32-byte read key `R`. Read link `nbl://f/<root>#<R>`; verify link `nbl://fv/<root>`.
- **Channels** are Ed25519 keypairs whose signed records (BEP 44 mutable items on the Mainline DHT) point to the latest file or collection. Read link `nbl://c/<pk>#<Rc>`, verify `nbl://cv/<pk>`, write seed never shared.
- A 24-word BIP39 phrase derives the master secret, channel seeds and a private keyring channel that records every `R`.

Borrowed from: Tahoe-LAFS (ideas only, GPL/TGPPL), BEP 44, pkarr (MIT).

### Consequences

- Good: publishing is sharing a link; storage nodes never see keys; gateways never see keys.
- Bad: a shared read link can never be revoked. Losing a channel's write seed freezes it forever.
