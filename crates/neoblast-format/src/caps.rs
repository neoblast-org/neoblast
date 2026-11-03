//! Capability links (decision 0004).
//!
//! Payloads are lowercase, unpadded base32 (RFC 4648 alphabet) so links survive
//! case-folding. Every payload before `#` starts with a one-byte version.
//! Secrets only ever appear after `#`, which browsers never send to servers.
//!
//! * File verify cap: `nbl://fv/<base32(0x01 || root)>`
//! * File read cap:   `nbl://f/<base32(0x01 || root)>#<base32(read_key)>`
//!
//! Channel caps (`nbl://c/`, `nbl://cv/`, `nbl://cw/`) arrive in Phase 2.

use data_encoding::BASE32_NOPAD;
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

/// Version byte inside every v1 cap payload.
pub const CAP_VERSION: u8 = 0x01;
const FILE_READ_PREFIX: &str = "nbl://f/";
const FILE_VERIFY_PREFIX: &str = "nbl://fv/";

/// Reasons a link is rejected.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CapError {
    /// The link does not start with a known `nbl://` prefix.
    #[error("not a neoblast file link")]
    Prefix,
    /// The base32 payload did not decode.
    #[error("bad base32 encoding")]
    Encoding,
    /// The decoded payload has the wrong length or version.
    #[error("wrong payload length or unsupported version")]
    Payload,
    /// A read link is missing its `#key` fragment.
    #[error("read link has no key fragment")]
    MissingKey,
}

/// Lets a node store, verify and heal a file's shards. Reveals nothing about content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileVerifyCap {
    /// The file root `H`.
    pub root: [u8; 32],
}

/// Lets the holder decrypt a file. Share this link to publish.
#[derive(Clone, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct FileReadCap {
    /// The file root `H`.
    pub root: [u8; 32],
    /// The per-file random read key `R`.
    pub read_key: [u8; 32],
}

impl core::fmt::Debug for FileReadCap {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FileReadCap")
            .field("root", &encode(&self.root))
            .field("read_key", &"<redacted>")
            .finish()
    }
}

impl FileReadCap {
    /// Derives the weaker verify cap.
    pub fn verify_cap(&self) -> FileVerifyCap {
        FileVerifyCap { root: self.root }
    }

    /// Formats the shareable link.
    pub fn to_link(&self) -> String {
        format!(
            "{FILE_READ_PREFIX}{}#{}",
            encode_versioned(&self.root),
            encode(&self.read_key)
        )
    }

    /// Parses a read link. Never panics on any input.
    pub fn parse(link: &str) -> Result<Self, CapError> {
        let rest = link
            .strip_prefix(FILE_READ_PREFIX)
            .ok_or(CapError::Prefix)?;
        let (root_part, key_part) = rest.split_once('#').ok_or(CapError::MissingKey)?;
        // Peer hints such as `&p=...` may follow the key in later versions.
        let key_part = key_part.split('&').next().unwrap_or_default();
        let root = decode_versioned(root_part)?;
        let read_key = decode_32(key_part)?;
        Ok(Self { root, read_key })
    }
}

impl FileVerifyCap {
    /// Formats the verify link.
    pub fn to_link(&self) -> String {
        format!("{FILE_VERIFY_PREFIX}{}", encode_versioned(&self.root))
    }

    /// Parses a verify link, or extracts the verify cap from a read link.
    pub fn parse(link: &str) -> Result<Self, CapError> {
        if let Some(rest) = link.strip_prefix(FILE_VERIFY_PREFIX) {
            return Ok(Self {
                root: decode_versioned(rest)?,
            });
        }
        Ok(FileReadCap::parse(link)?.verify_cap())
    }
}

fn encode(bytes: &[u8]) -> String {
    BASE32_NOPAD.encode(bytes).to_ascii_lowercase()
}

fn encode_versioned(root: &[u8; 32]) -> String {
    let mut payload = [0u8; 33];
    payload[0] = CAP_VERSION;
    payload[1..].copy_from_slice(root);
    encode(&payload)
}

