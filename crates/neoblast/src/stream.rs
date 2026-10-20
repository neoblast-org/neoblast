//! Streaming seal, open and heal: one segment in memory at a time.
//!
//! Peak memory depends on the segment size and k/n, not on the file size,
//! apart from the manifest's shard hashes (`n × 32` bytes per segment, about
//! 0.16% of the file at the defaults, 20-of-50 and 1 MiB segments).
//! [`open_manifest`] and [`heal_manifest`] hold that manifest once:
//! [`Manifest`] keeps its encoding, which [`Manifest::decode_owned`] takes
//! over from the caller. [`open_stream`] and [`heal_stream`] take the
//! manifest's bytes and decode a copy. The in-memory [`seal`](crate::seal),
//! [`open`](crate::open) and [`heal`](crate::heal) are thin wrappers over
//! these functions, so both paths produce identical bytes.
//!
//! [`open_stream`] writes each segment as soon as it is verified and decrypted.
//! Every segment is authenticated on its own (AEAD with the header as associated
//! data and the index and last-segment flag in the nonce), so nothing unverified
//! is ever written. If a later segment cannot be recovered, though, the output so
//! far is incomplete: callers writing to a file should write to a temporary name
//! and rename only on success, as the CLI does.

use std::io::{self, Read, Write};

use neoblast_crypto::{CryptoError, SegmentCipher};
use neoblast_erasure::{ErasureError, ReedSolomonSimd};
use neoblast_format::header::LENGTH_PREFIX;
use neoblast_format::{FileReadCap, FileVerifyCap, Header};
use zeroize::Zeroizing;

use crate::file::{Error, Params};
use crate::manifest::Manifest;
use crate::tree;

/// Somewhere shards can be written: a directory now, peers in Phase 2.
pub trait ShardSink {
    /// Stores one shard. Calls arrive in segment order.
    fn put(&mut self, segment: u32, index: u16, bytes: &[u8]) -> io::Result<()>;

