//! Erasure coding for Neoblast segments.
//!
//! `reed-solomon-simd` has a small blast radius (8 reverse dependencies on
//! crates.io, Oct 2026) and its parity bytes become part of the on-disk format.
//! It therefore sits behind the [`ErasureCode`] trait, is pinned to an exact
//! version, and its output is locked by the golden vectors in `spec/test-vectors`.
//! Whether to vendor it is open item 7 in `ROADMAP.md`.
//!
//! All functions take k, n and shard length from a validated header, never from
//! a peer (code-review issue 1). Callers pass only shards they have already
//! checked against the manifest's hashes (issue 2).
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use std::collections::BTreeMap;

use thiserror::Error;

/// Errors from encoding or reconstruction.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ErasureError {
    /// k, n or the shard length are unusable.
    #[error("invalid parameters")]
    Params,
    /// Fewer than k shards are available.
    #[error("need {need} shards, have {have}")]
    NotEnough {
        /// Shards required (k).
        need: usize,
        /// Shards available.
        have: usize,
    },
    /// A shard has the wrong length.
    #[error("shard {0} has the wrong length")]
    ShardLength(usize),
    /// The underlying codec failed.
    #[error("codec error: {0}")]
    Codec(String),
}

/// A k-of-n erasure code over equal-length shards.
pub trait ErasureCode {
    /// Computes `recovery_count` parity shards from the original data shards.
    fn encode(
        &self,
        original: &[&[u8]],
        recovery_count: usize,
    ) -> Result<Vec<Vec<u8>>, ErasureError>;

    /// Restores missing original shards. Returns only the restored ones, keyed by index.
    fn restore(
        &self,
        original_count: usize,
        recovery_count: usize,
        original: &[(usize, &[u8])],
        recovery: &[(usize, &[u8])],
    ) -> Result<BTreeMap<usize, Vec<u8>>, ErasureError>;
}

/// Reed-Solomon over GF(2^16), provided by `reed-solomon-simd` 3.1.0.
#[derive(Clone, Copy, Debug, Default)]
pub struct ReedSolomonSimd;

impl ErasureCode for ReedSolomonSimd {
    fn encode(
        &self,
        original: &[&[u8]],
        recovery_count: usize,
    ) -> Result<Vec<Vec<u8>>, ErasureError> {
        reed_solomon_simd::encode(original.len(), recovery_count, original.iter())
            .map_err(|e| ErasureError::Codec(e.to_string()))
    }

    fn restore(
        &self,
        original_count: usize,
        recovery_count: usize,
        original: &[(usize, &[u8])],
        recovery: &[(usize, &[u8])],
    ) -> Result<BTreeMap<usize, Vec<u8>>, ErasureError> {
        if original.len() == original_count {
            return Ok(BTreeMap::new());
        }
        let restored = reed_solomon_simd::decode(
            original_count,
            recovery_count,
            original.iter().copied(),
            recovery.iter().copied(),
        )
        .map_err(|e| ErasureError::Codec(e.to_string()))?;
        Ok(restored.into_iter().collect())
    }
}

/// Splits one segment's ciphertext into `n` shards of `shard_len` bytes:
/// `k` zero-padded data shards followed by `n - k` parity shards.
pub fn encode_segment(
    code: &impl ErasureCode,
    ciphertext: &[u8],
    k: usize,
    n: usize,
    shard_len: usize,
) -> Result<Vec<Vec<u8>>, ErasureError> {
    check(k, n, shard_len)?;
    let capacity = k.checked_mul(shard_len).ok_or(ErasureError::Params)?;
    if ciphertext.len() > capacity {
        return Err(ErasureError::Params);
    }
    let mut padded = ciphertext.to_vec();
    padded.resize(capacity, 0);
    let data: Vec<&[u8]> = padded.chunks_exact(shard_len).collect();
    let parity = code.encode(&data, n.saturating_sub(k))?;
    let mut shards: Vec<Vec<u8>> = data.into_iter().map(<[u8]>::to_vec).collect();
    shards.extend(parity);
    Ok(shards)
}

