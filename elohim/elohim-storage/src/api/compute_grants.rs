//! Explicit provider consent for the existing native compute delegation.
//! A grant and its consent are existing Class-A Commitment/EconomicEvent records;
//! activation is a Class-A2 CommitmentByState link. SQL is only their projection.
//! Caller holds the local compute capability, verified performer and MUTATIONS lock.
//! Read-before-write preserves exact actions across retries on this local issuer;
//! independent direct-conductor writers are outside that serialization guarantee.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use holochain_types::prelude::*;
use serde_json::{json, Value};

use crate::{
    db::{AppContext, DbPool},
    error::StorageError,
    hc_client::HcClient,
    services::{
        commitment_record::CommitmentRecordPins,
        conductor_writes::{self as native, CreateMishpatCommitmentInput},
    },
};

fn invalid(message: &str) -> StorageError {
    StorageError::InvalidInput(format!("compute grant: {message}"))
}

fn stamp(input: &Value, field: &str) -> Result<DateTime<Utc>, StorageError> {
    let raw = input[field]
        .as_str()
        .ok_or_else(|| invalid("issuedAt, validFrom and validUntil are required"))?;
    DateTime::parse_from_rfc3339(raw)
        .map(|t| t.with_timezone(&Utc))
        .map_err(|_| invalid("timestamps must be RFC3339"))
}

/// The delegated-scope vocabulary this surface will notarize. A caller NAMES a
/// scope, it does not invent one: every accepted value is a `&'static str` from
/// this list, so no caller-supplied bytes ever reach the notarized payload.
/// Extending the protocol's social vocabulary is an entry in this list plus its
/// consumer, never a new entry type — the commitment stays the existing
/// `delegates-compute` Mishpat entry and the DNA hash does not move.
const GRANT_SCOPES: [&str; 2] = ["sweettest-feedback", "hosted-cell"];

/// The scope a grant request names, or the historical default.
///
/// Pure. An absent scope answers `"sweettest-feedback"` so every caller that
/// predates the field notarizes byte-identical payload bytes; a named scope is
/// admitted only when it is one of [`GRANT_SCOPES`]; anything else is refused
/// rather than silently dropped (dropping is what made every grant this surface
/// ever issued read as `sweettest-feedback`, whatever it was actually for).
fn grant_scope(input: &Value) -> Result<&'static str, StorageError> {
    let Some(named) = input.get("scope") else {
        return Ok(GRANT_SCOPES[0]);
    };
    let named = named
        .as_str()
        .ok_or_else(|| invalid("scope must be a string"))?;
    GRANT_SCOPES
        .iter()
        .find(|known| **known == named)
        .copied()
        .ok_or_else(|| invalid("unknown grant scope"))
}

fn grant_input(
    provider: &str,
    input: &Value,
    now: DateTime<Utc>,
) -> Result<CreateMishpatCommitmentInput, StorageError> {
    let fields = input
        .as_object()
        .ok_or_else(|| invalid("expected object"))?;
    if fields.keys().any(|k| {
        !matches!(
            k.as_str(),
            "recipient" | "scope" | "issuedAt" | "validFrom" | "validUntil" | "bounds"
        )
    }) {
        return Err(invalid(
            "unknown grant field; provider is server-owned and scope must be named from the accepted list",
        ));
    }
    let delegated = grant_scope(input)?;
    let recipient = input["recipient"]
        .as_str()
        .ok_or_else(|| invalid("recipient required"))?;
    AgentPubKey::try_from(recipient)
        .map_err(|_| invalid("recipient must be a Holochain agent key"))?;
    let issued = stamp(input, "issuedAt")?;
    let from = stamp(input, "validFrom")?;
    let until = stamp(input, "validUntil")?;
    if issued > now || from >= until || until <= now || issued >= until {
        return Err(invalid(
            "grant requires an unexpired finite window and non-future issuedAt",
        ));
    }
    let bounds = input["bounds"]
        .as_object()
        .ok_or_else(|| invalid("explicit bounds required"))?;
    // Narrow supported subset of delegates-compute.schema.json. Do not accept
    // extra controls that the launch validator would silently fail to enforce.
    if bounds.keys().any(|k| {
        !matches!(
            k.as_str(),
            "epr_scope" | "reach_ceiling" | "rate_per_hour" | "rotation_ttl_days"
        )
    }) {
        return Err(invalid("unsupported compute bound"));
    }
    let scope = input["bounds"]["epr_scope"]
        .as_array()
        .ok_or_else(|| invalid("epr_scope required"))?;
    if scope.is_empty()
        || scope.len() > 64
        || scope
            .iter()
            .any(|v| v.as_str().is_none_or(|s| s.is_empty() || s.len() > 256))
    {
        return Err(invalid(
            "epr_scope requires 1..64 explicit task CIDs or '*'",
        ));
    }
    for value in scope {
        let target = value.as_str().expect("checked scope string");
        if target != "*" && target.parse::<cid::Cid>().is_err() {
            return Err(invalid("epr_scope must name task CIDs or explicit '*'"));
        }
    }
    let rate = input["bounds"]["rate_per_hour"]
        .as_u64()
        .filter(|n| *n > 0 && *n <= u32::MAX as u64);
    let days = input["bounds"]["rotation_ttl_days"]
        .as_u64()
        .filter(|n| *n > 0 && *n <= u32::MAX as u64);
    if rate.is_none() || days.is_none() || input["bounds"]["reach_ceiling"] != "commons" {
        return Err(invalid("positive finite rate_per_hour and rotation_ttl_days, and commons reach_ceiling required"));
    }
    if (until - from).num_seconds() > days.expect("checked rotation days") as i64 * 86400 {
        return Err(invalid(
            "validity window exceeds explicit rotation lifetime",
        ));
    }
    let payload = json!({
        "action":"delegates-compute", "scope":delegated,
        "provider":provider, "recipient":recipient, "bounds":input["bounds"],
        "valid_from":from.to_rfc3339(), "valid_until":until.to_rfc3339()
    });
    Ok(CreateMishpatCommitmentInput {
        action: "delegates-compute".into(),
        payload_json: payload.to_string(),
        signed_at: issued.to_rfc3339(),
    })
}

