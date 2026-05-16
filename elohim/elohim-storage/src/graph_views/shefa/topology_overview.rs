//! Shefa view builder — TopologyOverviewView (fully graph-native)
//!
//! Rolls up households, collectives, and inbound reciprocity for a contributor DID.
//! Source of truth: CozoDB graph projection via MEMBER_OF + RECIPROCATES_WITH edges
//! (Operational, Category C). Fully graph-backed — no relational composition needed.
//!
//! This builder produces the NEW `TopologyOverviewView` shape (from Task 22 schema),
//! not one of the existing view structs.

use crate::graph::engine::{GraphEngine, GraphError};
use crate::graph_views::data_value::*;
use cozo::DataValue;
use serde::{Deserialize, Serialize};

/// Rolled-up topology for a single contributor DID.
///
/// Wire format: `topology-overview-view.schema.json`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopologyOverviewView {
    pub contributor_did: String,
    pub households: Vec<HouseholdEntry>,
    pub collectives: Vec<CollectiveEntry>,
    pub reciprocity_inbound: Vec<ReciprocityInboundEntry>,
}

/// A household the contributor belongs to.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HouseholdEntry {
    pub household_cid: String,
    pub members: Vec<String>,
    pub device_count: i64,
}

/// A collective the contributor is a member of.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectiveEntry {
    pub collective_cid: String,
    pub member_count: i64,
}

/// An inbound reciprocity edge.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReciprocityInboundEntry {
    pub from_did: String,
    pub weight: f64,
}

/// Build a `TopologyOverviewView` for the given `contributor_did`.
///
/// Queries:
/// - MEMBER_OF edges from contributor to household/collective nodes
/// - Co-members in each household (same MEMBER_OF target)
/// - OPERATES_DEVICE count per household
/// - Inbound RECIPROCATES_WITH edges to the contributor
pub fn build(
    engine: &GraphEngine,
    contributor_did: &str,
) -> Result<TopologyOverviewView, GraphError> {
    // Find households (MEMBER_OF targets) and their member counts + device counts.
    let household_result = engine.run_script(
        r#"?[household_cid] :=
            *epr_edge{from_cid: $contributor, to_cid: household_cid, rel_type: 'MEMBER_OF'}"#,
        &[("contributor", DataValue::from(contributor_did))],
    )?;

    let mut households: Vec<HouseholdEntry> = Vec::new();
    for household_row in &household_result.rows {
        let household_cid = str_at(household_row, 0);

        // Co-members in this household.
        let members_result = engine.run_script(
            r#"?[member_cid] :=
                *epr_edge{from_cid: member_cid, to_cid: $household, rel_type: 'MEMBER_OF'}"#,
            &[("household", DataValue::from(household_cid.as_str()))],
        )?;
        let members: Vec<String> = members_result.rows.iter().map(|r| str_at(r, 0)).collect();

        // Devices operated by any member in this household.
        let device_result = engine.run_script(
            r#"?[device_cid] :=
                *epr_edge{from_cid: member, to_cid: $household, rel_type: 'MEMBER_OF'},
                *epr_edge{from_cid: member, to_cid: device_cid, rel_type: 'OPERATES_DEVICE'}"#,
            &[("household", DataValue::from(household_cid.as_str()))],
        )?;
        let device_count = device_result.rows.len() as i64;

        households.push(HouseholdEntry {
            household_cid,
            members,
            device_count,
        });
    }

    // Find all distinct MEMBER_OF targets and their member counts.
    // CozoDB aggregation: `count(v)` in query head; groups by remaining bound vars automatically.
    let collective_result = engine.run_script(
        r#"?[collective_cid, count(member)] :=
            *epr_edge{from_cid: member, to_cid: collective_cid, rel_type: 'MEMBER_OF'}"#,
        &[],
    )?;

    let households_set: std::collections::HashSet<String> =
        households.iter().map(|h| h.household_cid.clone()).collect();
    let collectives: Vec<CollectiveEntry> = collective_result
        .rows
        .iter()
        .filter_map(|row| {
            let cid = str_at(row, 0);
            // Skip nodes already counted as households in this contributor's perspective.
            if households_set.contains(&cid) {
                return None;
            }
            let member_count = i64_at(row, 1);
            Some(CollectiveEntry {
                collective_cid: cid,
                member_count,
            })
        })
        .collect();

    // Inbound RECIPROCATES_WITH edges.
    let reciprocity_result = engine.run_script(
        r#"?[from_did] :=
            *epr_edge{from_cid: from_did, to_cid: $contributor, rel_type: 'RECIPROCATES_WITH'}"#,
        &[("contributor", DataValue::from(contributor_did))],
    )?;
    let reciprocity_inbound: Vec<ReciprocityInboundEntry> = reciprocity_result
        .rows
        .iter()
        .map(|row| ReciprocityInboundEntry {
            from_did: str_at(row, 0),
            weight: 1.0, // Uniform weight — weighted reciprocity requires REA event accounting
        })
        .collect();

    Ok(TopologyOverviewView {
        contributor_did: contributor_did.to_string(),
        households,
        collectives,
        reciprocity_inbound,
    })
}
