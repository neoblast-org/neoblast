# Finish Phase 1 and release v0.0.1

Status: in progress (started 2026-10-14). Owner: maintainer, with Claude doing the code and docs.

**2026-10-14, maintainer's calls:** first, v0.0.1 was to be the fuzzed `e24f453`,
with the heal memory fix held for v0.0.2. Then the reviews (A5) found S-001, a
store directory that makes `heal` write through symlinks, and the maintainer
chose to fix it before the release, together with W-001 and W-002 (code PR
#6). The 24 h campaign reruns once it merges. The heal memory fix is a
separate experiment on `exp/heal-reuse`, kept for measurement and not part of
that PR. A3 then showed that it lowers heal's peak but does not stop the growth,
so the maintainer left it out of v0.0.1. The segment-size decision moves to
v0.0.2 with it, as ROADMAP item 2's rule says.

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
   - Any later format change needs a new magic, suite or link version byte.
5. **Published.**
   - All four crates are on crates.io at 0.0.1.
   - Owners are `neoblast-dev` and the secret `neoblast-org` team.
   - Trusted publishing is configured for each crate, and the one-time token is
     revoked.
   - The tag is signed, and a GitHub release with notes exists.
6. ~~**Segment size decided** (open item 2), if the heal memory fix is ready.~~
   Moved to v0.0.2: the fix was not ready (A3). `CHANGELOG.md` keeps heal's
   memory growth and the provisional default as known issues.
7. **Phase closed.** ROADMAP marks Phase 1 done and Phase 2 current, and this
   plan moves to `plans/completed/` with results.

## Precedent to match

- Plan format: `plans/completed/erasure-leopard-check.md`.
- Release steps: `RELEASING.md` (written for exactly this).
- Decision record: `decisions/0006-erasure-code.md`.

## Steps

The code change (A0) lands first; then two tracks run in parallel while the M1
fuzzes. Once the campaign has started, anything that changes `crates/` waits
until after the tag, because it would reset the fuzz hours.

### Track A: before and during the campaign

| # | Step | Who | Time |
| --- | --- | --- | --- |
| A0 | Review and merge the pre-release code change (#6): S-001, W-001 and W-002, with regression tests | Claude, then maintainer | PR open |
| A1 | ~~Start the campaign on `e24f453`~~ Stopped (superseded). Start `fuzz/campaign.sh all 24` on `main` once A0 is merged | Maintainer | 30 min, then 24 h |
| A2 | ~~Memory test on the dev Mac~~ Done: heal grew with the file. The fix is the experiment on `exp/heal-reuse` | Maintainer | 3 min |
| A3 | ~~On `exp/heal-reuse`: `bench/heal-check.sh` (heal flat at 256 MiB and 1 GiB?), then `bench/segment-size.sh`~~ Done, not flat: see the results below. `segment-size.sh` was not run | Maintainer | 20 min |
| A4 | ~~Write the segment-size decision (0009) from A3~~ Moved to v0.0.2 | Claude | 30 min |
| A5 | ~~Code review and security review from the empty tree to `e24f453`~~ Done and logged; 6 findings; S-001, W-001 and W-002 fixed in A0. After A0 merges, run both again (`/review-since`, `/security-review-since`) over the new commits | Claude | 1 h |
| A6 | Freeze PR: spec wording, CI hash check of the vectors, `CHANGELOG.md`. Drafted; the `vectors` job in `ci.yml` is applied by hand | Claude, then maintainer (CI files are moved in by hand) | 1 h |
| A7 | GitHub release setup from `RELEASING.md`: `release` environment, `v*` tag ruleset, secret `maintainers` team, crates.io access for the org | Maintainer | 15 min |
| A8 | crates.io: sign in as `neoblast-dev`, verify the email, prepare a 1-day token | Maintainer | 10 min |

If a review finds something that must be fixed in `crates/` before release, the
fix lands, the affected targets are fuzzed again for 24 h, and the tag moves to
the fixed commit. That is what happened with S-001 (see the status note).

### A3 results

2026-10-14 on the reference Mac. Heal's peak memory in MiB, median of three
`bench/heal-check.sh` runs each, on `main` (`b3c3c63`) and on `exp/heal-reuse`
(`3dc5c83`). A 1 KB file peaks at 11.0 on both.

| File | Segment | `main` | `exp/heal-reuse` |
| --- | --- | --- | --- |
| 256 MiB | 1 MiB | 22.4 | 21.3 |
| 1 GiB | 1 MiB | 30.6 | 26.0 |
| 256 MiB | 4 MiB | 46.9 | 42.0 |
| 1 GiB | 4 MiB | 53.9 | 49.6 |

The fix lowers the median peak by 1–5 MiB (4–5 MiB in three of the four
cases), less than the spread between runs, which was up to 8.5 MiB. With it,
heal still grows by about 5 MiB (1 MiB segments) and 8 MiB (4 MiB segments)
between the two file sizes. At 4 MiB segments heal's working memory on a 1 GiB
file is about 39 MiB with the fix and 43 MiB without it, above ROADMAP item 2's
32 MiB either way.

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

- The open low findings: S-002, S-003 and S-004.
- Heal's memory growth: find what still grows (A3 results), then the
  segment-size decision (0009) and any change of default it calls for. The
  module doc of `crates/neoblast/src/stream.rs` says peak memory grows only by
  the manifest's shard hashes (about 1.6 MiB for a 1 GiB file at 1 MiB
  segments); heal grows more than that. Correct it with the fix.
- A new 24-hour campaign on the v0.0.2 code before tagging it.
- Phase 2 prerequisites:
  - accept decisions 0004 and 0005;
  - the libp2p vs iroh spike (open item 6), which needs a small VPS.
