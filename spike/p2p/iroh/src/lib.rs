//! The iroh side of the spike: its documented NAT traversal (its relay,
//! address discovery through it, and QUIC hole punching), with our own
//! relay only, measured by `p2p-spike-common`.
//!
//! B's echo is a QUIC stream that A sends back. Rule 3's reading, as the
//! plan's step 2 notes map it onto iroh 1.x: the echo came back, and the
//! path iroh had selected was a direct IP path both when it was sent and
//! when it came back, so it went without the relay.

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use iroh::endpoint::{Connection, PortmapperConfig, presets};
use iroh::tls::CaTlsConfig;
use iroh::{Endpoint, EndpointAddr, EndpointId, RelayMap, RelayMode, RelayUrl, TransportAddr};
use p2p_spike_common::{Attempt, Report, Timings};

/// The library version measured, checked against `Cargo.lock` by a test.
pub const LIBRARY_VERSION: &str = "1.4.1";

/// Errors are reported, not handled: the harness logs them.
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

const ALPN: &[u8] = b"neoblast-p2p-spike/echo/0";

/// A peer's options.
#[derive(Clone, Debug)]
pub struct PeerOptions {
    /// Rule 6: bind IPv4 only, so only IPv4 addresses are advertised.
    pub ipv4_only: bool,
    /// Direct paths on (the spike) or off, by `clear_ip_transports()`, as
    /// iroh's own `endpoint_two_relay_only_no_ip` test does (criterion 3's
    /// relayed-only check).
    pub direct_paths: bool,
    /// Port mapping (UPnP, PCP, NAT-PMP), on in iroh by default.
    pub portmap: bool,
    /// How the relay's certificate is checked; `None` for iroh's default.
    /// The tests' local relay needs `CaTlsConfig::insecure_skip_verify()`.
    pub relay_tls: Option<CaTlsConfig>,
    /// When echoes are sent and readings taken.
    pub timings: Timings,
}

/// An endpoint that uses only `relays`: no address lookup (DNS, pkarr,
/// mDNS) and none of n0's relays, which the `Minimal` preset leaves out.
async fn endpoint(relays: RelayMap, opts: &PeerOptions) -> Result<Endpoint> {
    let mut builder = Endpoint::builder(presets::Minimal)
        .relay_mode(RelayMode::Custom(relays))
        .alpns(vec![ALPN.to_vec()]);
    if !opts.portmap {
        builder = builder.portmapper_config(PortmapperConfig::Disabled);
    }
    if let Some(tls) = opts.relay_tls.clone() {
        builder = builder.ca_tls_config(tls);
    }
    if !opts.direct_paths {
        builder = builder.clear_ip_transports();
    } else if opts.ipv4_only {
        builder = builder.clear_ip_transports().bind_addr("0.0.0.0:0")?;
    }
    Ok(builder.bind().await?)
}

/// What A shows the world, for the IPv4 test.
#[derive(Debug)]
pub struct Shown {
    /// A's bound sockets.
    pub bound: Vec<SocketAddr>,
    /// The IP addresses in A's endpoint address.
    pub advertised: Vec<SocketAddr>,
}

/// A: connects to its relay and returns its endpoint ID, the only thing B
/// is given besides the relay's URL (rule 1). It then echoes every stream
/// it is sent, in the returned task, until that is aborted.
pub async fn listen(
    relays: RelayMap,
    opts: PeerOptions,
) -> Result<(EndpointId, Shown, tokio::task::JoinHandle<()>)> {
    let ep = endpoint(relays, &opts).await?;
    tokio::time::timeout(opts.timings.connect, ep.online())
        .await
        .map_err(|_| "timed out reaching the relay")?;
    let shown = Shown {
        bound: ep.bound_sockets(),
        advertised: ep.addr().ip_addrs().copied().collect(),
    };
    let id = ep.id();
    let task = tokio::spawn(async move {
        while let Some(incoming) = ep.accept().await {
            tokio::spawn(async move {
                let Ok(conn) = incoming.await else { return };
                while let Ok((mut send, mut recv)) = conn.accept_bi().await {
                    tokio::spawn(async move {
                        if let Ok(data) = recv.read_to_end(64).await
                            && send.write_all(&data).await.is_ok()
                        {
                            let _ = send.finish();
                            let _ = send.stopped().await;
                        }
                    });
                }
            });
        }
    });
    Ok((id, shown, task))
}

