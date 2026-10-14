# File header, layout and root (v1)

Implemented in `crates/nbl-format`, `crates/nbl-erasure` and
`crates/neoblast/src/tree.rs`. Locked by `test-vectors/phase1.json` and, for the
parity bytes, `test-vectors/erasure-leopard.json`.

## Header (32 bytes, big-endian)

| Offset | Bytes | Field | Rule |
| --- | --- | --- | --- |
| 0 | 4 | magic | `4e 42 4c 01` (`NBL\x01`) |
| 4 | 1 | suite | `0x01` = XChaCha20-Poly1305 + BLAKE3 + Leopard-RS high-rate Reed-Solomon over GF(2^16) |
| 5 | 2 | k | 2 ≤ k ≤ 64 |
| 7 | 2 | n | k < n ≤ 128 |
| 9 | 4 | segment_size | 64 ≤ s ≤ 64 MiB; plaintext bytes per full segment |
| 13 | 4 | shard_size | MUST equal `shard_len(segment_size, k)` |
| 17 | 4 | segment_count | MUST equal `ceil(padded_length / segment_size)` |
| 21 | 8 | padded_length | Length of the framed plaintext; MUST satisfy `padme(x) == x` and x ≥ 8 |
| 29 | 2 | flags | MUST be 0 |
| 31 | 1 | reserved | MUST be 0 |

A decoder MUST reject any header that breaks a rule. Decoders MUST NOT panic on any input.

## Framing and padding

```text
framed = u64_be(payload_len) || payload || zero bytes up to padded_length
padded_length = padme(payload_len + 8)
```

`padme(L)`: for L < 2 return L. Otherwise E = ⌊log2 L⌋, S = ⌊log2 E⌋ + 1,
m = 2^(E−S) − 1, return (L + m) & ¬m. Leaks O(log log L) bits of the length, at most ~12% overhead.
Openers MUST reject non-zero padding.

## Segments and encryption

Segment `i` holds bytes `[i·segment_size, min((i+1)·segment_size, padded_length))` of `framed`.

```text
data_key   = BLAKE3.derive_key("nbl/v1/file-data", read_key)
nonce(i)   = 15 zero bytes || u64_be(i) || (0x01 if last segment else 0x00)
ciphertext = XChaCha20-Poly1305(data_key, nonce(i), plaintext_i, aad = header)
```

The read key is fresh and random per file, so deterministic nonces never repeat under one key.

## Shards

```text
shard_len(plain, k) = 64 · ceil(ceil((plain + 16) / k) / 64)
```

Shards are always a multiple of 64 bytes, Leopard-RS's alignment. Each segment's
ciphertext is zero-padded to `k · shard_len` and cut into `k` data shards. The last
segment uses its own `shard_len`. Openers truncate the joined data shards to
`plain + 16` bytes.

The `n − k` parity shards are the recovery shards of the Leopard-RS **high-rate**
algorithm over GF(2^16) (decision 0006): `leo_encode` from
[catid/leopard](https://github.com/catid/leopard) at commit
`22ddc7804998d31c8f1a2617ee720e063b1fa6cd`, built
without the 8-bit field (`LEO_HAS_FF8` off) and, when `n − k > k`, with its
`recovery_count > original_count` argument check removed from `leo_encode` and
`leo_decode`. The algorithm itself
is unchanged. In Rust this is `reed_solomon_simd::rate::HighRateEncoder` 3.1.0;
the crate's default rate selection MUST NOT be used, because for some k and n it
picks the low-rate algorithm, which yields different parity.
`test-vectors/erasure-leopard.json` holds Leopard's own output for 66 cases.

## Hash tree and root

```text
shard_hash   = BLAKE3(shard)
shard leaf   = BLAKE3(0x00 || u16_be(index) || shard_hash)
inner node   = BLAKE3(0x01 || left || right)          odd node promoted unchanged
segment root = tree over the n shard leaves
segment leaf = BLAKE3(0x02 || u32_be(segment) || segment_root)
content root = tree over the segment leaves
H            = BLAKE3.derive_key("nbl/v1/root", header || content_root)
```

## Verify manifest

`header || shard_hash[0][0..n] || shard_hash[1][0..n] || …` (32 bytes each).
A manifest is accepted only if it recomputes to the expected `H`. It reveals the
padded size, the parameters and shard hashes; nothing about content.
