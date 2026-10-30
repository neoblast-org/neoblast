//! One test per issue found in the review of the first Phase 1 prototype.
//! Each test's doc comment describes the issue. These must never be deleted.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::MemStore;
use neoblast::{Error, Header, Params, heal, open, seal, seal_with_key};

const SMALL: Params = Params {
    k: 4,
    n: 7,
    segment_size: 256,
};

/// Issue 1: k and n come only from the committed header. Forging them in the
/// manifest changes the root, so the forged manifest is rejected outright.
#[test]
fn issue_01_forged_parameters_are_rejected() {
    let sealed = seal(b"payload", SMALL).unwrap();
    let mut forged = sealed.manifest.encode();
    forged[5..7].copy_from_slice(&9u16.to_be_bytes()); // k = 9 > n = 7
    let store = MemStore::from_sealed(&sealed);
    assert!(open(&sealed.cap, &forged, &store).is_err());
    assert!(heal(&sealed.cap.verify_cap(), &forged, &store).is_err());
}

/// Issue 2: a corrupt shard is skipped, not fatal. One bad peer cannot block healing.
#[test]
fn issue_02_bad_shard_does_not_block_heal() {
    let sealed = seal(&[1u8; 900], SMALL).unwrap();
    let manifest = sealed.manifest.encode();
    let original = MemStore::from_sealed(&sealed);
    let mut store = original.clone();
    store.0.remove(&(0, 0));
    store.0.get_mut(&(0, 1)).unwrap()[0] ^= 0xFF; // tampered
    let rebuilt = heal(&sealed.cap.verify_cap(), &manifest, &store).unwrap();
    let fixed: Vec<_> = rebuilt
        .iter()
        .filter(|r| r.segment == 0)
        .map(|r| r.index)
        .collect();
    assert_eq!(fixed, vec![0, 1]);
    for r in &rebuilt {
        assert_eq!(Some(&r.bytes), original.0.get(&(r.segment, r.index)));
    }
    assert_eq!(
        open(&sealed.cap, &manifest, &store).unwrap(),
        vec![1u8; 900]
    );
}

/// Issue 3: out-of-range values never reach the decoder; truncated or padded
/// manifests are rejected without panicking.
#[test]
fn issue_03_truncated_or_extended_manifest_rejected() {
    let sealed = seal(b"payload", SMALL).unwrap();
    let bytes = sealed.manifest.encode();
    let store = MemStore::from_sealed(&sealed);
    for cut in [0, 1, 31, 32, bytes.len() - 1] {
        assert!(open(&sealed.cap, &bytes[..cut], &store).is_err());
    }
    let mut longer = bytes.clone();
    longer.push(0);
    assert!(open(&sealed.cap, &longer, &store).is_err());
}

/// Issue 4: the exact payload length is not visible to storage nodes. Payloads of
/// different lengths in the same Padmé bucket produce identical headers.
#[test]
fn issue_04_length_hidden_by_padding() {
    let a = Header::new(1000, 4, 7, 4096).unwrap();
    let b = Header::new(1010, 4, 7, 4096).unwrap();
    assert_eq!(a.encode(), b.encode());
}

/// Issue 5: large files are split into independently sealed segments.
#[test]
fn issue_05_multi_segment_files() {
    let payload: Vec<u8> = (0..5000u32).map(|i| (i % 251) as u8).collect();
    let sealed = seal(&payload, SMALL).unwrap();
    assert!(sealed.manifest.header().segment_count() > 10);
    let store = MemStore::from_sealed(&sealed);
    assert_eq!(
        open(&sealed.cap, &sealed.manifest.encode(), &store).unwrap(),
        payload
    );
}

/// Issue 6: segments are bound to their file and position (header as AAD, index
/// and last flag in the nonce). Moving shards between files fails.
#[test]
fn issue_06_segments_cannot_move_between_files() {
    let key = [5u8; 32];
    let a = seal_with_key(&[0xAA; 600], SMALL, &key).unwrap();
    let b = seal_with_key(&[0xBB; 600], SMALL, &key).unwrap();
    // b's shards under a's manifest: every hash check fails, so nothing opens.
    let store = MemStore::from_sealed(&b);
    let err = open(&a.cap, &a.manifest.encode(), &store).unwrap_err();
    assert!(matches!(err, Error::Unrecoverable { .. }));
}

