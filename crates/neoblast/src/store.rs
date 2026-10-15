//! A directory of shards: the Phase 1 stand-in for the network.
//!
//! ```text
//! <dir>/manifest.nbl
//! <dir>/seg-000000/000.shard
//! <dir>/seg-000000/001.shard
//! ...
//! ```
//!
//! The directory may come from someone else, so it is treated as hostile (review
//! S-001): only regular files are read, a symlinked segment directory is neither
//! read nor written, and files are written to a new temporary file that is
//! renamed into place, so a planted symlink is replaced, never followed. The
//! store root itself is the caller's choice and may be a symlink. The checks run just before each
//! open; a directory changed by someone racing the running command is out of scope.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use neoblast_format::{HEADER_LEN, Header};

use crate::file::{RebuiltShard, Sealed, ShardSource};
use crate::manifest::Manifest;
use crate::stream::ShardSink;

/// File name of the verify manifest inside a store directory.
pub const MANIFEST_FILE: &str = "manifest.nbl";

/// The largest manifest a store reads or writes: 256 MiB, enough for files of
/// about 640 GiB at the default parameters. The manifest is read into memory
/// before it can be checked against the root, so a larger one is refused
/// unread (review W-001).
pub const MAX_MANIFEST_LEN: u64 = 256 * 1024 * 1024;

/// The size in bytes of the manifest for `header`: the header, then one 32-byte
/// hash per shard. `None` if it overflows a `u64`.
pub fn manifest_len(header: &Header) -> Option<u64> {
    u64::from(header.segment_count())
        .checked_mul(u64::from(header.n()))?
        .checked_mul(32)?
        .checked_add(HEADER_LEN as u64)
}

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
    /// declares and be at most [`MAX_MANIFEST_LEN`], so neither a planted
    /// oversized manifest nor a huge one that matches its own header is read.
    /// The caller still checks the manifest against the link's root.
    pub fn read_manifest(&self) -> io::Result<Vec<u8>> {
        let mut file = open_regular(&self.manifest_path(), &self.root)?;
        let mut header = [0u8; HEADER_LEN];
        file.read_exact(&mut header)?;
        let invalid =
            |what: &str| io::Error::new(io::ErrorKind::InvalidData, format!("manifest {what}"));
        let h = Header::decode(&header).map_err(|_| invalid("header is invalid"))?;
        let expected = manifest_len(&h)
            .filter(|&len| len <= MAX_MANIFEST_LEN)
            .ok_or_else(|| invalid("is larger than this store reads"))?;
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
        replace_file(
            &self.manifest_path(),
            &checked_len(sealed.manifest.encode())?,
        )?;
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
        replace_file(&self.manifest_path(), &checked_len(manifest.encode())?)
    }

    /// Writes rebuilt shards back into place.
    pub fn write_rebuilt(&self, shards: &[RebuiltShard]) -> io::Result<()> {
        for s in shards {
            write_shard(&self.shard_path(s.segment, s.index), &s.bytes)?;
        }
        Ok(())
    }
}

/// Refuses a manifest that `read_manifest` would refuse to read back.
fn checked_len(manifest: Vec<u8>) -> io::Result<Vec<u8>> {
    if manifest.len() as u64 > MAX_MANIFEST_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "manifest is larger than a store reads",
        ));
    }
    Ok(manifest)
}

fn write_shard(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        match fs::symlink_metadata(dir) {
            Ok(meta) if meta.is_dir() => {}
            Ok(_) => return Err(not_plain(dir)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => fs::create_dir_all(dir)?,
            Err(e) => return Err(e),
        }
    }
    replace_file(path, bytes)
}

fn not_plain(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("{} is not a plain file or directory", path.display()),
    )
}

/// Writes `bytes` to a new file beside `path`, then renames it over `path`. The
/// rename replaces a symlink at `path` rather than writing through it, and
/// `create_new` never follows or truncates anything already at the temporary name.
fn replace_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(format!(".{}.tmp", std::process::id()));
    let tmp = PathBuf::from(tmp);
    let mut file = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
    // From here on the temporary file is ours, so it is removed on failure.
    let written = file.write_all(bytes).and_then(|()| fs::rename(&tmp, path));
    if written.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    written
}

/// Opens `path` only if it is a regular file, not a symlink, FIFO or device,
/// in a directory that is not a symlink either. The store root is the caller's
/// choice and is not checked.
fn open_regular(path: &Path, root: &Path) -> io::Result<File> {
    if let Some(dir) = path.parent().filter(|&dir| dir != root) {
        if !fs::symlink_metadata(dir)?.is_dir() {
            return Err(not_plain(dir));
        }
    }
    if !fs::symlink_metadata(path)?.is_file() {
        return Err(not_plain(path));
    }
    File::open(path)
}

impl ShardSource for DirStore {
    /// Reads a shard only if the file is exactly `len` bytes long.
    fn get(&self, segment: u32, index: u16, len: usize) -> Option<Vec<u8>> {
        let file = open_regular(&self.shard_path(segment, index), &self.root).ok()?;
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
