//! Streaming seal, open and heal (open item 2's prerequisite).
//!
//! The reference below seals the whole payload at once through
//! `neoblast_crypto::seal_segments`, which frames and segments the payload with
//! its own code. It shares the per-segment cipher and the erasure code with the
//! streaming path; the golden vectors pin those. The streaming path must produce
//! byte-identical manifests and shards for any length, parameters, and way the
//! input arrives.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::cell::Cell;
use std::io::{self, Read, Write};

use common::MemStore;
use neoblast::{
    Error, Header, Manifest, Params, ShardSink, ShardSource, heal, heal_stream, open_stream,
    seal_stream, seal_stream_with_key, seal_with_key,
};
use neoblast_erasure::ReedSolomonSimd;
use proptest::prelude::*;

const KEY: [u8; 32] = [9u8; 32];

/// Whole-file sealing, kept independent of `neoblast::stream`.
fn reference_seal(payload: &[u8], params: Params) -> (Vec<u8>, Vec<Vec<Vec<u8>>>) {
    let header = Header::new(
        payload.len() as u64,
        params.k,
        params.n,
        params.segment_size,
    )
    .unwrap();
    let ciphertexts = neoblast_crypto::seal_segments(&KEY, &header, payload).unwrap();
    let (k, n) = (usize::from(params.k), usize::from(params.n));
    let mut shards = Vec::new();
    let mut hashes = Vec::new();
    for (ct, i) in ciphertexts.iter().zip(0u32..) {
        let shard_len = header.segment_shard_len(i).unwrap() as usize;
        let segment =
            neoblast_erasure::encode_segment(&ReedSolomonSimd, ct, k, n, shard_len).unwrap();
        hashes.push(
            segment
                .iter()
                .map(|s| neoblast::tree::shard_hash(s))
                .collect(),
        );
        shards.push(segment);
    }
    (Manifest::new(header, hashes).unwrap().encode(), shards)
}

/// Collects shards from a streaming seal, checking the call order.
#[derive(Default)]
struct Collect(Vec<Vec<Vec<u8>>>);

impl ShardSink for Collect {
    fn put(&mut self, segment: u32, index: u16, bytes: &[u8]) -> io::Result<()> {
        if segment as usize == self.0.len() {
            self.0.push(Vec::new());
        }
        assert_eq!(segment as usize + 1, self.0.len(), "segments out of order");
        let shards = self.0.last_mut().unwrap();
        assert_eq!(usize::from(index), shards.len(), "shards out of order");
        shards.push(bytes.to_vec());
        Ok(())
    }
}

/// Hands out its data in uneven chunks and is sometimes interrupted.
struct Chunky<'a> {
    data: &'a [u8],
    step: usize,
    calls: usize,
}

impl<'a> Chunky<'a> {
    fn new(data: &'a [u8], step: usize) -> Self {
        Self {
            data,
            step,
            calls: 0,
        }
    }
}

impl Read for Chunky<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.calls += 1;
        if self.calls % 5 == 0 {
            return Err(io::ErrorKind::Interrupted.into());
        }
        let len = buf
            .len()
            .min(self.data.len())
            .min(1 + self.calls * self.step % 97);
        let (now, rest) = self.data.split_at(len);
        buf[..len].copy_from_slice(now);
        self.data = rest;
        Ok(len)
    }
}

/// Accepts at most a few bytes per write call.
struct SmallWrites(Vec<u8>);