/// Issue 7: sealing is deterministic given the read key, so golden vectors exist.
#[test]
fn issue_07_deterministic_given_key() {
    let a = seal_with_key(b"same", SMALL, &[1; 32]).unwrap();
    let b = seal_with_key(b"same", SMALL, &[1; 32]).unwrap();
    assert_eq!(a.manifest.encode(), b.manifest.encode());
    assert_eq!(a.shards, b.shards);
}

/// Issue 8: odd tree nodes are promoted, never paired with themselves.
/// Covered in `tree::tests::odd_node_is_not_self_paired`; re-checked here via n = 7.
#[test]
fn issue_08_odd_shard_count_has_unique_root() {
    let a = seal_with_key(b"x", SMALL, &[1; 32]).unwrap();
    let b = seal_with_key(b"x", Params { n: 8, ..SMALL }, &[1; 32]).unwrap();
    assert_ne!(a.cap.root, b.cap.root);
}

/// Issue 9: opening needs only k shards per segment and never re-encodes parity.
#[test]
fn issue_09_open_uses_only_data_path() {
    let sealed = seal(&[3u8; 700], SMALL).unwrap();
    let mut store = MemStore::from_sealed(&sealed);
    // Keep only the k data shards; all parity gone.
    store.0.retain(|&(_, i), _| i < SMALL.k);
    assert_eq!(
        open(&sealed.cap, &sealed.manifest.encode(), &store).unwrap(),
        vec![3u8; 700]
    );
}

/// Issue 10, revised by decision 0006: shards are padded to a multiple of 64 bytes,
/// the Leopard-RS alignment, so our bytes stay reproducible by Leopard itself.
#[test]
fn issue_10_shard_lengths_are_64_byte_multiples() {
    // 333 bytes over 256-byte segments: a full segment plus a shorter last one.
    let sealed = seal(&[0u8; 333], SMALL).unwrap();
    let header = sealed.manifest.header();
    assert!(sealed.shards.len() >= 2);
    for (i, segment) in sealed.shards.iter().enumerate() {
        let expected = header.segment_shard_len(i as u32).unwrap() as usize;
        for shard in segment {
            assert_eq!(shard.len() % 64, 0);
            assert_eq!(shard.len(), expected);
        }
    }
}

/// Decision 0006: the format crate and the erasure crate enforce the same rules.
#[test]
fn decision_0006_rules_agree_across_crates() {
    assert_eq!(
        neoblast_format::layout::SHARD_ALIGN,
        neoblast_erasure::SHARD_ALIGN as u64
    );
    assert_eq!(
        usize::from(neoblast_format::header::MIN_K),
        neoblast_erasure::MIN_K
    );
}

// Findings of the first full code and security reviews (2026-10-14), from the
// worklists in `plans/`. They use a real directory, because `DirStore` is the fix.

mod dir {
    pub use super::common::scratch;

    /// Seals 3 KB 2-of-4 into `<dir>/store`; returns the cap, store and manifest.
    pub fn sealed(
        dir: &std::path::Path,
    ) -> (neoblast::FileReadCap, neoblast::store::DirStore, Vec<u8>) {
        use neoblast::store::DirStore;
        use neoblast::{Params, seal_stream};
        let mut store = DirStore::new(dir.join("store"));
        let data = vec![7u8; 3000];
        let params = Params {
            k: 2,
            n: 4,
            segment_size: 1024,
        };
        let (cap, manifest) = seal_stream(&data[..], 3000, params, &mut store).unwrap();
        store.write_manifest(&manifest).unwrap();
        let manifest = store.read_manifest().unwrap();
        (cap, store, manifest)
    }
}

