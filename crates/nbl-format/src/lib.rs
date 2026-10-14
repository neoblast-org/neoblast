//! Neoblast byte formats.
//!
//! This crate is pure: no I/O, no randomness, no cryptography beyond what the
//! formats themselves define. Everything here is a fuzz target.
//!
//! * [`header`]: the 32-byte header every file root commits to (decision 0002).
//! * [`layout`]: how a file's length maps onto segments and shards.
//! * [`caps`]: capability links such as `nbl://f/<root>#<key>` (decision 0004).
#![cfg_attr(not(test), deny(clippy::indexing_slicing))]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod caps;
pub mod header;
pub mod layout;

pub use caps::{CapError, FileReadCap, FileVerifyCap};
pub use header::{HEADER_LEN, Header, HeaderError};
