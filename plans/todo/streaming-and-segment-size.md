# Streaming seal/open, then the segment-size benchmark (ROADMAP open item 2)

Status: todo. Blocks open item 2.

## Why streaming comes first

Open item 2's rule is "largest segment size keeping peak RAM ≤ 32 MiB per
operation". Today `seal` takes the whole payload as one slice and returns every
shard in memory, and the CLI reads the whole input file first. Sealing a 1 GiB
file at 20-of-50 therefore needs about 3.5 GiB of RAM whatever the segment size,
so a benchmark now would measure file size, not segment size.

Rough per-segment working set at 20-of-50 for segment size s:

- plaintext s, ciphertext s + 16, shards 2.5 s, so about 4.5 s in total;
- that is about 18 MiB at 4 MiB segments and about 36 MiB at 8 MiB.

The rule therefore probably picks 4 MiB, but it has to be measured.

## Success criteria

1. **Streaming seal.** A new `seal_stream(reader, params, sink)` encrypts, codes,
   hashes and writes one segment at a time. Peak memory is O(segment size), not
   O(file size). The manifest is written last.
2. **Streaming open.** A new `open_stream(cap, manifest, source, writer)` fetches,
   verifies, decodes and writes one segment at a time.
3. **Identical bytes.** Streamed and in-memory sealing produce byte-identical
   manifests and shards for every golden vector. A property test covers random
   sizes, including empty files, exactly one segment and one byte over.
4. **CLI uses streaming.** `neoblast seal` and `neoblast open` on a 1 GiB file
   stay under 32 MiB peak RSS at 4 MiB segments, measured with `/usr/bin/time -l`
   on macOS.
5. **Benchmark tool.** `bench/segment-size.sh` runs seal, open and heal at 1, 4,
   8 and 16 MiB segments on a 1 GiB file. It records wall time and peak RSS in
   `bench/results/` in the same privacy-safe style as `fuzz/campaigns/`.
6. **Decision.** The decision record applies the pre-written rule to the
   measured numbers and updates `Params::default()` only if the rule says so.
7. fmt, clippy `-D warnings`, the tests and `cargo deny` stay green. Fuzz
   targets cover the streaming open path.

## Precedent to match

- API shape: `std::io::copy`-style `Read` in, `Write` out; `ShardSource` for shard input.
- Tests: `crates/neoblast/tests/end_to_end.rs` and `golden_vectors.rs`.
- Records: `fuzz/campaign.sh` (JSON per run, `code_id`, no host details).
- Decision: `decisions/0006-erasure-code.md`.
