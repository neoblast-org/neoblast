//! `spike-iroh`: one end of an attempt, or the harness's health check. The
//! relay is iroh's own `iroh-relay`. Run by `run.sh`; see
//! `spike/p2p/README.md`.
//!
//! ```text
//! spike-iroh listen --relay <url> [--dual] [--no-portmap] [--no-direct]   # A
//! spike-iroh dial   --to <id> --relay <url> [--dual] [--no-portmap]       # B
//! spike-iroh probe  --relay <url>
//! ```
//!
//! `listen` prints A's endpoint ID as one line, then answers until it is
//! stopped; it exits with 3 if it cannot reach the relay in time. `dial`
//! prints one JSON line, B's report. Errors and logs go to standard error,
//! never into a record: they can hold addresses.

use std::io::Write;
use std::process::ExitCode;
use std::time::Duration;

use clap::{Parser, Subcommand};
use iroh::{EndpointId, RelayMap, RelayUrl};
use p2p_spike_common::{Attempt, Timings};
use p2p_spike_iroh::{PeerOptions, Result, dial, listen, probe};

#[derive(Parser)]
#[command(about = "The iroh side of the libp2p or iroh spike")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// A: connects to the relay and prints its endpoint ID.
    Listen {
        #[arg(long)]
        relay: RelayUrl,
        #[command(flatten)]
        peer: PeerArgs,
        /// No direct paths (criterion 3's relayed-only check).
        #[arg(long)]
        no_direct: bool,
    },
    /// B: connects to A by its ID and the relay's URL, and prints the report.
    Dial {
        #[arg(long)]
        to: EndpointId,
        #[arg(long)]
        relay: RelayUrl,
        #[command(flatten)]
        peer: PeerArgs,
    },
    /// Connects to the relay once; exits 0 if it answers.
    Probe {
        #[arg(long)]
        relay: RelayUrl,
    },
}

#[derive(clap::Args)]
struct PeerArgs {
    /// Use IPv6 as well as IPv4 (the separate IPv6 runs; rule 6).
    #[arg(long)]
    dual: bool,
    /// Turn iroh's port mapping (UPnP, PCP, NAT-PMP) off.
    #[arg(long)]
    no_portmap: bool,
}

impl PeerArgs {
    fn options(&self, direct_paths: bool) -> PeerOptions {
        PeerOptions {
            ipv4_only: !self.dual,
            direct_paths,
            portmap: !self.no_portmap,
            relay_tls: None,
            timings: Timings::RULE,
        }
    }
}

fn relays(url: &RelayUrl) -> RelayMap {
    // The relay's address discovery is on its default QUIC port, 7842.
    RelayMap::from_iter([url.clone()])
}

async fn run(command: Command) -> Result<ExitCode> {
    match command {
        Command::Listen {
            relay,
            peer,
            no_direct,
        } => match listen(relays(&relay), peer.options(!no_direct)).await {
            Ok((id, _, task)) => {
                println!("{id}");
                std::io::stdout().flush()?;
                task.await?;
            }
            Err(e) => {
                eprintln!("listen: {e}");
                return Ok(ExitCode::from(3));
            }
        },
        Command::Dial { to, relay, peer } => {
            // The attempt starts as B does (rule 2).
            let attempt = Attempt::new(Timings::RULE);
            let report = match dial(
                to,
                relay.clone(),
                relays(&relay),
                peer.options(true),
                attempt,
            )
            .await
            {
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
            if let Err(e) = probe(relays(&relay), Duration::from_secs(10), None).await {
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
