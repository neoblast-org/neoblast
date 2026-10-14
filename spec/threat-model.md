# Threat model (v1)

Neoblast v1 keeps published content **available and unreadable** against four
adversaries. It does **not** hide who published or read something.

| Adversary | Can | Defence | Built in |
| --- | --- | --- | --- |
| Hosting provider | Read its disks, delete data, shut down every VM it hosts, hand over logs, act on DMCA or abuse notices | Nodes hold only ciphertext. Placement puts at most one shard per provider/ASN per segment. Healing rebuilds lost shards elsewhere | Phase 2–3 |
| ISP | See who talks to whom, block IPs and ports, throttle, fingerprint protocols | QUIC on UDP 443 with no fixed port; HTTPS relays on TCP 443. A DPI-capable ISP can still tell someone runs Neoblast | Phase 2 |
| National firewall | Block bootstrap IPs and domains, poison DNS, block known node IPs, block BitTorrent's DHT | Multiple bootstrap paths: the Mainline DHT, signed peer lists, peer hints in links, cached peers. Shards spread across countries | Phase 2; pluggable transports later |
| Court order | Order the authors to remove content or add a backdoor; order GitHub, the registrar or app stores to drop code or domains; order local operators to delete or log | No kill switch exists. Code is mirrored with reproducible builds. The network does not need the domain. Operators comply locally with a denylist while the network heals elsewhere | From day 1 |

## Out of scope for v1

- Publisher and reader anonymity. Peers see your IP address.
- A global passive observer.
- A state-run Sybil swarm at scale (partly mitigated by node-ID proof-of-work and placement diversity).
- Compromised devices, or users forced to hand over keys.

## Design rules that follow

1. No message type, key or API may delete another party's shards. Only lease expiry removes data.
2. Nodes MUST take every parameter from a header verified against the file root.
3. Secrets MUST only appear after `#` in links.

## Scenario tests

Each adversary has scripted scenarios with pass criteria: see [scenarios.md](scenarios.md).
