//! Regression tests for the manifest's copies, counted exactly with dhat.
//! One test, because dhat allows one profiler per process.
#![allow(clippy::unwrap_used)]

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

use neoblast::{Manifest, Params, seal_with_key};

/// Allocations allowed besides the manifest: the root's tree builders.
const SMALL: usize = 64 * 1024;

/// S-005 and #16: `decode_owned` copies nothing, and `decode` rejects a
/// manifest that does not match before copying it. Accepting one with
/// `decode` does copy it, which shows the count sees copies.
#[test]
fn the_manifest_is_copied_only_by_an_accepting_decode() {
    let params = Params {
        k: 2,
        n: 3,
        segment_size: 64,
    };
    let sealed = seal_with_key(&[5u8; 600_000], params, &[1u8; 32]).unwrap();
    let bytes = sealed.manifest.encode();
    let len = bytes.len();
    assert!(len > 8 * SMALL, "manifest too small to tell: {len} bytes");
    let owned = bytes.clone();
    let root = sealed.cap.root;

    let _profiler = dhat::Profiler::builder().testing().build();
    let kept = Manifest::decode_owned(owned, &root).unwrap();
    let stats = dhat::HeapStats::get();
    dhat::assert!(stats.max_bytes < SMALL, "decode_owned: {stats:?}");
    drop(kept);

    // Called outside `dhat::assert!`, which need not evaluate its argument.
    let rejected = Manifest::decode(&bytes, &[0u8; 32]);
    let stats = dhat::HeapStats::get();
    dhat::assert!(rejected.is_err(), "decode accepted the wrong root");
    dhat::assert!(stats.max_bytes < SMALL, "rejecting decode: {stats:?}");

    let copy = Manifest::decode(&bytes, &root).unwrap();
    let stats = dhat::HeapStats::get();
    dhat::assert!(stats.max_bytes >= len, "accepting decode: {stats:?}");
    drop(copy);
}
