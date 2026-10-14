//! `neoblast` command-line tool (Phase 1: offline seal / open / heal).
#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use neoblast::store::DirStore;
use neoblast::{FileReadCap, FileVerifyCap, Params};

/// Censorship-resistant storage. Experimental and not audited.
#[derive(Parser)]
#[command(name = "neoblast", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Encrypt and shard a file into a directory, then print its read link.
    Seal {
        /// File to seal.
        input: PathBuf,
        /// Directory to write the manifest and shards into.
        #[arg(long)]
        out: PathBuf,
        /// Data shards per segment.
        #[arg(long, default_value_t = Params::default().k)]
        k: u16,
        /// Total shards per segment.
        #[arg(long, default_value_t = Params::default().n)]
        n: u16,
        /// Plaintext bytes per segment.
        #[arg(long, default_value_t = Params::default().segment_size)]
        segment_size: u32,
    },
    /// Rebuild a file from its read link and a shard directory.
    Open {
        /// The `nbl://f/...` read link.
        link: String,
        /// Directory holding the manifest and shards.
        #[arg(long)]
        from: PathBuf,
        /// Where to write the recovered file.
        #[arg(long)]
        out: PathBuf,
    },
    /// Rebuild missing or corrupt shards. Needs only the verify link, never the key.
    Heal {
        /// A `nbl://fv/...` verify link (a read link also works).
        link: String,
        /// Directory holding the manifest and shards.
        #[arg(long)]
        from: PathBuf,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Command::Seal {
            input,
            out,
            k,
            n,
            segment_size,
        } => {
            let payload = std::fs::read(&input)?;
            let sealed = neoblast::seal(&payload, Params { k, n, segment_size })?;
            DirStore::new(&out).write_sealed(&sealed)?;
            let h = sealed.manifest.header();
            eprintln!(
                "sealed {} bytes into {} segment(s) of {k}-of-{n} shards in {}",
                payload.len(),
                h.segment_count(),
                out.display()
            );
            eprintln!(
                "verify link (safe for storage nodes): {}",
                sealed.cap.verify_cap().to_link()
            );
            eprintln!("read link (anyone holding it can decrypt):");
            println!("{}", sealed.cap.to_link());
        }
        Command::Open { link, from, out } => {
            let cap = FileReadCap::parse(&link)?;
            let store = DirStore::new(&from);
            let payload = neoblast::open(&cap, &store.read_manifest()?, &store)?;
            std::fs::write(&out, &payload)?;
            eprintln!("recovered {} bytes to {}", payload.len(), out.display());
        }
        Command::Heal { link, from } => {
            let cap = FileVerifyCap::parse(&link)?;
            let store = DirStore::new(&from);
            let rebuilt = neoblast::heal(&cap, &store.read_manifest()?, &store)?;
            store.write_rebuilt(&rebuilt)?;
            eprintln!("rebuilt {} shard(s) without the read key", rebuilt.len());
        }
    }
    Ok(())
}
