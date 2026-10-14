# 0008: Crate names and publishing

- Status: accepted
- Date: 2026-10-14

## Context and problem statement

The `neoblast` crate depended on three workspace crates (`nbl-format`,
`nbl-crypto`, `nbl-erasure`) that were marked `publish = false`, so
`cargo publish` could not succeed. A choice was needed before v0.0.1:

- publish all four crates under `neoblast-*` names;
- publish them under the existing `nbl-*` names;
- merge everything into one `neoblast` crate.

## Decision outcome

**Publish four crates: `neoblast`, `neoblast-format`, `neoblast-crypto` and
`neoblast-erasure`.** The renamed directories are `crates/neoblast-*`, and the
Rust paths are `neoblast_format::` and so on.

- The `neoblast-` prefix reserves the project's namespace on crates.io and makes
  each crate's origin obvious. Look-alikes such as `nbl-*` mean nothing to a reader.
- `neoblast-format` stays small (no I/O, no crypto) so later browser and Python
  bindings can depend on it alone (Phase 5).
- All four share one version from `[workspace.package]` and are released
  together with `cargo publish --workspace`.

Wire identifiers are unchanged: `nbl://` links, the `nbl/v1/...` key-derivation
contexts and the `NBL\x01` magic are part of the format, not crate names.

### Ownership and how releases happen

- crates.io owner: the `neoblast-dev` account, plus a team in the `neoblast-org`
  GitHub organisation as a second owner. crates.io owner lists are public, so
  personal accounts are never added.
- Releases run from `.github/workflows/release.yml` when a `v*` tag is pushed. It
  uses crates.io trusted publishing (a short-lived token minted for that run) in a
  `release` environment that needs a maintainer's approval.
- crates.io only allows trusted publishing for crates that already exist, so the
  first version of each crate is published once by hand with a short-lived,
  narrowly scoped token. The token is revoked straight afterwards. The steps are
  in `RELEASING.md`.

### Consequences

- Good: `cargo publish --workspace --dry-run` packages and verifies all four crates.
- Good: no long-lived registry token exists after the first release.
- Bad: four crates appear on crates.io where one would have been simpler. They
  are always released together, so this is cosmetic.
- Neutral: the directory rename shows up in `git log --follow`, not plain
  `git log`.

## Considered options

- **`nbl-*` names.** Shorter, but they don't identify the project and leave the
  `neoblast-*` names open to anyone.
- **One crate.** Simplest to release, but bindings would pull in the CLI and
  everything else.
