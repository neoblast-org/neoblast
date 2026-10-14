# Fuzz targets

Requires nightly Rust and `cargo install cargo-fuzz`.

```sh
cargo +nightly fuzz run header_decode -- -max_total_time=86400
cargo +nightly fuzz run link_parse    -- -max_total_time=86400
cargo +nightly fuzz run manifest_decode -- -max_total_time=86400
```

Phase gates require 24 hours per target with no crash. CI runs each for 10 minutes nightly.
