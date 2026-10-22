//! The concern ontology, natively: a concern is a DECLARED ENTITY, and a reference to one is valid
//! when it RESOLVES in the home its relation names — never because its spelling matches a grammar.
//!
//! Operator ruling 2026-10-22 ("the concern as a declared entity with relations, resolution over
//! grammar"); spec `genesis/docs/superpowers/specs/2026-10-22-sense-organ-design.md` §3. Four
//! spellings of the `@concern:` token live across Python, TypeScript, `.mjs` and Rust, and every
//! join they make resolves: the variance was never the defect, an unresolved join is.
//!
//! **Three homes declare concerns.**
//! - *habit* — a `<dir>/.epr-meta/<id>.habit.md` atom: its `id`, plus every `@concern:` its
//!   `checks:` adopt (read through [`super::registers::read_habits`], the register's own reader);
//! - *canon* — `.claude/epr-meta/concerns.yaml` and `policies.yaml`: invariant classes (`C6a`, …)
//!   and their row ids, Precedent-shaped;
//! - *story* — an a2o `.feature` tag line: what a scenario declares itself evidence of (read
//!   through [`super::walk::story_concerns`], the walker's own feature reader).
//!
//! **Four relations, each fixed by WHERE a reference is written, not by how it is spelled.**
//! [`Relation::EvidenceOf`] a habit check's tag resolves in a story; [`Relation::Prices`] a lens
//! `concern:` resolves in a habit; [`Relation::GovernedBy`] a policy `concern:` resolves in the
//! canon; [`Relation::PainOf`] an open findings-ledger row's `concern` resolves in a habit or the
//! canon. Read in the substrate's own terms: a habit is a standing commitment, a story a declared
//! intent to prove, a check adopting it the commitment's `satisfies`, a lens a bound priced against
//! the commitment, a canon class the Precedent that governs it, a finding pain observed against it.
//!
//! This module is the native probe behind `concern-joins-unresolved@1` and
//! `concern-evidence-unadopted@1` (`probe: { native: … }` in `measures.yaml`), run in-process by
//! `epr flow sense`. The lowest-level primitives are Rust for durability and speed; a script that
//! once computed the same answer is a disposable sketch of the design, not a second home for it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_yaml::Value;

use super::measures::{default_measures, Registry};
use super::FlowResult;

/// The tag prefix every concern reference is written with.
pub const TAG_PREFIX: &str = "@concern:";

/// The one token rule for a concern reference: the text after [`TAG_PREFIX`], with the
/// punctuation prose wraps a tag in (`,;)]"'` and backticks) trimmed off the end.
///
/// Deliberately permissive. The HOME decides what is a concern; this only finds where a reference
/// is written, so a token this accepts and no home declares is reported as unresolved, never
/// silently dropped for its spelling.
pub fn reference_in(token: &str) -> Option<&str> {
    let rest = token.strip_prefix(TAG_PREFIX)?;
    let rest = rest.trim_end_matches([',', ';', ')', ']', '"', '\'', '`', '.', ':']);
    (!rest.is_empty()).then_some(rest)
}

/// Every concern reference written anywhere in `text` (free prose, e.g. a habit's `checks:`).
pub fn references_in(text: &str) -> BTreeSet<String> {
    text.split_whitespace()
        .filter_map(|token| {
            let start = token.find(TAG_PREFIX)?;
            reference_in(&token[start..]).map(str::to_string)
        })
        .collect()
}

/// Where a concern reference points, fixed by where it is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Relation {
    /// A habit check's `@concern:X` — the habit commits to be satisfied by story X.
    EvidenceOf,
    /// A lens's `concern:` — the bound prices habit X.
    Prices,
    /// A policy's `concern:` — the row is governed by canon class X.
    GovernedBy,
    /// An open findings-ledger row's `concern` — pain observed against X.
    PainOf,
}

impl Relation {
    pub fn as_str(self) -> &'static str {
        match self {
            Relation::EvidenceOf => "evidence-of",
            Relation::Prices => "prices",
            Relation::GovernedBy => "governed-by",
            Relation::PainOf => "pain-of",
        }
    }
}

/// The three homes, read once.
#[derive(Debug, Default)]
pub struct Homes {
    /// habit id -> (atom path, the story concerns its checks adopt)
    pub habits: BTreeMap<String, (Option<String>, BTreeSet<String>)>,
    /// Every habit id and every address a habit adopts.
    pub habit_addresses: BTreeSet<String>,
    /// Canon row ids and the invariant classes they carry.
    pub canon: BTreeSet<String>,
    /// story concern -> the feature files declaring it.
    pub stories: BTreeMap<String, BTreeSet<String>>,
}

impl Homes {
    pub fn read(root: &Path) -> FlowResult<Self> {
        let mut homes = Homes::default();
        for habit in super::registers::read_habits(root)? {
            let adopted: BTreeSet<String> =
                habit.checks.iter().flat_map(|c| references_in(c)).collect();
            homes.habit_addresses.insert(habit.id.clone());
            homes.habit_addresses.extend(adopted.iter().cloned());
            homes.habits.insert(habit.id, (habit.source, adopted));
        }
        for (rel, section) in [
            (".claude/epr-meta/concerns.yaml", "concerns"),
            (".claude/epr-meta/policies.yaml", "policies"),
        ] {
            for row in yaml_rows(root, rel, section) {
                for field in ["id", "concern"] {
                    if let Some(value) = scalar(&row, field) {
                        homes.canon.insert(value);
                    }
                }
            }
        }
        homes.stories = super::walk::story_concerns(root);
        Ok(homes)
    }

