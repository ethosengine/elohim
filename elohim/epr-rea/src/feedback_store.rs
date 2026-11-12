//! Private retained evidence on the same locked, synced sidecar mechanics as flows/actors.
//! This log is not in any peer export allow-list. Summaries are rebuilt, never retained here.

mod generation;
mod retention;
pub use generation::CapacityReport;

use std::path::{Path, PathBuf};

use crate::feedback::{invalid, Entry, Feedback, Outcome, Policy, Relation};
use crate::sidecar::{open_log, LockedLog};
use crate::{FabricError, Result};
use cid::Cid;

const LOG: &str = "feedback.jsonl";

pub struct FeedbackStore {
    root: PathBuf,
    policy: Policy,
}

impl FeedbackStore {
    pub fn new(root: &Path, policy: Policy) -> Result<Self> {
        policy.validate()?;
        Ok(Self {
            root: root.into(),
            policy,
        })
    }

    pub fn read(&self) -> Result<Vec<Entry>> {
        let guard = self.coordinate(false)?;
        if guard.is_none() && !self.root.join(".eprfs/status/feedback.jsonl").exists() {
            return Ok(vec![]);
        }
        self.read_locked()
    }

    pub(crate) fn read_locked(&self) -> Result<Vec<Entry>> {
        if self.root.join(".eprfs/status/feedback/state.json").exists() {
            return generation::read(self);
        }
        let path = self.root.join(".eprfs/status").join(LOG);
        let log = match LockedLog::shared(&path) {
            Ok(log) => log,
            Err(FabricError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(vec![])
            }
            Err(error) => return Err(error),
        };
        let read_limit = self
            .policy
            .capacity
            .as_ref()
            .map_or(self.policy.max_bytes, |c| c.parameters.total_bytes);
        decode(&log.contents_bounded(read_limit)?)
    }

    /// Read/check/append is one exclusive transaction. Source retries return the old address;
    /// conflicting evidence for an existing source identity refuses rather than overwriting it.
    pub fn append(&self, record: Feedback, deliberate: bool) -> Result<Cid> {
        self.append_protected(record, deliberate, false)
    }

    pub fn append_protected(
        &self,
        record: Feedback,
        deliberate: bool,
        protected: bool,
    ) -> Result<Cid> {
        if deliberate && !self.policy.deliberate_evidence {
            return invalid("policy refuses deliberate evidence");
        }
        if !deliberate && !self.policy.automatic_capture {
            return invalid("policy refuses automatic capture");
        }
        if let Feedback::Occurrence(o) = &record {
            if !self.policy.producers.contains(&o.source.producer)
                || o.count == 0
                || o.source.run.is_empty()
                || o.source.item.is_empty()
                || o.subject.is_empty()
                || o.observed_at.is_empty()
            {
                return invalid("invalid occurrence or producer outside policy");
            }
            if !deliberate && o.message.is_some() {
                return invalid("automatic narrative capture refused");
            }
        } else if !deliberate {
            return invalid("interpretation and linking require deliberate authorship");
        }
        let entry = if self.policy.capacity.is_some() {
            Entry::captured(self.policy.clone(), record, !deliberate, protected)?
        } else {
            if protected {
                return invalid("protected capture needs capacity format migration");
            }
            Entry::new(self.policy.clone(), record)?
        };
        let line = serde_json::to_string(&entry).map_err(|e| FabricError::Encode(e.to_string()))?;
        if self.policy.capacity.is_none() && line.len() as u64 + 1 > self.policy.record_bytes() {
            return invalid("feedback record retention limit");
        }
        let _guard = self.coordinate(true)?;
        if self.policy.capacity.is_some() {
            return generation::append(self, entry, line);
        }
        if self.root.join(".eprfs/status/feedback/state.json").exists() {
            return invalid("unsupported feedback format: legacy writer refuses migrated store");
        }
        let path = open_log(&self.root, LOG)?;
        let mut log = LockedLog::exclusive(&path)?;
        let contents = log.contents_bounded(self.policy.max_bytes)?;
        let entries = decode(&contents)?;
        for old in &entries {
            if old.cid == entry.cid {
                return Ok(old.cid);
            }
            if let (Feedback::Occurrence(a), Feedback::Occurrence(b)) = (&old.record, &entry.record)
            {
                if a.source == b.source {
                    // Regenerating a report may change its generatedAt; it is not a new run.
                    let mut b = b.clone();
                    b.observed_at.clone_from(&a.observed_at);
                    if *a == b {
                        return Ok(old.cid);
                    }
                    return invalid("source identity already retained with different evidence");
                }
            }
        }
        validate_links(&entries, &entry.record)?;
        if contents.len() as u64 + line.len() as u64 + 1 > self.policy.max_bytes {
            return invalid("feedback store retention limit; retained evidence preserved");
        }
        log.append(line)?;
        Ok(entry.cid)
    }
}

