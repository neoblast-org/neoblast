# Releasing

All four crates (`neoblast`, `neoblast-format`, `neoblast-crypto`,
`neoblast-erasure`) share one version and are released together (decision 0008).

## Every release

1. **Bump the version** through a pull request:
   - In `Cargo.toml`, set `version` under `[workspace.package]` and the three
     `version = "..."` entries under `[workspace.dependencies]`.
   - Update both lock files and commit them with the bump. Release builds use
     `--locked`, and the fuzzing script refuses a stale `fuzz/Cargo.lock`.

     ```sh
     cargo update -w
     cargo update -w --manifest-path fuzz/Cargo.toml
     ```

2. **Tag** the merged commit on `main` with a signed tag, and push it:

   ```sh
   git switch main && git pull
   git tag -s v0.0.2 -m "v0.0.2"
   git push origin v0.0.2
   ```

3. **Approve.** The `Release` workflow's `verify` job checks three things:
   - the tag is on `main`;
   - the tag matches the version;
   - tests and a publish dry run pass.

   Then the `publish` job waits for your approval in the `release` environment.
   Approve it on the run's page, and it publishes all four crates with a
   short-lived token from crates.io trusted publishing.

## One-time setup

### GitHub

1. **Settings → Environments → New environment**, named `release`:
   - add yourself (the `neoblast-dev` account) as a required reviewer, and allow
     self-review;
   - under deployment branches and tags, allow only tags matching `v*`.
2. **Settings → Rules → Rulesets → New tag ruleset** for `v*`: restrict creation,
   update and deletion to organisation admins.
3. In `neoblast-org`, create a **secret** team named `maintainers` containing
   `neoblast-dev`. Under **Settings → Third-party access**, approve the crates.io
   OAuth application so crates.io can see the team.

### crates.io: the first release, by hand

crates.io only allows trusted publishing for crates that already exist, so
v0.0.1 is published from your machine.

1. **Sign in.** In a private browser window, sign in to crates.io with GitHub as
   `neoblast-dev`, and verify the email `dev@neoblast.dev` under Account Settings.
2. **Create an API token** with:
   - scopes `publish-new`, `publish-update` and `change-owners`;
   - crates `neoblast*`;
   - an expiry of 1 day.
3. **Publish** from a clean checkout of `main` at the release commit. The
   workspace version is already 0.0.1, so no bump is needed for this one:

   ```sh
   read -rs CARGO_REGISTRY_TOKEN && export CARGO_REGISTRY_TOKEN   # paste; not saved to history
   cargo publish --workspace --locked --dry-run
   cargo publish --workspace --locked
   ```

4. **Configure each of the four crates** on crates.io:
   - Under **Settings → Trusted Publishing**, add a GitHub publisher with owner
     `neoblast-org`, repository `neoblast`, workflow `release.yml` and
     environment `release`.
   - Add the team as an owner: `cargo owner --add github:neoblast-org:maintainers <crate>`.
5. **Clean up.** Run `unset CARGO_REGISTRY_TOKEN`, then revoke the token under
   Account Settings → API Tokens.
6. **Tag** the commit as v0.0.1 and push the tag, as in step 2 above. The Release
   workflow runs, but v0.0.1 is already published, so **reject** the approval
   request for that one run.

From v0.0.2 on, use "Every release" above. No token is needed again.
