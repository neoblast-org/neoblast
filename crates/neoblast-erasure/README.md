# neoblast-erasure

Part of [Neoblast](https://github.com/neoblast-org/neoblast): censorship-resistant
storage where encrypted, erasure-coded shards can be verified and healed by
nodes that never hold the key.

This crate holds erasure coding: Reed-Solomon over GF(2^16) using the Leopard-RS high-rate algorithm, checked byte for byte against the original Leopard C library.

Most users want the [`neoblast`](https://crates.io/crates/neoblast) crate instead.
This one is published separately so other implementations and bindings can
reuse it. The byte formats are defined in the repository's spec/header.md and decision 0006.

**Experimental and not audited.** Licensed under MIT or Apache-2.0, at your option.
