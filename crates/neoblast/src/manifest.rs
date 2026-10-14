//! The verify manifest: everything a storage node needs to check shards
//! against the file root, and nothing that reveals content.
//!
//! Encoding: `header (32 bytes) || shard hashes`, where the hashes are listed
//! segment by segment, `n` per segment, 32 bytes each. The manifest is only
//! accepted if it hashes to the expected file root `H`.

use nbl_format::{HEADER_LEN, Header};

use crate::file::Error;
use crate::tree::{self, Hash};

/// A verified manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    header: Header,
    hashes: Vec<Vec<Hash>>,
    root: Hash,
}

impl Manifest {
    /// Builds a manifest from a header and per-segment shard hashes.
    pub fn new(header: Header, hashes: Vec<Vec<Hash>>) -> Result<Self, Error> {
        let root = compute_root(&header, &hashes)?;
        Ok(Self {
            header,
            hashes,
            root,
        })
    }

    /// The validated header.
    pub fn header(&self) -> &Header {
        &self.header
    }

    /// The file root `H`.
    pub fn root(&self) -> Hash {
        self.root
    }

    /// Expected hash of shard `index` in `segment`.
    pub fn shard_hash(&self, segment: u32, index: u16) -> Option<&Hash> {
        self.hashes
            .get(usize::try_from(segment).ok()?)?
            .get(usize::from(index))
    }

    /// Encodes the manifest.
    pub fn encode(&self) -> Vec<u8> {
        let count: usize = self.hashes.iter().map(Vec::len).sum();
        let mut out = Vec::with_capacity(HEADER_LEN.saturating_add(count.saturating_mul(32)));
        out.extend_from_slice(&self.header.encode());
        for segment in &self.hashes {
            for h in segment {
                out.extend_from_slice(h);
            }
        }
        out
    }

    /// Decodes a manifest and checks it against the expected root.
    /// Never panics on any input.
    pub fn decode(bytes: &[u8], expected_root: &Hash) -> Result<Self, Error> {
        let (head, rest) = bytes.split_at_checked(HEADER_LEN).ok_or(Error::Manifest)?;
        let header = Header::decode(head).map_err(|_| Error::Manifest)?;
        let n = usize::from(header.n());
        let segments = usize::try_from(header.segment_count()).map_err(|_| Error::Manifest)?;
        let want = n
            .checked_mul(segments)
            .and_then(|c| c.checked_mul(32))
            .ok_or(Error::Manifest)?;
        if rest.len() != want {
            return Err(Error::Manifest);
        }
        let hashes: Vec<Vec<Hash>> = rest
            .chunks_exact(n.saturating_mul(32))
            .map(|seg| {
                seg.chunks_exact(32)
                    .map(|c| c.try_into().unwrap_or([0u8; 32]))
                    .collect()
            })
            .collect();
        let manifest = Self::new(header, hashes)?;
        if &manifest.root != expected_root {
            return Err(Error::RootMismatch);
        }
        Ok(manifest)
    }
}

fn compute_root(header: &Header, hashes: &[Vec<Hash>]) -> Result<Hash, Error> {
    if hashes.len() != header.segment_count() as usize
        || hashes.iter().any(|s| s.len() != usize::from(header.n()))
    {
        return Err(Error::Manifest);
    }
    let segment_roots: Vec<Hash> = hashes
        .iter()
        .map(|s| tree::segment_root(s).ok_or(Error::Manifest))
        .collect::<Result<_, _>>()?;
    let content = tree::content_root(&segment_roots).ok_or(Error::Manifest)?;
    Ok(tree::file_root(&header.encode(), &content))
}
