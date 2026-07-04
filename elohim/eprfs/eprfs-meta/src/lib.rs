//! `.epr-meta` authored-source adapter for eprfs.
//!
//! This crate resolves the current directory-local format into the eprfs EPR
//! meta model. It does not replace the existing hook resolver yet.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use eprfs_core::{
    EprHeadCoupling, EprMetaGovernance, EprMetaRecord, EprMetaResolution, EprMetaSource,
    EprMetaSubject, EprRef, GovernanceBinding, GovernancePolicyBinding, GovernanceRule,
    GovernanceRuleClass, GovernanceRulePredicate, GovernanceScope, GovernanceValidator,
    ProjectionPath,
};
use serde::Deserialize;
use serde_json::Value;

pub const MANIFEST_NAME: &str = ".epr-meta";

pub type Result<T> = std::result::Result<T, EprMetaError>;

#[derive(Debug, thiserror::Error)]
pub enum EprMetaError {
    #[error("failed to read {path:?}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("missing .epr-meta frontmatter block in {0:?}")]
    MissingFrontmatter(PathBuf),

    #[error("failed to parse .epr-meta YAML at {path:?}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },

    #[error(transparent)]
    Eprfs(#[from] eprfs_core::EprfsError),
}

pub fn parse_meta_file(path: impl AsRef<Path>) -> Result<EprMetaRecord> {
    let path = path.as_ref();
    let authored = read_authored_meta(path)?;
    authored.to_record(None, path, None)
}

pub fn resolve_path(
    repo_root: impl AsRef<Path>,
    target: impl AsRef<Path>,
) -> Result<EprMetaResolution> {
    let repo_root = repo_root.as_ref();
    let target = target.as_ref();
    let path = projection_path(repo_root, target)?;
    let mut records = Vec::new();

    for meta in collect_cascade(repo_root, target) {
        let authored = read_authored_meta(&meta)?;
        let subject_path = meta
            .parent()
            .and_then(|parent| relative_path(repo_root, parent));
        records.push(authored.to_record(Some(repo_root), &meta, subject_path)?);
    }

    let mut rules = BTreeMap::new();
    let mut policies = BTreeMap::new();
    let mut coupling = EprHeadCoupling::default();

    for record in &records {
        merge_coupling(&mut coupling, &record.coupling);
        if let Some(governance) = &record.governance {
            for binding in &governance.bindings {
                if let Some(rule) = &binding.rule {
                    rules.insert(rule.id.clone(), rule.clone());
                }
                if let Some(policy) = &binding.policy {
                    policies.insert(policy.id.clone(), policy.clone());
                }
            }
        }
    }

    Ok(EprMetaResolution {
        path,
        records,
        effective_rules: rules.into_values().collect(),
        effective_policies: policies.into_values().collect(),
        coupling,
        metadata: Value::Null,
    })
}

fn collect_cascade(repo_root: &Path, target: &Path) -> Vec<PathBuf> {
    let mut current = if target.is_dir() {
        target.to_path_buf()
    } else {
        target.parent().unwrap_or(repo_root).to_path_buf()
    };
    let mut chain = Vec::new();

    loop {
        let meta = current.join(MANIFEST_NAME);
        if meta.is_file() {
            let is_root = read_authored_meta(&meta)
                .map(|authored| authored.root)
                .unwrap_or(false);
            chain.push(meta);
            if is_root {
                break;
            }
        }

        if current == repo_root || !current.pop() {
            break;
        }
    }

    chain.reverse();
    chain
}

fn read_authored_meta(path: &Path) -> Result<AuthoredMeta> {
    let text = fs::read_to_string(path).map_err(|source| EprMetaError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let frontmatter =
        frontmatter(&text).ok_or_else(|| EprMetaError::MissingFrontmatter(path.into()))?;
    serde_yaml::from_str(frontmatter).map_err(|source| EprMetaError::Parse {
        path: path.to_path_buf(),
        source,
    })
}

fn frontmatter(text: &str) -> Option<&str> {
    let rest = text.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    Some(&rest[..end])
}

fn relative_path(root: &Path, path: &Path) -> Option<ProjectionPath> {
    let rel = path.strip_prefix(root).ok()?;
    if rel.as_os_str().is_empty() {
        None
    } else {
        ProjectionPath::new(rel).ok()
    }
}

fn projection_path(root: &Path, path: &Path) -> Result<ProjectionPath> {
    let rel = path.strip_prefix(root).unwrap_or(path);
    ProjectionPath::new(rel).map_err(Into::into)
}

fn merge_coupling(target: &mut EprHeadCoupling, source: &EprHeadCoupling) {
    if source.story.is_some() {
        target.story = source.story.clone();
    }
    if source.value.is_some() {
        target.value = source.value.clone();
    }
    if source.governance.is_some() {
        target.governance = source.governance.clone();
    }
    if source.place.is_some() {
        target.place = source.place.clone();
    }
    target
        .attestations
        .extend(source.attestations.iter().cloned());
    target.claims.extend(source.claims.iter().cloned());
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct AuthoredMeta {
    #[serde(rename = "epr-meta-version")]
    _version: u32,
    id: Option<String>,
    #[serde(default)]
    root: bool,
    covers: Option<String>,
    purpose: Option<String>,
    coupling: Option<AuthoredCoupling>,
    #[serde(default)]
    rules: Vec<AuthoredRule>,
    #[serde(default)]
    validators: Vec<AuthoredValidator>,
    #[serde(default)]
    cites: Vec<String>,
}

impl AuthoredMeta {
    fn to_record(
        &self,
        repo_root: Option<&Path>,
        source_path: &Path,
        subject_path: Option<ProjectionPath>,
    ) -> Result<EprMetaRecord> {
        let subject = if self.root {
            EprMetaSubject::Projection
        } else if self.covers.as_deref() == Some("subtree") {
            match subject_path.clone() {
                Some(path) => EprMetaSubject::Subtree { path },
                None => EprMetaSubject::Projection,
            }
        } else {
            match subject_path.clone() {
                Some(path) => EprMetaSubject::Children { path },
                None => EprMetaSubject::Projection,
            }
        };

        let source_projection_path = repo_root
            .and_then(|root| relative_path(root, source_path))
            .or_else(|| {
                source_path
                    .file_name()
                    .and_then(|name| ProjectionPath::new(Path::new(name)).ok())
            });
        let governance = if self.rules.is_empty() && self.validators.is_empty() {
            None
        } else {
            Some(EprMetaGovernance {
                scope: match subject.clone() {
                    EprMetaSubject::Projection => GovernanceScope::Projection,
                    EprMetaSubject::Path { path } => GovernanceScope::Path { path },
                    EprMetaSubject::Children { path } => GovernanceScope::Children { path },
                    EprMetaSubject::Subtree { path } => GovernanceScope::Subtree { path },
                    EprMetaSubject::Pattern { pattern } => GovernanceScope::Pattern { pattern },
                    EprMetaSubject::Source { namespace, id } => {
                        GovernanceScope::Source { namespace, id }
                    }
                },
                bindings: self.rules.iter().map(AuthoredRule::to_binding).collect(),
                validators: self
                    .validators
                    .iter()
                    .map(AuthoredValidator::to_validator)
                    .collect(),
                metadata: Value::Null,
            })
        };

        Ok(EprMetaRecord {
            id: self.id.clone().unwrap_or_else(|| "unnamed-epr-meta".into()),
            subject,
            purpose: self.purpose.clone(),
            coupling: self.coupling.clone().unwrap_or_default().into(),
            governance,
            cites: self.cites.clone(),
            source: EprMetaSource {
                format: ".epr-meta".into(),
                path: source_projection_path,
                epr: None,
            },
            metadata: Value::Null,
        })
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthoredCoupling {
    story: Option<String>,
    value: Option<String>,
    governance: Option<String>,
    place: Option<String>,
    #[serde(default)]
    attestations: Vec<String>,
    #[serde(default)]
    claims: Vec<String>,
}

impl From<AuthoredCoupling> for EprHeadCoupling {
    fn from(value: AuthoredCoupling) -> Self {
        Self {
            story: value.story.map(EprRef::new),
            value: value.value.map(EprRef::new),
            governance: value.governance.map(EprRef::new),
            place: value.place.map(EprRef::new),
            attestations: value.attestations.into_iter().map(EprRef::new).collect(),
            claims: value.claims.into_iter().map(EprRef::new).collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct AuthoredRule {
    id: String,
    class: Option<String>,
    policy: Option<String>,
    why: Option<String>,
    when: Option<serde_yaml::Value>,
    params: Option<serde_yaml::Value>,
    #[serde(rename = "require-frontmatter")]
    require_frontmatter: Option<serde_yaml::Value>,
    #[serde(rename = "allowed-types")]
    allowed_types: Option<serde_yaml::Value>,
    #[serde(rename = "route-to")]
    route_to: Option<serde_yaml::Value>,
    #[serde(rename = "no-new-subdirs")]
    no_new_subdirs: Option<bool>,
    #[serde(rename = "require-sibling")]
    require_sibling: Option<String>,
    #[serde(rename = "dedupe-of")]
    dedupe_of: Option<String>,
    #[serde(rename = "max-files")]
    max_files: Option<serde_yaml::Value>,
    measure: Option<serde_yaml::Value>,
    validator: Option<String>,
}

impl AuthoredRule {
    fn to_binding(&self) -> GovernanceBinding {
        GovernanceBinding {
            id: self.id.clone(),
            rule: self.policy.is_none().then(|| GovernanceRule {
                id: self.id.clone(),
                class: rule_class(self.class.as_deref()),
                when: yaml_to_json(self.when.as_ref()),
                predicate: self.predicate(),
                why: self.why.clone(),
            }),
            policy: self.policy.as_ref().map(|policy| GovernancePolicyBinding {
                id: self.id.clone(),
                policy: policy.clone(),
                when: self.when.as_ref().map(|when| yaml_to_json(Some(when))),
                params: yaml_to_json(self.params.as_ref()),
                why: self.why.clone(),
            }),
        }
    }

    fn predicate(&self) -> GovernanceRulePredicate {
        if self.require_frontmatter.is_some() {
            GovernanceRulePredicate::RequireFrontmatter
        } else if self.allowed_types.is_some() {
            GovernanceRulePredicate::AllowedTypes
        } else if self.route_to.is_some() {
            GovernanceRulePredicate::RouteTo
        } else if self.no_new_subdirs == Some(true) {
            GovernanceRulePredicate::NoNewSubdirs
        } else if self.require_sibling.is_some() {
            GovernanceRulePredicate::RequireSibling
        } else if self.dedupe_of.is_some() {
            GovernanceRulePredicate::DedupeOf
        } else if self.max_files.is_some() {
            GovernanceRulePredicate::MaxFiles
        } else if self.measure.is_some() {
            GovernanceRulePredicate::Measure
        } else if self.validator.is_some() {
            GovernanceRulePredicate::Validator
        } else {
            GovernanceRulePredicate::Unknown
        }
    }
}

#[derive(Debug, Deserialize)]
struct AuthoredValidator {
    #[serde(rename = "ref")]
    reference: String,
    cid: Option<String>,
    fuel: Option<u32>,
}

impl AuthoredValidator {
    fn to_validator(&self) -> GovernanceValidator {
        GovernanceValidator {
            reference: self.reference.clone(),
            cid: self.cid.clone(),
            fuel: self.fuel,
        }
    }
}

fn rule_class(class: Option<&str>) -> GovernanceRuleClass {
    match class {
        Some("deny") => GovernanceRuleClass::Deny,
        Some("ask") => GovernanceRuleClass::Ask,
        Some("measure") => GovernanceRuleClass::Measure,
        Some("dispatch") => GovernanceRuleClass::Dispatch,
        _ => GovernanceRuleClass::Inject,
    }
}

fn yaml_to_json(value: Option<&serde_yaml::Value>) -> Value {
    value
        .and_then(|value| serde_json::to_value(value).ok())
        .unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use eprfs_core::{EprMetaSubject, GovernanceRulePredicate};
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn resolves_meta_cascade_for_path() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join(MANIFEST_NAME),
            r#"---
epr-meta-version: 1
id: root-meta
root: true
purpose: root
coupling:
  story: epr:story
  value: epr:rea
rules:
  - id: root-policy
    policy: source-file-loc-ceiling@1
---
"#,
        )
        .unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(
            dir.path().join("src").join(MANIFEST_NAME),
            r#"---
epr-meta-version: 1
id: src-meta
covers: subtree
purpose: source
coupling:
  governance: epr:governance
rules:
  - id: route-plans-out
    class: ask
    when: { write: "*-plan.md" }
    route-to: { dest: docs/plans }
---
"#,
        )
        .unwrap();
        fs::write(dir.path().join("src/lib.rs"), "fn main() {}\n").unwrap();

        let resolution = resolve_path(dir.path(), dir.path().join("src/lib.rs")).unwrap();

        assert_eq!(resolution.records.len(), 2);
        assert_eq!(resolution.coupling.story.unwrap().as_str(), "epr:story");
        assert_eq!(resolution.coupling.value.unwrap().as_str(), "epr:rea");
        assert_eq!(
            resolution.coupling.governance.unwrap().as_str(),
            "epr:governance"
        );
        assert_eq!(
            resolution.effective_policies[0].policy,
            "source-file-loc-ceiling@1"
        );
        assert_eq!(
            resolution.effective_rules[0].predicate,
            GovernanceRulePredicate::RouteTo
        );
        assert!(matches!(
            resolution.records[1].subject,
            EprMetaSubject::Subtree { .. }
        ));
        assert_eq!(
            resolution.records[1]
                .source
                .path
                .as_ref()
                .unwrap()
                .as_path(),
            Path::new("src/.epr-meta")
        );
    }
}