/// Reuse Holochain's canonical app-entry encoding and hash, never a parallel CID encoder.
fn grant_hash(input: &CreateMishpatCommitmentInput) -> Result<EntryHash, StorageError> {
    let bytes = rmp_serde::to_vec_named(input).map_err(|e| invalid(&e.to_string()))?;
    let app = AppEntryBytes::try_from(SerializedBytes::from(UnsafeBytes::from(bytes)))
        .map_err(|e| invalid(&e.to_string()))?;
    Ok(EntryHash::with_data_sync(&Entry::App(app)))
}

/// Fixed number of bounded native calls, no polling/retry loop or implicit grants.
pub async fn issue(
    hc: &Arc<HcClient>,
    pool: &DbPool,
    input: &Value,
) -> Result<Value, StorageError> {
    let provider = hc.agent_key_uhcak();
    let delegated = grant_scope(input)?;
    let grant = grant_input(&provider, input, Utc::now())?;
    let entry_hash = grant_hash(&grant)?;
    let cid = entry_hash.to_string();
    // The generic native read may choose a foreign duplicate Create. We refuse
    // that answer below rather than treating matching payload as provider consent.
    let action_hash = match native::get_commitment(hc, &cid).await? {
        Some(existing) => existing.action_hash,
        None => {
            let bytes = native::call_create_commitment(hc, grant.clone()).await?;
            let created: native::CreateCommitmentOutput =
                rmp_serde::from_slice(&bytes).map_err(|e| invalid(&e.to_string()))?;
            if created.entry_hash != entry_hash {
                return Err(invalid("native grant entry differs from submitted content"));
            }
            created.action_hash
        }
    };
    let authenticated = native::get_authenticated_commitment_record(
        hc,
        CommitmentRecordPins {
            action_hash: action_hash.clone(),
            entry_hash: entry_hash.clone(),
            author: AgentPubKey::from_raw_39(hc.agent_pub_key()),
        },
    )
    .await?
    .ok_or_else(|| invalid("signed grant unavailable"))?;
    let content = authenticated.content();
    if content.action != grant.action
        || content.payload_json != grant.payload_json
        || content.signed_at != grant.signed_at
    {
        return Err(invalid("existing grant differs from explicit consent"));
    }
    let states = native::get_commitment_authority_links(hc, &cid).await?;
    if states.iter().any(|s| {
        s.author == provider && matches!(s.state.as_str(), "revoked" | "cancelled" | "sunset")
    }) {
        return Err(invalid("withdrawn grant cannot be reactivated"));
    }
    {
        let mut conn = pool.get().map_err(|e| invalid(&e.to_string()))?;
        if let Some(row) = crate::db::mishpat_commitments::get_by_cid(&mut conn, &cid)? {
            if row.revoked_at.is_some()
                || matches!(row.state.as_str(), "revoked" | "cancelled" | "sunset")
            {
                return Err(invalid(
                    "withdrawn projection requires reconciliation, never reactivation",
                ));
            }
        }
    }

    let metadata =
        json!({"grant_cid":cid,"grant_action_hash":action_hash.to_string(),"scope":delegated});
    let consent_input: shefa_types::CreateReaEconomicEventInput = serde_json::from_value(json!({
        "id":format!("compute-consent:{cid}"), "action":"accept", "provider":provider,
        "receiver":input["recipient"], "has_point_in_time":grant.signed_at,
        "metadata_json":metadata.to_string()
    }))?;
    let query = rmp_serde::to_vec_named(&consent_input.id).map_err(|e| invalid(&e.to_string()))?;
    let bytes = hc
        .call_zome("content_store", "get_rea_economic_event", query)
        .await?;
    let existing: Option<shefa_types::ReaEconomicEventOutput> =
        rmp_serde::from_slice(&bytes).map_err(|e| invalid(&e.to_string()))?;
    let consent = match existing {
        Some(event) => event,
        None => rmp_serde::from_slice(
            &native::call_create_rea_economic_event(hc, &consent_input).await?,
        )
        .map_err(|e| invalid(&e.to_string()))?,
    };
    // Recover the actual signed record, not a provider string supplied by a peer.
    let carried = native::call_get_record_for_action(hc, &consent.action_hash.to_string())
        .await?
        .ok_or_else(|| invalid("native consent record unavailable"))?;
    let event = verify_consent(&carried.record, &consent, &consent_input, hc)?;
    let event_hash = consent.action_hash.to_string();
    if !states
        .iter()
        .any(|s| s.author == provider && s.state == "active" && s.event_hash == event_hash)
    {
        native::call_create_commitment_state_link(
            hc,
            native::CreateCommitmentStateLinkInput {
                commitment_cid: cid.clone(),
                state: "active".into(),
                event_hash: event_hash.clone(),
                signed_at: grant.signed_at.clone(),
            },
        )
        .await?;
    }
    let confirmed = native::get_commitment_authority_links(hc, &cid).await?;
    if confirmed.iter().any(|s| {
        s.author == provider && matches!(s.state.as_str(), "revoked" | "cancelled" | "sunset")
    }) || !confirmed
        .iter()
        .any(|s| s.author == provider && s.state == "active" && s.event_hash == event_hash)
    {
        return Err(invalid("provider activation not confirmed"));
    }
    // Eagerly project only authenticated content, after native activation. The
    // consent does not carry bounded_by: it grants authority, it does not spend it.
    let mut row = match crate::mishpat_projection::parse_commitment_payload(
        &content.action,
        &content.payload_json,
        &cid,
        &action_hash.to_string(),
    )
    .map_err(|e| invalid(&e))?
    {
        crate::mishpat_projection::CommitmentProjection::Upsert(row) => row,
        _ => return Err(invalid("grant projection mismatch")),
    };
    row.state = "active".into();
    {
        let mut conn = pool.get().map_err(|e| invalid(&e.to_string()))?;
        crate::db::mishpat_commitments::upsert_with_anchor(&mut conn, row)?;
    }
    let signal = serde_json::from_value(json!({"type":"ReaEconomicEventCommitted", "payload":{
        "action_hash":event_hash, "entry_hash":consent.entry_hash.to_string(),
        "author":provider, "event":event
    }}))?;
    crate::rea_projection::handle_rea_signal(signal, pool, &AppContext::default())?;
    Ok(
        json!({"grantCid":cid,"grantActionHash":action_hash.to_string(),
        "consentActionHash":event_hash,"provider":provider,"recipient":input["recipient"],"state":"active"}),
    )
}

