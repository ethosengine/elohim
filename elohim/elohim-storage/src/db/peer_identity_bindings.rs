//! CRUD for the `peer_identity_bindings` table.
//!
//! Source of truth: Holochain DHT (imagodei AgentPeerBinding entry — Task A.2).
//! This table is a Category C operational projection rebuildable from signal replay.
//!
//! ## Primary operations
//!
//! - `upsert` — called by the ReconcileController when an AgentPeerBinding signal
//!   arrives (Task A.4 controller, Task A.8 signal handler).
//! - `lookup_active` — called by `HolochainBackedPeerIdentityMap::lookup` on the
//!   P2P hot path and by `services::hub_resolver::resolve_peer_dwelling_hub`;
//!   both callers receive the verified-first active binding for a peer.

use crate::db::diesel_schema::peer_identity_bindings;
use crate::db::models::{NewPeerIdentityBindingRow, PeerIdentityBindingRow};
use crate::error::StorageError;
use diesel::prelude::*;

/// Upsert a peer identity binding row.
///
/// Uses INSERT OR REPLACE semantics keyed on `(peer_id, dht_anchor_hash)`.
/// If the same DHT entry is observed again (e.g. from a second source), the
/// row is replaced with the latest `observed_at` and `source`.
///
/// ## Authority on `superseded_by`
///
/// This function unconditionally writes whatever `superseded_by` value is on
/// the incoming row. Only the DHT-arrival path (the imagodei
/// `AgentPeerBindingCreated` signal) is authoritative on supersession state,
/// so only the controller's projection writer should call this function
/// directly. Non-authoritative writers (handshake, gossip) MUST instead call
/// [`upsert_preserving_supersession`], which preserves any prior
/// `superseded_by` value when the incoming row carries `None`.
pub fn upsert(
    conn: &mut SqliteConnection,
    row: &NewPeerIdentityBindingRow,
) -> Result<(), StorageError> {
    diesel::replace_into(peer_identity_bindings::table)
        .values(row)
        .execute(conn)
        .map(|_| ())
        .map_err(|e| StorageError::Database(format!("peer_identity_bindings upsert: {e}")))
}

/// Upsert variant for non-authoritative sources (handshake, gossip) that
/// must not overwrite an existing `superseded_by` value.
///
/// The DHT-arrival path is the only authoritative writer of `superseded_by`;
/// non-authoritative re-arrivals of the same `(peer_id, dht_anchor_hash)`
/// row would otherwise erase that field via INSERT OR REPLACE semantics.
/// This function reads the existing row's `superseded_by` first and folds it
/// into the incoming row before delegating to [`upsert`].
///
/// Behavior:
/// - If a row already exists for `(peer_id, dht_anchor_hash)` with
///   `superseded_by = Some(_)` and the incoming row has `superseded_by =
///   None`, the prior value is preserved.
/// - In every other case (no prior row; prior row has `None`; incoming row
///   already carries `Some(_)`) the function behaves like plain `upsert`.
pub fn upsert_preserving_supersession(
    conn: &mut SqliteConnection,
    row: &NewPeerIdentityBindingRow,
) -> Result<(), StorageError> {
    use peer_identity_bindings::dsl;

    let existing_superseded_by: Option<Option<String>> = dsl::peer_identity_bindings
        .filter(dsl::peer_id.eq(&row.peer_id))
        .filter(dsl::dht_anchor_hash.eq(&row.dht_anchor_hash))
        .select(dsl::superseded_by)
        .first::<Option<String>>(conn)
        .optional()
        .map_err(|e| {
            StorageError::Database(format!("peer_identity_bindings preserve-lookup: {e}"))
        })?;

    // If the existing row already has a non-null superseded_by AND the
    // incoming row's value is None, fold the prior value forward.
    let merged = match existing_superseded_by {
        Some(Some(prior)) if row.superseded_by.is_none() => {
            let mut new_row = row.clone();
            new_row.superseded_by = Some(prior);
            new_row
        }
        _ => row.clone(),
    };
    upsert(conn, &merged)
}

