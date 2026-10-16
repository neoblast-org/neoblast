//! The `neoblast` command line: streaming seal, heal and open on real files.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::scratch;

fn neoblast(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_neoblast"))
        .args(args)
        .output()
        .unwrap()
}

fn shard(store: &Path, segment: u32, index: u16) -> PathBuf {
    store
        .join(format!("seg-{segment:06}"))
        .join(format!("{index:03}.shard"))
}

#[test]
fn seal_heal_open_round_trip_and_failed_open_leaves_no_file() {
    let dir = scratch("cli-round-trip");
    let input = dir.join("in.bin");
    let data: Vec<u8> = (0..300_000u32).map(|i| (i * 7 % 251) as u8).collect();
    fs::write(&input, &data).unwrap();
    let store = dir.join("store");
    let s = |p: &Path| p.to_str().unwrap().to_owned();

    let out = neoblast(&[
        "seal",
        &s(&input),
        "--out",
        &s(&store),
        "--k",
        "4",
        "--n",
        "7",
        "--segment-size",
        "65536",
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let link = String::from_utf8(out.stdout).unwrap().trim().to_owned();
    let verify = link.replace("nbl://f/", "nbl://fv/");
    let verify = verify.split('#').next().unwrap();

    // Lose 3 of 7 shards in every segment, heal, then open from the data shards.
    let segments = fs::read_dir(&store).unwrap().filter(|e| {
        e.as_ref()
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("seg-")
    });
    let segments = segments.count() as u32;
    assert!(segments > 1);
    for seg in 0..segments {
        for i in [0, 3, 6] {
            fs::remove_file(shard(&store, seg, i)).unwrap();
        }
    }
    let out = neoblast(&["heal", verify, "--from", &s(&store)]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(shard(&store, segments - 1, 6).exists());

    let recovered = dir.join("out.bin");
    let out = neoblast(&["open", &link, "--from", &s(&store), "--out", &s(&recovered)]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(fs::read(&recovered).unwrap(), data);

    // Make the last segment unrecoverable: the open fails, and neither the
    // output file nor a leftover partial file exists.
    for i in 0..4 {
        fs::remove_file(shard(&store, segments - 1, i)).unwrap();
    }
    let failed = dir.join("failed.bin");
    let out = neoblast(&["open", &link, "--from", &s(&store), "--out", &s(&failed)]);
    assert!(!out.status.success());
    assert!(!failed.exists());
    assert!(!dir.join("failed.bin.partial").exists());
}

fn sorted_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// Seals a small file and returns (link, store).
fn sealed_store(dir: &Path) -> (String, PathBuf) {
    let input = dir.join("in.bin");
    fs::write(&input, vec![5u8; 50_000]).unwrap();
    let store = dir.join("store");
    let out = neoblast(&[
        "seal",
        input.to_str().unwrap(),
        "--out",
        store.to_str().unwrap(),
        "--k",
        "2",
        "--n",
        "3",
        "--segment-size",
        "16384",
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    (
        String::from_utf8(out.stdout).unwrap().trim().to_owned(),
        store,
    )
}

#[test]
fn open_refuses_a_directory_and_a_failed_open_changes_nothing() {
    let dir = scratch("cli-open-failures");
    let (link, store) = sealed_store(&dir);
    let store_arg = store.to_str().unwrap();

    // Output is an existing directory: refused, nothing written.
    let target = dir.join("target-dir");
    fs::create_dir(&target).unwrap();
    let before = sorted_names(&dir);
    let out = neoblast(&[
        "open",
        &link,
        "--from",
        store_arg,
        "--out",
        target.to_str().unwrap(),
    ]);
    assert!(!out.status.success());
    assert_eq!(sorted_names(&dir), before);
    assert!(fs::read_dir(&target).unwrap().next().is_none());

    // A failed open keeps an existing output file and leaves no partial file,
    // and a user's own *.partial file next to it is untouched.
    let existing = dir.join("existing.bin");
    fs::write(&existing, b"old contents").unwrap();
    fs::write(dir.join("existing.bin.partial"), b"mine").unwrap();
    for i in 0..2 {
        fs::remove_file(shard(&store, 0, i)).unwrap();
    }
    let before = sorted_names(&dir);
    let out = neoblast(&[
        "open",
        &link,
        "--from",
        store_arg,
        "--out",
        existing.to_str().unwrap(),
    ]);
    assert!(!out.status.success());
    assert_eq!(sorted_names(&dir), before);
    assert_eq!(fs::read(&existing).unwrap(), b"old contents");
    assert_eq!(fs::read(dir.join("existing.bin.partial")).unwrap(), b"mine");
}

#[test]
fn seal_refuses_a_non_empty_store_unless_forced() {
    let dir = scratch("cli-seal-force");
    let (_, store) = sealed_store(&dir);
    let manifest_before = fs::read(store.join("manifest.nbl")).unwrap();
    let input = dir.join("other.bin");
    fs::write(&input, b"another file").unwrap();
    let args = |force: bool| {
        let mut a = vec![
            "seal",
            input.to_str().unwrap(),
            "--out",
            store.to_str().unwrap(),
        ];
        if force {
            a.push("--force");
        }
        neoblast(&a)
    };
    let out = args(false);
    assert!(!out.status.success());
    assert_eq!(
        fs::read(store.join("manifest.nbl")).unwrap(),
        manifest_before
    );
    let out = args(true);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_ne!(
        fs::read(store.join("manifest.nbl")).unwrap(),
        manifest_before
    );
}

/// Planted oversized files are refused by size before they are read.
#[test]
fn oversized_shard_and_manifest_files_are_refused() {
    use neoblast::ShardSource;
    use neoblast::store::DirStore;
    let dir = scratch("cli-oversized");
    let (link, store) = sealed_store(&dir);
    let dir_store = DirStore::new(&store);
    let good = fs::read(shard(&store, 0, 0)).unwrap();
    assert_eq!(dir_store.get(0, 0, good.len()), Some(good.clone()));
    assert_eq!(dir_store.get(0, 0, good.len() - 1), None);

    // A shard grown by one byte is treated as missing, and heal replaces it.
    let mut bigger = good.clone();
    bigger.push(0);
    fs::write(shard(&store, 0, 0), &bigger).unwrap();
    let verify = link.replace("nbl://f/", "nbl://fv/");
    let verify = verify.split('#').next().unwrap();
    let out = neoblast(&["heal", verify, "--from", store.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(fs::read(shard(&store, 0, 0)).unwrap(), good);

    // A manifest with trailing bytes is refused before it is parsed.
    let mut manifest = fs::read(store.join("manifest.nbl")).unwrap();
    manifest.push(0);
    fs::write(store.join("manifest.nbl"), &manifest).unwrap();
    assert!(dir_store.read_manifest().is_err());
}

#[test]
fn seal_refuses_a_file_whose_manifest_open_could_not_read() {
    // Code-review W-001: open and heal refuse manifests over MAX_MANIFEST_LEN,
    // so seal refuses to make one, before writing anything.
    let dir = scratch("cli-manifest-too-large");
    let input = dir.join("big.bin");
    fs::write(&input, vec![0u8; 5_000_000]).unwrap();
    let store = dir.join("store");
    let out = neoblast(&[
        "seal",
        input.to_str().unwrap(),
        "--out",
        store.to_str().unwrap(),
        "--k",
        "2",
        "--n",
        "128",
        "--segment-size",
        "64",
    ]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("--segment-size"));
    assert!(!store.exists());

    // Nor does --force remove an existing store's manifest first.
    let (_, existing) = sealed_store(&dir);
    let before = fs::read(existing.join("manifest.nbl")).unwrap();
    let out = neoblast(&[
        "seal",
        input.to_str().unwrap(),
        "--out",
        existing.to_str().unwrap(),
        "--force",
        "--k",
        "2",
        "--n",
        "128",
        "--segment-size",
        "64",
    ]);
    assert!(!out.status.success());
    assert_eq!(fs::read(existing.join("manifest.nbl")).unwrap(), before);
}

#[cfg(unix)]
#[test]
fn seal_reads_a_pipe() {
    use std::io::Write;
    use std::process::Stdio;
    let dir = scratch("cli-pipe");
    let store = dir.join("store");
    let data = vec![42u8; 70_000];
    let mut child = Command::new(env!("CARGO_BIN_EXE_neoblast"))
        .args(["seal", "/dev/stdin", "--out", store.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&data).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let link = String::from_utf8(out.stdout).unwrap().trim().to_owned();
    let recovered = dir.join("out.bin");
    let out = neoblast(&[
        "open",
        &link,
        "--from",
        store.to_str().unwrap(),
        "--out",
        recovered.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(fs::read(&recovered).unwrap(), data);
}
