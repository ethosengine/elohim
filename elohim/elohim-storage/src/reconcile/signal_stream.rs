//! DNA signal stream — storage-internal canonical signal types and subscription trait.
//!
//! ## Design classification
//!
//! The signal stream is **Category C — operational**. Each `DnaSignal` is a
//! transient projection of an underlying DHT-notarized entry. The stream itself
//! is never persisted; it is fully reconstructable by re-querying the imagodei
//! DNA at any time. Source-of-truth for each variant is the DHT entry referenced
//! by the `action_hash` field.
//!
//! ## Translation boundary
//!
//! The real implementation (`HolochainAppSignalStream`, Task A.11) translates
//! from the DNA-side `RecoveryV2Signal` enum into these storage-internal types:
//!
//! | DNA-side (`RecoveryV2Signal`)    | Storage-internal (`DnaSignal`)   |
//! |----------------------------------|----------------------------------|
//! | `KeyRotationCommitted`           | `DnaSignal::KeyRotation`         |
//! | `KeyRevocationEffective`         | `DnaSignal::KeyRevocation`       |
//! | `KeyRevocationRequested`         | `DnaSignal::RevocationAttestation` (kind = request) |
//! | `RevocationVoteSubmitted`        | `DnaSignal::RevocationAttestation` (kind = vote)    |
//! | *(future create_agent_peer_binding)* | `DnaSignal::AgentPeerBinding` |
//!
//! The storage-internal shape is deliberately decoupled from the DNA-side enum
//! to keep the controller's interface stable across DNA evolution.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::mpsc;

// ---------------------------------------------------------------------------
// Device archetype
// ---------------------------------------------------------------------------

/// Storage-side device archetype enum — mirrors `imagodei_integrity::DeviceArchetype`.
///
/// Duplicated here (rather than re-exported from the imagodei crate) because
/// elohim-storage does not depend on the Holochain DNA crates at compile time.
/// The `imagodei_integrity` crate targets `wasm32-unknown-unknown` and carries
/// HDI dependencies that conflict with native builds. Values are identical;
/// keep in sync with `elohim/sdk/schemas/v1/enums/device-archetype.schema.json`
/// and `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeviceArchetype {
    /// Dedicated elohim-node server or blade.
    Node,
    /// Tauri-wrapped desktop client (workstation or laptop).
    Desktop,
    /// Mobile client (phone or tablet).
    Mobile,
    /// Steward process managing collective infrastructure.
    Steward,
}

// ---------------------------------------------------------------------------
// Per-variant payload structs
// ---------------------------------------------------------------------------

/// Payload for `DnaSignal::KeyRotation`.
///
/// Projection of the imagodei `KeyRotation` DHT entry becoming committed.
/// Source of truth: DHT (Category A notarized entry referenced by `action_hash`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyRotationSignal {
    /// Holochain ActionHash (base64) of the `KeyRotation` DHT entry.
    pub action_hash: String,
    /// CID of the agent whose key rotated.
    pub agent_cid: String,
    /// The incoming ed25519 pubkey (base64, 32 bytes).
    pub new_pubkey: String,
    /// The rotated-out ed25519 pubkey (base64, 32 bytes).
    pub old_pubkey: String,
    /// Timestamp at which the DHT entry records the rotation.
    pub rotated_at: DateTime<Utc>,
    /// Timestamp at which the signal was emitted from the conductor.
    pub emitted_at: DateTime<Utc>,
}

