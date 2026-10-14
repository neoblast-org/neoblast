//! Seal, lose shards, heal, open.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::MemStore;
use neoblast::{Error, FileReadCap, Params, heal, open, seal, seal_with_key};
use proptest::prelude::*;

const SMALL: Params = Params {
    k: 4,
    n: 7,
    segment_size: 256,
};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Drop any n - k shards in every segment: open still returns the exact payload,
    /// and heal (verify cap only) restores byte-identical shards.
    #[test]
    fn survives_any_n_minus_k_losses(
        payload in proptest::collection::vec(any::<u8>(), 0..3000),
        seed in any::<u64>(),
    ) {
        let sealed = seal(&payload, SMALL).unwrap();
        let manifest = sealed.manifest.encode();
        let original = MemStore::from_sealed(&sealed);
        let mut store = original.clone();
        let n = u64::from(SMALL.n);
        let drop = SMALL.n - SMALL.k;
        for segment in 0..sealed.manifest.header().segment_count() {
            let mut removed = 0;
            let mut s = seed ^ u64::from(segment);
            while removed < drop {
                s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                let i = u16::try_from((s >> 33) % n).unwrap();
                if store.0.remove(&(segment, i)).is_some() {
                    removed += 1;
                }
            }
        }
        prop_assert_eq!(open(&sealed.cap, &manifest, &store).unwrap(), payload);

        let rebuilt = heal(&sealed.cap.verify_cap(), &manifest, &store).unwrap();
        for r in rebuilt {
            prop_assert_eq!(Some(&r.bytes), original.0.get(&(r.segment, r.index)));
            store.0.insert((r.segment, r.index), r.bytes);
        }
        prop_assert_eq!(store.0.len(), original.0.len());
    }
}

#[test]
fn one_shard_too_few_is_a_clean_error() {
    let sealed = seal(b"not enough shards", SMALL).unwrap();
    let mut store = MemStore::from_sealed(&sealed);
    for i in 0..=(SMALL.n - SMALL.k) {
        store.0.remove(&(0, i));
    }
    let err = open(&sealed.cap, &sealed.manifest.encode(), &store).unwrap_err();
    assert!(matches!(
        err,
        Error::Unrecoverable {
            segment: 0,
            have: 3,
            need: 4
        }
    ));
}

#[test]
fn default_params_round_trip() {
    let payload = vec![0x5Au8; 300_000];
    let sealed = seal(&payload, Params::default()).unwrap();
    assert_eq!(sealed.shards[0].len(), 50);
    let store = MemStore::from_sealed(&sealed);
    assert_eq!(
        open(&sealed.cap, &sealed.manifest.encode(), &store).unwrap(),
        payload
    );
}

#[test]
fn link_round_trips_through_text() {
    let sealed = seal_with_key(b"hello", SMALL, &[42; 32]).unwrap();
    let link = sealed.cap.to_link();
    let parsed = FileReadCap::parse(&link).unwrap();
    let store = MemStore::from_sealed(&sealed);
    assert_eq!(
        open(&parsed, &sealed.manifest.encode(), &store).unwrap(),
        b"hello"
    );
}

#[test]
fn wrong_root_is_rejected() {
    let a = seal(b"file a", SMALL).unwrap();
    let b = seal(b"file b", SMALL).unwrap();
    let store = MemStore::from_sealed(&a);
    let err = open(&b.cap, &a.manifest.encode(), &store).unwrap_err();
    assert!(matches!(err, Error::RootMismatch));
}
