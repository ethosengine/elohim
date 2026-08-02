//! Identity-head projection CRUD (Category A DHT projection) — Wave C1 of the
//! identity-head-key-lineage arc.
//!
//! Read-optimised cache of a `binds-identity` Mishpat::Commitment (the identity-head
//! declaration; design §2.2/§3). Source of truth is the Holochain DHT (mishpat DNA
//! Commitment entry, action='binds-identity'); these rows are the P1 reconciliation
//! projection, populated from the create_commitment post-commit signal.
//!
//! A NULL `dht_anchor_hash` means un-notarized / storage-only. BOTH reads fail-close
//! on such rows. `cid` is the Commitment `entry_hash` (NEVER `action_hash`);
//! `head_key` is the current head agent_cid (the resolver's join key); `chain_root`
//! is the stable identity-chain id.
//!
//! ## Two reads, deliberately distinct
//!
//! - [`find_head_by_head_key`] — the **live head** (`revoked_at IS NULL`): who
//!   controls this key right now.
//! - [`find_notarized_head_by_head_key`] — the newest notarized declaration
//!   **revoked or not**, carrying `revoked_at`. This is the `did:elohim` resolver's
//!   read, because a revoked head that merely *vanishes* is indistinguishable from
//!   one that was never declared — and an absent `controller` in a DID document
//!   means the subject controls it, so the vanishing served a revoked identity a
//!   fully-armed, implicitly self-controlled document.
//!
//! Mirrors `db::lenses` (the sibling A-class Commitment projection) field-for-field,
//! including the sticky-on-set anchor/revoked preservation rule.
//!
//! Spec: genesis/docs/superpowers/specs/2026-07-17-identity-head-key-lineage-design.md.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use super::diesel_schema::identity_heads::dsl as ih;
use super::models::{current_timestamp, IdentityHeadRow, NewIdentityHead};

/// Idempotent upsert on `cid` (= Commitment entry_hash).
///
/// **Sticky-on-set preservation rule** (mirrors `lenses::upsert_with_anchor`):
/// `dht_anchor_hash`, `revoked_at` AND `successor_head_key` are only overwritten
/// when the incoming value is `Some(_)`. A later re-projection from the
/// `binds-identity` signal always carries `dht_anchor_hash = Some` but
/// `revoked_at = None` — so an un-anchored or post-revoke replay (Holochain
/// re-emits on conductor restart/gossip) must never (a) strip the notarised anchor
/// the resolver requires, nor (b) resurrect a revoked head by clobbering
/// `revoked_at` back to NULL, nor (c) erase a named successor and silently convert
/// a rotation into a terminal revocation. Revocation is owned by `set_revoked_at`,
/// not by the create-projection upsert.
pub fn upsert_with_anchor(
    conn: &mut SqliteConnection,
    new: NewIdentityHead,
) -> QueryResult<IdentityHeadRow> {
    let now = current_timestamp();

    diesel::insert_into(ih::identity_heads)
        .values((
            ih::cid.eq(&new.cid),
            ih::chain_root.eq(&new.chain_root),
            ih::head_key.eq(&new.head_key),
            ih::controllers_json.eq(&new.controllers_json),
            ih::controller_policy_json.eq(&new.controller_policy_json),
            ih::signed_at.eq(&new.signed_at),
            ih::revoked_at.eq(&new.revoked_at),
            ih::successor_head_key.eq(&new.successor_head_key),
            ih::dht_anchor_hash.eq(&new.dht_anchor_hash),
            ih::created_at.eq(&now),
            ih::updated_at.eq(&now),
        ))
        .on_conflict(ih::cid)
        .do_update()
        .set((
            ih::chain_root.eq(new.chain_root.clone()),
            ih::head_key.eq(new.head_key.clone()),
            ih::controllers_json.eq(new.controllers_json.clone()),
            ih::controller_policy_json.eq(new.controller_policy_json.clone()),
            ih::signed_at.eq(new.signed_at.clone()),
            // dht_anchor_hash, revoked_at AND successor_head_key are updated
            // conditionally below, never here — so an incoming None cannot clobber
            // an existing anchor, resurrect a revoked head, or erase a named
            // successor.
            ih::updated_at.eq(&now),
        ))
        .execute(conn)?;

    if let Some(ref anchor) = new.dht_anchor_hash {
        diesel::update(ih::identity_heads.filter(ih::cid.eq(&new.cid)))
            .set(ih::dht_anchor_hash.eq(anchor))
            .execute(conn)?;
    }

    if let Some(ref ts) = new.revoked_at {
        diesel::update(ih::identity_heads.filter(ih::cid.eq(&new.cid)))
            .set(ih::revoked_at.eq(ts))
            .execute(conn)?;
    }

    if let Some(ref successor) = new.successor_head_key {
        diesel::update(ih::identity_heads.filter(ih::cid.eq(&new.cid)))
            .set(ih::successor_head_key.eq(successor))
            .execute(conn)?;
    }

    ih::identity_heads.filter(ih::cid.eq(&new.cid)).first(conn)
}

