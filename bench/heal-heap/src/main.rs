//! Prints heal's peak live heap in bytes, counted exactly by dhat rather than
//! read from the resident set size, which varies by several MiB between
//! identical runs. Run through `bench/heal-heap.sh`.
//!
//! ```text
//! neoblast-heal-heap <verify link> <store dir> <missing>
//! ```
//!
//! Heals as the CLI does: the manifest is read and decoded in place, then
//! healed with `heal_manifest`. The first `<missing>` shards of every segment
//! are treated as missing and the rebuilt shards are counted and dropped, so
//! the store is only read, and every run heals the same shards.

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

use std::io;
use std::process::ExitCode;

use neoblast::store::{DirStore, manifest_len};
use neoblast::{FileVerifyCap, Manifest, ShardSink, ShardSource};

/// The store with its first `missing` shards of every segment hidden.
struct Damaged<'a> {
    store: &'a DirStore,
    missing: u16,
}

impl ShardSource for Damaged<'_> {
    fn get(&self, segment: u32, index: u16, len: usize) -> Option<Vec<u8>> {
        (index >= self.missing)
            .then(|| self.store.get(segment, index, len))
            .flatten()
    }
}

/// Counts rebuilt shards and drops them.
struct Count(usize);

impl ShardSink for Count {
    fn put(&mut self, _: u32, _: u16, _: &[u8]) -> io::Result<()> {
        self.0 += 1;
        Ok(())
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [link, dir, missing] = args.as_slice() else {
        return Err("usage: neoblast-heal-heap <verify link> <store dir> <missing>".into());
    };
    let cap = FileVerifyCap::parse(link)?;
    let store = DirStore::new(dir);
    let source = Damaged {
        store: &store,
        missing: missing.parse()?,
    };
    let _profiler = dhat::Profiler::builder().testing().build();
    let manifest = Manifest::decode_owned(store.read_manifest()?, &cap.root)?;
    let mut sink = Count(0);
    let rebuilt = neoblast::heal_manifest(&cap, &manifest, &source, &mut sink)?;
    let stats = dhat::HeapStats::get();
    let manifest = manifest_len(manifest.header()).ok_or("manifest length overflows")?;
    println!(
        "rebuilt={rebuilt} peak_heap={} manifest={manifest}",
        stats.max_bytes
    );
    Ok(())
}