impl Write for SmallWrites {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = buf.len().min(13);
        self.0.extend_from_slice(&buf[..len]);
        Ok(len)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn payload(len: usize, salt: u8) -> Vec<u8> {
    (0..len)
        .map(|i| (i as u8).wrapping_mul(31) ^ salt)
        .collect()
}

fn store_of(shards: &[Vec<Vec<u8>>]) -> MemStore {
    let mut store = MemStore::default();
    for (s, segment) in (0u32..).zip(shards) {
        for (i, bytes) in (0u16..).zip(segment) {
            store.0.insert((s, i), bytes.clone());
        }
    }
    store
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn streamed_seal_matches_whole_file_sealing(
        len in 0usize..3000,
        k in 2u16..6,
        extra in 1u16..5,
        segment_size in prop::sample::select(vec![64u32, 100, 128, 256, 1000]),
        step in 1usize..50,
        salt in any::<u8>(),
    ) {
        let params = Params { k, n: k + extra, segment_size };
        let data = payload(len, salt);
        let mut sink = Collect::default();
        let (cap, manifest) =
            seal_stream_with_key(Chunky::new(&data, step), len as u64, params, &KEY, &mut sink).unwrap();
        let (want_manifest, want_shards) = reference_seal(&data, params);
        prop_assert_eq!(manifest.encode(), want_manifest);
        prop_assert_eq!(&sink.0, &want_shards);

        // And it opens again through a writer that takes a few bytes at a time.
        let mut out = SmallWrites(Vec::new());
        let n = open_stream(&cap, &manifest.encode(), &store_of(&sink.0), &mut out).unwrap();
        prop_assert_eq!(n, len as u64);
        prop_assert_eq!(out.0, data);
    }
}

/// Every length from empty to several segments, so the length prefix, the
/// payload and the padding each cross segment boundaries in every way.
#[test]
fn every_length_across_segment_boundaries() {
    let params = Params {
        k: 2,
        n: 3,
        segment_size: 64,
    };
    for len in 0..=700 {
        let data = payload(len, 7);
        let mut sink = Collect::default();
        let (cap, manifest) =
            seal_stream_with_key(&data[..], len as u64, params, &KEY, &mut sink).unwrap();
        let (want_manifest, want_shards) = reference_seal(&data, params);
        assert_eq!(manifest.encode(), want_manifest, "length {len}");
        assert_eq!(sink.0, want_shards, "length {len}");
        let mut out = Vec::new();
        open_stream(&cap, &manifest.encode(), &store_of(&sink.0), &mut out).unwrap();
        assert_eq!(out, data, "length {len}");
    }
}

#[test]
fn input_shorter_or_longer_than_declared_is_rejected() {
    let params = Params {
        k: 4,
        n: 6,
        segment_size: 128,
    };
    let data = payload(500, 1);
    for declared in [499u64, 501, 0, 1000] {
        let err = seal_stream_with_key(&data[..], declared, params, &KEY, &mut Collect::default())
            .unwrap_err();
        assert!(
            matches!(err, Error::InputLength { expected } if expected == declared),
            "declared {declared}: {err:?}"
        );
    }
}

#[test]
fn read_errors_and_sink_errors_are_reported() {
    struct Failing;
    impl Read for Failing {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("disk gone"))
        }
    }
    struct Full;
    impl ShardSink for Full {
        fn put(&mut self, _: u32, _: u16, _: &[u8]) -> io::Result<()> {
            Err(io::Error::other("disk full"))
        }
    }
    let params = Params {
        k: 2,
        n: 3,
        segment_size: 64,
    };
    let err = seal_stream_with_key(Failing, 10, params, &KEY, &mut Collect::default()).unwrap_err();
    assert!(matches!(err, Error::Io(_)), "{err:?}");
    let err = seal_stream_with_key(&[1u8; 10][..], 10, params, &KEY, &mut Full).unwrap_err();
    assert!(matches!(err, Error::Io(_)), "{err:?}");
}

#[test]
fn random_keys_differ_between_seals() {
    let params = Params {
        k: 2,
        n: 3,
        segment_size: 64,
    };
    let (a, _) = seal_stream(&b"same"[..], 4, params, &mut Collect::default()).unwrap();
    let (b, _) = seal_stream(&b"same"[..], 4, params, &mut Collect::default()).unwrap();
    assert_ne!(a.read_key, b.read_key);
    assert_ne!(a.root, b.root);
}

/// A failed open has written only verified bytes: a prefix of the payload.
#[test]
fn open_stream_stops_at_an_unrecoverable_segment() {
    let params = Params {
        k: 2,
        n: 4,
        segment_size: 64,
    };
    let data = payload(300, 3);
    let sealed = seal_with_key(&data, params, &KEY).unwrap();
    let last = sealed.shards.len() as u32 - 1;
    let mut store = MemStore::from_sealed(&sealed);
    store.0.retain(|&(s, i), _| s != last || i == 0); // one shard left in the last segment
    let mut out = Vec::new();
    let err = open_stream(&sealed.cap, &sealed.manifest.encode(), &store, &mut out).unwrap_err();
    assert!(
        matches!(err, Error::Unrecoverable { segment, have: 1, need: 2 } if segment == last),
        "{err:?}"
    );
    assert!(data.starts_with(&out) && out.len() < data.len());
}

/// An intact file is opened from exactly k shards per segment: no parity reads.
#[test]
fn open_stream_reads_only_k_shards_when_intact() {
    struct Counting(MemStore, Cell<usize>);
    impl ShardSource for Counting {
        fn get(&self, segment: u32, index: u16, len: usize) -> Option<Vec<u8>> {
            self.1.set(self.1.get() + 1);
            self.0.get(segment, index, len)
        }
    }
    let params = Params {
        k: 3,
        n: 8,
        segment_size: 128,
    };
    let data = payload(1000, 5);
    let sealed = seal_with_key(&data, params, &KEY).unwrap();
    let source = Counting(MemStore::from_sealed(&sealed), Cell::new(0));
    let mut out = Vec::new();
    open_stream(&sealed.cap, &sealed.manifest.encode(), &source, &mut out).unwrap();
    assert_eq!(out, data);
    assert_eq!(source.1.get(), 3 * sealed.shards.len());
}

