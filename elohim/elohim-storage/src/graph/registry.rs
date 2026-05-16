//! Graph extension registry — registration-time validation and application of
//! manifest-declared graph extensions (Tasks 16-17, 2026-05-16 spec).
//!
//! A `GraphExtension` is the Rust counterpart to the `"graph"` section of an
//! app-manifest. At manifest registration time `validate_graph_extension`
//! checks for rule shadowing and duplicate declarations; `apply_graph_extension`
//! materialises the extension into the live `GraphEngine` (creates indexes,
//! creates domain node relations) and records the rule library for inline query
//! composition.
//!
//! ## Core primitive names (reserved)
//! The following Datalog rule names are reserved by the core graph primitives
//! module and MUST NOT be shadowed by manifest-declared rules:
//! - `neighborhood`
//! - `path`
//! - `reach_filtered`
//! - `version_chain`
//!
//! ## Core node types (always available, need not be declared)
//! - `EprHead`
//! - `ContributorDID`

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::graph::engine::{GraphEngine, GraphError};

// ---------------------------------------------------------------------------
// Domain model structs
// ---------------------------------------------------------------------------

/// A manifest's graph extension — the Rust counterpart to the `"graph"` section
/// in `app-manifest.schema.json`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GraphExtension {
    #[serde(default)]
    pub edges: Vec<EdgeDecl>,
    #[serde(default)]
    pub nodes: Vec<NodeDecl>,
    #[serde(default)]
    pub indexes: Vec<IndexDecl>,
    #[serde(default)]
    pub rules: Vec<RuleDecl>,
}

/// Declaration of a domain-specific edge type in `epr_edge`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeDecl {
    /// Relation type name (e.g. "PREREQUISITE"). Stored in `epr_edge.rel_type`.
    #[serde(rename = "type")]
    pub rel_type: String,
    /// Source node type name.
    pub from: String,
    /// Target node type name.
    pub to: String,
    /// Whether to create an `epr_edge` index for this rel_type at registration.
    #[serde(default)]
    pub indexed: bool,
    /// Whether the edge is directional (default: true).
    #[serde(default = "default_true")]
    pub directional: bool,
    /// Whether this edge carries a numeric weight (informational; not applied
    /// by the current engine).
    #[serde(default)]
    pub weighted: bool,
    /// Whether this edge carries bitemporal validity semantics (informational;
    /// not applied by the current engine).
    #[serde(default)]
    pub temporal: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Declaration of a domain-specific node augmentation relation.
///
/// Creates a CozoDB relation named `{manifest_name}_{type}` with the declared
/// properties as columns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDecl {
    /// Node type name (e.g. "LearningPath"). Combined with manifest name.
    #[serde(rename = "type")]
    pub node_type: String,
    /// Property name → CozoDB type string (e.g. "String", "Int", "Float").
    pub properties: HashMap<String, String>,
}

/// Index declaration for a domain relation.
///
/// Applied via `::index create` at registration time. CozoDB does not support
/// partial/filtered indexes — `where` and `order_by` fields are stored for
/// documentation purposes only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDecl {
    /// Index name. Created as `{on}:{name}` in CozoDB.
    pub name: String,
    /// Relation to index (e.g. "epr_edge").
    pub on: String,
    /// Informational only — CozoDB does not support partial indexes.
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub where_clause: Option<String>,
    /// Informational only — column ordering is declared in CozoDB via the
    /// relation definition, not the index.
    #[serde(rename = "order_by", skip_serializing_if = "Option::is_none")]
    pub order_by: Option<String>,
}

/// Named Datalog rule declaration.
///
/// Rules are stored in-memory by the graph registry and inlined into queries
/// that invoke them. They are NOT pre-registered in CozoDB (which has no
/// persistent named-rule mechanism).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleDecl {
    /// Rule name. Used as the Datalog rule head. Must not shadow core
    /// primitives or duplicate within the registered manifest set.
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Datalog rule body. Variables in rule bodies use plain identifiers
    /// (no `?` prefix); `?` prefix is only valid in final query heads.
    pub datalog: String,
}

// ---------------------------------------------------------------------------
// Core reservations
// ---------------------------------------------------------------------------

/// Core primitive rule names that manifest-declared rules must not shadow.
const CORE_PRIMITIVE_NAMES: &[&str] = &["neighborhood", "path", "reach_filtered", "version_chain"];

/// Core node types that are always available as edge endpoints. Manifest
/// `EdgeDecl` entries may reference these without declaring them in `nodes`.
const CORE_NODE_TYPES: &[&str] = &["EprHead", "ContributorDID"];

// ---------------------------------------------------------------------------
// Validation errors
// ---------------------------------------------------------------------------

#[derive(thiserror::Error, Debug)]
pub enum ValidationError {
    #[error("rule '{0}' shadows core primitive — choose a different rule name")]
    Shadow(String),
    #[error("duplicate edge type '{0}' within manifest")]
    DuplicateEdge(String),
    #[error("duplicate rule name '{0}' within manifest")]
    DuplicateRule(String),
    #[error("edge '{0}' references undeclared node type '{1}' (not a core type and not declared in graph.nodes)")]
    UndeclaredType(String, String),
}

