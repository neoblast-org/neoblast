//! `neoblast` command-line tool (Phase 1: offline seal / open / heal).
#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, IsTerminal, Read};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use neoblast::store::{DirStore, MAX_MANIFEST_LEN, manifest_len};
use neoblast::{FileReadCap, FileVerifyCap, Header, Manifest, Params};
use neoblast_format::HeaderError;
use zeroize::{Zeroize, Zeroizing};

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
        /// Refuse a file that would need more working memory than this, in
        /// MiB.
        #[arg(long, value_name = "MIB", default_value_t = DEFAULT_MAX_MEMORY_MIB)]
        max_memory: u64,
    },
    /// Rebuild a file from its read link and a shard directory.
    Open {
        /// The `nbl://f/...` read link, or `-` to read it from stdin (a file
        /// or a pipe), as when left out. Other local users can see a link
        /// given here, and the shell keeps it in its history; prefer stdin.
        link: Option<String>,
        /// Directory holding the manifest and shards.
        #[arg(long)]
        from: PathBuf,
        /// Where to write the recovered file.
        #[arg(long)]
        out: PathBuf,
        /// Refuse a file that would need more working memory than this, in
        /// MiB.
        #[arg(long, value_name = "MIB", default_value_t = DEFAULT_MAX_MEMORY_MIB)]
        max_memory: u64,
    },
    /// Rebuild missing or corrupt shards. Needs only the verify link, never the key.
    Heal {
        /// A `nbl://fv/...` verify link (a read link also works), or `-` to
        /// read it from stdin (a file or a pipe), as when left out. Other
        /// local users can see a read link given here, and the shell keeps it
        /// in its history.
        link: Option<String>,
        /// Directory holding the manifest and shards.
        #[arg(long)]
        from: PathBuf,
        /// Refuse a file that would need more working memory than this, in
        /// MiB.
        #[arg(long, value_name = "MIB", default_value_t = DEFAULT_MAX_MEMORY_MIB)]
        max_memory: u64,
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
            max_memory,
        } => {
            let mut store = DirStore::new(&out);
            let params = Params { k, n, segment_size };
            // Refused before any input is read, so an endless pipe is never
            // buffered (review W-005).
            let replacing = !store.is_empty()?;
            if replacing && !force {
                return Err(format!(
                    "{} is not empty; use --force to overwrite what is there",
                    out.display()
                )
                .into());
            }
            let mut file = File::open(&input)?;
            let meta = file.metadata()?;
            // The header (bound into every segment) needs the length up front.
            // A regular file is streamed; a pipe or device is read into memory first.
            let (len, piped) = if meta.is_file() {
                (meta.len(), None)
            } else {
                let mut data = Vec::new();
                file.read_to_end(&mut data)?;
                (data.len() as u64, Some(data))
            };
            // Everything that can refuse the input runs before the store is touched.
            check_params(len, params, max_memory)?;
            if replacing {
                // Without a manifest the directory no longer claims to hold a
                // complete file, even if this seal fails part-way.
                store.remove_manifest()?;
            }
            let (cap, manifest) = match piped {
                None => neoblast::seal_stream(BufReader::new(file), len, params, &mut store)?,
                Some(data) => neoblast::seal_stream(&data[..], len, params, &mut store)?,
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
        Command::Open {
            link,
            from,
            out,
            max_memory,
        } => {
            let cap = FileReadCap::parse(&link_or_stdin(link)?)?;
            let store = DirStore::new(&from);
            // Taken over, not copied, so the manifest is in memory once.
            let manifest = Manifest::decode_owned(store.read_manifest()?, &cap.root)?;
            within_budget(manifest.header(), max_memory, RAISE_THE_BUDGET)?;
            let len = write_atomically(&out, |w| {
                neoblast::open_manifest(&cap, &manifest, &store, w)
            })?;
            eprintln!("recovered {len} bytes to {}", out.display());
        }
        Command::Heal {
            link,
            from,
            max_memory,
        } => {
            let cap = FileVerifyCap::parse(&link_or_stdin(link)?)?;
            let store = DirStore::new(&from);
            let manifest = Manifest::decode_owned(store.read_manifest()?, &cap.root)?;
            within_budget(manifest.header(), max_memory, RAISE_THE_BUDGET)?;
            let rebuilt = neoblast::heal_manifest(&cap, &manifest, &store, &mut store.clone())?;
            eprintln!("rebuilt {rebuilt} shard(s) without the read key");
        }
    }
    Ok(())
}

