#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use neoblast::{Sealed, ShardSource};

/// A fresh scratch directory under the target directory, removed at the start.
pub fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

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
    fn get(&self, segment: u32, index: u16, _len: usize) -> Option<Vec<u8>> {
        self.0.get(&(segment, index)).cloned()
    }
}
