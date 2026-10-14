# Streaming seal, open and heal

Status: completed 2026-10-14. Prerequisite for ROADMAP open item 2 (see
`plans/todo/segment-size-decision.md`).

## Why

`seal` took the whole payload as one slice and returned every shard in memory,
and the CLI read the whole input first: sealing 1 GiB at 20-of-50 needed about
3.5 GiB of RAM. A segment-size benchmark against a memory limit was meaningless
until memory stopped growing with the file.

## Success criteria

1. **Streaming seal.** `seal_stream` encrypts, codes, hashes and hands over one
   segment at a time. Memory grows with the segment size, not the file size;
   only the manifest's hashes (n × 32 bytes per segment) grow with the file. The
   manifest comes last.
2. **Streaming open.** `open_stream` fetches, verifies, decodes and writes one
   segment at a time, reading only k shards per intact segment.
3. **Identical bytes.** The streamed output equals whole-file sealing for every
   length and parameter tested. The golden vectors are unchanged.
4. **Hostile inputs and failures handled:**
   - input shorter or longer than declared → `Error::InputLength`;
   - read and sink errors → `Error::Io`;
   - non-canonical framing (non-zero padding, bad length prefix) → `Framing`;
   - a failed open has written only a verified prefix of the payload.
5. **CLI streams.** It never leaves a truncated output file under the requested
   name.
6. **A benchmark tool records peak memory and time** per operation, without
   host details.
7. fmt, clippy `-D warnings`, all tests and the fuzz targets stay green, with
   no new dependencies.

## Precedent matched

- API pairs: `seal` / `seal_with_key` (now `seal_stream` / `seal_stream_with_key`);
  `ShardSink` mirrors `ShardSource`.
- Tests: `crates/neoblast/tests/end_to_end.rs` and `golden_vectors.rs`.
- Records: `fuzz/campaign.sh` (one JSON record per run, no host details).

## Results

- `seal`, `open` and `heal` are now thin wrappers over the streaming functions,
  so there is one code path. `neoblast_crypto::SegmentCipher` encrypts one
  segment in place, and `neoblast_erasure::encode_parity` codes without copying
  the data shards.
- Tests:
  - streamed output matches an independent whole-file reference (property test
    over lengths, k/n, segment sizes, uneven and interrupted reads);
  - every length from 0 to 700 bytes at 64-byte segments;
  - framing rejection, checked by a mutation that disables the padding check;
  - intact opens read exactly k shards per segment;
  - a CLI test shows a failed open leaves no file.
- Indicative numbers from a Linux x86-64 cloud VM, 256 MiB file at 20-of-50
  (the reference run is the maintainer's Mac). Each cell is peak / working MiB;
  working is the peak minus the same operation on a 1 KB file.

  | Segment | seal | open | heal | open from parity |
  | --- | --- | --- | --- | --- |
  | 1 MiB | 15.6 / 4.4 | 5.6 / 2.7 | 17.3 / 5.8 | 17.5 / 6.0 |
  | 4 MiB | 27.7 / 16.5 | 11.1 / 8.3 | 32.6 / 21.0 | 32.5 / 21.1 |
  | 8 MiB | 44.0 / 32.8 | 19.0 / 16.1 | 53.6 / 42.1 | 53.2 / 41.8 |
  | 16 MiB | 76.7 / 65.5 | 35.0 / 32.2 | 94.6 / 83.1 | 111.5 / 99.9 |

  The fixed cost is about 3 MiB for the program, plus 8 MiB for the erasure
  library's multiplication table in any operation that encodes or decodes. The
  largest per-segment cost is the decoder's working buffer, about 3.2× the
  segment size at 20-of-50.
- Second review (independent agent): no byte-compatibility or authentication
  bugs. Fixed:
  - **CLI open:**
    - writes to a new hidden `.<name>.<pid>.partial` (`create_new`, so it never
      truncates or follows an existing file or symlink);
    - the rename is inside the error path, so no decrypted partial file is ever
      left behind;
    - an output that is a directory is refused up front;
    - a failed open keeps any existing output.
  - **CLI seal:**
    - refuses a non-empty store without `--force`;
    - with `--force`, removes the old manifest first, so a failed seal can't
      leave a manifest describing mixed shards;
    - pipes and devices work again (read into memory, since the header needs
      the length first).
  - **Bounded reads:**
    - `ShardSource::get` now receives the expected shard length, and
      `DirStore` refuses files of any other size before reading;
    - `read_manifest` checks the size its header implies.
  - **Heal** repairs every recoverable segment before reporting an
    unrecoverable one.
  - **Seal** no longer pre-allocates from a caller's declared length (a huge
    length aborted the process).
  - **Intact opens** skip the extra copy (working memory 12.4 → 8.3 MiB at
    4 MiB segments).
  - **`seal_segments`** no longer leaves an unwiped plaintext copy behind.
  - **Benchmark:**
    - finds the binary through `cargo metadata`;
    - uses `time(1)` wall time;
    - records the start time and computes working memory before rounding;
    - deletes each output after checking it;
    - cleans up on Ctrl-C.
  - **New tests:**
    - the exact rebuilt set, including a good shard after the first k;
    - heal past an unrecoverable segment;
    - k+1 reads with a corrupt data shard;
    - writer and sink I/O errors;
    - a huge declared length;
    - CLI: output is a directory, a failed open changes nothing, `--force`,
      oversized shard and manifest files, sealing from a pipe.
- Deliberately left:
  - `open` doesn't check `padme(payload_len + 8) == padded_length` (true
    before this change too); a canonical-length rule would be a spec change.
  - The output directory isn't fsynced after the rename.
  - Benchmark records have no `code_id` like fuzz records; they carry the
    commit and an uncommitted-changes flag instead.
