//! Neoblast: censorship-resistant storage built from encrypted, erasure-coded
//! shards that any node can verify and heal without the decryption key.
//!
//! **Experimental. Not audited. Do not rely on it to protect anything yet.**
//!
//! Phase 1 is fully offline: [`seal`] turns a payload into a verify manifest and
//! shards, [`open`] turns a read link plus enough shards back into the payload,
//! and [`heal`] rebuilds missing shards using only the verify cap.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod file;
pub mod manifest;
pub mod store;
pub mod tree;

pub use file::{Error, Params, RebuiltShard, Sealed, ShardSource, heal, open, seal, seal_with_key};
pub use manifest::Manifest;
pub use neoblast_format::{FileReadCap, FileVerifyCap, Header};
