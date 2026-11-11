# libp2p or iroh spike (ROADMAP item 6)

## Why

Phase 2 builds on one networking library, and ROADMAP item 6 picks it by a
rule: the library with the higher direct-connection rate between peers
behind NAT wins, and libp2p wins a tie. `plans/todo/p2p-library-spike.md`
fixes what that rule means in numbers. This directory is how the numbers
are made: two small programs, one per library, a harness that runs them
between two networks, and a script that applies the rule to the records.

It is throwaway measurement code, kept as the record of how the numbers
were made, like `bench/`. It is its own workspace, excluded from the root
one, so CI, `cargo deny` and Dependabot do not build or watch it.

## What is here

| Path | What it does |
| --- | --- |
| `common/` | An attempt's timings, and how B's echoes become its result (rules 2 and 3) |
| `libp2p/` | `spike-libp2p`: A, B, the relay and the health check, with libp2p 0.57.1 |
| `iroh/` | `spike-iroh`: A, B and the health check, with iroh 1.4.1 |
| `iroh-relay.toml` | The settings for iroh's own relay server, `iroh-relay` 1.4.1 |
| `relays.env.example` | What `run.sh` needs to know about the relays |
| `run.sh` | Runs a pair's attempts, A on one machine and B on the other |
| `SETUP.md` | Step 4 step by step: the VPS, each computer, a smoke test and the runs |
| `summary.py` | Applies the rule to the records; `test_summary.py` tests it |
| `records/` | One JSON line per attempt, committed after the runs (step 5) |

## How an attempt works

A and B are fresh processes with new identities on two networks. A connects
to its library's relay and reserves a place there. B is given only what
reaches A through the relay: A's circuit address (libp2p), or A's endpoint
ID and the relay's URL (iroh). B then sends A an echo every second.

- **Connected:** an echo comes back within 30 seconds of B starting.
- **Direct:** 30 seconds after the first echo, an echo comes back without
  the relay within 5 seconds. libp2p's echo is its ping, which runs on
  each connection and names the one it used; it counts as direct if that
  connection's `ConnectedPoint::is_relayed()` is false. iroh's echo is a
  QUIC stream; it counts as direct if the path iroh had selected was an IP
  path (`Path::is_selected()` and `is_ip()` in `Connection::paths()`) both
  when it was sent and when it came back. A relayed path still open beside
  a direct one does not matter.
- B also records whether it was direct at 10 and 60 seconds, the time to
  its first direct echo, and whether the connection was still carrying
  echoes at each reading.

libp2p reaches its relay over QUIC, so its hole punch is over UDP, as
iroh's is. iroh looks up no addresses (no DNS, pkarr or mDNS) and uses only
our relay; its port mapping (UPnP, PCP, NAT-PMP), on by default, is set by
`IROH_PORTMAP` and recorded in every record.

## Building and testing

```sh
cd spike/p2p
cargo build --release          # the toolchain is pinned in rust-toolchain.toml
cargo test                     # one machine: direct, relayed, IPv4 only
cargo test -- --ignored        # a relayed attempt at the full timings, ~75 s each
python3 -I test_summary.py
```

The tests run each library's relay on the loopback address and check:

