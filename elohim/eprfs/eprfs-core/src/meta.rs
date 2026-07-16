use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{address::EprRef, projection::ProjectionPath};

/// Projected EPR head coupling for a path, subtree, or resource.
///
/// This is the eprfs-native view of the EPR head relationship: story/knowledge,
/// value or REA, governance, place, attestations, and claims are carried
/// together. Governance is one leg of the coupling, not the whole model.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EprHeadCoupling {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub story: Option<EprRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<EprRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub governance: Option<EprRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<EprRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attestations: Vec<EprRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub claims: Vec<EprRef>,
}

/// Authored or projected metadata that answers "what EPR head/meta governs and
/// contextualizes this resource?"
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EprMetaRecord {
    pub id: String,
    pub subject: EprMetaSubject,
    pub purpose: Option<String>,
    pub coupling: EprHeadCoupling,
    pub governance: Option<EprMetaGovernance>,
    pub cites: Vec<String>,
    pub source: EprMetaSource,
    pub metadata: Value,
}

/// The resolved answer for a projected path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EprMetaResolution {
    pub path: ProjectionPath,
    pub records: Vec<EprMetaRecord>,
    pub effective_rules: Vec<GovernanceRule>,
    pub effective_policies: Vec<GovernancePolicyBinding>,
    pub coupling: EprHeadCoupling,
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EprMetaSubject {
    Projection,
    Path { path: ProjectionPath },
    Children { path: ProjectionPath },
    Subtree { path: ProjectionPath },
    Pattern { pattern: String },
    Source { namespace: String, id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EprMetaSource {
    pub format: String,
    pub path: Option<ProjectionPath>,
    pub epr: Option<EprRef>,
}

/// Governance-specific part of an EPR meta record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EprMetaGovernance {
    pub scope: GovernanceScope,
    pub bindings: Vec<GovernanceBinding>,
    pub validators: Vec<GovernanceValidator>,
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceBinding {
    pub id: String,
    pub rule: Option<GovernanceRule>,
    pub policy: Option<GovernancePolicyBinding>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceRule {
    pub id: String,
    pub class: GovernanceRuleClass,
    pub when: Value,
    pub predicate: GovernanceRulePredicate,
    /// Predicate-specific authored data, preserved losslessly for native
    /// evaluators (`require-frontmatter` fields, `route-to.dest`, measure
    /// ceilings, validator refs, and so on).
    #[serde(default)]
    pub parameters: Value,
    /// The version-pinned registry policy that produced this rule, when the
    /// concrete rule was expanded from a policy binding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_ref: Option<String>,
    pub why: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GovernanceRuleClass {
    Deny,
    Ask,
    Inject,
    Measure,
    Dispatch,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernancePolicyBinding {
    pub id: String,
    pub policy: String,
    pub when: Option<Value>,
    pub params: Value,
    pub why: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceValidator {
    pub reference: String,
    pub cid: Option<String>,
    pub fuel: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GovernanceScope {
    Projection,
    Path { path: ProjectionPath },
    Children { path: ProjectionPath },
    Subtree { path: ProjectionPath },
    Pattern { pattern: String },
    Source { namespace: String, id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GovernanceRulePredicate {
    RequireFrontmatter,
    AllowedTypes,
    RouteTo,
    NoNewSubdirs,
    RequireSibling,
    DedupeOf,
    MaxFiles,
    Measure,
    Validator,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_record_carries_head_coupling_and_governance() {
        let record = EprMetaRecord {
            id: "repo-root".into(),
            subject: EprMetaSubject::Projection,
            purpose: Some("project root".into()),
            coupling: EprHeadCoupling {
                story: Some(EprRef::new("epr:story")),
                value: Some(EprRef::new("epr:rea")),
                governance: Some(EprRef::new("epr:governance")),
                place: Some(EprRef::new("epr:place")),
                attestations: vec![EprRef::new("epr:attestation")],
                claims: vec![EprRef::new("epr:claim")],
            },
            governance: Some(EprMetaGovernance {
                scope: GovernanceScope::Projection,
                bindings: vec![GovernanceBinding {
                    id: "rs-loc-ceiling".into(),
                    rule: None,
                    policy: Some(GovernancePolicyBinding {
                        id: "rs-loc-ceiling".into(),
                        policy: "source-file-loc-ceiling@1".into(),
                        when: None,
                        params: Value::Null,
                        why: None,
                    }),
                }],
                validators: vec![],
                metadata: Value::Null,
            }),
            cites: vec!["root-citation".into()],
            source: EprMetaSource {
                format: ".epr-meta".into(),
                path: Some(ProjectionPath::new(".epr-meta").unwrap()),
                epr: None,
            },
            metadata: Value::Null,
        };

        assert_eq!(record.coupling.story.unwrap().as_str(), "epr:story");
        assert_eq!(
            record
                .governance
                .unwrap()
                .bindings
                .first()
                .unwrap()
                .policy
                .as_ref()
                .unwrap()
                .policy,
            "source-file-loc-ceiling@1"
        );
    }
}
