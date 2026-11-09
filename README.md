# Neoblast

[![CI](https://github.com/neoblast-org/neoblast/actions/workflows/ci.yml/badge.svg)](https://github.com/neoblast-org/neoblast/actions/workflows/ci.yml)

**Censorship-resistant storage where encrypted shards heal themselves.**

> **Status: experimental. v0.0.3 is the offline core (Phase 1); Phase 2 is in
> progress. Not audited. Do not rely on it to protect anything yet.**

Neoblast encrypts a file on your machine, erasure-codes the ciphertext into
shards, and commits every byte to a single root hash. Storage nodes only ever
hold ciphertext. Any node can check a shard and rebuild a missing one
**without the decryption key**, so taking hosts down doesn't take the file
down. You publish by sharing a link:

```text
nbl://f/<root>#<read-key>
```

The key sits after `#`, so a web gateway serving the link never sees it.

## What works today (Phase 1)

```sh
cargo install --path crates/neoblast          # or: cargo run -p neoblast --

(umask 077; set -C; neoblast seal photo.jpg --out ./store >photo.link)   # the read link
rm ./store/seg-000000/00{0..9}.shard                # lose shards (up to n - k per segment)
neoblast heal 'nbl://fv/...' --from ./store         # rebuild them with the verify link only
neoblast open --from ./store --out photo-copy.jpg <photo.link
```

Anyone who can read `photo.link` can decrypt the file, so `umask 077` makes
a new one readable by you alone (review W-051). It leaves the mode of a file
that already exists as it is, so `set -C` makes the shell refuse to write
into one, and `seal` does not run: remove an old `photo.link` first (reviews
W-061 and W-066). `open` and `heal` read the link from stdin, a file or a
pipe, when it is left out or `-`. A read link given as an argument works
too, but other local users can see it, and the shell keeps it in its
history.

Defaults are 20-of-50 shards over 1 MiB segments ([decision
0009](decisions/0009-segment-size.md)). `seal`, `open` and `heal` work one
segment at a time, so the memory they allocate does not grow with the file
size, apart from the manifest's shard hashes (about 0.16% of the file). It
grows with a segment's n shards instead: 2.6 MB at the defaults, but up to
several GiB for parameters a publisher may choose (review S-037), so the
CLI refuses a file that would need more than 1 GiB unless `--max-memory`
raises the limit (decision 0011). Heal's measured peak still steps up a few
MiB between a 256 MiB and a 1 GiB file (decision 0009, finding 2).
The network (Phase 2), storage swaps and leases (Phase 3) and blind search
(Phase 4) are not built yet.
See [ROADMAP.md](ROADMAP.md) for phases, gates and open questions.

## What Neoblast protects, and what it doesn't

v1 keeps content **available and unreadable** against a hosting provider, an
ISP, a national firewall and a court order. It does **not** hide who published
or read something: peers you exchange shards with see your IP address. Use Tor
yourself if you need that. Full threat model: [`spec/threat-model.md`](spec/threat-model.md).

There is no kill switch. No key, server or index exists that could delete
content from the network, including for the maintainers.

## Layout

| Path | What |
| --- | --- |
| `crates/neoblast-format` | Header, layout rules, capability links. Pure, fuzzed |
| `crates/neoblast-crypto` | Key derivation, segmented XChaCha20-Poly1305 |
| `crates/neoblast-erasure` | Reed-Solomon behind a swappable trait |
| `crates/neoblast` | The main crate and the `neoblast` CLI. All four crates are published (see [RELEASING.md](RELEASING.md)) |
| `spec/` | The format spec and golden test vectors |
| `decisions/` | Design decision records (MADR) |
| `fuzz/` | `cargo-fuzz` targets |
| `sim/` | Durability model used to choose k/n |
| `ROADMAP.md` | Phases, gates and open items |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Report security issues privately:
[SECURITY.md](SECURITY.md).

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
