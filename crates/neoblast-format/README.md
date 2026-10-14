# neoblast-format

Part of [Neoblast](https://github.com/neoblast-org/neoblast): censorship-resistant
storage where encrypted, erasure-coded shards can be verified and healed by
nodes that never hold the key.

This crate holds the byte formats: the 32-byte committed file header, layout rules (Padmé padding, segment and shard sizes) and `nbl://` capability links. Pure Rust, no I/O, no `unsafe`.

Most users want the [`neoblast`](https://crates.io/crates/neoblast) crate instead.
This one is published separately so other implementations and bindings can
reuse it. The byte formats are defined in the repository's spec/header.md and spec/caps.md.

**Experimental and not audited.** Licensed under MIT or Apache-2.0, at your option.
