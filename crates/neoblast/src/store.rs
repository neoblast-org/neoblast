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
//!
//! Every file is synced before it is renamed into place, and each directory
//! once after its last write, so what a seal or heal reports as written survives
//! a power loss (review W-007). Directories can only be synced on Unix, and a
//! file system that cannot sync them is accepted as it is (see `sync_dir`).
//!
//! On Apple systems a file gets a plain `fsync`, which leaves its data in the
//! drive's write cache, and its directory a full sync (`F_FULLFSYNC`), which
//! flushes that whole cache, taking every file on the same drive synced
//! before it along. So each segment pays for one full flush instead of one
//! per shard, and a file is durable once its directory has been synced. A file
//! and its directory are always on the same drive here. Elsewhere `fsync` is
//! all there is, and both get it.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use neoblast_format::{HEADER_LEN, Header};

use crate::file::{RebuiltShard, Sealed, ShardSource};
use crate::manifest::Manifest;
use crate::stream::ShardSink;

/// File name of the verify manifest inside a store directory.
pub const MANIFEST_FILE: &str = "manifest.nbl";

/// The largest manifest a store reads or writes, and so the largest any seal
/// makes ([`Params::header`](crate::Params::header)): 256 MiB, enough for files
/// of about 162 GiB at the default parameters (decision 0009). The manifest is
/// read into memory before it can be checked against the root, so a larger one
/// is refused unread (review W-001). Checking one costs little more than its
/// own size: the root is compared before anything else that size is built
/// (review S-005).
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
    /// The segment directory written last as a [`ShardSink`], not yet synced.
    /// [`ShardSink::finish`] syncs it.
    unsynced: Unsynced,
}

impl DirStore {
    /// Opens (without creating) a store at `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            unsynced: Unsynced::default(),
        }
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
        // Exactly the declared size, so reading never doubles the buffer.
        let mut bytes = Vec::with_capacity(
            usize::try_from(expected).map_err(|_| invalid("is larger than this store reads"))?,
        );
        bytes.extend_from_slice(&header);
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

    /// Writes a sealed file: every shard, then the manifest. A manifest too
    /// large to read back is refused before anything is written.
    pub fn write_sealed(&self, sealed: &Sealed) -> io::Result<()> {
        check_len(sealed.manifest.header())?;
        let mut unsynced = Unsynced::default();
        for (segment, shards) in (0u32..).zip(&sealed.shards) {
            for (index, bytes) in (0u16..).zip(shards) {
                let path = self.shard_path(segment, index);
                write_shard(&path, bytes)?;
                unsynced.wrote(&path)?;
            }
        }
        unsynced.sync()?;
        self.write_manifest_bytes(sealed.manifest.as_bytes())
    }

    /// Writes the verify manifest. Streaming seals call this after all shards.
    pub fn write_manifest(&self, manifest: &Manifest) -> io::Result<()> {
        check_len(manifest.header())?;
        self.write_manifest_bytes(manifest.as_bytes())
    }

    fn write_manifest_bytes(&self, bytes: &[u8]) -> io::Result<()> {
        let path = self.manifest_path();
        create_dirs(&self.root)
            .and_then(|()| replace_file(&path, bytes))
            .and_then(|()| sync_dir(&self.root))
            .map_err(|e| cannot("write", &path, e))
    }

    /// Writes rebuilt shards back into place, stopping at the first that
    /// cannot be written. [`heal_stream`](crate::heal_stream) with this store
    /// as its sink writes the others too.
    pub fn write_rebuilt(&self, shards: &[RebuiltShard]) -> io::Result<()> {
        let mut unsynced = Unsynced::default();
        for s in shards {
            let path = self.shard_path(s.segment, s.index);
            write_shard(&path, &s.bytes)?;
            unsynced.wrote(&path)?;
        }
        unsynced.sync()
    }
}

/// The directory of the last write, until it is synced. Each segment directory
/// is synced once, when the writes move on, instead of after every shard.
#[derive(Clone, Debug, Default)]
struct Unsynced(Option<PathBuf>);

impl Unsynced {
    /// Notes a finished write to `path`, syncing the previous directory first
    /// if `path` is in another one.
    fn wrote(&mut self, path: &Path) -> io::Result<()> {
        let dir = parent_dir(path);
        if self.0.as_deref() == Some(dir) {
            return Ok(());
        }
        let synced = self.sync();
        self.0 = Some(dir.to_owned());
        synced
    }

    /// Syncs the pending directory, if any.
    fn sync(&mut self) -> io::Result<()> {
        match self.0.take() {
            Some(dir) => sync_dir(&dir).map_err(|e| cannot("sync", &dir, e)),
            None => Ok(()),
        }
    }
}