/// Payload for `DnaSignal::KeyRevocation`.
///
/// Projection of the imagodei `KeyRevocation` DHT entry becoming effective.
/// Source of truth: DHT (Category A notarized entry referenced by `action_hash`).
/// Corresponds to Recovery M4 `RecoveryV2Signal::KeyRevocationEffective`.
///
/// The `compromise_at` field is used by the controller sweep (Task A.8) to
/// retroactively invalidate EPR attestations: any EPR referencing `revoked_pubkey`
/// that was issued after `compromise_at` is considered tainted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyRevocationSignal {
    /// Holochain ActionHash (base64) of the `KeyRevocation` DHT entry.
    pub action_hash: String,
    /// CID of the agent whose key was revoked.
    pub agent_cid: String,
    /// The revoked ed25519 pubkey (base64, 32 bytes).
    pub revoked_pubkey: String,
    /// Point in time the key is declared to have first been compromised.
    /// EPR attestations referencing `revoked_pubkey` issued after this timestamp
    /// are tainted and must be re-verified or invalidated.
    pub compromise_at: DateTime<Utc>,
    /// Point at which the revocation became effective in the DHT (quorum reached).
    pub effective_at: DateTime<Utc>,
    /// ID linking back to the `KeyRevocation` request chain. `None` for
    /// administrative revocations with no vote chain.
    pub triggering_revocation_id: Option<String>,
    /// Timestamp at which the signal was emitted from the conductor.
    pub emitted_at: DateTime<Utc>,
}

/// Payload for `DnaSignal::AgentPeerBinding`.
///
/// Projection of the imagodei `AgentPeerBinding` DHT entry being created.
/// Source of truth: DHT (Category A notarized entry referenced by `action_hash`).
/// Emitted by the imagodei coordinator `create_agent_peer_binding` (Task A.9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPeerBindingSignal {
    /// Holochain ActionHash (base64) of the `AgentPeerBinding` DHT entry.
    /// Same value as `binding_action_hash`; present for uniform stream deserialization.
    pub action_hash: String,
    /// multibase-encoded libp2p PeerId of the bound peer.
    pub peer_id: String,
    /// CID of the agent to which this peer is bound.
    pub agent_cid: String,
    /// Timestamp from which this binding is valid.
    pub valid_from: DateTime<Utc>,
    /// Timestamp at which this binding expires. `None` means no declared expiry.
    pub valid_until: Option<DateTime<Utc>>,
    /// Hardware/deployment archetype of the bound device.
    pub device_archetype: DeviceArchetype,
    /// Semantic alias for `action_hash` — the DHT ActionHash of the `AgentPeerBinding`
    /// entry. Consumers tracking binding provenance specifically should use this field.
    pub binding_action_hash: String,
    /// Timestamp at which the signal was emitted from the conductor.
    pub emitted_at: DateTime<Utc>,
}

/// Discriminates the two kinds of revocation attestation.
///
/// Used by `RevocationAttestationSignal::attestation_kind` to distinguish the
/// initial revocation trigger from subsequent steward votes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AttestationKind {
    /// Initial revocation trigger — maps from M4 `KeyRevocationRequested`.
    Request,
    /// Subsequent steward vote — maps from M4 `RevocationVoteSubmitted`.
    Vote,
}

/// Payload for `DnaSignal::RevocationAttestation`.
///
/// Unified projection of revocation vote-collection state from the imagodei DNA.
/// Source of truth: DHT (Category A notarized entries referenced by `action_hash`
/// and `revocation_id`).
///
/// Unifies two Recovery M4 signals:
/// - `RecoveryV2Signal::KeyRevocationRequested` → `attestation_kind = Request`
/// - `RecoveryV2Signal::RevocationVoteSubmitted` → `attestation_kind = Vote`
///
/// The controller uses this to maintain in-progress revocation vote state in
/// the `key_revocations` projection table until quorum is reached and a
/// `DnaSignal::KeyRevocation` fires.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevocationAttestationSignal {
    /// Holochain ActionHash (base64) of the DHT entry that originated this signal.
    /// For `Request` kind: the `KeyRevocation` entry. For `Vote` kind: the
    /// `RevocationVote` entry.
    pub action_hash: String,
    /// Stable identifier linking this attestation to the overall revocation request.
    pub revocation_id: String,
    /// Agent CID or pubkey of the steward submitting the attestation.
    pub steward_id: String,
    /// Whether this attestation approves the revocation. Always `true` for
    /// `Request` kind (the request itself is an affirmative act).
    pub approved: bool,
    /// Whether this is the initial request or a subsequent vote.
    pub attestation_kind: AttestationKind,
    /// Current count of affirmative votes at the time of this attestation.
    pub current_votes: u32,
    /// Vote threshold required for the revocation to become effective.
    pub required_votes: u32,
    /// Whether the vote threshold has been reached as of this attestation.
    /// When `true`, a `DnaSignal::KeyRevocation` should follow shortly.
    pub threshold_reached: bool,
    /// Timestamp at which the attestation was recorded in the DNA.
    pub attested_at: DateTime<Utc>,
    /// Timestamp at which the signal was emitted from the conductor.
    pub emitted_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Top-level DnaSignal enum
