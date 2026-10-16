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
///
/// Held as its encoding and nothing else, so a file's manifest is in memory
/// once, and read in place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    header: Header,
    /// The encoding: the header, then every shard hash, `n` per segment.
    bytes: Vec<u8>,
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
        let mut bytes = Vec::with_capacity(encoded_len(&header).ok_or(Error::Manifest)?);
        bytes.extend_from_slice(&header.encode());
        hashes
            .iter()
            .flatten()
            .for_each(|h| bytes.extend_from_slice(h));
        Self::from_encoding(bytes)
    }

    /// A manifest from its encoding, as a streaming seal builds it: the root
    /// is computed, not checked.
    pub(crate) fn from_encoding(bytes: Vec<u8>) -> Result<Self, Error> {
        let (header, root) = parse(&bytes)?;
        Ok(Self {
            header,
            bytes,
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
            .checked_add(usize::from(index))?
            .checked_mul(32)?
            .checked_add(HEADER_LEN)?;
        self.bytes.get(at..at.checked_add(32)?)?.try_into().ok()
    }

    /// The manifest's encoding, as stored and as hashed into the root.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Encodes the manifest: a copy of [`Manifest::as_bytes`].
    pub fn encode(&self) -> Vec<u8> {
        self.bytes.clone()
    }

    /// Decodes a manifest and checks it against the expected root.
    /// Never panics on any input.
    ///
    /// The root is computed from `bytes` as they are, one segment at a time,
    /// and compared before anything the size of the manifest is allocated, so
    /// rejecting a planted manifest costs little beyond `bytes` (review S-005).
    /// [`Manifest::decode_owned`] keeps the bytes it is given instead of a copy.
    pub fn decode(bytes: &[u8], expected_root: &Hash) -> Result<Self, Error> {
        let (header, root) = parse(bytes)?;
        if &root != expected_root {
            return Err(Error::RootMismatch);
        }
        note_built();
        Ok(Self {
            header,
            bytes: bytes.to_vec(),
            root,
        })
    }

    /// [`Manifest::decode`] that takes `bytes` over as the manifest instead of
    /// copying them, so a caller that read the manifest holds it only once.
    /// Nothing the manifest's size is allocated, whether it is accepted or not.
    pub fn decode_owned(bytes: Vec<u8>, expected_root: &Hash) -> Result<Self, Error> {
        let (header, root) = parse(&bytes)?;
        if &root != expected_root {
            return Err(Error::RootMismatch);
        }
        Ok(Self {
            header,
            bytes,
            root,
        })
    }
}

/// The encoded length for `header`: the header, then `n` hashes per segment.
fn encoded_len(header: &Header) -> Option<usize> {
    usize::try_from(header.segment_count())
        .ok()?
        .checked_mul(usize::from(header.n()))?
        .checked_mul(32)?
        .checked_add(HEADER_LEN)
}

/// Checks an encoding's header and length and computes its root, one segment
/// at a time, without allocating anything its size.
fn parse(bytes: &[u8]) -> Result<(Header, Hash), Error> {
    let head = bytes.get(..HEADER_LEN).ok_or(Error::Manifest)?;
    let header = Header::decode(head).map_err(|_| Error::Manifest)?;
    if Some(bytes.len()) != encoded_len(&header) {
        return Err(Error::Manifest);
    }
    let n = usize::from(header.n());
    let rest = bytes.get(HEADER_LEN..).ok_or(Error::Manifest)?;
    let root = compute_root(
        &header,
        rest.chunks_exact(n.saturating_mul(32))
            .map(|segment| segment.chunks_exact(32).map(to_hash)),
    )?;
    Ok((header, root))
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
    /// How many times [`Manifest::decode`] copied its input on this thread.
    static BUILT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Counts a copy of the input for the tests below; does nothing outside tests.
fn note_built() {
    #[cfg(test)]
    BUILT.set(BUILT.get() + 1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Params, seal_with_key};

    /// S-005: a manifest that does not match the link's root is rejected
    /// before its bytes are copied, so a planted one near the size cap costs
    /// little more than its own bytes. This marks only the copy; that nothing
    /// else the manifest's size is allocated first is tested exactly in
    /// `bench/heal-heap/tests/`.
    #[test]
    fn s005_the_root_is_compared_before_the_bytes_are_copied() {
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

    /// `decode_owned` keeps the caller's buffer as the manifest: no copy, so
    /// the CLI holds the manifest once.
    #[test]
    fn decode_owned_keeps_the_callers_bytes() {
        let params = Params {
            k: 2,
            n: 3,
            segment_size: 64,
        };
        let sealed = seal_with_key(&[5u8; 1000], params, &[1u8; 32]).unwrap();
        let bytes = sealed.manifest.encode();
        let at = bytes.as_ptr();
        BUILT.set(0);
        let decoded = Manifest::decode_owned(bytes, &sealed.cap.root).unwrap();
        assert_eq!(decoded.as_bytes().as_ptr(), at);
        assert_eq!(BUILT.get(), 0);
        assert_eq!(decoded, sealed.manifest);
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
