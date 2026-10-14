//! A directory of shards: the Phase 1 stand-in for the network.
//!
//! ```text
//! <dir>/manifest.nbl
//! <dir>/seg-000000/000.shard
//! <dir>/seg-000000/001.shard
//! ...
//! ```

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use neoblast_format::{HEADER_LEN, Header};

use crate::file::{RebuiltShard, Sealed, ShardSource};
use crate::manifest::Manifest;
use crate::stream::ShardSink;

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

    /// Reads the manifest bytes. The file's size must match what its header
    /// declares, so a planted oversized manifest is refused before it is read.
    /// The caller still checks the manifest against the link's root.
    pub fn read_manifest(&self) -> io::Result<Vec<u8>> {
        let mut file = File::open(self.manifest_path())?;
        let mut header = [0u8; HEADER_LEN];
        file.read_exact(&mut header)?;
        let invalid =
            |what: &str| io::Error::new(io::ErrorKind::InvalidData, format!("manifest {what}"));
        let h = Header::decode(&header).map_err(|_| invalid("header is invalid"))?;
        let expected = u64::from(h.segment_count())
            .checked_mul(u64::from(h.n()))
            .and_then(|hashes| hashes.checked_mul(32))
            .and_then(|bytes| bytes.checked_add(HEADER_LEN as u64))
            .ok_or_else(|| invalid("is too large"))?;
        if file.metadata()?.len() != expected {
            return Err(invalid("has the wrong size"));
        }
        let mut bytes = header.to_vec();
        file.take(expected - HEADER_LEN as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 != expected {
            return Err(invalid("changed while being read"));
        }
        Ok(bytes)
    }

    /// True if the store directory is missing or empty.
    pub fn is_empty(&self) -> io::Result<bool> {
        match fs::read_dir(&self.root) {
            Ok(mut entries) => Ok(entries.next().is_none()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(true),
            Err(e) => Err(e),
        }
    }

    /// Removes the manifest if there is one, so the directory no longer
    /// describes a complete file. Used before overwriting a store.
    pub fn remove_manifest(&self) -> io::Result<()> {
        match fs::remove_file(self.manifest_path()) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
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

    /// Writes the verify manifest. Streaming seals call this after all shards.
    pub fn write_manifest(&self, manifest: &Manifest) -> io::Result<()> {
        fs::create_dir_all(&self.root)?;
        fs::write(self.manifest_path(), manifest.encode())
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
    /// Reads a shard only if the file is exactly `len` bytes long.
    fn get(&self, segment: u32, index: u16, len: usize) -> Option<Vec<u8>> {
        let file = File::open(self.shard_path(segment, index)).ok()?;
        if file.metadata().ok()?.len() != len as u64 {
            return None;
        }
        let mut bytes = Vec::with_capacity(len);
        file.take(len as u64 + 1).read_to_end(&mut bytes).ok()?;
        (bytes.len() == len).then_some(bytes)
    }
}

impl ShardSink for DirStore {
    fn put(&mut self, segment: u32, index: u16, bytes: &[u8]) -> io::Result<()> {
        write_shard(&self.shard_path(segment, index), bytes)
    }
}
