#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;

use neoblast::{Sealed, ShardSource};

/// In-memory shard source for tests.
#[derive(Default, Clone)]
pub struct MemStore(pub HashMap<(u32, u16), Vec<u8>>);

impl MemStore {
    pub fn from_sealed(sealed: &Sealed) -> Self {
        let mut map = HashMap::new();
        for (s, shards) in (0u32..).zip(&sealed.shards) {
            for (i, bytes) in (0u16..).zip(shards) {
                map.insert((s, i), bytes.clone());
            }
        }
        Self(map)
    }
}

impl ShardSource for MemStore {
    fn get(&self, segment: u32, index: u16) -> Option<Vec<u8>> {
        self.0.get(&(segment, index)).cloned()
    }
}
