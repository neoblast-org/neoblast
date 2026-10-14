//! The 32-byte file header (decision 0002).
//!
//! Layout, all integers big-endian:
//!
//! | offset | bytes | field           |
//! |--------|-------|-----------------|
//! | 0      | 4     | magic `NBL\x01` |
//! | 4      | 1     | suite           |
//! | 5      | 2     | k               |
//! | 7      | 2     | n               |
//! | 9      | 4     | segment_size    |
//! | 13     | 4     | shard_size      |
//! | 17     | 4     | segment_count   |
//! | 21     | 8     | padded_length   |
//! | 29     | 2     | flags           |
//! | 31     | 1     | reserved        |
//!
//! A header is only ever constructed through [`Header::new`] or
//! [`Header::decode`], both of which run [`Header::validate`]. Code that holds a
//! `Header` can therefore trust every field: this is the fix for code-review
//! issues 1 and 3 (forged or out-of-range parameters).

use thiserror::Error;

use crate::layout;

/// Length of an encoded header in bytes.
pub const HEADER_LEN: usize = 32;

/// Magic bytes plus format version 1.
pub const MAGIC_V1: [u8; 4] = *b"NBL\x01";

/// Suite 1: XChaCha20-Poly1305, BLAKE3, and Reed-Solomon over GF(2^16) using the
/// Leopard-RS high-rate algorithm (decision 0006).
pub const SUITE_V1: u8 = 0x01;

/// Smallest k allowed in v1. With k = 1, Leopard-RS treats the code as plain
/// replication and its parity bytes differ from `reed-solomon-simd` (decision 0006).
pub const MIN_K: u16 = 2;
/// Largest k allowed in v1.
pub const MAX_K: u16 = 64;
/// Largest n allowed in v1.
pub const MAX_N: u16 = 128;
/// Largest segment allowed in v1 (64 MiB).
pub const MAX_SEGMENT_SIZE: u32 = 64 * 1024 * 1024;
/// Smallest segment allowed in v1.
pub const MIN_SEGMENT_SIZE: u32 = 64;
/// Bytes of the big-endian length prefix inside the framed plaintext.
pub const LENGTH_PREFIX: u64 = 8;

/// Reasons a header is rejected.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum HeaderError {
    /// Input was not exactly [`HEADER_LEN`] bytes.
    #[error("header must be {HEADER_LEN} bytes, got {0}")]
    Length(usize),
    /// Magic or version bytes did not match.
    #[error("bad magic or unsupported version")]
    Magic,
    /// Unknown cipher suite.
    #[error("unsupported suite {0:#04x}")]
    Suite(u8),
    /// k or n outside the v1 limits.
    #[error("invalid k/n: need {MIN_K} <= k < n <= {MAX_N} and k <= {MAX_K}, got k={k} n={n}")]
    Params {
        /// Data shards per segment.
        k: u16,
        /// Total shards per segment.
        n: u16,
    },
    /// Segment size outside the v1 limits.
    #[error("segment size {0} outside {MIN_SEGMENT_SIZE}..={MAX_SEGMENT_SIZE}")]
    SegmentSize(u32),
    /// The length, segment count and shard size disagree with each other.
    #[error("layout fields are inconsistent")]
    Layout,
    /// Flags or reserved byte set in v1.
    #[error("flags and reserved byte must be zero in v1")]
    Reserved,
}

/// A validated file header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    k: u16,
    n: u16,
    segment_size: u32,
    shard_size: u32,
    segment_count: u32,
    padded_length: u64,
}

impl Header {
    /// Plans and validates a header for a payload of `payload_len` bytes.
    ///
    /// The framed plaintext is an 8-byte big-endian length prefix, the payload,
    /// then zero padding up to the Padmé length (see [`layout::padme`]).
    pub fn new(payload_len: u64, k: u16, n: u16, segment_size: u32) -> Result<Self, HeaderError> {
        let framed = payload_len
            .checked_add(LENGTH_PREFIX)
            .ok_or(HeaderError::Layout)?;
        let padded_length = layout::padme(framed);
        let segment_count = layout::segment_count(padded_length, segment_size)
            .ok_or(HeaderError::SegmentSize(segment_size))?;
        let shard_size =
            layout::shard_len(u64::from(segment_size), k).ok_or(HeaderError::Layout)?;
        let header = Self {
            k,
            n,
            segment_size,
            shard_size,
            segment_count,
            padded_length,
        };
        header.validate()?;
        Ok(header)
    }

