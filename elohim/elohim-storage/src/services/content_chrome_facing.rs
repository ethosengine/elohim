//! Content-chrome facing loader — the impure half of the facing whose pure fold is
//! `elohim_facings::folds::content_chrome`.
//!
//! The chrome (the chip and its page-info cards) shows, for one content record:
//! its source and witness, what it may do (an html5-app's declared capabilities),
//! who holds it, who stewards it, the claims made about it, and its EPR legs. Each
//! reading is selected here from the projection that owns it, mapped onto the
//! fold's DB-free rows, and folded once:
//!
//! | Reading | Selected from |
//! |---|---|
//! | record, app | `content` (the Notarized record's projection) |
//! | witness | `content_witness::witness_for` — the conductor's receipts and held copy for the record's anchor, and on the authoring peer one other peer (derived per read, never written) |
//! | holders | `household_resilience::snapshot_with_staleness_secs` (the same snapshot the trust-mark dial reads) |
//! | claims | `attestations` by subject — the record id and its bundle CID, as the EPR page asks |
//! | legs | `epr_coupling` of the record's EPR atom (by CID, then `schema_key`) |
//! | steward | active `stewardship_allocations` ⋈ `contributor_presences` |
//!
//! The witness is the one reading that is not a projection: green once a
//! conductor other than the record's author holds the record as valid, amber
//! until then and whenever that cannot be read within the chip budget. The
//! anchor alone is not a witness — authoring stamps it at commit.
//!
//! **Source of truth:** none of its own. The facing is Ephemeral (Category C):
//! recomputed on every request from the Notarized record and the projections it
//! composes, and never stored — no table, no entry type, no DNA change. It is
//! served by `GET /api/v1/epr/{id}/chrome` only with the content chrome on
//! ([`content_chrome_enabled`]; declared then in
//! `build_manifest_with_content_chrome()` so a doorway's registry reaches it).
//! [`island_from_facing`] fills the context island of the storage peer's own
//! `/apps` shell; that shell is not wired yet (backlog
//! `content-chrome-element-and-app-port`), so today only its tests call it.

use std::sync::{Arc, OnceLock};

use diesel::prelude::*;
use elohim_chrome_asset::{
    ContentDelivery, ContentFrame, ContentIsland, DeliveryCheck, DeliveryHop, HolderReading,
    HopReporter, Isolation, TrustMode, WitnessState,
};
use elohim_facings::folds::content_chrome::{
    self, ClaimRow, ContentChromeRelation, HoldersRow, RecordRow, StewardRow, WitnessEvidence,
};
use elohim_facings::folds::epr_content::CouplingRow;

use crate::blob_store::BlobStore;
use crate::db::content_diesel::{self, MinTrust};
use crate::db::models::Content;
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::content_witness::{self, WitnessProbe};
use crate::views::ContentChromeFacingView;

/// The environment variable that switches the content chrome on.
pub const CONTENT_CHROME_ENV: &str = "ELOHIM_CONTENT_CHROME";

/// Whether the content chrome is on, from the value of [`CONTENT_CHROME_ENV`].
///
/// OFF unless the value is exactly one of `1`, `true`, `yes`, `on`
/// (case-insensitive): an absent, empty or unrecognised value is off, so a
/// typo never switches the chrome on. While off, `GET /api/v1/epr/{id}/chrome`
/// and `POST /api/v1/epr/{id}/signals` are not answered (the paths fall
/// through to the EPR routes exactly as before), `/manifest` declares neither,
/// and the witness reading registers no conductor or peers.
#[must_use]
pub fn content_chrome_enabled_from(value: Option<&str>) -> bool {
    matches!(
        value.map(|v| v.trim().to_ascii_lowercase()).as_deref(),
        Some("1" | "true" | "yes" | "on")
    )
}

/// [`content_chrome_enabled_from`] over the process environment. Read once at
/// startup, never on a request path.
#[must_use]
pub fn content_chrome_enabled() -> bool {
    content_chrome_enabled_from(std::env::var(CONTENT_CHROME_ENV).ok().as_deref())
}