/// S-001: a symlink planted in place of a shard is replaced by the rebuilt shard;
/// heal never writes through it, whether its target exists or not.
#[cfg(unix)]
#[test]
fn s001_heal_replaces_planted_symlinks_instead_of_following_them() {
    use std::os::unix::fs::symlink;
    let tmp = dir::scratch("s001-symlink");
    let (cap, store, manifest) = dir::sealed(&tmp);
    let victim = tmp.join("victim.txt");
    std::fs::write(&victim, b"must not change").unwrap();
    let absent = tmp.join("absent.txt");
    let good = std::fs::read(store.shard_path(0, 0)).unwrap();
    std::fs::remove_file(store.shard_path(0, 0)).unwrap();
    symlink(&victim, store.shard_path(0, 0)).unwrap();
    std::fs::remove_file(store.shard_path(1, 0)).unwrap();
    symlink(&absent, store.shard_path(1, 0)).unwrap();

    let rebuilt =
        neoblast::heal_stream(&cap.verify_cap(), &manifest, &store, &mut store.clone()).unwrap();
    assert_eq!(rebuilt, 2);
    assert_eq!(std::fs::read(&victim).unwrap(), b"must not change");
    assert!(!absent.exists());
    let meta = std::fs::symlink_metadata(store.shard_path(0, 0)).unwrap();
    assert!(meta.is_file());
    assert_eq!(std::fs::read(store.shard_path(0, 0)).unwrap(), good);
}

/// S-001: a segment directory that is a symlink is neither read nor written:
/// open cannot use the shards behind it, and heal writes nothing there.
#[cfg(unix)]
#[test]
fn s001_heal_refuses_a_symlinked_segment_directory() {
    use std::os::unix::fs::symlink;
    let tmp = dir::scratch("s001-segment-dir");
    let (cap, store, manifest) = dir::sealed(&tmp);
    // The segment moves outside the store, one shard short, behind a symlink.
    // Reads refuse the symlinked directory, so heal finds the segment
    // unrecoverable and never writes; the write guard has its own test (W-008).
    let outside = tmp.join("outside");
    let segment = store.shard_path(0, 0).parent().unwrap().to_owned();
    std::fs::rename(&segment, &outside).unwrap();
    std::fs::remove_file(outside.join("000.shard")).unwrap();
    symlink(&outside, &segment).unwrap();

    let mut out = Vec::new();
    assert!(neoblast::open_stream(&cap, &manifest, &store, &mut out).is_err());
    assert!(
        neoblast::heal_stream(&cap.verify_cap(), &manifest, &store, &mut store.clone()).is_err()
    );
    assert!(!outside.join("000.shard").exists());
    assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 3);
}

/// S-001: a FIFO in place of a shard is treated as missing, so heal and open
/// finish instead of blocking on it.
#[cfg(unix)]
#[test]
fn s001_a_fifo_shard_does_not_hang_heal_or_open() {
    use std::sync::mpsc;
    use std::time::Duration;
    let tmp = dir::scratch("s001-fifo");
    let (cap, store, manifest) = dir::sealed(&tmp);
    std::fs::remove_file(store.shard_path(0, 0)).unwrap();
    let made = std::process::Command::new("mkfifo")
        .arg(store.shard_path(0, 0))
        .status()
        .unwrap();
    assert!(made.success());

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut out = Vec::new();
        let opened = neoblast::open_stream(&cap, &manifest, &store, &mut out).is_ok();
        let healed =
            neoblast::heal_stream(&cap.verify_cap(), &manifest, &store, &mut store.clone());
        tx.send((opened, out == vec![7u8; 3000], healed.ok()))
            .unwrap();
    });
    let result = rx.recv_timeout(Duration::from_secs(20));
    assert_eq!(
        result,
        Ok((true, true, Some(1))),
        "heal or open hung on a FIFO"
    );
}

