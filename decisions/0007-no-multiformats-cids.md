# 0007: Links keep their own version byte, not multiformats CIDs

- Status: accepted
- Date: 2026-10-14
- Settles: ROADMAP open item 3

## Context and problem statement

Links carry a version byte (`0x01`) followed by the file root `H`
(`spec/caps.md`). IPFS identifies content with CIDs instead: self-describing
multiformats strings made of a multibase prefix, a CID version, a content-type
code and a multihash. Adopting CIDs would change the bytes of every link, so the
question has to be settled before the Phase 1 format is frozen.

Decision rule, written before the check: **adopt CIDs only if a planned feature
needs IPFS tooling; otherwise keep the version byte.**

## Method

A desk check of every planned feature in `ROADMAP.md` and the original design
notes, asking for each whether it needs IPFS software (gateways, Bitswap, the
IPFS DHT, pinning services) to read our identifiers. The check also looked at
what a CID for our data would actually claim.

## Findings

1. **Shard discovery (Phase 2).** The design notes describe Kademlia
   `PROVIDE(shard id)` records. libp2p's Kademlia keys records by arbitrary bytes,
   and iroh addresses blobs by raw BLAKE3 hashes. Neither needs a CID, and open
   item 6 chooses between them.
2. **Channels (Phase 2).** These are BEP 44 records on the BitTorrent Mainline
   DHT, keyed by Ed25519 public keys. That is not IPFS.
3. **Gateways.** An IPFS gateway can only serve content it can fetch over IPFS
   protocols. Our shards travel on Neoblast's own protocol IDs, so a CID would
   not make a gateway work. A Neoblast gateway reads `nbl://` links directly.
4. **Pinning and donors (Phase 2).** `neoblast pin` and donor nodes store shards
   for others over our protocol. IPFS pinning services fetch over Bitswap, which
   our nodes do not speak, so CIDs would not let them help.
5. **Blind search, leases, swaps and the per-node denylist (Phases 3–4)** use our
   own records. IPFS "bad bits" denylists are keyed by CID, but our denylist is
   each operator's own list of file roots and shard hashes, matched by our code.
   It needs no IPFS format.
6. **Bindings, browser and testnet (Phase 5).** WASM, Python and the other SDKs
   reproduce our golden vectors and speak our protocol. A public testnet runs
   our daemon. None of them talks to IPFS.
7. **What a CID would claim.** The file root `H` is
   `BLAKE3.derive_key("nbl/v1/root", header ‖ content_root)`, the output of
   BLAKE3's key-derivation mode and not the hash of any stored block. A CID for
   it would say "the BLAKE3 of this block is H", which no IPFS tool could ever
   verify. Of the things we store, only shards are plain BLAKE3 hashes of a block.
8. **Size.** In base32 a `nbl://fv/` payload is 53 characters. A CIDv1
   (`b` + `0x01` + codec + `0x1e 0x20` + 32 bytes) is 59 or more, with no gain.

No planned feature needs IPFS tooling.

## Decision outcome

**Keep the version byte.** Links stay as specified in `spec/caps.md`. Neoblast
does not use multiformats in v1.

If an IPFS bridge is ever wanted, shards can be given CIDs without any format
change: a shard's CID is `CIDv1(raw = 0x55, blake3 = 0x1e, shard_hash)`,
computable from the verify manifest. A future link format would use a new
version byte (for example `0x02`) rather than reinterpret `0x01`.

### Consequences

- Good: no change to Phase 1 bytes. The links and golden vectors stand.
- Good: short links, and no dependency on the multiformats crates.
- Bad: IPFS tools will not recognise `nbl://` links. That was never going to
  work for encrypted shards on our own protocol anyway (findings 3, 4 and 7).
- Neutral: the shard-CID mapping above stays available if a bridge is wanted.
