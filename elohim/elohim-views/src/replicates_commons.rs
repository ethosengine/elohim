use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Typed view of a `replicates-commons` Commitment payload. Variant-tagged on
/// `variant`: `content` (pure provide of one commons EPR) or `capacity` (hosting
/// capacity offer to the commons). Mirrors `replicates_dwelling::RatioAttestation`
/// style. Source of truth: Holochain DHT (Mishpat Commitment, action discriminator).
/// Spec: genesis/docs/superpowers/specs/2026-06-08-epr-acquisition-slice2b-provide-loop-design.md §4.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "variant", rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum ReplicatesCommonsPayload {
    // Per-variant `rename_all` is required: the enum-level `rename_all` only
    // camelCases the variant *tag* values, not the fields *inside* each variant.
    // Without this the boundary rule (snake_case never leaves Rust) would be
    // violated — `head_ref`/`commons_bytes` would leak as snake_case.
    #[serde(rename = "content", rename_all = "camelCase")]
    Content {
        head_ref: String,
        closure_rule: Option<String>,
        reach: String,
        bounds: CommonsBounds,
    },
    #[serde(rename = "capacity", rename_all = "camelCase")]
    Capacity {
        commons_bytes: u64,
        bounds: CommonsBounds,
        ratio_attestation: CommonsRatioAttestation,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CommonsBounds {
    pub rate_per_minute: u32,
    pub reach_ceiling: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CommonsRatioAttestation {
    pub commons_pct: u8,
    pub dwelling_pct: u8,
    pub collective_pct: u8,
    pub free_pct: u8,
    pub effective_ratio_cid: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_variant_round_trips_with_variant_tag() {
        let p = ReplicatesCommonsPayload::Content {
            head_ref: "bafyhead".into(),
            closure_rule: Some("transitive-1".into()),
            reach: "commons".into(),
            bounds: CommonsBounds {
                rate_per_minute: 30,
                reach_ceiling: "commons".into(),
            },
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"variant\":\"content\""), "json was: {json}");
        assert!(
            json.contains("\"headRef\":\"bafyhead\""),
            "camelCase headRef; json was: {json}"
        );
        let back: ReplicatesCommonsPayload = serde_json::from_str(&json).unwrap();
        matches!(back, ReplicatesCommonsPayload::Content { .. });
    }

    #[test]
    fn capacity_variant_round_trips_with_variant_tag() {
        let p = ReplicatesCommonsPayload::Capacity {
            commons_bytes: 50_000_000_000,
            bounds: CommonsBounds {
                rate_per_minute: 30,
                reach_ceiling: "commons".into(),
            },
            ratio_attestation: CommonsRatioAttestation {
                commons_pct: 20,
                dwelling_pct: 40,
                collective_pct: 25,
                free_pct: 15,
                effective_ratio_cid: "bafkrei-x".into(),
            },
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(
            json.contains("\"variant\":\"capacity\""),
            "json was: {json}"
        );
        assert!(
            json.contains("\"commonsBytes\":50000000000"),
            "camelCase commonsBytes; json was: {json}"
        );
        let back: ReplicatesCommonsPayload = serde_json::from_str(&json).unwrap();
        matches!(back, ReplicatesCommonsPayload::Capacity { .. });
    }
}