/// Fetch a single identity head by its CID. Returns `Ok(None)` when no row exists.
pub fn get_by_cid(conn: &mut SqliteConnection, cid: &str) -> QueryResult<Option<IdentityHeadRow>> {
    ih::identity_heads
        .filter(ih::cid.eq(cid))
        .first(conn)
        .optional()
}

/// The current **live** (non-revoked), notarized identity head whose `head_key` is
/// `agent_cid`. Only a non-revoked AND notarized (`dht_anchor_hash IS NOT NULL`)
/// head is surfaced — an un-notarized or revoked declaration must never populate a
/// `controller`.
///
/// # ⚠ A revoked head is INVISIBLE to this query
///
/// This is a *live-head* read, not an *existence* read. Never use it to decide
/// whether a head was ever declared: `Ok(None)` here folds "no declaration exists"
/// together with "the declaration was revoked", and an absent `controller` in a DID
/// document means **the subject controls it** — so answering `NeverDeclared` from
/// this query's `None` serves a revoked identity a fully-armed, implicitly
/// self-controlled document. That is exactly the degradation
/// [`did_bridge::IdentityHeadAnswer`] exists to close, and it is why the
/// `did:elohim` resolver reads [`find_notarized_head_by_head_key`] instead.
///
/// Kept as the scoped "who controls this key right now" read for callers that have
/// already established the head exists and want only a live one.
///
/// **Head selection is DHT-canonical, node-independent** — `(signed_at DESC, cid
/// ASC)`, NEVER `created_at` (local signal-arrival order, which differs per node and
/// would make the resolved head depend on out-of-order arrival / projector catch-up
/// / a stale re-projection landing after a legitimate rotation). `signed_at` is the
/// notarized Commitment-envelope signing time (same on every node); `cid` (the
/// content-addressed entry_hash) is the deterministic tiebreak for a `signed_at`
/// collision. This is the design's "heads move by judgment over history, never
/// last-writer-wins" rule (design §1, kinship-lineage spec).
pub fn find_head_by_head_key(
    conn: &mut SqliteConnection,
    agent_cid: &str,
) -> QueryResult<Option<IdentityHeadRow>> {
    ih::identity_heads
        .filter(ih::head_key.eq(agent_cid))
        .filter(ih::revoked_at.is_null())
        .filter(ih::dht_anchor_hash.is_not_null())
        .order((ih::signed_at.desc(), ih::cid.asc()))
        .first(conn)
        .optional()
}

