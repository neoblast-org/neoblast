#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        if let Ok(cap) = neoblast_format::FileReadCap::parse(s) {
            assert_eq!(neoblast_format::FileReadCap::parse(&cap.to_link()).ok(), Some(cap));
        }
        let _ = neoblast_format::FileVerifyCap::parse(s);
    }
});