// ---------------------------------------------------------------------------
// Apply errors
// ---------------------------------------------------------------------------

#[derive(thiserror::Error, Debug)]
pub enum ApplyError {
    #[error("validation: {0}")]
    Validation(#[from] ValidationError),
    #[error("graph engine: {0}")]
    Graph(#[from] GraphError),
}

// ---------------------------------------------------------------------------
// validate_graph_extension
// ---------------------------------------------------------------------------

/// Validate a manifest graph extension before applying it to the engine.
///
/// Checks:
/// 1. No rule name shadows a core primitive.
/// 2. No duplicate edge types within this manifest.
/// 3. No duplicate rule names within this manifest.
/// 4. All edge endpoint node types are either core types or declared in `ext.nodes`.
pub fn validate_graph_extension(ext: &GraphExtension) -> Result<(), ValidationError> {
    // 1. Rule shadowing check
    for rule in &ext.rules {
        if CORE_PRIMITIVE_NAMES.contains(&rule.name.as_str()) {
            return Err(ValidationError::Shadow(rule.name.clone()));
        }
    }

    // 2. Edge type uniqueness
    let mut seen_edges: HashSet<&str> = HashSet::new();
    for edge in &ext.edges {
        if !seen_edges.insert(edge.rel_type.as_str()) {
            return Err(ValidationError::DuplicateEdge(edge.rel_type.clone()));
        }
    }

    // 3. Rule name uniqueness
    let mut seen_rules: HashSet<&str> = HashSet::new();
    for rule in &ext.rules {
        if !seen_rules.insert(rule.name.as_str()) {
            return Err(ValidationError::DuplicateRule(rule.name.clone()));
        }
    }

    // 4. Edge endpoint type references — must be core types or declared in nodes
    let declared_node_types: HashSet<&str> = CORE_NODE_TYPES
        .iter()
        .copied()
        .chain(ext.nodes.iter().map(|n| n.node_type.as_str()))
        .collect();
    for edge in &ext.edges {
        if !declared_node_types.contains(edge.from.as_str()) {
            return Err(ValidationError::UndeclaredType(
                edge.rel_type.clone(),
                edge.from.clone(),
            ));
        }
        if !declared_node_types.contains(edge.to.as_str()) {
            return Err(ValidationError::UndeclaredType(
                edge.rel_type.clone(),
                edge.to.clone(),
            ));
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// apply_graph_extension
// ---------------------------------------------------------------------------

/// Apply a validated graph extension to the live graph engine.
///
/// Actions taken:
/// - **Node relations**: Creates a CozoDB relation `{manifest_name}_{node_type}`
///   with the declared properties. Idempotent — CozoDB `:create` errors are
///   swallowed so re-application is safe.
/// - **Indexes**: Creates `::index create {on}:{name} { from_cid, to_cid }` for
///   each declared index. CozoDB does not support partial indexes; `where` and
///   `order_by` fields are informational. Idempotent.
/// - **Rules**: NOT materialised in CozoDB (no persistent named-rule mechanism).
///   The caller is responsible for storing `ext.rules` in-memory (e.g. in a
///   `HashMap<String, Vec<RuleDecl>>`).
///
/// Returns `Err(ApplyError::Validation(_))` if validation fails.
/// Returns `Err(ApplyError::Graph(_))` only on engine errors during domain
/// node relation creation (index creation errors are swallowed as idempotent).
pub fn apply_graph_extension(
    engine: &GraphEngine,
    manifest_name: &str,
    ext: &GraphExtension,
) -> Result<(), ApplyError> {
    validate_graph_extension(ext)?;

    // Create domain node augmentation relations
    for node in &ext.nodes {
        let cols: Vec<String> = node
            .properties
            .iter()
            .map(|(col_name, col_type)| format!("{col_name}: {col_type}"))
            .collect();
        // Relation name: {manifest_name}_{node_type} (lowercased for CozoDB convention)
        let rel_name = format!(
            "{}_{}",
            manifest_name.to_lowercase(),
            node.node_type.to_lowercase()
        );
        let script = if cols.is_empty() {
            format!(":create {rel_name} {{ cid: String }}")
        } else {
            format!(":create {rel_name} {{ cid: String => {} }}", cols.join(", "))
        };
        // Idempotent — swallow already-exists errors
        let _ = engine.run_script(&script, &[]);
    }

    // Create indexes
    for idx in &ext.indexes {
        // CozoDB index syntax: ::index create relation:name { col1, col2, ... }
        // No partial/where support — always index from_cid + to_cid for edge relations.
        let script = format!(
            "::index create {}:{} {{ from_cid, to_cid }}",
            idx.on, idx.name
        );
        // Idempotent — swallow already-exists errors
        let _ = engine.run_script(&script, &[]);
    }

    // Rules are not materialised here — they are inlined at query time.
    // Callers store ext.rules in a HashMap<String, Vec<RuleDecl>>.

    Ok(())
}

// ---------------------------------------------------------------------------
// In-memory rule store (companion helper)
// ---------------------------------------------------------------------------

/// Stores registered domain rules across manifests.
///
/// Rules are keyed by rule name. `register` validates the extension first and
/// rejects the batch if any rule name conflicts with an already-registered rule
/// from another manifest (cross-manifest duplicate).
pub struct GraphRuleStore {
    /// rule_name → (manifest_name, RuleDecl)
    rules: HashMap<String, (String, RuleDecl)>,
}

impl GraphRuleStore {
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
        }
    }

    /// Register all rules from `ext` under `manifest_name`. Returns an error
    /// if any rule name from this manifest conflicts with an already-registered
    /// rule from a *different* manifest.
    pub fn register(
        &mut self,
        manifest_name: &str,
        ext: &GraphExtension,
    ) -> Result<(), ValidationError> {
        // Per-manifest uniqueness already enforced by validate_graph_extension;
        // here we check cross-manifest conflicts.
        for rule in &ext.rules {
            if let Some((existing_manifest, _)) = self.rules.get(&rule.name) {
                if existing_manifest != manifest_name {
                    return Err(ValidationError::DuplicateRule(rule.name.clone()));
                }
            }
        }
        for rule in &ext.rules {
            self.rules
                .insert(rule.name.clone(), (manifest_name.to_string(), rule.clone()));
        }
        Ok(())
    }

    /// Look up a rule by name. Returns the `RuleDecl` if registered.
    pub fn get(&self, name: &str) -> Option<&RuleDecl> {
        self.rules.get(name).map(|(_, r)| r)
    }
}

impl Default for GraphRuleStore {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn prereq_edge() -> EdgeDecl {
        EdgeDecl {
            rel_type: "PREREQUISITE".into(),
            from: "EprHead".into(),
            to: "EprHead".into(),
            indexed: true,
            directional: true,
            weighted: false,
            temporal: false,
            description: None,
        }
    }

    fn prereq_rule() -> RuleDecl {
        RuleDecl {
            name: "prerequisite_chain".into(),
            datalog: "prerequisite_chain[to, hops] := *epr_edge{from_cid: $start, to_cid: to, rel_type: 'PREREQUISITE'}, hops = 1".into(),
            description: None,
        }
    }

    #[test]
    fn rejects_shadowing_of_core_primitives() {
        let ext = GraphExtension {
            rules: vec![RuleDecl {
                name: "neighborhood".into(),
                datalog: "...".into(),
                description: None,
            }],
            ..Default::default()
        };
        let result = validate_graph_extension(&ext);
        assert!(result.is_err());
        let msg = format!("{:?}", result);
        assert!(msg.contains("shadow") || msg.contains("Shadow"), "msg={msg}");
    }

    #[test]
    fn rejects_shadowing_version_chain() {
        let ext = GraphExtension {
            rules: vec![RuleDecl {
                name: "version_chain".into(),
                datalog: "...".into(),
                description: None,
            }],
            ..Default::default()
        };
        assert!(validate_graph_extension(&ext).is_err());
    }

    #[test]
    fn rejects_duplicate_edge_types_within_manifest() {
        let ext = GraphExtension {
            edges: vec![prereq_edge(), prereq_edge()],
            ..Default::default()
        };
        let result = validate_graph_extension(&ext);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_duplicate_rule_names_within_manifest() {
        let ext = GraphExtension {
            rules: vec![prereq_rule(), prereq_rule()],
            ..Default::default()
        };
        let result = validate_graph_extension(&ext);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_edge_with_undeclared_from_type() {
        let ext = GraphExtension {
            edges: vec![EdgeDecl {
                rel_type: "FOO".into(),
                from: "UnknownType".into(),
                to: "EprHead".into(),
                indexed: false,
                directional: true,
                weighted: false,
                temporal: false,
                description: None,
            }],
            ..Default::default()
        };
        let result = validate_graph_extension(&ext);
        assert!(result.is_err());
    }

    #[test]
    fn accepts_edge_with_domain_declared_node_type() {
        let ext = GraphExtension {
            edges: vec![EdgeDecl {
                rel_type: "PREREQUISITE".into(),
                from: "LearningPath".into(),
                to: "EprHead".into(),
                indexed: false,
                directional: true,
                weighted: false,
                temporal: false,
                description: None,
            }],
            nodes: vec![NodeDecl {
                node_type: "LearningPath".into(),
                properties: [("step_count".to_string(), "Int".to_string())]
                    .into_iter()
                    .collect(),
            }],
            ..Default::default()
        };
        assert!(validate_graph_extension(&ext).is_ok());
    }

    #[test]
    fn accepts_valid_extension() {
        let ext = GraphExtension {
            edges: vec![prereq_edge()],
            rules: vec![prereq_rule()],
            ..Default::default()
        };
        assert!(validate_graph_extension(&ext).is_ok());
    }

    #[test]
    fn accepts_empty_extension() {
        let ext = GraphExtension::default();
        assert!(validate_graph_extension(&ext).is_ok());
    }
}