    /// Whether `target` resolves in the home `relation` names.
    pub fn resolves(&self, relation: Relation, target: &str) -> bool {
        match relation {
            Relation::EvidenceOf => self.stories.contains_key(target),
            Relation::Prices => self.habit_addresses.contains(target),
            Relation::GovernedBy => self.canon.contains(target),
            Relation::PainOf => {
                self.habit_addresses.contains(target) || self.canon.contains(target)
            }
        }
    }
}

/// One file holding concern references that do not resolve, as `relation:target` symbols.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedJoins {
    pub subject: String,
    pub references: Vec<String>,
}

/// Every file holding a concern reference that does not resolve in its relation's home.
pub fn unresolved_joins(root: &Path) -> FlowResult<Vec<UnresolvedJoins>> {
    let homes = Homes::read(root)?;
    let mut by_file: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut flag = |subject: &str, relation: Relation, target: &str| {
        if !homes.resolves(relation, target) {
            by_file
                .entry(subject.to_string())
                .or_default()
                .insert(format!("{}:{target}", relation.as_str()));
        }
    };

    for (id, (source, adopted)) in &homes.habits {
        let Some(source) = source else { continue };
        for target in adopted.iter().filter(|t| *t != id) {
            flag(source, Relation::EvidenceOf, target);
        }
    }

    if let Some(registry) = Registry::open_optional(&default_measures(root))? {
        for bound in registry.lens_bounds().iter().filter(|b| !b.retired()) {
            if let Some(target) = &bound.concern {
                flag(super::measures::MEASURES_REL, Relation::Prices, target);
            }
        }
    }

    for row in yaml_rows(root, super::measures::POLICIES_REL, "policies") {
        if scalar(&row, "status").as_deref() == Some("superseded") {
            continue;
        }
        if let Some(target) = scalar(&row, "concern") {
            flag(super::measures::POLICIES_REL, Relation::GovernedBy, &target);
        }
    }

    let ledgers = root.join(".claude/data");
    if let Ok(entries) = std::fs::read_dir(&ledgers) {
        let mut paths: Vec<_> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.to_string_lossy().ends_with("-findings.jsonl"))
            .collect();
        paths.sort();
        for path in paths {
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let rel = format!(
                ".claude/data/{}",
                path.file_name().unwrap_or_default().to_string_lossy()
            );
            for line in text.lines() {
                let Ok(row) = serde_json::from_str::<serde_json::Value>(line) else {
                    continue;
                };
                let open = row.get("status").and_then(|s| s.as_str()).unwrap_or("open") == "open";
                if let (true, Some(target)) = (open, row.get("concern").and_then(|c| c.as_str())) {
                    if !target.is_empty() {
                        flag(&rel, Relation::PainOf, target);
                    }
                }
            }
        }
    }

    Ok(by_file
        .into_iter()
        .map(|(subject, references)| UnresolvedJoins {
            subject,
            references: references.into_iter().collect(),
        })
        .collect())
}

/// Story concerns no habit adopts — evidence serving no declared promise. Not a broken join: a
/// story may precede its habit. Template placeholders (`<name>`) are not concerns.
pub fn unadopted_stories(root: &Path) -> FlowResult<BTreeMap<String, BTreeSet<String>>> {
    let homes = Homes::read(root)?;
    Ok(homes
        .stories
        .into_iter()
        .filter(|(tag, _)| !tag.starts_with('<') && !homes.habit_addresses.contains(tag))
        .collect())
}

fn yaml_rows(root: &Path, rel: &str, section: &str) -> Vec<Value> {
    let Ok(text) = std::fs::read_to_string(root.join(rel)) else {
        return Vec::new();
    };
    let Ok(doc) = serde_yaml::from_str::<Value>(&text) else {
        return Vec::new();
    };
    doc.get(section)
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default()
}

fn scalar(row: &Value, key: &str) -> Option<String> {
    match row.get(key)? {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reference_is_found_wherever_prose_writes_it_and_its_wrapping_is_trimmed() {
        let refs = references_in(
            "a2o @concern:ledgers-fold-once (features/x.feature); see `@concern:run-plane`, \
             and \"@concern:Mixed_Case\".",
        );
        let refs: Vec<&str> = refs.iter().map(String::as_str).collect();
        assert_eq!(refs, ["Mixed_Case", "ledgers-fold-once", "run-plane"]);
    }

    #[test]
    fn spelling_is_never_the_test_a_home_is() {
        let mut homes = Homes::default();
        homes.habit_addresses.insert("Mixed_Case".into());
        homes.canon.insert("C6a".into());
        homes.stories.insert(
            "run-plane".into(),
            BTreeSet::from(["f.feature".to_string()]),
        );
        assert!(homes.resolves(Relation::Prices, "Mixed_Case"));
        assert!(
            !homes.resolves(Relation::Prices, "C6a"),
            "a class prices no habit"
        );
        assert!(homes.resolves(Relation::GovernedBy, "C6a"));
        assert!(homes.resolves(Relation::PainOf, "C6a"));
        assert!(homes.resolves(Relation::EvidenceOf, "run-plane"));
        assert!(!homes.resolves(Relation::EvidenceOf, "Mixed_Case"));
    }
}