/// The longest first line of stdin read as a link, its line ending
/// included. A read link is about 115 bytes; this leaves room for the peer
/// hints that later versions may add after the key.
const MAX_LINK_LINE: usize = 4096;

/// The link given, or, when it is `-` or left out, the first line of stdin
/// without its line ending, `\n`, `\r\n` or a last `\r` (review S-003). A
/// link on the command line can be seen by other local users and is kept in
/// shell history; one on stdin is not. Stdin must be a file or a pipe, not a
/// terminal, which would wait with no prompt and show the link as it is
/// typed or pasted (review W-050). The line is read into a buffer already of
/// its largest size, so it is never moved, and is wiped on drop; the
/// standard library's own stdin buffer is not.
fn link_or_stdin(link: Option<String>) -> Result<Zeroizing<String>, Box<dyn std::error::Error>> {
    let stdin = std::io::stdin();
    link_or_input(link, stdin.is_terminal(), &mut stdin.lock())
}

/// What `neoblast` says when it gets no link, whether stdin is a terminal or
/// empty (review W-060).
const NO_LINK: &str = "no link: give it as an argument, or on stdin from a file or a pipe";

/// [`link_or_stdin`], with stdin and whether it is a terminal passed in, so
/// that a test can reach the terminal check (review W-058).
fn link_or_input(
    link: Option<String>,
    input_is_terminal: bool,
    input: &mut impl BufRead,
) -> Result<Zeroizing<String>, Box<dyn std::error::Error>> {
    if let Some(link) = link.filter(|l| l.as_str() != "-") {
        return Ok(Zeroizing::new(link));
    }
    if input_is_terminal {
        return Err(NO_LINK.into());
    }
    let mut line = Zeroizing::new(Vec::with_capacity(MAX_LINK_LINE + 1));
    input
        .take(MAX_LINK_LINE as u64 + 1)
        .read_until(b'\n', &mut line)?;
    if line.len() > MAX_LINK_LINE {
        return Err(format!("the first line of stdin is over {MAX_LINK_LINE} bytes").into());
    }
    if line.ends_with(b"\n") {
        line.pop();
    }
    if line.ends_with(b"\r") {
        line.pop();
    }
    if line.is_empty() {
        return Err(NO_LINK.into());
    }
    match String::from_utf8(std::mem::take(&mut *line)) {
        Ok(link) => Ok(Zeroizing::new(link)),
        Err(e) => {
            e.into_bytes().zeroize();
            Err("the link on stdin is not UTF-8".into())
        }
    }
}

/// Refuses, before anything is written, bad parameters and a file whose
/// manifest would be too large for `open` and `heal` to read back (review
/// W-001). Too many segments, or too many shards, whether for the manifest or
/// for the header, gets the same hint (review W-009). A file over the memory
/// budget, which readers would refuse too, is refused by [`within_budget`],
/// with [`SHRINK_THE_FILE`] as its advice (decision 0011, reviews W-068 and
/// W-070).
fn check_params(
    len: u64,
    params: Params,
    max_memory: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    match params.header(len) {
        Ok(header) => within_budget(&header, max_memory, SHRINK_THE_FILE),
        Err(neoblast::Error::ManifestTooLarge | neoblast::Error::Params(HeaderError::Layout)) => {
            Err(format!(
                "a {len}-byte file needs too many shards at these parameters \
                 (a manifest over {} MiB); use a larger --segment-size or a smaller --n",
                MAX_MANIFEST_LEN >> 20
            )
            .into())
        }
        Err(e) => Err(e.into()),
    }
}