/// Refuses a manifest that `read_manifest` would refuse to read back.
/// Measured from the header, before the manifest is encoded (review W-010).
fn check_len(header: &Header) -> io::Result<()> {
    if manifest_len(header).is_none_or(|len| len > MAX_MANIFEST_LEN) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "manifest is larger than a store reads",
        ));
    }
    Ok(())
}

/// Writes one shard; the caller syncs its directory. Errors name the shard's
/// path (review W-003).
fn write_shard(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let written = match fs::symlink_metadata(parent_dir(path)) {
        Ok(meta) if meta.is_dir() => Ok(()),
        Ok(_) => Err(not_plain(parent_dir(path))),
        Err(e) if e.kind() == io::ErrorKind::NotFound => create_dirs(parent_dir(path)),
        Err(e) => Err(e),
    };
    written
        .and_then(|()| replace_file(path, bytes))
        .map_err(|e| cannot("write", path, e))
}

fn cannot(what: &str, path: &Path, e: io::Error) -> io::Error {
    io::Error::new(e.kind(), format!("cannot {what} {}: {e}", path.display()))
}

fn not_plain(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("{} is not a plain file or directory", path.display()),
    )
}

/// Writes `bytes` to a new file beside `path`, syncs it and renames it over
/// `path` (review W-007); the caller syncs the directory. The rename replaces a
/// symlink at `path` rather than writing through it.
fn replace_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let (tmp, mut file) = create_temp(path)?;
    // From here on the temporary file is ours, so it is removed on failure.
    let written = file
        .write_all(bytes)
        .and_then(|()| sync_file(&file, path))
        .and_then(|()| fs::rename(&tmp, path));
    if written.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    written
}

/// Creates a temporary file `<path>.<random>.tmp`. `create_new` never follows
/// or truncates anything already there, and the random name means someone
/// else's files cannot take every name in advance (review W-004).
fn create_temp(path: &Path) -> io::Result<(PathBuf, File)> {
    let mut taken = None;
    for _ in 0..8 {
        let mut tmp = path.as_os_str().to_owned();
        tmp.push(format!(
            ".{:016x}.tmp",
            getrandom::u64().map_err(io::Error::other)?
        ));
        let tmp = PathBuf::from(tmp);
        match OpenOptions::new().write(true).create_new(true).open(&tmp) {
            Ok(file) => return Ok((tmp, file)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => taken = Some(e),
            Err(e) => return Err(e),
        }
    }
    Err(taken.unwrap_or_else(|| io::ErrorKind::AlreadyExists.into()))
}

/// `fs::create_dir_all`, then syncs the parent of every directory it created,
/// so the new directories survive a power loss (review W-007).
fn create_dirs(dir: &Path) -> io::Result<()> {
    let missing: Vec<&Path> = dir
        .ancestors()
        .take_while(|d| !d.as_os_str().is_empty() && fs::symlink_metadata(d).is_err())
        .collect();
    fs::create_dir_all(dir)?;
    for d in missing.iter().rev() {
        sync_dir(parent_dir(d))?;
    }
    Ok(())
}

/// The directory holding `path`; `.` for a bare file name.
fn parent_dir(path: &Path) -> &Path {
    match path.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir,
        _ => Path::new("."),
    }
}

/// Syncs the temporary file that will become `path`. On Apple systems this is
/// a plain `fsync`, retried if a signal interrupts it as `sync_all` is; the
/// data reaches stable storage with the full sync of its directory, which
/// every write is followed by (see the module docs).
fn sync_file(file: &File, path: &Path) -> io::Result<()> {
    #[cfg(target_vendor = "apple")]
    rustix::io::retry_on_intr(|| rustix::fs::fsync(file))?;
    #[cfg(not(target_vendor = "apple"))]
    file.sync_all()?;
    note_synced(path);
    Ok(())
}

/// Syncs a directory fully, so the entries renamed or created in it, and the
/// data of every file synced before it, survive a power loss. Only Unix can
/// open a directory to sync it; elsewhere this does nothing.
/// A file system that cannot sync directories at all (some network and FUSE
/// file systems) is accepted as it is, rather than making every write fail;
/// on Apple systems, files there get only their plain `fsync`.
fn sync_dir(dir: &Path) -> io::Result<()> {
    #[cfg(unix)]
    match File::open(dir)?.sync_all() {
        Err(e)
            if !matches!(
                e.kind(),
                io::ErrorKind::Unsupported | io::ErrorKind::InvalidInput
            ) =>
        {
            return Err(e);
        }
        _ => {}
    }
    note_synced(dir);
    Ok(())
}

