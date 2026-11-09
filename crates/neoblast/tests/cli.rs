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

#[test]
fn seal_reports_bad_parameters_plainly() {
    // Code-review W-009: k = 0 was reported as an inconsistent layout.
    let dir = scratch("cli-bad-params");
    let input = dir.join("in.bin");
    fs::write(&input, b"data").unwrap();
    let store = dir.join("store");
    let out = neoblast(&[
        "seal",
        input.to_str().unwrap(),
        "--out",
        store.to_str().unwrap(),
        "--k",
        "0",
    ]);
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("invalid k/n"), "{err}");
    assert!(!store.exists());
}

#[cfg(unix)]
#[test]
fn seal_hints_at_the_segment_size_when_segments_overflow() {
    // Code-review W-009: more segments than the header can count (300 GiB in
    // 64-byte segments) gets the same `--segment-size` hint as an oversized
    // manifest. The input is a sparse file, so this needs Unix.
    let dir = scratch("cli-segment-overflow");
    let input = dir.join("huge.bin");
    fs::File::create(&input)
        .unwrap()
        .set_len(300 * 1024 * 1024 * 1024)
        .unwrap();
    let store = dir.join("store");
    let out = neoblast(&[
        "seal",
        input.to_str().unwrap(),
        "--out",
        store.to_str().unwrap(),
        "--segment-size",
        "64",
    ]);
    assert!(!out.status.success());
    // Sparse, but tools that copy target/ may not know that.
    fs::remove_file(&input).unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("--segment-size"), "{err}");
    assert!(!store.exists());
}

