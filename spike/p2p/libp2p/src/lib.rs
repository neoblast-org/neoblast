//! The libp2p side of the spike: its documented NAT traversal, as in
//! rust-libp2p's `examples/dcutr` and `examples/relay-server` (circuit relay
//! v2, DCUtR and identify), measured by `p2p-spike-common`.
//!
//! B's echo is libp2p's ping, which runs on every connection on its own and
//! reports which connection each round trip used. So rule 3's reading, an
//! echo on a named connection that is not relayed, is a ping that came back
//! on a connection whose address has no `/p2p-circuit`.

use std::collections::HashMap;
use std::error::Error;
use std::net::IpAddr;
use std::time::{Duration, Instant};

use futures::StreamExt;
use libp2p::core::multiaddr::Protocol;
use libp2p::swarm::behaviour::toggle::Toggle;
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{Multiaddr, PeerId, Swarm, dcutr, identify, identity, noise, ping, relay, tcp, yamux};
use p2p_spike_common::{Attempt, Report, Timings};

/// The library version measured, checked against `Cargo.lock` by a test.
pub const LIBRARY_VERSION: &str = "0.57.1";

/// Errors are reported, not handled: the harness logs them.
pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

const PROTOCOL_VERSION: &str = "/neoblast-p2p-spike/0";

/// How long a connection with nothing but pings on it stays open. Longer
/// than an attempt, so no connection closes for being idle.
const IDLE: Duration = Duration::from_secs(600);

/// A peer's options.
#[derive(Clone, Copy, Debug)]
pub struct PeerOptions {
    /// Rule 6: bind and advertise IPv4 only.
    pub ipv4_only: bool,
    /// DCUtR on (the spike) or off (criterion 3's relayed-only check).
    pub holepunch: bool,
    /// When echoes are sent and readings taken.
    pub timings: Timings,
}

#[derive(NetworkBehaviour)]
struct Peer {
    relay_client: relay::client::Behaviour,
    ping: ping::Behaviour,
    identify: identify::Behaviour,
    dcutr: Toggle<dcutr::Behaviour>,
}

fn peer_swarm(opts: &PeerOptions) -> Result<Swarm<Peer>> {
    let ping = ping::Config::new()
        .with_interval(opts.timings.echo_every)
        .with_timeout(opts.timings.echo_timeout);
    let holepunch = opts.holepunch;
    let swarm = libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            tcp::Config::default().nodelay(true),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_quic()
        .with_relay_client(noise::Config::new, yamux::Config::default)?
        .with_behaviour(|key, relay_client| Peer {
            relay_client,
            ping: ping::Behaviour::new(ping),
            identify: identify::Behaviour::new(identify::Config::new(
                PROTOCOL_VERSION.to_string(),
                key.public(),
            )),
            dcutr: Toggle::from(
                holepunch.then(|| dcutr::Behaviour::new(key.public().to_peer_id())),
            ),
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(IDLE))
        .build();
    Ok(swarm)
}

/// Listens on fresh QUIC and TCP ports, IPv4 and, unless IPv4 only, IPv6.
fn listen_everywhere<B: NetworkBehaviour>(swarm: &mut Swarm<B>, ipv4_only: bool) -> Result<()> {
    swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?)?;
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
    if !ipv4_only {
        swarm.listen_on("/ip6/::/udp/0/quic-v1".parse()?)?;
        swarm.listen_on("/ip6/::/tcp/0".parse()?)?;
    }
    Ok(())
}

/// The peer ID at the end of an address, if it has one.
fn peer_of(addr: &Multiaddr) -> Option<PeerId> {
    addr.iter().last().and_then(|p| match p {
        Protocol::P2p(id) => Some(id),
        _ => None,
    })
}

/// The relay's part of a circuit address: everything before `/p2p-circuit`.
fn relay_part(circuit: &Multiaddr) -> Multiaddr {
    circuit
        .iter()
        .take_while(|p| *p != Protocol::P2pCircuit)
        .collect()
}