/// Look up the most recently observed, currently-active binding for `peer_id`.
///
/// "Active" means: `valid_until IS NULL OR valid_until > now_iso`, AND
/// `superseded_by IS NULL`. The query returns at most one row — the one with
/// a verified binding first, then the latest `observed_at`. The verified-first
/// preference is deterministic and positive-matches `cross_signed`; an
/// unverified binding remains usable for routing only when no verified binding
/// is active. Both `p2p::identity_map::HolochainBackedPeerIdentityMap::lookup`
/// and `services::hub_resolver::resolve_peer_dwelling_hub` use this same
/// verified-first ordering. The partial index
/// `idx_peer_identity_bindings_current_per_agent` (keyed on `agent_cid`) covers
/// supersession-aware filtering for agent-side queries; the per-peer lookup
/// here uses the existing `idx_peer_identity_bindings_peer_id` index.
///
/// Returns `None` if no active binding exists for the peer.
pub fn lookup_active(
    conn: &mut SqliteConnection,
    peer_id: &str,
    now_iso: &str,
) -> Result<Option<PeerIdentityBindingRow>, StorageError> {
    use peer_identity_bindings::dsl;

    let verified = dsl::peer_identity_bindings
        .filter(dsl::peer_id.eq(peer_id))
        .filter(dsl::valid_until.is_null().or(dsl::valid_until.gt(now_iso)))
        .filter(dsl::superseded_by.is_null())
        .filter(dsl::proof_status.eq(crate::p2p::binding_proof_wire::PROOF_STATUS_CROSS_SIGNED))
        .order((dsl::observed_at.desc(), dsl::dht_anchor_hash.asc()))
        .first::<PeerIdentityBindingRow>(conn)
        .optional()
        .map_err(|e| StorageError::Database(format!("peer_identity_bindings lookup: {e}")))?;
    if verified.is_some() {
        return Ok(verified);
    }
    dsl::peer_identity_bindings
        .filter(dsl::peer_id.eq(peer_id))
        .filter(dsl::valid_until.is_null().or(dsl::valid_until.gt(now_iso)))
        .filter(dsl::superseded_by.is_null())
        .order((dsl::observed_at.desc(), dsl::dht_anchor_hash.asc()))
        .first::<PeerIdentityBindingRow>(conn)
        .optional()
        .map_err(|e| StorageError::Database(format!("peer_identity_bindings lookup: {e}")))
}

/// Delete all bindings for a peer (used in tests and reconciliation resets).
#[cfg(test)]
pub fn delete_all_for_peer(
    conn: &mut SqliteConnection,
    peer_id: &str,
) -> Result<usize, StorageError> {
    use peer_identity_bindings::dsl;

    diesel::delete(dsl::peer_identity_bindings.filter(dsl::peer_id.eq(peer_id)))
        .execute(conn)
        .map_err(|e| StorageError::Database(format!("peer_identity_bindings delete: {e}")))
}

/// List every currently-active binding for `agent_cid`.
///
/// "Active" means: `valid_until IS NULL OR valid_until > now_iso`, AND
/// `superseded_by IS NULL`. Each row represents one of the agent's known
/// peer-stewarded devices. Order is by `observed_at DESC` so the most
/// recently observed binding for a (peer_id, agent_cid) pair appears first.
///
/// This is the input to view-federation (Phase 4 view services + F-T21
/// `Federator::query`): one outbound view-federation request per row.
///
/// ## This is the ROUTING cut — it serves self-asserted bindings
///
/// Rows here may carry `proof_status = 'unverified'`, and today essentially all
/// of them do. That is safe for *selection* — which peer to dial, which device
/// to display — because content addressing bounds the worst case: a spoofed
/// binding costs a wasted dial, and the bytes still verify by hash. It is NOT
/// safe for deciding **whom to credit**. An economic join must call
/// [`list_attributable_for_agent`] instead, which applies the cross-signature
/// cut (habit `identity-cross-signed`).
///
/// Returns an empty `Vec` if the agent has no active bindings.
pub fn list_active_for_agent(
    conn: &mut SqliteConnection,
    agent_cid: &str,
    now_iso: &str,
) -> Result<Vec<PeerIdentityBindingRow>, StorageError> {
    use peer_identity_bindings::dsl;

    dsl::peer_identity_bindings
        .filter(dsl::agent_cid.eq(agent_cid))
        .filter(dsl::valid_until.is_null().or(dsl::valid_until.gt(now_iso)))
        .filter(dsl::superseded_by.is_null())
        .order(dsl::observed_at.desc())
        .load::<PeerIdentityBindingRow>(conn)
        .map_err(|e| {
            StorageError::Database(format!("peer_identity_bindings list_active_for_agent: {e}"))
        })
}

