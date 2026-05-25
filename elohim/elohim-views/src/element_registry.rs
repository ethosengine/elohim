//! Element-Registry EPR types — per spec §3.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A manifest of custom elements a pillar exposes for cross-pillar embedding.
///
/// Stored as a Content row with `contentFormat: "element-registry-manifest"`.
/// Source of truth: `content` table (Holochain Content entry type on the elohim
/// DNA `content_store_integrity` zome). This view is a read-projection.
/// Category C — operational projection (no new DHT entry type).
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ElementRegistryView {
    /// EPR id of the registry itself (e.g. "elohim-core-elements").
    pub epr_id: String,
    /// Pillar this registry belongs to (e.g. "elohim-core", "lamad", "qahal").
    pub pillar: String,
    /// Elements published by this pillar.
    pub elements: Vec<ElementEntry>,
}

/// One custom element entry in a registry.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ElementEntry {
    /// Custom-element tag name (e.g. "elohim-button", "qahal-attestation-chip").
    pub tag_name: String,
    /// Content address of the element bundle (e.g. "sha256-...").
    pub cid: String,
    /// Semver version string (e.g. "1.0.0").
    pub version: String,
    /// ts-rs view types this element consumes as props.
    pub view_deps: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn element_registry_view_serializes_to_camel_case() {
        let view = ElementRegistryView {
            epr_id: "elohim-core-elements".into(),
            pillar: "elohim-core".into(),
            elements: vec![ElementEntry {
                tag_name: "elohim-button".into(),
                cid: "sha256-abc".into(),
                version: "1.0.0".into(),
                view_deps: vec![],
            }],
        };
        let json = serde_json::to_string(&view).unwrap();
        assert!(json.contains("\"eprId\":\"elohim-core-elements\""));
        assert!(json.contains("\"tagName\":\"elohim-button\""));
        assert!(json.contains("\"viewDeps\":[]"));
    }

    #[test]
    fn element_entry_serializes_all_fields() {
        let entry = ElementEntry {
            tag_name: "shefa-balance-card".into(),
            cid: "sha256-deadbeef".into(),
            version: "2.1.3".into(),
            view_deps: vec!["ShefaBalanceView".into(), "AccountView".into()],
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"tagName\":\"shefa-balance-card\""));
        assert!(json.contains("\"viewDeps\":[\"ShefaBalanceView\",\"AccountView\"]"));
    }
}
