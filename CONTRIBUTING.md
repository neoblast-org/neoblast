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

The v1 format is frozen (`spec/header.md`): its golden vectors never change, and
CI checks their hashes. Anything that changes bytes on disk or on the wire (the
header, the hash tree, caps, the erasure code) is a new format and needs:

1. a new magic version byte, a new suite byte or, for links, a new link version byte,
2. a decision record in `decisions/`,
3. an update to the matching file in `spec/`,
4. a new golden-vector file beside the v1 ones (see `spec/test-vectors/README.md`),
   with the change explained in the PR.

## Dependencies

Follow the blast-radius rule (see `decisions/README.md`): widely used crates are
fine; a crate with few dependents needs a written reason, an exact version pin,
and a wrapper trait.

## Code rules

- `unsafe` is forbidden in all first-party crates.
- No `unwrap`, `expect` or `panic` outside tests. Parsers must never panic on any input.
- Every fixed bug gets a regression test.
