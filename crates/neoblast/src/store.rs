//! A directory of shards: the Phase 1 stand-in for the network.
//!
//! ```text
//! <dir>/manifest.nbl
//! <dir>/seg-000000/000.shard
//! <dir>/seg-000000/001.shard
//! ...
//! ```

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::file::{RebuiltShard, Sealed, ShardSource};

/// File name of the verify manifest inside a store directory.
pub const MANIFEST_FILE: &str = "manifest.nbl";

/// A shard store backed by a local directory.
#[derive(Clone, Debug)]
pub struct DirStore {
    root: PathBuf,
}

impl DirStore {
    /// Opens (without creating) a store at `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Path of one shard file.
    pub fn shard_path(&self, segment: u32, index: u16) -> PathBuf {
        self.root
            .join(format!("seg-{segment:06}"))
            .join(format!("{index:03}.shard"))
    }

    /// Path of the manifest.
    pub fn manifest_path(&self) -> PathBuf {
        self.root.join(MANIFEST_FILE)
    }

    /// Reads the manifest bytes.
    pub fn read_manifest(&self) -> io::Result<Vec<u8>> {
        fs::read(self.manifest_path())
    }

    /// Writes a sealed file: manifest plus every shard.
    pub fn write_sealed(&self, sealed: &Sealed) -> io::Result<()> {
        fs::create_dir_all(&self.root)?;
        fs::write(self.manifest_path(), sealed.manifest.encode())?;
        for (segment, shards) in (0u32..).zip(&sealed.shards) {
            for (index, bytes) in (0u16..).zip(shards) {
                write_shard(&self.shard_path(segment, index), bytes)?;
            }
        }
        Ok(())
    }

    /// Writes rebuilt shards back into place.
    pub fn write_rebuilt(&self, shards: &[RebuiltShard]) -> io::Result<()> {
        for s in shards {
            write_shard(&self.shard_path(s.segment, s.index), &s.bytes)?;
        }
        Ok(())
    }
}

fn write_shard(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)
}

impl ShardSource for DirStore {
    fn get(&self, segment: u32, index: u16) -> Option<Vec<u8>> {
        fs::read(self.shard_path(segment, index)).ok()
    }
}
