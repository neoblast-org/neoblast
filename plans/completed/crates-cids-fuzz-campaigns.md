# Crate names, content IDs and fuzz campaigns

Status: completed 2026-10-14. Records: decisions 0007 and 0008.

## Success criteria

1. **Crates publishable.** All four crates (`neoblast`, `neoblast-format`,
   `neoblast-crypto`, `neoblast-erasure`) package and verify with
   `cargo publish --workspace --dry-run`. No `publish = false` remains on a
   library the CLI depends on.
2. **Wire format untouched.** The rename leaves the format alone: `nbl://`,
   `nbl/v1/...` and `NBL\x01` are unchanged, the golden vectors are unchanged,
   and no dependency versions moved in `Cargo.lock`.
3. **Safe releases.** Releasing needs no long-lived token: a tag-triggered
   workflow uses trusted publishing behind an approval environment, and the
   one-time manual first publish is written down step by step.
4. **Open item 3 settled.** It is settled by the rule written before the check,
   with every planned feature examined.
5. **New fuzz targets.** They cover the attacker-facing paths beyond parsers
   (`open`/`heal` with hostile shards, erasure decoding) and check correctness,
   not only "no panic".
6. **Runs are recorded in the repo.** Records hold no host, user or path
   details, hours add up only on identical code, and interrupted runs and
   crashes are recorded correctly.
7. **A fuzzing Mac can be set up from the README alone,** with no credentials on it.

## Precedent matched

- Decision records: `decisions/0006-erasure-code.md`.
- Fuzz targets: the existing `fuzz/fuzz_targets/*.rs`.
- Generator-style tooling and its README: `tools/leopard-vectors/`.
- Workflow style (pinned action SHAs, minimal permissions): `.github/workflows/ci.yml`.

## Results

- Dry run: all four crates package and verify. The names are unclaimed on
  crates.io as of today.
- New targets: they build without warnings and pass 3,000 random inputs each.
  Real coverage-guided runs need nightly, which is for the fuzzing Mac.
- `campaign.sh` was tested end to end with stand-ins for the nightly tools:
  - a timed run is recorded as `clean`;
  - SIGTERM stops all five fuzzers, which are recorded as `stopped` with exit 72
    and leave no stray processes;
  - a crash is recorded as `crash` with its artifact name;
  - the clean-tree guard refuses uncommitted code.
- Second review (independent agent): no blockers and no false alarms from the
  fuzz oracles. Fixed:
  - interrupt race (18 stop timings from 0.1 s to 6 s: no hangs, all `stopped`);
  - `code_id` now covers both manifests and lock files, and is captured at launch;
  - per-target run time comes from each fuzzer's own exit;
  - records go to the untracked `fuzz/runs/`, so `git pull` keeps working on the
    fuzzing Mac;
  - `heal` must rebuild exactly the missing or wrong shards (1,500 targeted inputs);
  - `erasure_restore` covers k up to 64 and n up to 128;
  - the release workflow tests in a job without publishing rights, and RELEASING.md
    was corrected (approval order, lock files, `change-owners` scope, the v0.0.1 run);
  - decision 0007 covers pinning, the denylist and Phase 5;
  - licence files are in every crate, and repo-dependent tests are excluded from
    the packages.
- Not done here: the segment benchmark. It needs streaming first; see
  `plans/todo/streaming-and-segment-size.md`.
