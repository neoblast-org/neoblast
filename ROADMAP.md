# Roadmap

Neoblast is built in phases. Each phase ends at a **gate**: a test that must pass
before the next phase starts. Estimates assume focused solo work; part-time
stretches them.

| Phase | What | Estimate | Gate |
| --- | --- | --- | --- |
| 0 · Spec and foundations | Threat model, reconciled spec, capability format, repo, CI | 2–3 weeks | Every format question resolved in `spec/` |
| **1 · Crypto core** (current) | Committed header, segmented AEAD, erasure coding, offline `seal` / `open` / `heal` | 4–6 weeks | Golden vectors frozen; drop-any-(n−k) property tests pass; 24 h fuzzing per target with no crash, recorded in `fuzz/campaigns/` |
| 2 · Publish and fetch (MVP) | P2P swarm on its own protocol IDs, NAT traversal, placement across providers, channel records on the Mainline DHT, donors and `neoblast pin` | 6–8 weeks | 20 nodes on 3+ networks; a file still fetches with n − k of its holders offline |
| 3 · Healing and fair storage | Storage swaps with claims, chunk challenges, lazy repair, leases, per-node denylist | 6–8 weeks | A week-long churn test keeps every file above k + margin; repair bandwidth measured |
| 4 · Blind search and collections | Unicode `NFKC_Casefold`, keyed-BLAKE3 trapdoors, salted manifests, collection caps | 4–6 weeks | Multi-script normalisation suite passes; written leakage analysis |
| 5 · Reach | Browser WASM and Python first; Go, Swift and Kotlin on demand; public testnet; outside crypto review | Ongoing | All bindings pass the golden vectors |

## Open items

Each is settled by the method in [`decisions/README.md`](decisions/README.md),
with the decision rule fixed *before* the result is known.

| # | Item | How we decide | Decision rule | When |
| --- | --- | --- | --- | --- |
| 1 | Default k/n | Binomial model (`sim/durability.py`), then Monte Carlo with uptime measured on the testnet | Smallest overhead where a 1 GiB file is readable ≥ 99.9% of the time at the measured median uptime. Current default 20-of-50 (needs ~70% holder uptime; 10-of-16 needs ~95%) | Confirm at the Phase 2 gate |
| 2 | Segment size | `bench/segment-size.sh` at 1, 4, 8, 16 MiB (needs the streaming seal/open now in place) | Largest size keeping peak RAM ≤ 32 MiB per operation, where per-operation RAM is **working memory**: the peak minus the same operation on a 1 KB file, so the fixed cost of the program and codec tables is excluded. Must hold for seal, open, heal and open-from-parity (reading fixed by the maintainer on 2026-10-14, before the final run; two earlier runs had been seen). Also test announcing once per file instead of per shard | Phase 1 if the heal memory fix is ready in time, else v0.0.2. Tool ready; waiting on the reference run with that fix |
| 3 | ~~Adopt multiformats CIDs?~~ | Desk check: encode links both ways; list features needing IPFS tooling | Adopt only if a planned feature needs it; otherwise keep the version byte. Leaning: no for v1 | **Settled: no.** No planned feature needs IPFS tooling; see [decision 0007](decisions/0007-no-multiformats-cids.md) |
| 4 | Channel tombstone format | Write from BEP 44 sequence rules | `seq = u64::MAX` + flag + optional successor link | Phase 2 |
| 5 | Lease length | Monte Carlo of owners and pinners going offline (7 / 30 / 90 days) | Shortest lease where an owner offline 3 weeks with no pinners loses nothing, and abandoned content is gone within two lease periods | Phase 3 |
| 6 | libp2p vs iroh | Two-evening spike each across home router, carrier-grade NAT and VPS | Higher direct-connection rate wins; on a tie, libp2p (built-in Kademlia) | Before Phase 2 |
| 7 | ~~Trusting `reed-solomon-simd`~~ | Check byte-for-byte parity against another implementation of the same code (Leopard first) | Match → differential test in CI. No match → golden vectors are the spec and the crate is vendored and frozen | **Settled: match** (high-rate, 64-byte shards, k ≥ 2). Differential test in CI; see [decision 0006](decisions/0006-erasure-code.md) |
| 8 | Challenge interval and punishment | Simulate swap partners with realistic sleep patterns | A node offline 8 h a night loses < 1% of swaps a month; a cheater loses all within 7 days | Phase 3 |
| 9 | Placement data (ASN, country) | Map the IP *we observe* with a public IP-to-ASN dataset (licence checked); never trust self-reports | ≥ 95% of testnet peers resolve | Phase 2 |
| 10 | Anonymity layer | Deferred by decision 0003; revisit with a Tor spike (`arti`) | — | After Phase 3 |

## Testing layers

Unit and property tests (`proptest`), one regression test per fixed bug, golden
vectors, fuzzing (`cargo-fuzz`), differential tests, benchmarks, supply-chain
checks (`cargo-deny`), the durability model, network simulation with
[Shadow](https://github.com/shadow/shadow) running the real daemon, a real
testnet, and SDK interop. Threat-model scenarios are in
[`spec/scenarios.md`](spec/scenarios.md).
