//! Criterion 3 on one machine: a relay on the loopback address, A and B as
//! in a real attempt, with shortened timings.

use std::net::Ipv4Addr;

use libp2p::core::multiaddr::Protocol;
use libp2p::{Multiaddr, identity};
use p2p_spike_common::{Attempt, Outcome, Timings};
use p2p_spike_libp2p::{
    LIBRARY_VERSION, PeerOptions, Seen, dial, listen, probe, relay_config, start_relay,
};

fn opts(ipv4_only: bool, holepunch: bool) -> PeerOptions {
    PeerOptions {
        ipv4_only,
        holepunch,
        timings: Timings::TEST,
    }
}

async fn attempt(a: PeerOptions, b: PeerOptions) -> (p2p_spike_common::Report, Seen) {
    let relay = start_relay(
        identity::Keypair::generate_ed25519(),
        0,
        Ipv4Addr::LOCALHOST.into(),
    )
    .await
    .unwrap();
    let (circuit, a_task) = listen(&relay.quic, a).await.unwrap();
    assert!(circuit.iter().any(|p| p == Protocol::P2pCircuit));
    let out = dial(&circuit, b, Attempt::new(Timings::TEST))
        .await
        .unwrap();
    a_task.abort();
    relay.task.abort();
    out
}

fn is_ip4(addr: &Multiaddr) -> bool {
    matches!(addr.iter().next(), Some(Protocol::Ip4(_)))
}

#[tokio::test(flavor = "multi_thread")]
async fn hole_punching_gives_direct_with_the_relayed_connection_still_open() {
    let (report, _) = attempt(opts(true, true), opts(true, true)).await;
    assert_eq!(report.outcome, Outcome::Direct, "{report:?}");
    assert_eq!(report.relay_open_at_30, Some(true), "{report:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn without_dcutr_it_stays_relayed() {
    let (report, seen) = attempt(opts(true, false), opts(true, false)).await;
    assert_eq!(report.outcome, Outcome::Relayed, "{report:?}");
    assert!(report.connect.is_some());
    assert_eq!(report.first_direct, None);
    assert!(report.alive_at.iter().all(|&a| a), "{report:?}");
    assert!(
        seen.connections
            .iter()
            .all(|c| c.iter().any(|p| p == Protocol::P2pCircuit))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_ipv4_only_peer_advertises_and_connects_over_ipv4_only() {
    // B listens on IPv6 too, so only A's restriction keeps IPv6 out.
    let (report, seen) = attempt(opts(true, true), opts(false, true)).await;
    assert_eq!(report.outcome, Outcome::Direct, "{report:?}");
    assert!(!seen.listen_addrs.is_empty());
    assert!(
        seen.listen_addrs.iter().all(is_ip4),
        "{:?}",
        seen.listen_addrs
    );
    assert!(
        seen.connections.iter().all(is_ip4),
        "{:?}",
        seen.connections
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_dual_stack_peer_does_advertise_ipv6() {
    // The IPv4 test above would pass vacuously if IPv6 never appeared.
    let (_, seen) = attempt(opts(false, true), opts(false, true)).await;
    assert!(
        seen.listen_addrs.iter().any(|a| !is_ip4(a)),
        "{:?}",
        seen.listen_addrs
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_probe_reaches_a_relay_and_fails_without_one() {
    let relay = start_relay(
        identity::Keypair::generate_ed25519(),
        0,
        Ipv4Addr::LOCALHOST.into(),
    )
    .await
    .unwrap();
    probe(&relay.quic, std::time::Duration::from_secs(5))
        .await
        .unwrap();
    probe(&relay.tcp, std::time::Duration::from_secs(5))
        .await
        .unwrap();
    relay.task.abort();
    let id = libp2p::PeerId::random();
    let nobody: Multiaddr = format!("/ip4/127.0.0.1/tcp/9/p2p/{id}").parse().unwrap();
    assert!(
        probe(&nobody, std::time::Duration::from_secs(5))
            .await
            .is_err()
    );
}

#[test]
fn the_relay_lets_a_circuit_outlast_an_attempt() {
    let c = relay_config();
    assert!(c.max_circuit_duration >= std::time::Duration::from_secs(120));
    assert_eq!(c.max_circuit_bytes, u64::MAX);
}

#[test]
fn the_version_recorded_is_the_one_built() {
    let lock =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../Cargo.lock")).unwrap();
    let entry = format!("name = \"libp2p\"\nversion = \"{LIBRARY_VERSION}\"\n");
    assert!(lock.contains(&entry));
}

/// A relayed attempt at the rule's own timings, about 75 seconds: the
/// relay carries echoes past the last reading. `cargo test -- --ignored`.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "takes over a minute"]
async fn a_relayed_attempt_lasts_the_full_rule_timings() {
    let rule = |hp| PeerOptions {
        ipv4_only: true,
        holepunch: hp,
        timings: Timings::RULE,
    };
    let relay = start_relay(
        identity::Keypair::generate_ed25519(),
        0,
        Ipv4Addr::LOCALHOST.into(),
    )
    .await
    .unwrap();
    let (circuit, a_task) = listen(&relay.quic, rule(false)).await.unwrap();
    let (report, _) = dial(&circuit, rule(false), Attempt::new(Timings::RULE))
        .await
        .unwrap();
    a_task.abort();
    assert_eq!(report.outcome, Outcome::Relayed);
    assert_eq!(report.alive_at, [true; 3], "{report:?}");
}
