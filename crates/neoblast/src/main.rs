//! `neoblast` command-line tool (Phase 1: offline seal / open / heal).
#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter, Read};
use std::path::{Path, PathBuf};
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
        /// Overwrite the output directory even if it is not empty.
        #[arg(long)]
        force: bool,
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
            force,
        } => {
            let mut store = DirStore::new(&out);
            if !store.is_empty()? {
                if !force {
                    return Err(format!(
                        "{} is not empty; use --force to overwrite what is there",
                        out.display()
                    )
                    .into());
                }
                // Without a manifest the directory no longer claims to hold a
                // complete file, even if this seal fails part-way.
                store.remove_manifest()?;
            }
            let params = Params { k, n, segment_size };
            let mut file = File::open(&input)?;
            let meta = file.metadata()?;
            // The header (bound into every segment) needs the length up front.
            // A regular file is streamed; a pipe or device is read into memory first.
            let (len, (cap, manifest)) = if meta.is_file() {
                let len = meta.len();
                let sealed = neoblast::seal_stream(BufReader::new(file), len, params, &mut store)?;
                (len, sealed)
            } else {
                let mut data = Vec::new();
                file.read_to_end(&mut data)?;
                let len = data.len() as u64;
                (
                    len,
                    neoblast::seal_stream(&data[..], len, params, &mut store)?,
                )
            };
            store.write_manifest(&manifest)?;
            let h = manifest.header();
            eprintln!(
                "sealed {len} bytes into {} segment(s) of {k}-of-{n} shards in {}",
                h.segment_count(),
                out.display()
            );
            eprintln!(
                "verify link (safe for storage nodes): {}",
                cap.verify_cap().to_link()
            );
            eprintln!("read link (anyone holding it can decrypt):");
            println!("{}", cap.to_link());
        }
        Command::Open { link, from, out } => {
            let cap = FileReadCap::parse(&link)?;
            let store = DirStore::new(&from);
            let manifest = store.read_manifest()?;
            let len =
                write_atomically(&out, |w| neoblast::open_stream(&cap, &manifest, &store, w))?;
            eprintln!("recovered {len} bytes to {}", out.display());
        }
        Command::Heal { link, from } => {
            let cap = FileVerifyCap::parse(&link)?;
            let store = DirStore::new(&from);
            let manifest = store.read_manifest()?;
            let rebuilt = neoblast::heal_stream(&cap, &manifest, &store, &mut store.clone())?;
            eprintln!("rebuilt {rebuilt} shard(s) without the read key");
        }
    }
    Ok(())
}

/// Writes to a new hidden file next to `out` (`.<name>.<pid>.partial`) and
/// renames it to `out` only on success. A failed open never leaves a truncated
/// file under the requested name, never leaves decrypted bytes in the partial
/// file, and never overwrites an existing file other than `out` itself.
fn write_atomically(
    out: &Path,
    write: impl FnOnce(&mut BufWriter<File>) -> Result<u64, neoblast::Error>,
) -> Result<u64, Box<dyn std::error::Error>> {
    if out.is_dir() {
        return Err(format!("{} is a directory", out.display()).into());
    }
    let name = out.file_name().ok_or("the output path has no file name")?;
    let mut partial_name = OsString::from(".");
    partial_name.push(name);
    partial_name.push(format!(".{}.partial", std::process::id()));
    let partial = out.with_file_name(partial_name);
    // create_new: never truncate or follow an existing file or symlink.
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&partial)?;
    let mut writer = BufWriter::new(file);
    let result = write(&mut writer)
        .map_err(Box::<dyn std::error::Error>::from)
        .and_then(|len| {
            let file = writer.into_inner().map_err(|e| e.into_error())?;
            file.sync_all()?;
            drop(file);
            fs::rename(&partial, out)?;
            Ok(len)
        });
    if result.is_err() {
        let _ = fs::remove_file(&partial);
    }
    result
}
