//! Sealing, opening and healing files (Phase 1, offline).

use std::io;

use neoblast_crypto::CryptoError;
use neoblast_erasure::ErasureError;
use neoblast_format::{FileReadCap, FileVerifyCap, Header, HeaderError};
use thiserror::Error;

use crate::manifest::Manifest;
use crate::store::{MAX_MANIFEST_LEN, manifest_len};
use crate::stream::{self, ShardSink};

/// Errors from sealing, opening or healing.
#[derive(Debug, Error)]
pub enum Error {
    /// Bad parameters for a new file.
    #[error("invalid parameters: {0}")]
    Params(#[from] HeaderError),
    /// The file's manifest would be larger than [`MAX_MANIFEST_LEN`], so a
    /// store could not read it back. Larger segments make it smaller.
    #[error("the manifest would be larger than {} MiB; use larger segments", MAX_MANIFEST_LEN >> 20)]
    ManifestTooLarge,
    /// Encryption or decryption failed.
    #[error(transparent)]
    Crypto(#[from] CryptoError),
    /// Erasure coding failed.
    #[error(transparent)]
    Erasure(#[from] ErasureError),
    /// The manifest is malformed.
    #[error("malformed manifest")]
    Manifest,
    /// The manifest does not hash to the link's root.
    #[error("manifest does not match the link's root")]
    RootMismatch,
    /// A segment has fewer than k verified shards.
    #[error("segment {segment}: {have} verified shards, need {need}")]
    Unrecoverable {
        /// Segment index.
        segment: u32,
        /// Verified shards found.
        have: usize,
        /// Shards needed (k).
        need: usize,
    },
    /// A rebuilt shard did not match its listed hash. Indicates a bug or a broken codec.
    #[error("rebuilt shard {index} of segment {segment} does not match the manifest")]
    RebuildMismatch {
        /// Segment index.
        segment: u32,
        /// Shard index.
        index: u16,
    },
    /// The input to a streaming seal was shorter or longer than declared.
    #[error("input was not exactly {expected} bytes")]
    InputLength {
        /// The declared payload length.
        expected: u64,
    },
    /// Reading the input or writing shards or output failed.
    #[error(transparent)]
    Io(#[from] io::Error),
}

/// Erasure-coding parameters for a new file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Params {
    /// Data shards per segment.
    pub k: u16,
    /// Total shards per segment.
    pub n: u16,
    /// Plaintext bytes per segment.
    pub segment_size: u32,
}

impl Params {
    /// Plans the header for a `payload_len`-byte file with these parameters.
    /// Fails with [`Error::Params`] for parameters outside the v1 limits, and
    /// with [`Error::ManifestTooLarge`] if the manifest would be larger than
    /// [`MAX_MANIFEST_LEN`], so every seal can refuse before it writes anything
    /// (review W-006).
    pub fn header(&self, payload_len: u64) -> Result<Header, Error> {
        let header = Header::new(payload_len, self.k, self.n, self.segment_size)?;
        match manifest_len(&header) {
            Some(len) if len <= MAX_MANIFEST_LEN => Ok(header),
            _ => Err(Error::ManifestTooLarge),
        }
    }
}

impl Default for Params {
    /// 20-of-50 (the durability check's current leaning) with 1 MiB segments
    /// (decision 0009).
    fn default() -> Self {
        Self {
            k: 20,
            n: 50,
            segment_size: 1024 * 1024,
        }
    }
}

/// The result of sealing a file.
pub struct Sealed {
    /// The shareable read capability (`nbl://f/...`).
    pub cap: FileReadCap,
    /// The verify manifest. Safe to give to storage nodes.
    pub manifest: Manifest,
    /// `shards[segment][index]`.
    pub shards: Vec<Vec<Vec<u8>>>,
}

/// Somewhere shards can be read from: a directory now, the network in Phase 2.
pub trait ShardSource {
    /// Returns the bytes stored for a shard, if any. Bytes are untrusted and
    /// are checked against the manifest by the caller.
    ///
    /// `len` is the length the verified header requires for this shard. A
    /// source should never read or return more than that, so a planted
    /// oversized shard cannot exhaust memory; [`DirStore`](crate::store::DirStore)
    /// returns `None` for a file of any other size.
    fn get(&self, segment: u32, index: u16, len: usize) -> Option<Vec<u8>>;
}

/// A shard rebuilt by [`heal`], already checked against the manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RebuiltShard {
    /// Segment index.
    pub segment: u32,
    /// Shard index within the segment.
    pub index: u16,
    /// The shard bytes.
    pub bytes: Vec<u8>,
}

/// Seals `payload` under a fresh random read key. Holds every shard in memory;
/// use [`seal_stream`](crate::seal_stream) for large files.
pub fn seal(payload: &[u8], params: Params) -> Result<Sealed, Error> {
    let key = neoblast_crypto::random_read_key()?;
    seal_with_key(payload, params, &key)
}

/// Seals `payload` under a caller-chosen read key. Deterministic, so it is what
/// the golden vectors use. Never reuse a read key for two different payloads.
/// Like every seal, refuses parameters whose manifest is over
/// [`MAX_MANIFEST_LEN`] ([`Params::header`]).
pub fn seal_with_key(payload: &[u8], params: Params, read_key: &[u8; 32]) -> Result<Sealed, Error> {
    let mut shards = MemShards::default();
    let (cap, manifest) =
        stream::seal_stream_with_key(payload, payload.len() as u64, params, read_key, &mut shards)?;
    Ok(Sealed {
        cap,
        manifest,
        shards: shards.0,
    })
}

/// Opens a file from its read link, its manifest and any `k` good shards per
/// segment, returning the whole payload. Use [`open_stream`](crate::open_stream)
/// for large files.
pub fn open(
    cap: &FileReadCap,
    manifest_bytes: &[u8],
    source: &impl ShardSource,
) -> Result<Vec<u8>, Error> {
    let mut payload = Vec::new();
    stream::open_stream(cap, manifest_bytes, source, &mut payload)?;
    Ok(payload)
}

/// Rebuilds every missing or corrupt shard using only the verify cap, and
/// returns them all. Use [`heal_stream`](crate::heal_stream) for large files.
///
/// Shards that fail their hash check are ignored rather than aborting the heal
/// (code-review issue 2), so one bad peer cannot block repair.
pub fn heal(
    cap: &FileVerifyCap,
    manifest_bytes: &[u8],
    source: &impl ShardSource,
) -> Result<Vec<RebuiltShard>, Error> {
    let mut rebuilt = Rebuilt::default();
    stream::heal_stream(cap, manifest_bytes, source, &mut rebuilt)?;
    Ok(rebuilt.0)
}

/// Collects sealed shards as `shards[segment][index]`.
#[derive(Default)]
struct MemShards(Vec<Vec<Vec<u8>>>);

impl ShardSink for MemShards {
    fn put(&mut self, segment: u32, index: u16, bytes: &[u8]) -> io::Result<()> {
        if segment as usize == self.0.len() {
            self.0.push(Vec::new());
        }
        let current = segment as usize + 1 == self.0.len();
        match self.0.last_mut() {
            Some(shards) if current && usize::from(index) == shards.len() => {
                shards.push(bytes.to_vec());
                Ok(())
            }
            _ => Err(io::Error::other("shards arrived out of order")),
        }
    }
}

/// Collects rebuilt shards.
#[derive(Default)]
struct Rebuilt(Vec<RebuiltShard>);

impl ShardSink for Rebuilt {
    fn put(&mut self, segment: u32, index: u16, bytes: &[u8]) -> io::Result<()> {
        self.0.push(RebuiltShard {
            segment,
            index,
            bytes: bytes.to_vec(),
        });
        Ok(())
    }
}
