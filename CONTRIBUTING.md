# Contributing

Thanks for helping. A few rules keep the format stable and the code reviewable.

## Developer Certificate of Origin

Every commit must be signed off, certifying the [DCO](https://developercertificate.org/):

```sh
git commit -s
```

## Commit messages

Describe the change, sign off (`-s`), and keep links to private tools out of
messages. CI rejects commit messages that link to tool sessions.

## Before you open a pull request

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --all
cargo deny check
```

## Changing the byte format

Anything that changes bytes on disk or on the wire (the header, the hash tree,
caps, the erasure code) needs:

1. a decision record in `decisions/`,
2. an update to the matching file in `spec/`,
3. regenerated golden vectors (`NBL_UPDATE_VECTORS=1 cargo test -p neoblast --test golden_vectors`),
   with the change explained in the PR.

## Dependencies

Follow the blast-radius rule (see `decisions/README.md`): widely used crates are
fine; a crate with few dependents needs a written reason, an exact version pin,
and a wrapper trait.

## Code rules

- `unsafe` is forbidden in all first-party crates.
- No `unwrap`, `expect` or `panic` outside tests. Parsers must never panic on any input.
- Every fixed bug gets a regression test.
