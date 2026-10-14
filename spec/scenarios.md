# Threat-model scenario tests

Each adversary in [threat-model.md](threat-model.md) gets scripted scenarios.
They run in [Shadow](https://github.com/shadow/shadow) first (repeatable, one
machine), then the cheap ones are repeated on the real testnet. A scenario
passes only if every criterion holds. They become runnable once the Phase 2
harness exists.

| ID | Adversary | Scenario | Pass criteria |
| --- | --- | --- | --- |
| H1 | Hosting provider | Kill every node at the largest provider (≥ 30% of the network) at once | Every file stays readable; no segment ever had two shards on one provider; lost shards rebuilt elsewhere within 24 h simulated |
| H2 | Hosting provider | Inspect a storage node's disk | Only ciphertext and headers; no file names, plaintext or read keys on non-owner nodes |
| I1 | ISP | Block UDP for a group of nodes | They reach the network through relays on TCP 443; fetches succeed |
| I2 | ISP | Capture traffic between two nodes | No plaintext, link keys or file names on the wire |
| F1 | National firewall | Partition a 20% "country"; block default bootstrap nodes and Neoblast's DHT inside it | Nodes inside join within 5 minutes via Mainline, link peer hints or cached peers |
| F2 | National firewall | Delete every shard held inside the country | Content readable inside and outside; healing happens outside |
| F3 | National firewall | Also block the Mainline DHT | Peer hints or relays still work; otherwise the gap is documented |
| C1 | Court order | Remove the project's bootstrap nodes, domain and GitHub releases | The network carries on; a new node joins from a link's peer hints alone; a mirror build reproduces the release hash |
| C2 | Court order | An operator denylists a root hash | That node refuses it; content stays readable elsewhere; its shards are rebuilt on other nodes |
| C3 | Court order | Look for any way for maintainers to delete content | No message type, key or API deletes another party's shards; only lease expiry removes data (enforced by a test listing every protocol message) |
| R1 | Malicious peer | Send forged headers, shards and proofs | All rejected, no crash, peer score drops |
| R2 | Free-rider | Discard everything it agreed to store | Loses all swaps within 7 days |
| R3 | Honest sleeper | Offline 8 hours every night | Loses < 1% of swaps a month |
| R4 | Sybil flood | 1,000 new identities try to upload | Each capped at the free allowance after proof-of-work; no honest data evicted |
| R5 | Stale records | Nodes serve an old channel record | Readers get the latest version whenever one queried node has it |
| R6 | Eclipse attempt | 20% malicious nodes cluster IDs near one file root | Lookups succeed in ≥ 99% of trials via disjoint paths |
