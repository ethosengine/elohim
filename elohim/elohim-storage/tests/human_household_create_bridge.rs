//! Create-surface bridge for `humans.household_id`.
//!
//! The resilience snapshot's commitment-backing join requires
//! `humans.agent_pub_key` + `humans.household_id IS NOT NULL`, but nothing
//! populated `household_id` through the create path — the DHT humans-replayer
//! is a stub and `household_backfill` only fills already-NULL rows from an
//! external mapping. These tests pin the short-term explicit bridge: a human
//! created through the HTTP input path (`CreateHumanInputView`) with both
//! `agentPubKey` and `householdId` lands both columns, satisfies the snapshot
//! junction, and is left untouched by the backfill pass.

use diesel::prelude::*;

use elohim_storage::db;
use elohim_storage::db::humans::{create_human, CreateHumanInput};
use elohim_storage::db::models::NewReaCommitment;
use elohim_storage::services::{household_backfill, household_resilience};
use elohim_storage::test_util::test_pool;
use elohim_storage::views::{CreateHumanInputView, HumanView};

/// Mirror the exact `CreateHumanInputView -> CreateHumanInput` mapping that the
/// `register_human` HTTP handler performs (api/identity.rs). Keeping this
/// inline (rather than reaching into the private handler, which takes a hyper
/// `Request`) exercises the same input-path boundary: camelCase view in,
/// snake_case db input out, household_id threaded.
fn input_path_create(
    conn: &mut diesel::SqliteConnection,
    body: CreateHumanInputView,
) -> elohim_storage::db::models::Human {
    let affinities_json = serde_json::to_string(&body.affinities).unwrap();
    let input = CreateHumanInput {
        id: body.id,
        agent_pub_key: body.agent_pub_key,
        display_name: body.display_name,
        bio: body.bio,
        affinities: affinities_json,
        profile_reach: body.profile_reach,
        location: body.location,
        profile_photo_url: body.profile_photo_url,
        h_app_id: "lamad".to_string(),
        household_id: body.household_id,
    };
    create_human(conn, input).unwrap()
}

fn view_with_household(id: &str, agent: &str, household: &str) -> CreateHumanInputView {
    CreateHumanInputView {
        id: id.into(),
        agent_pub_key: Some(agent.into()),
        display_name: id.into(),
        bio: None,
        affinities: vec![],
        profile_reach: "commons".into(),
        location: None,
        profile_photo_url: None,
        household_id: Some(household.into()),
    }
}

/// (a) A human created through the HTTP input path with `agentPubKey` +
/// `householdId` lands with BOTH columns set, and the output `HumanView`
/// exposes them for read-back.
#[test]
fn input_path_sets_agent_key_and_household_id() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    let human = input_path_create(
        &mut conn,
        view_with_household("h-matthew", "agent-matthew", "household-dowell"),
    );

    assert_eq!(human.agent_pub_key.as_deref(), Some("agent-matthew"));
    assert_eq!(human.household_id.as_deref(), Some("household-dowell"));

    // Read-back: the row fetched by id carries household_id, and the output
    // view surfaces it (snapshot read path reads humans rows; clients read it
    // through HumanView).
    let fetched = db::humans::get_human_by_id(&mut conn, "h-matthew")
        .unwrap()
        .unwrap();
    let view = HumanView::from(fetched);
    assert_eq!(view.agent_pub_key.as_deref(), Some("agent-matthew"));
    assert_eq!(view.household_id.as_deref(), Some("household-dowell"));
}

/// (b) A human created with `agentPubKey` + `householdId` via the input path,
/// plus an active provide commitment whose provider == that agent key and
/// scope == `content:<reach>`, satisfies the resilience-snapshot junction:
/// `commitment_backed_collectives >= 1`.
///
/// This is the load-bearing assertion — before this bridge, the create path
/// always wrote `household_id = NULL`, so the snapshot's
/// `humans.household_id IS NOT NULL` filter dropped every provider and the
/// count was always 0.
#[test]
fn input_path_human_satisfies_snapshot_commitment_junction() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    // Create the provider human through the input path with both columns set.
    let provider_key = "agent-provider";
    input_path_create(
        &mut conn,
        view_with_household("h-provider", provider_key, "household-provider"),
    );

    // An active provide commitment scoped to content:commons, provided by the
    // same agent key the human carries. (The snapshot's content-reach lookup
    // falls back to "commons" when no content row exists, so scope is
    // "content:commons" — matching the D3 fixtures in household_resilience.rs.)
    diesel::insert_into(db::diesel_schema::rea_commitments::table)
        .values(&NewReaCommitment {
            id: "commit-provide-1",
            h_app_id: "lamad",
            action: "provide",
            provider: provider_key,
            receiver: "commons",
            resource_conforms_to: None,
            resource_classified_as: Some("content:commons"),
            resource_quantity_value: None,
            resource_quantity_unit: None,
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_beginning: None,
            has_end: None,
            due: None,
            clause_of: None,
            in_scope_of: None,
            medium_of_exchange_id: None,
            state: "active",
            finished: 0,
            note: None,
            metadata_json: None,
            dht_anchor_hash: None,
        })
        .execute(&mut conn)
        .unwrap();

    let ctx = elohim_storage::db::AppContext {
        h_app_id: "lamad".into(),
        local_libp2p_peer_id: None,
    };

    // Drive the snapshot for any content id (commitment-backing counts
    // households over provide+active+content:<reach> rows; it does not depend
    // on this content having a shard manifest).
    let snapshot = household_resilience::snapshot(&pool, &ctx, "content-anything", None).unwrap();

    assert!(
        snapshot.commitment_backed_collectives >= 1,
        "input-path human (agent={provider_key}, household=household-provider) with an \
         active provide content:commons commitment must satisfy the snapshot junction; \
         got commitment_backed_collectives={}",
        snapshot.commitment_backed_collectives
    );
}

/// (c) Backfill-composition: a row created WITH household_id through the input
/// path is left untouched by `household_backfill::run_once`, even when the DHT
/// mapping proposes a different household for that id.
#[test]
fn backfill_does_not_overwrite_input_path_household_id() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    input_path_create(
        &mut conn,
        view_with_household("h-set", "agent-set", "household-original"),
    );

    // A DHT mapping that would change the household — backfill must ignore it
    // because the row's household_id is already non-NULL.
    let mapping = vec![("h-set".to_string(), "household-different".to_string())];
    let filled = household_backfill::run_once(&pool, mapping).unwrap();
    assert_eq!(filled, 0, "no NULL rows to fill — set value is sticky");

    let after = db::humans::get_human_by_id(&mut conn, "h-set")
        .unwrap()
        .unwrap();
    assert_eq!(
        after.household_id.as_deref(),
        Some("household-original"),
        "backfill must not overwrite a household_id set at create-time"
    );
}
