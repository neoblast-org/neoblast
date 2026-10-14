//! Neoblast key derivation and segmented authenticated encryption.
//!
//! * Every file gets a fresh random 32-byte read key `R` (decision 0004).
//! * The data key is `blake3::derive_key("nbl/v1/file-data", R)`.
//! * The framed plaintext (8-byte big-endian length, payload, zero padding to the
//!   Padmé length) is split into segments. Each segment is sealed with
//!   XChaCha20-Poly1305 under a deterministic nonce built from the segment index
//!   and a last-segment flag (the STREAM construction), with the 32-byte header
//!   as associated data. Reordering, truncating or moving segments between files
//!   therefore fails authentication (code-review issues 5 and 6).
//! * The real length lives inside the ciphertext, never in the clear (issue 4).
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use chacha20poly1305::XChaCha20Poly1305;
use chacha20poly1305::aead::{AeadInOut, KeyInit};
use neoblast_format::Header;
use thiserror::Error;
use zeroize::Zeroizing;

/// KDF context for the per-file data key.
pub const CTX_FILE_DATA: &str = "nbl/v1/file-data";

/// Length of keys and hashes.
pub const KEY_LEN: usize = 32;

/// Errors from sealing or opening.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CryptoError {
    /// The operating system's random source failed.
    #[error("random number generator unavailable")]
    Rng,
    /// The payload length does not match the header.
    #[error("payload length does not match the header")]
    LengthMismatch,
    /// A segment is missing or has the wrong length.
    #[error("segment {0} missing or wrong length")]
    Segment(u32),
    /// Authentication failed: wrong key, tampered data, or reordered segments.
    #[error("segment {0} failed authentication")]
    Auth(u32),
    /// The decrypted framing is malformed (bad length prefix or non-zero padding).
    #[error("malformed plaintext framing")]
    Framing,
}

/// Generates a fresh random read key.
pub fn random_read_key() -> Result<Zeroizing<[u8; KEY_LEN]>, CryptoError> {
    let mut key = Zeroizing::new([0u8; KEY_LEN]);
    getrandom::fill(key.as_mut()).map_err(|_| CryptoError::Rng)?;
    Ok(key)
}

/// Derives the data-encryption key from a file's read key.
pub fn data_key(read_key: &[u8; KEY_LEN]) -> Zeroizing<[u8; KEY_LEN]> {
    Zeroizing::new(blake3::derive_key(CTX_FILE_DATA, read_key))
}

/// Builds the 24-byte nonce for segment `index`.
///
/// Layout: 15 zero bytes, the index as a big-endian `u64`, then `0x01` for the
/// last segment and `0x00` otherwise. The key is unique per file, so a
/// deterministic nonce never repeats under the same key.
pub fn segment_nonce(index: u32, last: bool) -> [u8; 24] {
    let mut nonce = [0u8; 24];
    for (dst, src) in nonce
        .iter_mut()
        .skip(15)
        .zip(u64::from(index).to_be_bytes())
    {
        *dst = src;
    }
    if let Some(flag) = nonce.last_mut() {
        *flag = u8::from(last);
    }
    nonce
}

/// Encrypts and decrypts the segments of one file, one at a time.
///
/// Used by the streaming paths, which hold a single segment in memory. Each call
/// checks the buffer length against the header, so a segment can't be sealed or
/// opened at the wrong size or index.
pub struct SegmentCipher {
    cipher: XChaCha20Poly1305,
    aad: [u8; neoblast_format::HEADER_LEN],
    header: Header,
}

impl core::fmt::Debug for SegmentCipher {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Never print key material.
        f.debug_struct("SegmentCipher").finish_non_exhaustive()
    }
}

impl SegmentCipher {
    /// Prepares the data key for the file described by `header`.
    pub fn new(read_key: &[u8; KEY_LEN], header: &Header) -> Self {
        let key = data_key(read_key);
        Self {
            cipher: XChaCha20Poly1305::new(&(*key).into()),
            aad: header.encode(),
            header: *header,
        }
    }

    /// Encrypts segment `index` in place. `buf` must hold exactly that segment's
    /// framed plaintext; on success it holds the ciphertext (16 bytes longer).
    pub fn seal_in_place(&self, index: u32, buf: &mut Vec<u8>) -> Result<(), CryptoError> {
        let want = self
            .header
            .segment_plain_len(index)
            .ok_or(CryptoError::Segment(index))?;
        if buf.len() as u64 != want {
            return Err(CryptoError::LengthMismatch);
        }
        let nonce = segment_nonce(index, self.is_last(index));
        self.cipher
            .encrypt_in_place(&nonce.into(), &self.aad, buf)
            .map_err(|_| CryptoError::Auth(index))
    }

    /// Decrypts and authenticates segment `index` in place. `buf` must hold
    /// exactly that segment's ciphertext; on success it holds the framed plaintext.
    pub fn open_in_place(&self, index: u32, buf: &mut Vec<u8>) -> Result<(), CryptoError> {
        let want = self
            .header
            .segment_cipher_len(index)
            .ok_or(CryptoError::Segment(index))?;
        if buf.len() as u64 != want {
            return Err(CryptoError::Segment(index));
        }
        let nonce = segment_nonce(index, self.is_last(index));
        self.cipher
            .decrypt_in_place(&nonce.into(), &self.aad, buf)
            .map_err(|_| CryptoError::Auth(index))
    }

    fn is_last(&self, index: u32) -> bool {
        index.checked_add(1) == Some(self.header.segment_count())
    }
}