/// W-001: a manifest larger than `MAX_MANIFEST_LEN` is refused before it is
/// read, even when its size matches its own header (a sparse file costs an
/// attacker nothing).
#[test]
fn w001_an_oversized_manifest_is_refused_unread() {
    use neoblast::store::{DirStore, MANIFEST_FILE, MAX_MANIFEST_LEN, manifest_len};
    let tmp = dir::scratch("w001-manifest");
    // 64-byte segments with n = 128: about 4 KiB of hashes per 64 bytes of file.
    let header = Header::new(5_000_000, 2, 128, 64).unwrap();
    let len = manifest_len(&header).unwrap();
    assert!(len > MAX_MANIFEST_LEN);
    let file = std::fs::File::create(tmp.join(MANIFEST_FILE)).unwrap();
    std::io::Write::write_all(&mut &file, &header.encode()).unwrap();
    file.set_len(len).unwrap();

    let err = DirStore::new(&tmp).read_manifest().unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
}

/// W-003: a shard heal cannot write, here because a directory was planted at
/// its path, is skipped. Every other missing shard, in every segment, is still
/// rebuilt, and the error names the path.
#[test]
fn w003_an_unwritable_shard_does_not_stop_heal() {
    let tmp = dir::scratch("w003-unwritable");
    let (cap, store, manifest) = dir::sealed(&tmp);
    let segments = neoblast::Manifest::decode(&manifest, &cap.root)
        .unwrap()
        .header()
        .segment_count();
    assert!(segments > 1);
    let mut lost = Vec::new();
    for segment in 0..segments {
        for index in [0, 1] {
            let path = store.shard_path(segment, index);
            lost.push((std::fs::read(&path).unwrap(), path.clone()));
            std::fs::remove_file(&path).unwrap();
        }
    }
    let planted = store.shard_path(0, 0);
    std::fs::create_dir(&planted).unwrap();
    std::fs::write(planted.join("keep"), b"not a shard").unwrap();

    let err = neoblast::heal_stream(&cap.verify_cap(), &manifest, &store, &mut store.clone())
        .unwrap_err();
    assert!(matches!(err, Error::Io(_)), "{err}");
    assert!(
        err.to_string().contains(&planted.display().to_string()),
        "{err}"
    );
    for (bytes, path) in lost.iter().filter(|(_, path)| *path != planted) {
        assert_eq!(&std::fs::read(path).unwrap(), bytes, "{}", path.display());
    }
    assert!(std::fs::symlink_metadata(&planted).unwrap().is_dir());
}

/// W-004: a file planted at the temporary name heal used to write first
/// (`<shard>.<pid>.tmp`) no longer makes the write fail. Temporary names are
/// now random, and none is left behind.
#[test]
fn w004_planted_temporary_names_do_not_stop_heal() {
    let tmp = dir::scratch("w004-tmp-names");
    let (cap, store, manifest) = dir::sealed(&tmp);
    let path = store.shard_path(0, 0);
    let good = std::fs::read(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    let mut planted = path.clone().into_os_string();
    planted.push(format!(".{}.tmp", std::process::id()));
    std::fs::write(&planted, b"planted").unwrap();

    let rebuilt =
        neoblast::heal_stream(&cap.verify_cap(), &manifest, &store, &mut store.clone()).unwrap();
    assert_eq!(rebuilt, 1);
    assert_eq!(std::fs::read(&path).unwrap(), good);
    assert_eq!(std::fs::read(&planted).unwrap(), b"planted");
    let segment = path.parent().unwrap();
    assert_eq!(std::fs::read_dir(segment).unwrap().count(), 4 + 1);
}

/// W-008: no shard is written into a segment directory that is a symlink. A
/// seal into a store with one planted reaches `write_shard`'s own guard, which
/// the S-001 heal test above does not: without it, shards land outside.
#[cfg(unix)]
#[test]
fn w008_seal_never_writes_through_a_symlinked_segment_directory() {
    use std::os::unix::fs::symlink;
    let tmp = dir::scratch("w008-segment-dir");
    let outside = tmp.join("outside");
    std::fs::create_dir(&outside).unwrap();
    let root = tmp.join("store");
    std::fs::create_dir(&root).unwrap();
    symlink(&outside, root.join("seg-000000")).unwrap();
    let mut store = neoblast::store::DirStore::new(&root);
    let params = Params {
        k: 2,
        n: 4,
        segment_size: 1024,
    };

    let err = neoblast::seal_stream(&[7u8; 3000][..], 3000, params, &mut store).unwrap_err();
    assert!(err.to_string().contains("seg-000000"), "{err}");
    assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 0);
}