/// Whether the path iroh has selected is a direct IP path.
fn selected_is_direct(conn: &Connection) -> bool {
    conn.paths().iter().any(|p| p.is_selected() && p.is_ip())
}

/// One echo: 32 bytes out on a new stream, the same 32 back.
async fn echo(conn: &Connection, n: u64) -> Result<()> {
    let (mut send, mut recv) = conn.open_bi().await?;
    let mut msg = [0u8; 32];
    msg[..8].copy_from_slice(&n.to_be_bytes());
    send.write_all(&msg).await?;
    send.finish()?;
    let back = recv.read_to_end(64).await?;
    if back != msg {
        return Err("the echo came back changed".into());
    }
    Ok(())
}

/// What B saw of A, for the IPv4 test.
#[derive(Debug, Default)]
pub struct Seen {
    /// The remote address of every direct path B had to A.
    pub direct_paths: Vec<SocketAddr>,
}

/// B: connects to its relay, then to A by its ID and the relay's URL
/// alone, and measures the attempt, which started when `attempt` was made.
pub async fn dial(
    a: EndpointId,
    relay: RelayUrl,
    relays: RelayMap,
    opts: PeerOptions,
    mut attempt: Attempt,
) -> Result<(Report, Seen)> {
    let until = attempt.deadline();
    let ep = endpoint(relays, &opts).await?;
    // Reaching the relay and connecting count against rule 2's 30 seconds.
    tokio::time::timeout_at(until.into(), ep.online())
        .await
        .map_err(|_| "timed out reaching the relay")?;
    let target = EndpointAddr::new(a).with_relay_url(relay);
    let conn = tokio::time::timeout_at(until.into(), ep.connect(target, ALPN))
        .await
        .map_err(|_| "timed out connecting")??;
    let mut seen = Seen::default();
    let mut tick = tokio::time::interval(opts.timings.echo_every);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut n = 0u64;
    loop {
        tick.tick().await;
        if Instant::now() >= attempt.deadline() {
            break;
        }
        n += 1;
        let before = selected_is_direct(&conn);
        let sent = tokio::time::timeout(opts.timings.echo_timeout, echo(&conn, n)).await;
        if matches!(sent, Ok(Ok(()))) {
            let now = Instant::now();
            let after = selected_is_direct(&conn);
            let paths = conn.paths();
            let relay_open = paths.iter().any(|p| p.is_relay());
            for p in paths.iter() {
                if let TransportAddr::Ip(addr) = p.remote_addr() {
                    seen.direct_paths.push(*addr);
                }
            }
            attempt.echo(now, before && after, relay_open);
        }
    }
    // The report is made before closing, so a slow close cannot lose it;
    // libp2p's side simply drops its swarm.
    let report = attempt.report();
    conn.close(0u32.into(), b"done");
    let _ = tokio::time::timeout(Duration::from_secs(5), ep.close()).await;
    Ok((report, seen))
}

/// The harness's health check (rule 10): a fresh endpoint that connects
/// to the relay within `within`.
pub async fn probe(
    relays: RelayMap,
    within: Duration,
    relay_tls: Option<CaTlsConfig>,
) -> Result<()> {
    let opts = PeerOptions {
        ipv4_only: true,
        direct_paths: true,
        portmap: false,
        relay_tls,
        timings: Timings::RULE,
    };
    let ep = endpoint(relays, &opts).await?;
    let reached = tokio::time::timeout(within, ep.online()).await;
    ep.close().await;
    reached.map_err(|_| "the relay did not answer")?;
    Ok(())
}