/// The default `--max-memory`, in MiB: one operation fits a 2 GiB VPS or a
/// 4 GiB phone with room to spare (decision 0011).
const DEFAULT_MAX_MEMORY_MIB: u64 = 1024;

/// What the estimate allows for buffers that do not grow with the shards:
/// decision 0009's budget for one operation at the defaults.
const FIXED_ALLOWANCE: u64 = 32 << 20;

/// An upper bound on the working memory `seal`, `open` or `heal` needs for a
/// file with this header, in bytes (decision 0011): two sets of its `n`
/// shards, the erasure codec's largest work buffer, its manifest, and
/// [`FIXED_ALLOWANCE`]. The first segment's shards are the largest. `None`
/// if it overflows, which [`check_budget`] refuses whatever the budget; no v1
/// header does.
fn working_memory(header: &Header) -> Option<u64> {
    let k = u64::from(header.k());
    let n = u64::from(header.n());
    // The high-rate codec, the only one used, works on at most this many
    // shards: its decoder on exactly this, its encoder on fewer.
    let work = n
        .saturating_sub(k)
        .checked_next_power_of_two()?
        .checked_add(k)?
        .checked_next_power_of_two()?;
    u64::from(header.segment_shard_len(0)?)
        .checked_mul(n.checked_mul(2)?.checked_add(work)?)?
        .checked_add(manifest_len(header)?)?
        .checked_add(FIXED_ALLOWANCE)
}

/// What `open` and `heal` advise when they refuse a file over the budget.
const RAISE_THE_BUDGET: &str = "raise --max-memory to allow it";

/// What `seal` advises instead: a file over the default budget is one that
/// every reader and healer refuses by default (decision 0011, review W-068).
const SHRINK_THE_FILE: &str = "use a smaller --n or --segment-size; a higher --max-memory \
     seals it, but every reader and healer would need one too";

/// Refuses a file whose estimated working memory, rounded up to MiB, is over
/// `max_memory` MiB, ending the message with `advice` (review S-037,
/// decision 0011).
fn within_budget(
    header: &Header,
    max_memory: u64,
    advice: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    check_budget(working_memory(header), max_memory, advice)
}