/// W-003: since heal now goes on past a shard it cannot write, a segment that
/// cannot be recovered later on is still the error reported: it means the file
/// is lost, which a failed write alone does not.
#[test]
fn w003_a_lost_segment_outranks_a_failed_write() {
    let tmp = dir::scratch("w003-lost-segment");
    let (cap, store, manifest) = dir::sealed(&tmp);
    std::fs::remove_file(store.shard_path(0, 0)).unwrap();
    std::fs::create_dir(store.shard_path(0, 0)).unwrap();
    for index in 0..3 {
        std::fs::remove_file(store.shard_path(2, index)).unwrap();
    }

    let err = neoblast::heal_stream(&cap.verify_cap(), &manifest, &store, &mut store.clone())
        .unwrap_err();
    assert!(
        matches!(
            err,
            Error::Unrecoverable {
                segment: 2,
                have: 1,
                need: 2
            }
        ),
        "{err}"
    );
}

/// W-015: a store can be made in a directory that can be written to but not
/// read, such as a shared drop box. Syncing the new store into it is skipped,
/// as it cannot be opened, instead of failing the seal.
#[cfg(unix)]
#[test]
fn w015_seal_into_an_unreadable_parent() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = dir::scratch("w015-drop-box");
    let drop_box = tmp.join("drop-box");
    std::fs::create_dir(&drop_box).unwrap();
    std::fs::set_permissions(&drop_box, std::fs::Permissions::from_mode(0o333)).unwrap();
    let mut store = neoblast::store::DirStore::new(drop_box.join("store"));
    let params = Params {
        k: 2,
        n: 4,
        segment_size: 1024,
    };

    let sealed = neoblast::seal_stream(&[7u8; 3000][..], 3000, params, &mut store)
        .and_then(|(_, manifest)| Ok(store.write_manifest(&manifest)?));
    std::fs::set_permissions(&drop_box, std::fs::Permissions::from_mode(0o755)).unwrap();
    sealed.unwrap();
    assert!(store.manifest_path().is_file());
}

/// W-015: only a directory outside the store is left unsynced when it cannot
/// be read. A store's own directory that cannot be read fails the seal, as its
/// new segment directories could not be made durable.
#[cfg(unix)]
#[test]
fn w015_an_unreadable_store_directory_is_not_skipped() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = dir::scratch("w015-store-dir");
    let root = tmp.join("store");
    std::fs::create_dir(&root).unwrap();
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o333)).unwrap();
    // Run as root, the directory can be read anyway; there is nothing to test.
    if std::fs::File::open(&root).is_ok() {
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }
    let mut store = neoblast::store::DirStore::new(&root);
    let params = Params {
        k: 2,
        n: 4,
        segment_size: 1024,
    };

    let sealed = neoblast::seal_stream(&[7u8; 3000][..], 3000, params, &mut store);
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o755)).unwrap();
    let err = sealed.unwrap_err();
    assert!(
        err.to_string()
            .starts_with(&format!("cannot sync {}", root.display())),
        "{err}"
    );
}

/// W-018: a directory that cannot be made is named in the error, instead of
/// the shard that was to be written into it.
#[cfg(unix)]
#[test]
fn w018_a_directory_that_cannot_be_made_is_named() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = dir::scratch("w018-names");
    let read_only = tmp.join("read-only");
    std::fs::create_dir(&read_only).unwrap();
    std::fs::set_permissions(&read_only, std::fs::Permissions::from_mode(0o555)).unwrap();
    let mut store = neoblast::store::DirStore::new(read_only.join("store"));
    let params = Params {
        k: 2,
        n: 4,
        segment_size: 1024,
    };

    // Run as root, the directory can be made anyway; there is nothing to test.
    if std::fs::create_dir(read_only.join("probe")).is_ok() {
        return;
    }
    let sealed = neoblast::seal_stream(&[7u8; 3000][..], 3000, params, &mut store);
    std::fs::set_permissions(&read_only, std::fs::Permissions::from_mode(0o755)).unwrap();
    let err = sealed.unwrap_err();
    let segment = read_only.join("store").join("seg-000000");
    assert!(
        err.to_string()
            .starts_with(&format!("cannot create {}", segment.display())),
        "{err}"
    );
}

