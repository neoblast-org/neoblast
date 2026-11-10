# libp2p or iroh (ROADMAP open item 6)

Status: todo. Opened 2026-11-10, before any spike code or run. Owner:
maintainer, with Claude doing the code and docs.

## Why now

Phase 2 builds the swarm, NAT traversal and shard discovery, and all three
rest on the networking library. ROADMAP item 6 must be settled "Before Phase
2", by its rule: **a two-evening spike each across a home router, carrier-grade
NAT and a VPS; the higher direct-connection rate wins; on a tie, libp2p
(built-in Kademlia).** This plan fixes what that rule means in numbers, before
any result is seen, as `plans/completed/segment-size-decision.md` did for
item 2.

## The rule, fixed before any run

These definitions are fixed when the maintainer approves this plan. A change
after any run has been seen is recorded here with its date and what had been
seen, as decision 0009 did for its memory figure. Notes added later (step 2's
reading, the networks chosen) go under dated headings below and do not
change this section.

Two readings of item 6 are the maintainer's to approve, as they narrow
its words: **(a)** only pairs with both ends behind NAT decide, and the VPS
pairs are controls (rule 4); **(b)** a tie means rates under 5.0 points
apart, not equal rates (rule 9). One likely result should be known before
approving: if the carrier's NAT defeats hole punching for both, both rates
are near 0, it is a tie, and libp2p wins by default.

1. **Attempt:** two fresh processes, A and B, each with a new random
   identity and new sockets, on two networks. A reserves a slot on the
   relay; B is given only what reaches A through the relay: A's
   relay-circuit address for libp2p, A's node ID and the relay's URL for
   iroh. Neither is given the other's own address. iroh's discovery
   services (DNS and pkarr) and its default relays are turned off. B then
   sends A an echo every second.
2. **Connected:** an echo has come back, through the relay or not, within
   30 seconds of B starting. Otherwise the attempt is **failed**.
3. **Direct:** one reading, 30 seconds after the first echo came back: the
   path the library uses at that moment carries an echo without the relay.
   A relayed path still open beside it does not matter, for either library.
   For libp2p, the reading opens an echo stream on a named connection
   between A and B whose remote address has no `/p2p-circuit`, and it
   succeeds. For iroh, an echo succeeds and the path iroh has selected for
   the connection is direct, which counts whether iroh calls the
   connection direct or mixed. The exact calls are named in step 3, before
   any run. A connected attempt that is not direct at that reading is
   **relayed**, whatever it did before or after.
4. **Decisive pairs:** those where both ends are behind NAT: home–CGNAT,
   and home–home if a second home network is available. Pairs with the VPS
   (home–VPS, CGNAT–VPS) are controls: the VPS has a public address, so
   either library reaches it without a hole punch. "Home" is a consumer
   router with NAT; "CGNAT" a mobile carrier's network, through a phone's
   hotspot.
5. **Controls first:** each library runs 20 attempts on each control pair
   before any decisive run. A library that connects in fewer than 18 of
   20 on a control pair has a broken harness or relay: it is fixed and
   the controls run again. One that never passes is not completed (rule
   11). Controls do not count.
6. **IPv4 only** for the decisive runs: IPv6 with global addresses needs
   no hole punch, which item 6 is about. Both programs bind IPv4 sockets
   only and advertise no IPv6 address, which a test checks. A network with
   no IPv4 (a hotspot with only IPv6 and NAT64, for instance) cannot run a
   decisive pair. Where both ends have IPv6, a separate run of 10 attempts
   per library records it; it does not count.
7. **Rate:** for each decisive pair, direct attempts divided by all
   attempts, failed ones included. A library's **direct-connection rate** is
   the mean of its decisive pairs' rates, so each pair counts equally. A
   pair counts only once both libraries have completed all 50 attempts on
   it; an interrupted pair is resumed until it has. If only one decisive
   pair completes, the rate is that pair's.
8. **Count and order:** 50 attempts per decisive pair per library. The
   libraries alternate attempt by attempt; which goes first alternates from
   pair to pair. After both processes exit there is a 10-second pause, so a
   NAT mapping from one attempt is less likely to help the next.
9. **Tie:** library rates less than 5.0 percentage points apart; then
   libp2p wins. At 5.0 points or more, the higher rate wins. The threshold
   is a convention, not a test of significance: with 50 attempts a pair, a
   gap of 5 or 6 points could be chance either way. The summary compares
   exact fractions, not floating point.
10. **Harness errors** are rerun and logged as such, not counted: the
    script failing before both processes have started, or the relay or VPS
    failing its health check, a connection to each relay made by the
    harness just before and just after each attempt. Anything else a
    library does after both processes have started, a crash included,
    counts.
11. **Not completed:** the maintainer declares a library not completed if
    its spike cannot be made to run its documented NAT traversal within its
    two evenings; the other library is chosen. If neither completes, item 6
    stays open and the maintainer decides how to go on.
12. **Vetoes:** a licence the project cannot accept, or an advisory that
    `cargo deny` reports, is raised with the maintainer before the
    decision. If the maintainer rules one library out for it, the other is
    chosen, unless it was not completed (rule 11); if both are out, either
    way, item 6 stays open. Nothing else in "Also recorded" changes the
    outcome.
13. **No decisive pair:** if no decisive pair can be run, item 6 stays open
    and the maintainer decides how to go on.

The result holds for the networks measured, one of each kind; the decision
record says so.

## Success criteria

1. **Rule first:** the definitions above are approved before any spike
   code runs between two networks, and any later change is recorded with
   its date.