/// Default liveness window for the holders reading (seconds), the same default
/// the `/api/v1/resilience/{id}/household` route applies.
pub const DEFAULT_PEER_STATUS_STALENESS_SECS: i64 = 900;

/// The holders reading's liveness window, read from `PEER_STATUS_STALENESS_SECS`
/// ONCE per process (never per request: an env read on the hot path is the
/// parallel-test flake class).
pub fn peer_status_staleness_secs() -> i64 {
    static WINDOW: OnceLock<i64> = OnceLock::new();
    *WINDOW.get_or_init(|| {
        std::env::var("PEER_STATUS_STALENESS_SECS")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(DEFAULT_PEER_STATUS_STALENESS_SECS)
    })
}

/// The canonical blob-store key (`sha256-{hex}`) of a content address in either
/// spelling, or `None` when `addr` is not an address.
pub fn canonical_address(addr: &str) -> Option<String> {
    BlobStore::parse_content_address(addr)
        .ok()
        .map(|hex| format!("sha256-{hex}"))
}

/// The spelling the chrome shows and keys grants on: CIDv1 (the
/// `X-Content-Address` spelling), from either stored spelling.
pub fn display_address(blob_hash: &str) -> Option<String> {
    let key = canonical_address(blob_hash)?;
    BlobStore::hash_to_cid(&key).ok().map(|cid| cid.to_string())
}

/// Resolve the record a chrome request names: by record id first, then — when
/// `id` is a content address — by the record whose bundle that address is. Both
/// paths apply the external serving floor ([`MinTrust::Amber`]), so a row the
/// node would not serve is not described either.
pub fn resolve_record(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<Option<Content>, StorageError> {
    if let Some(row) = content_diesel::get_content(conn, ctx, id, MinTrust::Amber)? {
        return Ok(Some(row));
    }
    let Some(key) = canonical_address(id) else {
        return Ok(None);
    };
    let mut spellings = vec![key.clone(), id.to_string()];
    if let Some(cid) = display_address(&key) {
        spellings.push(cid);
    }
    use crate::db::diesel_schema::content::dsl as c;
    let owner: Option<String> = c::content
        .filter(c::h_app_id.eq(&ctx.h_app_id))
        .filter(
            c::blob_hash
                .eq_any(&spellings)
                .or(c::blob_cid.eq_any(&spellings)),
        )
        .order(c::updated_at.desc())
        .select(c::id)
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Database(e.to_string()))?;
    match owner {
        Some(owner) => content_diesel::get_content(conn, ctx, &owner, MinTrust::Amber),
        None => Ok(None),
    }
}

/// The record's DB-free mirror.
pub fn record_row(record: &Content) -> RecordRow {
    RecordRow {
        id: record.id.clone(),
        title: record.title.clone(),
        content_format: record.content_format.clone(),
        reach: record.reach.clone(),
        content_address: record
            .blob_hash
            .as_deref()
            .or(record.blob_cid.as_deref())
            .and_then(display_address),
        content_body: record.content_body.clone(),
        dht_anchor_hash: record.dht_anchor_hash.clone(),
    }
}

/// Select the record's claims, legs and stewards (everything but the holders
/// reading, which reads through its own pooled snapshot). Each lens degrades on
/// its own: a failed read leaves that lens unselected (absent) with a warning,
/// never an empty list that would read as "none recorded".
pub fn load_content_chrome_relation(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    record: &Content,
) -> ContentChromeRelation {
    let row = record_row(record);
    let claims = load_claims(conn, record)
        .map_err(
            |e| tracing::warn!(record = %record.id, error = %e, "chrome facing: claims unread"),
        )
        .ok();
    let couplings = load_couplings(conn, &record.id)
        .map_err(|e| tracing::warn!(record = %record.id, error = %e, "chrome facing: legs unread"))
        .ok()
        .flatten();
    let stewards = load_stewards(conn, ctx, &record.id)
        .map_err(
            |e| tracing::warn!(record = %record.id, error = %e, "chrome facing: stewards unread"),
        )
        .ok();
    ContentChromeRelation {
        record: row,
        // Read by `build_facing` after the DB work, within the chip budget.
        witness: WitnessEvidence::Unread,
        holders: None,
        couplings,
        claims,
        stewards,
    }
}