#[cfg(unix)]
#[test]
fn seal_refuses_a_non_empty_store_before_reading_a_pipe() {
    // Code-review W-005: a non-empty store without `--force` is refused before
    // the input is read, so a pipe that never ends is not buffered first.
    use std::process::Stdio;
    use std::time::{Duration, Instant};
    let dir = scratch("cli-pipe-refused");
    let (_, store) = sealed_store(&dir);
    let mut child = Command::new(env!("CARGO_BIN_EXE_neoblast"))
        .args(["seal", "/dev/stdin", "--out", store.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // The pipe stays open and empty: reading it would block until the timeout.
    let _stdin = child.stdin.take().unwrap();
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break Some(status);
        }
        if start.elapsed() > Duration::from_secs(20) {
            child.kill().unwrap();
            break None;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let status = status.expect("seal read the pipe before refusing the store");
    assert!(!status.success());
    let mut err = String::new();
    std::io::Read::read_to_string(&mut child.stderr.take().unwrap(), &mut err).unwrap();
    assert!(err.contains("not empty"), "{err}");
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

/// Runs `neoblast` with `stdin` as its standard input.
fn neoblast_stdin(args: &[&str], stdin: &[u8]) -> Output {
    use std::io::Write;
    use std::process::Stdio;
    let mut child = Command::new(env!("CARGO_BIN_EXE_neoblast"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // It stops reading after the first line, or one too long, so a broken
    // pipe here is no failure.
    let _ = child.stdin.take().unwrap().write_all(stdin);
    child.wait_with_output().unwrap()
}

/// S-003: `open` and `heal` read the link from the first line of stdin
/// when it is `-` or left out, so it need not be on the command line, where
/// other local users can see it and the shell keeps it. Each failing case
/// differs from a passing one in one way.
#[test]
fn s003_open_and_heal_read_the_link_from_stdin() {
    let dir = scratch("cli-link-stdin");
    let (link, store) = sealed_store(&dir);
    let store_arg = store.to_str().unwrap();
    let want = vec![5u8; 50_000];
    let line = |end: &str| format!("{link}{end}").into_bytes();
    // A peer hint after `&` is ignored by `parse`, so these lines open the
    // file at any length: 4096 bytes, the newline included, and one more.
    let hint = |len: usize| {
        let mut l = format!("{link}&").into_bytes();
        l.resize(len - 1, b'a');
        l.push(b'\n');
        l
    };
    let mut not_utf8 = format!("{link}&").into_bytes();
    not_utf8.extend(b"\xff\n");
    let cases = [
        ("newline", None, line("\n"), None),
        ("dash", Some("-"), line("\n"), None),
        ("crlf", None, line("\r\n"), None),
        ("cr", None, line("\r"), None),
        ("eof", None, line(""), None),
        ("two-lines", None, line("\nnot a link\n"), None),
        (
            "argument",
            Some(link.as_str()),
            b"not a link\n".to_vec(),
            None,
        ),
        ("longest", None, hint(4096), None),
        ("too-long", None, hint(4097), Some("over 4096 bytes")),
        ("empty", None, Vec::new(), Some("no link")),
        ("blank", None, b"\n".to_vec(), Some("no link")),
        ("dash-empty", Some("-"), Vec::new(), Some("no link")),
        ("not-utf8", None, not_utf8, Some("not UTF-8")),
    ];
    for (name, arg, stdin, error) in cases {
        let out_path = dir.join(format!("{name}.bin"));
        let mut args = vec!["open"];
        args.extend(arg);
        args.extend(["--from", store_arg, "--out", out_path.to_str().unwrap()]);
        let out = neoblast_stdin(&args, &stdin);
        let stderr = String::from_utf8_lossy(&out.stderr);
        match error {
            None => {
                assert!(out.status.success(), "{name}: {stderr}");
                assert_eq!(fs::read(&out_path).unwrap(), want, "{name}");
            }
            Some(error) => {
                assert!(!out.status.success(), "{name}");
                assert!(stderr.contains(error), "{name}: {stderr}");
                assert!(!out_path.exists(), "{name}");
            }
        }
    }

    let verify = link.replace("nbl://f/", "nbl://fv/");
    let verify = verify.split('#').next().unwrap();
    for (name, arg, stdin) in [
        ("verify", None, format!("{verify}\n")),
        ("read", Some("-"), format!("{link}\n")),
        ("argument", Some(verify), "not a link\n".to_owned()),
    ] {
        let mut args = vec!["heal"];
        args.extend(arg);
        args.extend(["--from", store_arg]);
        let out = neoblast_stdin(&args, stdin.as_bytes());
        assert!(
            out.status.success(),
            "heal {name}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let out = neoblast_stdin(&["heal", "--from", store_arg], b"");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("no link"));

    // The help says why.
    for command in ["open", "heal"] {
        let help = neoblast(&[command, "--help"]);
        let help = String::from_utf8(help.stdout).unwrap();
        assert!(help.contains("stdin") && help.contains("history"), "{help}");
    }
}

/// The MiB a refusal says a file needs: `--max-memory 1` is below any file.
fn needed_mib(out: &Output) -> u64 {
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    let (_, rest) = err
        .split_once("needs about ")
        .unwrap_or_else(|| panic!("{err}"));
    rest.split(' ').next().unwrap().parse().unwrap()
}

/// S-037, decision 0011: `seal`, `open` and `heal` refuse a file whose
/// estimated working memory is over `--max-memory`, before writing
/// anything, and take it at exactly its estimate. Each refused run differs
/// from one that passes only by a budget one MiB lower.
#[test]
fn s037_seal_open_and_heal_keep_to_the_memory_budget() {
    let dir = scratch("cli-memory-budget");
    let (link, store) = sealed_store(&dir);
    let store_arg = store.to_str().unwrap();

    // seal: a refused seal leaves its output directory as it was.
    let input = dir.join("in.bin");
    let seal = |out: &Path, mib: &str| {
        neoblast(&[
            "seal",
            input.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
            "--k",
            "2",
            "--n",
            "3",
            "--segment-size",
            "16384",
            "--max-memory",
            mib,
        ])
    };
    let mib = needed_mib(&seal(&dir.join("seal-probe"), "1"));
    assert!(!dir.join("seal-probe").exists());
    let refused = seal(&dir.join("seal-refused"), &(mib - 1).to_string());
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("--max-memory"));
    assert!(!dir.join("seal-refused").exists());
    let sealed = seal(&dir.join("seal-ok"), &mib.to_string());
    assert!(
        sealed.status.success(),
        "{}",
        String::from_utf8_lossy(&sealed.stderr)
    );
    // A refused `seal --force` leaves the store it would replace whole.
    let before = sorted_names(&store);
    let forced = neoblast(&[
        "seal",
        input.to_str().unwrap(),
        "--out",
        store_arg,
        "--k",
        "2",
        "--n",
        "3",
        "--segment-size",
        "16384",
        "--force",
        "--max-memory",
        &(mib - 1).to_string(),
    ]);
    assert!(!forced.status.success());
    assert_eq!(sorted_names(&store), before);
    assert!(store.join("manifest.nbl").exists());

    // open: a refused open writes no output file.
    let open = |out: &Path, mib: &str| {
        neoblast(&[
            "open",
            &link,
            "--from",
            store_arg,
            "--out",
            out.to_str().unwrap(),
            "--max-memory",
            mib,
        ])
    };
    let mib = needed_mib(&open(&dir.join("probe.bin"), "1"));
    let files = sorted_names(&dir);
    assert!(
        !open(&dir.join("refused.bin"), &(mib - 1).to_string())
            .status
            .success()
    );
    // A refused open leaves no file behind, not even a partial one.
    assert_eq!(sorted_names(&dir), files);
    let opened = open(&dir.join("ok.bin"), &mib.to_string());
    assert!(
        opened.status.success(),
        "{}",
        String::from_utf8_lossy(&opened.stderr)
    );

    // heal: a refused heal rebuilds nothing.
    let lost = shard(&store, 0, 0);
    fs::remove_file(&lost).unwrap();
    let heal = |mib: &str| neoblast(&["heal", &link, "--from", store_arg, "--max-memory", mib]);
    let mib = needed_mib(&heal("1"));
    assert!(!heal(&(mib - 1).to_string()).status.success());
    assert!(!lost.exists());
    let healed = heal(&mib.to_string());
    assert!(
        healed.status.success(),
        "{}",
        String::from_utf8_lossy(&healed.stderr)
    );
    assert!(lost.exists());
}
