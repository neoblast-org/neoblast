#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(h) = neoblast_format::Header::decode(data) {
        // Anything accepted must re-encode to the same bytes.
        assert_eq!(&h.encode()[..], data);
        for i in 0..h.segment_count().min(4) {
            let _ = h.segment_shard_len(i);
        }
    }
});