/// Does this node already hold a CURRENT cross-signed binding for
/// `(agent_cid, peer_id)`?
///
/// The minting path's idempotency gate (C2-S2): a node mints its own binding
/// once and then leaves it alone until it nears expiry. "Current" means all of:
/// not superseded, `valid_until` beyond `min_valid_until_iso` (the caller passes
/// `now + re-mint lead time`, so a binding is renewed BEFORE it lapses rather
/// than after), and `proof_status = 'cross_signed'` read through
/// [`PeerIdentityBindingRow::is_cross_signed`] — never a string comparison, and
/// never a `!= 'unverified'` gate.
///
/// A row that exists but is only self-asserted answers `false`, which is the
/// point: the sentinel row a peer already gossiped about us must not suppress
/// the mint that replaces it.
pub fn has_current_cross_signed(
    conn: &mut SqliteConnection,
    agent_cid: &str,
    peer_id: &str,
    min_valid_until_iso: &str,
) -> Result<bool, StorageError> {
    use peer_identity_bindings::dsl;

    let rows = dsl::peer_identity_bindings
        .filter(dsl::agent_cid.eq(agent_cid))
        .filter(dsl::peer_id.eq(peer_id))
        .filter(dsl::superseded_by.is_null())
        .filter(dsl::valid_until.gt(min_valid_until_iso))
        .load::<PeerIdentityBindingRow>(conn)
        .map_err(|e| {
            StorageError::Database(format!(
                "peer_identity_bindings has_current_cross_signed: {e}"
            ))
        })?;

    Ok(rows.iter().any(|r| r.is_cross_signed()))
}

// ============================================================================
// The attribution cut (habit `identity-cross-signed`, C2-S5)
// ============================================================================

/// How this deployment treats bindings that are NOT cross-signed when an
/// economic attribution join asks for them.
///
/// The two postures exist because the cut and the minting path land in that
/// order: verification is buildable today, but nothing on the fleet emits a real
/// proof yet (C2-S2). Flipping straight to `Enforce` before minting exists would
/// empty every attribution surface on every peer at once — a self-inflicted
/// outage in the name of a property no peer can yet satisfy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributionPosture {
    /// Serve unverified bindings to attribution joins, but COUNT them
    /// (`elohim_attribution_unverified_bindings_total`). Default. Behaviour is
    /// byte-identical to before this slice; the counter is the measure that says
    /// how much attribution is riding self-asserted identity right now.
    Observe,
    /// Refuse them. Only `proof_status = 'cross_signed'` rows reach an
    /// attribution join; everything else is dropped, and the surface reads
    /// honestly empty rather than confidently wrong.
    Enforce,
}

/// Read the deployment's attribution posture from
/// `ELOHIM_ATTRIBUTION_CROSS_SIGNED` (`enforce` | `observe`).
///
/// Defaults to [`AttributionPosture::Observe`] — see the enum doc for why, and
/// `genesis/manifests/habits.yaml` (`identity-cross-signed`) for the flip
/// condition. Any unrecognised value reads as `observe`: an operator typo must
/// not silently blank the economic surfaces.
pub fn attribution_posture() -> AttributionPosture {
    match std::env::var("ELOHIM_ATTRIBUTION_CROSS_SIGNED")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "enforce" | "1" | "true" => AttributionPosture::Enforce,
        _ => AttributionPosture::Observe,
    }
}

