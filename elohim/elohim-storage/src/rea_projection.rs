//! REA Projection Signal Handler
//!
//! Receives post-commit signals from the Holochain conductor and projects
//! REA entries (Agreement, Commitment, EconomicEvent) into SQLite storage
//! with dht_anchor_hash for cryptographic verification.
//!
//! DHT is the truth. Storage is the index. This handler bridges them.
//!
//! ## Wire format
//!
//! The DNA's `ProjectionSignal` enum uses `#[serde(tag = "type", content =
//! "payload")]` (adjacent tagging). The payload variants carry the FULL
//! DHT entry — `Agreement`, `Commitment`, `EconomicEvent` — not a
//! pre-converted projection input. This module mirrors that wire shape via
//! the `*Entry` structs and does the projection-input conversion inside
//! `handle_rea_signal` (parsing `*_json` fields, downcasting f64→f32, etc.).
//!
//! ## Signal Flow
//!
//! 1. Coordinator zome commits an REA entry to the DHT
//! 2. Post-commit hook emits a `ProjectionSignal` with the action_hash + entry
//! 3. `HcClient::subscribe_rea_projection_signals` (main.rs) receives + decodes
//! 4. This handler converts the entry shape → CreateInput → upsert_with_anchor
//! 5. If the row already exists (optimistic pre-write), it sets dht_anchor_hash
//! 6. If new (DHT-first write), it inserts with anchor

use chrono::Utc;
use serde::Deserialize;
use tracing::{debug, info, warn};

use crate::db::agreements::{self, CreateAgreementInput};
use crate::db::content_diesel::{self, ContentProjectionPatch};
use crate::db::context::AppContext;
use crate::db::economic_events::{self, CreateEconomicEventInput};
use crate::db::rea_commitments::CreateReaCommitmentInput;
use crate::db::DbPool;
use crate::error::StorageError;
use crate::signals::HoloHashB64;

/// A graduation that needs a `CommitmentByState` link authored (the SQL cache is
/// already flipped; this carries the DHT-truth write to the subscriber that holds
/// the HcClient). Decouples the sync projection from the async link author.
///
/// The SQL `state` flip is the functional path; the link is the durability +
/// peer-observability upgrade. `signed_at` is the transition's signing time
/// (the graduating event's projection time — the storage path supplies it, never
/// `sys_time()` in-zome).
#[derive(Debug, Clone)]
pub struct PendingStateLink {
    pub commitment_cid: String,
    pub state: String,
    pub event_hash: String,
    pub signed_at: String,
}

/// Set once by the signal subscriber at startup; the projection path pushes
/// graduations onto it for the subscriber's async drain task (which holds the
/// HcClient and calls `conductor_writes::call_create_commitment_state_link`).
static STATE_LINK_TX: std::sync::OnceLock<tokio::sync::mpsc::UnboundedSender<PendingStateLink>> =
    std::sync::OnceLock::new();

/// Install the channel the subscriber drains. Idempotent: a second call is a
/// no-op (`OnceLock`). Called from main.rs after the HcClient is available.
pub fn install_state_link_sink(tx: tokio::sync::mpsc::UnboundedSender<PendingStateLink>) {
    let _ = STATE_LINK_TX.set(tx);
}

/// Record a pending state-link transition. No-op when no sink is installed
/// (e.g. unit tests / conductor-less mode) — the SQL cache flip already stands;
/// the link is the durable upgrade the subscriber path performs once wired.
/// Content rows this file writes WITHOUT an `EventBus` in reach (the REA
/// signal subscriber is composed before the services exist). The producer that
/// projects rows into their sync docs listens for `StorageEvent::ContentUpdated`;
/// a row touched here and never announced leaves its doc stale until the next
/// cold-start back-fill. Measured 2026-08-29 (household mesh): two consecutive
/// cold starts of the same peer re-projected 1270 then 137 docs — the anchor
/// writes of `Projecting Content from DHT` were the class. Bounded channel:
/// a full sink drops the touch (counted), the back-fill remains the floor.
static CONTENT_TOUCH_TX: std::sync::OnceLock<tokio::sync::mpsc::Sender<String>> =
    std::sync::OnceLock::new();

/// Install the sink once; returns `false` if one is already installed (the
/// caller must not spawn a second forwarder for a channel nobody feeds).
pub fn install_content_touch_sink(tx: tokio::sync::mpsc::Sender<String>) -> bool {
    CONTENT_TOUCH_TX.set(tx).is_ok()
}

/// Capacity of the touch channel: a seed-sized burst (~3.4k) fits twice over.
pub const CONTENT_TOUCH_CAPACITY: usize = 8192;

/// Announce a content-row write made without an `EventBus` in reach (see
/// [`install_content_touch_sink`]). Safe to call from any writer.
pub fn notify_content_touched(id: &str) {
    if let Some(tx) = CONTENT_TOUCH_TX.get() {
        match tx.try_send(id.to_string()) {
            Ok(()) => crate::metrics::inc_content_touch("sent"),
            Err(_) => crate::metrics::inc_content_touch("dropped"),
        }
    } else {
        crate::metrics::inc_content_touch("no_sink");
    }
}

fn record_pending_state_link(commitment_cid: &str, state: &str, event_hash: &str, signed_at: &str) {
    if let Some(tx) = STATE_LINK_TX.get() {
        let _ = tx.send(PendingStateLink {
            commitment_cid: commitment_cid.to_string(),
            state: state.to_string(),
            event_hash: event_hash.to_string(),
            signed_at: signed_at.to_string(),
        });
    }
}

// ============================================================================
// Signal Types — mirror DNA-side ProjectionSignal exactly
//
// The DNA's ProjectionSignal uses #[serde(tag = "type", content = "payload")]
// (adjacent tagging). Variants embed the FULL DHT entry — Agreement,
// Commitment, EconomicEvent. The *Entry structs below must match the
// integrity-zome entry shapes field-for-field (see
// elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs).
//
// Any drift here is silently fatal: serde_json::from_value returns Err,
// the subscriber logs at debug, and the signal is dropped. Symptom is
// "REA commitment X written via conductor but projection did not land" —
// the in-process bounded poll times out at 1s.
// ============================================================================

// Wire-shape note (2026-06-13, the REA arm of the conductor-signal
// msgpack decode-class bug): the DNA `ProjectionSignal` variants declare
// `action_hash: ActionHash`, `entry_hash: EntryHash`, `author: AgentPubKey`
// (verified field-by-field against content_store/src/lib.rs ~10390-10552).
// On the conductor app-signal wire (MessagePack `ExternIO`) those holo_hash
// types serialize as raw 39-BYTE ARRAYS, NOT base64 strings — so the mirror
// MUST type them as `HoloHashB64` (accepts bytes/byte-seq/string, normalizes
// to canonical "u"+base64url), not `String`/`Option<String>`. The old
// `String` mirror + the `rmp → serde_json::Value` pre-pass dropped every real
// signal at decode (serde_json::Value has no byte-array representation): the
// exact dark class fixed for `InfrastructureSignal` in d33b0e1f5. The embedded
// `agreement`/`commitment`/`event`/`content` sub-structs are all-`String` on
// the DNA side (no holo_hash-typed fields), so they need NO HoloHashB64
// treatment and survive the wire intact.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ReaProjectionSignal {
    AgreementCommitted {
        action_hash: HoloHashB64,
        #[serde(default)]
        entry_hash: Option<HoloHashB64>,
        agreement: AgreementEntry,
        #[serde(default)]
        author: Option<HoloHashB64>,
    },
    ReaCommitmentCommitted {
        action_hash: HoloHashB64,
        #[serde(default)]
        entry_hash: Option<HoloHashB64>,
        commitment: CommitmentEntry,
        #[serde(default)]
        author: Option<HoloHashB64>,
    },
    ReaEconomicEventCommitted {
        action_hash: HoloHashB64,
        #[serde(default)]
        entry_hash: Option<HoloHashB64>,
        event: EconomicEventEntry,
        #[serde(default)]
        author: Option<HoloHashB64>,
    },
    /// Lamad Content entry committed. Carries the full Content entry shape
    /// for projection into the local SQL `content` table with anchor.
    /// Fires from the DNA post_commit for both create_content (initial
    /// publish) and update_content (e.g. blob_cid patches from stageSpaBlobs).
    ContentCommitted {
        action_hash: HoloHashB64,
        #[serde(default)]
        entry_hash: Option<HoloHashB64>,
        content: ContentEntry,
        #[serde(default)]
        author: Option<HoloHashB64>,
    },
    /// Notary-declared HEAD for a content id's version DAG (HEAD-election
    /// projection, Plan C3 / notary-authority Leg 2). The authoring conductor
    /// emits this ONLY for locally-authored HEAD declarations, so local
    /// authorship is implied. Field names/types mirror the DNA signal exactly:
    /// `ProjectionSignal::ContentHeadDeclared { content_id, head_action_hash,
    /// entry_hash, author, canonical_declared_at, canonical_earned }` — the
    /// wire field is `head_action_hash` (a raw 39-byte `ActionHash` array on the
    /// msgpack wire, hence `HoloHashB64`).
    ContentHeadDeclared {
        content_id: String,
        head_action_hash: HoloHashB64,
        #[serde(default)]
        entry_hash: Option<HoloHashB64>,
        #[serde(default)]
        author: Option<HoloHashB64>,
        /// Present together on cross-root declarations whose coordinator has
        /// already run the canonical election including the link it authored.
        /// Both absent is the legacy/single-author signal shape.
        #[serde(default)]
        canonical_declared_at: Option<i64>,
        #[serde(default)]
        canonical_earned: Option<bool>,
    },
}

/// Mirror-variant tags: a decode failure on one of these is a REAL projection
/// miss (loud); any other tag is a foreign signal sharing the app interface
/// (quiet at debug). The live `content_store` coordinator emits all four.
const REA_MIRROR_VARIANTS: &[&str] = &[
    "AgreementCommitted",
    "ReaCommitmentCommitted",
    "ReaEconomicEventCommitted",
    "ContentCommitted",
    "ContentHeadDeclared",
];