/// Encrypts `payload` into one ciphertext per segment, as laid out by `header`.
///
/// Holds the whole file in memory. The `neoblast` crate's streaming functions
/// produce the same bytes one segment at a time.
pub fn seal_segments(
    read_key: &[u8; KEY_LEN],
    header: &Header,
    payload: &[u8],
) -> Result<Vec<Vec<u8>>, CryptoError> {
    let expected = Header::new(
        payload.len() as u64,
        header.k(),
        header.n(),
        header.segment_size(),
    )
    .map_err(|_| CryptoError::LengthMismatch)?;
    if &expected != header {
        return Err(CryptoError::LengthMismatch);
    }
    let framed_len =
        usize::try_from(header.padded_length()).map_err(|_| CryptoError::LengthMismatch)?;
    let mut framed = Zeroizing::new(Vec::with_capacity(framed_len));
    framed.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    framed.extend_from_slice(payload);
    framed.resize(framed_len, 0);

    let cipher = SegmentCipher::new(read_key, header);
    let seg = usize::try_from(header.segment_size()).map_err(|_| CryptoError::LengthMismatch)?;
    framed
        .chunks(seg)
        .zip(0u32..)
        .map(|(chunk, i)| {
            // Room for the tag up front, so the plaintext is never reallocated,
            // and wiped if sealing fails.
            let mut buf = Zeroizing::new(Vec::with_capacity(chunk.len() + 16));
            buf.extend_from_slice(chunk);
            cipher.seal_in_place(i, &mut buf)?;
            Ok(core::mem::take(&mut *buf))
        })
        .collect()
}

/// Decrypts and authenticates every segment, then strips the framing.
pub fn open_segments(
    read_key: &[u8; KEY_LEN],
    header: &Header,
    segments: &[Vec<u8>],
) -> Result<Vec<u8>, CryptoError> {
    let count = header.segment_count();
    if segments.len() != count as usize {
        return Err(CryptoError::Segment(
            segments.len().try_into().unwrap_or(u32::MAX),
        ));
    }
    let cipher = SegmentCipher::new(read_key, header);
    let mut framed = Zeroizing::new(Vec::with_capacity(
        usize::try_from(header.padded_length()).map_err(|_| CryptoError::Framing)?,
    ));
    for (segment, i) in segments.iter().zip(0u32..) {
        let mut buf = Zeroizing::new(segment.clone());
        cipher.open_in_place(i, &mut buf)?;
        framed.extend_from_slice(&buf);
    }
    unframe(&framed)
}

fn unframe(framed: &[u8]) -> Result<Vec<u8>, CryptoError> {
    const _: () = assert!(neoblast_format::header::LENGTH_PREFIX == 8);
    let (prefix, rest) = framed
        .split_first_chunk::<8>()
        .ok_or(CryptoError::Framing)?;
    let len = usize::try_from(u64::from_be_bytes(*prefix)).map_err(|_| CryptoError::Framing)?;
    if len > rest.len() {
        return Err(CryptoError::Framing);
    }
    let (payload, padding) = rest.split_at(len);
    // Canonical encoding: padding must be all zero.
    if padding.iter().any(|&b| b != 0) {
        return Err(CryptoError::Framing);
    }
    Ok(payload.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn header_for(len: usize, seg: u32) -> Header {
        Header::new(len as u64, 4, 7, seg).unwrap()
    }

    #[test]
    fn nonce_layout() {
        let n = segment_nonce(258, true);
        assert_eq!(&n[..15], &[0u8; 15]);
        assert_eq!(&n[15..23], &258u64.to_be_bytes());
        assert_eq!(n[23], 1);
    }

    #[test]
    fn wrong_key_fails() {
        let payload = b"hello neoblast".to_vec();
        let h = header_for(payload.len(), 64);
        let segs = seal_segments(&[1; 32], &h, &payload).unwrap();
        assert_eq!(
            open_segments(&[2; 32], &h, &segs),
            Err(CryptoError::Auth(0))
        );
    }

    #[test]
    fn swapped_segments_fail() {
        let payload = vec![0xAB; 1000];
        let h = header_for(payload.len(), 64);
        let mut segs = seal_segments(&[1; 32], &h, &payload).unwrap();
        assert!(segs.len() > 2);
        segs.swap(0, 1);
        assert!(open_segments(&[1; 32], &h, &segs).is_err());
    }

    #[test]
    fn header_is_bound_as_associated_data() {
        let payload = vec![7u8; 500];
        let h = header_for(payload.len(), 64);
        let segs = seal_segments(&[1; 32], &h, &payload).unwrap();
        // Same layout, different k/n: authentication must fail.
        let other = Header::new(payload.len() as u64, 5, 9, 64).unwrap();
        assert!(open_segments(&[1; 32], &other, &segs).is_err());
    }

    proptest! {
        #[test]
        fn round_trip(payload in proptest::collection::vec(any::<u8>(), 0..5000), seg in 64u32..2048) {
            let h = header_for(payload.len(), seg);
            let segs = seal_segments(&[9; 32], &h, &payload).unwrap();
            prop_assert_eq!(segs.len(), h.segment_count() as usize);
            prop_assert_eq!(open_segments(&[9; 32], &h, &segs).unwrap(), payload);
        }

        #[test]
        fn any_bit_flip_is_rejected(payload in proptest::collection::vec(any::<u8>(), 1..2000), pos in any::<usize>(), bit in 0u8..8) {
            let h = header_for(payload.len(), 256);
            let mut segs = seal_segments(&[3; 32], &h, &payload).unwrap();
            let s = pos % segs.len();
            let b = (pos / segs.len()) % segs[s].len();
            segs[s][b] ^= 1 << bit;
            prop_assert!(open_segments(&[3; 32], &h, &segs).is_err());
        }
    }
}