#[cfg(test)]
thread_local! {
    /// Every file and directory synced on this thread, in order.
    static SYNCED: std::cell::RefCell<Vec<PathBuf>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Records a sync for the tests below; does nothing outside tests.
fn note_synced(_path: &Path) {
    #[cfg(test)]
    SYNCED.with_borrow_mut(|synced| synced.push(_path.to_owned()));
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
        let path = self.shard_path(segment, index);
        write_shard(&path, bytes)?;
        self.unsynced.wrote(&path)
    }

    fn finish(&mut self) -> io::Result<()> {
        self.unsynced.sync()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Params, heal, heal_stream, seal, seal_stream};

    const PARAMS: Params = Params {
        k: 2,
        n: 4,
        segment_size: 1024,
    };

    fn synced() -> Vec<PathBuf> {
        SYNCED.with_borrow_mut(std::mem::take)
    }

    /// True if `path` was synced and its directory was synced after its last sync.
    fn durable(log: &[PathBuf], path: &Path) -> bool {
        let dir = parent_dir(path);
        log.iter()
            .rposition(|p| p == path)
            .and_then(|at| log.get(at..))
            .is_some_and(|after| after.iter().any(|p| p == dir))
    }

    /// Checks that every file in `paths` was synced, and its directory after it.
    fn all_durable(log: &[PathBuf], paths: &[PathBuf]) {
        for path in paths {
            assert!(durable(log, path), "{} not synced", path.display());
        }
    }

    /// Checks that seal synced each segment directory it created into the store.
    fn dirs_durable(log: &[PathBuf], store: &DirStore, segments: u32) {
        for segment in 0..segments {
            let dir = store.shard_path(segment, 0);
            let dir = parent_dir(&dir);
            assert!(durable(log, dir), "{} not synced", dir.display());
        }
    }

    fn every_shard(store: &DirStore, segments: u32) -> Vec<PathBuf> {
        (0..segments)
            .flat_map(|s| (0..PARAMS.n).map(move |i| (s, i)))
            .map(|(s, i)| store.shard_path(s, i))
            .collect()
    }

    /// W-007: every way of writing a store syncs each file before renaming it
    /// into place and its directory after, and every new directory into its
    /// parent: streaming and in-memory seal, and streaming (in place or into a
    /// new store) and in-memory heal.
    #[test]
    fn w007_every_write_is_synced() {
        let tmp = std::env::temp_dir().join(format!("neoblast-w007-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let data = [7u8; 3000];

        let mut store = DirStore::new(tmp.join("stream"));
        let _ = synced();
        let (cap, manifest) = seal_stream(&data[..], 3000, PARAMS, &mut store).unwrap();
        store.write_manifest(&manifest).unwrap();
        let log = synced();
        let segments = manifest.header().segment_count();
        assert!(segments > 1);
        assert!(
            log.contains(&tmp),
            "the new store is synced into its parent"
        );
        assert!(durable(&log, &store.manifest_path()));
        all_durable(&log, &every_shard(&store, segments));
        dirs_durable(&log, &store, segments);

        let lost: Vec<PathBuf> = (0..segments).map(|s| store.shard_path(s, 1)).collect();
        for path in &lost {
            fs::remove_file(path).unwrap();
        }
        let bytes = store.read_manifest().unwrap();
        let rebuilt = heal_stream(&cap.verify_cap(), &bytes, &store, &mut store.clone()).unwrap();
        assert_eq!(rebuilt, lost.len());
        all_durable(&synced(), &lost);

        // Healing into a new store creates it and a segment directory in one
        // go; both are synced into their parents.
        let path = store.shard_path(0, 2);
        fs::remove_file(&path).unwrap();
        let mut fresh = DirStore::new(tmp.join("fresh"));
        let rebuilt = heal_stream(&cap.verify_cap(), &bytes, &store, &mut fresh).unwrap();
        assert_eq!(rebuilt, 1);
        let log = synced();
        assert!(
            log.contains(&tmp),
            "the new store is synced into its parent"
        );
        assert!(
            log.contains(&tmp.join("fresh")),
            "the new segment directory is synced"
        );
        all_durable(&log, &[fresh.shard_path(0, 2)]);

        let sealed = seal(&data, PARAMS).unwrap();
        let store = DirStore::new(tmp.join("in-memory"));
        store.write_sealed(&sealed).unwrap();
        let log = synced();
        assert!(durable(&log, &store.manifest_path()));
        all_durable(&log, &every_shard(&store, segments));
        dirs_durable(&log, &store, segments);

        let lost: Vec<PathBuf> = (0..segments).map(|s| store.shard_path(s, 1)).collect();
        for path in &lost {
            fs::remove_file(path).unwrap();
        }
        let bytes = store.read_manifest().unwrap();
        let rebuilt = heal(&sealed.cap.verify_cap(), &bytes, &store).unwrap();
        assert_eq!(rebuilt.len(), lost.len());
        store.write_rebuilt(&rebuilt).unwrap();
        all_durable(&synced(), &lost);
        fs::remove_dir_all(&tmp).unwrap();
    }
}
