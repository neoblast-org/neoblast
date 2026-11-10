//! Criterion 3 on one machine: iroh's in-process relay on the loopback
//! address, A and B as in a real attempt, with shortened timings.

use std::time::Duration;

use iroh::RelayMap;
use iroh::test_utils::run_relay_server;
use iroh::tls::CaTlsConfig;
use p2p_spike_common::{Attempt, Outcome, Report, Timings};
use p2p_spike_iroh::{LIBRARY_VERSION, PeerOptions, Seen, Shown, dial, listen, probe};

fn opts(ipv4_only: bool, direct_paths: bool, timings: Timings) -> PeerOptions {
    PeerOptions {
        ipv4_only,
        direct_paths,
        portmap: false,
        relay_tls: Some(CaTlsConfig::insecure_skip_verify()),
        timings,
    }
}

async fn attempt(a: PeerOptions, b: PeerOptions) -> (Report, Seen, Shown) {
    let timings = b.timings;
    let (relays, url, _server) = run_relay_server().await.unwrap();
    let (id, shown, a_task) = listen(relays.clone(), a).await.unwrap();
    let (report, seen) = dial(id, url, relays, b, Attempt::new(timings))
        .await
        .unwrap();
    a_task.abort();
    (report, seen, shown)
}

const T: Timings = Timings::TEST;

#[tokio::test(flavor = "multi_thread")]
async fn hole_punching_gives_direct_with_the_relay_path_still_open() {
    // The relay path open beside a selected direct one is what old iroh
    // called mixed; rule 3 counts it as direct.
    let (report, seen, _) = attempt(opts(true, true, T), opts(true, true, T)).await;
    assert_eq!(report.outcome, Outcome::Direct, "{report:?}");
    assert_eq!(report.relay_open_at_30, Some(true), "{report:?}");
    assert!(!seen.direct_paths.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn without_direct_paths_it_stays_relayed() {
    let (report, seen, _) = attempt(opts(true, false, T), opts(true, true, T)).await;
    assert_eq!(report.outcome, Outcome::Relayed, "{report:?}");
    assert!(report.connect.is_some());
    assert_eq!(report.first_direct, None);
    assert_eq!(report.alive_at, [true; 3], "{report:?}");
    assert!(seen.direct_paths.is_empty(), "{:?}", seen.direct_paths);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_ipv4_only_peer_binds_advertises_and_connects_over_ipv4_only() {
    // B binds IPv6 too, so only A's restriction keeps IPv6 out.
    let (report, seen, shown) = attempt(opts(true, true, T), opts(false, true, T)).await;
    assert_eq!(report.outcome, Outcome::Direct, "{report:?}");
    assert!(!shown.bound.is_empty());
    assert!(shown.bound.iter().all(|a| a.is_ipv4()), "{:?}", shown.bound);
    assert!(
        shown.advertised.iter().all(|a| a.is_ipv4()),
        "{:?}",
        shown.advertised
    );
    assert!(!seen.direct_paths.is_empty());
    assert!(
        seen.direct_paths.iter().all(|a| a.is_ipv4()),
        "{:?}",
        seen.direct_paths
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_dual_stack_peer_does_bind_ipv6() {
    // The IPv4 test above would pass vacuously if IPv6 never appeared.
    let (_, _, shown) = attempt(opts(false, true, T), opts(false, true, T)).await;
    assert!(shown.bound.iter().any(|a| a.is_ipv6()), "{:?}", shown.bound);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_probe_reaches_a_relay_and_fails_without_one() {
    let tls = || Some(CaTlsConfig::insecure_skip_verify());
    let (relays, _url, _server) = run_relay_server().await.unwrap();
    probe(relays, Duration::from_secs(10), tls()).await.unwrap();
    let nobody = RelayMap::from_iter(["https://127.0.0.1:9".parse::<iroh::RelayUrl>().unwrap()]);
    assert!(probe(nobody, Duration::from_secs(3), tls()).await.is_err());
}

#[test]
fn the_version_recorded_is_the_one_built() {
    let lock =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../Cargo.lock")).unwrap();
    let entry = format!("name = \"iroh\"\nversion = \"{LIBRARY_VERSION}\"\n");
    assert!(lock.contains(&entry));
}

/// A relayed attempt at the rule's own timings, about 75 seconds: the
/// relay carries echoes past the last reading. `cargo test -- --ignored`.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "takes over a minute"]
async fn a_relayed_attempt_lasts_the_full_rule_timings() {
    let r = Timings::RULE;
    let (report, _, _) = attempt(opts(true, false, r), opts(true, true, r)).await;
    assert_eq!(report.outcome, Outcome::Relayed);
    assert_eq!(report.alive_at, [true; 3], "{report:?}");
}
