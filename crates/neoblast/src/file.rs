//! Sealing, opening and healing files (Phase 1, offline).

use neoblast_crypto::CryptoError;
use neoblast_erasure::{ErasureError, ReedSolomonSimd};
use neoblast_format::{FileReadCap, FileVerifyCap, Header, HeaderError};
use thiserror::Error;
use zeroize::Zeroizing;

use crate::manifest::Manifest;
use crate::tree;

/// Errors from sealing, opening or healing.
#[derive(Debug, Error)]
pub enum Error {
    /// Bad parameters for a new file.
    #[error("invalid parameters: {0}")]
    Params(#[from] HeaderError),
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

impl Default for Params {
    /// 20-of-50 with 4 MiB segments: the current leaning from the durability check.
    fn default() -> Self {
        Self {
            k: 20,
            n: 50,
            segment_size: 4 * 1024 * 1024,
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
    /// Returns the bytes stored for a shard, if any. Bytes are untrusted.
    fn get(&self, segment: u32, index: u16) -> Option<Vec<u8>>;
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

/// Seals `payload` under a fresh random read key.
pub fn seal(payload: &[u8], params: Params) -> Result<Sealed, Error> {
    let key = neoblast_crypto::random_read_key()?;
    seal_with_key(payload, params, &key)
}

/// Seals `payload` under a caller-chosen read key. Deterministic, so it is what
/// the golden vectors use. Never reuse a read key for two different payloads.
pub fn seal_with_key(payload: &[u8], params: Params, read_key: &[u8; 32]) -> Result<Sealed, Error> {
    let header = Header::new(
        payload.len() as u64,
        params.k,
        params.n,
        params.segment_size,
    )?;
    let ciphertexts = neoblast_crypto::seal_segments(read_key, &header, payload)?;
    let k = usize::from(header.k());
    let n = usize::from(header.n());
    let mut shards = Vec::with_capacity(ciphertexts.len());
    let mut hashes = Vec::with_capacity(ciphertexts.len());
    for (ct, i) in ciphertexts.iter().zip(0u32..) {
        let shard_len = shard_len(&header, i)?;
        let segment = neoblast_erasure::encode_segment(&ReedSolomonSimd, ct, k, n, shard_len)?;
        hashes.push(segment.iter().map(|s| tree::shard_hash(s)).collect());
        shards.push(segment);
    }
    let manifest = Manifest::new(header, hashes)?;
    let cap = FileReadCap {
        root: manifest.root(),
        read_key: *read_key,
    };
    Ok(Sealed {
        cap,
        manifest,
        shards,
    })
}

/// Opens a file from its read link, its manifest and any `k` good shards per segment.
pub fn open(
    cap: &FileReadCap,
    manifest_bytes: &[u8],
    source: &impl ShardSource,
) -> Result<Vec<u8>, Error> {
    let manifest = Manifest::decode(manifest_bytes, &cap.root)?;
    let header = *manifest.header();
    let k = usize::from(header.k());
    let mut ciphertexts = Vec::with_capacity(header.segment_count() as usize);
    for segment in 0..header.segment_count() {
        let (held, _) = collect_verified(&manifest, segment, source);
        let shard_len = shard_len(&header, segment)?;
        let data = neoblast_erasure::reconstruct_data(
            &ReedSolomonSimd,
            k,
            usize::from(header.n()),
            shard_len,
            &held,
        )
        .map_err(|e| unrecoverable(e, segment, &held, k))?;
        let mut ct: Vec<u8> = data.concat();
        let ct_len = header
            .segment_cipher_len(segment)
            .and_then(|l| usize::try_from(l).ok())
            .ok_or(Error::Manifest)?;
        ct.truncate(ct_len);
        ciphertexts.push(ct);
    }
    let key = Zeroizing::new(cap.read_key);
    Ok(neoblast_crypto::open_segments(&key, &header, &ciphertexts)?)
}

/// Rebuilds every missing or corrupt shard using only the verify cap.
///
/// Shards that fail their hash check are ignored rather than aborting the heal
/// (code-review issue 2), so one bad peer cannot block repair.
pub fn heal(
    cap: &FileVerifyCap,
    manifest_bytes: &[u8],
    source: &impl ShardSource,
) -> Result<Vec<RebuiltShard>, Error> {
    let manifest = Manifest::decode(manifest_bytes, &cap.root)?;
    let header = *manifest.header();
    let k = usize::from(header.k());
    let n = usize::from(header.n());
    let mut rebuilt = Vec::new();
    for segment in 0..header.segment_count() {
        let (held, missing) = collect_verified(&manifest, segment, source);
        if missing.is_empty() {
            continue;
        }
        let shard_len = shard_len(&header, segment)?;
        let all = neoblast_erasure::regenerate(&ReedSolomonSimd, k, n, shard_len, &held)
            .map_err(|e| unrecoverable(e, segment, &held, k))?;
        for index in missing {
            let bytes = all
                .get(usize::from(index))
                .cloned()
                .ok_or(Error::RebuildMismatch { segment, index })?;
            if manifest.shard_hash(segment, index) != Some(&tree::shard_hash(&bytes)) {
                return Err(Error::RebuildMismatch { segment, index });
            }
            rebuilt.push(RebuiltShard {
                segment,
                index,
                bytes,
            });
        }
    }
    Ok(rebuilt)
}

/// Fetches every shard of a segment, keeps those matching the manifest, and
/// lists the indices that are missing or bad.
fn collect_verified(
    manifest: &Manifest,
    segment: u32,
    source: &impl ShardSource,
) -> (Vec<Option<Vec<u8>>>, Vec<u16>) {
    let n = manifest.header().n();
    let mut held = Vec::with_capacity(usize::from(n));
    let mut missing = Vec::new();
    for index in 0..n {
        let good = source
            .get(segment, index)
            .filter(|bytes| manifest.shard_hash(segment, index) == Some(&tree::shard_hash(bytes)));
        if good.is_none() {
            missing.push(index);
        }
        held.push(good);
    }
    (held, missing)
}

fn shard_len(header: &Header, segment: u32) -> Result<usize, Error> {
    header
        .segment_shard_len(segment)
        .and_then(|l| usize::try_from(l).ok())
        .ok_or(Error::Manifest)
}

fn unrecoverable(e: ErasureError, segment: u32, held: &[Option<Vec<u8>>], k: usize) -> Error {
    match e {
        ErasureError::NotEnough { .. } => Error::Unrecoverable {
            segment,
            have: held.iter().flatten().count(),
            need: k,
        },
        other => Error::Erasure(other),
    }
}