/// Rebuilds all `n` shards from any `k` verified ones.
///
/// `shards[i]` is `Some` for each shard the caller holds and has verified.
pub fn regenerate(
    code: &impl ErasureCode,
    k: usize,
    n: usize,
    shard_len: usize,
    shards: &[Option<Vec<u8>>],
) -> Result<Vec<Vec<u8>>, ErasureError> {
    let data = reconstruct_data(code, k, n, shard_len, shards)?;
    let refs: Vec<&[u8]> = data.iter().map(Vec::as_slice).collect();
    let parity = code.encode(&refs, n.saturating_sub(k))?;
    let mut all = data;
    all.extend(parity);
    Ok(all)
}

/// Returns the `k` data shards, restoring any that are missing.
pub fn reconstruct_data(
    code: &impl ErasureCode,
    k: usize,
    n: usize,
    shard_len: usize,
    shards: &[Option<Vec<u8>>],
) -> Result<Vec<Vec<u8>>, ErasureError> {
    check(k, n, shard_len)?;
    if shards.len() != n {
        return Err(ErasureError::Params);
    }
    let mut original = Vec::new();
    let mut recovery = Vec::new();
    for (i, shard) in shards.iter().enumerate() {
        if let Some(bytes) = shard {
            if bytes.len() != shard_len {
                return Err(ErasureError::ShardLength(i));
            }
            if i < k {
                original.push((i, bytes.as_slice()));
            } else {
                recovery.push((i.saturating_sub(k), bytes.as_slice()));
            }
        }
    }
    let have = original.len().saturating_add(recovery.len());
    if have < k {
        return Err(ErasureError::NotEnough { need: k, have });
    }
    let mut restored = code.restore(k, n.saturating_sub(k), &original, &recovery)?;
    (0..k)
        .map(|i| match shards.get(i) {
            Some(Some(bytes)) => Ok(bytes.clone()),
            _ => restored
                .remove(&i)
                .ok_or(ErasureError::NotEnough { need: k, have }),
        })
        .collect()
}

fn check(k: usize, n: usize, shard_len: usize) -> Result<(), ErasureError> {
    if k == 0 || n <= k || shard_len == 0 || shard_len % 2 != 0 {
        return Err(ErasureError::Params);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]
        #[test]
        fn any_k_shards_rebuild_everything(
            data in proptest::collection::vec(any::<u8>(), 1..4000),
            k in 1usize..12,
            extra in 1usize..12,
            seed in any::<u64>(),
        ) {
            let n = k + extra;
            let shard_len = (data.len().div_ceil(k) + 1) & !1;
            let code = ReedSolomonSimd;
            let shards = encode_segment(&code, &data, k, n, shard_len).unwrap();
            // Drop n - k shards chosen by the seed.
            let mut order: Vec<usize> = (0..n).collect();
            let mut s = seed;
            for i in (1..n).rev() {
                s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                order.swap(i, (s >> 33) as usize % (i + 1));
            }
            let mut held: Vec<Option<Vec<u8>>> = shards.iter().cloned().map(Some).collect();
            for &i in order.iter().take(n - k) {
                held[i] = None;
            }
            let rebuilt = regenerate(&code, k, n, shard_len, &held).unwrap();
            prop_assert_eq!(rebuilt, shards);
        }
    }

    #[test]
    fn too_few_shards_is_an_error_not_a_panic() {
        let code = ReedSolomonSimd;
        let shards = encode_segment(&code, b"abcdefgh", 2, 4, 4).unwrap();
        let held = vec![Some(shards[0].clone()), None, None, None];
        assert_eq!(
            reconstruct_data(&code, 2, 4, 4, &held),
            Err(ErasureError::NotEnough { need: 2, have: 1 })
        );
    }

    #[test]
    fn odd_shard_length_rejected() {
        assert_eq!(
            encode_segment(&ReedSolomonSimd, b"abc", 2, 4, 3),
            Err(ErasureError::Params)
        );
    }
}
