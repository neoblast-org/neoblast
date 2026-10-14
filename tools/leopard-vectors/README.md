# Leopard fixture generator

Builds [Leopard-RS](https://github.com/catid/leopard) (BSD-3-Clause, by Christopher
A. Taylor) at a pinned commit and writes its parity output to
`spec/test-vectors/erasure-leopard.json`. See decision 0006 for why.

Leopard is cloned into a temporary directory on each run and is **not vendored**;
nothing from it is committed here. `harness.cpp` is our own code.

```sh
tools/leopard-vectors/generate.sh            # rewrites spec/test-vectors/erasure-leopard.json
tools/leopard-vectors/generate.sh out.json   # writes elsewhere, e.g. to diff
```

Needs git, a C++14 compiler (`c++`), python3 with `venv`, and network access (to
clone Leopard and install the `blake3` Python package into a throwaway venv).

## What it builds

| Build | Change to Leopard | Used for |
| --- | --- | --- |
| `ff16` | `LEO_HAS_FF8` disabled, so GF(2^16) is always used | cases with m ≤ k |
| `ff16-m-gt-k` | `ff16` plus the `recovery_count > original_count` argument check removed (it appears in both `leo_encode` and `leo_decode`) | cases with m > k |

Both changes are exact text replacements, and the script stops unless each
target occurs the expected number of times. Output is written to a temporary
file first, so a failed run never leaves a partial fixture file.

k = 1 is not covered: Leopard's k = 1 shortcut copies input buffers that do not
exist when m > 1 (it crashes once the check is removed), so there is no reference.

## Cases

`cases.txt` lists `k m shard_bytes`, one case per line. Shard bytes must be a
multiple of 64 and k ≥ 2. The input data is one splitmix64 stream, seeded with
`k*1000003 + m*1009 + shard_bytes`, matching `originals()` in
`crates/nbl-erasure/tests/leopard_fixtures.rs`.

On x86 the build uses `-mssse3 -mavx2`, which needs an AVX2 CPU (any x86 machine
from about 2015 on). Set `CXXFLAGS_SIMD=-mssse3` to build without AVX2 and confirm
the output does not depend on the SIMD path: it should be identical. Set
`CXXFLAGS_SIMD=` (empty) to pass no SIMD flags.
