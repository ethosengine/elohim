//! Epic B: the production creation path for active provide commitments.
//!
//! ## What this proves
//!
//! Before this work, a production database had ZERO `rea_commitments` rows with
//! `action='provide'` / `state='active'` / `resource_classified_as='content:<reach>'`
//! — so the resilience snapshot's `commitment_backed_collectives` was permanently
//! 0. Those rows existed only in `test_util`; the production provide loop authored
//! `replicates-commons` Commitments that projected ONLY into `mishpat_commitments`,
//! never into the `rea_commitments` provide shape the snapshot + PeerSelection read.
//!
//! The creation path now lives in the signal projector: when a notarized
//! `replicates-commons` CONTENT commitment lands via `handle_mishpat_signal`
//! (`MishpatSignal::CommitmentCommitted`), it ALSO records a standing
//! `rea_commitments` provide row (substrate-anchored, idempotent).
//!
//! These tests drive the REAL signal handler with the REAL production payload
//! shape (`conductor_commitment_author::build_content_payload`), then assert the
//! snapshot — going through the ACTUAL snapshot query, including the
//! `humans.agent_pub_key == rea_commitments.provider` junction — reports
//! `commitment_backed_collectives >= 1`.

use diesel::{ExpressionMethods, RunQueryDsl};
use elohim_storage::db;
use elohim_storage::db::models::{NewHuman, NewShardLocation, NewShardManifest};
use elohim_storage::mishpat_projection::CommitmentPayload;
use elohim_storage::services::conductor_commitment_author::build_content_payload;
use elohim_storage::services::household_resilience;
use elohim_storage::signals::{handle_mishpat_signal, MishpatSignal};
use elohim_storage::test_util::test_pool;

const APP: &str = "lamad";

fn ctx() -> db::AppContext {
    db::AppContext {
        h_app_id: APP.into(),
        local_libp2p_peer_id: None,
    }
}

/// Seed a human whose `agent_pub_key` matches the commitment provider — this is
/// the snapshot join key. `household_id` must be non-null for the household to
/// be counted as commitment-backed.
fn seed_human(conn: &mut diesel::SqliteConnection, agent_key: &str, household_id: Option<&str>) {
    diesel::insert_into(db::diesel_schema::humans::table)
        .values(&NewHuman {
            id: format!("human-{agent_key}"),
            agent_pub_key: Some(agent_key.into()),
            display_name: agent_key.into(),
            bio: None,
            affinities: "[]".into(),
            profile_reach: "commons".into(),
            location: None,
            profile_photo_url: None,
            h_app_id: APP.into(),
            household_id: household_id.map(str::to_string),
        })
        .execute(conn)
        .unwrap();
}