2. **A fair spike:** two small programs in `spike/p2p/`, one per library,
   each using the library's documented NAT traversal with its own relay,
   both relays self-hosted on the same VPS. Exact versions are pinned in
   the spike's own `Cargo.lock`, and its own `rust-toolchain.toml` names
   the toolchain both need. Each fits in two evenings of work (rule 11).
3. **Tested before it is run:** on one machine, each program reports
   "direct" for a direct connection, also with a relayed connection still
   open beside it (and, for iroh, when it calls the connection mixed); and
   "relayed" with its hole punching turned off (by the library's option
   where there is one, by a firewall rule where not). Each binds and
   advertises IPv4 only when asked to. Both relays are set so that a
   relayed connection lasts at least 120 seconds and carries the echoes, so
   neither library's relay cuts an attempt short.
4. **Records:** one JSON line per attempt in `spike/p2p/records/`, with the
   library and its version, the pair, IPv4 or IPv6, the outcome, the time
   to a direct path if any, whether it was direct at 10 and 60 seconds as
   well as 30, and the run's and the attempt's numbers. **No dates or times,
   IP addresses, hostnames, node identities, carrier or provider names:**
   the repository is public, and times or dates of evening runs would show
   the maintainer's time zone.
5. **Summary:** `spike/p2p/summary.py` applies the rule to the records
   alone and prints each pair's rate, each library's rate and the outcome.
   A test runs it on made-up records: a win each way, a gap under 5.0
   points, a gap of exactly 5.0, a failed attempt, a harness error, a pair
   not yet complete, one decisive pair and two, an attempt direct at 10
   seconds but relayed at 30, and rates whose mean of pairs differs from
   the pooled rate. The real networks cannot be tested; the records are
   the evidence.
6. **Also recorded:** each library's relay fallback rate, its median time to
   a direct path, and its rates at 10 and 60 seconds; whether it can listen
   on UDP 443 and relay over TCP 443, as the threat model asks; its licence
   and `cargo deny` result, run once with the repository's `deny.toml`
   against the spike's manifest; its direct dependency count and reverse
   dependencies on crates.io (the blast-radius rule); and the minimum Rust
   version it needs, against the workspace's 1.87.
7. **Kept apart:** `spike/p2p` is excluded from the workspace in the root
   `Cargo.toml`, as `fuzz` and `bench/heal-heap` are, and has
   `publish = false`. CI, `cargo deny` and Dependabot, which read the root
   workspace, do not build or watch it. It is kept as the record of how the
   numbers were made, as `bench/` is; as throwaway code it is not held to
   the crates' rules, but it builds without warnings.
8. **Decided:** decision 0013 applies the rule as written, and ROADMAP
   item 6 is settled, or stays open as rules 11 to 13 say; either way this
   plan moves to `plans/completed/`. Choosing otherwise
   would take a later decision that supersedes 0013.
9. Nothing under `crates/` changes; CI, the golden vectors and
   `cargo deny` stay green.

## Precedent to match

- The plan: `plans/completed/segment-size-decision.md` (a measured
  question, its rule fixed and dated before the final run) and
  `plans/completed/erasure-leopard-check.md` (a spike that settles a
  ROADMAP item).
- The decision: `decisions/0009-segment-size.md` (method, findings,
  outcome) and `decisions/0006-erasure-code.md`.
- Records and their summary: `fuzz/campaigns/` and
  `fuzz/campaign.sh summary`; a separate workspace: `bench/heal-heap/`.

## Steps

Each step is its own branch and pull request, reviewed by a second agent
before it is handed over.

| # | Step | Touches | Who |
| --- | --- | --- | --- |
| 1 | This plan; the maintainer approves the rule and says whether a second home network is available, which is then written here, dated | `plans/` | Claude, then maintainer |
| 2 | Retrieve: how two or three projects that use each library traverse NATs, and their issue trackers; both licences. Notes go here under a dated heading | `plans/` | Claude |
| 3 | The two spike programs, the relay setup (the rust-libp2p relay server, `iroh-relay`), `run.sh`, `summary.py` and its test, with criterion 3's checks | `spike/p2p/`, root `Cargo.toml` (`exclude`) | Claude, then maintainer |
| 4 | Set up both relays on the VPS; run the matrix; copy the records from the Mac and the VPS into a branch | nothing tracked until step 5 | Maintainer |
| 5 | The records; the summary; decision 0013 applies the rule; ROADMAP item 6 settled; this plan to `completed/` | `spike/p2p/records/`, `decisions/`, `ROADMAP.md`, `plans/` | Claude, then maintainer |

## What the maintainer needs

- A VPS with a public IPv4 address, root access and UDP and TCP 443 free,
  for the two relays and as one end of the control pairs.
- A domain name for `iroh-relay`'s TLS certificate. It is never recorded
  in the repository, but certificate transparency logs make the relay's
  hostname public, so the test's relays are not secret; no third party runs
  them or sees the peers' addresses through them.
- Two computers, such as the reference Mac and the M1: one on a home
  network, one on a phone's hotspot that uses a mobile carrier, for the
  home–CGNAT pair. A second home network adds the home–home pair.
- Time for the runs. An attempt takes at most about 100 seconds (30 to
  connect, 60 after the first echo, the 10-second pause), most about 75.
  Assuming 2 decisive pairs at 50 attempts and 2 control pairs at 20, for
  each library, and IPv6 runs of 10 on all 4 pairs, that is about 360
  attempts: about 7.5 hours, 10 at worst, over several evenings. `run.sh`
  runs unattended and can resume.

## Also out of scope

Kademlia, the Mainline DHT, placement and shard transfer: the spike
measures only how often two peers get a direct path. Those come after the
decision, in Phase 2's own plan.
