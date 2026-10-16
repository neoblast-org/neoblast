# Fuzzing

## Why

Every Neoblast node parses bytes from strangers: links, headers, manifests and
shards. A fuzzer feeds the code millions of malformed inputs per minute and
uses code coverage to steer toward paths no hand-written test reaches. It finds
two kinds of bug:

- **Crashes.** A panic on one crafted input would let anyone knock storage nodes
  offline, which is a censorship tool in its own right.
- **Wrong answers.** `open_hostile_shards` and `erasure_restore` check results as
  well as crashes: anything accepted must be the original data, and enough good
  shards must always be enough.

The Phase 1 gate asks for 24 hours per target with no crash before the format is
frozen and v0.0.1 is published. CI's nightly job only runs 10 minutes per target.

## Targets

| Target | Feeds |
| --- | --- |
| `header_decode` | arbitrary bytes to `Header::decode` |
| `link_parse` | arbitrary text to the `nbl://` link parsers |
| `manifest_decode` | arbitrary bytes to `Manifest::decode` |
| `open_hostile_shards` | a real file whose shards are missing, damaged or replaced, to `open` and `heal` |
| `erasure_restore` | any shard subset and any shard lengths to the erasure decoder |

## Campaigns and records

`campaign.sh` runs targets and writes one JSON record per run into `fuzz/runs/`,
which git ignores. Copying a record into `fuzz/campaigns/` and committing it
makes it part of the history. A record holds the target, result (`clean`, `stopped`,
`crash` or `error`), start and end times in UTC, hours run, executions,
coverage, corpus size, the commit and toolchain, and the OS, architecture and CPU
model. It holds no host name, user name or file path.

```sh
fuzz/campaign.sh all 24          # all targets in parallel for 24 hours
fuzz/campaign.sh link_parse 8    # one target for 8 hours
fuzz/campaign.sh summary         # hours per target, and whether the gate is met
```

- **Hours add up across runs** (several nights are fine) as long as the code under
  test is the same. Each record has a `code_id`: a hash of `crates/`, both
  `Cargo.toml` files, both `Cargo.lock` files and the target's source. The
  summary counts runs in `fuzz/campaigns/` and `fuzz/runs/` whose `code_id`
  matches the current checkout. Hours are wall-clock time: runs of one target
  that overlapped count once.
- **One campaign at a time.** The script refuses to start while another
  campaign is running on the same clone. If a campaign was killed and the
  refusal names a process that is not a campaign, delete
  `fuzz/runs/.campaign.pid`.
- **Ctrl-C stops early**, and the hours so far are recorded as `stopped`.
- The script refuses to run with uncommitted changes in `crates/`, `fuzz/`,
  `Cargo.toml` or `Cargo.lock`, so every record matches a real commit.
- Each target keeps its corpus in `fuzz/corpus/<target>`, which is not committed.
  Later runs start from where earlier ones stopped.

## Setting up a dedicated fuzzing Mac

Any Apple Silicon Mac works; an M1 runs all five targets at once comfortably.
The machine only needs a read-only clone: no SSH keys, no GitHub login and no
commit identity.

```sh
xcode-select --install                                   # compiler and git
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustup toolchain install nightly
cargo +nightly install cargo-fuzz --version 0.13.2 --locked
git clone https://github.com/neoblast-org/neoblast.git ~/neoblast-fuzz
```

For each campaign:

```sh
cd ~/neoblast-fuzz && git pull
fuzz/campaign.sh all 24
```

- Keep the Mac on power with the lid open, or use an external display. The
  script stops idle sleep, but closing the lid still sleeps a laptop.
- Leave the Terminal window open. To be able to close it, run the campaign
  inside `tmux` (`brew install tmux`), or inside `screen` if your macOS still
  has it.

**Getting records into the repository.** New records land in
`~/neoblast-fuzz/fuzz/runs/` on the fuzzing Mac:

1. Copy them to `fuzz/campaigns/` in your development clone. AirDrop, a shared
   folder or `scp` all work.
2. Commit them there through a pull request as usual.

Because the fuzzing Mac never writes into tracked folders, `git pull` keeps
working there.

## If a target crashes

1. The input is saved in `fuzz/artifacts/<target>/`. Reproduce it with
   `cargo +nightly fuzz run <target> fuzz/artifacts/<target>/<file>`.
2. Treat it as a security issue (see `SECURITY.md`). Don't publish the input
   before the fix is merged.
3. Fix it and add a regression test (`crates/neoblast/tests/review_regressions.rs`).
   The fix changes the `code_id`, so the 24-hour count for that target restarts.
4. Commit the run's record anyway: a `crash` record is part of the history.
