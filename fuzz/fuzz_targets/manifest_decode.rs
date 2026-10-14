#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Root is attacker-unknown here; decode must reject or accept without panicking.
    let _ = neoblast::Manifest::decode(data, &[0u8; 32]);
});
