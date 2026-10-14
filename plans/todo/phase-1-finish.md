# Finish Phase 1 and release v0.0.1

Status: in progress (started 2026-10-14). Owner: maintainer, with Claude doing the code and docs.

**2026-10-14, maintainer's call:** v0.0.1 is the fuzzed `e24f453`. The heal memory
fix waits on the local branch `perf/heal-memory` for v0.0.2, and the segment-size
decision (criterion 6) moves after the tag, because it needs the fixed heal's numbers.

## The gate (ROADMAP.md, Phase 1)

> Golden vectors frozen; drop-any-(n−k) property tests pass; 24 h fuzzing per
> target with no crash, recorded in `fuzz/campaigns/`.

The drop-any-(n−k) tests already pass: `survives_any_n_minus_k_losses` in
`crates/neoblast/tests/end_to_end.rs` and the erasure property test. What
remains is the fuzz gate, the freeze and the release.

## Success criteria

1. **Fuzz gate met.** `fuzz/campaign.sh summary` on `main` shows `met` for all
   five targets. The run records are committed in `fuzz/campaigns/`.
2. **v0.0.1 is the fuzzed code.** The tag sits on a commit whose `code_id`
   equals the fuzzed one for every target. That means nothing in `crates/`,
   `fuzz/`, `Cargo.toml` or `Cargo.lock` changed between the campaign and the
   tag; docs, spec, plans and CI may change.
3. **Reviews done.** `/review-since` and `/security-review-since` cover
   everything up to the release commit, and both logs in `docs/logs/` have new
   entries with no open blocker.
4. **Vectors frozen.**
   - CI fails if `spec/test-vectors/phase1.json` or `erasure-leopard.json`
     changes at all.
   - `spec/header.md` and `spec/test-vectors/README.md` say v1 is frozen, and
     the pre-freeze exception is removed.
   - Any later format change needs a new magic or suite byte.
5. **Published.**
   - All four crates are on crates.io at 0.0.1.
   - Owners are `neoblast-dev` and the secret `neoblast-org` team.
   - Trusted publishing is configured for each crate, and the one-time token is
     revoked.
   - The tag is signed, and a GitHub release with notes exists.
6. ~~**Segment size decided** (open item 2).~~ Moved to v0.0.2 (see the
   status note): the record needs numbers from the fixed heal, which is new
   code. `plans/todo/segment-size-decision.md` tracks it. `CHANGELOG.md` lists
   the heal memory growth and the provisional default as known issues.
7. **Phase closed.** ROADMAP marks Phase 1 done and Phase 2 current, and this
   plan moves to `plans/completed/` with results.

## Precedent to match

- Plan format: `plans/completed/erasure-leopard-check.md`.
- Release steps: `RELEASING.md` (written for exactly this).
- Decision record: `decisions/0006-erasure-code.md`.

## Steps

Two tracks run in parallel while the M1 fuzzes. Anything that changes
`crates/` waits until after the tag, because it would reset the fuzz hours.

### Track A: while the campaign runs (no code changes)

| # | Step | Who | Time |
| --- | --- | --- | --- |
| A1 | Finish M1 setup; start `fuzz/campaign.sh all 24` on `main` (`e24f453`) | Maintainer | 30 min, then 24 h |
| A2 | ~~Memory test on the dev Mac: RSS vs "peak memory footprint" for heal~~ Done: heal grows; fixed on `perf/heal-memory` (v0.0.2) | Maintainer | 3 min |
| A3 | ~~Rerun the benchmark~~ Moved to v0.0.2, on the fixed heal | Claude, then maintainer | 1 h + 10 min |
| A4 | ~~Write the segment-size decision (0009)~~ Moved to v0.0.2 | Claude | 30 min |
| A5 | Code review and security review since the last logged reviews; log both | Claude (`/review-since`, `/security-review-since`) | 1–2 h |
| A6 | Freeze PR: spec wording, CI hash check of the vectors, `CHANGELOG.md`. Drafted; the `vectors` job in `ci.yml` is applied by hand | Claude, then maintainer (CI files are moved in by hand) | 1 h |
| A7 | GitHub release setup from `RELEASING.md`: `release` environment, `v*` tag ruleset, secret `maintainers` team, crates.io access for the org | Maintainer | 15 min |
| A8 | crates.io: sign in as `neoblast-dev`, verify the email, prepare a 1-day token | Maintainer | 10 min |

If A5 finds something that must be fixed in `crates/` before release, the fix
lands, the affected targets are fuzzed again for 24 h, and the tag moves to the
fixed commit. Doing A5 early is what keeps this off the critical path.

### Track B: after the campaign passes

| # | Step | Who |
| --- | --- | --- |
| B1 | Copy the M1 records from `fuzz/runs/` into `fuzz/campaigns/`, PR, merge. `summary` shows `met` ×5 | Maintainer |
| B2 | Check that `main`'s `code_id` still equals the fuzzed one (`fuzz/campaign.sh summary` on `main`) | Maintainer |
| B3 | Check that `CHANGELOG.md`'s claims hold (the `vectors` CI job is on `main` and green; `summary` shows `met` ×5), then set its release date (docs only); tag `v0.0.1` (signed) on that commit; manual first publish (`RELEASING.md` "first release") | Maintainer |
| B4 | Trusted publishing and team owner on each crate; revoke the token; reject the Release workflow's v0.0.1 run | Maintainer |
| B5 | GitHub release with notes from `CHANGELOG.md` | Claude drafts, maintainer publishes |
| B6 | Close the phase: ROADMAP update, this plan to `completed/` | Claude |

### If the campaign finds a crash

1. Stop. Treat it as a security issue (`SECURITY.md`); don't publish the input.
2. Reproduce it, fix it, and add a regression test.
3. Re-fuzz the affected target, or all of them if the fix is in shared code,
   for 24 h on the fixed commit.
4. The tag moves to the fixed commit.

## After v0.0.1 (v0.0.2, not part of this gate)

- The heal memory fix (`perf/heal-memory`), then the segment-size rerun and
  decision 0009, changing the default only if the rule says so.
- Update the pre-freeze wording in `crates/neoblast/tests/golden_vectors.rs`'s
  module comment, which can't change before the tag without resetting the fuzz
  hours.
- A new 24-hour campaign on the v0.0.2 code before tagging it.
- Phase 2 prerequisites:
  - accept decisions 0004 and 0005;
  - the libp2p vs iroh spike (open item 6), which needs a small VPS.