/// S-011: a publisher can commit to shards that are not one codeword, so a
/// shard rebuilt from the others does not match the manifest. Heal skips it
/// and still repairs every other segment, then reports it; what it wrote is
/// still made durable (W-017).
#[test]
fn s011_an_inconsistent_segment_does_not_stop_heal() {
    // Segment 0's parity no longer matches its data, and the manifest says so.
    let (sealed, manifest) = sealed_as(|shards| {
        for parity in &mut shards[0][2..] {
            parity.iter_mut().for_each(|b| *b ^= 0x5a);
        }
    });
    let cap = neoblast::FileVerifyCap {
        root: manifest.root(),
    };
    let mut store = MemStore::from_sealed(&sealed);
    store.0.remove(&(0, 0));
    store.0.remove(&(2, 1));

    let mut sink = Recorder::new(false);
    let err = neoblast::heal_manifest(&cap, &manifest, &store, &mut sink).unwrap_err();
    assert!(
        matches!(
            err,
            Error::RebuildMismatch {
                segment: 0,
                index: 0
            }
        ),
        "{err}"
    );
    assert_eq!(sink.put, [(2, 1)]);
    assert!(sink.finished);
}

/// A sink that records what it is given, failing every write and the final
/// sync when `fail` is set.
struct Recorder {
    put: Vec<(u32, u16)>,
    finished: bool,
    fail: bool,
}

impl Recorder {
    fn new(fail: bool) -> Self {
        Self {
            put: Vec::new(),
            finished: false,
            fail,
        }
    }

    fn result(&self) -> std::io::Result<()> {
        match self.fail {
            true => Err(std::io::Error::other("sink failed")),
            false => Ok(()),
        }
    }
}

impl neoblast::ShardSink for Recorder {
    fn put(&mut self, segment: u32, index: u16, _: &[u8]) -> std::io::Result<()> {
        self.put.push((segment, index));
        self.result()
    }
    fn finish(&mut self) -> std::io::Result<()> {
        self.finished = true;
        self.result()
    }
}

/// Seals 3 KB 2-of-4 under a fixed key, lets `change` alter the shards, and
/// returns them with a manifest that commits to them as they now are.
fn sealed_as(
    change: impl FnOnce(&mut Vec<Vec<Vec<u8>>>),
) -> (neoblast::Sealed, neoblast::Manifest) {
    let params = Params {
        k: 2,
        n: 4,
        segment_size: 1024,
    };
    let mut sealed = seal_with_key(&[9u8; 3000], params, &[3u8; 32]).unwrap();
    assert!(sealed.shards.len() > 2);
    change(&mut sealed.shards);
    let hashes = sealed
        .shards
        .iter()
        .map(|segment| {
            segment
                .iter()
                .map(|s| neoblast::tree::shard_hash(s))
                .collect()
        })
        .collect();
    let manifest = neoblast::Manifest::new(*sealed.manifest.header(), hashes).unwrap();
    (sealed, manifest)
}