    /// Makes every shard stored so far durable. [`seal_stream`] calls it once,
    /// after the last `put` of a run that succeeds; [`heal_manifest`] once at
    /// the end of every run past its root check, also one that fails (review
    /// W-017). A caller
    /// using `put` directly calls it too. The default does nothing.
    fn finish(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Seals `payload_len` bytes read from `input` under a fresh random read key,
/// handing every shard to `sink` as soon as its segment is coded.
///
/// Returns the read capability and the verify manifest; store the manifest
/// after all shards. Fails with [`Error::InputLength`] if `input` yields more or
/// fewer than `payload_len` bytes, and refuses parameters whose manifest a
/// store could not read back ([`Params::header`]) before handing `sink` any
/// shard.
pub fn seal_stream(
    input: impl Read,
    payload_len: u64,
    params: Params,
    sink: &mut impl ShardSink,
) -> Result<(FileReadCap, Manifest), Error> {
    let key = neoblast_crypto::random_read_key()?;
    seal_stream_with_key(input, payload_len, params, &key, sink)
}

/// [`seal_stream`] under a caller-chosen read key. Never reuse a read key for
/// two different payloads.
pub fn seal_stream_with_key(
    mut input: impl Read,
    payload_len: u64,
    params: Params,
    read_key: &[u8; 32],
    sink: &mut impl ShardSink,
) -> Result<(FileReadCap, Manifest), Error> {
    let header = params.header(payload_len)?;
    let cipher = SegmentCipher::new(read_key, &header);
    let k = usize::from(header.k());
    let n = usize::from(header.n());
    let mut remaining = payload_len;
    // The manifest's encoding: grows segment by segment, never sized up front
    // from a caller's length.
    let mut manifest = header.encode().to_vec();
    // One buffer holds a segment's plaintext, then its ciphertext, then its k
    // zero-padded data shards. It starts as plaintext, so it is wiped on drop.
    let mut buf = Zeroizing::new(Vec::new());

    for segment in 0..header.segment_count() {
        let plain_len = segment_len(header.segment_plain_len(segment))?;
        let shard_len = shard_len(&header, segment)?;
        buf.clear();
        buf.reserve(k.saturating_mul(shard_len).max(plain_len));
        if segment == 0 {
            buf.extend_from_slice(&payload_len.to_be_bytes());
        }
        let room = plain_len.saturating_sub(buf.len()) as u64;
        let take = remaining.min(room);
        let got = (&mut input).take(take).read_to_end(&mut buf)? as u64;
        if got != take {
            return Err(Error::InputLength {
                expected: payload_len,
            });
        }
        remaining -= take;
        buf.resize(plain_len, 0);

        cipher.seal_in_place(segment, &mut buf)?;
        buf.resize(k.saturating_mul(shard_len), 0);
        let parity = neoblast_erasure::encode_parity(&ReedSolomonSimd, &buf, k, n, shard_len)?;

        let shards = buf
            .chunks_exact(shard_len)
            .chain(parity.iter().map(Vec::as_slice));
        for (shard, index) in shards.zip(0u16..) {
            manifest.extend_from_slice(&tree::shard_hash(shard));
            sink.put(segment, index, shard)?;
        }
    }

    // The input must end exactly at `payload_len`.
    let mut probe = [0u8; 1];
    if remaining != 0 || read_some(&mut input, &mut probe)? != 0 {
        return Err(Error::InputLength {
            expected: payload_len,
        });
    }
    sink.finish()?;
    let manifest = Manifest::from_encoding(manifest)?;
    let cap = FileReadCap {
        root: manifest.root(),
        read_key: *read_key,
    };
    Ok((cap, manifest))
}

/// Opens a file segment by segment, writing the payload to `out`. Returns the
/// payload length. Decodes a copy of `manifest_bytes`; [`open_manifest`] with
/// [`Manifest::decode_owned`] holds the manifest once.
///
/// Fetches shards in index order and stops at the first k that match the
/// manifest, so an intact file is read from its data shards alone. On error,
/// `out` may already hold the verified start of the file (see the module docs).
pub fn open_stream(
    cap: &FileReadCap,
    manifest_bytes: &[u8],
    source: &impl crate::ShardSource,
    out: impl Write,
) -> Result<u64, Error> {
    open_manifest(
        cap,
        &Manifest::decode(manifest_bytes, &cap.root)?,
        source,
        out,
    )
}

/// [`open_stream`] with a manifest already decoded, for example by
/// [`Manifest::decode_owned`] so that it is held only once. Fails with
/// [`Error::RootMismatch`] if it is not the manifest of `cap`'s file.
pub fn open_manifest(
    cap: &FileReadCap,
    manifest: &Manifest,
    source: &impl crate::ShardSource,
    mut out: impl Write,
) -> Result<u64, Error> {
    if manifest.root() != cap.root {
        return Err(Error::RootMismatch);
    }
    let header = *manifest.header();
    let cipher = SegmentCipher::new(&cap.read_key, &header);
    let k = usize::from(header.k());
    let n = usize::from(header.n());
    let mut payload_len = 0u64;
    let mut left = 0u64;

    for segment in 0..header.segment_count() {
        let shard_len = shard_len(&header, segment)?;
        let held = first_k_verified(manifest, segment, source, k, shard_len);
        let cipher_len = segment_len(header.segment_cipher_len(segment))?;
        let data: Vec<Vec<u8>> = if held.iter().take(k).all(Option::is_some) {
            // Every data shard is here: no decoding.
            held.into_iter().take(k).flatten().collect()
        } else {
            // Decode first and free the shards fetched for it before the output
            // buffer is allocated.
            let data = neoblast_erasure::reconstruct_data(&ReedSolomonSimd, k, n, shard_len, &held)
                .map_err(|e| unrecoverable(e, segment, &held, k))?;
            drop(held);
            data
        };
        let mut buf = Zeroizing::new(Vec::with_capacity(k.saturating_mul(shard_len)));
        for shard in data {
            buf.extend_from_slice(&shard);
        }
        buf.truncate(cipher_len);
        cipher.open_in_place(segment, &mut buf)?;

        let mut plain: &[u8] = &buf;
        if segment == 0 {
            let (prefix, rest) = plain
                .split_first_chunk::<{ LENGTH_PREFIX as usize }>()
                .ok_or(CryptoError::Framing)?;
            payload_len = u64::from_be_bytes(*prefix);
            // The payload must fit in the framed length; checked before writing.
            if payload_len > header.padded_length().saturating_sub(LENGTH_PREFIX) {
                return Err(CryptoError::Framing.into());
            }
            left = payload_len;
            plain = rest;
        }
        let write = plain.len().min(usize::try_from(left).unwrap_or(usize::MAX));
        let (payload, padding) = plain.split_at(write);
        // Canonical encoding: padding must be all zero.
        if padding.iter().any(|&b| b != 0) {
            return Err(CryptoError::Framing.into());
        }
        out.write_all(payload)?;
        left -= write as u64;
    }
    if left != 0 {
        return Err(CryptoError::Framing.into());
    }
    out.flush()?;
    Ok(payload_len)
}

/// Rebuilds every missing or corrupt shard using only the verify cap, handing
/// each rebuilt shard to `sink` as soon as its segment is done. Returns how
/// many shards were rebuilt. Decodes a copy of `manifest_bytes`;
/// [`heal_manifest`] with [`Manifest::decode_owned`] holds the manifest once.
///
/// A segment with fewer than k good shards is skipped, and so is a shard that
/// `sink` fails to store (review W-003) or a rebuilt shard that does not match
/// the manifest, which a publisher can cause by committing to shards that are
/// not one codeword (reviews S-002 and S-011); everything else is still
/// repaired. Then the first segment that could not be repaired is returned, as
/// [`Error::Unrecoverable`] or [`Error::RebuildMismatch`], since the file is
/// lost or inconsistent there; failing that, the sink's first error.
///
/// Shards that fail their hash check are ignored rather than aborting the heal
/// (code-review issue 2), so one bad peer cannot block repair.
pub fn heal_stream(
    cap: &FileVerifyCap,
    manifest_bytes: &[u8],
    source: &impl crate::ShardSource,
    sink: &mut impl ShardSink,
) -> Result<usize, Error> {
    heal_manifest(
        cap,
        &Manifest::decode(manifest_bytes, &cap.root)?,
        source,
        sink,
    )
}

/// [`heal_stream`] with a manifest already decoded, for example by
/// [`Manifest::decode_owned`] so that it is held only once. Fails with
/// [`Error::RootMismatch`] if it is not the manifest of `cap`'s file.
pub fn heal_manifest(
    cap: &FileVerifyCap,
    manifest: &Manifest,
    source: &impl crate::ShardSource,
    sink: &mut impl ShardSink,
) -> Result<usize, Error> {
    if manifest.root() != cap.root {
        return Err(Error::RootMismatch);
    }
    let mut rebuilt = 0;
    let mut first_failure = None;
    let run = heal_segments(manifest, source, sink, &mut rebuilt, &mut first_failure);
    // Even after a failure, what was written is made durable (review W-017).
    if let Err(e) = sink.finish() {
        first_failure.get_or_insert(Error::Io(e));
    }
    run?;
    match first_failure {
        Some(failure) => Err(failure),
        None => Ok(rebuilt),
    }
}

/// The segments of [`heal_manifest`]: counts each shard stored in `rebuilt`,
/// and keeps in `first_failure` the first segment that could not be repaired
/// or, failing that, the sink's first error. Returns early only on an error
/// that means a bug, not a bad store.
fn heal_segments(
    manifest: &Manifest,
    source: &impl crate::ShardSource,
    sink: &mut impl ShardSink,
    rebuilt: &mut usize,
    first_failure: &mut Option<Error>,
) -> Result<(), Error> {
    // A lost or inconsistent segment outranks a failed write.
    let lost = |failure: &mut Option<Error>, error: Error| {
        if !matches!(
            failure,
            Some(Error::Unrecoverable { .. } | Error::RebuildMismatch { .. })
        ) {
            *failure = Some(error);
        }
    };
    let header = *manifest.header();
    let k = usize::from(header.k());
    let n = usize::from(header.n());
    for segment in 0..header.segment_count() {
        let shard_len = shard_len(&header, segment)?;
        let (held, missing) = verify_all(manifest, segment, source, k, shard_len);
        if missing.is_empty() {
            continue;
        }
        let all = match neoblast_erasure::regenerate(&ReedSolomonSimd, k, n, shard_len, &held) {
            Ok(all) => all,
            Err(e) => match unrecoverable(e, segment, &held, k) {
                // Keep repairing the other segments; report this one at the end.
                failure @ Error::Unrecoverable { .. } => {
                    lost(first_failure, failure);
                    continue;
                }
                other => return Err(other),
            },
        };
        drop(held);
        for index in missing {
            let listed = manifest.shard_hash(segment, index);
            // Not the shard the manifest lists: keep repairing the others
            // (review S-011) and report this one at the end.
            let Some(bytes) = all
                .get(usize::from(index))
                .filter(|b| listed == Some(&tree::shard_hash(b)))
            else {
                lost(first_failure, Error::RebuildMismatch { segment, index });
                continue;
            };
            match sink.put(segment, index, bytes) {
                Ok(()) => *rebuilt += 1,
                // Keep writing the other shards; report this one at the end.
                Err(e) => {
                    first_failure.get_or_insert(Error::Io(e));
                }
            }
        }
    }
    Ok(())
}

/// Fetches shards in index order and keeps the first `k` that match the
/// manifest. Fetching stops there: an intact segment costs exactly k reads.
fn first_k_verified(
    manifest: &Manifest,
    segment: u32,
    source: &impl crate::ShardSource,
    k: usize,
    shard_len: usize,
) -> Vec<Option<Vec<u8>>> {
    let n = manifest.header().n();
    let mut held = Vec::with_capacity(usize::from(n));
    let mut good = 0;
    for index in 0..n {
        let shard = if good < k {
            source.get(segment, index, shard_len).filter(|bytes| {
                manifest.shard_hash(segment, index) == Some(&tree::shard_hash(bytes))
            })
        } else {
            None
        };
        good += usize::from(shard.is_some());
        held.push(shard);
    }
    held
}

/// Checks every shard of a segment. Keeps the first `k` good ones for
/// rebuilding (more are never needed) and lists the indices that are missing
/// or fail their hash.
fn verify_all(
    manifest: &Manifest,
    segment: u32,
    source: &impl crate::ShardSource,
    k: usize,
    shard_len: usize,
) -> (Vec<Option<Vec<u8>>>, Vec<u16>) {
    let n = manifest.header().n();
    let mut held = Vec::with_capacity(usize::from(n));
    let mut missing = Vec::new();
    let mut kept = 0;
    for index in 0..n {
        let good = source
            .get(segment, index, shard_len)
            .filter(|bytes| manifest.shard_hash(segment, index) == Some(&tree::shard_hash(bytes)));
        match good {
            None => {
                missing.push(index);
                held.push(None);
            }
            Some(bytes) if kept < k => {
                kept += 1;
                held.push(Some(bytes));
            }
            Some(_) => held.push(None),
        }
    }
    (held, missing)
}

/// Reads until at least one byte arrives or the input ends, retrying on
/// interruption.
fn read_some(input: &mut impl Read, buf: &mut [u8]) -> io::Result<usize> {
    loop {
        match input.read(buf) {
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            other => return other,
        }
    }
}

fn segment_len(len: Option<u64>) -> Result<usize, Error> {
    len.and_then(|l| usize::try_from(l).ok())
        .ok_or(Error::Manifest)
}

pub(crate) fn shard_len(header: &Header, segment: u32) -> Result<usize, Error> {
    segment_len(header.segment_shard_len(segment).map(u64::from))
}

fn unrecoverable(e: ErasureError, segment: u32, held: &[Option<Vec<u8>>], k: usize) -> Error {
    match e {
        ErasureError::NotEnough { .. } => Error::Unrecoverable {
            segment,
            have: held.iter().flatten().count(),
            need: k,
        },
        other => Error::Erasure(other),
    }
}
