use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{AxisId, Predicate};

/// Provenance of a clause; admission does not decide which sources earn wider reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClauseSource {
    /// Named by the build inputs.
    Declared,
    /// Read from the artifact's bytes.
    Extracted,
    /// The exact bytes ran on that host class.
    Proven,
}

/// One original requirement, preserving the source of its value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clause {
    /// Dimension of compatibility.
    pub axis: AxisId,
    /// Required value and comparison operation.
    pub predicate: Predicate,
    /// Where this requirement was learned.
    pub source: ClauseSource,
}

/// The stricter requirement for an axis, with all contributing original evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergedClause {
    pub(crate) axis: AxisId,
    pub(crate) predicate: Predicate,
    pub(crate) evidence: BTreeSet<Clause>,
}

impl MergedClause {
    /// Axis being checked.
    pub fn axis(&self) -> &AxisId {
        &self.axis
    }

    /// Conjunction of the original predicates, never a weaker replacement.
    pub fn predicate(&self) -> &Predicate {
        &self.predicate
    }

    /// Original values and sources; a merged union does not fabricate proven evidence.
    pub fn evidence(&self) -> &BTreeSet<Clause> {
        &self.evidence
    }
}

/// A disagreement retained independently of insertion order or intermediate merges.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Disagreement {
    /// Original build declaration.
    pub declared: Clause,
    /// Original observation of artifact bytes.
    pub extracted: Clause,
}

/// Incompatible clauses cannot be represented by one stricter predicate.
/// The insertion is rejected atomically; callers must not ignore this error and run bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeError {
    /// Axis whose predicates conflict.
    pub axis: AxisId,
    /// Existing effective requirement.
    pub existing: Box<Predicate>,
    /// Incoming requirement which cannot be conjoined.
    pub incoming: Box<Predicate>,
}

impl fmt::Display for MergeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "incompatible requirements on {}: {:?} and {:?}",
            self.axis.0, self.existing, self.incoming
        )
    }
}

impl std::error::Error for MergeError {}

/// A set of requirements normalized to one clause per axis.
/// The wire form is an array of original [`Clause`] values; decoding performs the
/// same checked merge as construction, so serialization cannot forge a weaker result.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<Clause>", into = "Vec<Clause>")]
pub struct Requires {
    clauses: BTreeMap<AxisId, MergedClause>,
}

impl Requires {
    /// Effective clauses in deterministic axis order.
    pub fn clauses(&self) -> impl ExactSizeIterator<Item = &MergedClause> {
        self.clauses.values()
    }

    /// Merge one clause. A failed merge leaves the previous set untouched.
    pub fn insert(&mut self, clause: Clause) -> Result<(), MergeError> {
        if let Some(existing) = self.clauses.get_mut(&clause.axis) {
            let predicate = existing
                .predicate
                .stricter(&clause.predicate)
                .ok_or_else(|| MergeError {
                    axis: clause.axis.clone(),
                    existing: Box::new(existing.predicate.clone()),
                    incoming: Box::new(clause.predicate.clone()),
                })?;
            existing.predicate = predicate;
            existing.evidence.insert(clause);
        } else {
            self.clauses.insert(
                clause.axis.clone(),
                MergedClause {
                    axis: clause.axis.clone(),
                    predicate: clause.predicate.clone(),
                    evidence: BTreeSet::from([clause]),
                },
            );
        }
        Ok(())
    }

    /// Conjoin two requirement sets, preserving both inputs even if the merge fails.
    pub fn merge(&self, other: &Self) -> Result<Self, MergeError> {
        let mut merged = self.clone();
        for clause in other.clauses.values().flat_map(|clause| &clause.evidence) {
            merged.insert(clause.clone())?;
        }
        Ok(merged)
    }

    /// Recorded declared/extracted disagreements, including original weaker values.
    /// Comparison is semantic, so version 2.34 and 2.34.0 do not disagree.
    pub fn disagreements(&self) -> BTreeSet<Disagreement> {
        let mut disagreements = BTreeSet::new();
        for clause in self.clauses.values() {
            for declared in clause
                .evidence
                .iter()
                .filter(|c| c.source == ClauseSource::Declared)
            {
                for extracted in clause
                    .evidence
                    .iter()
                    .filter(|c| c.source == ClauseSource::Extracted)
                {
                    let equivalent = match (&declared.predicate, &extracted.predicate) {
                        (Predicate::AtLeast(left), Predicate::AtLeast(right)) => {
                            left.compare(right) == Some(std::cmp::Ordering::Equal)
                        }
                        (left, right) => left == right,
                    };
                    if !equivalent {
                        disagreements.insert(Disagreement {
                            declared: declared.clone(),
                            extracted: extracted.clone(),
                        });
                    }
                }
            }
        }
        disagreements
    }
}

impl TryFrom<Vec<Clause>> for Requires {
    type Error = MergeError;

    fn try_from(clauses: Vec<Clause>) -> Result<Self, Self::Error> {
        let mut requires = Self::default();
        for clause in clauses {
            requires.insert(clause)?;
        }
        Ok(requires)
    }
}

impl From<Requires> for Vec<Clause> {
    fn from(requires: Requires) -> Self {
        requires
            .clauses
            .into_values()
            .flat_map(|clause| clause.evidence)
            .collect()
    }
}
