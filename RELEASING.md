# Releasing

All four crates (`neoblast`, `neoblast-format`, `neoblast-crypto`,
`neoblast-erasure`) share one version and are released together (decision 0008).

## Every release

1. **Bump the version** through a pull request:
   - In `Cargo.toml`, set `version` under `[workspace.package]` and the three
     `version = "..."` entries under `[workspace.dependencies]`.
   - Update the three lock files and commit them with the bump. Release
     builds use `--locked`, the fuzzing script refuses a stale
     `fuzz/Cargo.lock`, and `bench/heal-heap.sh` builds with `--locked`.

     ```sh
     cargo update -w
     cargo update -w --manifest-path fuzz/Cargo.toml
     cargo update -w --manifest-path bench/heal-heap/Cargo.toml
     ```

2. **Tag** the release commit on `main`. It dates the version in
   `CHANGELOG.md` and logs the code and security reviews of every commit
   before it, so it changes only `docs/logs/`, `plans/` and `CHANGELOG.md`;
   a second agent reviews it before it merges, and the next reviews log it
   (W-037, S-032, S-033). Tag it on the UTC day `CHANGELOG.md` gives. First,
   at the top of the work tree, bring `main` up to date and check that the
   release commit's parent is the newest Tip in both review logs (the first
   two lines) and both baselines (the next two), that it has one parent, and
   that the last command prints nothing:

   ```sh
   git switch main && git pull
   grep -m1 '^| Tip |' docs/logs/code-review-log.md docs/logs/security-review-log.md
   grep '^<!-- baseline -->' docs/logs/code-review-log.md docs/logs/security-review-log.md
   git rev-list --parents -1 main
   git diff --no-renames --name-only main^ main -- . ':!docs/logs/' ':!plans/' ':!CHANGELOG.md'
   ```

   Then make the tag, SSH-signed and in UTC, and push it. The hygiene job
   and the Release workflow refuse a tag that is not the project's, not in
   UTC (S-015), not of a commit (S-017), or with anything but its name and
   its signature as message (S-022), so check it before pushing. Make it of
   the commit, never of an existing tag:

   ```sh
   TZ=UTC git tag -s v0.0.2 -m "v0.0.2"
   git for-each-ref refs/tags/v0.0.2 --format='%(objecttype) %(type) %(taggername) %(taggeremail) %(taggerdate:format:%z)'
   git cat-file tag v0.0.2
   git push origin v0.0.2
   ```

   The first check must print `tag commit neoblast maintainers <dev@neoblast.dev> +0000`.
   The second must show only the lines `object`, `type commit`, `tag v0.0.2`
   and `tagger neoblast maintainers <dev@neoblast.dev> … +0000`, a blank
   line, `v0.0.2` and an SSH signature block.
   If not, delete the tag with `git tag -d v0.0.2` and make it again. Never
   tag the old tag (`git tag -f v0.0.2 v0.0.2`): that makes a tag of a tag.

3. **Approve.** The `Release` workflow's `verify` job checks five things:
   - the tag is on `main`;
   - the tag is annotated, of a commit, by the project in UTC, with only its
     name as message;
   - the tag matches the version;
   - `cargo deny` passes, so an advisory published since the last merge
     stops the release (S-023);
   - tests and a publish dry run pass.

   Then the `publish` job waits for your approval in the `release` environment.
   The workflow does not wait for the hygiene job on the tag's push, so
   approve only once that job has passed. Approve it on the run's page, and
   it publishes all four crates with a short-lived token from crates.io
   trusted publishing.

4. **Publish the GitHub release** as `neoblast-dev`, only once the `publish`
   job has succeeded and crates.io shows all four crates at the new version,
   so the release never names crates that are not there yet. Its notes follow
   the previous release's: a short intro with the threat-model link and
   `cargo install neoblast --locked`, links to the four crates, then the
   version's section of `CHANGELOG.md`, with links pinned to the tag:

   ```sh
   gh release create v0.0.2 --repo neoblast-org/neoblast --verify-tag --title v0.0.2 --notes-file <notes>
   ```

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