/// Dials the relay and waits until each side has told the other the
/// address it sees, as `examples/dcutr` does, so DCUtR has a candidate.
async fn meet_relay(swarm: &mut Swarm<Peer>, relay: &Multiaddr, until: Instant) -> Result<()> {
    let relay_id = peer_of(relay).ok_or("the relay address has no /p2p/ peer ID")?;
    swarm.dial(relay.clone())?;
    let (mut sent, mut received) = (false, false);
    while !(sent && received) {
        let event = tokio::time::timeout_at(until.into(), swarm.select_next_some())
            .await
            .map_err(|_| "timed out meeting the relay")?;
        match event {
            SwarmEvent::Behaviour(PeerEvent::Identify(identify::Event::Sent {
                peer_id, ..
            })) if peer_id == relay_id => sent = true,
            SwarmEvent::Behaviour(PeerEvent::Identify(identify::Event::Received {
                peer_id,
                ..
            })) if peer_id == relay_id => received = true,
            SwarmEvent::OutgoingConnectionError {
                peer_id: Some(p),
                error,
                ..
            } if p == relay_id => {
                return Err(format!("could not reach the relay: {error}").into());
            }
            _ => {}
        }
    }
    Ok(())
}

/// A: meets the relay, reserves a slot, and returns its circuit address,
/// the only thing B is given (rule 1). The swarm then runs in the returned
/// task, answering pings, until it is aborted.
pub async fn listen(
    relay: &Multiaddr,
    opts: PeerOptions,
) -> Result<(Multiaddr, tokio::task::JoinHandle<()>)> {
    let until = Instant::now() + opts.timings.connect;
    let mut swarm = peer_swarm(&opts)?;
    listen_everywhere(&mut swarm, opts.ipv4_only)?;
    meet_relay(&mut swarm, relay, until).await?;
    swarm.listen_on(relay.clone().with(Protocol::P2pCircuit))?;
    let me = *swarm.local_peer_id();
    let circuit = loop {
        let event = tokio::time::timeout_at(until.into(), swarm.select_next_some())
            .await
            .map_err(|_| "timed out waiting for a reservation")?;
        // A circuit listen address means the reservation was accepted,
        // whether or not its event arrived (rust-libp2p#6141).
        if let SwarmEvent::NewListenAddr { address, .. } = event
            && address.iter().any(|p| p == Protocol::P2pCircuit)
        {
            break match peer_of(&address) {
                Some(id) if id == me => address,
                _ => address.with(Protocol::P2p(me)),
            };
        }
    };
    let task = tokio::spawn(async move {
        loop {
            swarm.select_next_some().await;
        }
    });
    Ok((circuit, task))
}

/// What B saw of A, for the IPv4 test.
#[derive(Debug, Default)]
pub struct Seen {
    /// The addresses A listed in identify.
    pub listen_addrs: Vec<Multiaddr>,
    /// The remote address of every connection to A.
    pub connections: Vec<Multiaddr>,
}

/// B: meets the relay, dials A's circuit address and measures the attempt,
/// which started when `attempt` was made.
pub async fn dial(
    target: &Multiaddr,
    opts: PeerOptions,
    mut attempt: Attempt,
) -> Result<(Report, Seen)> {
    let a = peer_of(target).ok_or("the target address has no /p2p/ peer ID")?;
    let relay = relay_part(target);
    let mut swarm = peer_swarm(&opts)?;
    listen_everywhere(&mut swarm, opts.ipv4_only)?;
    // Meeting the relay counts against rule 2's 30 seconds.
    meet_relay(&mut swarm, &relay, attempt.deadline()).await?;
    swarm.dial(target.clone())?;
    let mut relayed: HashMap<libp2p::swarm::ConnectionId, bool> = HashMap::new();
    let mut seen = Seen::default();
    loop {
        let event = match tokio::time::timeout_at(
            attempt.deadline().into(),
            swarm.select_next_some(),
        )
        .await
        {
            Ok(event) => event,
            Err(_) => break,
        };
        match event {
            SwarmEvent::ConnectionEstablished {
                peer_id,
                connection_id,
                endpoint,
                ..
            } if peer_id == a => {
                relayed.insert(connection_id, endpoint.is_relayed());
                seen.connections.push(endpoint.get_remote_address().clone());
            }
            SwarmEvent::ConnectionClosed { connection_id, .. } => {
                relayed.remove(&connection_id);
            }
            SwarmEvent::Behaviour(PeerEvent::Ping(ping::Event {
                peer,
                connection,
                result: Ok(_),
            })) if peer == a => {
                if let Some(&was_relayed) = relayed.get(&connection) {
                    let relay_open = relayed.values().any(|&r| r);
                    attempt.echo(Instant::now(), !was_relayed, relay_open);
                }
            }
            SwarmEvent::Behaviour(PeerEvent::Identify(identify::Event::Received {
                peer_id,
                info,
                ..
            })) if peer_id == a => {
                seen.listen_addrs.extend(info.listen_addrs);
            }
            _ => {}
        }
    }
    Ok((attempt.report(), seen))
}