/// Gathers `(segment, index, bytes)` from a heal.
#[derive(Default)]
struct Gather(Vec<(u32, u16, Vec<u8>)>);

impl ShardSink for Gather {
    fn put(&mut self, s: u32, i: u16, b: &[u8]) -> io::Result<()> {
        self.0.push((s, i, b.to_vec()));
        Ok(())
    }
}

/// Heal rebuilds exactly the shards whose stored bytes are missing or wrong:
/// never a good one (even one after the first k), never skipping a bad one.
#[test]
fn heal_stream_rebuilds_exactly_the_bad_shards() {
    let params = Params {
        k: 3,
        n: 7,
        segment_size: 128,
    };
    let data = payload(900, 9);
    let sealed = seal_with_key(&data, params, &KEY).unwrap();
    assert!(sealed.shards.len() >= 3);
    let mut store = MemStore::from_sealed(&sealed);
    // Segment 0: first k good, a good one after them, then corrupt, missing,
    // and wrong-length shards.
    store.0.get_mut(&(0, 4)).unwrap()[0] ^= 1;
    store.0.remove(&(0, 5));
    store.0.get_mut(&(0, 6)).unwrap().push(0);
    // Segment 1: a corrupt data shard and a missing one.
    store.0.get_mut(&(1, 0)).unwrap()[3] ^= 0x80;
    store.0.remove(&(1, 1));
    // Segment 2 onward: untouched.

    let mut expected = Vec::new();
    for (s, shards) in (0u32..).zip(&sealed.shards) {
        for (i, original) in (0u16..).zip(shards) {
            if store.0.get(&(s, i)) != Some(original) {
                expected.push((s, i, original.clone()));
            }
        }
    }
    assert_eq!(expected.len(), 5);

    let mut got = Gather::default();
    let count = heal_stream(
        &sealed.cap.verify_cap(),
        &sealed.manifest.encode(),
        &store,
        &mut got,
    )
    .unwrap();
    assert_eq!(count, 5);
    assert_eq!(got.0, expected);
    // The in-memory wrapper agrees.
    let wrapped = heal(&sealed.cap.verify_cap(), &sealed.manifest.encode(), &store).unwrap();
    let wrapped: Vec<_> = wrapped
        .into_iter()
        .map(|r| (r.segment, r.index, r.bytes))
        .collect();
    assert_eq!(wrapped, expected);
}

/// A segment that can't be rebuilt doesn't stop the others from being repaired.
#[test]
fn heal_stream_repairs_other_segments_then_reports_the_failure() {
    let params = Params {
        k: 2,
        n: 4,
        segment_size: 64,
    };
    let sealed = seal_with_key(&payload(200, 4), params, &KEY).unwrap();
    let mut store = MemStore::from_sealed(&sealed);
    store.0.retain(|&(s, i), _| !(s == 0 && i > 0)); // segment 0: one shard left
    store.0.remove(&(1, 3)); // segment 1: repairable
    let mut got = Gather::default();
    let err = heal_stream(
        &sealed.cap.verify_cap(),
        &sealed.manifest.encode(),
        &store,
        &mut got,
    )
    .unwrap_err();
    assert!(
        matches!(
            err,
            Error::Unrecoverable {
                segment: 0,
                have: 1,
                need: 2
            }
        ),
        "{err:?}"
    );
    assert_eq!(got.0, vec![(1, 3, sealed.shards[1][3].clone())]);
}

/// A corrupt data shard costs exactly one extra read in its segment.
#[test]
fn open_stream_reads_one_more_shard_per_bad_data_shard() {
    struct Counting(MemStore, Cell<usize>);
    impl ShardSource for Counting {
        fn get(&self, segment: u32, index: u16, len: usize) -> Option<Vec<u8>> {
            self.1.set(self.1.get() + 1);
            self.0.get(segment, index, len)
        }
    }
    let params = Params {
        k: 3,
        n: 8,
        segment_size: 128,
    };
    let data = payload(1000, 5);
    let sealed = seal_with_key(&data, params, &KEY).unwrap();
    let mut store = MemStore::from_sealed(&sealed);
    store.0.get_mut(&(0, 1)).unwrap()[0] ^= 1;
    let source = Counting(store, Cell::new(0));
    let mut out = Vec::new();
    open_stream(&sealed.cap, &sealed.manifest.encode(), &source, &mut out).unwrap();
    assert_eq!(out, data);
    assert_eq!(source.1.get(), 3 * sealed.shards.len() + 1);
}

