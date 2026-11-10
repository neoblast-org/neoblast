# libp2p or iroh (ROADMAP open item 6)

Status: todo. Opened 2026-11-10, before any spike code or run. Owner:
maintainer, with Claude doing the code and docs. Step 1 done 2026-11-10
(#58): the maintainer approved the rule, readings (a) and (b) included.
Step 2's notes 2026-11-10, below. Step 3's code is in `spike/p2p/`.

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

## Notes

### 2026-11-10: the networks

The maintainer has more than one home network, so home–home runs and is a
decisive pair beside home–CGNAT (rule 4): two decisive pairs, as the time
estimate assumed. Rule 4 names one home–home pair, so two home networks are
used, "home 1" and "home 2"; home 1 is also the home end of home–CGNAT, so
the two pairs share one NAT and are not independent. Rule 7 still weighs
them equally. Where there is a choice, the two homes have different routers
and providers, so the pair is not one router's behaviour twice; that is a
preference, not a rule. The maintainer chooses them before the first run,
and the records name them only as home 1 and home 2 (criterion 4).

Two computers are enough, as pairs run one after another: one stays on
home 1 throughout, and the other is on home 2 for home–home and on the
phone's hotspot for home–CGNAT. **For the maintainer to confirm:** the
line after the rules says the result holds for the networks measured,
"one of each kind". With two homes that phrase is not exact, so decision
0013 states what was measured, one CGNAT network and two home networks.
This clarifies the sentence; it does not change what the rule decides.

### 2026-11-10: what step 2 found

Read before any spike code, as `decisions/README.md` asks: each library's
design docs, its code, the projects that use it and its issue tracker.
Versions, licences, MSRVs and the relay defaults below were checked in the
released crates' sources and on crates.io; the rest comes from the linked
pages, as read on this date, issue and pull-request states included.

| | libp2p | iroh |
| --- | --- | --- |
| Current release | 0.57.1 (2026-10-15) | 1.4.1 (2026-10-21) |
| Licence | MIT | MIT OR Apache-2.0 |
| `rust-version` | 1.88 | 1.91 |
| Reverse dependencies | 300 | 340 |
| Blast radius | medium | medium |
| Open advisories | none; seven fixed in 2026 | none; three fixed in 1.1.0 |

- **Licences:** both are on `deny.toml`'s list. Their dependency trees are
  checked by `cargo deny` in step 3 (criterion 6).
- **MSRV:** both releases need more than the workspace's 1.87. The spike's
  `rust-toolchain.toml` names 1.91 or later. Adopting either in Phase 2
  would mean raising the workspace's MSRV, which is Phase 2's to decide;
  decision 0013 records the cost. It is not a veto under rule 12. libp2p
  0.56.0 declares 1.83 but has an older relay (0.21); the spike uses each
  library's current release.
- **Advisories,** checked in RustSec and in each project's GitHub
  advisories. RustSec's entries for libp2p crates are old and patched.
  rust-libp2p published seven on GitHub in 2026, all high: four in
  gossipsub and two in rendezvous, which the spike does not use, and one
  in `libp2p-quic` (a remote panic), fixed in 0.13.1; libp2p 0.57.1 takes
  0.14 ([advisories](https://github.com/libp2p/rust-libp2p/security/advisories)).
  libp2p's QUIC transport also uses `quinn-proto`, which had several
  RustSec advisories in 2026, the latest fixed in 0.11.18; the spike's lock
  file has 0.11.19 or later. iroh's three, one high, are on GitHub only
  ([advisories](https://github.com/n0-computer/iroh/security/advisories)).
- **Blast radius:** both fall in "medium" (100 to 1,000 dependents):
  either would go behind our own types with an exact pin
  (`decisions/README.md`).

**How each one gets a direct path**

- libp2p: the relay v2 client and server, DCUtR and identify, as in the
  project's `examples/dcutr` and `examples/relay-server`. Over the relayed
  connection, the two peers swap their observed addresses and dial each
  other at the same moment, up to three times. The relayed connection stays
  open after a direct one is made
  ([DCUtR spec](https://github.com/libp2p/specs/blob/master/relay/DCUtR.md)).
  The example leaves AutoNAT out, and so does the spike; one user found it
  got in the way of hole punching
  ([autonomi#1640](https://github.com/maidsafe/autonomi/pull/1640)).
- iroh: since 0.96, one QUIC connection holds the relay path and direct
  paths together (QUIC multipath), and the address exchange and hole
  punching run inside QUIC
  ([0.96](https://iroh.computer/blog/iroh-0-96-0-the-quic-multipaths-to-1-0)).
  The relay tells each peer its public address (QAD, which replaced STUN;
  [QAD](https://iroh.computer/blog/qad)). By default iroh prefers IPv6
  paths to IPv4.

**What others have measured**

- libp2p: a 2022–23 campaign of 4.4 million attempts from 85,000 networks
  found 70% ± 7.1% of hole punches succeed, with TCP and QUIC alike,
  counting only attempts where the relay reservation and address discovery
  had worked ([arXiv 2510.27500](https://arxiv.org/html/2510.27500v1)). A 2026 follow-up
  found about 83% behind port-restricted NATs and 40% behind symmetric
  ones, the latter in doubt
  ([ProbeLab](https://probelab.io/blog/can-libp2p-punch-through-nats/)).
  Neither covers mobile networks. Of the production users found, Pathfinder
  reported "limited success" and dropped DCUtR
  ([pathfinder#1070](https://github.com/software-mansion/pathfinder/issues/1070)),
  Ceramic turned relay and DCUtR off
  ([rust-ceramic#323](https://github.com/ceramicnetwork/rust-ceramic/pull/323)),
  and Forest uses UPnP, with no relay or DCUtR
  ([discovery.rs](https://github.com/ChainSafe/forest/blob/main/src/libp2p/discovery.rs)).
- iroh: n0 says about 9 in 10 connections go direct, with no date or method
  ([FAQ](https://docs.iroh.computer/about/faq)). Delta Chat uses iroh 0.35
  ([Cargo.toml](https://github.com/chatmail/core/blob/main/Cargo.toml)),
  and a case study on n0's blog says 90.7% of Strada's transfer volume
  stayed off central servers
  ([Strada](https://www.iroh.computer/blog/strada-direct-file-transfer));
  neither splits direct from relayed. No independent measurement was found.
  iroh's own NAT tests skip a symmetric NAT against a port-restricted or
  symmetric one as "not yet passing (and likely can't without port
  guessing)" (`tests/patchbay/nat.rs` in 1.4.1).
- So if the carrier's NAT is symmetric, both libraries may mostly stay
  relayed on home–CGNAT, as the rule section foresaw, and home–home may
  carry most of the difference. Nothing here changes the rule; the spike's
  own records decide.

**What step 3 must get right**

1. libp2p's direct reading (rule 3): `libp2p-stream` (0.5.0-alpha) opens a
   stream on any connection to the peer, which may be the relayed one. The
   reading needs its own behaviour that sends on one connection by its ID,
   and `ConnectedPoint::is_relayed()` to tell that the connection is not
   relayed (it checks the dialled or the local listen address for
   `/p2p-circuit`).
2. **For the maintainer to confirm:** iroh's direct reading (rule 3).
   iroh 1.x has no connection type, so no "direct" or "mixed" label. The
   reading is that the path in `Connection::paths()` for which
   `is_selected()` is true is also `is_ip()`, with a relay path still open
   beside it or not, as rule 3 says. Old iroh's "mixed" meant "both a UDP
   and a relay connection are used"
   ([0.35](https://docs.rs/iroh/0.35.0/iroh/endpoint/enum.ConnectionType.html)),
   so this is no looser than it. Criterion 3's mixed case becomes a test of
   a selected IP path with the relay path still open.
3. The libp2p relay's defaults end a circuit after 2 minutes or 128 KiB;
   both are raised (criterion 3). Since relay 0.22 the server stops
   advertising the relay protocol until it has a confirmed external
   address, so it is given the VPS's
   ([#6154](https://github.com/libp2p/rust-libp2p/pull/6154)).
4. `iroh-relay` ships with QUIC address discovery off; it is turned on, or
   iroh's peers do not learn their public addresses from it.
5. Both relays run at once, as the libraries alternate, so they cannot both
   have TCP 443 on one address. Proposed for the matrix: `iroh-relay` on
   TCP 443 for its HTTPS, with its address discovery on its default UDP
   port, 7842; libp2p's relay on one other port for TCP and UDP. Either
   library then reaches its relay off UDP 443. The relay's port decides
   only whether a peer reaches the relay, not the direct path between the
   peers. The maintainer confirms this in step 3's review. A network may
   block the uncommon port and so fail only libp2p's controls (rule 5),
   which, never passing, would make libp2p not completed (rule 11) for a
   reason that is not NAT traversal. So a blocked relay port is fixed by
   moving that relay to another port and rerunning the controls; it never
   counts against the library. Home 2 has no control pair, so a blocked
   port there shows only as repeated harness errors (item 9). The 443
   question in criterion 6 is checked once for each library, outside the
   matrix.
6. libp2p: each peer waits for identify to finish with the relay before it
   listens or dials, as `examples/dcutr` does; otherwise DCUtR may have no
   addresses to try (open PR
   [#5583](https://github.com/libp2p/rust-libp2p/pull/5583)).
7. IPv4 only (rule 6): libp2p listens on `/ip4/` addresses only; iroh clears
   its IP transports and binds `0.0.0.0` alone. iroh's address filter does
   not reach the candidates its hole punching sends
   ([iroh#4399](https://github.com/n0-computer/iroh/issues/4399)), so the
   test in criterion 3 checks what is sent, not only what is bound.
8. Relayed-only checks (criterion 3), by each library's option: libp2p
   without DCUtR; iroh with `clear_ip_transports()` on one endpoint, as
   iroh's own test `endpoint_two_relay_only_no_ip` does. That removes the
   endpoint's IP transports rather than only its hole punching, which
   leaves it no direct path, as criterion 3 wants. The check confirms the
   relay path still works; a firewall rule only if it does not.
9. **For the maintainer to confirm**, a reading of rule 10: if A never
   reaches its relay or never gets its reservation, the script cannot
   start B, so the attempt is a harness error and is rerun, for either
   library alike. It would favour the library that fails there more often:
   a user of rust-libp2p reported about 1% of reservations accepted
   without the event that says so, on the 0.56 line; it may still occur
   ([#6141](https://github.com/libp2p/rust-libp2p/issues/6141)). The other
   reading counts it as failed. Either way, the records log each harness
   error by library, and decision 0013 reports the counts.
10. Issues the records may show: on one carrier's CGNAT, iroh's direct path
    dropped to the relay 48 to 75 seconds after it was made, for 1 to 26
    seconds, then was punched again; the reporter puts it down to the
    carrier's fixed UDP mapping timeout, which would hit any library
    ([iroh#4424](https://github.com/n0-computer/iroh/issues/4424)). The
    readings at 10, 30 and 60 seconds may catch it. An iroh peer
    behind a filtering NAT stayed relayed when dialled with its full
    address, but went direct when dialled with its ID alone
    ([iroh#4658](https://github.com/n0-computer/iroh/issues/4658)); rule 1
    gives B the node ID and the relay URL only.

### 2026-11-10: step 3, before any run

Rule 3 asks for the exact calls to be named here. Both programs share one
judge, `spike/p2p/common`, so the libraries differ only in what counts as
an echo that went direct:

- **libp2p:** B's echo is libp2p's ping, sent every second on each
  connection with a 5-second timeout. `ping::Event` names the connection
  each round trip used; it went direct if that connection's
  `ConnectedPoint::is_relayed()` is false, which is rule 3's named
  connection with no `/p2p-circuit`.
- **iroh:** B's echo is 32 bytes on a new QUIC stream, sent back by A,
  every second with a 5-second timeout. It went direct if, in
  `Connection::paths()`, the path with `is_selected()` was `is_ip()` both
  when it was sent and when it came back.
- **"At that moment":** rule 3's reading is met by an echo that went
  direct and came back within 5 seconds of the reading's time, the echo
  timeout, so an echo sent at the reading counts. The readings at 10 and
  60 seconds use the same window.

The tests on one machine pass for both: direct with the relayed path still
open, relayed with hole punching off, IPv4 only, and, at the full timings,
a relayed connection still carrying echoes at the last reading.

**For the maintainer to confirm before the first run:**

1. **iroh's port mapping.** iroh asks the router for a port mapping (UPnP,
   PCP, NAT-PMP) by default; libp2p's hole punching does not, and its
   separate UPnP behaviour would not help, as DCUtR uses only the addresses
   peers observe. Rule 1 asks for each library's documented NAT traversal,
   so the proposal is iroh's default, on (`IROH_PORTMAP=on`), recorded in
   every record. Off would measure hole punching alone. Either is set
   before the first run and kept for all of them.
2. **libp2p over QUIC.** libp2p reaches its relay over QUIC, so DCUtR
   punches over UDP, as iroh does. The examples show TCP; the 2022–23
   campaign found the two alike.

**`cargo deny`, run once with the repository's `deny.toml` against
`spike/p2p/Cargo.toml` (criterion 6).** Sources are fine; duplicate
versions only warn. It fails on:

- licences outside `deny.toml`'s list, all three through iroh alone:
  `attohttpc` (MPL-2.0), which iroh's default port mapping pulls in
  through `portmapper` and `igd-next`; and `webpki-roots` and
  `webpki-root-certs` (CDLA-Permissive-2.0, the bundled root
  certificates), through `iroh-relay`, `iroh-dns` and `reqwest`;
- one advisory for both: `paste` is unmaintained (RUSTSEC-2024-0436),
  through each library's network-interface crates.

**For the maintainer, under rule 12:** whether these licences rule iroh
out, or are accepted, perhaps with port mapping left out of a build that
adopts it. The unmaintained `paste` affects both alike. Nothing here is
decided by the spike's code; decision 0013 records the answer.

**Two readings of the rule in `summary.py`,** for the maintainer to
confirm:

- Rules 11 and 12 come before rule 13. A library not completed leaves
  every decisive pair short of its 50 attempts, so if rule 13 came first,
  rule 11 could never choose the other library. With no declaration, no
  complete decisive pair leaves item 6 open.
- Criterion 5 asks for a test where the mean of the pairs' rates differs
  from the rate pooled over their attempts. Once rule 7 counts only pairs
  with all 50 attempts each, the two are always equal; they differ only if
  an incomplete pair is pooled, which the tests check is not done.

`run.sh` makes A the first-named end of each pair: home 1 in both decisive
pairs and in `home1-vps`, the hotspot in `cgnat-vps`. `home1-cgnat` starts
with libp2p and `home1-home2` with iroh (rule 8); the order is in
`spike/p2p/README.md`.