/// [`within_budget`] for an estimate in bytes. One that overflowed is
/// refused whatever the budget (review W-069).
fn check_budget(
    bytes: Option<u64>,
    max_memory: u64,
    advice: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(needed) = bytes.map(|bytes| bytes.div_ceil(1 << 20)) else {
        return Err("this file's working memory is too large to estimate".into());
    };
    if needed <= max_memory {
        return Ok(());
    }
    Err(format!(
        "this file needs about {needed} MiB of working memory, over the {max_memory} MiB \
         allowed; {advice}"
    )
    .into())
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    const MIB: u64 = 1 << 20;

    /// Decision 0011: the estimate for each case it measured is two sets of
    /// n shards, the codec's largest work buffer (in shards), the manifest
    /// and 32 MiB, and only case A is over the default budget. Cases A to E
    /// are one segment long, as the decision's are: the 8-byte length prefix
    /// fills the segment. In MiB, rounded up, the estimates are the
    /// decision's figures, which round down (review W-071).
    #[test]
    fn working_memory_is_decision_0011s_estimate() {
        // (k, n, segment size, file length, the codec's work in shards,
        // the estimate in MiB, refused by default)
        let one = |segment_size| segment_size - 8;
        let cases = [
            (2, 128, 8 * MIB, one(8 * MIB), 256, 2081, true), // A
            (20, 50, 16 * MIB, one(16 * MIB), 64, 164, false), // B
            (64, 128, 64 * MIB, one(64 * MIB), 128, 417, false), // C
            (2, 3, 64 * MIB, one(64 * MIB), 4, 353, false),   // D
            (20, 50, MIB, one(MIB), 64, 41, false),           // E, the defaults
            (20, 50, MIB, 16 * MIB, 64, 41, false),           // 17 segments at the defaults
            (20, 50, 64 * MIB, 64 * MIB, 64, 557, false),     // the largest segments at 20-of-50
            (20, 50, 64 * MIB, 100 * MIB, 64, 557, false),    // a short last segment
        ];
        for (k, n, segment_size, len, work, mib, refused) in cases {
            let params = Params {
                k,
                n,
                segment_size: u32::try_from(segment_size).unwrap(),
            };
            let header = params.header(len).unwrap();
            let shards =
                u64::from(header.segment_shard_len(0).unwrap()) * (2 * u64::from(n) + work);
            assert_eq!(
                working_memory(&header),
                Some(shards + manifest_len(&header).unwrap() + 32 * MIB),
                "{k}-of-{n}"
            );
            assert_eq!(
                working_memory(&header).unwrap().div_ceil(MIB),
                mib,
                "{k}-of-{n}"
            );
            assert_eq!(
                within_budget(&header, DEFAULT_MAX_MEMORY_MIB, RAISE_THE_BUDGET).is_err(),
                refused,
                "{k}-of-{n}"
            );
        }
        let a = Params {
            k: 2,
            n: 128,
            segment_size: 8 << 20,
        };
        assert_eq!(a.header(one(8 * MIB)).unwrap().segment_count(), 1);
        assert_eq!(a.header(8 * MIB).unwrap().segment_count(), 2);
    }

    /// At a terminal, no link and `-` are refused at once, before stdin is
    /// read, and with the same message as an empty stdin; a link given as an
    /// argument is used either way (reviews W-050, W-058 and W-060).
    #[test]
    fn a_terminal_gives_no_link() {
        let piped = b"nbl://f/root#key\n";
        for (link, terminal, expected) in [
            (None, true, Err(NO_LINK)),
            (Some("-"), true, Err(NO_LINK)),
            (None, false, Ok("nbl://f/root#key")),
            (Some("-"), false, Ok("nbl://f/root#key")),
            (Some("nbl://f/arg#key"), true, Ok("nbl://f/arg#key")),
            (Some("nbl://f/arg#key"), false, Ok("nbl://f/arg#key")),
        ] {
            let mut input = &piped[..];
            let result = link_or_input(link.map(String::from), terminal, &mut input)
                .map(|l| l.to_string())
                .map_err(|e| e.to_string());
            assert_eq!(
                result.as_deref().map_err(String::as_str),
                expected,
                "{link:?}, {terminal}"
            );
            let read = input.len() < piped.len();
            assert_eq!(
                read,
                link.is_none_or(|l| l == "-") && !terminal,
                "{link:?}, {terminal}"
            );
        }
        let mut empty: &[u8] = b"";
        let err = link_or_input(None, false, &mut empty).unwrap_err();
        assert_eq!(err.to_string(), NO_LINK);
    }

    /// A budget of the estimate rounded up to MiB passes; one MiB less is
    /// refused, with a message naming the option.
    #[test]
    fn the_budget_is_in_mib_and_inclusive() {
        let header = Params::default().header(16 * MIB).unwrap();
        let mib = working_memory(&header).unwrap().div_ceil(MIB);
        assert!(within_budget(&header, mib, RAISE_THE_BUDGET).is_ok());
        let err = within_budget(&header, mib - 1, RAISE_THE_BUDGET)
            .unwrap_err()
            .to_string();
        assert!(err.contains(&format!("about {mib} MiB")), "{err}");
        assert!(err.ends_with(RAISE_THE_BUDGET), "{err}");
        let err = within_budget(&header, mib - 1, SHRINK_THE_FILE)
            .unwrap_err()
            .to_string();
        assert!(err.ends_with(SHRINK_THE_FILE), "{err}");
    }

    /// An estimate that overflowed is refused even at the largest budget,
    /// and one just under it passes (review W-069).
    #[test]
    fn an_estimate_that_overflowed_is_refused() {
        assert!(check_budget(None, u64::MAX, RAISE_THE_BUDGET).is_err());
        assert!(check_budget(Some(u64::MAX), u64::MAX, RAISE_THE_BUDGET).is_ok());
    }
}
