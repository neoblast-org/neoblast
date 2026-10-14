# Channel records (draft, Phase 2)

A channel record points a channel's public key at its latest target (a file
read cap or a collection manifest). It is published as a BEP 44 mutable item on
the BitTorrent Mainline DHT under the Ed25519 public key, and mirrored in
Neoblast's own DHT. It MUST fit BEP 44's 1,000-byte value limit.

```text
record = version(1) || flags(1) || seq(u64_be) || expires_at(u64_be unix secs)
       || nonce(24) || XChaCha20-Poly1305(Rc-derived key, nonce, target, aad = pk || seq)
signature = Ed25519(seed, "nbl/v1/channel-record" || record)
```

- Highest `seq` wins. Readers SHOULD query several nodes and take the highest valid one.
- Anyone MAY re-publish a signed record without the key; pinners keep channels alive this way.
- **Tombstone (open item 4):** `seq = u64::MAX`, flag bit 0 set, optional successor read link as the target.

Open questions: exact derivation of the record encryption key; expiry and re-publish interval.
