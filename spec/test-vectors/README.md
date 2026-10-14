# Golden test vectors

`phase1.json` locks the v1 byte format: header, hash tree, root, links and the
erasure-coded shards (through the manifest hash).

Every implementation MUST reproduce every field from the inputs. Payload specs:
`empty`, `ascii:<text>`, or `mod251:<len>` (byte *i* = *i* mod 251). The read key
is bytes `0x00..0x1f`.

Changing this file is a format change: it needs a decision record and a spec
version bump. Regenerate with:

```sh
NBL_UPDATE_VECTORS=1 cargo test -p neoblast --test golden_vectors
```
