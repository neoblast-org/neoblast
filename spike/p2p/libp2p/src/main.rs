//! `spike-libp2p`: one end of an attempt, the relay, or the harness's
//! health check. Run by `run.sh`; see `spike/p2p/README.md`.
//!
//! ```text
//! spike-libp2p relay  --port <port> --external-ip <ip> --key <file>
//! spike-libp2p listen --relay <addr> [--dual] [--no-holepunch]   # A
//! spike-libp2p dial   --to <addr>    [--dual] [--no-holepunch]   # B
//! spike-libp2p probe  --relay <addr>
//! ```
//!
//! `listen` prints A's circuit address as one line, then answers until it
//! is stopped; it exits with 3 if it gets no reservation in time. `dial`
//! prints one JSON line, B's report. Errors and logs go to standard error,
//! never into a record: they can hold addresses.

use std::io::{Read, Write};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use clap::{Parser, Subcommand};
use libp2p::{Multiaddr, identity};
use p2p_spike_common::{Attempt, Timings};
use p2p_spike_libp2p::{PeerOptions, Result, dial, listen, probe, start_relay};

#[derive(Parser)]
#[command(about = "The libp2p side of the libp2p or iroh spike")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Runs the relay until stopped.
    Relay {
        #[arg(long)]
        port: u16,
        /// The address peers reach the relay at.
        #[arg(long)]
        external_ip: IpAddr,
        /// The relay's key, made on first use, so its address stays the same.
        #[arg(long)]
        key: PathBuf,
    },
    /// A: reserves a slot on the relay and prints its circuit address.
    Listen {
        #[arg(long)]
        relay: Multiaddr,
        #[command(flatten)]
        peer: PeerArgs,
    },
    /// B: dials A's circuit address and prints the attempt's report.
    Dial {
        #[arg(long)]
        to: Multiaddr,
        #[command(flatten)]
        peer: PeerArgs,
    },
    /// Connects to the relay once; exits 0 if it answers.
    Probe {
        #[arg(long)]
        relay: Multiaddr,
    },
}

#[derive(clap::Args)]
struct PeerArgs {
    /// Use IPv6 as well as IPv4 (the separate IPv6 runs; rule 6).
    #[arg(long)]
    dual: bool,
    /// Turn DCUtR off (criterion 3's relayed-only check).
    #[arg(long)]
    no_holepunch: bool,
}

impl PeerArgs {
    fn options(&self) -> PeerOptions {
        PeerOptions {
            ipv4_only: !self.dual,
            holepunch: !self.no_holepunch,
            timings: Timings::RULE,
        }
    }
}

/// Reads the relay's key, or makes one, readable by its owner only.
fn relay_key(path: &Path) -> Result<identity::Keypair> {
    let mut bytes = [0u8; 32];
    match std::fs::File::open(path) {
        Ok(mut f) => f.read_exact(&mut bytes)?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let fresh = identity::Keypair::generate_ed25519();
            let secret = fresh.try_into_ed25519()?.secret();
            bytes.copy_from_slice(secret.as_ref());
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
            options.open(path)?.write_all(&bytes)?;
        }
        Err(e) => return Err(e.into()),
    }
    Ok(identity::Keypair::ed25519_from_bytes(bytes)?)
}

async fn run(command: Command) -> Result<ExitCode> {
    match command {
        Command::Relay {
            port,
            external_ip,
            key,
        } => {
            let relay = start_relay(relay_key(&key)?, port, external_ip).await?;
            println!("{}", relay.quic);
            println!("{}", relay.tcp);
            relay.task.await?;
        }
        Command::Listen { relay, peer } => match listen(&relay, peer.options()).await {
            Ok((circuit, task)) => {
                println!("{circuit}");
                std::io::stdout().flush()?;
                task.await?;
            }
            Err(e) => {
                eprintln!("listen: {e}");
                return Ok(ExitCode::from(3));
            }
        },
        Command::Dial { to, peer } => {
            // The attempt starts as B does (rule 2).
            let attempt = Attempt::new(Timings::RULE);
            let report = match dial(&to, peer.options(), attempt).await {
                Ok((report, _)) => report,
                Err(e) => {
                    // B's own failure after it started counts (rule 10).
                    eprintln!("dial: {e}");
                    Attempt::new(Timings::RULE).report()
                }
            };
            println!("{}", report.to_json());
        }
        Command::Probe { relay } => {
            if let Err(e) = probe(&relay, Duration::from_secs(10)).await {
                eprintln!("probe: {e}");
                return Ok(ExitCode::FAILURE);
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

#[tokio::main]
async fn main() -> ExitCode {
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();
    match run(Cli::parse().command).await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
