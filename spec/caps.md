# Capabilities and links (v1)

Nested capabilities: **verify ⊂ read ⊂ write**. Holding one lets you derive the
weaker ones, never the stronger (decision 0004).

## Encoding

- Payloads are RFC 4648 base32, **lowercase, unpadded**. Decoders MUST accept either case.
- Every payload before `#` starts with a version byte, `0x01` in v1. Links are not
  multiformats CIDs (decision 0007).
- Secrets MUST appear only after `#`. Anything after `&` in the fragment is reserved for hints (for example `&p=<peer>`) and MUST be ignored by v1 decoders.

## File caps (implemented)

| Cap | Link | Contains |
| --- | --- | --- |
| Verify | `nbl://fv/<b32(0x01 ‖ H)>` | File root `H` |
| Read | `nbl://f/<b32(0x01 ‖ H)>#<b32(R)>` | `H` and the 32-byte random read key `R` |

## Channel caps (Phase 2, draft)

| Cap | Link | Contains |
| --- | --- | --- |
| Verify | `nbl://cv/<b32(0x01 ‖ pk)>` | Ed25519 public key |
| Read | `nbl://c/<b32(0x01 ‖ pk)>#<b32(Rc)>` | Public key and read key `Rc` |
| Write | `nbl://cw/#<b32(seed)>` | 32-byte Ed25519 seed. MUST NOT be shared |

```text
Rc       = BLAKE3.derive_key("nbl/v1/channel-read", seed)
K_search = BLAKE3.derive_key("nbl/v1/collection-search", Rc)
seed_i   = BLAKE3.derive_key("nbl/v1/channel/<i>", master)
keyring  = BLAKE3.derive_key("nbl/v1/keyring", master)
master   = from a 24-word BIP39 phrase
```

## Limits

A shared read cap cannot be revoked. Losing a write seed freezes the channel.
