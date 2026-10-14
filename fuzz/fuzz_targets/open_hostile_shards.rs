//! `open` and `heal` against a store controlled by an attacker.
//!
//! A real three-segment file is sealed once. The fuzzer then decides, shard by
//! shard, what the store returns: nothing, the true shard, the true shard with a
//! flipped byte, or arbitrary bytes of any length. Two properties must hold:
//!
//! - integrity: anything `open` returns is the original payload, and every shard
//!   `heal` rebuilds is the original shard;
//! - availability: if every segment still has k intact shards, both succeed, and
//!   `heal` rebuilds exactly the shards that are missing or wrong.
//!
//! "Intact" is decided by comparing bytes, so fuzzer bytes that happen to equal
//! the true shard never cause a false alarm.
#![no_main]

use std::collections::{BTreeSet, HashMap};
use std::sync::OnceLock;

use libfuzzer_sys::fuzz_target;
use neoblast::{Params, ShardSource, heal, open, seal_with_key};

const PARAMS: Params = Params {
    k: 4,
    n: 7,
    segment_size: 256,
};

struct Fixture {
    payload: Vec<u8>,
    link: String,
    manifest: Vec<u8>,
    shards: Vec<Vec<Vec<u8>>>,
}

fn fixture() -> &'static Fixture {
    static FIXTURE: OnceLock<Fixture> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let payload: Vec<u8> = (0..700u32).map(|i| (i % 251) as u8).collect();
        let sealed = seal_with_key(&payload, PARAMS, &[7u8; 32]).expect("seal");
        Fixture {
            payload,
            link: sealed.cap.to_link(),
            manifest: sealed.manifest.encode(),
            shards: sealed.shards,
        }
    })
}

struct Input<'a>(&'a [u8]);

impl Input<'_> {
    fn byte(&mut self) -> u8 {
        match self.0.split_first() {
            Some((b, rest)) => {
                self.0 = rest;
                *b
            }
            // Out of input: serve the true shard.
            None => 1,
        }
    }
}

struct Store(HashMap<(u32, u16), Vec<u8>>);

impl ShardSource for Store {
    fn get(&self, segment: u32, index: u16, _len: usize) -> Option<Vec<u8>> {
        self.0.get(&(segment, index)).cloned()
    }
}

fuzz_target!(|data: &[u8]| {
    let f = fixture();
    let mut input = Input(data);
    let mut store: HashMap<(u32, u16), Vec<u8>> = HashMap::new();
    let mut every_segment_recoverable = true;
    let mut needs_rebuild = BTreeSet::new();

    for (segment, shards) in (0u32..).zip(&f.shards) {
        let mut intact = 0;
        for (index, shard) in (0u16..).zip(shards) {
            match input.byte() % 4 {
                0 => {} // missing
                1 => {
                    store.insert((segment, index), shard.clone());
                }
                2 => {
                    let mut bad = shard.clone();
                    let at = usize::from(input.byte()) % bad.len();
                    bad[at] ^= input.byte() | 1;
                    store.insert((segment, index), bad);
                }
                _ => {
                    let len = usize::from(input.byte()) * 2;
                    let bytes = (0..len).map(|_| input.byte()).collect();
                    store.insert((segment, index), bytes);
                }
            }
            if store.get(&(segment, index)) == Some(shard) {
                intact += 1;
            } else {
                needs_rebuild.insert((segment, index));
            }
        }
        if intact < usize::from(PARAMS.k) {
            every_segment_recoverable = false;
        }
    }
    let store = Store(store);

    let cap = neoblast::FileReadCap::parse(&f.link).expect("link");
    match open(&cap, &f.manifest, &store) {
        Ok(payload) => assert_eq!(payload, f.payload, "open returned wrong bytes"),
        Err(e) => assert!(!every_segment_recoverable, "open failed with k intact shards: {e}"),
    }

    match heal(&cap.verify_cap(), &f.manifest, &store) {
        Ok(rebuilt) => {
            let mut done = BTreeSet::new();
            for shard in rebuilt {
                let original = f
                    .shards
                    .get(shard.segment as usize)
                    .and_then(|s| s.get(usize::from(shard.index)))
                    .expect("heal returned a shard index outside the file");
                assert_eq!(&shard.bytes, original, "heal rebuilt a wrong shard");
                assert!(done.insert((shard.segment, shard.index)), "heal returned a shard twice");
            }
            if every_segment_recoverable {
                assert_eq!(done, needs_rebuild, "heal did not rebuild exactly the bad shards");
            }
        }
        Err(e) => assert!(!every_segment_recoverable, "heal failed with k intact shards: {e}"),
    }
});