/// Attestations whose subject is the record: asked by the record id and by its
/// bundle CID (no content-quality minter has fixed the subject key yet), the same
/// pair the EPR page asks. The fold de-duplicates by attestation id.
fn load_claims(
    conn: &mut SqliteConnection,
    record: &Content,
) -> Result<Vec<ClaimRow>, StorageError> {
    let mut subjects = vec![record.id.clone()];
    for addr in [record.blob_cid.as_deref(), record.blob_hash.as_deref()]
        .into_iter()
        .flatten()
    {
        if let Some(cid) = display_address(addr) {
            if !subjects.contains(&cid) {
                subjects.push(cid);
            }
        }
    }
    let mut rows = Vec::new();
    for subject in &subjects {
        for a in crate::db::attestations::list_by_subject(conn, subject, None)? {
            rows.push(ClaimRow {
                id: a.id,
                attestation_kind: a.attestation_kind,
                issuer_cid: a.issuer_cid,
                proof_class: a.proof_class,
                title: a.title,
                created_at: a.created_at,
                expires_at: a.expires_at,
                revoked_at: a.revoked_at,
            });
        }
    }
    Ok(rows)
}

/// The forward couplings of the record's EPR atom — `Ok(None)` when no atom
/// names the record (the legs lens is then not selected).
fn load_couplings(
    conn: &mut SqliteConnection,
    record_id: &str,
) -> Result<Option<Vec<CouplingRow>>, StorageError> {
    use crate::db::epr_atoms;
    let atom = match epr_atoms::fetch_atom_by_cid(conn, record_id)
        .map_err(|e| StorageError::Database(e.to_string()))?
    {
        Some(atom) => Some(atom),
        None => epr_atoms::fetch_atom_by_schema_key(conn, record_id)
            .map_err(|e| StorageError::Database(e.to_string()))?,
    };
    let Some(atom) = atom else {
        return Ok(None);
    };
    let rows = epr_atoms::fetch_coupling_for_atom(conn, &atom.cid)
        .map_err(|e| StorageError::Database(e.to_string()))?
        .into_iter()
        .map(|r| CouplingRow {
            epr_cid: r.epr_cid,
            leg: r.leg,
            target_cid: r.target_cid,
        })
        .collect();
    Ok(Some(rows))
}

/// Active stewardship allocations with the steward's presence name when held.
fn load_stewards(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    record_id: &str,
) -> Result<Vec<StewardRow>, StorageError> {
    let stewardship =
        crate::db::stewardship_allocations::get_content_stewardship(conn, ctx, record_id)?;
    Ok(stewardship
        .allocations
        .into_iter()
        .map(|a| StewardRow {
            presence_id: a.allocation.steward_presence_id,
            display_name: a.steward.map(|p| p.display_name),
            contribution_type: a.allocation.contribution_type,
            allocation_ratio: a.allocation.allocation_ratio,
            governance_state: a.allocation.governance_state,
        })
        .collect())
}

/// The holders reading: the resilience snapshot's inputs to `build_felt_status`,
/// stamped with the time this peer took it. `None` (lens not selected) when the
/// snapshot cannot be taken.
pub fn load_holders_row(
    pool: &DbPool,
    ctx: &AppContext,
    record_id: &str,
    staleness_secs: i64,
    observed_at: &str,
) -> Option<HoldersRow> {
    match crate::services::household_resilience::snapshot_with_staleness_secs(
        pool,
        ctx,
        record_id,
        None,
        staleness_secs,
    ) {
        Ok(snapshot) => {
            let felt = snapshot.felt_status?;
            Some(HoldersRow {
                distribution_state: snapshot.distribution_state,
                placement_gaps: snapshot.placement_gaps,
                held_by: felt.held_by,
                tier: felt.floor.tier,
                tier_declared: felt.floor.tier_declared,
                commitment_backed_collectives: snapshot.commitment_backed_collectives,
                observed_at: observed_at.to_string(),
            })
        }
        Err(e) => {
            tracing::warn!(record = %record_id, error = %e, "chrome facing: holders unread");
            None
        }
    }
}

