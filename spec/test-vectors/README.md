# Golden test vectors

## `phase1.json`

`phase1.json` locks the v1 byte format: header, hash tree, root, links and the
erasure-coded shards (through the manifest hash).

Every implementation MUST reproduce every field from the inputs. Payload specs:
`empty`, `ascii:<text>`, or `mod251:<len>` (byte *i* = *i* mod 251). The read key
is bytes `0x00..0x1f`.

## Frozen

`phase1.json` and `erasure-leopard.json` were frozen at the Phase 1 gate, with
v0.0.1. They never change: CI checks both files against pinned SHA-256 hashes.
A format change needs a new magic version byte, a new suite byte or, for links,
a new link version byte (see [`header.md`](../header.md)), a decision record,
the spec text for the new format and a **new** vector file next to these. The v1 files stay as they are.

`NBL_UPDATE_VECTORS=1 cargo test -p neoblast --test golden_vectors` rewrites
`phase1.json`. Run it only to check that it still writes the same bytes.

## `erasure-leopard.json`

Parity shards produced by the original Leopard-RS C library, so the erasure code
is defined by an implementation independent of ours (decision 0006). Each case
gives `k`, `m` (parity count), `shard_bytes`, the BLAKE3 of all `m` recovery shards
concatenated, and the first 16 bytes of recovery shard 0. The input data is one
splitmix64 stream described in the file's `input` field.

`crates/neoblast-erasure/tests/leopard_fixtures.rs` checks every case on every test run,
and also rebuilds the data from Leopard's parity. `tools/leopard-vectors/generate.sh`
(needs a C++ compiler and network access) rebuilds it from upstream Leopard; the
result must be byte-identical.