fn verify_consent(
    bytes: &[u8],
    output: &shefa_types::ReaEconomicEventOutput,
    expected: &shefa_types::CreateReaEconomicEventInput,
    hc: &HcClient,
) -> Result<Value, StorageError> {
    use ed25519_dalek::VerifyingKey;
    if bytes.len() > crate::services::commitment_record::MAX_COMMITMENT_RECORD_BYTES {
        return Err(invalid("consent record exceeds limit"));
    }
    let record: Record = rmp_serde::from_slice(bytes).map_err(|e| invalid(&e.to_string()))?;
    let author = AgentPubKey::from_raw_39(hc.agent_pub_key());
    if record.action().author() != &author
        || ActionHash::with_data_sync(record.action()) != output.action_hash
    {
        return Err(invalid("consent action or author mismatch"));
    }
    let ActionData::Create(create) = &record.action().data else {
        return Err(invalid("consent requires Create"));
    };
    let EntryType::App(definition) = &create.entry_type else {
        return Err(invalid("consent requires an app entry"));
    };
    // lamad's sole integrity zome; EntryTypes::EconomicEvent is index 26.
    if definition.zome_index != ZomeIndex(0)
        || definition.entry_index != EntryDefIndex(26)
        || definition.visibility != EntryVisibility::Public
    {
        return Err(invalid("consent must be the native EconomicEvent entry"));
    }
    let Some(entry @ Entry::App(app)) = record.entry().as_option() else {
        return Err(invalid("consent entry missing"));
    };
    if EntryHash::with_data_sync(entry) != output.entry_hash
        || create.entry_hash != output.entry_hash
    {
        return Err(invalid("consent entry hash mismatch"));
    }
    let key_bytes: [u8; 32] = author
        .get_raw_32()
        .try_into()
        .map_err(|_| invalid("invalid provider key"))?;
    let key = VerifyingKey::from_bytes(&key_bytes).map_err(|_| invalid("invalid provider key"))?;
    let sig = ed25519_dalek::Signature::from_slice(record.signature().as_ref())
        .map_err(|_| invalid("invalid consent signature"))?;
    key.verify_strict(
        &encode(record.action()).map_err(|e| invalid(&e.to_string()))?,
        &sig,
    )
    .map_err(|_| invalid("consent signature mismatch"))?;
    let event: shefa_types::EconomicEvent =
        rmp_serde::from_slice(app.bytes()).map_err(|e| invalid(&e.to_string()))?;
    if event.id != expected.id
        || event.action != expected.action
        || event.provider != expected.provider
        || event.receiver != expected.receiver
        || event.has_point_in_time != expected.has_point_in_time
        || event.metadata_json != expected.metadata_json.as_deref().unwrap_or("")
    {
        return Err(invalid("existing consent differs from explicit grant"));
    }
    Ok(serde_json::to_value(event)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> Value {
        json!({"recipient":AgentPubKey::from_raw_32(vec![3;32]).to_string(),
            "issuedAt":"2026-09-07T00:00:00Z", "validFrom":"2026-09-07T00:00:00Z",
            "validUntil":"2026-09-08T00:00:00Z",
            "bounds":{"epr_scope":["*"],"reach_ceiling":"commons","rate_per_hour":2,"rotation_ttl_days":1}})
    }
    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-09-07T01:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn explicit_window_and_bounds_are_required_without_defaults() {
        for pointer in ["issuedAt", "validFrom", "validUntil", "bounds", "recipient"] {
            let mut raw = input();
            raw.as_object_mut().unwrap().remove(pointer);
            assert!(grant_input("provider", &raw, now()).is_err(), "{pointer}");
        }
        for (field, value) in [
            ("rate_per_hour", json!(0)),
            ("rotation_ttl_days", json!(0)),
            ("reach_ceiling", json!("public")),
            ("epr_scope", json!([])),
        ] {
            let mut raw = input();
            raw["bounds"][field] = value;
            assert!(grant_input("provider", &raw, now()).is_err(), "{field}");
        }
    }

    #[test]
    fn exact_retry_hash_is_stable_and_changed_consent_is_new_content() {
        let first = grant_input("provider", &input(), now()).unwrap();
        let second = grant_input("provider", &input(), now() + chrono::Duration::hours(1)).unwrap();
        assert_eq!(grant_hash(&first).unwrap(), grant_hash(&second).unwrap());
        let mut changed = input();
        changed["bounds"]["rate_per_hour"] = json!(3);
        assert_ne!(
            grant_hash(&first).unwrap(),
            grant_hash(&grant_input("provider", &changed, now()).unwrap()).unwrap()
        );
        let schema: Value = serde_json::from_str(include_str!(
            "../../../sdk/schemas/v1/commitments/delegates-compute.schema.json"
        ))
        .unwrap();
        let payload: Value = serde_json::from_str(&first.payload_json).unwrap();
        assert!(jsonschema::validator_for(&schema)
            .unwrap()
            .is_valid(&payload));
    }

    #[test]
    fn a_named_scope_reaches_the_notarized_payload() {
        let mut raw = input();
        raw["scope"] = json!("hosted-cell");
        let grant = grant_input("provider", &raw, now()).expect("accepted");
        let payload: Value = serde_json::from_str(&grant.payload_json).unwrap();
        assert_eq!(payload["scope"], "hosted-cell");
    }

    #[test]
    fn an_unnamed_scope_still_defaults_to_the_sweettest_feedback_lane() {
        // existing callers pass no scope; their notarized payload must not change
        let grant = grant_input("provider", &input(), now()).expect("accepted");
        let payload: Value = serde_json::from_str(&grant.payload_json).unwrap();
        assert_eq!(payload["scope"], "sweettest-feedback");
    }

    #[test]
    fn an_unknown_scope_value_is_refused() {
        let mut raw = input();
        raw["scope"] = json!("whatever-i-want");
        assert!(grant_input("provider", &raw, now()).is_err());
    }

    #[test]
    fn expired_extended_and_caller_owned_authority_are_refused() {
        for (field, value) in [
            ("issuedAt", json!("2026-09-09T00:00:00Z")),
            ("validUntil", json!("2026-09-07T00:30:00Z")),
            ("validUntil", json!("2026-09-10T00:00:00Z")),
            ("provider", json!("another-agent")),
            ("scope", json!("operator-reconcile")),
        ] {
            let mut raw = input();
            raw[field] = value;
            assert!(grant_input("provider", &raw, now()).is_err(), "{field}");
        }
    }
}
