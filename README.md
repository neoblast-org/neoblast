# Neoblast

[![CI](https://github.com/neoblast-org/neoblast/actions/workflows/ci.yml/badge.svg)](https://github.com/neoblast-org/neoblast/actions/workflows/ci.yml)

**Censorship-resistant storage where encrypted shards heal themselves.**

> **Status: experimental. v0.0.1 is the offline core (Phase 1); Phase 2 is in
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

neoblast seal photo.jpg --out ./store          # prints an nbl://f/... read link
rm ./store/seg-000000/00{0..9}.shard           # lose shards (up to n - k per segment)
neoblast heal  'nbl://fv/...' --from ./store   # rebuild them with the verify link only
neoblast open  'nbl://f/...#...' --from ./store --out photo-copy.jpg
```

Defaults are 20-of-50 shards over 4 MiB segments. `seal`, `open` and `heal`
work one segment at a time, so their memory use is meant not to grow with the
file size. Measured on macOS, `heal`'s still does; see [CHANGELOG.md](CHANGELOG.md).
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