/// Decodes into buffers that are wiped when dropped, as the key fragment
/// passes through here (review S-003). The output is sized by `decode_len`,
/// as `decode_mut` requires, and is filled in place, so it is never
/// reallocated, which would leave a copy behind.
fn decode(text: &str) -> Result<Zeroizing<Vec<u8>>, CapError> {
    let upper = Zeroizing::new(text.to_ascii_uppercase());
    let len = BASE32_NOPAD
        .decode_len(upper.len())
        .map_err(|_| CapError::Encoding)?;
    let mut bytes = Zeroizing::new(vec![0u8; len]);
    let written = BASE32_NOPAD
        .decode_mut(upper.as_bytes(), &mut bytes)
        .map_err(|_| CapError::Encoding)?;
    bytes.truncate(written);
    Ok(bytes)
}

fn decode_versioned(text: &str) -> Result<[u8; 32], CapError> {
    let bytes = decode(text)?;
    match bytes.split_first() {
        Some((&CAP_VERSION, rest)) => rest.try_into().map_err(|_| CapError::Payload),
        _ => Err(CapError::Payload),
    }
}

/// The 32 bytes are returned by value, for [`FileReadCap`] to hold and wipe
/// on drop; a copy that the compiler leaves on the stack on the way is not
/// wiped (review W-055).
fn decode_32(text: &str) -> Result<[u8; 32], CapError> {
    decode(text)?
        .as_slice()
        .try_into()
        .map_err(|_| CapError::Payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn read_links_round_trip(root in any::<[u8; 32]>(), key in any::<[u8; 32]>()) {
            let cap = FileReadCap { root, read_key: key };
            let link = cap.to_link();
            prop_assert!(link.starts_with("nbl://f/"));
            prop_assert_eq!(link.to_ascii_lowercase(), link.clone());
            prop_assert_eq!(FileReadCap::parse(&link).unwrap(), cap.clone());
            prop_assert_eq!(FileVerifyCap::parse(&link).unwrap().root, root);
        }

        #[test]
        fn parse_never_panics(s in ".{0,200}") {
            let _ = FileReadCap::parse(&s);
            let _ = FileVerifyCap::parse(&s);
        }
    }

    #[test]
    fn verify_link_carries_no_key() {
        let cap = FileReadCap {
            root: [7; 32],
            read_key: [9; 32],
        };
        let v = cap.verify_cap().to_link();
        assert!(!v.contains('#'));
        assert_eq!(FileVerifyCap::parse(&v).unwrap().root, [7; 32]);
        assert!(FileReadCap::parse(&v).is_err());
    }

    #[test]
    fn debug_redacts_key() {
        let cap = FileReadCap {
            root: [1; 32],
            read_key: [2; 32],
        };
        assert!(format!("{cap:?}").contains("redacted"));
    }

    /// S-003: decoding into wiped buffers keeps every result. A key that is
    /// not base32, of a length base32 cannot have, or that decodes to other
    /// than 32 bytes is refused, as before; upper case still parses.
    #[test]
    fn s003_key_fragment_errors_are_unchanged() {
        let cap = FileReadCap {
            root: [3; 32],
            read_key: [4; 32],
        };
        let link = cap.to_link();
        let (head, key) = link.split_once('#').unwrap();
        let parse = |key: &str| FileReadCap::parse(&format!("{head}#{key}"));
        assert_eq!(parse(key), Ok(cap.clone()));
        assert_eq!(parse(&key.to_ascii_uppercase()), Ok(cap.clone()));
        // `1` is not in the alphabet, and 54 characters is a length base32
        // never has (the key's 52, plus 2); 31, 33 and 0 bytes decode.
        assert_eq!(parse(&format!("1{}", &key[1..])), Err(CapError::Encoding));
        assert_eq!(parse(&format!("{key}aa")), Err(CapError::Encoding));
        assert_eq!(parse(&encode(&[4; 31])), Err(CapError::Payload));
        assert_eq!(parse(&encode(&[4; 33])), Err(CapError::Payload));
        assert_eq!(parse(""), Err(CapError::Payload));
    }

    #[test]
    fn rejects_wrong_version() {
        let mut payload = vec![0x02u8];
        payload.extend_from_slice(&[0u8; 32]);
        let link = format!("nbl://fv/{}", encode(&payload));
        assert_eq!(FileVerifyCap::parse(&link), Err(CapError::Payload));
    }
}
