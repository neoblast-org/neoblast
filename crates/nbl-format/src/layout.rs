//! Length and layout rules shared by every implementation.

/// Poly1305 tag appended to each encrypted segment.
pub const TAG_LEN: u64 = 16;

/// Shard lengths are multiples of this many bytes (decision 0006). Leopard-RS
/// requires it, and `reed-solomon-simd` keeps its byte output stable across
/// versions only for multiples of 64.
pub const SHARD_ALIGN: u64 = 64;

/// Padmé padding (Nikitin et al., "Reducing Metadata Leakage from Encrypted
/// Files and Communication with PURBs", PETS 2019).
///
/// Rounds `len` up so that at most O(log log len) bits of the length leak,
/// with at most ~12% overhead. Idempotent: `padme(padme(x)) == padme(x)`.
pub fn padme(len: u64) -> u64 {
    if len < 2 {
        return len;
    }
    let e = u64::from(len.ilog2());
    let s = u64::from(e.ilog2()).saturating_add(1);
    let last_bits = e.saturating_sub(s);
    let mask = 1u64
        .checked_shl(u32::try_from(last_bits).unwrap_or(u32::MAX))
        .map_or(u64::MAX, |v| v.saturating_sub(1));
    len.checked_add(mask).map_or(len, |v| v & !mask)
}

/// Number of segments for `padded_length` bytes, or `None` if the segment size
/// is zero or the count does not fit in a `u32`.
pub fn segment_count(padded_length: u64, segment_size: u32) -> Option<u32> {
    if segment_size == 0 || padded_length == 0 {
        return None;
    }
    u32::try_from(padded_length.div_ceil(u64::from(segment_size))).ok()
}

/// Shard length for a segment holding `plain_len` plaintext bytes split into
/// `k` data shards: ciphertext (plaintext + tag) divided by k, rounded up to a
/// multiple of [`SHARD_ALIGN`] (64) bytes (decision 0006).
pub fn shard_len(plain_len: u64, k: u16) -> Option<u32> {
    if k == 0 {
        return None;
    }
    let cipher = plain_len.checked_add(TAG_LEN)?;
    let per = cipher.div_ceil(u64::from(k));
    let aligned = per.div_ceil(SHARD_ALIGN).checked_mul(SHARD_ALIGN)?;
    u32::try_from(aligned).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn padme_known_values() {
        assert_eq!(padme(0), 0);
        assert_eq!(padme(8), 8);
        assert_eq!(padme(9), 10);
        assert_eq!(padme(1000), 1024);
        assert_eq!(padme(1_000_000), 1_015_808);
    }

    #[test]
    fn shard_len_known_values() {
        // 1000 + 16 = 1016 bytes over 4 shards = 254, rounded up to 256.
        assert_eq!(shard_len(1000, 4), Some(256));
        // Even tiny segments get a full 64-byte shard.
        assert_eq!(shard_len(0, 20), Some(64));
        assert_eq!(shard_len(4 * 1024 * 1024, 20), Some(209_728));
        assert_eq!(shard_len(10, 0), None);
    }

    proptest! {
        #[test]
        fn shard_len_is_aligned_and_sufficient(plain in 0u64..(1 << 36), k in 1u16..=128) {
            let exact = (plain + TAG_LEN).div_ceil(u64::from(k)).div_ceil(SHARD_ALIGN) * SHARD_ALIGN;
            match shard_len(plain, k) {
                Some(s) => {
                    let s = u64::from(s);
                    prop_assert_eq!(s, exact);
                    prop_assert_eq!(s % SHARD_ALIGN, 0);
                    prop_assert!(s * u64::from(k) >= plain + TAG_LEN);
                    prop_assert!((s - SHARD_ALIGN) * u64::from(k) < plain + TAG_LEN);
                }
                // Only refused when the shard would not fit the header's u32 field.
                None => prop_assert!(exact > u64::from(u32::MAX)),
            }
        }

        #[test]
        fn padme_is_idempotent_and_bounded(len in 0u64..(1 << 50)) {
            let p = padme(len);
            prop_assert!(p >= len);
            prop_assert_eq!(padme(p), p);
            // Overhead never exceeds 12.5% for lengths above 256 bytes.
            if len > 256 {
                prop_assert!((p - len) * 8 <= len);
            }
        }
    }
}