/// Cumulative count of REAL REA decode misses this process.
static REA_DECODE_MISSES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Cumulative REAL REA decode misses (mirror-variant tag present but the typed
/// decode failed). Exposed for status surfaces / tests.
pub fn rea_decode_miss_count() -> u64 {
    REA_DECODE_MISSES.load(std::sync::atomic::Ordering::Relaxed)
}

/// Decode a conductor app-signal payload (MessagePack `ExternIO` bytes) into
/// the typed `ReaProjectionSignal` mirror, classifying failures so the
/// subscriber can be loud about real misses without spamming on foreign
/// signals. Identical class/treatment to `decode_infrastructure_signal`
/// (d33b0e1f5).
pub fn decode_rea_projection_signal(
    bytes: &[u8],
) -> Result<ReaProjectionSignal, crate::signals::SignalDecodeMiss> {
    use crate::signals::SignalDecodeMiss;
    match rmp_serde::from_slice::<ReaProjectionSignal>(bytes) {
        Ok(signal) => Ok(signal),
        Err(typed_err) => {
            // Tag-only peek: serde skips the payload via IgnoredAny (which,
            // unlike `serde_json::Value`, tolerates msgpack byte arrays).
            #[derive(Deserialize)]
            struct TagOnly {
                #[serde(rename = "type")]
                type_tag: String,
            }
            match rmp_serde::from_slice::<TagOnly>(bytes) {
                Ok(tag) if REA_MIRROR_VARIANTS.contains(&tag.type_tag.as_str()) => {
                    REA_DECODE_MISSES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    Err(SignalDecodeMiss::ReaShapeMismatch {
                        type_tag: tag.type_tag,
                        error: typed_err.to_string(),
                    })
                }
                Ok(tag) => Err(SignalDecodeMiss::ForeignSignal {
                    type_tag: tag.type_tag,
                }),
                Err(tag_err) => Err(SignalDecodeMiss::Undecodable {
                    error: format!("typed: {typed_err}; tag peek: {tag_err}"),
                }),
            }
        }
    }
}

/// Mirror of DNA `Agreement` entry shape.
#[derive(Debug, Clone, Deserialize)]
pub struct AgreementEntry {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

/// Mirror of DNA `Commitment` entry shape. Field names/types match the
/// integrity zome exactly — including `_json` suffixes on multi-value
/// fields and `f64` quantities (storage downcasts to f32 in the handler).
#[derive(Debug, Clone, Deserialize)]
pub struct CommitmentEntry {
    pub id: String,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    #[serde(default)]
    pub resource_conforms_to: Option<String>,
    #[serde(default)]
    pub resource_inventoried_as: Option<String>,
    /// JSON-encoded `Vec<String>` on the wire. Decoded in the handler.
    #[serde(default)]
    pub resource_classified_as_json: Option<String>,
    #[serde(default)]
    pub resource_quantity_value: Option<f64>,
    #[serde(default)]
    pub resource_quantity_unit: Option<String>,
    #[serde(default)]
    pub effort_quantity_value: Option<f64>,
    #[serde(default)]
    pub effort_quantity_unit: Option<String>,
    #[serde(default)]
    pub has_point_in_time: Option<String>,
    #[serde(default)]
    pub has_beginning: Option<String>,
    #[serde(default)]
    pub has_end: Option<String>,
    #[serde(default)]
    pub due: Option<String>,
    #[serde(default)]
    pub clause_of: Option<String>,
    #[serde(default)]
    pub agreed_in: Option<String>,
    #[serde(default)]
    pub input_of: Option<String>,
    #[serde(default)]
    pub output_of: Option<String>,
    #[serde(default)]
    pub satisfies: Option<String>,
    /// JSON-encoded scope list on the wire. Decoded in the handler.
    #[serde(default)]
    pub in_scope_of_json: Option<String>,
    #[serde(default)]
    pub finished: bool,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub metadata_json: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// Mirror of DNA `Content` entry shape (lamad zome content_store).
///
/// Field naming note: the DNA's blob field is `blob_cid` (Phase 0 refactor
/// per substrate-rea-replication-fix Addendum 5). The storage projection
/// mirrors that to both `blob_cid` AND the legacy `blob_hash` SQL column
/// inside upsert_with_anchor.
#[derive(Debug, Clone, Deserialize)]
pub struct ContentEntry {
    pub id: String,
    pub content_type: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub content: String,
    pub content_format: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub source_path: Option<String>,
    #[serde(default)]
    pub related_node_ids: Vec<String>,
    #[serde(default)]
    pub author_id: Option<String>,
    pub reach: String,
    #[serde(default)]
    pub trust_score: f64,
    #[serde(default)]
    pub estimated_minutes: Option<u32>,
    #[serde(default)]
    pub thumbnail_url: Option<String>,
    #[serde(default)]
    pub metadata_json: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub validation_status: String,
    #[serde(default)]
    pub blob_cid: Option<String>,
    #[serde(default)]
    pub content_size_bytes: Option<u64>,
    #[serde(default)]
    pub content_hash: Option<String>,
}

/// Mirror of DNA `EconomicEvent` entry shape.
#[derive(Debug, Clone, Deserialize)]
pub struct EconomicEventEntry {
    pub id: String,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    #[serde(default)]
    pub resource_conforms_to: Option<String>,
    #[serde(default)]
    pub resource_inventoried_as: Option<String>,
    #[serde(default)]
    pub to_resource_inventoried_as: Option<String>,
    #[serde(default)]
    pub resource_classified_as_json: Option<String>,
    #[serde(default)]
    pub resource_quantity_value: Option<f64>,
    #[serde(default)]
    pub resource_quantity_unit: Option<String>,
    #[serde(default)]
    pub effort_quantity_value: Option<f64>,
    #[serde(default)]
    pub effort_quantity_unit: Option<String>,
    #[serde(default)]
    pub has_point_in_time: Option<String>,
    #[serde(default)]
    pub has_duration: Option<String>,
    #[serde(default)]
    pub input_of: Option<String>,
    #[serde(default)]
    pub output_of: Option<String>,
    #[serde(default)]
    pub fulfills_json: Option<String>,
    #[serde(default)]
    pub realization_of: Option<String>,
    #[serde(default)]
    pub satisfies_json: Option<String>,
    #[serde(default)]
    pub in_scope_of_json: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub triggered_by: Option<String>,
    #[serde(default)]
    pub at_location: Option<String>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub lamad_event_type: Option<String>,
    #[serde(default)]
    pub metadata_json: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub substrate_signal: Option<String>,
}

// ============================================================================
// Helpers
// ============================================================================

/// Parse a JSON-encoded `Vec<String>` field from the DNA entry. Empty
/// string or invalid JSON → empty Vec. Used for the `_json` resource and
/// scope fields. Drops empty entries so downstream code can treat `is_empty`
/// as "no value".
///
/// Public so the eager-projection path in the service layer can share the
/// same logic without duplicating it (Gap-F fix — see
/// `services/rea_commitment_service.rs` and `services/content_service.rs`).
pub fn parse_json_strings(raw: Option<&str>) -> Vec<String> {
    let s = match raw {
        Some(s) if !s.is_empty() => s,
        _ => return Vec::new(),
    };
    serde_json::from_str::<Vec<String>>(s).unwrap_or_else(|_| Vec::new())
}

/// Take the first element of a parsed JSON Vec<String>, or None.
/// Storage's CreateReaCommitmentInput stores resource_classified_as and
/// in_scope_of as single-value `Option<String>` columns; downstream readers
/// can reconstruct multi-value via the DHT entry if needed.
///
/// Public so the eager-projection path in the service layer can share the
/// same logic (Gap-F fix).
pub fn first_or_none(v: Vec<String>) -> Option<String> {
    v.into_iter().find(|s| !s.is_empty())
}

/// The Commitment fields the storage projection consumes, in the shape both
/// the post-commit signal (`CommitmentEntry`, `_json` fields `Option<String>`)
/// and the conductor read (`shefa_types::Commitment`, `_json` fields `String`)
/// can produce. Borrowed so neither caller has to clone its source entry.
///
/// Factored out (P1 reconciliation stream) so the wire→`CreateReaCommitmentInput`
/// mapping has exactly ONE home: the signal handler and the projection
/// reconciler both go through [`project_commitment_from_wire`]. A second
/// bespoke mapping would be a coherence violation — the same discipline the
/// reconcile rails enforce for the gap state machine.
pub struct CommitmentWireFields<'a> {
    pub id: &'a str,
    pub action: &'a str,
    pub provider: &'a str,
    pub receiver: &'a str,
    pub resource_conforms_to: Option<&'a str>,
    /// Raw JSON-encoded `Vec<String>` (or None / empty). The builder parses +
    /// takes first, matching the storage column's single-value shape.
    pub resource_classified_as_json: Option<&'a str>,
    pub resource_quantity_value: Option<f64>,
    pub resource_quantity_unit: Option<&'a str>,
    pub effort_quantity_value: Option<f64>,
    pub effort_quantity_unit: Option<&'a str>,
    pub has_beginning: Option<&'a str>,
    pub has_end: Option<&'a str>,
    pub due: Option<&'a str>,
    pub clause_of: Option<&'a str>,
    /// Raw JSON-encoded scope list (or None / empty).
    pub in_scope_of_json: Option<&'a str>,
    pub note: Option<&'a str>,
    pub metadata_json: Option<&'a str>,
    /// The commitment's LIFECYCLE STATE as the DHT entry carries it
    /// (`proposed` | `active` | `fulfilled` | …).
    ///
    /// `None` — or an empty string, which is what both wire sources produce for
    /// an entry authored before the field existed — means the wire genuinely
    /// omits a state, and the projection falls back to the REA birth state
    /// `proposed` (see [`crate::db::rea_commitments::DEFAULT_COMMITMENT_STATE`]).
    /// A carried state is projected VERBATIM.
    ///
    /// Added 2026-09-12: without it, a custody-blob commitment reached every
    /// peer and then froze at `proposed` on every non-authoring peer while the
    /// author held `active` — rows travelled, standing did not
    /// (blob-durability DELTA 2026-09-12c, cause 1). The projection-reconcile
    /// discovery arm compares this field across peers
    /// (`p2p::projection_reconcile::classify_rea_row_gap`), so dropping it here
    /// would silently un-measure every state-only divergence on the fleet.
    pub state: Option<&'a str>,
}

/// Map the canonical typed coordinator/record Commitment through the existing wire mapper.
pub(crate) fn project_typed_commitment(c: &shefa_types::Commitment) -> CreateReaCommitmentInput {
    project_commitment_from_wire(&CommitmentWireFields {
        id: &c.id,
        action: &c.action,
        provider: &c.provider,
        receiver: &c.receiver,
        resource_conforms_to: c.resource_conforms_to.as_deref(),
        // shefa_types::Commitment carries `_json` as non-optional String;
        // an empty string parses to an empty Vec in the shared mapping.
        resource_classified_as_json: Some(c.resource_classified_as_json.as_str()),
        resource_quantity_value: c.resource_quantity_value,
        resource_quantity_unit: c.resource_quantity_unit.as_deref(),
        effort_quantity_value: c.effort_quantity_value,
        effort_quantity_unit: c.effort_quantity_unit.as_deref(),
        has_beginning: c.has_beginning.as_deref(),
        has_end: c.has_end.as_deref(),
        due: c.due.as_deref(),
        clause_of: c.clause_of.as_deref(),
        in_scope_of_json: Some(c.in_scope_of_json.as_str()),
        note: c.note.as_deref(),
        metadata_json: Some(c.metadata_json.as_str()),
        // The authenticated entry's own standing, projected verbatim. The
        // lifecycle CAS re-states it authoritatively, but the mapped input must
        // not disagree with the record it was built from.
        state: Some(c.state.as_str()),
    })
}

/// Build the storage-side `CreateReaCommitmentInput` from the canonical
/// Commitment wire fields. THE single mapping site for both the post-commit
/// signal path ([`handle_rea_signal`]) and the projection reconciler
/// (`p2p::projection_reconcile`).
///
/// `supersedes` is always `None`: a projection of an already-committed entry
/// never re-runs supersession — that happened on the originating create.
/// `medium_of_exchange_id` is `None`: not carried on the DHT Commitment entry.
///
/// `state` is threaded through as `Option<String>`: `Some` ⇒ the wire carried a
/// lifecycle state and the projection adopts it verbatim; `None` ⇒ the wire
/// genuinely omitted one and the insert paths fall back to
/// [`crate::db::rea_commitments::DEFAULT_COMMITMENT_STATE`]. An EMPTY string is
/// an omission, not a state — both wire sources default the field to `""`.
pub fn project_commitment_from_wire(fields: &CommitmentWireFields<'_>) -> CreateReaCommitmentInput {
    let classified = first_or_none(parse_json_strings(fields.resource_classified_as_json));
    let in_scope_of = first_or_none(parse_json_strings(fields.in_scope_of_json));
    let state = fields
        .state
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    CreateReaCommitmentInput {
        id: Some(fields.id.to_string()),
        action: fields.action.to_string(),
        provider: fields.provider.to_string(),
        receiver: fields.receiver.to_string(),
        resource_conforms_to: fields.resource_conforms_to.map(str::to_string),
        resource_classified_as: classified,
        resource_quantity_value: fields.resource_quantity_value.map(|v| v as f32),
        resource_quantity_unit: fields.resource_quantity_unit.map(str::to_string),
        effort_quantity_value: fields.effort_quantity_value.map(|v| v as f32),
        effort_quantity_unit: fields.effort_quantity_unit.map(str::to_string),
        has_beginning: fields.has_beginning.map(str::to_string),
        has_end: fields.has_end.map(str::to_string),
        due: fields.due.map(str::to_string),
        clause_of: fields.clause_of.map(str::to_string),
        in_scope_of,
        medium_of_exchange_id: None,
        note: fields.note.map(str::to_string),
        metadata_json: fields.metadata_json.map(str::to_string),
        supersedes: None,
        state,
    }
}

// ============================================================================
// Signal Handler
// ============================================================================

/// Ordered declarations must acquire their authenticated payload before they
/// advance the SQL head. Legacy declarations retain the synchronous stamp path;
/// a half-present ordering pair remains malformed and is refused there.
pub fn requires_authenticated_head_projection(signal: &ReaProjectionSignal) -> bool {
    matches!(
        signal,
        ReaProjectionSignal::ContentHeadDeclared {
            canonical_declared_at: Some(_),
            canonical_earned: Some(_),
            ..
        }
    )
}

fn current_lamad_client(
    registry: &crate::hc_client_registry::HcClientRegistry,
) -> Result<std::sync::Arc<crate::hc_client::HcClient>, StorageError> {
    registry.lamad_client().ok_or_else(|| {
        StorageError::Conductor(
            "authenticated content_store read deferred: current lamad bridge unavailable".into(),
        )
    })
}

fn validate_ordered_content_head(
    content_id: &str,
    head_action_hash: &HoloHashB64,
    ordering: content_diesel::CanonicalOrdering,
    head: crate::services::conductor_writes::ContentHeadWire,
) -> Result<crate::services::conductor_writes::ContentHeadWire, StorageError> {
    if head.content_id != content_id
        || head.content.id != content_id
        || head.head_action_hash.as_str() != head_action_hash.as_str()
        || !head.canonical
        || head.canonical_ordering() != Some(ordering)
    {
        return Err(StorageError::InvalidInput(format!(
            "ordered ContentHeadDeclared did not resolve its exact canonical payload: {content_id}"
        )));
    }
    Ok(head)
}

fn apply_ordered_content_head(
    content_id: &str,
    head_action_hash: &HoloHashB64,
    ordering: content_diesel::CanonicalOrdering,
    head: crate::services::conductor_writes::ContentHeadWire,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<content_diesel::StampOutcome, StorageError> {
    let head = validate_ordered_content_head(content_id, head_action_hash, ordering, head)?;
    crate::p2p::projection_reconcile::project_authenticated_content_head(&head, pool, ctx)
}

fn ordered_head_from_batch(
    content_id: &str,
    output: crate::services::conductor_writes::BatchResolveOutput<
        crate::services::conductor_writes::ContentHeadWire,
    >,
) -> Result<crate::services::conductor_writes::ContentHeadWire, StorageError> {
    if output.schema_version != crate::services::conductor_writes::BATCH_RESOLVE_SCHEMA_VERSION {
        return Err(StorageError::Serialization(format!(
            "ordered head batch schema {} is unsupported",
            output.schema_version
        )));
    }
    if !output.unattempted.is_empty() || output.stop_reason.is_some() {
        return Err(StorageError::Conductor(
            "ordered head batch returned partial or stopped work".into(),
        ));
    }
    let mut attempted = output.attempted.into_iter();
    let Some(attempt) = attempted.next() else {
        return Err(StorageError::NotFound(format!(
            "ordered head payload unavailable for {content_id}"
        )));
    };
    if attempt.id != content_id || attempted.next().is_some() {
        return Err(StorageError::Serialization(
            "ordered head batch returned an unexpected item set".into(),
        ));
    }
    match attempt.outcome {
        crate::services::conductor_writes::BatchOutcome::Resolved(Some(head)) => Ok(head),
        crate::services::conductor_writes::BatchOutcome::Resolved(None) => Err(
            StorageError::NotFound(format!("ordered head payload unavailable for {content_id}")),
        ),
        crate::services::conductor_writes::BatchOutcome::Failed(failure) => {
            Err(StorageError::Conductor(format!(
                "ordered head payload resolution failed: {:?}/{:?}",
                failure.phase, failure.reason
            )))
        }
    }
}

async fn bounded_ordered_head_call<F>(
    call: F,
) -> Result<
    crate::services::conductor_writes::BatchCall<
        crate::services::conductor_writes::ContentHeadWire,
    >,
    StorageError,
>
where
    F: std::future::Future<
        Output = Result<
            crate::services::conductor_writes::BatchCall<
                crate::services::conductor_writes::ContentHeadWire,
            >,
            StorageError,
        >,
    >,
{
    tokio::time::timeout(
        crate::p2p::projection_reconcile::HEAL_ATTEMPT_TIMEOUT,
        call,
    )
    .await
    .map_err(|_| {
        StorageError::Conductor(
            "ordered head payload read exceeded the existing heal attempt timeout; already-running WASM work may continue"
                .into(),
        )
    })?
}

/// Hydrate one complete ordered declaration from the own conductor before any
/// SQL pointer moves. The one-id batch extern carries its existing in-WASM
/// budget; an unavailable, unattempted, failed, stale, or malformed answer
/// writes nothing. A fresh A→B signal therefore keeps its prior divergent
/// anchor for normal reconciliation; an already-split row awaits another exact
/// signal or authenticated projection rather than being mutated further.
pub async fn handle_authenticated_content_head_signal(
    signal: ReaProjectionSignal,
    registry: &crate::hc_client_registry::HcClientRegistry,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<(), StorageError> {
    let ReaProjectionSignal::ContentHeadDeclared {
        content_id,
        head_action_hash,
        canonical_declared_at: Some(canonical_declared_at),
        canonical_earned: Some(canonical_earned),
        ..
    } = signal
    else {
        return Err(StorageError::InvalidInput(
            "authenticated head projection requires a complete ordered declaration".into(),
        ));
    };

    // This subscriber is registered through the infrastructure bridge because
    // that app websocket carries every installed cell's signals. The content
    // read itself belongs to lamad, though. Resolve its CURRENT supervised
    // client for every job so a conductor restart swaps this path along with
    // the rest of the registry instead of leaving a captured websocket behind.
    let hc = current_lamad_client(registry)?;

    let ids = [content_id.clone()];
    let budget_ms =
        u32::try_from(crate::services::head_batch_resolver::BATCH_EXTERN_BUDGET.as_millis())
            .unwrap_or(u32::MAX);
    let call = bounded_ordered_head_call(
        crate::services::conductor_writes::call_resolve_content_heads_local(
            &hc,
            &ids,
            Some(budget_ms),
        ),
    )
    .await?;
    let head = ordered_head_from_batch(&content_id, call.out)?;
    apply_ordered_content_head(
        &content_id,
        &head_action_hash,
        (canonical_declared_at, canonical_earned),
        head,
        pool,
        ctx,
    )?;
    notify_content_touched(&content_id);
    Ok(())
}

/// Handle an incoming REA projection signal from the conductor.
///
/// Main entry point — called from the signal dispatch loop. Acquires a DB
/// connection from the pool and upserts the projection row with dht_anchor_hash.
pub fn handle_rea_signal(
    signal: ReaProjectionSignal,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<(), StorageError> {
    let mut conn = pool
        .get()
        .map_err(|e| StorageError::Internal(format!("Pool error: {e}")))?;
    // Content rows written below change fields the sync doc projects
    // (anchor, declared head, value patch) — announce them once the write
    // has committed, at the end of this fn.
    let touched_content_id: Option<String> = match &signal {
        ReaProjectionSignal::ContentCommitted { content, .. } => Some(content.id.clone()),
        ReaProjectionSignal::ContentHeadDeclared { content_id, .. } => Some(content_id.clone()),
        _ => None,
    };

    match signal {
        ReaProjectionSignal::AgreementCommitted {
            action_hash,
            agreement,
            ..
        } => {
            info!(id = %agreement.id, hash = %action_hash, "Projecting Agreement from DHT");
            let input = CreateAgreementInput {
                id: Some(agreement.id),
                name: agreement.name,
                note: agreement.note,
                // Agreement DNA entry has no metadata_json; projection writes None.
                metadata_json: None,
            };
            agreements::upsert_agreement(&mut conn, ctx, input, Some(action_hash.as_str()))?;
        }
        ReaProjectionSignal::ReaCommitmentCommitted { .. } => {
            return Err(StorageError::InvalidInput(
                "REA lifecycle requires authenticated async projection".into(),
            ));
        }
        ReaProjectionSignal::ReaEconomicEventCommitted {
            action_hash, event, ..
        } => {
            info!(id = %event.id, hash = %action_hash, "Projecting EconomicEvent from DHT");

            // Phase 4 T4 — side-projection: if action='ack-projection', also
            // write into the projection_events operational log. Self-filtering:
            // other EconomicEvent actions (custody-blob, serve-blob) are ignored.
            let classified = parse_json_strings(event.resource_classified_as_json.as_deref());
            {
                let first_resource = classified.first().cloned().unwrap_or_default();
                let emitted_at = event
                    .has_point_in_time
                    .clone()
                    .unwrap_or_else(|| Utc::now().to_rfc3339());
                if let Err(e) = crate::p2p::projection_ack_handler::handle_projection_ack_sync(
                    pool,
                    &event.action,
                    &event.provider,
                    &first_resource,
                    action_hash.as_str(),
                    &emitted_at,
                ) {
                    warn!(
                        target = "rea_projection",
                        error = %e,
                        "projection_ack side-projection failed (non-fatal)"
                    );
                }
            }

            // Spec §6.5 — projection-driven graduation: extract bounded_by
            // BEFORE moving event fields into the input struct.  bounded_by is
            // carried in metadata_json as `{"bounded_by": "<cid>"}` (emit service
            // annotation binding — the emit service puts it there for diagnostics
            // and projection consumers; it is the source the local SQL bounded_by
            // column is populated from when writing via the HTTP path).
            let bounded_by_cid: Option<String> = event
                .metadata_json
                .as_deref()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                .and_then(|v| {
                    v.get("bounded_by")
                        .and_then(|b| b.as_str())
                        .map(str::to_string)
                })
                .filter(|s| !s.is_empty());

            let input = CreateEconomicEventInput {
                id: Some(event.id),
                action: event.action,
                provider: event.provider,
                receiver: event.receiver,
                resource_conforms_to: event.resource_conforms_to,
                resource_inventoried_as: event.resource_inventoried_as,
                resource_classified_as: classified,
                resource_quantity_value: event.resource_quantity_value.map(|v| v as f32),
                resource_quantity_unit: event.resource_quantity_unit,
                effort_quantity_value: event.effort_quantity_value.map(|v| v as f32),
                effort_quantity_unit: event.effort_quantity_unit,
                has_point_in_time: event.has_point_in_time,
                has_duration: event.has_duration,
                input_of: event.input_of,
                output_of: event.output_of,
                lamad_event_type: event.lamad_event_type,
                // The CreateEconomicEventInput surface has app-specific link fields
                // (content_id, contributor_presence_id, path_id) that are not on
                // the DNA wire — leave None; the HTTP write path sets them.
                content_id: None,
                contributor_presence_id: None,
                path_id: None,
                triggered_by: event.triggered_by,
                note: event.note,
                metadata_json: event.metadata_json,
                at_location: event.at_location,
                scope_collab_cid: None,
                substrate_signal: event.substrate_signal,
            };
            economic_events::upsert_with_anchor(&mut conn, ctx, input, Some(action_hash.as_str()))?;

            // The act of providing IS the acceptance: a bounded_by event projecting
            // graduates its Mishpat commitment proposed → active (spec §6.5). No-op
            // if the commitment isn't yet projected or isn't 'proposed'.
            if let Some(ref bounded) = bounded_by_cid {
                match crate::db::mishpat_commitments::graduate_to_active(&mut conn, bounded) {
                    Ok(rows) if rows > 0 => {
                        // SQL cache flipped (an actual proposed→active transition) →
                        // record the lifecycle TRUTH as a CommitmentByState link. The
                        // link author is async and needs the conductor; hand the
                        // transition to the signal subscriber (which holds the
                        // HcClient) to drain. The action_hash of THIS graduating event
                        // is the proof the link's target carries; signed_at is the
                        // projection time (Category-A — supplied here, never sys_time
                        // in-zome).
                        let signed_at = Utc::now().to_rfc3339();
                        info!(
                            cid = %bounded,
                            event_hash = %action_hash,
                            "graduation projection: proposed→active (CommitmentByState link author queued)"
                        );
                        record_pending_state_link(
                            bounded,
                            "active",
                            action_hash.as_str(),
                            &signed_at,
                        );
                    }
                    Ok(_) => { /* not 'proposed' — no transition, no link */ }
                    Err(e) => {
                        debug!(
                            error = %e,
                            cid = %bounded,
                            "graduation projection: graduate_to_active failed"
                        );
                    }
                }
            }
        }
        ReaProjectionSignal::ContentCommitted {
            action_hash,
            content,
            ..
        } => {
            info!(
                id = %content.id,
                hash = %action_hash,
                blob_cid = ?content.blob_cid,
                "Projecting Content from DHT"
            );
            // u64 → i32 with saturating cast. Real content sizes fit in
            // i32 with room to spare (max ~2.1 GB per blob; doorway proxy
            // caps far lower); the cast is defensive against malformed
            // entries claiming absurd sizes.
            let size_i32 = content
                .content_size_bytes
                .map(|n| i32::try_from(n).unwrap_or(i32::MAX));
            let patch = ContentProjectionPatch {
                blob_cid: content.blob_cid,
                content_size_bytes: size_i32,
                title: Some(content.title),
                description: Some(content.description),
                content_type: Some(content.content_type),
                content_format: Some(content.content_format),
                reach: Some(content.reach),
                metadata_json: Some(content.metadata_json),
                // 1.4b: the post-commit signal carries the Content ENTRY, not a
                // signed Record — see the measure-first finding. `None`
                // preserves; the producer fill adds the record asynchronously.
                declared_head_record_json: None,
            };
            // HEAD-ELECTION: the async own-conductor commit signal stamps the
            // ANCHOR only — it is NOT a declaration channel.
            //
            // This arm fires for EVERY own-conductor commit, including the ones
            // the heal sweeps cause (boot re-anchor, ghost witness). If it
            // declared, the adopt-before-author fix would be cosmetic: the eager
            // write would preserve the adopted head and this signal would re-crown
            // the freshly-authored local root milliseconds later, exactly
            // reproducing the defect. Declaration is the job of the explicit
            // channels — `ContentHeadDeclared` (the arm directly below), the
            // declare routes, and the adopt-before-author pre-flight.
            content_diesel::upsert_with_anchor(
                &mut conn,
                ctx,
                &content.id,
                patch,
                action_hash.as_str(),
                content_diesel::HeadElection::PreserveExistingDeclaration,
            )?;
        }
        ReaProjectionSignal::ContentHeadDeclared {
            content_id,
            head_action_hash,
            canonical_declared_at,
            canonical_earned,
            ..
        } => {
            info!(
                id = %content_id,
                hash = %head_action_hash,
                "Declaring Content HEAD from DHT"
            );
            // Own-conductor-witnessed HEAD declaration → verified stamp on the
            // EXISTING row only (no insert: a HEAD declaration for a row this
            // node never seeded is a no-op here). No value patch — the HEAD
            // declaration carries only the action, not content fields.
            // Cross-root signals carry the exact winning link ordering and
            // replay the canonical heal guard. Legacy/single-author signals
            // carry neither ordering field: they may fill, refresh, or move an
            // unordered legacy head, but may not move an ordered head. Modern
            // cross-root declarations are routed by the subscriber through the
            // async authenticated-payload handler above; reaching this sync arm
            // with a complete pair refuses before any pointer moves. The
            // independent projection-reconcile sweep also re-reads this node's
            // authenticated conductor election. A half-present pair is
            // malformed and must not move the row.
            let stamped = match (canonical_declared_at, canonical_earned) {
                (Some(_), Some(_)) => {
                    return Err(StorageError::InvalidInput(
                        "ordered ContentHeadDeclared requires authenticated payload projection"
                            .into(),
                    ))
                }
                (None, None) => content_diesel::stamp_declared_head_mode(
                    &mut conn,
                    ctx,
                    &content_id,
                    head_action_hash.as_str(),
                    None,
                    None,
                    content_diesel::StampMode::LegacySignal,
                    None,
                )?,
                _ => {
                    tracing::warn!(
                        id = %content_id,
                        "ContentHeadDeclared: incomplete canonical ordering — refusing stamp"
                    );
                    content_diesel::StampOutcome::SkippedStale
                }
            };
            if stamped == content_diesel::StampOutcome::NoRow {
                debug!(
                    id = %content_id,
                    "ContentHeadDeclared: no local row to stamp — declared-head projection skipped"
                );
            }
        }
    }

    if let Some(id) = touched_content_id {
        notify_content_touched(&id);
    }

    Ok(())
}

/// Process a commitment notification as a hint to read its exact own-conductor
/// record; never project lifecycle or parties from the unverified signal payload.
pub async fn handle_authenticated_commitment_signal(
    signal: ReaProjectionSignal,
    registry: &crate::hc_client_registry::HcClientRegistry,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<(), StorageError> {
    let ReaProjectionSignal::ReaCommitmentCommitted {
        action_hash,
        commitment,
        ..
    } = signal
    else {
        return Err(StorageError::InvalidInput(
            "expected Commitment signal".into(),
        ));
    };
    let hc = current_lamad_client(registry)?;
    let outcome = crate::services::rea_commitment_projection::project(
        &hc,
        pool,
        ctx,
        &commitment.id,
        action_hash.as_str(),
    )
    .await?;
    if outcome == crate::db::rea_commitment_lifecycle::ApplyOutcome::Deferred {
        return Err(StorageError::InvalidInput(
            "REA lifecycle concurrent change; deferred".into(),
        ));
    }
    Ok(())
}

/// Try to parse and handle a raw signal payload as an REA projection signal.
/// Returns Ok(true) if handled, Ok(false) if not an REA signal, Err on failure.
pub fn try_handle_signal(
    raw_payload: &[u8],
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<bool, StorageError> {
    match serde_json::from_slice::<ReaProjectionSignal>(raw_payload) {
        Ok(signal) => {
            handle_rea_signal(signal, pool, ctx)?;
            Ok(true)
        }
        Err(_) => {
            // Not an REA projection signal — caller should try other handlers
            Ok(false)
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signals::SignalDecodeMiss;

    fn content_signal_test_pool() -> DbPool {
        use diesel::r2d2::{ConnectionManager, Pool};
        use diesel::SqliteConnection;

        let url = format!(
            "file:content_signal_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(url))
            .expect("pool");
        crate::db::run_migrations(&pool).expect("migrations");
        pool
    }

    fn ordered_head(
        id: &str,
        action_hash: &str,
        blob: &str,
        ordering: content_diesel::CanonicalOrdering,
    ) -> crate::services::conductor_writes::ContentHeadWire {
        serde_json::from_value(serde_json::json!({
            "content_id": id,
            "head_action_hash": action_hash,
            "declared_at": 1_700_000_000_000_000i64,
            "canonical": true,
            "canonical_declared_at": ordering.0,
            "canonical_earned": ordering.1,
            "content": {
                "id": id,
                "content_type": "concept",
                "title": format!("title-{blob}"),
                "description": "d",
                "content_format": "markdown",
                "reach": "commons",
                "metadata_json": format!(r#"{{"blob":"{blob}"}}"#),
                "blob_cid": blob,
            },
        }))
        .expect("ContentHeadWire fixture must deserialize")
    }

    fn seed_content(pool: &DbPool, id: &str, blob: &str) {
        let mut conn = pool.get().expect("connection");
        crate::db::content_diesel::create_content(
            &mut conn,
            &AppContext::default_lamad(),
            crate::db::content_diesel::CreateContentInput {
                id: id.into(),
                title: format!("title-{blob}"),
                description: Some("d".into()),
                content_type: "concept".into(),
                content_format: "markdown".into(),
                blob_hash: Some(blob.into()),
                blob_cid: Some(blob.into()),
                content_size_bytes: None,
                metadata_json: Some(format!(r#"{{"blob":"{blob}"}}"#)),
                reach: "commons".into(),
                created_by: None,
                tags: Vec::new(),
                content_body: None,
                dht_anchor_hash: None,
            },
        )
        .expect("seed content");
    }

    #[test]
    fn subscriber_routes_only_complete_ordered_heads_to_authenticated_hydration() {
        let signal = |at, earned| ReaProjectionSignal::ContentHeadDeclared {
            content_id: "route-head".into(),
            head_action_hash: HoloHashB64("uhCkk-route-head".into()),
            entry_hash: None,
            author: None,
            canonical_declared_at: at,
            canonical_earned: earned,
        };
        assert!(requires_authenticated_head_projection(&signal(
            Some(10),
            Some(true)
        )));
        assert!(!requires_authenticated_head_projection(&signal(None, None)));
        assert!(!requires_authenticated_head_projection(&signal(
            Some(10),
            None
        )));
    }

    #[tokio::test]
    async fn ordered_head_hydration_requires_the_current_lamad_registry_slot() {
        let pool = content_signal_test_pool();
        let ctx = AppContext::default_lamad();
        let registry = crate::hc_client_registry::HcClientRegistry::empty();
        let result = handle_authenticated_content_head_signal(
            ReaProjectionSignal::ContentHeadDeclared {
                content_id: "role-bound-head".into(),
                head_action_hash: HoloHashB64("uhCkk-role-bound-head".into()),
                entry_hash: None,
                author: None,
                canonical_declared_at: Some(10),
                canonical_earned: Some(true),
            },
            &registry,
            &pool,
            &ctx,
        )
        .await;

        assert!(matches!(
            result,
            Err(StorageError::Conductor(message))
                if message.contains("current lamad bridge unavailable")
        ));
        let mut conn = pool.get().expect("connection");
        assert!(
            crate::db::content_diesel::get_content(
                &mut conn,
                &ctx,
                "role-bound-head",
                crate::db::content_diesel::MinTrust::Invisible,
            )
            .expect("read projection")
            .is_none(),
            "an absent lamad slot must not fall back to the infrastructure client or touch SQL"
        );
    }

    #[test]
    fn exact_authenticated_payload_repairs_a_split_head_and_blob_atomically() {
        let pool = content_signal_test_pool();
        let ctx = AppContext::default_lamad();
        let id = "split-head-payload";
        seed_content(&pool, id, "blob-a");
        {
            let mut conn = pool.get().expect("connection");
            content_diesel::stamp_declared_head_mode(
                &mut conn,
                &ctx,
                id,
                "uhCkk-head-b",
                Some(20),
                None,
                content_diesel::StampMode::HealCanonical,
                Some((20, true)),
            )
            .expect("reproduce split pointer stamp");
        }

        apply_ordered_content_head(
            id,
            &HoloHashB64("uhCkk-head-b".into()),
            (20, true),
            ordered_head(id, "uhCkk-head-b", "blob-b", (20, true)),
            &pool,
            &ctx,
        )
        .expect("hydrate exact ordered payload");

        let mut conn = pool.get().expect("connection");
        let row =
            content_diesel::get_content(&mut conn, &ctx, id, content_diesel::MinTrust::Invisible)
                .expect("read")
                .expect("row");
        assert_eq!(
            row.declared_head_action_hash.as_deref(),
            Some("uhCkk-head-b")
        );
        assert_eq!(row.dht_anchor_hash.as_deref(), Some("uhCkk-head-b"));
        assert_eq!(row.blob_cid.as_deref(), Some("blob-b"));
        assert_eq!(row.blob_hash.as_deref(), Some("blob-b"));
        assert_eq!(row.metadata_json.as_deref(), Some(r#"{"blob":"blob-b"}"#));
    }

    #[test]
    fn stale_or_misbound_ordered_payload_cannot_advance_any_projection_field() {
        let pool = content_signal_test_pool();
        let ctx = AppContext::default_lamad();
        let id = "refuse-misbound-payload";
        seed_content(&pool, id, "blob-a");

        let error = apply_ordered_content_head(
            id,
            &HoloHashB64("uhCkk-head-b".into()),
            (20, true),
            ordered_head(id, "uhCkk-head-a", "blob-stale", (10, true)),
            &pool,
            &ctx,
        )
        .expect_err("mismatched action and ordering must refuse");
        assert!(error.to_string().contains("exact canonical payload"));

        let mut conn = pool.get().expect("connection");
        let row =
            content_diesel::get_content(&mut conn, &ctx, id, content_diesel::MinTrust::Invisible)
                .expect("read")
                .expect("row");
        assert_eq!(row.declared_head_action_hash, None);
        assert_eq!(row.dht_anchor_hash, None);
        assert_eq!(row.blob_cid.as_deref(), Some("blob-a"));
        assert_eq!(row.metadata_json.as_deref(), Some(r#"{"blob":"blob-a"}"#));
    }

    #[test]
    fn absent_or_failed_bounded_head_read_is_a_deferral_not_a_pointer() {
        use crate::services::conductor_writes::{
            BatchAttempt, BatchFailureReason, BatchOutcome, BatchResolveFailure,
            BatchResolveOutput, BatchResolvePhase, BATCH_RESOLVE_SCHEMA_VERSION,
        };
        let output = |outcome| BatchResolveOutput {
            schema_version: BATCH_RESOLVE_SCHEMA_VERSION,
            attempted: vec![BatchAttempt {
                id: "deferred-head".into(),
                outcome,
            }],
            unattempted: Vec::new(),
            stop_reason: None,
            elapsed_ms: 1,
        };
        assert!(matches!(
            ordered_head_from_batch("deferred-head", output(BatchOutcome::Resolved(None))),
            Err(StorageError::NotFound(_))
        ));
        assert!(matches!(
            ordered_head_from_batch(
                "deferred-head",
                output(BatchOutcome::Failed(BatchResolveFailure {
                    reason: BatchFailureReason::PermitTimeout,
                    phase: BatchResolvePhase::HeadResolve,
                }))
            ),
            Err(StorageError::Conductor(_))
        ));

        let stopped = BatchResolveOutput {
            schema_version: BATCH_RESOLVE_SCHEMA_VERSION,
            attempted: Vec::new(),
            unattempted: vec!["deferred-head".into()],
            stop_reason: Some(crate::services::conductor_writes::BatchStopReason::BudgetExhausted),
            elapsed_ms: 12_000,
        };
        assert!(matches!(
            ordered_head_from_batch("deferred-head", stopped),
            Err(StorageError::Conductor(_))
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn timed_out_ordered_head_read_leaves_the_projection_untouched() {
        let pool = content_signal_test_pool();
        let ctx = AppContext::default_lamad();
        let id = "timed-out-head";
        seed_content(&pool, id, "blob-a");

        let error = bounded_ordered_head_call(std::future::pending::<
            Result<
                crate::services::conductor_writes::BatchCall<
                    crate::services::conductor_writes::ContentHeadWire,
                >,
                StorageError,
            >,
        >())
        .await
        .expect_err("the existing heal attempt timeout must bound the worker");
        assert!(error.to_string().contains("heal attempt timeout"));

        let mut conn = pool.get().expect("connection");
        let row =
            content_diesel::get_content(&mut conn, &ctx, id, content_diesel::MinTrust::Invisible)
                .expect("read")
                .expect("row");
        assert_eq!(row.declared_head_action_hash, None);
        assert_eq!(row.dht_anchor_hash, None);
        assert_eq!(row.blob_cid.as_deref(), Some("blob-a"));
    }

    #[test]
    fn delayed_legacy_head_signal_cannot_roll_back_an_ordered_projection() {
        let pool = content_signal_test_pool();
        let ctx = AppContext::default_lamad();
        let mut conn = pool.get().expect("connection");
        crate::db::content_diesel::create_content(
            &mut conn,
            &ctx,
            crate::db::content_diesel::CreateContentInput {
                id: "signal-ordered-head".into(),
                title: "signal-ordered-head".into(),
                description: None,
                content_type: "concept".into(),
                content_format: "markdown".into(),
                blob_hash: Some("sha256-browser-b".into()),
                blob_cid: Some("sha256-browser-b".into()),
                content_size_bytes: Some(10),
                metadata_json: Some("{}".into()),
                reach: "commons".into(),
                created_by: None,
                tags: Vec::new(),
                content_body: None,
                dht_anchor_hash: None,
            },
        )
        .expect("seed row");
        crate::db::content_diesel::stamp_declared_head_mode(
            &mut conn,
            &ctx,
            "signal-ordered-head",
            "uhCkk-browser-b",
            Some(2_000),
            None,
            crate::db::content_diesel::StampMode::HealCanonical,
            Some((2_000, true)),
        )
        .expect("ordered stamp");
        drop(conn);

        handle_rea_signal(
            ReaProjectionSignal::ContentHeadDeclared {
                content_id: "signal-ordered-head".into(),
                head_action_hash: HoloHashB64("uhCkk-browser-a".into()),
                entry_hash: None,
                author: None,
                canonical_declared_at: None,
                canonical_earned: None,
            },
            &pool,
            &ctx,
        )
        .expect("legacy signal is handled");

        let mut conn = pool.get().expect("connection");
        let row = crate::db::content_diesel::get_content(
            &mut conn,
            &ctx,
            "signal-ordered-head",
            crate::db::content_diesel::MinTrust::Invisible,
        )
        .expect("read row")
        .expect("row exists");
        assert_eq!(
            row.declared_head_action_hash.as_deref(),
            Some("uhCkk-browser-b")
        );
        assert_eq!(row.dht_anchor_hash.as_deref(), Some("uhCkk-browser-b"));
    }

    /// THE 2026-06-13 root-cause regression test (REA arm): decode the signal
    /// from the REAL conductor wire — MessagePack (`ExternIO`), where
    /// `ActionHash`/`EntryHash`/`AgentPubKey` serialize as raw 39-BYTE ARRAYS,
    /// not base64 strings. The old `String` mirror (and the `rmp →
    /// serde_json::Value` pre-pass before it) failed on every such signal,
    /// silently never landing the substrate-correct write-path projection.
    #[test]
    fn rea_commitment_signal_decodes_from_conductor_msgpack_wire() {
        use holochain_types::prelude::{ActionHash, AgentPubKey, EntryHash};
        use serde::Serialize;

        // DNA-side shape: the content_store `ProjectionSignal::ReaCommitmentCommitted`
        // variant with REAL holo_hash types and an all-String Commitment entry
        // (field-by-field per content_store_integrity::Commitment).
        #[derive(Serialize)]
        struct DnaCommitment {
            id: String,
            action: String,
            provider: String,
            receiver: String,
            resource_classified_as_json: String,
            in_scope_of_json: String,
            finished: bool,
            state: String,
            metadata_json: String,
            created_at: String,
            updated_at: String,
        }
        #[derive(Serialize)]
        #[serde(tag = "type", content = "payload")]
        enum DnaProjectionSignal {
            ReaCommitmentCommitted {
                action_hash: ActionHash,
                entry_hash: EntryHash,
                commitment: DnaCommitment,
                author: AgentPubKey,
            },
        }

        let action_hash = ActionHash::from_raw_36(vec![0x22; 36]);
        let entry_hash = EntryHash::from_raw_36(vec![0x33; 36]);
        let author = AgentPubKey::from_raw_36(vec![0x11; 36]);
        let dna_signal = DnaProjectionSignal::ReaCommitmentCommitted {
            action_hash: action_hash.clone(),
            entry_hash,
            commitment: DnaCommitment {
                id: "doorway:test|epr:test-app".into(),
                action: "project-epr".into(),
                provider: "doorway:test".into(),
                receiver: "epr:test-app".into(),
                resource_classified_as_json: "[]".into(),
                in_scope_of_json: "[\"doorway:test|epr:test-app\"]".into(),
                finished: false,
                state: "proposed".into(),
                metadata_json: "{}".into(),
                created_at: "2026-05-26T12:00:00Z".into(),
                updated_at: "2026-05-26T12:00:00Z".into(),
            },
            author,
        };

        // emit_signal encodes via ExternIO == rmp_serde::to_vec_named.
        let wire = rmp_serde::to_vec_named(&dna_signal).expect("encode DNA REA signal");

        let decoded = decode_rea_projection_signal(&wire)
            .expect("conductor msgpack wire format must decode into ReaProjectionSignal");
        match decoded {
            ReaProjectionSignal::ReaCommitmentCommitted {
                action_hash: got_action,
                commitment,
                ..
            } => {
                // Normalized form must match holochain's canonical base64
                // ("u" + base64url-no-pad over the raw 39 bytes).
                assert_eq!(got_action.0, format!("{action_hash}"));
                assert_eq!(commitment.id, "doorway:test|epr:test-app");
                assert_eq!(commitment.action, "project-epr");
                assert!(!commitment.finished);
            }
            other => panic!("unexpected variant: {other:?}"),
        }
        // (The miss counter is process-global; a successful decode not counting
        // is asserted indirectly via the dedicated counted/quiet tests, which
        // are the only place the counter value is load-bearing.)
    }

    /// Foreign signals (infra, mishpat, …) share the app interface — they must
    /// classify quiet, not as REA misses.
    #[test]
    fn rea_foreign_signal_classifies_quiet() {
        use serde::Serialize;
        #[derive(Serialize)]
        #[serde(tag = "type", content = "payload")]
        enum OtherSignal {
            PeerStatusRecorded { status: String },
        }
        let wire = rmp_serde::to_vec_named(&OtherSignal::PeerStatusRecorded {
            status: "online".into(),
        })
        .expect("encode");
        match decode_rea_projection_signal(&wire) {
            Err(SignalDecodeMiss::ForeignSignal { type_tag }) => {
                assert_eq!(type_tag, "PeerStatusRecorded");
            }
            other => panic!("expected ForeignSignal, got {other:?}"),
        }
    }

    /// A REA mirror-variant tag whose payload no longer decodes is a REAL miss
    /// — it must be loud (counted).
    #[test]
    fn rea_shape_mismatch_on_mirror_variant_is_counted() {
        use serde::Serialize;
        #[derive(Serialize)]
        #[serde(tag = "type", content = "payload")]
        enum BrokenShape {
            ReaCommitmentCommitted { unexpected_only_field: bool },
        }
        let wire = rmp_serde::to_vec_named(&BrokenShape::ReaCommitmentCommitted {
            unexpected_only_field: true,
        })
        .expect("encode");
        // Strictly-increased, not exactly +1: the counter is process-global and
        // parallel tests may also increment it concurrently.
        let before = rea_decode_miss_count();
        match decode_rea_projection_signal(&wire) {
            Err(SignalDecodeMiss::ReaShapeMismatch { type_tag, .. }) => {
                assert_eq!(type_tag, "ReaCommitmentCommitted");
            }
            other => panic!("expected ReaShapeMismatch, got {other:?}"),
        }
        assert!(rea_decode_miss_count() > before);
    }

    /// Reference fixture: a JSON shape matching what the DNA's
    /// `ProjectionSignal::ReaCommitmentCommitted` emits over the wire
    /// (adjacent tagging — tag + payload). This test guards against
    /// silent breakage of the substrate-correct write path.
    #[test]
    fn decode_rea_commitment_signal_from_dna_wire_shape() {
        let wire = serde_json::json!({
            "type": "ReaCommitmentCommitted",
            "payload": {
                "action_hash": "uhCkk-abc123",
                "entry_hash": "uhCEk-def456",
                "commitment": {
                    "id": "doorway:test|epr:test-app",
                    "action": "project-epr",
                    "provider": "doorway:test-doorway",
                    "receiver": "epr:test-app",
                    "resource_conforms_to": null,
                    "resource_inventoried_as": null,
                    "resource_classified_as_json": "[]",
                    "resource_quantity_value": null,
                    "resource_quantity_unit": null,
                    "effort_quantity_value": null,
                    "effort_quantity_unit": null,
                    "has_point_in_time": null,
                    "has_beginning": null,
                    "has_end": null,
                    "due": null,
                    "clause_of": null,
                    "agreed_in": null,
                    "input_of": null,
                    "output_of": null,
                    "satisfies": null,
                    "in_scope_of_json": "[\"doorway:test-doorway|epr:test-app\"]",
                    "finished": false,
                    "state": "proposed",
                    "note": null,
                    "metadata_json": "{}",
                    "created_at": "2026-05-26T12:00:00Z",
                    "updated_at": "2026-05-26T12:00:00Z"
                },
                "author": "uhCAk-xyz789"
            }
        });

        let signal: ReaProjectionSignal = serde_json::from_value(wire)
            .expect("DNA wire shape must decode into ReaProjectionSignal");

        match signal {
            ReaProjectionSignal::ReaCommitmentCommitted {
                action_hash,
                commitment,
                ..
            } => {
                assert_eq!(action_hash.0, "uhCkk-abc123");
                assert_eq!(commitment.id, "doorway:test|epr:test-app");
                assert_eq!(commitment.action, "project-epr");
                assert_eq!(
                    commitment.in_scope_of_json.as_deref(),
                    Some("[\"doorway:test-doorway|epr:test-app\"]")
                );
                assert!(!commitment.finished);
            }
            other => panic!("unexpected variant: {other:?}"),
        }
    }

    #[test]
    fn decode_agreement_signal_from_dna_wire_shape() {
        let wire = serde_json::json!({
            "type": "AgreementCommitted",
            "payload": {
                "action_hash": "uhCkk-ag1",
                "entry_hash": "uhCEk-ag2",
                "agreement": {
                    "id": "agreement-001",
                    "name": "Test Agreement",
                    "note": null,
                    "created_at": "2026-05-26T12:00:00Z"
                },
                "author": "uhCAk-author"
            }
        });

        let signal: ReaProjectionSignal = serde_json::from_value(wire).unwrap();
        match signal {
            ReaProjectionSignal::AgreementCommitted {
                action_hash,
                agreement,
                ..
            } => {
                assert_eq!(action_hash.0, "uhCkk-ag1");
                assert_eq!(agreement.id, "agreement-001");
                assert_eq!(agreement.name.as_deref(), Some("Test Agreement"));
            }
            _ => panic!("expected AgreementCommitted"),
        }
    }

    #[test]
    fn decode_economic_event_signal_from_dna_wire_shape() {
        let wire = serde_json::json!({
            "type": "ReaEconomicEventCommitted",
            "payload": {
                "action_hash": "uhCkk-ee1",
                "entry_hash": "uhCEk-ee2",
                "event": {
                    "id": "event-001",
                    "action": "ack-projection",
                    "provider": "doorway:test",
                    "receiver": "epr:test",
                    "resource_conforms_to": null,
                    "resource_inventoried_as": null,
                    "to_resource_inventoried_as": null,
                    "resource_classified_as_json": "[\"doorway:test|epr:test\"]",
                    "resource_quantity_value": null,
                    "resource_quantity_unit": null,
                    "effort_quantity_value": null,
                    "effort_quantity_unit": null,
                    "has_point_in_time": "2026-05-26T12:00:00Z",
                    "has_duration": null,
                    "input_of": null,
                    "output_of": null,
                    "fulfills_json": "[]",
                    "realization_of": null,
                    "satisfies_json": "[]",
                    "in_scope_of_json": "[]",
                    "note": null,
                    "state": "settled",
                    "triggered_by": null,
                    "at_location": null,
                    "image": null,
                    "lamad_event_type": null,
                    "metadata_json": "{}",
                    "created_at": "2026-05-26T12:00:00Z"
                },
                "author": "uhCAk-author"
            }
        });

        let signal: ReaProjectionSignal = serde_json::from_value(wire).unwrap();
        match signal {
            ReaProjectionSignal::ReaEconomicEventCommitted { event, .. } => {
                assert_eq!(event.id, "event-001");
                assert_eq!(event.action, "ack-projection");
                assert_eq!(
                    event.resource_classified_as_json.as_deref(),
                    Some("[\"doorway:test|epr:test\"]")
                );
            }
            _ => panic!("expected ReaEconomicEventCommitted"),
        }
    }

    #[test]
    fn decode_content_signal_from_dna_wire_shape() {
        let wire = serde_json::json!({
            "type": "ContentCommitted",
            "payload": {
                "action_hash": "uhCkk-content-anchor",
                "entry_hash": "uhCEk-content-entry",
                "content": {
                    "id": "elohim-host-landing",
                    "content_type": "html5-app",
                    "title": "Elohim Host Landing",
                    "description": "The hosted landing page bundle.",
                    "summary": null,
                    "content": "",
                    "content_format": "html5-app",
                    "tags": ["landing", "spa"],
                    "source_path": null,
                    "related_node_ids": [],
                    "author_id": null,
                    "reach": "commons",
                    "trust_score": 1.0,
                    "estimated_minutes": null,
                    "thumbnail_url": null,
                    "metadata_json": "{}",
                    "created_at": "2026-05-26T12:00:00Z",
                    "updated_at": "2026-05-26T13:30:00Z",
                    "schema_version": 1,
                    "validation_status": "Valid",
                    "blob_cid": "sha256-deadbeefcafe1234",
                    "content_size_bytes": 4096,
                    "content_hash": "sha256-deadbeefcafe1234"
                },
                "author": "uhCAk-author"
            }
        });

        let signal: ReaProjectionSignal =
            serde_json::from_value(wire).expect("DNA wire shape for ContentCommitted must decode");

        match signal {
            ReaProjectionSignal::ContentCommitted {
                action_hash,
                content,
                ..
            } => {
                assert_eq!(action_hash.0, "uhCkk-content-anchor");
                assert_eq!(content.id, "elohim-host-landing");
                assert_eq!(content.blob_cid.as_deref(), Some("sha256-deadbeefcafe1234"));
                assert_eq!(content.content_size_bytes, Some(4096));
                assert_eq!(content.tags.len(), 2);
                assert!((content.trust_score - 1.0).abs() < f64::EPSILON);
            }
            _ => panic!("expected ContentCommitted variant"),
        }
    }

    /// HEAD-election decode (notary-authority Leg 2): the `ContentHeadDeclared`
    /// signal must decode from the REAL conductor wire — MessagePack (`ExternIO`),
    /// where the DNA's `head_action_hash: ActionHash` (and entry_hash/author)
    /// serialize as raw 39-BYTE ARRAYS, not base64 strings. The `HoloHashB64`
    /// mirror field normalizes those bytes to the canonical "u"+base64url form;
    /// a `String` field would silently drop every such signal (the d33b0e1f5
    /// dark class). This is the raw-bytes HoloHash encoding trick from the REA
    /// commitment msgpack test, applied to the new variant.
    #[test]
    fn content_head_declared_signal_decodes_from_conductor_msgpack_wire() {
        use holochain_types::prelude::{ActionHash, AgentPubKey, EntryHash};
        use serde::Serialize;

        // DNA-side shape: content_store `ProjectionSignal::ContentHeadDeclared`
        // with REAL holo_hash types (field-by-field mirror; the wire field name
        // is `head_action_hash`).
        #[derive(Serialize)]
        #[serde(tag = "type", content = "payload")]
        enum DnaProjectionSignal {
            ContentHeadDeclared {
                content_id: String,
                head_action_hash: ActionHash,
                entry_hash: EntryHash,
                author: AgentPubKey,
                canonical_declared_at: Option<i64>,
                canonical_earned: Option<bool>,
            },
        }

        let head_action_hash = ActionHash::from_raw_36(vec![0x44; 36]);
        let entry_hash = EntryHash::from_raw_36(vec![0x55; 36]);
        let author = AgentPubKey::from_raw_36(vec![0x66; 36]);
        let dna_signal = DnaProjectionSignal::ContentHeadDeclared {
            content_id: "elohim-host-landing".into(),
            head_action_hash: head_action_hash.clone(),
            entry_hash,
            author,
            canonical_declared_at: Some(1_700_000_000_000_001),
            canonical_earned: Some(false),
        };

        // emit_signal encodes via ExternIO == rmp_serde::to_vec_named.
        let wire =
            rmp_serde::to_vec_named(&dna_signal).expect("encode DNA ContentHeadDeclared signal");

        let decoded = decode_rea_projection_signal(&wire)
            .expect("conductor msgpack wire format must decode into ReaProjectionSignal");
        match decoded {
            ReaProjectionSignal::ContentHeadDeclared {
                content_id,
                head_action_hash: got_head,
                canonical_declared_at,
                canonical_earned,
                ..
            } => {
                assert_eq!(content_id, "elohim-host-landing");
                // Normalized form must match holochain's canonical base64.
                assert_eq!(got_head.0, format!("{head_action_hash}"));
                assert_eq!(canonical_declared_at, Some(1_700_000_000_000_001));
                assert_eq!(canonical_earned, Some(false));
            }
            other => panic!("unexpected variant: {other:?}"),
        }
    }

    /// JSON-shape reference fixture for `ContentHeadDeclared` (adjacent tagging),
    /// mirroring the ContentCommitted JSON test — guards the wire field names.
    #[test]
    fn decode_content_head_declared_signal_from_dna_wire_shape() {
        let wire = serde_json::json!({
            "type": "ContentHeadDeclared",
            "payload": {
                "content_id": "elohim-host-landing",
                "head_action_hash": "uhCkk-declared-head",
                "entry_hash": "uhCEk-content-entry",
                "author": "uhCAk-author"
            }
        });

        let signal: ReaProjectionSignal = serde_json::from_value(wire)
            .expect("DNA wire shape for ContentHeadDeclared must decode");
        match signal {
            ReaProjectionSignal::ContentHeadDeclared {
                content_id,
                head_action_hash,
                ..
            } => {
                assert_eq!(content_id, "elohim-host-landing");
                assert_eq!(head_action_hash.0, "uhCkk-declared-head");
            }
            _ => panic!("expected ContentHeadDeclared variant"),
        }
    }

    #[test]
    fn parse_json_strings_handles_empty_and_invalid() {
        assert_eq!(parse_json_strings(None), Vec::<String>::new());
        assert_eq!(parse_json_strings(Some("")), Vec::<String>::new());
        assert_eq!(parse_json_strings(Some("[]")), Vec::<String>::new());
        assert_eq!(parse_json_strings(Some("not json")), Vec::<String>::new());
        assert_eq!(
            parse_json_strings(Some("[\"a\", \"b\"]")),
            vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn first_or_none_picks_first_non_empty() {
        assert_eq!(first_or_none(vec![]), None);
        assert_eq!(first_or_none(vec!["".to_string()]), None);
        assert_eq!(first_or_none(vec!["x".to_string()]), Some("x".to_string()));
        assert_eq!(
            first_or_none(vec!["a".to_string(), "b".to_string()]),
            Some("a".to_string())
        );
    }

    /// Old internally-tagged shape (what storage USED to expect) must FAIL
    /// to decode — guards against accidental revert of the wire-shape fix.
    #[test]
    fn old_internal_tagging_shape_fails_to_decode() {
        let stale_wire = serde_json::json!({
            "type": "ReaCommitmentCommitted",
            "action_hash": "uhCkk-abc",
            "commitment": {
                "id": "x",
                "action": "y",
                "provider": "p",
                "receiver": "r"
            }
        });
        let result: Result<ReaProjectionSignal, _> = serde_json::from_value(stale_wire);
        assert!(
            result.is_err(),
            "internally-tagged wire shape must NOT decode (would mean storage drifted away from DNA again)"
        );
    }

    /// The sink decoupling (T11): once a subscriber installs the channel, a
    /// recorded graduation arrives on it with the right fields — without any
    /// conductor in the loop. This is the seam the graduation block uses on a
    /// real proposed→active flip; the subscriber's drain task authors the link.
    #[tokio::test]
    async fn install_sink_receives_pending_state_link() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        install_state_link_sink(tx);

        // Direct recorder call (the graduation block calls this on a real flip).
        record_pending_state_link(
            "anchor:commit-1",
            "active",
            "uhCkk-event-1",
            "2026-06-11T10:00:00Z",
        );

        let got = rx.recv().await.expect("pending link must arrive");
        assert_eq!(got.commitment_cid, "anchor:commit-1");
        assert_eq!(got.state, "active");
        assert_eq!(got.event_hash, "uhCkk-event-1");
        assert_eq!(got.signed_at, "2026-06-11T10:00:00Z");
    }

    /// No-op without a sink: `record_pending_state_link` must never panic when
    /// no subscriber has installed a channel (unit-test / conductor-less mode).
    /// The SQL cache flip stands alone; the link is the durable upgrade. This
    /// also documents that the `OnceLock` means `install_sink_receives_pending_state_link`
    /// is the ONLY test that may install a sink (a second install is a no-op).
    #[test]
    fn record_without_sink_is_noop() {
        // If install_sink_receives_pending_state_link already ran and set the
        // OnceLock, this still must not panic (the send is best-effort).
        record_pending_state_link("anchor:x", "active", "uhCkk-y", "2026-06-11T00:00:00Z");
    }

    /// End-to-end: `substrate_signal` reaches the SQL `economic_events` row
    /// via the PRODUCTION DHT-projection path (`ReaEconomicEventCommitted` →
    /// `handle_rea_signal` → `upsert_with_anchor`), not the fixture projector.
    ///
    /// Regression gate: a future read-side aggregate must
    /// `COALESCE(substrate_signal,'attention')` so pre-migration NULL rows
    /// bucket correctly under `GROUP BY` (planning note — not implemented here).
    #[test]
    fn rea_committed_event_projects_substrate_signal_to_sql() {
        use diesel::prelude::*;

        use crate::db::context::AppContext;
        use crate::db::diesel_schema::economic_events;
        use crate::db::{init_pool, run_migrations};

        // ---- set up an in-memory pool with all migrations applied ----
        let pool = init_pool(":memory:").expect("in-memory pool");
        run_migrations(&pool).expect("migrations");

        let ctx = AppContext::new("test-app");

        // ---- build the signal exactly as the DNA post-commit emits it ----
        let event = EconomicEventEntry {
            id: "event-substrate-e2e".to_string(),
            action: "use".to_string(), // non-ack-projection ⇒ no side-projection
            provider: "agent:test-provider".to_string(),
            receiver: "agent:test-receiver".to_string(),
            resource_conforms_to: None,
            resource_inventoried_as: None,
            to_resource_inventoried_as: None,
            resource_classified_as_json: None,
            resource_quantity_value: None,
            resource_quantity_unit: None,
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_point_in_time: Some("2026-06-09T10:00:00Z".to_string()),
            has_duration: None,
            input_of: None,
            output_of: None,
            fulfills_json: None,
            realization_of: None,
            satisfies_json: None,
            in_scope_of_json: None,
            note: None,
            state: None,
            triggered_by: None,
            at_location: None,
            image: None,
            lamad_event_type: None,
            metadata_json: None,
            created_at: None,
            substrate_signal: Some("attention".to_string()),
        };

        let signal = ReaProjectionSignal::ReaEconomicEventCommitted {
            action_hash: "uhCkk-e2e-anchor".into(),
            entry_hash: Some("uhCEk-e2e-entry".into()),
            event,
            author: Some("uhCAk-e2e-author".into()),
        };

        // ---- drive the REAL production handler ----
        handle_rea_signal(signal, &pool, &ctx).expect("handle_rea_signal must succeed");

        // ---- assert the projected row carries substrate_signal ----
        let mut conn = pool.get().expect("pool connection");
        let got: Option<String> = economic_events::table
            .select(economic_events::substrate_signal)
            .first(&mut conn)
            .expect("row must exist after projection");

        assert_eq!(
            got.as_deref(),
            Some("attention"),
            "substrate_signal must reach SQL on the PRODUCTION DHT path \
             (ReaEconomicEventCommitted -> upsert_with_anchor), not just the fixture projector"
        );
    }
}

// ============================================================================
// Custody standing on the wire (2026-09-12) — blob-durability DELTA 12c cause 1
// ============================================================================

/// The commitment LIFECYCLE STATE must survive the peer boundary.
///
/// Before this, every insert path hardcoded `"proposed"` and
/// `CommitmentWireFields` carried no state at all, so a custody-blob commitment
/// reached each peer's projection and then FROZE at the birth state while the
/// author graduated it to `active`. The rows agreed, the anchors agreed, the
/// reconcile arm reported convergence — and a household's custody read as
/// merely *proposed* on every non-authoring peer.
#[cfg(test)]
mod custody_state_on_the_wire_tests {
    use super::*;
    use crate::db::context::AppContext;
    use crate::db::rea_commitments;
    use crate::db::{init_pool, run_migrations};

    /// Minimal wire fields for a custody-blob commitment, with `state` supplied
    /// exactly as the caller under test would.
    fn wire(id: &str, state: Option<&str>) -> CreateReaCommitmentInput {
        project_commitment_from_wire(&CommitmentWireFields {
            id,
            action: "custody-blob",
            provider: "agent:matthew",
            receiver: "agent:jessica",
            resource_conforms_to: None,
            resource_classified_as_json: None,
            resource_quantity_value: None,
            resource_quantity_unit: None,
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_beginning: None,
            has_end: None,
            due: None,
            clause_of: None,
            in_scope_of_json: None,
            note: None,
            metadata_json: None,
            state,
        })
    }

    /// THE cross-peer simulation. Peer B projected the create and holds
    /// `proposed`; peer A (the author) has since graduated the same commitment
    /// to `active`. B's reconcile re-reads the entry from its OWN conductor and
    /// re-projects it — after which BOTH peers read `active`.
    #[test]
    fn a_reconciled_peer_converges_onto_the_authors_active_standing() {
        let pool = init_pool(":memory:").expect("in-memory pool");
        run_migrations(&pool).expect("migrations");
        let ctx = AppContext::new("test-app");
        let mut conn = pool.get().expect("conn");

        // --- peer B: the create signal landed; standing is the birth state.
        rea_commitments::upsert_with_anchor(
            &mut conn,
            &ctx,
            wire("custody:blob-1", Some("proposed")),
            Some("uhCkkCREATE"),
        )
        .expect("project create");
        let before = rea_commitments::get_commitment(&mut conn, &ctx, "custody:blob-1")
            .expect("read")
            .expect("row");
        assert_eq!(
            before.state, "proposed",
            "a freshly-projected create is proposed on every peer"
        );

        // --- peer A graduated it. B re-reads the SAME entry from its own
        // conductor (the reconcile heal) and re-projects.
        rea_commitments::upsert_with_anchor(
            &mut conn,
            &ctx,
            wire("custody:blob-1", Some("active")),
            Some("uhCkkGRADUATED"),
        )
        .expect("reconcile heal");

        let after = rea_commitments::get_commitment(&mut conn, &ctx, "custody:blob-1")
            .expect("read")
            .expect("row");
        assert_eq!(
            after.state, "active",
            "custody STANDING must travel with the row — a reconciled peer that keeps \
             reading `proposed` while the author holds `active` is the 2026-09-12c red"
        );
        assert_eq!(
            after.dht_anchor_hash.as_deref(),
            Some("uhCkkGRADUATED"),
            "the anchor advances alongside the standing"
        );
    }

    /// The other half of the contract: `"proposed"` is the fallback for a
    /// GENUINE omission only. A pre-field peer (or any wire that simply carries
    /// no state) must still land the REA birth state — never an empty string,
    /// and never a state invented from somewhere else.
    #[test]
    fn a_wire_without_state_still_lands_proposed() {
        let pool = init_pool(":memory:").expect("in-memory pool");
        run_migrations(&pool).expect("migrations");
        let ctx = AppContext::new("test-app");
        let mut conn = pool.get().expect("conn");

        assert_eq!(
            wire("custody:blob-2", None).state,
            None,
            "an omitted state stays None through the shared mapping"
        );
        rea_commitments::upsert_with_anchor(
            &mut conn,
            &ctx,
            wire("custody:blob-2", None),
            Some("uhCkkCREATE2"),
        )
        .expect("project");
        assert_eq!(
            rea_commitments::get_commitment(&mut conn, &ctx, "custody:blob-2")
                .expect("read")
                .expect("row")
                .state,
            "proposed"
        );

        // An EMPTY string is what both wire mirrors produce for an absent field
        // (`#[serde(default)]` on a `String`); it is an omission, not a state.
        assert_eq!(wire("custody:blob-3", Some("")).state, None);
        assert_eq!(wire("custody:blob-4", Some("   ")).state, None);
    }

    /// A state-less projection input must never CLOBBER an existing standing.
    /// The anchor-only refresh path (`update_state_via_conductor`, a seeder
    /// reseed) carries no state, and a heal that reset `active` back to the
    /// birth default would be worse than the freeze it cures.
    #[test]
    fn an_anchor_only_refresh_never_resets_an_existing_standing() {
        let pool = init_pool(":memory:").expect("in-memory pool");
        run_migrations(&pool).expect("migrations");
        let ctx = AppContext::new("test-app");
        let mut conn = pool.get().expect("conn");

        rea_commitments::upsert_with_anchor(
            &mut conn,
            &ctx,
            wire("custody:blob-5", Some("active")),
            Some("uhCkkA"),
        )
        .expect("project active");
        rea_commitments::upsert_with_anchor(
            &mut conn,
            &ctx,
            wire("custody:blob-5", None),
            Some("uhCkkB"),
        )
        .expect("anchor-only refresh");

        let row = rea_commitments::get_commitment(&mut conn, &ctx, "custody:blob-5")
            .expect("read")
            .expect("row");
        assert_eq!(
            row.state, "active",
            "heal FILLS standing, it never resets it"
        );
        assert_eq!(row.dht_anchor_hash.as_deref(), Some("uhCkkB"));
    }

    /// The projection inventory a peer advertises must carry the state, or the
    /// reconcile arm has nothing to compare and the freeze stays invisible.
    #[test]
    fn the_advertised_inventory_carries_the_standing() {
        let pool = init_pool(":memory:").expect("in-memory pool");
        run_migrations(&pool).expect("migrations");
        let ctx = AppContext::new("test-app");
        let mut conn = pool.get().expect("conn");

        rea_commitments::upsert_with_anchor(
            &mut conn,
            &ctx,
            wire("custody:blob-6", Some("active")),
            Some("uhCkkA"),
        )
        .expect("project");

        let (entries, total) =
            rea_commitments::inventory_for_reconcile(&mut conn, &ctx, 0, i64::MAX).expect("inv");
        assert_eq!(total, 1);
        assert_eq!(
            entries[0],
            (
                "custody:blob-6".to_string(),
                "uhCkkA".to_string(),
                "active".to_string()
            ),
        );
    }
}
