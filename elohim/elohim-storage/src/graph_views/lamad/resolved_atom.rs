//! Lamad view builder — ResolvedAtomView
//!
//! Assembles the three-pillar graph projection for a single EPR CID.
//! Source of truth: CozoDB graph projection (Operational, Category C).
//! Reconstructed per request from DHT-canonical epr_atoms relational projection.

use crate::graph::engine::{GraphEngine, GraphError};
use crate::graph_views::data_value::*;
use cozo::DataValue;
use serde::{Deserialize, Serialize};

/// Three-pillar EPR atom view assembled from the graph projection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedAtomView {
    pub cid: String,
    pub slug: String,
    pub content_cid: String,
    pub version: i64,
    pub author_did: Option<String>,
    pub updated_at: String,
    pub lamad: LamadFields,
    pub shefa: Option<ShefaFields>,
    pub qahal: QahalFields,
}

/// Knowledge-pillar fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LamadFields {
    pub title: String,
    pub content_type: String,
    pub description: Option<String>,
    pub content_format: Option<String>,
    pub tags: Vec<String>,
}

/// Value-pillar fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShefaFields {
    pub stewards: Vec<String>,
    pub allocations: Vec<f64>,
}

/// Governance-pillar fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QahalFields {
    pub reach: Option<String>,
    pub layer: Option<String>,
    pub attestation_requirements: Vec<String>,
}

/// Build a `ResolvedAtomView` for the given CID from the graph projection.
///
/// Returns `GraphError::Schema` if the node or lamad/qahal rows are absent.
pub fn build(engine: &GraphEngine, cid: &str) -> Result<ResolvedAtomView, GraphError> {
    let node_result = engine.run_script(
        r#"?[slug, content_cid, version, author_did] :=
            *epr_node{cid: $cid, slug, content_cid, version, author_did}"#,
        &[("cid", DataValue::from(cid))],
    )?;
    let node_row = node_result
        .rows
        .first()
        .ok_or_else(|| GraphError::Schema(format!("no epr_node for cid {cid}")))?;

    let lamad_result = engine.run_script(
        r#"?[title, content_type, description, content_format, tags] :=
            *epr_lamad{cid: $cid, title, content_type, description, content_format, tags}"#,
        &[("cid", DataValue::from(cid))],
    )?;
    let lamad_row = lamad_result
        .rows
        .first()
        .ok_or_else(|| GraphError::Schema(format!("no epr_lamad for cid {cid}")))?;

    let shefa_result = engine.run_script(
        r#"?[stewards, allocations] :=
            *epr_shefa{cid: $cid, stewards, allocations}"#,
        &[("cid", DataValue::from(cid))],
    )?;

    let qahal_result = engine.run_script(
        r#"?[reach, layer, attestation_requirements] :=
            *epr_qahal{cid: $cid, reach, layer, attestation_requirements}"#,
        &[("cid", DataValue::from(cid))],
    )?;
    let qahal_row = qahal_result
        .rows
        .first()
        .ok_or_else(|| GraphError::Schema(format!("no epr_qahal for cid {cid}")))?;

    Ok(ResolvedAtomView {
        cid: cid.to_string(),
        slug: str_at(node_row, 0),
        content_cid: str_at(node_row, 1),
        version: i64_at(node_row, 2),
        author_did: opt_str_at(node_row, 3),
        // updated_at is a Validity [i64, bool] in the schema — return epoch seconds as string.
        // A full bitemporal projection would decode the Validity tuple; for now return empty.
        updated_at: String::new(),
        lamad: LamadFields {
            title: str_at(lamad_row, 0),
            content_type: str_at(lamad_row, 1),
            description: opt_str_at(lamad_row, 2),
            content_format: opt_str_at(lamad_row, 3),
            tags: list_str_at(lamad_row, 4),
        },
        shefa: shefa_result.rows.first().map(|s_row| ShefaFields {
            stewards: list_str_at(s_row, 0),
            allocations: list_f64_at(s_row, 1),
        }),
        qahal: QahalFields {
            reach: opt_str_at(qahal_row, 0),
            layer: opt_str_at(qahal_row, 1),
            attestation_requirements: list_str_at(qahal_row, 2),
        },
    })
}