/// Select and fold the facing for `id` (a record id or a content address).
/// `Ok(None)` when no servable record answers to `id`. `now` (RFC 3339) stamps
/// both `computedAt` and the holders reading's `asOf`. `witness` answers the
/// witness reading ([`content_witness::node_probe`] on a running node); it is
/// waited on no longer than [`content_witness::INLINE_BUDGET`].
pub async fn build_facing(
    pool: &DbPool,
    ctx: &AppContext,
    id: &str,
    staleness_secs: i64,
    now: &str,
    witness: Arc<dyn WitnessProbe>,
) -> Result<Option<(ContentChromeFacingView, Content)>, StorageError> {
    let mut conn = pool
        .get()
        .map_err(|e| StorageError::Internal(format!("pool: {e}")))?;
    let Some(record) = resolve_record(&mut conn, ctx, id)? else {
        return Ok(None);
    };
    let mut relation = load_content_chrome_relation(&mut conn, ctx, &record);
    // The snapshot takes its own pooled connection; release ours first.
    drop(conn);
    relation.witness = content_witness::witness_for(
        witness,
        &ctx.h_app_id,
        &record.id,
        record.dht_anchor_hash.as_deref(),
    )
    .await;
    relation.holders = load_holders_row(pool, ctx, &record.id, staleness_secs, now);
    Ok(Some((
        content_chrome::fold_content_chrome(&relation, now),
        record,
    )))
}

/// Fill the content island (chrome-context `content`) from the facing — the
/// readings come from the facing alone; the medium supplies only where the frame
/// loads from, how it is isolated, and the delivery it witnessed.
///
/// `None` when the facing has no app lens or no content address: only an
/// html5-app record with bytes is framed by a content shell.
pub fn island_from_facing(
    facing: &ContentChromeFacingView,
    frame: ContentFrame,
    isolation: Isolation,
    delivery_path: Vec<DeliveryHop>,
    checked: Option<DeliveryCheck>,
) -> Option<ContentIsland> {
    let app = facing.app.as_ref()?;
    let content_address = facing.content_address.clone()?;
    Some(ContentIsland {
        record_id: facing.record_id.clone(),
        content_address,
        format: facing.format.clone(),
        entry: app.entry.clone(),
        mode: TrustMode::Trustful,
        reach: Some(facing.reach.clone()),
        witness: if facing.witness == "green" {
            WitnessState::Green
        } else {
            WitnessState::Amber
        },
        capabilities: app.capabilities.clone(),
        holders: facing.holders.as_ref().map(|h| HolderReading {
            holders: h.holders,
            target: h.target,
            headline: Some(h.headline.clone()),
            as_of: Some(h.as_of.clone()),
        }),
        frame,
        isolation,
        delivery: ContentDelivery {
            path: delivery_path,
            checked,
            receipt: None,
        },
    })
}