/// Seed a content + manifest + shard_locations so the snapshot reads `measured`
/// and the content's reach resolves to `commons` (matching the provide scope).
fn seed_content_with_steward(
    conn: &mut diesel::SqliteConnection,
    content_id: &str,
    steward_agent: &str,
) {
    let shard = format!("shard-{content_id}");
    let manifest = NewShardManifest {
        content_id,
        h_app_id: APP,
        blob_hash: "blob-hash-stub",
        blob_cid: None,
        encoding: "identity",
        data_shard_count: 1,
        parity_shard_count: 0,
        shard_hashes_json: &format!(r#"["{shard}"]"#),
        total_size_bytes: 0,
        shard_size_bytes: 0,
        mime_type: "application/octet-stream",
        reach: "commons",
    };
    db::shard_manifests::upsert_manifest(conn, &manifest).unwrap();

    // content row so snapshot's reach lookup returns "commons" explicitly.
    diesel::insert_into(db::diesel_schema::content::table)
        .values((
            db::diesel_schema::content::id.eq(content_id),
            db::diesel_schema::content::h_app_id.eq(APP),
            db::diesel_schema::content::title.eq("epic-b fixture"),
            db::diesel_schema::content::content_type.eq("concept"),
            db::diesel_schema::content::reach.eq("commons"),
        ))
        .execute(conn)
        .unwrap();

    let loc = NewShardLocation {
        shard_hash: &shard,
        peer_id: steward_agent,
        h_app_id: APP,
        status: "announced",
    };
    db::shard_locations::upsert_location(conn, &loc).unwrap();
}

/// Drive the REAL signal handler with a notarized `replicates-commons` CONTENT
/// commitment, exactly as the production provide loop authors it.
fn project_commons_commitment(
    pool: &db::DbPool,
    provider: &str,
    head_ref: &str,
    entry_hash: &str,
    action_hash: &str,
) {
    let payload_json = build_content_payload(
        provider,
        head_ref,
        "2026-06-01T00:00:00Z",
        "2027-06-01T00:00:00Z",
        60,
    );
    let signal = MishpatSignal::CommitmentCommitted {
        action_hash: action_hash.into(),
        entry_hash: entry_hash.into(),
        author: provider.into(),
        commitment: CommitmentPayload {
            action: "replicates-commons".into(),
            payload_json,
            signed_at: "2026-06-01T00:00:00Z".into(),
        },
    };
    let mut conn = pool.get().expect("pool connection");
    handle_mishpat_signal(&mut conn, APP, signal)
        .expect("handle_mishpat_signal must succeed for a well-formed replicates-commons signal");
}

#[test]
fn replicates_commons_signal_creates_active_provide_row() {
    use diesel::prelude::*;
    let pool = test_pool();
    project_commons_commitment(
        &pool,
        "agent:steward-a",
        "epr:album-1",
        "uhCEk-entry-1",
        "uhCkk-action-1",
    );

    let mut conn = pool.get().unwrap();
    // The provide row landed: action='provide', state='active', content:commons,
    // provider==commitment provider, dht_anchor_hash==action_hash.
    use db::diesel_schema::rea_commitments as r;
    let rows: Vec<(String, String, Option<String>, String, Option<String>)> = r::table
        .filter(r::action.eq("provide"))
        .filter(r::provider.eq("agent:steward-a"))
        .select((
            r::action,
            r::state,
            r::resource_classified_as,
            r::provider,
            r::dht_anchor_hash,
        ))
        .load(&mut conn)
        .unwrap();
    assert_eq!(rows.len(), 1, "exactly one provide row for the provider");
    let (action, state, scope, provider, anchor) = &rows[0];
    assert_eq!(action, "provide");
    assert_eq!(state, "active");
    assert_eq!(scope.as_deref(), Some("content:commons"));
    assert_eq!(provider, "agent:steward-a");
    assert_eq!(
        anchor.as_deref(),
        Some("uhCkk-action-1"),
        "provenance: the provide projection carries the commitment action_hash"
    );

    // The mishpat_commitments projection still landed (the authoritative write
    // must not be shadowed by the side-projection).
    use db::diesel_schema::mishpat_commitments as m;
    let mishpat_count: i64 = m::table
        .filter(m::action.eq("replicates-commons"))
        .filter(m::provider.eq("agent:steward-a"))
        .count()
        .get_result(&mut conn)
        .unwrap();
    assert_eq!(mishpat_count, 1, "authoritative mishpat row also projected");
}

#[test]
fn repeated_commons_commitments_collapse_to_one_provide_row() {
    use diesel::prelude::*;
    let pool = test_pool();
    // Same provider, two different content head_refs — one standing provide row
    // per (provider, reach), idempotent.
    project_commons_commitment(
        &pool,
        "agent:steward-a",
        "epr:album-1",
        "uhCEk-e1",
        "uhCkk-a1",
    );
    project_commons_commitment(
        &pool,
        "agent:steward-a",
        "epr:album-2",
        "uhCEk-e2",
        "uhCkk-a2",
    );

    let mut conn = pool.get().unwrap();
    use db::diesel_schema::rea_commitments as r;
    let count: i64 = r::table
        .filter(r::action.eq("provide"))
        .filter(r::provider.eq("agent:steward-a"))
        .count()
        .get_result(&mut conn)
        .unwrap();
    assert_eq!(
        count, 1,
        "two commons commitments collapse to one provide row"
    );
}

#[test]
fn capacity_variant_does_not_create_provide_row() {
    use diesel::prelude::*;
    let pool = test_pool();
    // A capacity pledge (no head_ref counterparty) must NOT mint a content-reach
    // provide row — it is a byte pledge, not a per-content offer.
    let payload_json = serde_json::json!({
        "action": "replicates-commons",
        "variant": "capacity",
        "commons_bytes": 1_000_000_000u64,
        "ratio_attestation": { "stewarded_bytes": 0, "used_bytes": 0 },
        "provider": "agent:pledger",
        "bounds": { "reach_ceiling": "commons" },
    })
    .to_string();
    let signal = MishpatSignal::CommitmentCommitted {
        action_hash: "uhCkk-cap".into(),
        entry_hash: "uhCEk-cap".into(),
        author: "agent:pledger".into(),
        commitment: CommitmentPayload {
            action: "replicates-commons".into(),
            payload_json,
            signed_at: "2026-06-01T00:00:00Z".into(),
        },
    };
    {
        let mut conn = pool.get().unwrap();
        handle_mishpat_signal(&mut conn, APP, signal).expect("capacity signal projects");
    }

    let mut conn = pool.get().unwrap();
    use db::diesel_schema::rea_commitments as r;
    let count: i64 = r::table
        .filter(r::action.eq("provide"))
        .count()
        .get_result(&mut conn)
        .unwrap();
    assert_eq!(count, 0, "a capacity pledge mints no provide row");
}

/// THE Epic B end-to-end assertion: content + the production provide creation
/// path + the humans junction → snapshot reports commitment_backed >= 1.
#[test]
fn snapshot_counts_commitment_backed_collective_via_creation_path() {
    let pool = test_pool();
    let content_id = "content-epic-b";
    let provider = "agent:steward-a";

    {
        let mut conn = pool.get().unwrap();
        // The steward stewards this content (shard_locations junction) AND is a
        // member of a household (the snapshot's distinct-household key).
        seed_human(&mut conn, provider, Some("home-a"));
        seed_content_with_steward(&mut conn, content_id, provider);
    }

    // Production creation path: a notarized replicates-commons commitment lands.
    project_commons_commitment(&pool, provider, "epr:some-head", "uhCEk-e2e", "uhCkk-e2e");

    // Go through the ACTUAL snapshot query (humans.agent_pub_key join included).
    let snapshot = household_resilience::snapshot(&pool, &ctx(), content_id, None).unwrap();
    assert_eq!(
        snapshot.commitment_backed_collectives, 1,
        "the steward's notarized commons commitment makes its household \
         commitment-backed: got {} (distribution_state={})",
        snapshot.commitment_backed_collectives, snapshot.distribution_state
    );
}

/// Provider with NO household junction does not count — the join is load-bearing.
#[test]
fn snapshot_excludes_provider_without_household() {
    let pool = test_pool();
    let content_id = "content-epic-b-nohh";
    let provider = "agent:steward-orphan";

    {
        let mut conn = pool.get().unwrap();
        // Human exists but household_id is NULL.
        seed_human(&mut conn, provider, None);
        seed_content_with_steward(&mut conn, content_id, provider);
    }

    project_commons_commitment(
        &pool,
        provider,
        "epr:orphan-head",
        "uhCEk-orph",
        "uhCkk-orph",
    );

    let snapshot = household_resilience::snapshot(&pool, &ctx(), content_id, None).unwrap();
    assert_eq!(
        snapshot.commitment_backed_collectives, 0,
        "a provider with no household_id is not a commitment-backed collective"
    );
}

// ── Non-commons (household reach) end-to-end ──────────────────────────────────
// These exercise the Stage-A producer path at a non-commons reach: the signal
// handler parses a household-reach content commitment, provide_projection_for
// reads the reach through, record_provide writes content:household, and the
// (unchanged) snapshot — scoped to the content's own reach — counts it. The
// author still emits the `replicates-commons` action string (Stage B renames the
// author); the reach is what generalizes. We build the payload directly here
// because the author-side `build_content_payload` is commons-pinned until Stage B.

/// Seed a content + manifest + shard_locations at an arbitrary reach so the
/// snapshot's reach lookup resolves to `reach` (matching the provide scope).
fn seed_content_with_steward_at_reach(
    conn: &mut diesel::SqliteConnection,
    content_id: &str,
    steward_agent: &str,
    reach: &str,
) {
    let shard = format!("shard-{content_id}");
    let manifest = NewShardManifest {
        content_id,
        h_app_id: APP,
        blob_hash: "blob-hash-stub",
        blob_cid: None,
        encoding: "identity",
        data_shard_count: 1,
        parity_shard_count: 0,
        shard_hashes_json: &format!(r#"["{shard}"]"#),
        total_size_bytes: 0,
        shard_size_bytes: 0,
        mime_type: "application/octet-stream",
        reach,
    };
    db::shard_manifests::upsert_manifest(conn, &manifest).unwrap();

    diesel::insert_into(db::diesel_schema::content::table)
        .values((
            db::diesel_schema::content::id.eq(content_id),
            db::diesel_schema::content::h_app_id.eq(APP),
            db::diesel_schema::content::title.eq("epic-b non-commons fixture"),
            db::diesel_schema::content::content_type.eq("concept"),
            db::diesel_schema::content::reach.eq(reach),
        ))
        .execute(conn)
        .unwrap();

    let loc = NewShardLocation {
        shard_hash: &shard,
        peer_id: steward_agent,
        h_app_id: APP,
        status: "announced",
    };
    db::shard_locations::upsert_location(conn, &loc).unwrap();
}

/// Drive the REAL signal handler with a household-reach content commitment.
/// `reach_ceiling` is set equal to `reach` (well-ordered). Author action string
/// stays `replicates-commons` (the migration-window alias) — the reach is what
/// generalizes.
fn project_content_commitment_at_reach(
    pool: &db::DbPool,
    provider: &str,
    head_ref: &str,
    reach: &str,
    entry_hash: &str,
    action_hash: &str,
) {
    let payload_json = serde_json::json!({
        "action": "replicates-commons",
        "variant": "content",
        "head_ref": head_ref,
        "reach": reach,
        "bounds": { "rate_per_minute": 60, "reach_ceiling": reach },
        "provider": provider,
        "valid_from": "2026-06-01T00:00:00Z",
        "valid_until": "2027-06-01T00:00:00Z",
    })
    .to_string();
    let signal = MishpatSignal::CommitmentCommitted {
        action_hash: action_hash.into(),
        entry_hash: entry_hash.into(),
        author: provider.into(),
        commitment: CommitmentPayload {
            action: "replicates-commons".into(),
            payload_json,
            signed_at: "2026-06-01T00:00:00Z".into(),
        },
    };
    let mut conn = pool.get().expect("pool connection");
    handle_mishpat_signal(&mut conn, APP, signal)
        .expect("handle_mishpat_signal must succeed for a well-formed household-reach signal");
}

/// A household-reach content commitment writes a `content:household` provide row
/// (reach read through, not pinned commons).
#[test]
fn household_reach_signal_creates_content_household_provide_row() {
    use diesel::prelude::*;
    let pool = test_pool();
    project_content_commitment_at_reach(
        &pool,
        "agent:steward-hh",
        "epr:household-record",
        "household",
        "uhCEk-hh",
        "uhCkk-hh",
    );

    let mut conn = pool.get().unwrap();
    use db::diesel_schema::rea_commitments as r;
    let scope: Option<String> = r::table
        .filter(r::action.eq("provide"))
        .filter(r::provider.eq("agent:steward-hh"))
        .select(r::resource_classified_as)
        .first(&mut conn)
        .unwrap();
    assert_eq!(
        scope.as_deref(),
        Some("content:household"),
        "the household-reach commitment projects content:household, not content:commons"
    );
}

/// THE non-commons counting assertion: household content + the production
/// creation path + the humans junction (with household_id) → snapshot counts it.
#[test]
fn snapshot_counts_non_commons_commitment_backed_collective() {
    let pool = test_pool();
    let content_id = "content-epic-b-household";
    let provider = "agent:steward-hh-counted";

    {
        let mut conn = pool.get().unwrap();
        seed_human(&mut conn, provider, Some("home-hh"));
        seed_content_with_steward_at_reach(&mut conn, content_id, provider, "household");
    }

    project_content_commitment_at_reach(
        &pool,
        provider,
        "epr:household-head",
        "household",
        "uhCEk-hh-e2e",
        "uhCkk-hh-e2e",
    );

    // The snapshot scopes to the content's own reach (household) — UNCHANGED
    // reader — and counts the content:household provide row.
    let snapshot = household_resilience::snapshot(&pool, &ctx(), content_id, None).unwrap();
    assert_eq!(
        snapshot.commitment_backed_collectives, 1,
        "a household-reach content with a household-reach provide commitment and a \
         provider carrying household_id IS commitment-backed: got {} (distribution_state={})",
        snapshot.commitment_backed_collectives, snapshot.distribution_state
    );
}

/// Correct-but-dormant: a household-reach provide row with a provider lacking
/// household_id counts zero (honest zero until the Epic-B junction lands).
#[test]
fn snapshot_excludes_non_commons_provider_without_household() {
    let pool = test_pool();
    let content_id = "content-epic-b-household-nohh";
    let provider = "agent:steward-hh-orphan";

    {
        let mut conn = pool.get().unwrap();
        seed_human(&mut conn, provider, None);
        seed_content_with_steward_at_reach(&mut conn, content_id, provider, "household");
    }

    project_content_commitment_at_reach(
        &pool,
        provider,
        "epr:household-orphan-head",
        "household",
        "uhCEk-hh-orph",
        "uhCkk-hh-orph",
    );

    let snapshot = household_resilience::snapshot(&pool, &ctx(), content_id, None).unwrap();
    assert_eq!(
        snapshot.commitment_backed_collectives, 0,
        "a non-commons provide row with no household_id is correct-but-dormant (counts 0)"
    );
}