/// The `did:elohim` resolver's read: the newest **notarized** declaration for
/// `head_key`, **revoked or not** — the row carries `revoked_at` so the caller can
/// tell a live head from a revoked one instead of a revoked head vanishing into the
/// same silence as one that was never declared.
///
/// **Still fail-closed on notarization**: `dht_anchor_hash IS NOT NULL` is
/// unchanged, so an un-notarized (storage-only) declaration surfaces neither as a
/// head nor as a revocation. Only the `revoked_at IS NULL` filter is relaxed, and
/// relaxing it is the point: the caller must *see* the revocation to serve a
/// deactivated document.
///
/// **Selection is the same DHT-canonical ordering** as
/// [`find_head_by_head_key`] — `(signed_at DESC, cid ASC)`, never `created_at` —
/// so the answer is node-independent. Revocation does NOT re-order: the newest
/// *declaration* wins and its lifecycle is then read off `revoked_at`. Two
/// consequences worth stating, because they are the whole behavioural difference:
///
/// - newest declaration revoked, an older live one behind it ⇒ **Revoked**. Serving
///   the older declaration's controllers would serve a superseded authority set.
/// - newest declaration live, an older revoked one behind it ⇒ **live head**. A
///   re-binding after a revocation (a community recovery re-declaring the same key)
///   is a legitimate return to service.
pub fn find_notarized_head_by_head_key(
    conn: &mut SqliteConnection,
    agent_cid: &str,
) -> QueryResult<Option<IdentityHeadRow>> {
    ih::identity_heads
        .filter(ih::head_key.eq(agent_cid))
        .filter(ih::dht_anchor_hash.is_not_null())
        .order((ih::signed_at.desc(), ih::cid.asc()))
        .first(conn)
        .optional()
}