/// The one hop a peer reports about itself when it serves its own copy.
pub fn self_hop() -> DeliveryHop {
    DeliveryHop::SelfPeer {
        reported_by: HopReporter::SelfPeer,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_chrome_is_off_unless_switched_on_exactly() {
        for off in [
            None,
            Some(""),
            Some("0"),
            Some("false"),
            Some("off"),
            Some("ture"),
        ] {
            assert!(!content_chrome_enabled_from(off), "{off:?} must read off");
        }
        for on in ["1", "true", "TRUE", " yes ", "On"] {
            assert!(content_chrome_enabled_from(Some(on)), "{on:?} must read on");
        }
    }
    use crate::db::models::AttestationRow;
    use crate::services::content_witness::WitnessFacts;
    use crate::test_util::test_pool;

    /// A conductor that answers the same facts for every anchor, and a peer
    /// that never holds anything.
    struct Facts(Option<WitnessFacts>);

    #[async_trait::async_trait]
    impl WitnessProbe for Facts {
        async fn facts(&self, _anchor: &str) -> Result<Option<WitnessFacts>, StorageError> {
            Ok(self.0.clone())
        }
        async fn local_agents(&self) -> Option<std::collections::HashSet<String>> {
            None
        }
        async fn ask_one_peer(&self, _content_id: &str, _anchor: &str) -> bool {
            false
        }
    }

    /// Authored by this node's own agent; `receipts` valid receipts back.
    fn authored_here(receipts: u32) -> Arc<dyn WitnessProbe> {
        Arc::new(Facts(Some(WitnessFacts {
            author: "uhCAkauthor".into(),
            authored_by_caller: true,
            held_valid: true,
            record_op_receipts: receipts,
            record_op_valid_receipts: receipts,
            valid_receipts_all_ops: receipts,
        })))
    }

    const HEX: &str = "a3f1c0b2d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f";

    fn insert_record(pool: &DbPool, id: &str, format: &str, anchor: Option<&str>) {
        let mut conn = pool.get().unwrap();
        let ctx = AppContext::default_lamad();
        let body = r#"{"slug":"evo","entryPoint":"index.html","capabilities":["forms","fullscreen"],"themeHint":{"colorScheme":"dark"}}"#;
        content_diesel::create_content(
            &mut conn,
            &ctx,
            content_diesel::CreateContentInput {
                id: id.to_string(),
                title: "The Evolution of Trust".to_string(),
                description: None,
                content_type: "simulation".to_string(),
                content_format: format.to_string(),
                blob_hash: Some(format!("sha256-{HEX}")),
                blob_cid: None,
                content_size_bytes: None,
                metadata_json: None,
                reach: "commons".to_string(),
                created_by: None,
                tags: vec![],
                content_body: Some(body.to_string()),
                dht_anchor_hash: anchor.map(str::to_string),
            },
        )
        .unwrap();
        // Servable without a witness: the libp2p publish marker is the amber floor.
        use crate::db::diesel_schema::content::dsl as c;
        diesel::update(c::content.filter(c::id.eq(id)))
            .set(c::p2p_published_at.eq(Some("2026-10-15T00:00:00Z")))
            .execute(&mut conn)
            .unwrap();
    }

    #[test]
    fn display_address_renders_cidv1_from_either_spelling() {
        let cid = display_address(&format!("sha256-{HEX}")).unwrap();
        assert!(cid.starts_with("bafkrei"), "{cid}");
        assert_eq!(display_address(&cid).as_deref(), Some(cid.as_str()));
        assert!(display_address("not-an-address").is_none());
    }

    #[tokio::test]
    async fn facing_composes_record_claims_and_stewards_and_omits_unselected_legs() {
        let pool = test_pool();
        insert_record(
            &pool,
            "evolution-of-trust",
            "html5-app",
            Some("uhCkkanchor"),
        );
        {
            let mut conn = pool.get().unwrap();
            crate::db::attestations::upsert(
                &mut conn,
                &AttestationRow {
                    id: "att-1".into(),
                    dht_anchor_hash: vec![1, 2, 3],
                    attestation_kind: "attestation:content-quality".into(),
                    subject_cid: "evolution-of-trust".into(),
                    subject_kind: "content".into(),
                    issuer_cid: "uhCAkreviewer".into(),
                    parent_governance_action_cid: None,
                    vote_value: None,
                    vote_weight: None,
                    proof_class: "witness".into(),
                    proof_evidence_json: "{}".into(),
                    evidence_json: "{}".into(),
                    expires_at: None,
                    supersedes_cid: None,
                    revocation_reason: None,
                    revoked_at: None,
                    created_at: "2026-10-01T00:00:00Z".into(),
                    manifest_ref: "lamad".into(),
                    title: "Reviewed for accuracy".into(),
                    description: None,
                },
            )
            .unwrap();
        }
        let ctx = AppContext::default_lamad();
        // Anchored but not yet witnessed: authored here, no receipt, no peer
        // holds it — amber.
        let (unwitnessed, _) =
            build_facing(&pool, &ctx, "evolution-of-trust", 0, "t", authored_here(0))
                .await
                .unwrap()
                .unwrap();
        assert_eq!(
            unwitnessed.witness, "amber",
            "an anchor alone is no witness"
        );

        let (view, record) = build_facing(
            &pool,
            &ctx,
            "evolution-of-trust",
            0,
            "2026-10-15T12:00:00Z",
            authored_here(2),
        )
        .await
        .unwrap()
        .expect("record resolves");
        assert_eq!(record.id, "evolution-of-trust");
        assert_eq!(
            view.witness, "green",
            "a remote validator's receipt witnesses it"
        );
        assert_eq!(view.mode, "trustful");
        let app = view.app.as_ref().expect("html5-app lens");
        assert_eq!(app.capabilities, vec!["forms", "fullscreen"]);
        let claims = view.claims.as_ref().expect("claims always selected");
        assert_eq!(claims.attestations.len(), 1);
        assert_eq!(claims.attestations[0].issuer_cid, "uhCAkreviewer");
        assert!(view.steward.as_ref().unwrap().stewards.is_empty());
        assert!(view.legs.is_none(), "no EPR atom → legs not selected");
        let holders = view
            .holders
            .as_ref()
            .expect("snapshot answers (unmeasured)");
        assert_eq!(holders.reassurance, "not-yet-seen");
        assert_eq!(holders.as_of, "2026-10-15T12:00:00Z");

        // Addressed by its bundle instead of its id: the same record answers.
        let cid = view.content_address.clone().unwrap();
        let (by_address, _) = build_facing(&pool, &ctx, &cid, 0, "t", authored_here(0))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(by_address.record_id, "evolution-of-trust");

        assert!(
            build_facing(&pool, &ctx, "no-such-record", 0, "t", authored_here(0))
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn island_carries_facing_readings_and_only_reported_checks() {
        let pool = test_pool();
        insert_record(&pool, "evolution-of-trust", "html5-app", None);
        let ctx = AppContext::default_lamad();
        // No anchor: amber even with receipts in hand.
        let (view, _) = build_facing(&pool, &ctx, "evolution-of-trust", 0, "t", authored_here(2))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(view.witness, "amber");
        let island = island_from_facing(
            &view,
            ContentFrame {
                src: "http://127.0.0.1:8090/apps/evolution-of-trust/index.html".into(),
                title: view.title.clone(),
            },
            Isolation {
                boundary: elohim_chrome_asset::IsolationBoundary::OriginPair,
                content_origin: Some("http://127.0.0.1:8090".into()),
            },
            vec![self_hop()],
            None,
        )
        .expect("html5-app with bytes is framed");
        let wire = serde_json::to_value(&island).unwrap();
        assert_eq!(wire["witness"], "amber");
        assert_eq!(
            wire["capabilities"],
            serde_json::json!(["forms", "fullscreen"])
        );
        assert_eq!(wire["delivery"]["path"][0]["role"], "self");
        assert_eq!(wire["delivery"]["path"][0]["reportedBy"], "self");
        assert!(wire["delivery"].get("checked").is_none());
        assert_eq!(wire["isolation"]["boundary"], "origin-pair");

        // A record that is not an html5-app is never framed by a content shell.
        insert_record(&pool, "a-page", "markdown", None);
        let (page, _) = build_facing(&pool, &ctx, "a-page", 0, "t", authored_here(0))
            .await
            .unwrap()
            .unwrap();
        assert!(island_from_facing(
            &page,
            ContentFrame {
                src: String::new(),
                title: String::new()
            },
            Isolation {
                boundary: elohim_chrome_asset::IsolationBoundary::OpaqueSandbox,
                content_origin: None
            },
            vec![self_hop()],
            None
        )
        .is_none());
    }
}
