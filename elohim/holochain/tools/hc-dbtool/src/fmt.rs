//! Rendering raw database columns into the forms an operator reads elsewhere.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

/// The holo_hash base64 form: `u` + base64url-nopad of the raw 39 bytes.
///
/// This is the string the conductor logs, the DNA manifests and `hc` all use
/// (`uhC0k…` for a DnaHash, `uhCAk…` for an AgentPubKey), so a row can be
/// grepped straight back into a log.
pub fn hash_b64(bytes: &[u8]) -> String {
    format!("u{}", URL_SAFE_NO_PAD.encode(bytes))
}

/// Which kind of hash a bare database column holds.
///
/// The DHT tables store the 36-byte core (32-byte digest + 4-byte DHT location)
/// WITHOUT the 3-byte type prefix — the prefix is implied by the column. A
/// `BlockSpan` payload, by contrast, decodes to a typed `HoloHash` and already
/// carries all 39 bytes. Rendering a 36-byte column with [`hash_b64`] alone
/// produces a string that does NOT match the same hash anywhere else
/// (`u-5aEHTDZ…` instead of `uhCQk-5aEHTDZ…`), which silently breaks the join
/// between a block and the warrant op it cites. Hence this typed form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashKind {
    Agent,
    DhtOp,
    Action,
    Dna,
    Entry,
}

impl HashKind {
    /// The 3-byte multihash prefix, i.e. the bytes behind `uhCAk` / `uhCQk` /
    /// `uhCkk` / `uhC0k` / `uhCEk`.
    pub const fn prefix(self) -> [u8; 3] {
        match self {
            HashKind::Agent => [0x84, 0x20, 0x24],
            HashKind::DhtOp => [0x84, 0x24, 0x24],
            HashKind::Action => [0x84, 0x29, 0x24],
            HashKind::Dna => [0x84, 0x2d, 0x24],
            HashKind::Entry => [0x84, 0x21, 0x24],
        }
    }
}

/// Render a raw hash column in the same `uhC…` form the rest of the system uses.
///
/// A 36-byte core gets its column's type prefix; an already-39-byte value is
/// passed through untouched; anything else is rendered with its length called
/// out rather than silently mis-prefixed.
pub fn hash_b64_kind(bytes: &[u8], kind: HashKind) -> String {
    match bytes.len() {
        36 => {
            let mut full = kind.prefix().to_vec();
            full.extend_from_slice(bytes);
            hash_b64(&full)
        }
        39 => hash_b64(bytes),
        n => format!("{}<{n}B>", hash_b64(bytes)),
    }
}

/// Latest microsecond timestamp we will render as a date rather than as
/// "unbounded". Matches the upper bound of `holochain_timestamp`'s own Display
/// range (`chrono_ext.rs`), so we never hand it a value it renders as raw µs.
pub const MAX_RENDERABLE_US: i64 = 253_402_214_400_000_000;

/// Render a microsecond timestamp.
///
/// `Timestamp::max()` is what `integrate_dht_ops_workflow` writes as a block's
/// `end_us`, and it is the whole point of the tool: the block never expires.
/// Say that in words rather than printing a year-292277 date.
pub fn timestamp_us(us: i64) -> String {
    if us >= MAX_RENDERABLE_US {
        return format!("never (Timestamp::max, {us})");
    }
    if us <= 0 {
        return format!("{us}");
    }
    match holochain_timestamp::Timestamp::from_micros(us).to_string() {
        s if s.is_empty() => format!("{us}"),
        s => s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dna_hash_renders_in_uhc0k_form() {
        // The node-registry DNA on the household mesh.
        let expected = "uhC0kNpWca3kckKu8YbNFbYT2Dva296lTxdRjcFS4_YLk6ZS12x1K";
        let raw = URL_SAFE_NO_PAD.decode(&expected[1..]).unwrap();
        assert_eq!(raw.len(), 39, "a holo hash is 39 raw bytes");
        assert_eq!(hash_b64(&raw), expected);
    }

    #[test]
    fn a_36_byte_column_gets_its_type_prefix() {
        // The 36-byte core of the agent james blocks, and the 39-byte form the
        // BlockSpan payload decodes to. They must render identically or the
        // block cannot be joined to the op that caused it.
        let full = "uhCAkcTbutPk5V2yHOm1KrQY9DEchSVZ-kWFhTvaw70m8UCeSjQwy";
        let raw39 = URL_SAFE_NO_PAD.decode(&full[1..]).unwrap();
        assert_eq!(raw39.len(), 39);
        assert_eq!(hash_b64_kind(&raw39, HashKind::Agent), full);
        assert_eq!(hash_b64_kind(&raw39[3..], HashKind::Agent), full);
    }

    #[test]
    fn a_wrong_length_column_is_flagged_not_mis_prefixed() {
        assert!(hash_b64_kind(&[1, 2, 3], HashKind::DhtOp).ends_with("<3B>"));
    }

    #[test]
    fn timestamp_max_reads_as_never() {
        assert!(timestamp_us(i64::MAX).starts_with("never (Timestamp::max"));
    }

    #[test]
    fn ordinary_timestamp_renders_as_a_date() {
        // 2026-09-05T00:00:00Z
        let rendered = timestamp_us(1_788_566_400_000_000);
        assert!(rendered.starts_with("2026-"), "got: {rendered}");
    }
}
