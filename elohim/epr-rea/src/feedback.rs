//! Private experience evidence, shared by human authors and operational adapters.
//!
//! Occurrences are facts reported by a producer, not diagnoses. Interpretations and links
//! are later acts. Neither a proposed objective nor a passing validation admits repair or
//! establishes recovery. These atoms use the existing EPR codec and offline sidecar floor;
//! there is deliberately no publication, Git, conductor, or execution dependency here.

use cid::Cid;
use serde::{Deserialize, Serialize};

use crate::{atom_cid, FabricError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    Reported,
    Passed,
    Failed,
    Skipped,
    Unmeasured,
    Refused,
    BridgeDeclared,
    FallbackDeclared,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Source {
    pub producer: String,
    /// Producer-assigned invocation/run identity, stable across retries of ingestion.
    pub run: String,
    /// Identity within that run (a finding fingerprint, command outcome, or authored item).
    pub item: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Occurrence {
    pub source: Source,
    pub observed_at: String,
    /// Reported participant, not authenticated identity. None preserves missing attribution.
    pub participant: Option<String>,
    pub subject: String,
    pub outcome: Outcome,
    /// A producer's aggregate count. One record is NOT this many individually witnessed acts.
    pub count: u64,
    pub duration_ms: Option<u64>,
    pub impact: Option<String>,
    /// Deliberately attached narrative; never filled from automatic command diagnostics.
    pub message: Option<String>,
    /// Adapter-owned structured evidence. The automatic adapter must allow-list its fields.
    pub detail: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Relation {
    Attempt,
    Workaround,
    VerifiedFix,
    AvailableRelease,
    ExperiencedRecovery,
    /// Deliberate classification of an old successful command with unknown capture provenance.
    RoutineSuccess,
    /// Explicit constitutional/counter-evidence protection of a retained dependency group.
    Protected,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Feedback {
    Occurrence(Occurrence),
    Interpretation {
        occurrences: Vec<Cid>,
        author: String,
        at: String,
        summary: String,
        /// A proposal only; admission remains with the existing commitment/execution rails.
        objective: Option<String>,
    },
    CommitmentLink {
        occurrence: Cid,
        commitment: Cid,
    },
    EvidenceLink {
        occurrence: Cid,
        evidence: Cid,
        relation: Relation,
        observer: String,
        at: String,
        outcome: Outcome,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Policy {
    pub id: String,
    pub authority: String,
    pub participant: Option<String>,
    pub producers: Vec<String>,
    pub automatic_capture: bool,
    pub deliberate_evidence: bool,
    /// Retention ceiling, not an eviction threshold. Full stores refuse, preserving evidence.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub max_bytes: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub max_record_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<crate::feedback_capacity::Capacity>,
    pub response: Response,
    pub sharing: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Response {
    Report,
    Propose,
    Repair,
}

fn is_zero(n: &u64) -> bool {
    *n == 0
}

impl Policy {
    pub fn active_bytes(&self) -> u64 {
        self.capacity
            .as_ref()
            .map_or(self.max_bytes, |c| c.parameters.active_bytes)
    }
    pub fn record_bytes(&self) -> u64 {
        self.capacity
            .as_ref()
            .map_or(self.max_record_bytes, |c| c.parameters.record_bytes)
    }
    pub fn validate(&self) -> Result<()> {
        if let Some(capacity) = &self.capacity {
            capacity.validate()?;
            if self.max_bytes != 0 || self.max_record_bytes != 0 {
                return invalid("capacity contract cannot coexist with legacy byte limits");
            }
        }
        if self.id.is_empty()
            || self.authority.is_empty()
            || self.producers.is_empty()
            || self.record_bytes() == 0
            || self.active_bytes() < self.record_bytes()
            || self.sharing != "separately-authorized"
        {
            return invalid(
                "missing policy authority, retention bounds, producers or sharing boundary",
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Entry {
    pub cid: Cid,
    pub policy: Policy,
    pub record: Feedback,
    /// None is historical, unknown capture provenance; it never means automatic success.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automatic: Option<bool>,
    /// Constitutional and counter-evidence are never eligible for automatic reclamation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protected: Option<bool>,
}

impl Entry {
    pub fn new(policy: Policy, record: Feedback) -> Result<Self> {
        policy.validate()?;
        let cid = atom_cid(&(&policy, &record))?;
        Ok(Self {
            cid,
            policy,
            record,
            automatic: None,
            protected: None,
        })
    }

    pub fn captured(
        policy: Policy,
        record: Feedback,
        automatic: bool,
        protected: bool,
    ) -> Result<Self> {
        let mut entry = Self::new(policy, record)?;
        entry.automatic = Some(automatic);
        entry.protected = protected.then_some(true);
        entry.cid = entry.address()?;
        Ok(entry)
    }

    fn address(&self) -> Result<Cid> {
        if self.automatic.is_none() && self.protected.is_none() {
            atom_cid(&(&self.policy, &self.record))
        } else {
            atom_cid(&(&self.policy, &self.record, self.automatic, self.protected))
        }
    }

    pub fn verify(&self) -> Result<()> {
        if self.protected == Some(false) {
            return invalid("unprotected records omit the protection marker");
        }
        self.policy.validate()?;
        let computed = self.address()?;
        if computed != self.cid {
            return Err(FabricError::Integrity {
                stored: self.cid.to_string(),
                computed: computed.to_string(),
            });
        }
        Ok(())
    }
}

pub(crate) fn invalid<T>(message: &str) -> Result<T> {
    Err(FabricError::InvalidFeedback(message.into()))
}

/// Rebuildable view. Counts, attempts, releases and experienced recovery remain separate.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Experience<'a> {
    #[serde(serialize_with = "crate::index::cid_serde::serialize")]
    pub cid: Cid,
    pub occurrence: &'a Occurrence,
    /// Distinct producer runs naming this exact subject, not a diagnosis or inferred people.
    pub observed_runs: usize,
    #[serde(serialize_with = "addresses")]
    pub interpretations: Vec<Cid>,
    #[serde(serialize_with = "addresses")]
    pub commitments: Vec<Cid>,
    pub attempts: usize,
    pub workaround_outcomes: Vec<Outcome>,
    pub verified_fix: bool,
    pub available_release: bool,
    pub experienced_recovery: bool,
}

// This is a read projection only; retained atoms keep their canonical CID encoding.
fn addresses<S: serde::Serializer>(
    ids: &[Cid],
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    serializer.collect_seq(ids.iter().map(Cid::to_string))
}

pub fn experiences(entries: &[Entry]) -> Vec<Experience<'_>> {
    entries
        .iter()
        .filter_map(|entry| {
            let Feedback::Occurrence(occurrence) = &entry.record else {
                return None;
            };
            let mut view = Experience {
                cid: entry.cid,
                occurrence,
                observed_runs: entries
                    .iter()
                    .filter_map(|entry| match &entry.record {
                        Feedback::Occurrence(other)
                            if other.source.producer == occurrence.source.producer
                                && other.subject == occurrence.subject =>
                        {
                            Some(other.source.run.as_str())
                        }
                        _ => None,
                    })
                    .collect::<std::collections::BTreeSet<_>>()
                    .len(),
                interpretations: vec![],
                commitments: vec![],
                attempts: 0,
                workaround_outcomes: vec![],
                verified_fix: false,
                available_release: false,
                experienced_recovery: false,
            };
            let mut attempt_evidence = std::collections::BTreeSet::new();
            for other in entries {
                match &other.record {
                    Feedback::Interpretation { occurrences, .. }
                        if occurrences.contains(&entry.cid) =>
                    {
                        view.interpretations.push(other.cid)
                    }
                    Feedback::CommitmentLink {
                        occurrence,
                        commitment,
                    } if *occurrence == entry.cid => view.commitments.push(*commitment),
                    Feedback::EvidenceLink {
                        occurrence,
                        relation,
                        outcome,
                        evidence,
                        ..
                    } if *occurrence == entry.cid => match relation {
                        Relation::Attempt => {
                            attempt_evidence.insert(*evidence);
                        }
                        Relation::Workaround => view.workaround_outcomes.push(*outcome),
                        Relation::VerifiedFix => view.verified_fix |= *outcome == Outcome::Passed,
                        Relation::AvailableRelease => {
                            view.available_release |= *outcome == Outcome::Passed
                        }
                        Relation::RoutineSuccess | Relation::Protected => {}
                        Relation::ExperiencedRecovery => {
                            view.experienced_recovery |= *outcome == Outcome::Passed
                        }
                    },
                    _ => {}
                }
            }
            view.attempts = attempt_evidence.len();
            Some(view)
        })
        .collect()
}