/// Mark an identity head revoked by its CID (= Commitment entry_hash). Mirrors
/// `lenses::set_revoked_at`.
///
/// A `revokes-commitment` may target a `mishpat_commitments` row, a `lenses` row,
/// OR an `identity_heads` row — distinct tables over the same CID space — so the
/// revoke projection calls all three (each no-ops, returning 0, when the CID is
/// absent from that table). Returns rows affected.
pub fn set_revoked_at(conn: &mut SqliteConnection, cid: &str, ts: &str) -> QueryResult<usize> {
    let now = current_timestamp();
    diesel::update(ih::identity_heads.filter(ih::cid.eq(cid)))
        .set((ih::revoked_at.eq(ts), ih::updated_at.eq(&now)))
        .execute(conn)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn test_conn() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory SQLite");
        conn.run_pending_migrations(MIGRATIONS).expect("migrations");
        conn
    }

    const HEAD_KEY: &str = "uhCAk39SDf7rynCg5bYgzroGaOJKGKrloI1o57Xao6S-U5KNZ0dUH";

    fn sample_head(cid: &str, head_key: &str, anchor: Option<&str>) -> NewIdentityHead {
        sample_head_signed(cid, head_key, anchor, "2026-07-17T00:00:00Z")
    }

    fn sample_head_signed(
        cid: &str,
        head_key: &str,
        anchor: Option<&str>,
        signed_at: &str,
    ) -> NewIdentityHead {
        NewIdentityHead {
            cid: cid.to_string(),
            chain_root: "bafyreichainroot0000".to_string(),
            head_key: head_key.to_string(),
            controllers_json: format!(r#"["{head_key}","uhCAkRecoveryQuorumKey"]"#),
            controller_policy_json: r#"{"kind":"recovery-quorum","m":2,"n":3}"#.to_string(),
            signed_at: signed_at.to_string(),
            revoked_at: None,
            successor_head_key: None,
            dht_anchor_hash: anchor.map(str::to_string),
        }
    }

    #[test]
    fn upsert_inserts_and_round_trips() {
        let mut conn = test_conn();
        let cid = "identity-head:1";

        let row = upsert_with_anchor(&mut conn, sample_head(cid, HEAD_KEY, Some("action-hash-1")))
            .expect("first upsert");
        assert_eq!(row.cid, cid, "cid is the entry_hash read key");
        assert_eq!(row.chain_root, "bafyreichainroot0000");
        assert_eq!(row.head_key, HEAD_KEY);
        assert_eq!(row.dht_anchor_hash.as_deref(), Some("action-hash-1"));

        let fetched = get_by_cid(&mut conn, cid)
            .expect("get_by_cid")
            .expect("row must exist");
        assert_eq!(fetched.controllers_json, row.controllers_json);
    }

    #[test]
    fn find_head_by_head_key_reads_chain_root_and_controllers() {
        // The load-bearing read the did:elohim resolver depends on: a notarized,
        // live binds-identity is read back keyed on head_key, with the right
        // chain_root + controllers.
        let mut conn = test_conn();
        upsert_with_anchor(&mut conn, sample_head("ih:live", HEAD_KEY, Some("a1"))).expect("seed");

        let head = find_head_by_head_key(&mut conn, HEAD_KEY)
            .expect("query")
            .expect("a live notarized head must be found");
        assert_eq!(head.chain_root, "bafyreichainroot0000");
        let controllers: Vec<String> =
            serde_json::from_str(&head.controllers_json).expect("controllers_json is a JSON array");
        assert_eq!(controllers.len(), 2);
        assert!(controllers.contains(&HEAD_KEY.to_string()));
    }

    #[test]
    fn find_head_picks_newest_signed_at_regardless_of_insert_order() {
        // DHT-canonical selection: when two live+notarized heads exist for one
        // head_key, the newer `signed_at` wins REGARDLESS of local insert order.
        // Insert the NEWER-signed head FIRST and the OLDER-signed head SECOND, so a
        // `created_at DESC` (local-arrival) ordering would wrongly pick the older
        // one. `signed_at DESC` must pick the newer.
        let mut conn = test_conn();
        // Inserted first → earlier created_at, but signed LATER.
        upsert_with_anchor(
            &mut conn,
            sample_head_signed("ih:newer", HEAD_KEY, Some("a-new"), "2026-07-17T09:00:00Z"),
        )
        .expect("newer-signed head");
        // Inserted second → later created_at, but signed EARLIER.
        upsert_with_anchor(
            &mut conn,
            sample_head_signed("ih:older", HEAD_KEY, Some("a-old"), "2026-07-17T08:00:00Z"),
        )
        .expect("older-signed head");

        let head = find_head_by_head_key(&mut conn, HEAD_KEY)
            .expect("query")
            .expect("a head must resolve");
        assert_eq!(
            head.cid, "ih:newer",
            "must pick the newer SIGNED_AT head, not the later-arriving (created_at) one"
        );
    }

    #[test]
    fn find_head_tiebreaks_on_cid_when_signed_at_collides() {
        // Two heads with an identical signed_at must resolve deterministically the
        // SAME on every node — `cid ASC` is the content-addressed tiebreak.
        let mut conn = test_conn();
        let ts = "2026-07-17T10:00:00Z";
        // Insert in the "wrong" order (larger cid first) to prove ordering, not arrival.
        upsert_with_anchor(
            &mut conn,
            sample_head_signed("ih:zzz", HEAD_KEY, Some("az"), ts),
        )
        .expect("zzz");
        upsert_with_anchor(
            &mut conn,
            sample_head_signed("ih:aaa", HEAD_KEY, Some("aa"), ts),
        )
        .expect("aaa");

        let head = find_head_by_head_key(&mut conn, HEAD_KEY)
            .expect("query")
            .expect("a head must resolve");
        assert_eq!(
            head.cid, "ih:aaa",
            "on a signed_at collision, cid ASC is the deterministic fleet-wide tiebreak"
        );
    }

    #[test]
    fn find_head_is_fail_closed() {
        let mut conn = test_conn();

        // Un-notarized (anchor NULL) — excluded.
        upsert_with_anchor(&mut conn, sample_head("ih:unanchored", HEAD_KEY, None))
            .expect("unanchored");
        assert!(
            find_head_by_head_key(&mut conn, HEAD_KEY)
                .expect("query")
                .is_none(),
            "an un-notarized head must never surface (fail-closed)"
        );

        // Revoked — excluded.
        let mut revoked = sample_head("ih:revoked", HEAD_KEY, Some("a2"));
        revoked.revoked_at = Some("2026-07-17T01:00:00Z".to_string());
        upsert_with_anchor(&mut conn, revoked).expect("revoked");
        assert!(
            find_head_by_head_key(&mut conn, HEAD_KEY)
                .expect("query")
                .is_none(),
            "a revoked head must never surface (fail-closed)"
        );

        // A different head_key — not matched.
        assert!(find_head_by_head_key(&mut conn, "uhCAkSomeOtherKey")
            .expect("query")
            .is_none());
    }

    #[test]
    fn upsert_preserves_anchor_and_revoked_when_new_is_null() {
        let mut conn = test_conn();
        let cid = "ih:preserve";

        // First projection: anchored + revoked + a named successor.
        let mut revoked = sample_head(cid, HEAD_KEY, Some("h1"));
        revoked.revoked_at = Some("2026-07-17T01:00:00Z".to_string());
        revoked.successor_head_key = Some("uhCAkSuccessorKey".to_string());
        upsert_with_anchor(&mut conn, revoked).expect("first upsert");

        // Replay carrying None for all three — must not strip the anchor, un-revoke,
        // nor erase the successor.
        upsert_with_anchor(&mut conn, sample_head(cid, HEAD_KEY, None)).expect("replay");

        let row = get_by_cid(&mut conn, cid)
            .expect("get_by_cid")
            .expect("row exists");
        assert_eq!(
            row.dht_anchor_hash.as_deref(),
            Some("h1"),
            "anchor preserved on a None replay"
        );
        assert_eq!(
            row.revoked_at.as_deref(),
            Some("2026-07-17T01:00:00Z"),
            "revoked_at preserved on a None replay (no fail-open resurrection)"
        );
        assert_eq!(
            row.successor_head_key.as_deref(),
            Some("uhCAkSuccessorKey"),
            "successor preserved on a None replay — erasing it would silently \
             convert a rotation into a terminal revocation"
        );
    }

    // ── find_notarized_head_by_head_key (the row-8 production closure) ─────────

    #[test]
    fn notarized_query_surfaces_a_revoked_head_the_live_query_hides() {
        // THE row-8 fix, at the query layer. The live-head query filters
        // `revoked_at IS NULL`, so a revoked head is indistinguishable from one that
        // was never declared — and the did:elohim resolver then assembles the
        // phase-1 implicit-self document (fully armed, implicitly self-controlled)
        // for a revoked identity. The notarized query must SURFACE the row, carrying
        // revoked_at, so the resolver can deactivate the document instead.
        let mut conn = test_conn();
        upsert_with_anchor(&mut conn, sample_head("ih:rev", HEAD_KEY, Some("a1"))).expect("seed");
        set_revoked_at(&mut conn, "ih:rev", "2026-07-17T03:00:00Z").expect("revoke");

        assert!(
            find_head_by_head_key(&mut conn, HEAD_KEY)
                .expect("live query")
                .is_none(),
            "the live-head query still hides a revoked head (unchanged semantics)"
        );

        let row = find_notarized_head_by_head_key(&mut conn, HEAD_KEY)
            .expect("notarized query")
            .expect("a revoked head MUST surface — invisibility is the vulnerability");
        assert_eq!(row.cid, "ih:rev");
        assert_eq!(
            row.revoked_at.as_deref(),
            Some("2026-07-17T03:00:00Z"),
            "revoked_at rides the row so the caller can tell revoked from live"
        );
    }

    #[test]
    fn notarized_query_still_fail_closes_on_un_notarized() {
        // Only the revoked filter is relaxed. An un-notarized (storage-only) row
        // must surface neither as a head nor as a revocation.
        let mut conn = test_conn();
        upsert_with_anchor(&mut conn, sample_head("ih:unanchored", HEAD_KEY, None))
            .expect("unanchored");
        assert!(
            find_notarized_head_by_head_key(&mut conn, HEAD_KEY)
                .expect("query")
                .is_none(),
            "an un-notarized declaration must never surface (fail-closed unchanged)"
        );
        // And an absent head_key is still None (absence is not invented).
        assert!(
            find_notarized_head_by_head_key(&mut conn, "uhCAkSomeOtherKey")
                .expect("query")
                .is_none()
        );
    }

    #[test]
    fn newest_declaration_wins_even_when_it_is_the_revoked_one() {
        // Revocation does NOT re-order: the newest DECLARATION wins and its
        // lifecycle is read off revoked_at. Serving the older live declaration's
        // controllers would serve a superseded authority set.
        let mut conn = test_conn();
        upsert_with_anchor(
            &mut conn,
            sample_head_signed(
                "ih:older-live",
                HEAD_KEY,
                Some("a-old"),
                "2026-07-17T08:00:00Z",
            ),
        )
        .expect("older live head");
        upsert_with_anchor(
            &mut conn,
            sample_head_signed(
                "ih:newer-revoked",
                HEAD_KEY,
                Some("a-new"),
                "2026-07-17T09:00:00Z",
            ),
        )
        .expect("newer head");
        set_revoked_at(&mut conn, "ih:newer-revoked", "2026-07-17T10:00:00Z").expect("revoke");

        let row = find_notarized_head_by_head_key(&mut conn, HEAD_KEY)
            .expect("query")
            .expect("a row must surface");
        assert_eq!(
            row.cid, "ih:newer-revoked",
            "the newest declaration wins even when revoked — the older live one is superseded"
        );
        assert!(row.revoked_at.is_some());
    }

    #[test]
    fn a_re_binding_after_revocation_returns_the_key_to_service() {
        // The mirror case: newest declaration LIVE, an older revoked one behind it
        // (a community recovery re-declaring the same key) is a legitimate return to
        // service, not a permanent deactivation.
        let mut conn = test_conn();
        upsert_with_anchor(
            &mut conn,
            sample_head_signed(
                "ih:old-revoked",
                HEAD_KEY,
                Some("a-old"),
                "2026-07-17T08:00:00Z",
            ),
        )
        .expect("older head");
        set_revoked_at(&mut conn, "ih:old-revoked", "2026-07-17T08:30:00Z").expect("revoke");
        upsert_with_anchor(
            &mut conn,
            sample_head_signed(
                "ih:re-bound",
                HEAD_KEY,
                Some("a-new"),
                "2026-07-17T09:00:00Z",
            ),
        )
        .expect("re-binding");

        let row = find_notarized_head_by_head_key(&mut conn, HEAD_KEY)
            .expect("query")
            .expect("a row must surface");
        assert_eq!(row.cid, "ih:re-bound");
        assert!(
            row.revoked_at.is_none(),
            "a re-binding after revocation is live again — the revocation does not stick to the key"
        );
    }

    #[test]
    fn successor_head_key_round_trips_when_a_declaration_names_one() {
        // No declaration names a successor today (the mishpat validator does not
        // require the field), so this pins the COLUMN plumbing ahead of its
        // producer: when a rotation starts naming one, it reaches the resolver.
        let mut conn = test_conn();
        let mut rotated = sample_head("ih:rotated", HEAD_KEY, Some("a1"));
        rotated.successor_head_key = Some("uhCAkSuccessorKey".to_string());
        upsert_with_anchor(&mut conn, rotated).expect("seed");
        set_revoked_at(&mut conn, "ih:rotated", "2026-07-17T03:00:00Z").expect("revoke");

        let row = find_notarized_head_by_head_key(&mut conn, HEAD_KEY)
            .expect("query")
            .expect("row surfaces");
        assert_eq!(
            row.successor_head_key.as_deref(),
            Some("uhCAkSuccessorKey"),
            "a named successor reaches the resolver so a reference on the revoked \
             head can follow the identity forward (C9 re-anchor)"
        );
    }

    #[test]
    fn set_revoked_at_marks_head_revoked_and_noops_on_absent() {
        let mut conn = test_conn();
        upsert_with_anchor(&mut conn, sample_head("ih:sr", HEAD_KEY, Some("a1"))).expect("seed");

        assert_eq!(
            set_revoked_at(&mut conn, "ih:sr", "2026-07-17T02:00:00Z").expect("revoke"),
            1
        );
        assert!(
            find_head_by_head_key(&mut conn, HEAD_KEY)
                .expect("query")
                .is_none(),
            "a revoked head is fail-closed excluded"
        );
        // Revoking a CID absent from identity_heads is a no-op.
        assert_eq!(
            set_revoked_at(&mut conn, "ih:absent", "2026-07-17T02:00:00Z").expect("noop"),
            0
        );
    }
}
