# Security Review Log

<!-- baseline --> **Next review diffs from:** `e24f4538e88320993d7cd5497239f3bf8a776786` (`main`, recorded 2026-10-14)

A running record of branch security reviews. Each entry records the exact commit range
reviewed; the baseline marker line above is the machine-owned frontier the next
incremental review starts from. Newest entry first. Code reviews are logged separately
in `code-review-log.md`.

**Starting point:** no security review has been logged here yet. The baseline is
`main`'s first commit, so the first `/security-review-since` covers the full history,
including that first commit (the review-since skill's step 3 explains how). Review it
against `spec/threat-model.md` and `spec/scenarios.md`.

**How to run the next review:** `/security-review-since` (or `/review-since
security`). The skill files (`.claude/skills/review-since/SKILL.md` and its alias)
are the sole definition of the mechanics — this header does not restate them. A
hand-run review must follow the same protocol steps, pinning the tip before reviewing
and doing both halves of the bookkeeping: prepend the entry AND move the baseline
marker line; a frontier-moving entry's Base must equal the previous one's Tip unless
the entry explains why not.

## Entries

### 2026-10-14 · Full history (first security review)

| | Commit | Date | Subject |
| --- | --- | --- | --- |
| Base | empty tree `4b825dc642cb6eb9a060e54bf8d69288fbee4904` (includes root `57d6ed43503ab1e0421fa43c20c23cc699bfe0ed`) | — (root 2026-10-14T05:08:11Z) | (root) Phase 0/1: workspace, spec, decisions and offline crypto core |
| Tip | `e24f4538e88320993d7cd5497239f3bf8a776786` | 2026-10-14T15:39:21Z | feat: streaming seal, open and heal; segment-size benchmark (#4) |

- **Scope:** 5 commits (`git rev-list --count e24f453`), 107 files, +10454/−0
  (`git diff --shortstat <empty tree> e24f453`). The logged baseline was the root
  commit with no entries yet, so the review started from the empty tree (protocol
  step 3). Only the committed range was reviewed; the uncommitted doc edits in the
  working tree at the time are not covered.
- **Method:** the range diff was read directly against `spec/threat-model.md` and
  `spec/scenarios.md` (R1 forged input, H2 disk contents, C3 no kill switch), in
  this order: `neoblast-format` (header, layout, caps), `neoblast-crypto`
  (KDF, nonce, header as AAD), `neoblast-erasure`, `neoblast` (manifest, tree,
  stream, store, CLI), fuzz targets, then supply chain (`Cargo.toml`, `Cargo.lock`
  sources, `deny.toml`, all four workflows, Dependabot, `tools/leopard-vectors`).
  **Deviation:** the protocol asks for subagent fan-out and verification by a fresh
  subagent. This review ran as a single agent that could not start subagents, so
  each finding was instead checked by trying to refute it against the code. S-001
  was reproduced with the CLI; S-002 to S-004 were checked by reading the code.
  Afterwards a fresh agent verified all four against `e24f453` and confirmed them
  at the same severities. It reproduced S-001 independently and found it wider
  (dangling symlinks, symlinked segment directories); it corrected S-004's "only
  unpinned tool" claim. The worklist has the details.
- **Findings:** 4 (0 high, 1 medium, 3 low), in
  [`plans/todo/security-review-s-series.md`](../../plans/todo/security-review-s-series.md):
  - **S-001 (Medium, open):** `DirStore` writes through symlinks and opens FIFOs
    in an untrusted shard directory. `neoblast heal` overwrote a file outside the
    store with bytes the publisher chose. Fixing it needs a `crates/` change;
    recommended as **blocking v0.0.1**, which means a new 24 h fuzz campaign.
    (Later the same day: the maintainer agreed; fixed before the release in #6,
    see the worklist.)
  - **S-002 (Low, open):** nothing checks that the shards form one codeword, so a
    publisher can serve different content to readers who decode from parity.
  - **S-003 (Low, open):** the read link passes through argv, and the decoded key
    is copied into buffers that are not zeroized.
  - **S-004 (Low, open):** the nightly fuzz job installs an unpinned `cargo-fuzz`.
- **Checked, no finding:**
  - Every parameter comes from a header checked against the root; the decoders
    don't panic and are fuzzed.
  - Manifest size is checked before it is read; shard reads are capped by length.
  - The domain-separated hash tree has no self-pairing.
  - The KDF contexts and the per-file random key make deterministic nonces safe.
    The last-segment flag and the header (as AAD) stop truncation, reordering and
    moving segments between files.
  - `Debug` redacts the read key, and errors never carry secrets.
  - Partial output is never left behind: writes go to a `create_new` file that is
    renamed only on success.
  - No API deletes another party's shards (C3).
  - The release workflow is split: the verify job runs third-party code; the
    publish job has `id-token`, runs no build and needs the environment's
    approval.
  - Actions are pinned to SHAs, `reed-solomon-simd` to an exact version, Leopard
    to a commit, and the lockfiles use only crates.io.
- **Carried-over items:** none (first entry).