/// A binding set that has passed the attribution cut.
///
/// The type is the point. Economic joins take `&AttributableBindings`, never
/// `&[PeerIdentityBindingRow]`, so "did anyone check these?" is answered by the
/// signature of the function rather than by remembering to add a `WHERE`. The
/// only constructor is [`list_attributable_for_agent`], which applies the cut.
///
/// It deliberately does NOT deref to the raw row slice: a caller that wants the
/// routing set (dial selection, device display) should say so by calling
/// [`list_active_for_agent`], which is honest about serving self-asserted rows.
#[derive(Debug, Clone)]
pub struct AttributableBindings {
    rows: Vec<PeerIdentityBindingRow>,
    posture: AttributionPosture,
    /// How many active bindings were self-asserted at read time — the ones
    /// `Enforce` drops and `Observe` lets through under protest.
    unverified_seen: usize,
    /// How many active bindings this join looked at, admitted or refused — the
    /// DENOMINATOR under `unverified_seen`. Without it a zero numerator is
    /// ambiguous between "every binding was proven" and "nobody looked", and the
    /// habit's flip-to-green measure cannot tell a clean fleet from a dead code
    /// path. Counted BEFORE the posture filter, so `Enforce` cannot shrink it.
    examined: usize,
}

impl AttributableBindings {
    /// The empty set — no binding may be credited.
    ///
    /// Safe to construct anywhere precisely because it can only ever *withhold*
    /// credit. Use it for the "this caller resolved no agent" branch.
    pub fn none() -> Self {
        Self {
            rows: Vec::new(),
            posture: attribution_posture(),
            unverified_seen: 0,
            examined: 0,
        }
    }

    /// Test-only: build a populated set without going through the cut, so unit
    /// tests can exercise the *arithmetic* of an attribution fold independently
    /// of the cut that feeds it. Not available outside the crate's own tests —
    /// production code has exactly one way in, [`list_attributable_for_agent`].
    #[cfg(test)]
    pub(crate) fn from_rows_for_test(rows: Vec<PeerIdentityBindingRow>) -> Self {
        Self {
            examined: rows.len(),
            rows,
            posture: AttributionPosture::Observe,
            unverified_seen: 0,
        }
    }

    /// The peer ids an economic join may credit.
    pub fn peer_ids(&self) -> Vec<&str> {
        self.rows.iter().map(|r| r.peer_id.as_str()).collect()
    }

    /// The admitted rows.
    pub fn rows(&self) -> &[PeerIdentityBindingRow] {
        &self.rows
    }

    /// No binding may be credited.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Posture this set was cut under.
    pub fn posture(&self) -> AttributionPosture {
        self.posture
    }

    /// Active-but-self-asserted bindings observed while building this set.
    /// Non-zero under `Observe` means attribution is riding unverified identity
    /// right now; non-zero under `Enforce` means it was refused.
    pub fn unverified_seen(&self) -> usize {
        self.unverified_seen
    }

    /// How many active bindings this join examined — the denominator that makes
    /// [`Self::unverified_seen`] readable.
    ///
    /// `unverified_seen() == 0 && examined() == 0` is SILENCE, not safety: the
    /// join reached no binding, so it proved nothing about identity. Only
    /// `unverified_seen() == 0 && examined() > 0` is the habit's green.
    pub fn examined(&self) -> usize {
        self.examined
    }
}