#[derive(NetworkBehaviour)]
struct Prober {
    ping: ping::Behaviour,
    identify: identify::Behaviour,
}

/// The harness's health check (rule 10): a fresh connection to the relay
/// that completes identify within `within`.
pub async fn probe(relay: &Multiaddr, within: Duration) -> Result<()> {
    let relay_id = peer_of(relay).ok_or("the relay address has no /p2p/ peer ID")?;
    let mut swarm = libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_quic()
        .with_behaviour(|key| Prober {
            ping: ping::Behaviour::new(ping::Config::new()),
            identify: identify::Behaviour::new(identify::Config::new(
                PROTOCOL_VERSION.to_string(),
                key.public(),
            )),
        })?
        .build();
    swarm.dial(relay.clone())?;
    let until = Instant::now() + within;
    loop {
        let event = tokio::time::timeout_at(until.into(), swarm.select_next_some())
            .await
            .map_err(|_| "the relay did not answer")?;
        match event {
            SwarmEvent::Behaviour(ProberEvent::Identify(identify::Event::Received {
                peer_id,
                ..
            })) if peer_id == relay_id => {
                return Ok(());
            }
            SwarmEvent::OutgoingConnectionError { error, .. } => {
                return Err(format!("could not reach the relay: {error}").into());
            }
            _ => {}
        }
    }
}

#[derive(NetworkBehaviour)]
struct RelayServer {
    relay: relay::Behaviour,
    ping: ping::Behaviour,
    identify: identify::Behaviour,
}

/// The relay's limits: rust-libp2p's defaults, except that a circuit lasts
/// 10 minutes and carries any number of bytes. The defaults, 2 minutes and
/// 128 KiB, could cut an attempt short (criterion 3).
pub fn relay_config() -> relay::Config {
    relay::Config {
        max_circuit_duration: Duration::from_secs(600),
        max_circuit_bytes: u64::MAX,
        ..Default::default()
    }
}

/// A running relay and the addresses peers use to reach it.
#[derive(Debug)]
pub struct RelayHandle {
    /// Its QUIC address, with its peer ID: the one the spike uses.
    pub quic: Multiaddr,
    /// Its TCP address, with its peer ID.
    pub tcp: Multiaddr,
    /// The task running it.
    pub task: tokio::task::JoinHandle<()>,
}

/// Starts a relay on `port` (0 for any), for TCP and QUIC, IPv4 only, as
/// `examples/relay-server` does. `external` is the address peers reach it
/// at; it is confirmed at once, as relay 0.22 offers relaying only once it
/// has a confirmed external address (rust-libp2p#6154).
pub async fn start_relay(
    key: identity::Keypair,
    port: u16,
    external: IpAddr,
) -> Result<RelayHandle> {
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(key)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_quic()
        .with_behaviour(|key| RelayServer {
            relay: relay::Behaviour::new(key.public().to_peer_id(), relay_config()),
            ping: ping::Behaviour::new(ping::Config::new()),
            identify: identify::Behaviour::new(identify::Config::new(
                PROTOCOL_VERSION.to_string(),
                key.public(),
            )),
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(IDLE))
        .build();
    swarm.listen_on(format!("/ip4/0.0.0.0/udp/{port}/quic-v1").parse()?)?;
    swarm.listen_on(format!("/ip4/0.0.0.0/tcp/{port}").parse()?)?;
    let (mut quic_port, mut tcp_port) = (None, None);
    while quic_port.is_none() || tcp_port.is_none() {
        if let SwarmEvent::NewListenAddr { address, .. } = swarm.select_next_some().await {
            for p in address.iter() {
                match p {
                    Protocol::Udp(p) => quic_port = Some(p),
                    Protocol::Tcp(p) => tcp_port = Some(p),
                    _ => {}
                }
            }
        }
    }
    let id = *swarm.local_peer_id();
    let ip = Multiaddr::empty().with(Protocol::from(external));
    let quic = ip
        .clone()
        .with(Protocol::Udp(quic_port.unwrap_or(port)))
        .with(Protocol::QuicV1);
    let tcp = ip.with(Protocol::Tcp(tcp_port.unwrap_or(port)));
    swarm.add_external_address(quic.clone());
    swarm.add_external_address(tcp.clone());
    let task = tokio::spawn(async move {
        loop {
            swarm.select_next_some().await;
        }
    });
    Ok(RelayHandle {
        quic: quic.with(Protocol::P2p(id)),
        tcp: tcp.with(Protocol::P2p(id)),
        task,
    })
}
