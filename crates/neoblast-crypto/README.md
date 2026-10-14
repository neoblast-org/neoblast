# neoblast-crypto

Part of [Neoblast](https://github.com/neoblast-org/neoblast): censorship-resistant
storage where encrypted, erasure-coded shards can be verified and healed by
nodes that never hold the key.

This crate holds key derivation (BLAKE3) and segmented XChaCha20-Poly1305 encryption with the file header bound as associated data.

Most users want the [`neoblast`](https://crates.io/crates/neoblast) crate instead.
This one is published separately so other implementations and bindings can
reuse it. The byte formats are defined in the repository's spec/header.md.

**Experimental and not audited.** Licensed under MIT or Apache-2.0, at your option.
