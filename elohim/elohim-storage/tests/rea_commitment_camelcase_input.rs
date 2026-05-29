//! Regression: `POST /api/v1/commitments` body deserialization must accept the
//! camelCase wire shape the seeder actually sends.
//!
//! ## Bug being fixed
//!
//! `api/rea_commitments.rs` does `parse_body::<CreateReaCommitmentInput>` — the
//! db struct is the direct HTTP boundary target. Before the
//! `#[serde(rename_all = "camelCase")]` fix, camelCase keys (`inScopeOf`,
//! `metadataJson`, ...) silently failed to match the snake_case fields and
//! dropped to `None`. Single-word fields (action/provider/receiver/note)
//! matched in both casings, masking the bug.
//!
//! The consequence: `in_scope_of` landed null on every project-epr projection;
//! `find_active_projections` filters on `in_scope_of LIKE 'doorway:X|%'` so it
//! matched nothing and `/lamad` returned 404 even with the projections present
//! in SQL. This test deserializes the *exact* payload
//! `genesis/seeder/src/seed-projections.ts` POSTs and asserts the routing
//! fields survive.

use elohim_storage::db::rea_commitments::CreateReaCommitmentInput;

#[test]
fn seed_projection_camelcase_payload_populates_in_scope_of_and_metadata() {
    // Mirrors seed-projections.ts createProjectionCommitment() output verbatim:
    // string `inScopeOf` (not array) + stringified `metadataJson` (not object).
    let body = serde_json::json!({
        "id": "project-epr-b8a51b1a1d8734c1",
        "action": "project-epr",
        "provider": "12D3KooW4ee8a8c90cf60ef4a959acb947ea3a8b1fca0d",
        "receiver": "12D3KooW4ee8a8c90cf60ef4a959acb947ea3a8b1fca0d",
        "inScopeOf": "doorway:alpha-elohim-host|epr:lamad-spa",
        "note": "Project lamad-spa at /lamad on alpha-elohim-host",
        "metadataJson": "{\"urlPath\":\"/lamad\",\"reach\":\"commons\",\"mode\":\"cached\"}"
    });

    let input: CreateReaCommitmentInput =
        serde_json::from_value(body).expect("seed camelCase body must deserialize");

    // The fields that were silently dropped before the rename_all fix:
    assert_eq!(
        input.in_scope_of.as_deref(),
        Some("doorway:alpha-elohim-host|epr:lamad-spa"),
        "inScopeOf must populate in_scope_of — find_active_projections filters on it"
    );
    assert!(
        input.metadata_json.is_some(),
        "metadataJson must populate metadata_json (carries urlPath/reach for routing)"
    );
    assert!(
        input
            .metadata_json
            .as_deref()
            .unwrap_or_default()
            .contains("/lamad"),
        "metadata_json must preserve the urlPath payload"
    );

    // Single-word fields were never affected — assert they still work post-rename.
    assert_eq!(input.action, "project-epr");
    assert_eq!(input.note.as_deref(), Some("Project lamad-spa at /lamad on alpha-elohim-host"));
}
