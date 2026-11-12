//! Deterministic dependency-group selection; this projection grants no deletion authority.
use super::*;
use crate::feedback_capacity::Capacity;
use chrono::{DateTime, Utc};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Selection {
    pub reclaimable_bytes: u64,
    pub protected_bytes: u64,
    pub selected_bytes: u64,
    pub waiting: Vec<String>,
    pub selected: BTreeSet<usize>,
}

pub(super) fn references(value: &serde_json::Value, out: &mut BTreeSet<Cid>) {
    if let Some(s) = value.as_str() {
        if let Ok(cid) = s.parse() {
            out.insert(cid);
        }
    }
    if let Ok(cid) = serde_json::from_value::<Cid>(value.clone()) {
        out.insert(cid);
    }
    match value {
        serde_json::Value::Array(a) => {
            for v in a {
                references(v, out);
            }
        }
        serde_json::Value::Object(o) => {
            for v in o.values() {
                references(v, out);
            }
        }
        _ => {}
    }
}

fn time(s: &str, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|t| t.to_utc())
        .filter(|t| *t <= now)
}

pub(super) fn select(
    entries: &[Entry],
    lines: &[&str],
    capacity: &Capacity,
    flow_refs: &BTreeSet<Cid>,
    now: DateTime<Utc>,
    incoming: &BTreeSet<Cid>,
    reserved_bytes: u64,
) -> Result<Selection> {
    let index: BTreeMap<_, _> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| (e.cid, i))
        .collect();
    let mut parent: Vec<usize> = (0..entries.len()).collect();
    fn root(parent: &[usize], mut i: usize) -> usize {
        while parent[i] != i {
            i = parent[i];
        }
        i
    }
    let mut unknown = BTreeSet::new();
    for (i, e) in entries.iter().enumerate() {
        let mut refs = BTreeSet::new();
        references(
            &serde_json::to_value(&e.record).map_err(|e| FabricError::Encode(e.to_string()))?,
            &mut refs,
        );
        for cid in refs {
            if let Some(j) = index.get(&cid) {
                let a = root(&parent, i);
                let b = root(&parent, *j);
                parent[a] = b;
            } else if !flow_refs.contains(&cid) {
                unknown.insert(i);
            }
        }
    }
    let mut groups = BTreeMap::<usize, Vec<usize>>::new();
    for i in 0..entries.len() {
        groups.entry(root(&parent, i)).or_default().push(i);
    }
    let mut eligible = vec![];
    let mut result = Selection {
        reclaimable_bytes: 0,
        protected_bytes: 0,
        selected_bytes: 0,
        waiting: vec![],
        selected: BTreeSet::new(),
    };
    for group in groups.values() {
        let bytes: u64 = group.iter().map(|i| lines[*i].len() as u64).sum();
        let mut protected = group.iter().any(|i| {
            entries[*i].protected == Some(true)
                || unknown.contains(i)
                || flow_refs.contains(&entries[*i].cid)
                || incoming.contains(&entries[*i].cid)
        });
        let mut newest = None;
        let classified: BTreeSet<Cid> = group
            .iter()
            .filter_map(|i| match &entries[*i].record {
                Feedback::EvidenceLink {
                    occurrence,
                    relation: Relation::RoutineSuccess,
                    ..
                } => Some(*occurrence),
                _ => None,
            })
            .collect();
        let mut routine = true;
        let mut recovery = BTreeMap::new();
        for i in group {
            let e = &entries[*i];
            match &e.record {
                Feedback::Occurrence(o) => {
                    let mut dependencies = BTreeSet::new();
                    references(
                        &serde_json::to_value(&e.record)
                            .map_err(|e| FabricError::Encode(e.to_string()))?,
                        &mut dependencies,
                    );
                    routine &= (e.automatic == Some(true) || classified.contains(&e.cid))
                        && o.outcome == Outcome::Passed
                        && o.source.producer != "human"
                        && o.message.is_none()
                        && dependencies.is_empty();
                    let t = time(&o.observed_at, now);
                    protected |= t.is_none();
                    newest = newest.max(t);
                }
                Feedback::EvidenceLink {
                    relation: Relation::Protected,
                    ..
                } => {
                    protected = true;
                    routine = false;
                }
                Feedback::EvidenceLink {
                    relation: Relation::RoutineSuccess,
                    at,
                    ..
                } => {
                    protected |= time(at, now).is_none();
                }
                Feedback::EvidenceLink {
                    occurrence,
                    evidence,
                    relation: Relation::ExperiencedRecovery,
                    at,
                    ..
                } => {
                    routine = false;
                    let t = time(at, now);
                    protected |= t.is_none();
                    newest = newest.max(t);
                    if let Some(t) = t {
                        recovery.insert(*occurrence, t);
                        recovery.insert(*evidence, t);
                    }
                }
                Feedback::EvidenceLink { at, .. } | Feedback::Interpretation { at, .. } => {
                    routine = false;
                    let t = time(at, now);
                    protected |= t.is_none();
                    newest = newest.max(t);
                }
                Feedback::CommitmentLink { .. } => {
                    routine = false;
                    protected = true;
                }
            }
        }
        let resolved = !recovery.is_empty()
            && group.iter().all(|i| match &entries[*i].record {
                Feedback::Occurrence(_) => recovery.contains_key(&entries[*i].cid),
                _ => true,
            });
        let age = newest.map(|t| (now - t).num_seconds());
        let days = if routine {
            capacity.parameters.routine_days
        } else {
            capacity.parameters.resolved_days
        };
        let old = age.is_some_and(|s| s >= i64::from(days) * 86400);
        if !protected && (routine || resolved) && old {
            result.reclaimable_bytes += bytes;
            let cid = group
                .iter()
                .map(|i| entries[*i].cid.to_string())
                .min()
                .unwrap_or_default();
            eligible.push((newest, cid, group, bytes));
        } else {
            result.protected_bytes += bytes;
            if unknown.iter().any(|i| group.contains(i)) {
                result
                    .waiting
                    .push("unavailable reference information".into());
            }
            if !routine && !resolved {
                result.waiting.push("experience needs classification or experienced recovery; legacy format is not a retention exemption".into());
            }
            if !old {
                result.waiting.push(
                    "minimum age not established (young, unknown or future timestamp)".into(),
                );
            }
            if protected {
                result
                    .waiting
                    .push("protected evidence, retained reference, or unknown dependency".into());
            }
        }
    }
    eligible.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
    let total: u64 = lines.iter().map(|s| s.len() as u64).sum();
    for (_, _, group, bytes) in eligible {
        if total.saturating_sub(result.selected_bytes)
            <= capacity.target().saturating_sub(reserved_bytes)
        {
            break;
        }
        result.selected.extend(group.iter().copied());
        result.selected_bytes += bytes;
    }
    result.waiting.sort();
    result.waiting.dedup();
    Ok(result)
}
