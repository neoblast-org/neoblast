//! Neoblast: censorship-resistant storage built from encrypted, erasure-coded
//! shards that any node can verify and heal without the decryption key.
//!
//! **Experimental. Not audited. Do not rely on it to protect anything yet.**
//!
//! Phase 1 is fully offline: [`seal`] turns a payload into a verify manifest and
//! shards, [`open`] turns a read link plus enough shards back into the payload,
//! and [`heal`] rebuilds missing shards using only the verify cap. The
//! [`seal_stream`], [`open_stream`] and [`heal_stream`] variants hold one
//! segment in memory at a time and produce the same bytes;
//! [`open_manifest`] and [`heal_manifest`] do the same from a decoded
//! [`Manifest`], which [`Manifest::decode_owned`] makes without copying.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod file;
pub mod manifest;
pub mod store;
pub mod stream;
pub mod tree;

pub use file::{Error, Params, RebuiltShard, Sealed, ShardSource, heal, open, seal, seal_with_key};
pub use manifest::Manifest;
pub use neoblast_format::{FileReadCap, FileVerifyCap, Header};
pub use stream::{
    ShardSink, heal_manifest, heal_stream, open_manifest, open_stream, seal_stream,
    seal_stream_with_key,
};