// ---------------------------------------------------------------------------

/// Storage-internal canonical signal shape for DHT-notarized identity events.
///
/// Category C — operational. Source of truth for each variant is the DHT entry
/// referenced by the `action_hash` field inside each payload struct.
///
/// This enum is the stable interface the reconcile controller (Task A.4) consumes.
/// The translator that maps DNA-side `RecoveryV2Signal` variants to `DnaSignal`
/// variants lives in `HolochainAppSignalStream` (Task A.11) and is intentionally
/// separate from this definition.
///
/// Serializes with a `"signalType"` discriminator tag in camelCase, matching the
/// `dna-signal-stream.schema.json` wire contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "signalType", rename_all = "camelCase")]
pub enum DnaSignal {
    /// A human's agent key has rotated successfully.
    KeyRotation(KeyRotationSignal),
    /// A key revocation has reached quorum and is now effective.
    KeyRevocation(KeyRevocationSignal),
    /// An agent–peer libp2p binding has been notarized on the DHT.
    AgentPeerBinding(AgentPeerBindingSignal),
    /// A steward has submitted a revocation request or cast a vote.
    RevocationAttestation(RevocationAttestationSignal),
}

// ---------------------------------------------------------------------------
// Cursor and error types
// ---------------------------------------------------------------------------

/// Opaque resume token for `DnaSignalStream::resume_from`.
///
/// Currently wraps a `String` (e.g. a sequence index or action hash). Will
/// become a typed union when `HolochainAppSignalStream` (Task A.11) defines
/// its real cursor semantics (likely an action hash or conductor signal ID).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalCursor(pub String);

impl SignalCursor {
    /// Construct a cursor from an opaque string (e.g. a sequence index).
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

/// Errors that can occur in stream cursor operations.
#[derive(Debug, Error)]
pub enum SignalStreamError {
    #[error("cursor position {0:?} is out of range for this stream")]
    CursorOutOfRange(SignalCursor),
    #[error("stream is closed")]
    Closed,
    #[error("stream error: {0}")]
    Other(String),
}

// ---------------------------------------------------------------------------
// DnaSignalStream trait
// ---------------------------------------------------------------------------

/// Async subscription interface for the DNA signal stream.
///
/// The reconcile controller (Task A.4) depends on this trait — not on any
/// concrete implementation. The stub implementations below serve controller
/// unit tests. The real implementation (`HolochainAppSignalStream`) lands in
/// Task A.11 and translates from the conductor's WebSocket signal channel.
///
/// ## Contract
///
/// - `next_signal` drives the controller loop. Returns `None` when the stream
///   is exhausted (for stubs) or when the conductor disconnects (real impl).
/// - `cursor` returns the current resume position. The cursor is meaningful
///   only while the stream is live; after reconnection `resume_from` is used.
/// - `resume_from` repositions the stream at the given cursor. The real impl
///   will query the conductor for missed signals since the cursor position.
#[async_trait]
pub trait DnaSignalStream: Send + Sync + 'static {
    /// Return the next available signal, or `None` if the stream is exhausted.
    async fn next_signal(&mut self) -> Option<DnaSignal>;

    /// Return the current resume cursor, or `None` if no signal has been
    /// delivered yet.
    async fn cursor(&self) -> Option<SignalCursor>;

    /// Reposition the stream to resume from `cursor`.
    ///
    /// For stub implementations this seeks within the pre-loaded signal vec.
    /// For `HolochainAppSignalStream` (Task A.11) this will replay missed
    /// signals from the conductor since the given position.
    async fn resume_from(&mut self, cursor: SignalCursor) -> Result<(), SignalStreamError>;
}