#[test]
fn writer_and_sink_errors_surface_as_io_errors() {
    struct Broken;
    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("pipe closed"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    struct Full;
    impl ShardSink for Full {
        fn put(&mut self, _: u32, _: u16, _: &[u8]) -> io::Result<()> {
            Err(io::Error::other("disk full"))
        }
    }
    let params = Params {
        k: 2,
        n: 3,
        segment_size: 64,
    };
    let sealed = seal_with_key(&payload(100, 2), params, &KEY).unwrap();
    let mut store = MemStore::from_sealed(&sealed);
    let err = open_stream(&sealed.cap, &sealed.manifest.encode(), &store, Broken).unwrap_err();
    assert!(matches!(err, Error::Io(_)), "{err:?}");
    store.0.remove(&(0, 2));
    let err = heal_stream(
        &sealed.cap.verify_cap(),
        &sealed.manifest.encode(),
        &store,
        &mut Full,
    )
    .unwrap_err();
    assert!(matches!(err, Error::Io(_)), "{err:?}");
}

/// A huge declared length fails on the first short read; nothing is
/// allocated from the declared size.
#[test]
fn huge_declared_length_is_an_error_not_an_abort() {
    let params = Params {
        k: 2,
        n: 3,
        segment_size: 64,
    };
    let err =
        seal_stream_with_key(&[][..], 1 << 37, params, &KEY, &mut Collect::default()).unwrap_err();
    assert!(
        matches!(err, Error::InputLength { expected } if expected == 1 << 37),
        "{err:?}"
    );
}

/// Builds a one-segment file whose framed plaintext is exactly `framed`,
/// correctly encrypted and committed, so only the framing checks can reject it.
fn craft(framed: &[u8], header: &Header) -> (neoblast::FileReadCap, Vec<u8>, MemStore) {
    let cipher = neoblast_crypto::SegmentCipher::new(&KEY, header);
    let mut ct = framed.to_vec();
    cipher.seal_in_place(0, &mut ct).unwrap();
    let shard_len = header.segment_shard_len(0).unwrap() as usize;
    let (k, n) = (usize::from(header.k()), usize::from(header.n()));
    let shards = neoblast_erasure::encode_segment(&ReedSolomonSimd, &ct, k, n, shard_len).unwrap();
    let hashes = vec![
        shards
            .iter()
            .map(|s| neoblast::tree::shard_hash(s))
            .collect(),
    ];
    let manifest = Manifest::new(*header, hashes).unwrap();
    let cap = neoblast::FileReadCap {
        root: manifest.root(),
        read_key: KEY,
    };
    (cap, manifest.encode(), store_of(&[shards]))
}

#[test]
fn non_canonical_framing_is_rejected() {
    // 11 bytes + the 8-byte prefix pad to 20, so there is a padding byte.
    let header = Header::new(11, 2, 3, 64).unwrap();
    assert_eq!(header.segment_count(), 1);
    let padded = header.padded_length() as usize;
    assert!(padded > 8 + 11);
    let framed = |len: u64, fill: u8| {
        let mut f = len.to_be_bytes().to_vec();
        f.extend(std::iter::repeat_n(fill, 11));
        f.resize(padded, 0);
        f
    };

    // The honest framing opens.
    let (cap, manifest, store) = craft(&framed(11, 0xaa), &header);
    let mut out = Vec::new();
    assert_eq!(open_stream(&cap, &manifest, &store, &mut out).unwrap(), 11);
    assert_eq!(out, vec![0xaa; 11]);

    let mut bad_padding = framed(11, 0xaa);
    *bad_padding.last_mut().unwrap() = 1;
    let cases = [
        ("non-zero padding", bad_padding),
        ("length shorter than the data", framed(5, 0xaa)),
        (
            "length past the framed size",
            framed(padded as u64 - 7, 0xaa),
        ),
        ("length of u64::MAX", framed(u64::MAX, 0xaa)),
    ];
    for (what, f) in cases {
        let (cap, manifest, store) = craft(&f, &header);
        let mut out = Vec::new();
        let err = open_stream(&cap, &manifest, &store, &mut out).unwrap_err();
        assert!(
            matches!(err, Error::Crypto(neoblast_crypto::CryptoError::Framing)),
            "{what}: {err:?}"
        );
        assert!(
            neoblast::open(&cap, &manifest, &store).is_err(),
            "{what}: in-memory open"
        );
    }
}
