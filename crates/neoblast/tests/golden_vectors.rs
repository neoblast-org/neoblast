//! Golden test vectors: lock the v1 byte format.
//!
//! Any change to these bytes is a format change. It needs a spec version bump and
//! a decision record (before the Phase 1 freeze, the record alone, as in decision
//! 0006), then regenerate with:
//!
//! ```sh
//! NBL_UPDATE_VECTORS=1 cargo test -p neoblast --test golden_vectors
//! ```
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use neoblast::{Params, seal_with_key};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Vector {
    name: String,
    /// How to rebuild the payload: `"empty"`, `"ascii:<text>"` or `"mod251:<len>"`
    /// (byte i = i mod 251).
    payload: String,
    payload_blake3: String,
    k: u16,
    n: u16,
    segment_size: u32,
    read_key: String,
    header: String,
    segment_count: u32,
    manifest_blake3: String,
    root: String,
    read_link: String,
    verify_link: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct VectorFile {
    format: String,
    note: String,
    vectors: Vec<Vector>,
}

fn payload(spec: &str) -> Vec<u8> {
    if spec == "empty" {
        Vec::new()
    } else if let Some(text) = spec.strip_prefix("ascii:") {
        text.as_bytes().to_vec()
    } else if let Some(len) = spec.strip_prefix("mod251:") {
        (0..len.parse::<u32>().unwrap())
            .map(|i| (i % 251) as u8)
            .collect()
    } else {
        panic!("unknown payload spec {spec}")
    }
}

fn key() -> [u8; 32] {
    core::array::from_fn(|i| i as u8)
}

fn build() -> VectorFile {
    let cases = [
        (
            "empty-4of7",
            "empty",
            Params {
                k: 4,
                n: 7,
                segment_size: 256,
            },
        ),
        (
            "ascii-4of7",
            "ascii:neoblast golden vector",
            Params {
                k: 4,
                n: 7,
                segment_size: 256,
            },
        ),
        (
            "multi-segment-4of7",
            "mod251:5000",
            Params {
                k: 4,
                n: 7,
                segment_size: 256,
            },
        ),
        ("default-20of50", "mod251:100000", Params::default()),
    ];
    let vectors = cases
        .iter()
        .map(|(name, spec, params)| {
            let data = payload(spec);
            let sealed = seal_with_key(&data, *params, &key()).unwrap();
            let manifest = sealed.manifest.encode();
            Vector {
                name: (*name).into(),
                payload: (*spec).into(),
                payload_blake3: blake3::hash(&data).to_hex().to_string(),
                k: params.k,
                n: params.n,
                segment_size: params.segment_size,
                read_key: hex::encode(key()),
                header: hex::encode(sealed.manifest.header().encode()),
                segment_count: sealed.manifest.header().segment_count(),
                manifest_blake3: blake3::hash(&manifest).to_hex().to_string(),
                root: hex::encode(sealed.cap.root),
                read_link: sealed.cap.to_link(),
                verify_link: sealed.cap.verify_cap().to_link(),
            }
        })
        .collect();
    VectorFile {
        format: "neoblast v1 (suite 0x01)".into(),
        note: "Read key is bytes 0x00..0x1f. Test-only: never reuse a read key.".into(),
        vectors,
    }
}

fn path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spec/test-vectors/phase1.json")
}

#[test]
fn golden_vectors_match() {
    let built = build();
    if std::env::var_os("NBL_UPDATE_VECTORS").is_some() {
        let json = serde_json::to_string_pretty(&built).unwrap();
        std::fs::write(path(), json + "\n").unwrap();
        return;
    }
    let stored: VectorFile = serde_json::from_str(
        &std::fs::read_to_string(path()).expect("vectors missing; see module docs"),
    )
    .unwrap();
    assert_eq!(
        stored, built,
        "byte format changed; see module docs before regenerating"
    );
}
