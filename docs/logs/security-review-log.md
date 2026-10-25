# Security Review Log

<!-- baseline --> **Next review diffs from:** `c3d405de4e4ff39c0132dd2f049f664396b22080` (`main`, recorded 2026-10-25)

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

### 2026-10-25 · `255353b..c3d405d` (#23, #25 and #26)

| | Commit | Date | Subject |
| --- | --- | --- | --- |
| Base | `255353b780c0a41de3d32259a2178d40d39691b5` | 2026-10-20T22:35:07Z | fuzz: campaign lock held by the fuzzers; warn on mixed compilers (#22) |
| Tip | `c3d405de4e4ff39c0132dd2f049f664396b22080` | 2026-10-25T02:34:33Z | fix: ctutils 0.4.5 for RUSTSEC-2026-0385; MSRV 1.87 (#26) |

- **Scope:** 3 commits (`git rev-list --count 255353b..c3d405d`) and 37
  files, +1096/−95 (`git diff --shortstat 255353b c3d405d`): #23, #25 and
  #26, of which 5 files, +303/−36, under `crates/` (the same command with
  `-- crates`). Base equals the previous entry's Tip. Only the committed
  range is covered. Run while the second v0.0.2 campaign runs on `c3d405d`.
- **Method:** a fresh agent read the `crates/` diff, the lock files and the
  MSRV change against `spec/threat-model.md`, `SECURITY.md` and CLAUDE.md,
  ran the tests, `cargo deny` on all three lock files and mutations; another
  read CI, scripts, the campaign records and docs for security and privacy,
  in scratch repositories and through GitHub's API. Two more fresh agents
  tried to refute each finding, together with the code review's.
- **Findings:** 4, all low, in
  [`plans/todo/security-review-s-series.md`](../../plans/todo/security-review-s-series.md):
  - **S-021:** the bench and vector scripts ran Python with the working
    directory on its module path, as S-020 was for `campaign.sh`
    (predates the range).
  - **S-022:** an identity could ride inside a commit or tag object, in a
    `mergetag` or another header or a tag's message, past the hygiene
    checks (the commit side predates the range).
  - **S-023:** #25 reached `main` while `deny` failed on an advisory
    published after the last green run, and the Release workflow does not
    run `cargo deny`.
  - **S-024:** library users can keep the `ctutils` that RUSTSEC-2026-0385
    flags; #26 fixed this repository's lock files only.
- **Not tracked:** a zero-width space, a fullwidth colon, a Cyrillic `у` or
  a line separator after prose still hides a `-by:` line from the
  attribution check, but git reads none of these as a trailer, and only a
  maintainer can write the message; the check exists for tools' ASCII
  trailers. The check also flags prose such as `(by: x)`: it fails closed,
  and no commit since `5393f67` matches.
- **Checked, no finding:**
  - #23's fixes fail closed: an `EIO` is returned, `EINTR` is retried, a
    shard counts only at the header's length, and heal still writes only
    bytes that match the manifest. Parameters still come only from the
    verified header.
  - `ctutils` 0.4.5 is in all three build graphs, its checksum matches the
    crate, it needs Rust 1.87 as the new MSRV says, and no other advisory is
    open.
  - S-017 and S-018 work as described; no `${{ }}` in any `run:` block;
    every step prints only hashes or tag names.
  - The 15 records hold no host, user or path; all are on `86b8a67`, with
    `nightly-2026-10-14`, `cargo-fuzz` 0.13.2 and no artifacts, and
    `summary` counts none of their hours for `c3d405d`.
  - Privacy: all 3 commits are the project's, in UTC, signed, signed off and
    fast-forwarded; no names, addresses, paths or session links in the
    range's text. The last line of `7c06619`'s message is garbled, but
    holds nothing private.
  - No open finding of medium severity or higher is described in public.
- **Outcome:** no blocker. S-021 to S-023 are fixed in the review PR that
  follows, outside `code_id`, the workflows applied by hand; so is S-024's
  CHANGELOG note, and its change to `Cargo.toml` waits until after v0.0.2.
- **Carried-over items:** S-002 open; S-003 open; S-007 open, with every
  merge a fast-forward (#25 before its checks had finished: S-023); S-008
  open until the GitHub-side copies are removed; S-016 to S-020 fixed in
  #23, the review-fix PR the last entry named.

### 2026-10-21 · `9bf18dc..255353b` (#20 to #22)

| | Commit | Date | Subject |
| --- | --- | --- | --- |
| Base | `9bf18dccf6e6603b10518b37852a1cda1a2b99bb` | 2026-10-17T17:30:52Z | chore: bump to 0.0.2; keep medium findings private until fixed (S-006) (#19) |
| Tip | `255353b780c0a41de3d32259a2178d40d39691b5` | 2026-10-20T22:35:07Z | fuzz: campaign lock held by the fuzzers; warn on mixed compilers (#22) |

- **Scope:** 3 commits (`git rev-list --count 9bf18dc..255353b`) and 18
  files, +909/−148 (`git diff --shortstat 9bf18dc 255353b`): #20, #21 and
  #22, of which 5 files, +379/−91, under `crates/` (the same command with
  `-- crates`). Base equals the previous entry's Tip. Only the committed
  range is covered. Run before the v0.0.2 campaign.
- **Method:** a fresh agent read the `crates/` diff against
  `spec/threat-model.md`, `SECURITY.md` and CLAUDE.md, ran the new tests
  against `9bf18dc` and probes; another read CI, scripts and docs for
  security and privacy, in a scratch repository. Two more fresh agents tried
  to refute each finding, together with the code review's; one read
  `actions/checkout`'s source and `v0.0.1`'s runs, the other probed
  `F_FULLFSYNC` on five file systems.
- **Findings:** 5, all low, in
  [`plans/todo/security-review-s-series.md`](../../plans/todo/security-review-s-series.md):
  - **S-016:** on macOS, a full sync that failed with an I/O error was
    followed by a plain `fsync` and reported as a success (from #20).
  - **S-017:** a tag of a tag, or of a tree, passed the tag checks (#21).
  - **S-018:** a line after a lone carriage return, or with a Unicode
    hyphen, passed the attribution check (#21).
  - **S-019:** a shard of the wrong length, from a source that does not
    check lengths, stopped `heal` (predates the range).
  - **S-020:** `campaign.sh` ran Python with `fuzz/` on its module path.
- **Not tracked:** the required `hygiene` check is matched by name, so a
  commit that changes the workflow can pass it (noted under S-014); GitHub
  sends no push event for more than three tags at once, and CI sees a
  branch or tag only once it is public, so `RELEASING.md` pushes one tag and
  checks it first.
- **Checked, no finding:**
  - Heal writes only bytes that match the manifest, and parameters still
    come only from the verified header. The `PermissionDenied` skip applies
    only outside the store; a symlinked segment directory is still refused.
  - No `${{ }}` in any `run:` block; every step prints only hashes or tag
    names, and fails closed. Ref names cannot break the parsing.
  - The campaign lock holds across a SIGKILL, also on Linux; records hold no
    host, user or path.
  - Privacy: all 3 commits are the project's, in UTC, signed, signed off and
    fast-forwarded; no names, addresses, paths or session links in the
    range's text; `v0.0.1` is the project's, in UTC.
  - No open finding of medium severity or higher is described in public.
- **Outcome:** no blocker. S-016, S-019 and S-020 are fixed in the
  review-fix PR that follows, before the v0.0.2 campaign; S-017 and S-018
  there too, applied by hand.
- **Carried-over items:** S-002 open (noted: heal rebuilds from the first k
  shards that match), S-003 open; S-007 open, with every merge a
  fast-forward; S-008 open until the GitHub-side copies are removed;
  S-010 and S-011 fixed in #20; S-012, S-013 and S-015 in #21 (step 7c′,
  not 7d as the last entry planned); S-014 fixed in the settings.

### 2026-10-20 · `5393f67..9bf18dc` (#11 to #19, the v0.0.2 candidate)

| | Commit | Date | Subject |
| --- | --- | --- | --- |
| Base | `5393f6704b5059c4c8ecfbb6adc01718c13c4455` | 2026-10-16T06:38:16Z | docs: release v0.0.1 on 2026-10-16 (#10) |
| Tip | `9bf18dccf6e6603b10518b37852a1cda1a2b99bb` | 2026-10-17T17:30:52Z | chore: bump to 0.0.2; keep medium findings private until fixed (S-006) (#19) |

- **Scope:** 11 commits (`git rev-list --count 5393f67..9bf18dc`) and 56 files,
  +3785/−306 (`git diff --shortstat 5393f67 9bf18dc`): #11 to #19, everything
  in v0.0.2 so far, of which 14 files, +1139/−201, under `crates/` (the same
  command with `-- crates`). Base equals the previous entry's Tip. Only the
  committed range is covered.
- **Method:** a fresh agent read the `crates/` diff against
  `spec/threat-model.md`, `SECURITY.md` and CLAUDE.md, with proofs of concept;
  another read CI, scripts and docs for security and privacy, testing the
  hygiene job's checks in a scratch repository. Two more fresh agents tried
  to refute each finding; S-010 and S-011 were reproduced.
- **Findings:** 6, all low, in
  [`plans/todo/security-review-s-series.md`](../../plans/todo/security-review-s-series.md):
  - **S-010:** on macOS, a volume without full syncs made every seal fail,
    against the docs (a regression from #13).
  - **S-011:** a publisher's inconsistent segment still stopped `heal`
    part-way, which the CHANGELOG said could no longer happen.
  - **S-012:** two older hygiene steps print more than hashes.
  - **S-013:** attribution outside git's trailer block passes the hygiene job.
  - **S-014:** `[skip ci]` skips the hygiene job unless it is required.
  - **S-015:** tags are not checked for the project's identity.
- **Checked, no finding:**
  - A hostile store: temporary files are made with `create_new` and renamed
    into place; the new syncs open only directories already checked as real
    ones, or ones the run made. S-001's, W-001's and W-008's tests pass.
  - S-005 is fixed: in this review's run a 264 MB planted manifest peaked
    `heal` at 265.7 MB (267 MB in #15's) and was refused in 2.5 s.
  - No secret reaches an error, a log or `Debug` output.
  - `rustix` is Apple-only, with its `fs` and `std` features; no `unsafe` in
    first-party code; `cargo deny` passes.
  - Privacy: no names, addresses, hosts, paths, time zones or session links in
    the range's text or the benchmark records; all 11 commits are the
    project's, in UTC, signed, and fast-forwarded.
  - Every number checked in the CHANGELOG, README, ROADMAP and decision 0009
    matches its record or the code.
  - Within the range no open finding of medium severity or higher is
    described in public. S-008's entry, written before `SECURITY.md`'s rule,
    stays as it is: it names no identity and points only at data already
    public on GitHub (maintainer's call, noted in the worklist).
- **Outcome:** no blocker. S-010 and S-011 are fixed in the review-fix PR
  that follows, before the v0.0.2 campaign; S-012 to S-015 in step 7d, applied
  by hand, with the hygiene check made required.
- **Carried-over items:** S-002 and S-003 open and unchanged; S-004 fixed in
  #12; S-005 fixed in #15 (its entry's test name corrected); S-006 fixed in
  #19; S-007 open, with every merge since #11 a fast-forward; S-008 open until
  the GitHub-side copies are removed; S-009 fixed.

### 2026-10-16 · `a9445d9..5393f67` (#8, #9, #10)

| | Commit | Date | Subject |
| --- | --- | --- | --- |
| Base | `a9445d9a32dcafe4a8bd018e4280e03411ce6a12` | 2026-10-15T00:34:56Z | docs: freeze the v1 format; changelog; first full code and security reviews (#7) |
| Tip | `5393f6704b5059c4c8ecfbb6adc01718c13c4455` | 2026-10-16T06:38:16Z | docs: release v0.0.1 on 2026-10-16 (#10) |

- **Scope:** 3 commits (`git rev-list --count a9445d9..5393f67`) and 24 files,
  +783/−20 (`git diff --shortstat a9445d9 5393f67`): #8 (review logs and
  worklists), #9 (15 fuzz records) and #10 (CHANGELOG date, decision 0004,
  `bench/heal-check.sh`, plan). Nothing under `crates/`, the Cargo files, the
  fuzz targets or `.github/`. Base equals the previous entry's Tip. Only the
  committed range is covered.
- **Method:** a fresh agent read the range against `spec/threat-model.md`,
  `SECURITY.md` and CLAUDE.md, recomputed every fuzz record's `code_id`, and
  checked the commit metadata and remote refs. A second fresh agent
  confirmed both findings.
- **Findings:** 2 (0 high, 1 medium, 1 low), in
  [`plans/todo/security-review-s-series.md`](../../plans/todo/security-review-s-series.md):
  - **S-008 (Medium):** a maintainer's personal account reached commit
    metadata through GitHub-side actions during the release. Fixed on `main`
    the same day; the remaining GitHub-side copies are being removed.
  - **S-009 (Low):** the 0.0.1 known issues omitted that a hostile store can
    stop `heal` (W-003, W-004). Fixed in the closing PR.
- **Checked, no finding:**
  - Within the range, no open medium-or-higher issue is described in public.
  - The fuzz records hold no host, user or path data, and all are UTC.
    Every record's `code_id` matches the release commit. Each target has a
    clean 24 h run with no artifacts; the overlap in the stopped runs is
    W-014.
  - `bench/heal-check.sh`: `mktemp -d` with an exit trap, quoted paths. It
    prints seal's and heal's stderr, which never contains the read link:
    `seal` prints the read link on stdout only, and heal gets only the
    verify link.
  - The CHANGELOG's S-001, W-001, S-005 and fuzz claims, and decision
    0004's new status, match the code and `spec/caps.md`.
- **History note:** while the review ran, the last two commits of the range
  were re-made on `main` with the project's identity and the same files
  (S-007, S-008). The review read the earlier commits; their trees equal
  `36cb370` and `5393f67`, so the Tip and the baseline name `5393f67`.
- **Carried-over items:** S-002, S-003, S-004 and S-006 open and unchanged;
  S-005 open, now listed in the CHANGELOG; S-007 open, with the last two
  commits re-made with the project's identity.

### 2026-10-15 · `e24f453..a9445d9` (#5, #6, #7)

| | Commit | Date | Subject |
| --- | --- | --- | --- |
| Base | `e24f4538e88320993d7cd5497239f3bf8a776786` | 2026-10-14T15:39:21Z | feat: streaming seal, open and heal; segment-size benchmark (#4) |
| Tip | `a9445d9a32dcafe4a8bd018e4280e03411ce6a12` | 2026-10-15T00:34:56Z | docs: freeze the v1 format; changelog; first full code and security reviews (#7) |

- **Scope:** 3 commits (`git rev-list --count e24f453..a9445d9`), 25 files,
  +942/−80 (`git diff --shortstat e24f453 a9445d9`). Code: #6's change to
  `crates/` (6 files, +335/−40), `bench/heal-check.sh` (#5) and the `vectors`
  CI job (#7); the rest is docs. Base equals the previous entry's Tip. Only the
  committed range is covered; the uncommitted code-review log edits in the
  working tree are not.
- **Method:** a fresh agent read the range diff directly against
  `spec/threat-model.md`, `spec/scenarios.md` and the hostile-store model in
  `crates/neoblast/src/store.rs`, and ran 10 hostile-store cases against a
  release build of `a9445d9`. A second fresh agent then checked each finding:
  S-005 reproduced twice with the release CLI, S-006 and S-007 confirmed from
  the history and commit metadata.
- **Findings:** 3 (0 high, 0 medium, 3 low), none blocking v0.0.1, in
  [`plans/todo/security-review-s-series.md`](../../plans/todo/security-review-s-series.md):
  - **S-005 (Low, open):** rejecting a planted manifest just under the 256 MiB
    cap costs about 3.2 times its size in memory (about 822 MiB) before the root
    check (`crates/neoblast/src/manifest.rs:76-84` and `:92-102`). Bounded, and no worse than
    an honest file with the same parameters.
  - **S-006 (Low, open):** S-001's exploit details were public on `main` for
    about 5.5 hours before its fix. Process gap; no release was affected.
  - **S-007 (Low, open):** GitHub's squash merges replace the project's UTC,
    signed commit metadata with the merger's, including a local time-zone
    offset.
- **Checked, no finding:**
  - S-001's fix holds within its model (races excluded): a shard hard-linked
    to an outside file, a shard path that is a directory, a `seg-*` entry that
    is a file or a symlink, `manifest.nbl` as a symlink, a dangling symlink or
    a FIFO, and `seal --force` over planted symlinks and hard links. Nothing
    outside the store was read, written or truncated, and nothing blocked.
  - `replace_file` removes its temporary file only after creating it.
  - `open_regular` refuses symlinks, FIFOs, sockets and devices for both the
    segment directory and the file; on Windows, junctions are reported as
    symlinks and refused too.
  - The W-001 cap is checked before any allocation on every read path, and
    `read_manifest` is the only place a manifest is read. The remaining cost is
    S-005, plus W-006 in the library.
  - `seal --force` validates the parameters and the manifest size before it
    removes the old manifest.
  - `Header::new`'s new ordering has no panic path.
  - The `vectors` CI job: checkout pinned to the same SHA as the other jobs,
    `contents: read`, `pull_request` trigger, no `${{ }}` in `run:`. It is a
    tripwire, not a control, since a PR can change a file and its hash
    together.
  - New errors show store paths and lengths, never links. The CHANGELOG's
    security bullets match the code.
  - Correction to the previous entry: "Manifest size is checked before it is
    read" held only for a manifest whose size disagreed with its own header
    (W-001); since #6 it holds in general.
  - Suggested, not filed: a regression test with a symlink or FIFO at
    `manifest.nbl` (W-008's area), and a threat-model row for a hostile local
    store. Races while healing a store others can write to are out of scope.
- **Carried-over items:** S-001 fixed in `9a542da` (#6); S-002, S-003 and S-004
  still open, unchanged in the range. The code-review findings W-003 and W-004
  (a hostile store can stop heal) stay low, with no security ID.

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
