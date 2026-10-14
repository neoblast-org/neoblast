# 0001: Rust-only MVP

- Status: accepted
- Date: 2026-10-13

## Context and problem statement

The original plan targeted seven language bindings and seven CLI locales in twelve weeks. Neoblast is built by one person, part-time.

## Decision outcome

One Rust workspace ships two binaries: `neoblast` (CLI) and, from Phase 2, `neoblastd` (node daemon). No language bindings until Phase 5; WASM and Python come first even then. Internal crates are `publish = false`; only `neoblast` is published to crates.io.

### Consequences

- Good: one toolchain, one test suite, a working publish/fetch loop sooner.
- Bad: no browser or Python users until Phase 5. The golden vectors keep the door open for other implementations.