    /// Checks every field and the relationships between them.
    pub fn validate(&self) -> Result<(), HeaderError> {
        if self.k < MIN_K || self.k > MAX_K || self.n <= self.k || self.n > MAX_N {
            return Err(HeaderError::Params {
                k: self.k,
                n: self.n,
            });
        }
        if !(MIN_SEGMENT_SIZE..=MAX_SEGMENT_SIZE).contains(&self.segment_size) {
            return Err(HeaderError::SegmentSize(self.segment_size));
        }
        if self.padded_length < LENGTH_PREFIX
            || layout::padme(self.padded_length) != self.padded_length
        {
            return Err(HeaderError::Layout);
        }
        if layout::segment_count(self.padded_length, self.segment_size) != Some(self.segment_count)
        {
            return Err(HeaderError::Layout);
        }
        if layout::shard_len(u64::from(self.segment_size), self.k) != Some(self.shard_size) {
            return Err(HeaderError::Layout);
        }
        Ok(())
    }

    /// Encodes the header to its 32 canonical bytes.
    pub fn encode(&self) -> [u8; HEADER_LEN] {
        let mut out = [0u8; HEADER_LEN];
        let mut w = Writer::new(&mut out);
        w.put(&MAGIC_V1);
        w.put(&[SUITE_V1]);
        w.put(&self.k.to_be_bytes());
        w.put(&self.n.to_be_bytes());
        w.put(&self.segment_size.to_be_bytes());
        w.put(&self.shard_size.to_be_bytes());
        w.put(&self.segment_count.to_be_bytes());
        w.put(&self.padded_length.to_be_bytes());
        w.put(&0u16.to_be_bytes());
        w.put(&[0u8]);
        out
    }

    /// Decodes and validates a header. Never panics on any input.
    pub fn decode(bytes: &[u8]) -> Result<Self, HeaderError> {
        let bytes: &[u8; HEADER_LEN] = bytes
            .try_into()
            .map_err(|_| HeaderError::Length(bytes.len()))?;
        let mut r = Reader::new(bytes);
        if r.take::<4>() != MAGIC_V1 {
            return Err(HeaderError::Magic);
        }
        let [suite] = r.take::<1>();
        if suite != SUITE_V1 {
            return Err(HeaderError::Suite(suite));
        }
        let k = u16::from_be_bytes(r.take());
        let n = u16::from_be_bytes(r.take());
        let segment_size = u32::from_be_bytes(r.take());
        let shard_size = u32::from_be_bytes(r.take());
        let segment_count = u32::from_be_bytes(r.take());
        let padded_length = u64::from_be_bytes(r.take());
        let flags = u16::from_be_bytes(r.take());
        let [reserved] = r.take::<1>();
        if flags != 0 || reserved != 0 {
            return Err(HeaderError::Reserved);
        }
        let header = Self {
            k,
            n,
            segment_size,
            shard_size,
            segment_count,
            padded_length,
        };
        header.validate()?;
        Ok(header)
    }

    /// Data shards needed per segment.
    pub fn k(&self) -> u16 {
        self.k
    }
    /// Total shards per segment.
    pub fn n(&self) -> u16 {
        self.n
    }
    /// Plaintext bytes in every segment except possibly the last.
    pub fn segment_size(&self) -> u32 {
        self.segment_size
    }
    /// Shard length for a full segment.
    pub fn shard_size(&self) -> u32 {
        self.shard_size
    }
    /// Number of segments.
    pub fn segment_count(&self) -> u32 {
        self.segment_count
    }
    /// Length of the framed, padded plaintext.
    pub fn padded_length(&self) -> u64 {
        self.padded_length
    }

    /// Framed-plaintext bytes in segment `index`, or `None` if out of range.
    pub fn segment_plain_len(&self, index: u32) -> Option<u64> {
        if index >= self.segment_count {
            return None;
        }
        let before = u64::from(index).checked_mul(u64::from(self.segment_size))?;
        let rest = self.padded_length.checked_sub(before)?;
        Some(rest.min(u64::from(self.segment_size)))
    }

