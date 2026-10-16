//! The verify manifest: everything a storage node needs to check shards
//! against the file root, and nothing that reveals content.
//!
//! Encoding: `header (32 bytes) || shard hashes`, where the hashes are listed
//! segment by segment, `n` per segment, 32 bytes each. The manifest is only
//! accepted if it hashes to the expected file root `H`.

use neoblast_format::{HEADER_LEN, Header};

use crate::file::Error;
use crate::tree::{self, Hash, TreeBuilder};

/// A verified manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    header: Header,
    /// Every shard hash, segment by segment, `n` per segment.
    hashes: Vec<Hash>,
    root: Hash,
}

impl Manifest {
    /// Builds a manifest from a header and per-segment shard hashes.
    pub fn new(header: Header, hashes: Vec<Vec<Hash>>) -> Result<Self, Error> {
        if hashes.len() != header.segment_count() as usize
            || hashes.iter().any(|s| s.len() != usize::from(header.n()))
        {
            return Err(Error::Manifest);
        }
        let root = compute_root(&header, hashes.iter().map(|s| s.iter().copied()))?;
        Ok(Self {
            header,
            hashes: hashes.concat(),
            root,
        })
    }

    /// [`Manifest::new`] for hashes already listed segment by segment in one
    /// list, as a streaming seal collects them.
    pub(crate) fn from_flat(header: Header, hashes: Vec<Hash>) -> Result<Self, Error> {
        let n = usize::from(header.n());
        if Some(hashes.len()) != (header.segment_count() as usize).checked_mul(n) || n == 0 {
            return Err(Error::Manifest);
        }
        let root = compute_root(&header, hashes.chunks_exact(n).map(|s| s.iter().copied()))?;
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
        let n = usize::from(self.header.n());
        if usize::from(index) >= n {
            return None;
        }
        let at = usize::try_from(segment)
            .ok()?
            .checked_mul(n)?
            .checked_add(usize::from(index))?;
        self.hashes.get(at)
    }

    /// Encodes the manifest.
    pub fn encode(&self) -> Vec<u8> {
        let mut out =
            Vec::with_capacity(HEADER_LEN.saturating_add(self.hashes.len().saturating_mul(32)));
        out.extend_from_slice(&self.header.encode());
        for h in &self.hashes {
            out.extend_from_slice(h);
        }
        out
    }

    /// Decodes a manifest and checks it against the expected root.
    /// Never panics on any input.
    ///
    /// The root is computed from `bytes` as they are, one segment at a time,
    /// and compared before anything the size of the manifest is allocated, so
    /// rejecting a planted manifest costs little beyond `bytes` (review S-005).
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
        let root = compute_root(
            &header,
            rest.chunks_exact(n.saturating_mul(32))
                .map(|segment| segment.chunks_exact(32).map(to_hash)),
        )?;
        if &root != expected_root {
            return Err(Error::RootMismatch);
        }
        note_built();
        Ok(Self {
            header,
            hashes: rest.chunks_exact(32).map(to_hash).collect(),
            root,
        })
    }
}

fn to_hash(bytes: &[u8]) -> Hash {
    bytes.try_into().unwrap_or([0u8; 32])
}

/// The file root over `segments`, each yielding its n shard hashes in order.
/// Holds one segment's tree and one node per level of the content tree. The
/// callers check that there are `segment_count` segments of n hashes each.
fn compute_root<S>(header: &Header, segments: impl Iterator<Item = S>) -> Result<Hash, Error>
where
    S: Iterator<Item = Hash>,
{
    let mut content = TreeBuilder::default();
    for (segment, i) in segments.zip(0u32..) {
        let root = tree::segment_root_of(segment).ok_or(Error::Manifest)?;
        content.push(tree::segment_leaf(i, &root));
    }
    let content = content.finish().ok_or(Error::Manifest)?;
    Ok(tree::file_root(&header.encode(), &content))
}

#[cfg(test)]
thread_local! {
    /// How many decoded manifests built their list of hashes on this thread.
    static BUILT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Counts a built list of hashes for the tests below; does nothing outside tests.
fn note_built() {
    #[cfg(test)]
    BUILT.set(BUILT.get() + 1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Params, seal_with_key};

    /// S-005: a manifest that does not match the link's root is rejected
    /// before its hashes are copied into a list, so a planted one near the
    /// size cap costs little more than its own bytes. This marks only the hash
    /// list; that nothing else sized by the manifest is built first is
    /// measured, not tested (the peak memory in the S-005 status line).
    #[test]
    fn s005_the_root_is_compared_before_the_hashes_are_listed() {
        let params = Params {
            k: 2,
            n: 3,
            segment_size: 64,
        };
        let sealed = seal_with_key(&[5u8; 1000], params, &[1u8; 32]).unwrap();
        let bytes = sealed.manifest.encode();
        assert!(sealed.manifest.header().segment_count() > 1);

        BUILT.set(0);
        let err = Manifest::decode(&bytes, &[0u8; 32]).unwrap_err();
        assert!(matches!(err, Error::RootMismatch), "{err}");
        assert_eq!(BUILT.get(), 0);

        let decoded = Manifest::decode(&bytes, &sealed.cap.root).unwrap();
        assert_eq!(BUILT.get(), 1);
        assert_eq!(decoded, sealed.manifest);
        assert_eq!(decoded.encode(), bytes);
    }

    #[test]
    fn shard_hash_is_none_outside_the_manifest() {
        let params = Params {
            k: 2,
            n: 3,
            segment_size: 64,
        };
        let sealed = seal_with_key(&[5u8; 200], params, &[1u8; 32]).unwrap();
        let m = &sealed.manifest;
        let last = m.header().segment_count() - 1;
        assert!(last > 0);
        // An index past n must not reach into the next segment.
        assert!(m.shard_hash(0, 3).is_none());
        assert!(m.shard_hash(last, 2).is_some());
        assert!(m.shard_hash(last, 3).is_none());
        assert!(m.shard_hash(last + 1, 0).is_none());
        assert!(m.shard_hash(0, u16::MAX).is_none());
        assert!(m.shard_hash(u32::MAX, 0).is_none());
    }
}