// ---------------------------------------------------------------------------
// InMemoryDnaSignalStream — pre-loaded stub for unit tests
// ---------------------------------------------------------------------------

/// Pre-loaded in-memory stub that delivers signals from a `Vec` in order.
///
/// Used by controller unit tests (Task A.4) to exercise the reconcile loop
/// without a live conductor. The cursor is the current index into the signal
/// vec (as a decimal string).
pub struct InMemoryDnaSignalStream {
    signals: Vec<DnaSignal>,
    index: usize,
}

impl InMemoryDnaSignalStream {
    /// Construct a stream pre-loaded with `signals`. Signals are delivered in
    /// iteration order; `next_signal` returns `None` after the last element.
    pub fn with_signals(signals: Vec<DnaSignal>) -> Self {
        Self { signals, index: 0 }
    }
}

#[async_trait]
impl DnaSignalStream for InMemoryDnaSignalStream {
    async fn next_signal(&mut self) -> Option<DnaSignal> {
        if self.index < self.signals.len() {
            let signal = self.signals[self.index].clone();
            self.index += 1;
            Some(signal)
        } else {
            None
        }
    }

    async fn cursor(&self) -> Option<SignalCursor> {
        if self.index == 0 {
            None
        } else {
            Some(SignalCursor::new(self.index.to_string()))
        }
    }

