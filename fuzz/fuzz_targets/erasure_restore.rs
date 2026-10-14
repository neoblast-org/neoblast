//! Erasure decoding with any subset of shards and any shard lengths.
//!
//! Part 1 encodes real data, drops and damages shards as the input says, then
//! checks the result: enough correct-length shards must restore the exact data,
//! and anything else must be an error, never a panic or a wrong answer.
//! Part 2 calls the decoder with arbitrary parameters, which must never panic.
#![no_main]

use libfuzzer_sys::fuzz_target;
use neoblast_erasure::{ReedSolomonSimd, encode_segment, reconstruct_data, regenerate};

struct Input<'a>(&'a [u8]);

impl Input<'_> {
    fn byte(&mut self) -> u8 {
        match self.0.split_first() {
            Some((b, rest)) => {
                self.0 = rest;
                *b
            }
            None => 0,
        }
    }
}

fuzz_target!(|data: &[u8]| {
    let mut input = Input(data);

    // Part 1: real encodings, damaged.
    // Up to the format's limits (k <= 64, n <= 128), so 20-of-50 is covered.
    let k = 2 + usize::from(input.byte() % 63); // 2..=64
    let m = 1 + usize::from(input.byte()) % (128 - k); // n = k + m <= 128
    let n = k + m;
    let shard_len = 64 * (1 + usize::from(input.byte() % 4));
    let ct_len = usize::from(input.byte()) * k * shard_len / 255;
    let ciphertext: Vec<u8> = (0..ct_len).map(|_| input.byte()).collect();

    let shards = encode_segment(&ReedSolomonSimd, &ciphertext, k, n, shard_len)
        .expect("valid parameters must encode");
    assert_eq!(shards.len(), n);

    let mut held = Vec::with_capacity(n);
    let mut present = 0;
    let mut wrong_length = false;
    for shard in &shards {
        match input.byte() % 4 {
            0 => held.push(None),
            3 => {
                let mut bad = shard.clone();
                if input.byte() % 2 == 0 {
                    bad.truncate(usize::from(input.byte()) % shard_len);
                } else {
                    bad.extend(std::iter::repeat_n(0, 1 + usize::from(input.byte())));
                }
                wrong_length = true;
                held.push(Some(bad));
            }
            _ => {
                present += 1;
                held.push(Some(shard.clone()));
            }
        }
    }

    let result = reconstruct_data(&ReedSolomonSimd, k, n, shard_len, &held);
    if wrong_length {
        // Contract: any present shard of the wrong length is an error, even when
        // k good shards are also present (the caller has a bug or a bad header).
        assert!(result.is_err(), "a wrong-length shard was accepted");
    } else if present >= k {
        let data_shards = result.expect("k correct shards must restore");
        assert_eq!(data_shards, shards[..k], "restored data differs");
        let all = regenerate(&ReedSolomonSimd, k, n, shard_len, &held).expect("regenerate");
        assert_eq!(all, shards, "regenerated shards differ");
    } else {
        assert!(result.is_err(), "restored from fewer than k shards");
    }

    // Part 2: arbitrary parameters and shard lengths. Only "no panic" is checked.
    let k = usize::from(input.byte() % 70);
    let n = usize::from(input.byte());
    let shard_len = usize::from(input.byte()) * 8;
    let held: Vec<Option<Vec<u8>>> = (0..n)
        .map(|_| match input.byte() % 3 {
            0 => None,
            1 => Some(vec![0xa5; shard_len]),
            _ => Some(vec![0x5a; usize::from(input.byte())]),
        })
        .collect();
    let _ = reconstruct_data(&ReedSolomonSimd, k, n, shard_len, &held);
    let _ = regenerate(&ReedSolomonSimd, k, n, shard_len, &held);
});