    /// Ciphertext bytes in segment `index` (plaintext plus the 16-byte tag).
    pub fn segment_cipher_len(&self, index: u32) -> Option<u64> {
        self.segment_plain_len(index)?.checked_add(layout::TAG_LEN)
    }

    /// Shard length for segment `index`.
    pub fn segment_shard_len(&self, index: u32) -> Option<u32> {
        layout::shard_len(self.segment_plain_len(index)?, self.k)
    }
}

struct Writer<'a> {
    buf: &'a mut [u8; HEADER_LEN],
    pos: usize,
}

impl<'a> Writer<'a> {
    fn new(buf: &'a mut [u8; HEADER_LEN]) -> Self {
        Self { buf, pos: 0 }
    }
    fn put(&mut self, bytes: &[u8]) {
        for (dst, src) in self.buf.iter_mut().skip(self.pos).zip(bytes) {
            *dst = *src;
        }
        self.pos = self.pos.saturating_add(bytes.len());
    }
}

struct Reader<'a> {
    buf: &'a [u8; HEADER_LEN],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(buf: &'a [u8; HEADER_LEN]) -> Self {
        Self { buf, pos: 0 }
    }
    fn take<const N: usize>(&mut self) -> [u8; N] {
        let mut out = [0u8; N];
        for (dst, src) in out.iter_mut().zip(self.buf.iter().skip(self.pos)) {
            *dst = *src;
        }
        self.pos = self.pos.saturating_add(N);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn round_trip_default_params() {
        let h = Header::new(1_000_000, 20, 50, 4 * 1024 * 1024).unwrap();
        assert_eq!(Header::decode(&h.encode()).unwrap(), h);
    }

    #[test]
    fn rejects_k_greater_than_n() {
        // Code-review issue 1: a peer sending n < k must be rejected, never panic.
        let mut bytes = Header::new(100, 4, 7, 1024).unwrap().encode();
        bytes[5..7].copy_from_slice(&9u16.to_be_bytes());
        assert!(matches!(
            Header::decode(&bytes),
            Err(HeaderError::Params { .. })
        ));
    }

    #[test]
    fn rejects_k_of_one() {
        // Decision 0006: k = 1 is outside the Leopard-compatible range.
        assert!(matches!(
            Header::new(100, 1, 4, 1024),
            Err(HeaderError::Params { .. })
        ));
    }

    #[test]
    fn rejects_inconsistent_layout() {
        let mut bytes = Header::new(100_000, 4, 7, 1024).unwrap().encode();
        bytes[17..21].copy_from_slice(&1u32.to_be_bytes());
        assert_eq!(Header::decode(&bytes), Err(HeaderError::Layout));
    }

    #[test]
    fn rejects_nonzero_reserved() {
        let mut bytes = Header::new(10, 4, 7, 1024).unwrap().encode();
        bytes[31] = 1;
        assert_eq!(Header::decode(&bytes), Err(HeaderError::Reserved));
    }

    proptest! {
        #[test]
        fn decode_never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..64)) {
            let _ = Header::decode(&bytes);
        }

        #[test]
        fn valid_headers_round_trip(
            len in 0u64..(1 << 40),
            k in MIN_K..=MAX_K,
            extra in 1u16..=64,
            seg in MIN_SEGMENT_SIZE..=MAX_SEGMENT_SIZE,
        ) {
            let n = k.saturating_add(extra).min(MAX_N);
            prop_assume!(n > k);
            let h = Header::new(len, k, n, seg).unwrap();
            prop_assert_eq!(Header::decode(&h.encode()).unwrap(), h);
            let total: u64 = (0..h.segment_count()).map(|i| h.segment_plain_len(i).unwrap()).sum();
            prop_assert_eq!(total, h.padded_length());
        }

        #[test]
        fn any_single_byte_change_is_caught_or_changes_header(
            pos in 0usize..HEADER_LEN,
            flip in 1u8..=255,
        ) {
            let h = Header::new(123_456, 20, 50, 4096).unwrap();
            let mut bytes = h.encode();
            bytes[pos] ^= flip;
            if let Ok(other) = Header::decode(&bytes) {
                prop_assert_ne!(other, h);
            }
        }
    }
}