    async fn resume_from(&mut self, cursor: SignalCursor) -> Result<(), SignalStreamError> {
        let pos: usize = cursor
            .0
            .parse()
            .map_err(|_| SignalStreamError::CursorOutOfRange(cursor.clone()))?;
        if pos > self.signals.len() {
            return Err(SignalStreamError::CursorOutOfRange(cursor));
        }
        self.index = pos;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// ChannelSignalStream — dynamic stub for controller tests (push-based)
// ---------------------------------------------------------------------------

/// Dynamic stub wrapping a `tokio::sync::mpsc::Receiver<DnaSignal>`.
///
/// Allows controller tests to push signals dynamically via the paired sender,
/// simulating a live conductor signal channel without requiring a real
/// WebSocket connection. Tests hold the `tokio::sync::mpsc::Sender<DnaSignal>`
/// and drop it to signal end-of-stream.
///
/// Cursor semantics: a monotonic sequence counter that increments on each
/// delivered signal. `resume_from` is not meaningful for channel streams
/// (missed signals cannot be replayed); it returns an error.
pub struct ChannelSignalStream {
    receiver: mpsc::Receiver<DnaSignal>,
    delivered: u64,
}

impl ChannelSignalStream {
    /// Construct a new channel stream from the receiver half of an mpsc channel.
    ///
    /// Callers typically create the channel with `tokio::sync::mpsc::channel(N)`
    /// and pass the `Receiver` here, retaining the `Sender` to push test signals.
    pub fn new(receiver: mpsc::Receiver<DnaSignal>) -> Self {
        Self {
            receiver,
            delivered: 0,
        }
    }
}

#[async_trait]
impl DnaSignalStream for ChannelSignalStream {
    async fn next_signal(&mut self) -> Option<DnaSignal> {
        match self.receiver.recv().await {
            Some(signal) => {
                self.delivered += 1;
                Some(signal)
            }
            None => None,
        }
    }

    async fn cursor(&self) -> Option<SignalCursor> {
        if self.delivered == 0 {
            None
        } else {
            Some(SignalCursor::new(self.delivered.to_string()))
        }
    }

    async fn resume_from(&mut self, cursor: SignalCursor) -> Result<(), SignalStreamError> {
        // Channel streams cannot replay missed signals; the conductor is the
        // source of truth and replaying requires a real implementation.
        Err(SignalStreamError::Other(format!(
            "ChannelSignalStream does not support resume_from (cursor = {cursor:?}); \
             use HolochainAppSignalStream (Task A.11) for cursor-based resumption"
        )))
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tokio::sync::mpsc;

    fn sample_key_rotation() -> DnaSignal {
        DnaSignal::KeyRotation(KeyRotationSignal {
            action_hash: "uhCkk-rotation-action-hash".to_string(),
            agent_cid: "bafybeicid-agent-1".to_string(),
            new_pubkey: "bmV3a2V5YmFzZTY0AAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
            old_pubkey: "b2xka2V5YmFzZTY0AAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
            rotated_at: Utc::now(),
            emitted_at: Utc::now(),
        })
    }

    fn sample_key_revocation() -> DnaSignal {
        DnaSignal::KeyRevocation(KeyRevocationSignal {
            action_hash: "uhCkk-revocation-action-hash".to_string(),
            agent_cid: "bafybeicid-agent-2".to_string(),
            revoked_pubkey: "cmV2b2tlZGtleWJhc2U2NAAAAAAAAAAAAAAAAAAAAAA".to_string(),
            compromise_at: Utc::now(),
            effective_at: Utc::now(),
            triggering_revocation_id: Some("rev-001".to_string()),
            emitted_at: Utc::now(),
        })
    }

    fn sample_agent_peer_binding() -> DnaSignal {
        DnaSignal::AgentPeerBinding(AgentPeerBindingSignal {
            action_hash: "uhCkk-binding-action-hash".to_string(),
            peer_id: "12D3KooWExamplePeerId".to_string(),
            agent_cid: "bafybeicid-agent-3".to_string(),
            valid_from: Utc::now(),
            valid_until: None,
            device_archetype: DeviceArchetype::Node,
            binding_action_hash: "uhCkk-binding-action-hash".to_string(),
            emitted_at: Utc::now(),
        })
    }

    fn sample_revocation_attestation() -> DnaSignal {
        DnaSignal::RevocationAttestation(RevocationAttestationSignal {
            action_hash: "uhCkk-vote-action-hash".to_string(),
            revocation_id: "rev-001".to_string(),
            steward_id: "bafybeicid-steward-1".to_string(),
            approved: true,
            attestation_kind: AttestationKind::Vote,
            current_votes: 2,
            required_votes: 3,
            threshold_reached: false,
            attested_at: Utc::now(),
            emitted_at: Utc::now(),
        })
    }

    #[tokio::test]
    async fn stub_signal_stream_delivers_in_order() {
        let signals = vec![sample_key_rotation(), sample_key_revocation()];
        let mut stream = InMemoryDnaSignalStream::with_signals(signals);

        let first = stream.next_signal().await;
        assert!(
            matches!(first, Some(DnaSignal::KeyRotation(_))),
            "expected KeyRotation first"
        );

        let second = stream.next_signal().await;
        assert!(
            matches!(second, Some(DnaSignal::KeyRevocation(_))),
            "expected KeyRevocation second"
        );

        let done = stream.next_signal().await;
        assert!(done.is_none(), "expected None after exhaustion");
    }

    #[tokio::test]
    async fn stub_signal_stream_cursor_tracks_position() {
        let mut stream = InMemoryDnaSignalStream::with_signals(vec![sample_key_rotation()]);

        assert_eq!(
            stream.cursor().await,
            None,
            "cursor is None before any signal delivered"
        );

        let _ = stream.next_signal().await;

        assert_eq!(
            stream.cursor().await,
            Some(SignalCursor::new("1")),
            "cursor is '1' after one signal delivered"
        );
    }

    #[tokio::test]
    async fn stub_signal_stream_resume_from_seeks_correctly() {
        let signals = vec![
            sample_key_rotation(),
            sample_key_revocation(),
            sample_agent_peer_binding(),
        ];
        let mut stream = InMemoryDnaSignalStream::with_signals(signals);

        // Consume first two
        let _ = stream.next_signal().await;
        let _ = stream.next_signal().await;

        // Resume from index 1 (re-deliver second signal)
        stream
            .resume_from(SignalCursor::new("1"))
            .await
            .expect("resume_from should succeed");

        let replayed = stream.next_signal().await;
        assert!(
            matches!(replayed, Some(DnaSignal::KeyRevocation(_))),
            "resume_from(1) should re-deliver the KeyRevocation signal"
        );
    }

    #[tokio::test]
    async fn channel_signal_stream_delivers_pushed() {
        let (tx, rx) = mpsc::channel(8);
        let mut stream = ChannelSignalStream::new(rx);

        tx.send(sample_agent_peer_binding()).await.unwrap();
        tx.send(sample_revocation_attestation()).await.unwrap();
        drop(tx); // signal end-of-stream

        let first = stream.next_signal().await;
        assert!(
            matches!(first, Some(DnaSignal::AgentPeerBinding(_))),
            "expected AgentPeerBinding first"
        );

        let second = stream.next_signal().await;
        assert!(
            matches!(second, Some(DnaSignal::RevocationAttestation(_))),
            "expected RevocationAttestation second"
        );

        let done = stream.next_signal().await;
        assert!(done.is_none(), "expected None after channel closed");
    }

    #[tokio::test]
    async fn channel_signal_stream_cursor_increments() {
        let (tx, rx) = mpsc::channel(8);
        let mut stream = ChannelSignalStream::new(rx);

        assert_eq!(stream.cursor().await, None);

        tx.send(sample_key_rotation()).await.unwrap();
        let _ = stream.next_signal().await;
        assert_eq!(stream.cursor().await, Some(SignalCursor::new("1")));

        tx.send(sample_key_revocation()).await.unwrap();
        let _ = stream.next_signal().await;
        assert_eq!(stream.cursor().await, Some(SignalCursor::new("2")));
    }

    #[tokio::test]
    async fn channel_signal_stream_resume_from_errors() {
        let (_tx, rx) = mpsc::channel(1);
        let mut stream = ChannelSignalStream::new(rx);

        let result = stream.resume_from(SignalCursor::new("0")).await;
        assert!(
            result.is_err(),
            "ChannelSignalStream.resume_from should always error"
        );
    }

    #[test]
    fn dna_signal_round_trips_through_json() {
        let samples: Vec<DnaSignal> = vec![
            sample_key_rotation(),
            sample_key_revocation(),
            sample_agent_peer_binding(),
            sample_revocation_attestation(),
        ];

        for signal in &samples {
            let json = serde_json::to_string(signal).expect("serialization should succeed");
            let parsed: DnaSignal =
                serde_json::from_str(&json).expect("deserialization should succeed");
            assert_eq!(signal, &parsed, "round-trip failed for signal: {json}");
        }
    }

    #[test]
    fn dna_signal_json_uses_camel_case_tag() {
        let signal = sample_key_rotation();
        let json = serde_json::to_value(&signal).expect("serialization should succeed");
        assert_eq!(
            json["signalType"].as_str(),
            Some("keyRotation"),
            "DnaSignal tag should serialize as camelCase 'keyRotation'"
        );

        let revocation = sample_key_revocation();
        let json2 = serde_json::to_value(&revocation).expect("serialization should succeed");
        assert_eq!(
            json2["signalType"].as_str(),
            Some("keyRevocation"),
            "DnaSignal tag should serialize as camelCase 'keyRevocation'"
        );
    }

    #[test]
    fn key_revocation_signal_serializes_nullable_triggering_id() {
        let with_id = DnaSignal::KeyRevocation(KeyRevocationSignal {
            triggering_revocation_id: Some("rev-001".to_string()),
            ..match sample_key_revocation() {
                DnaSignal::KeyRevocation(s) => s,
                _ => unreachable!(),
            }
        });
        let without_id = DnaSignal::KeyRevocation(KeyRevocationSignal {
            triggering_revocation_id: None,
            ..match sample_key_revocation() {
                DnaSignal::KeyRevocation(s) => s,
                _ => unreachable!(),
            }
        });

        let json_with = serde_json::to_value(&with_id).unwrap();
        assert_eq!(
            json_with["triggeringRevocationId"].as_str(),
            Some("rev-001")
        );

        let json_without = serde_json::to_value(&without_id).unwrap();
        assert!(
            json_without["triggeringRevocationId"].is_null(),
            "None should serialize as null"
        );
    }
}
