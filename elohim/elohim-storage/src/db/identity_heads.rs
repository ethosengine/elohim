//! Identity-head projection CRUD (Category A DHT projection) — Wave C1 of the
//! identity-head-key-lineage arc.
//!
//! Read-optimised cache of a `binds-identity` Mishpat::Commitment (the identity-head
//! declaration; design §2.2/§3). Source of truth is the Holochain DHT (mishpat DNA
//! Commitment entry, action='binds-identity'); these rows are the P1 reconciliation
//! projection, populated from the create_commitment post-commit signal.
//!
//! A NULL `dht_anchor_hash` means un-notarized / storage-only. The `did:elohim` head
//! resolver (`find_head_by_head_key`) fail-closes on such rows. `cid` is the
//! Commitment `entry_hash` (NEVER `action_hash`); `head_key` is the current head
//! agent_cid (the resolver's join key); `chain_root` is the stable identity-chain id.
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
/// `dht_anchor_hash` AND `revoked_at` are only overwritten when the incoming value
/// is `Some(_)`. A later re-projection from the `binds-identity` signal always
/// carries `dht_anchor_hash = Some` but `revoked_at = None` — so an un-anchored or
/// post-revoke replay (Holochain re-emits on conductor restart/gossip) must never
/// (a) strip the notarised anchor the resolver requires, nor (b) resurrect a revoked
/// head by clobbering `revoked_at` back to NULL. Revocation is owned by
/// `set_revoked_at`, not by the create-projection upsert.
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
            // dht_anchor_hash AND revoked_at are updated conditionally below, never
            // here — so an incoming None cannot clobber an existing anchor or
            // resurrect a revoked head.
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

    ih::identity_heads.filter(ih::cid.eq(&new.cid)).first(conn)
}

/// Fetch a single identity head by its CID. Returns `Ok(None)` when no row exists.
pub fn get_by_cid(conn: &mut SqliteConnection, cid: &str) -> QueryResult<Option<IdentityHeadRow>> {
    ih::identity_heads
        .filter(ih::cid.eq(cid))
        .first(conn)
        .optional()
}

/// The did:elohim resolver's read: the current live, notarized identity head whose
/// `head_key` is `agent_cid`. **Fail-closed**: only a non-revoked AND notarized
/// (`dht_anchor_hash IS NOT NULL`) head is surfaced — an un-notarized or revoked
/// declaration must never populate a `controller`.
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

        // First projection: anchored + revoked.
        let mut revoked = sample_head(cid, HEAD_KEY, Some("h1"));
        revoked.revoked_at = Some("2026-07-17T01:00:00Z".to_string());
        upsert_with_anchor(&mut conn, revoked).expect("first upsert");

        // Replay carrying None for both — must not strip the anchor nor un-revoke.
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
