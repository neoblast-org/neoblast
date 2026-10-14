//! The hash tree under the file root (decision 0002).
//!
//! ```text
//! shard_hash      = BLAKE3(shard bytes)
//! shard leaf      = BLAKE3(0x00 || u16_be(index) || shard_hash)
//! inner node      = BLAKE3(0x01 || left || right)        odd node promoted unchanged
//! segment root    = tree over the n shard leaves
//! segment leaf    = BLAKE3(0x02 || u32_be(segment) || segment_root)
//! content root    = tree over the segment leaves
//! file root H     = BLAKE3.derive_key("nbl/v1/root", header || content_root)
//! ```

/// KDF context for the file root.
pub const CTX_ROOT: &str = "nbl/v1/root";

/// A 32-byte hash.
pub type Hash = [u8; 32];

/// Hash of a shard's bytes.
pub fn shard_hash(bytes: &[u8]) -> Hash {
    *blake3::hash(bytes).as_bytes()
}

fn shard_leaf(index: u16, hash: &Hash) -> Hash {
    let mut h = blake3::Hasher::new();
    h.update(&[0x00]);
    h.update(&index.to_be_bytes());
    h.update(hash);
    *h.finalize().as_bytes()
}

fn segment_leaf(segment: u32, root: &Hash) -> Hash {
    let mut h = blake3::Hasher::new();
    h.update(&[0x02]);
    h.update(&segment.to_be_bytes());
    h.update(root);
    *h.finalize().as_bytes()
}

fn node(left: &Hash, right: &Hash) -> Hash {
    let mut h = blake3::Hasher::new();
    h.update(&[0x01]);
    h.update(left);
    h.update(right);
    *h.finalize().as_bytes()
}

/// Builds a binary tree over `leaves`; an odd node is promoted unchanged
/// (no self-pairing, avoiding the CVE-2012-2459 duplicate-leaf pattern).
/// Returns `None` for an empty list.
pub fn tree_root(mut level: Vec<Hash>) -> Option<Hash> {
    while level.len() > 1 {
        level = level
            .chunks(2)
            .map(|pair| match pair {
                [l, r] => node(l, r),
                [only] => *only,
                _ => [0u8; 32],
            })
            .collect();
    }
    level.into_iter().next()
}

/// Root of one segment from its n shard hashes.
pub fn segment_root(shard_hashes: &[Hash]) -> Option<Hash> {
    tree_root(
        shard_hashes
            .iter()
            .zip(0u16..)
            .map(|(h, i)| shard_leaf(i, h))
            .collect(),
    )
}

/// Content root from every segment root.
pub fn content_root(segment_roots: &[Hash]) -> Option<Hash> {
    tree_root(
        segment_roots
            .iter()
            .zip(0u32..)
            .map(|(r, i)| segment_leaf(i, r))
            .collect(),
    )
}

/// The file root `H` over the header bytes and the content root.
pub fn file_root(header: &[u8; nbl_format::HEADER_LEN], content_root: &Hash) -> Hash {
    let mut material = [0u8; nbl_format::HEADER_LEN + 32];
    let (a, b) = material.split_at_mut(nbl_format::HEADER_LEN);
    a.copy_from_slice(header);
    b.copy_from_slice(content_root);
    blake3::derive_key(CTX_ROOT, &material)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn odd_node_is_not_self_paired() {
        let a = [1u8; 32];
        let b = [2u8; 32];
        let c = [3u8; 32];
        // With self-pairing, [a,b,c] and [a,b,c,c] would share a root.
        assert_ne!(tree_root(vec![a, b, c]), tree_root(vec![a, b, c, c]));
    }

    #[test]
    fn index_is_bound_into_leaves() {
        let h = [9u8; 32];
        assert_ne!(segment_root(&[h, [0; 32]]), segment_root(&[[0; 32], h]));
    }
}