/// List the bindings for `agent_cid` that may back an **economic attribution**
/// join — crediting bytes, custody, or reciprocity to an identity.
///
/// This is the cut named by `elohim-storage/CLAUDE.md`: a binding may decide
/// *where to dial*, never *whom to credit*. Selection is safe on an unverified
/// binding because content addressing bounds the worst case (a spoofed dial
/// costs a wasted round trip, and the bytes still verify by hash); attribution
/// is not, because the error lands durably in a projection that assigns value
/// to the wrong agent.
///
/// Under [`AttributionPosture::Observe`] (default) the set is the same one
/// [`list_active_for_agent`] returns, with the self-asserted rows counted;
/// under `Enforce` only cross-signed rows survive.
pub fn list_attributable_for_agent(
    conn: &mut SqliteConnection,
    agent_cid: &str,
    now_iso: &str,
) -> Result<AttributableBindings, StorageError> {
    list_attributable_for_agent_with_posture(conn, agent_cid, now_iso, attribution_posture())
}

/// [`list_attributable_for_agent`] with the posture supplied rather than read
/// from the environment.
///
/// Exists so the cut's behaviour under `Enforce` is provable at the desk without
/// mutating process-global environment state from a parallel test — the posture
/// is an argument, so both branches are exercised deterministically.
pub fn list_attributable_for_agent_with_posture(
    conn: &mut SqliteConnection,
    agent_cid: &str,
    now_iso: &str,
    posture: AttributionPosture,
) -> Result<AttributableBindings, StorageError> {
    let active = list_active_for_agent(conn, agent_cid, now_iso)?;
    let unverified_seen = active.iter().filter(|r| !r.is_cross_signed()).count();
    let examined = active.len();

    let posture_label = match posture {
        AttributionPosture::Observe => "observe",
        AttributionPosture::Enforce => "enforce",
    };

    // The denominator, emitted UNCONDITIONALLY — including the join that reaches
    // no binding at all. `unverified_seen` fires only on the interesting path,
    // which is exactly why its zero could not be read: a flat zero is equally
    // consistent with "every binding examined was cross-signed" and "no join
    // reached a binding", and on alpha it has read zero across all 56 pod
    // instances for 7 days while the analysis in
    // `.claude/memory/project_attribution_cut_binding_proof_status.md` predicts
    // a NON-zero count (minting is libp2p-only; alpha runs dual). Both cannot
    // hold. Only the conjunction `examined > 0 && unverified == 0` is the
    // habit's green; `examined == 0` is silence, and must never read as safety.
    crate::metrics::ATTRIBUTION_JOINS
        .with_label_values(&[posture_label])
        .inc();
    crate::metrics::ATTRIBUTION_BINDINGS_EXAMINED
        .with_label_values(&[posture_label])
        .inc_by(examined as u64);

    if unverified_seen > 0 {
        crate::metrics::ATTRIBUTION_UNVERIFIED_BINDINGS
            .with_label_values(&[posture_label])
            .inc_by(unverified_seen as u64);
    }

    let rows = match posture {
        AttributionPosture::Observe => active,
        AttributionPosture::Enforce => active.into_iter().filter(|r| r.is_cross_signed()).collect(),
    };

    Ok(AttributableBindings {
        rows,
        posture,
        unverified_seen,
        examined,
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::NewPeerIdentityBindingRow;
    use crate::db::{run_migrations, DbPool};
    use diesel::r2d2::{ConnectionManager, Pool};

    fn test_pool() -> DbPool {
        let url = format!(
            "file:pib_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    fn binding(
        peer_id: &str,
        agent_cid: &str,
        dht_anchor_hash: &str,
        valid_from: &str,
        valid_until: Option<&str>,
        observed_at: &str,
        source: &str,
    ) -> NewPeerIdentityBindingRow {
        NewPeerIdentityBindingRow {
            peer_id: peer_id.to_string(),
            agent_cid: agent_cid.to_string(),
            dht_anchor_hash: dht_anchor_hash.to_string(),
            valid_from: valid_from.to_string(),
            valid_until: valid_until.map(|s| s.to_string()),
            observed_at: observed_at.to_string(),
            source: source.to_string(),
            device_archetype: "node".to_string(),
            superseded_by: None,
            signature: String::new(),
            proof_status: crate::p2p::binding_proof_wire::BindingProofStatus::unverified(),
        }
    }

    #[test]
    fn upsert_and_lookup_active_binding() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let row = binding(
            "12D3KooWXyz",
            "bafyreiabc",
            "uhCAkABCDEF",
            "2026-01-01T00:00:00Z",
            None,
            "2026-04-24T00:00:00Z",
            "dht",
        );
        upsert(&mut conn, &row).expect("upsert");

        let result =
            lookup_active(&mut conn, "12D3KooWXyz", "2026-04-24T12:00:00Z").expect("lookup");
        assert!(result.is_some(), "expected active binding");
        let found = result.unwrap();
        assert_eq!(found.agent_cid, "bafyreiabc");
        assert_eq!(found.source, "dht");
    }

    #[test]
    fn lookup_returns_none_for_unknown_peer() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let result =
            lookup_active(&mut conn, "12D3KooWUnknown", "2026-04-24T12:00:00Z").expect("lookup");
        assert!(result.is_none(), "expected None for unknown peer");
    }

    #[test]
    fn lookup_excludes_expired_binding() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        // Binding valid 2026-01-01 → 2026-02-01; query now = 2026-04-24 (past expiry)
        let row = binding(
            "12D3KooWXyz",
            "bafyreiabc",
            "uhCAkABCDEF",
            "2026-01-01T00:00:00Z",
            Some("2026-02-01T00:00:00Z"),
            "2026-04-24T00:00:00Z",
            "dht",
        );
        upsert(&mut conn, &row).expect("upsert");

        let result =
            lookup_active(&mut conn, "12D3KooWXyz", "2026-04-24T12:00:00Z").expect("lookup");
        assert!(result.is_none(), "expired binding must not be returned");
    }

    #[test]
    fn lookup_returns_most_recently_observed_among_multiple() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        // Two bindings for the same peer, different DHT anchors (e.g. key rotation)
        upsert(
            &mut conn,
            &binding(
                "12D3KooWXyz",
                "bafyreiabc",
                "anchor-1",
                "2026-01-01T00:00:00Z",
                None,
                "2026-04-20T00:00:00Z",
                "dht",
            ),
        )
        .expect("upsert 1");
        upsert(
            &mut conn,
            &binding(
                "12D3KooWXyz",
                "bafyreinew",
                "anchor-2",
                "2026-04-01T00:00:00Z",
                None,
                "2026-04-24T00:00:00Z",
                "gossip",
            ),
        )
        .expect("upsert 2");

        let result =
            lookup_active(&mut conn, "12D3KooWXyz", "2026-04-24T12:00:00Z").expect("lookup");
        assert!(result.is_some());
        // Most recently observed should win
        assert_eq!(result.unwrap().agent_cid, "bafyreinew");
    }

    #[test]
    fn preserves_supersession_on_non_authoritative_rewrite() {
        // Regression: handshake / gossip writers must NOT erase a
        // `superseded_by` value previously written by the DHT-arrival path.
        // INSERT OR REPLACE would clobber the field if we used plain `upsert`;
        // `upsert_preserving_supersession` reads the prior value and folds it
        // forward when the incoming row carries `None`.
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        // 1) DHT-path upsert with superseded_by = Some(...).
        upsert(
            &mut conn,
            &NewPeerIdentityBindingRow {
                peer_id: "peer_X".into(),
                agent_cid: "agent_Y".into(),
                dht_anchor_hash: "uhCkk-row1".into(),
                valid_from: "2026-05-01T00:00:00Z".into(),
                valid_until: None,
                observed_at: "2026-05-01T00:00:00Z".into(),
                source: "dht".into(),
                device_archetype: "desktop".into(),
                superseded_by: Some("uhCkk-prev".into()),
                signature: String::new(),
                proof_status: crate::p2p::binding_proof_wire::BindingProofStatus::unverified(),
            },
        )
        .expect("dht upsert");

        // 2) Gossip-path upsert with superseded_by = None — must NOT erase the prior supersession.
        upsert_preserving_supersession(
            &mut conn,
            &NewPeerIdentityBindingRow {
                peer_id: "peer_X".into(),
                agent_cid: "agent_Y".into(),
                dht_anchor_hash: "uhCkk-row1".into(),
                valid_from: "2026-05-01T00:00:00Z".into(),
                valid_until: None,
                observed_at: "2026-05-01T00:01:00Z".into(),
                source: "gossip".into(),
                device_archetype: "node".into(),
                superseded_by: None, // gossip wire never carries supersession
                signature: String::new(),
                proof_status: crate::p2p::binding_proof_wire::BindingProofStatus::unverified(),
            },
        )
        .expect("gossip preserving upsert");

        use peer_identity_bindings::dsl;
        let row: PeerIdentityBindingRow = dsl::peer_identity_bindings
            .filter(dsl::peer_id.eq("peer_X"))
            .first(&mut conn)
            .expect("row");
        assert_eq!(
            row.superseded_by,
            Some("uhCkk-prev".to_string()),
            "non-authoritative gossip writer must not erase superseded_by"
        );
        // Other fields should reflect the latest gossip arrival.
        assert_eq!(row.source, "gossip");
        assert_eq!(row.observed_at, "2026-05-01T00:01:00Z");
        assert_eq!(row.device_archetype, "node");
    }

    #[test]
    fn preserve_variant_falls_through_when_no_prior_row() {
        // First-observer path: if no prior row exists, the preserving variant
        // behaves identically to plain `upsert` — the row lands with
        // `superseded_by: None` (because the incoming gossip/handshake row
        // carries None and there is nothing to fold forward).
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        upsert_preserving_supersession(
            &mut conn,
            &NewPeerIdentityBindingRow {
                peer_id: "peer_first".into(),
                agent_cid: "agent_first".into(),
                dht_anchor_hash: "anchor-first".into(),
                valid_from: "2026-05-01T00:00:00Z".into(),
                valid_until: None,
                observed_at: "2026-05-01T00:00:00Z".into(),
                source: "gossip".into(),
                device_archetype: "node".into(),
                superseded_by: None,
                signature: String::new(),
                proof_status: crate::p2p::binding_proof_wire::BindingProofStatus::unverified(),
            },
        )
        .expect("first-observer upsert");

        use peer_identity_bindings::dsl;
        let row: PeerIdentityBindingRow = dsl::peer_identity_bindings
            .filter(dsl::peer_id.eq("peer_first"))
            .first(&mut conn)
            .expect("row");
        assert!(row.superseded_by.is_none());
        assert_eq!(row.source, "gossip");
    }

    #[test]
    fn lookup_excludes_superseded_binding() {
        // Two rows for the same peer:
        //   - older row, NOT superseded → must be the active binding
        //   - newer row, superseded_by = Some(...) → must NOT be returned
        // even though it has the latest observed_at.
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        // Older row: un-superseded, observed earlier.
        let older = NewPeerIdentityBindingRow {
            peer_id: "12D3KooWXyz".to_string(),
            agent_cid: "bafyreiabc".to_string(),
            dht_anchor_hash: "anchor-current".to_string(),
            valid_from: "2026-01-01T00:00:00Z".to_string(),
            valid_until: None,
            observed_at: "2026-04-20T00:00:00Z".to_string(),
            source: "dht".to_string(),
            device_archetype: "node".to_string(),
            superseded_by: None,
            signature: String::new(),
            proof_status: crate::p2p::binding_proof_wire::BindingProofStatus::unverified(),
        };
        upsert(&mut conn, &older).expect("upsert older");

        // Newer row: superseded, observed later. Must be excluded by the filter.
        let newer_superseded = NewPeerIdentityBindingRow {
            peer_id: "12D3KooWXyz".to_string(),
            agent_cid: "bafyreiabc".to_string(),
            dht_anchor_hash: "anchor-stale".to_string(),
            valid_from: "2026-01-01T00:00:00Z".to_string(),
            valid_until: None,
            observed_at: "2026-04-24T00:00:00Z".to_string(),
            source: "dht".to_string(),
            device_archetype: "node".to_string(),
            superseded_by: Some("uhCkk-supersedes-this".to_string()),
            signature: String::new(),
            proof_status: crate::p2p::binding_proof_wire::BindingProofStatus::unverified(),
        };
        upsert(&mut conn, &newer_superseded).expect("upsert newer-superseded");

        let result =
            lookup_active(&mut conn, "12D3KooWXyz", "2026-04-24T12:00:00Z").expect("lookup");
        let row = result.expect("expected an active (un-superseded) binding");
        assert_eq!(
            row.dht_anchor_hash, "anchor-current",
            "lookup_active must skip superseded rows even when their observed_at is later"
        );
        assert!(
            row.superseded_by.is_none(),
            "active row must have superseded_by == NULL"
        );
    }

    #[test]
    fn list_active_for_agent_returns_all_devices() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        for (peer, archetype) in [
            ("12D3KooW_M_desktop", "desktop"),
            ("12D3KooW_M_node", "node"),
            ("12D3KooW_M_mobile", "mobile"),
        ] {
            let mut row = binding(
                peer,
                "agent_M",
                &format!("anchor-{peer}"),
                "2026-01-01T00:00:00Z",
                None,
                "2026-04-24T00:00:00Z",
                "dht",
            );
            row.device_archetype = archetype.to_string();
            upsert(&mut conn, &row).expect("upsert");
        }

        let bindings = list_active_for_agent(&mut conn, "agent_M", "2026-04-24T12:00:00Z")
            .expect("list_active_for_agent");
        assert_eq!(bindings.len(), 3);
        assert!(bindings
            .iter()
            .any(|b| b.peer_id == "12D3KooW_M_desktop" && b.device_archetype == "desktop"));
    }

    #[test]
    fn list_active_for_agent_filters_superseded() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        // Active binding for agent_M.
        upsert(
            &mut conn,
            &binding(
                "12D3KooW_active",
                "agent_M",
                "anchor-active",
                "2026-01-01T00:00:00Z",
                None,
                "2026-04-24T00:00:00Z",
                "dht",
            ),
        )
        .expect("upsert active");

        // Superseded binding for the same agent — should be filtered out.
        let mut superseded = binding(
            "12D3KooW_old",
            "agent_M",
            "anchor-old",
            "2026-01-01T00:00:00Z",
            None,
            "2026-04-24T00:00:00Z",
            "dht",
        );
        superseded.superseded_by = Some("anchor-newer".to_string());
        upsert(&mut conn, &superseded).expect("upsert superseded");

        let bindings = list_active_for_agent(&mut conn, "agent_M", "2026-04-24T12:00:00Z")
            .expect("list_active_for_agent");
        assert_eq!(bindings.len(), 1, "superseded row must be filtered out");
        assert_eq!(bindings[0].peer_id, "12D3KooW_active");
    }

    #[test]
    fn list_active_for_agent_empty_for_unknown_agent() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let bindings = list_active_for_agent(&mut conn, "agent_unknown", "2026-04-24T12:00:00Z")
            .expect("list_active_for_agent");
        assert!(bindings.is_empty());
    }

    #[test]
    fn list_active_for_agent_excludes_expired() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        upsert(
            &mut conn,
            &binding(
                "12D3KooW_expired",
                "agent_M",
                "anchor-expired",
                "2026-01-01T00:00:00Z",
                Some("2026-02-01T00:00:00Z"),
                "2026-04-24T00:00:00Z",
                "dht",
            ),
        )
        .expect("upsert expired");

        let bindings = list_active_for_agent(&mut conn, "agent_M", "2026-04-24T12:00:00Z")
            .expect("list_active_for_agent");
        assert!(bindings.is_empty(), "expired bindings must not be returned");
    }
}