/// W-024: a lost or inconsistent segment is reported over a write that failed
/// earlier in the run, and over a failed final sync, which still runs (W-017).
#[test]
fn w024_an_inconsistent_segment_outranks_a_failed_write() {
    // Segment 2's parity no longer matches its data, and the manifest says so.
    let (sealed, manifest) = sealed_as(|shards| {
        for parity in &mut shards[2][2..] {
            parity.iter_mut().for_each(|b| *b ^= 0x5a);
        }
    });
    let cap = neoblast::FileVerifyCap {
        root: manifest.root(),
    };
    let mut store = MemStore::from_sealed(&sealed);
    store.0.remove(&(0, 1));
    store.0.remove(&(2, 0));

    let mut sink = Recorder::new(true);
    let err = neoblast::heal_manifest(&cap, &manifest, &store, &mut sink).unwrap_err();
    assert!(
        matches!(
            err,
            Error::RebuildMismatch {
                segment: 2,
                index: 0
            }
        ),
        "{err}"
    );
    assert_eq!(sink.put, [(0, 1)]);
    assert!(sink.finished);
}

/// S-019: a publisher can commit to a shard of the wrong length. Heal and open
/// treat it as missing, whatever the source returns, instead of handing it to
/// the decoder, which stopped heal there and hid an earlier lost segment.
#[test]
fn s019_a_shard_of_the_wrong_length_counts_as_missing() {
    let (sealed, manifest) = sealed_as(|shards| shards[1][0].extend_from_slice(&[0u8; 64]));
    let read_cap = neoblast::FileReadCap {
        root: manifest.root(),
        read_key: [3u8; 32],
    };
    // `MemStore` hands back whatever it holds, whatever length is asked for.
    let mut store = MemStore::from_sealed(&sealed);
    let mut out = Vec::new();
    neoblast::open_manifest(&read_cap, &manifest, &store, &mut out).unwrap();
    assert_eq!(out, vec![9u8; 3000]);

    // Segment 0 is lost; segment 1 must be rebuilt past its long shard.
    for index in 0..3 {
        store.0.remove(&(0, index));
    }
    store.0.remove(&(1, 3));
    store.0.remove(&(2, 1));
    let mut sink = Recorder::new(false);
    let err =
        neoblast::heal_manifest(&read_cap.verify_cap(), &manifest, &store, &mut sink).unwrap_err();
    assert!(
        matches!(
            err,
            Error::Unrecoverable {
                segment: 0,
                have: 1,
                need: 2
            }
        ),
        "{err}"
    );
    assert_eq!(sink.put, [(1, 3), (2, 1)]);
}

/// W-025: a failed seal still syncs what it wrote, so a `DirStore` sealed
/// into again afterwards does not try to sync a directory of the failed run.
#[test]
fn w025_a_store_can_be_sealed_into_again_after_a_failed_seal() {
    let tmp = dir::scratch("w025-reuse");
    let root = tmp.join("store");
    let mut store = neoblast::store::DirStore::new(&root);
    let params = Params {
        k: 2,
        n: 4,
        segment_size: 1024,
    };
    let err = neoblast::seal_stream(&[7u8; 2500][..], 3000, params, &mut store).unwrap_err();
    assert!(matches!(err, Error::InputLength { .. }), "{err}");

    std::fs::remove_dir_all(&root).unwrap();
    neoblast::seal_stream(&[7u8; 3000][..], 3000, params, &mut store).unwrap();
}

/// W-006: `seal_stream` refuses parameters whose manifest a store could not
/// read back before it hands out a single shard, not after writing them all.
#[test]
fn w006_seal_stream_refuses_an_oversized_manifest_before_any_shard() {
    use std::io::Read;
    /// Fails on the first shard, so a regression fails fast.
    struct NoShards;
    impl neoblast::ShardSink for NoShards {
        fn put(&mut self, _: u32, _: u16, _: &[u8]) -> std::io::Result<()> {
            Err(std::io::Error::other("a shard was written"))
        }
    }
    // 64-byte segments with n = 128: about 4 KiB of hashes per 64 bytes of file.
    let params = Params {
        k: 2,
        n: 128,
        segment_size: 64,
    };
    let input = std::io::repeat(0).take(5_000_000);
    let err = neoblast::seal_stream(input, 5_000_000, params, &mut NoShards).unwrap_err();
    assert!(matches!(err, Error::ManifestTooLarge), "{err}");
    assert!(matches!(
        params.header(5_000_000),
        Err(Error::ManifestTooLarge)
    ));
}
