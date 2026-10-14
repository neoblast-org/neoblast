# Golden test vectors

## `phase1.json`

`phase1.json` locks the v1 byte format: header, hash tree, root, links and the
erasure-coded shards (through the manifest hash).

Every implementation MUST reproduce every field from the inputs. Payload specs:
`empty`, `ascii:<text>`, or `mod251:<len>` (byte *i* = *i* mod 251). The read key
is bytes `0x00..0x1f`.

Changing this file is a format change: it needs a decision record and a spec
version bump. The one exception is before the Phase 1 gate freezes the vectors,
when nothing has been released: then a decision record suffices (as in 0006). Regenerate with:

```sh
NBL_UPDATE_VECTORS=1 cargo test -p neoblast --test golden_vectors
```

## `erasure-leopard.json`

Parity shards produced by the original Leopard-RS C library, so the erasure code
is defined by an implementation independent of ours (decision 0006). Each case
gives `k`, `m` (parity count), `shard_bytes`, the BLAKE3 of all `m` recovery shards
concatenated, and the first 16 bytes of recovery shard 0. The input data is one
splitmix64 stream described in the file's `input` field.

`crates/nbl-erasure/tests/leopard_fixtures.rs` checks every case on every test run,
and also rebuilds the data from Leopard's parity. Regenerate (needs a C++
compiler and network access) with:

```sh
tools/leopard-vectors/generate.sh
```