impl FeedbackStore {
    fn coordinate(&self, write: bool) -> Result<Option<LockedLog>> {
        if write {
            return coordination(&self.root).map(Some);
        }
        let path = self.root.join(".eprfs/status/feedback.lock");
        match LockedLog::shared(&path) {
            Ok(lock) => Ok(Some(lock)),
            Err(FabricError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                if self.root.join(".eprfs/status/feedback/state.json").exists() {
                    return invalid("feedback coordination lock missing; store integrity unknown");
                }
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }
}

/// Lock order: stable feedback lock -> flows log -> generation/latch. Never reacquire.
/// Kept outside replaceable generations. Flow writers use this even before feedback exists.
pub(crate) fn coordination(root: &Path) -> Result<LockedLog> {
    if root.join(".eprfs/status/feedback/state.json").exists()
        && !root.join(".eprfs/status/feedback.lock").exists()
    {
        return invalid("feedback coordination lock missing; refuse replacement of a live lock");
    }
    LockedLog::exclusive(&open_log(root, "feedback.lock")?)
}

fn occurrence<'a>(entries: &'a [Entry], cid: &Cid) -> Result<&'a crate::feedback::Occurrence> {
    entries
        .iter()
        .find_map(|e| match &e.record {
            Feedback::Occurrence(o) if e.cid == *cid => Some(o),
            _ => None,
        })
        .ok_or_else(|| FabricError::InvalidFeedback("occurrence reference is not retained".into()))
}

fn validate_links(entries: &[Entry], record: &Feedback) -> Result<()> {
    match record {
        Feedback::Occurrence(o) => {
            if chrono::DateTime::parse_from_rfc3339(&o.observed_at).is_err() {
                return invalid("occurrence needs an RFC3339 observation time");
            }
        }
        Feedback::Interpretation {
            occurrences,
            author,
            summary,
            ..
        } => {
            if occurrences.is_empty() || author.is_empty() || summary.is_empty() {
                return invalid("interpretation needs evidence, author and summary");
            }
            for cid in occurrences {
                occurrence(entries, cid)?;
            }
        }
        Feedback::CommitmentLink {
            occurrence: target,
            commitment,
        } => {
            occurrence(entries, target)?;
            if entries.iter().any(|e| matches!(&e.record, Feedback::CommitmentLink { occurrence, commitment: c } if occurrence == target && c != commitment)) {
                return invalid("occurrence already linked to repair; inspect existing commitment");
            }
        }
        Feedback::EvidenceLink {
            occurrence: target,
            evidence,
            relation,
            observer,
            outcome,
            ..
        } => {
            let original = occurrence(entries, target)?;
            if observer.is_empty() {
                return invalid("evidence link needs observer");
            }
            if *relation == Relation::RoutineSuccess
                && (*evidence != *target
                    || *outcome != Outcome::Passed
                    || original.outcome != Outcome::Passed
                    || original.source.producer == "human"
                    || original.message.is_some())
            {
                return invalid("routine-success classification needs a successful non-human command without narrative, self-reference and deliberate observer");
            }
            if *relation == Relation::ExperiencedRecovery {
                let recovered = occurrence(entries, evidence)?;
                if *outcome != Outcome::Passed
                    || recovered.outcome != Outcome::Passed
                    || original.participant.as_deref() != Some(observer)
                    || recovered.participant.as_deref() != Some(observer)
                    || recovered.subject != original.subject
                    || recovered.source.producer != original.source.producer
                    || recovered.source.run == original.source.run
                    || chrono::DateTime::parse_from_rfc3339(&recovered.observed_at).ok()
                        < chrono::DateTime::parse_from_rfc3339(&original.observed_at).ok()
                {
                    return invalid("recovery needs a later successful experience by the original participant on the same subject");
                }
            }
        }
    }
    Ok(())
}

pub(super) fn decode(contents: &str) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for line in contents.lines() {
        let entry: Entry =
            serde_json::from_str(line).map_err(|e| FabricError::Decode(e.to_string()))?;
        entry.verify()?;
        validate_links(&entries, &entry.record)?;
        entries.push(entry);
    }
    Ok(entries)
}
