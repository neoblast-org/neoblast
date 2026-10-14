# 0002: The file root commits to every parameter

- Status: accepted
- Date: 2026-10-13

## Context and problem statement

In the first prototype, `k`, `n`, the shard index and the proof length travelled beside each shard but were not covered by the root hash. A peer could forge them, for example set `n < k`, and crash any healer (code-review issues 1, 3 and 8).

## Decision outcome

The file root `H` covers a fixed 32-byte big-endian header and a hash tree over every shard (see `spec/header.md`). Nodes take every parameter from a header that has been checked against `H`, never from a peer.

### Consequences

- Good: forged parameters are rejected before any decoding; the header is also the AEAD associated data, so ciphertext cannot be moved between files.
- Good: the format is fully pinned by golden vectors.
- Bad: changing any header field is a format change requiring a version bump.