- that A and B report **direct**, also with the relayed path still open;
- that they report **relayed** with hole punching off (libp2p without
  DCUtR; iroh with `clear_ip_transports()`, as iroh's own tests do);
- that an IPv4-only peer binds, advertises and connects over IPv4 only, and
  that a dual-stack one does use IPv6, so the first check means something;
- that a relayed connection still carries echoes at the last reading.

They cannot test a real NAT; the records from the real networks are the
evidence.

## Setting up the VPS

`SETUP.md` has every step, from an empty cloud account to the records;
this section and the next are the short version.

The VPS needs a public IPv4 address, a domain name pointing at it for
`iroh-relay`'s certificate, and these ports open:

| Port | For |
| --- | --- |
| TCP 80 | `iroh-relay`: the certificate's ACME challenge and its captive-portal check |
| TCP 443 | `iroh-relay`: relaying, over HTTPS |
| UDP 7842 | `iroh-relay`: QUIC address discovery |
| TCP and UDP 4001 | the libp2p relay, TCP and QUIC |

Only one relay can have TCP 443, so the libp2p relay uses 4001 (plan, step 2
notes, item 5). Then, on the VPS:

```sh
cargo install iroh-relay --version 1.4.1 --features server --locked
# Fill in the hostname and contact in a copy of iroh-relay.toml, then:
iroh-relay --config-path iroh-relay.toml

cd spike/p2p && cargo build --release
target/release/spike-libp2p relay --port 4001 --external-ip <VPS IPv4> \
    --key ~/p2p-spike-libp2p-relay.key
```

The libp2p relay prints its QUIC and TCP addresses; the QUIC one goes in
`relays.env`. Its key is kept, so the address stays the same between
evenings. Both relays serve anyone who finds them while they run, so stop
them between evenings.

## Running the pairs

Each end needs the release build and a `spike/p2p/relays.env` made from
`relays.env.example`, and each must reach the VPS over ssh with a key, as
`run.sh` passes messages through a directory there (`--mailbox`). On the
VPS itself, use `--mailbox local`.

A is the first-named end of each pair. Start A, then B:

```sh
bash spike/p2p/run.sh a --mailbox <vps>                       # the A end
bash spike/p2p/run.sh b --mailbox <vps> --pair <pair> --run 1 --first <library>
```

| Order | Pair | A on | B on | `--first` | Attempts |
| --- | --- | --- | --- | --- | --- |
| 1 | `home1-vps` (control) | home 1 | the VPS (`--mailbox local`) | `libp2p` | 20 |
| 2 | `cgnat-vps` (control) | the hotspot | the VPS (`--mailbox local`) | `iroh` | 20 |
| 3 | `home1-cgnat` | home 1 | the hotspot | `libp2p` | 50 |
| 4 | `home1-home2` | home 1 | home 2 | `iroh` | 50 |
| 5 | each pair, if both ends have IPv6 | | | | 10, with `--ip dual` on B |

- The controls come first (rule 5). If a library connects in fewer than 18
  of 20, fix the harness or the relay and run the controls again as
  `--run 2`. A blocked relay port is moved, never counted against the
  library (plan, step 2 notes, item 5).
- B writes `spike/p2p/records/run<n>-<pair>-<ip>.jsonl`. Stopping and
  restarting B carries on from the last attempt recorded; an attempt cut
  short is run again from the start, with fresh processes. A keeps running,
  answers only the newest request, and ignores any more than 2 minutes old. B sends A the IP mode and
  `IROH_PORTMAP` with each request, so the two ends always agree.
- A dropped ssh connection is retried; neither end needs restarting.
- B checks both relays before and after each attempt. If either fails
  (`relay-before`, `relay-after`), A cannot get ready (`a-not-ready`), or
  the mailbox stays out of reach (`mailbox`), the attempt is a harness
  error: it is recorded as such and run again (rule 10).
- `spike/p2p/logs/` holds each process's log. Logs can hold addresses: they
  are not committed.

## Records

One JSON line per attempt, harness errors included:

```json
{"alive_10": true, "alive_30": true, "alive_60": true, "attempt": 7,
 "connect_ms": 2140, "direct_10": true, "direct_30": true, "direct_60": true,
 "first_direct_ms": 3310, "ip": "v4", "iroh_portmap": "on",
 "library": "iroh", "outcome": "direct", "pair": "home1-cgnat",
 "relay_open_30": true, "run": 1, "try": 1, "version": "1.4.1"}
```

`outcome` is `direct`, `relayed`, `failed` or `harness-error` (with a
`reason`). Durations are from B's start. There are no dates or times,
addresses, hostnames, node IDs, carriers or providers (criterion 4).

## The summary

```sh
python3 -I spike/p2p/summary.py
python3 -I spike/p2p/summary.py --not-completed iroh    # rule 11, if declared
```

It prints each pair's rate, each library's rate as an exact fraction, the
controls, the IPv6 runs, the other figures in criterion 6, and the outcome.
A pair counts only once both libraries have all 50 attempts on it; as every
pair then has 50, the mean of the pairs' rates is also the rate over all
their attempts.
