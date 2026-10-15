//! Differential test against Leopard-RS (decision 0006).
//!
//! `spec/test-vectors/erasure-leopard.json` holds parity produced by the original
//! Leopard-RS C library (see `tools/leopard-vectors/`). Our encoder must match it
//! byte for byte, and our decoder must rebuild the data from Leopard's parity.
//! If this fails after a dependency bump, the parity bytes on disk changed.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use neoblast_erasure::{
    ErasureCode, ReedSolomonSimd, ReedSolomonSimdReuse, encode_segment, reconstruct_data,
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixtures {
    format: String,
    #[allow(dead_code)]
    note: String,
    leopard: Leopard,
    #[allow(dead_code)]
    input: String,
    #[allow(dead_code)]
    output: String,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Leopard {
    repo: String,
    commit: String,
    builds: BTreeMap<String, String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    k: usize,
    m: usize,
    shard_bytes: usize,
    leopard_build: String,
    recovery_head: String,
    recovery_blake3: String,
}

/// The fixture input: one splitmix64 stream, low byte of each output.
fn originals(k: usize, m: usize, shard_bytes: usize) -> Vec<u8> {
    let mut state = (k as u64)
        .wrapping_mul(1_000_003)
        .wrapping_add((m as u64).wrapping_mul(1009))
        .wrapping_add(shard_bytes as u64);
    (0..k * shard_bytes)
        .map(|_| {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            (z ^ (z >> 31)) as u8
        })
        .collect()
}

fn repo_file(path: &str) -> String {
    let full = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path);
    std::fs::read_to_string(full).unwrap()
}

fn load() -> Fixtures {
    serde_json::from_str(&repo_file("spec/test-vectors/erasure-leopard.json")).unwrap()
}

#[test]
fn fixtures_match_the_case_list_and_pinned_leopard() {
    let fixtures = load();
    assert_eq!(fixtures.format, "neoblast erasure fixtures v1");
    assert_eq!(fixtures.leopard.repo, "https://github.com/catid/leopard");
    assert_eq!(
        fixtures.leopard.commit,
        "22ddc7804998d31c8f1a2617ee720e063b1fa6cd"
    );
    assert_eq!(
        fixtures.leopard.builds.keys().collect::<Vec<_>>(),
        ["ff16", "ff16-m-gt-k"]
    );

    // Exactly the cases in tools/leopard-vectors/cases.txt, in order: none
    // dropped, none added by hand.
    let listed: Vec<(usize, usize, usize)> = repo_file("tools/leopard-vectors/cases.txt")
        .lines()
        .map(|line| line.split('#').next().unwrap().trim())
        .filter(|line| !line.is_empty())
        .map(|line| {
            let v: Vec<usize> = line
                .split_whitespace()
                .map(|x| x.parse().unwrap())
                .collect();
            (v[0], v[1], v[2])
        })
        .collect();
    let fixture: Vec<_> = fixtures
        .cases
        .iter()
        .map(|c| (c.k, c.m, c.shard_bytes))
        .collect();
    assert_eq!(fixture, listed);

    for c in &fixtures.cases {
        let build = if c.m > c.k { "ff16-m-gt-k" } else { "ff16" };
        assert_eq!(c.leopard_build, build, "{}/{}", c.k, c.m);
    }

    // 20-of-50 with the real shard size of a full 4 MiB segment, and the format's
    // extremes: smallest k at the largest n, largest k at the largest n.
    for (k, m, bytes) in [(20, 30, 209_728), (2, 126, 64), (64, 64, 64)] {
        assert!(listed.contains(&(k, m, bytes)), "{k}/{m} x {bytes}");
    }
}

#[test]
fn parity_matches_leopard_byte_for_byte() {
    check_all(&ReedSolomonSimd);
}

/// One reusing codec runs every case in turn, so its working buffers are
/// resized between very different shapes; the bytes must not change.
#[test]
fn reusing_codec_matches_leopard_across_all_cases() {
    let code = ReedSolomonSimdReuse::default();
    check_all(&code);
    // And again in reverse order, starting from the largest buffers.
    let mut cases = load().cases;
    cases.reverse();
    check_cases(&code, cases);
}

fn check_all(code: &impl ErasureCode) {
    check_cases(code, load().cases);
}

fn check_cases(code: &impl ErasureCode, cases: Vec<Case>) {
    for case in cases {
        let Case {
            k, m, shard_bytes, ..
        } = case;
        let label = format!("{k}/{m} x {shard_bytes} ({})", case.leopard_build);
        let data = originals(k, m, shard_bytes);
        let shards = encode_segment(code, &data, k, k + m, shard_bytes).unwrap();

        let parity: Vec<u8> = shards[k..].concat();
        assert_eq!(parity.len(), m * shard_bytes, "{label}");
        assert_eq!(hex::encode(&parity[..16]), case.recovery_head, "{label}");
        assert_eq!(
            blake3::hash(&parity).to_hex().as_str(),
            case.recovery_blake3,
            "{label}"
        );

        // Decoding: lose the first min(m, k) data shards. When m >= k every data
        // shard is rebuilt from Leopard's parity alone.
        let lost = m.min(k);
        let held: Vec<Option<Vec<u8>>> = shards
            .into_iter()
            .enumerate()
            .map(|(i, s)| (i >= lost).then_some(s))
            .collect();
        let rebuilt = reconstruct_data(code, k, k + m, shard_bytes, &held).unwrap();
        assert_eq!(rebuilt.concat(), data, "{label}: restore");
    }
}
