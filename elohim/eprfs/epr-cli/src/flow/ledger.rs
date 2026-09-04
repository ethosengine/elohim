//! `epr flow ledger <path|cid>` — the projection that makes a progress file and a spec's
//! status section GENERATED rather than authored (spec §9 and §12, the named slice-2
//! graduation).
//!
//! Every other reader in this family answers "what stands now" and therefore reads newest
//! first. This one answers "what happened, in order", because that is the only shape a pasted
//! status section can have: a reader of a ledger is reconstructing a story, and a story told
//! backwards is a list. So the ordering here is REVERSED relative to `context` — deliberately,
//! and by the same key: `occurred_at` first, sidecar append order as the tie-break, ascending.
//!
//! **Its scope is the atom AND the commitments scoped by it**, exactly as `context`'s note
//! roll-up is, because a claim, its fulfilment and the verdict a review seat left on it are one
//! story and live at three different addresses. A record made on a commitment carries a
//! `via <gap-id>` marker so the ledger never implies it was written about the atom.
//!
//! Nothing here re-derives anything. The note selector, the slot parsing and the ordering key
//! all come from [`super::read`]; this module decides only what a record MEANS in a story and
//! how one line of it reads.
//!
//! The human render is markdown on purpose: `## Ledger — <label>` and one
//! `- <date> · <kind> · <actor> · <text>` line per record, so the output is pasted into a
//! document verbatim rather than transcribed. A projection a human has to retype is not a
//! projection.

use std::collections::BTreeMap;
use std::path::Path;

use cid::Cid;
use elohim_epr_rea::{FlowRecord, FlowStore, SidecarFlowStore};
use serde::Serialize;

use super::read::{is_note, note_view, OccurredAtKey};
use super::{body_cid_of_file, confine_under, rel_to_root, short_cid, FlowError, FlowResult};

/// One record, as a story reads it.
#[derive(Debug, Clone, Serialize)]
pub struct LedgerEntry {
    /// The record's own atom address.
    pub cid: String,
    /// `claim` | `fulfilment` | the note kind without its `run:` prefix (`ruling`, `verdict`, …).
    pub kind: String,
    /// Who acted: the commitment's provider, the event's provider.
    pub actor: String,
    pub occurred_at: String,
    /// The commitment this record was made ON, when it was not the atom itself.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub via: Option<String>,
    /// The one-line human text this entry renders as.
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brief: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub commits: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub switched_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verdict: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steward: Option<String>,
}

/// The whole ledger for one atom.
#[derive(Debug, Serialize)]
pub struct LedgerResult {
    /// The atom's repo-relative path, or its address when named by one.
    pub label: String,
    pub cid: String,
    /// Oldest first.
    pub entries: Vec<LedgerEntry>,
}

/// `epr flow ledger <path|cid>`.
///
/// Read-only end to end. An atom with no history is an EMPTY ledger, not a refusal: "nothing has
/// happened here yet" is a true and useful answer, and refusing it would make the verb unusable
/// on exactly the atoms a status section is started for.
pub fn ledger(root: &Path, target: &str) -> FlowResult<LedgerResult> {
    let store = SidecarFlowStore::open(root)?;
    let records = store.records()?;
    let (cid, label) = resolve_target(root, target, &records)?;

    // Pass 1: the commitments scoped by this atom, in append order. Each is a claim in the
    // story, and each is also an ADDRESS later records may have been made against.
    let mut vias: BTreeMap<Cid, String> = BTreeMap::new();
    let mut staged: Vec<(OccurredAtKey, usize, LedgerEntry)> = Vec::new();
    for (index, (record_cid, record)) in records.iter().enumerate() {
        let FlowRecord::Commitment(commitment) = record else {
            continue;
        };
        if commitment.in_scope_of != cid {
            continue;
        }
        let slots = &commitment.resource_spec.classified_as;
        let gap_id = slots.get(1).cloned();
        let via = gap_id.clone().unwrap_or_else(|| short_cid(record_cid));
        vias.insert(*record_cid, via);

        let brief = slot(slots, "brief:");
        let occurred_at = commitment.valid_from.clone().unwrap_or_default();
        let text = match (&gap_id, &brief) {
            (Some(id), Some(brief)) => format!("claimed {id} (brief {})", short_str(brief)),
            (Some(id), None) => format!("claimed {id}"),
            (None, _) => format!("claimed {}", short_cid(record_cid)),
        };
        staged.push((
            OccurredAtKey::parse(&occurred_at),
            index,
            LedgerEntry {
                cid: record_cid.to_string(),
                kind: "claim".to_string(),
                actor: commitment.provider.0.clone(),
                occurred_at,
                via: None,
                text,
                gap_id,
                brief,
                status: None,
                report: None,
                commits: Vec::new(),
                reason: None,
                switched_to: None,
                verdict: None,
                steward: slot(slots, "steward:"),
            },
        ));
    }

    // Pass 2: the events. A note anywhere in scope, and a fulfilment of any commitment above.
    for (index, (record_cid, record)) in records.iter().enumerate() {
        let FlowRecord::Event(event) = record else {
            continue;
        };
        let entry = if is_note(event) {
            let via = if event.resource == cid {
                None
            } else if let Some(via) = vias.get(&event.resource) {
                Some(via.clone())
            } else {
                continue;
            };
            let mut view = note_view(record_cid, event);
            view.via = via;
            let text = match (&view.reason, &view.verdict) {
                (Some(reason), Some(verdict)) => format!("[{verdict}] {reason}"),
                (Some(reason), None) => reason.clone(),
                (None, Some(verdict)) => format!("[{verdict}]"),
                (None, None) => String::new(),
            };
            LedgerEntry {
                cid: view.cid,
                // `run:ruling` reads as `ruling` in a story; the tag's prefix is a namespace for
                // machines, and a ledger line is for a person.
                kind: view
                    .kind
                    .strip_prefix("run:")
                    .unwrap_or(&view.kind)
                    .to_string(),
                actor: view.actor,
                occurred_at: view.occurred_at,
                via: view.via,
                text,
                gap_id: None,
                brief: None,
                status: None,
                report: None,
                commits: Vec::new(),
                reason: view.reason,
                switched_to: view.switched_to,
                verdict: view.verdict,
                steward: view.steward,
            }
        } else if let Some(via) = event
            .fulfills
            .iter()
            .find_map(|fulfilled| vias.get(fulfilled))
        {
            let slots = &event.classified_as;
            let status = slots
                .first()
                .and_then(|s| s.strip_prefix("report:"))
                .map(str::to_string);
            let report = slot(slots, "evidence:");
            let commits: Vec<String> = slots
                .iter()
                .filter_map(|s| s.strip_prefix("commit:").map(str::to_string))
                .collect();
            let text = match (&status, &report) {
                (Some(status), Some(report)) => {
                    format!("{status} — report {}", short_str(report))
                }
                (Some(status), None) => status.clone(),
                // The a2o arm carries no report slot at all: it discharges from a sprint report,
                // which is a CI observation and not an authored delivery.
                (None, _) => "discharged".to_string(),
            };
            LedgerEntry {
                cid: record_cid.to_string(),
                kind: "fulfilment".to_string(),
                actor: event.provider.0.clone(),
                occurred_at: event.occurred_at.clone(),
                via: Some(via.clone()),
                text,
                gap_id: slots.get(1).cloned(),
                brief: None,
                status,
                report,
                commits,
                reason: None,
                switched_to: None,
                verdict: None,
                steward: slot(slots, "steward:"),
            }
        } else {
            continue;
        };
        staged.push((OccurredAtKey::parse(&entry.occurred_at), index, entry));
    }

    // ONE sort, ASCENDING — the same key `read` uses, read the other way round.
    staged.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

    Ok(LedgerResult {
        label,
        cid: cid.to_string(),
        entries: staged.into_iter().map(|(_, _, entry)| entry).collect(),
    })
}

fn slot(slots: &[String], prefix: &str) -> Option<String> {
    slots
        .iter()
        .find_map(|s| s.strip_prefix(prefix).map(str::to_string))
}

/// A content address shortened for a prose line, left alone if it is not one.
fn short_str(value: &str) -> String {
    match value.parse::<Cid>() {
        Ok(parsed) => short_cid(&parsed),
        Err(_) => value.to_string(),
    }
}

/// Resolve the target to `(address, label)`. A CID must be an atom this sidecar knows — the
/// same membership rule `note` and `context` hold, and for the same reason: an address for
/// something never recorded would render an empty ledger that reads as "nothing happened".
fn resolve_target(
    root: &Path,
    target: &str,
    records: &[(Cid, FlowRecord)],
) -> FlowResult<(Cid, String)> {
    if let Ok(cid) = target.parse::<Cid>() {
        let known = records.iter().any(|(record_cid, record)| {
            record_cid == &cid
                || match record {
                    FlowRecord::Event(e) => e.resource == cid || e.in_scope_of == cid,
                    FlowRecord::Commitment(c) => c.in_scope_of == cid,
                    FlowRecord::Intent(i) => i.in_scope_of == cid,
                    FlowRecord::Process(p) => {
                        p.in_scope_of == cid || p.inputs.contains(&cid) || p.outputs.contains(&cid)
                    }
                    FlowRecord::Spec(_) | FlowRecord::Edge(_) => false,
                }
        });
        if known {
            return Ok((cid, target.to_string()));
        }
        return Err(FlowError::UnknownResource(format!(
            "{target} (a well-formed CID, but no record in this sidecar mints or names it)"
        )));
    }

    let canonical_root = std::fs::canonicalize(root).map_err(|source| FlowError::Read {
        path: root.to_path_buf(),
        source,
    })?;
    let abs = if Path::new(target).is_absolute() {
        std::path::PathBuf::from(target)
    } else {
        canonical_root.join(target)
    };
    let confined = confine_under(&canonical_root, &abs)?;
    let cid = body_cid_of_file(&confined)
        .ok_or_else(|| FlowError::UnknownResource(target.to_string()))?;
    Ok((cid, rel_to_root(&canonical_root, &confined)))
}

impl LedgerResult {
    /// Markdown, so the output is PASTED rather than transcribed.
    pub fn render(&self) {
        println!("## Ledger — {}", self.label);
        println!();
        if self.entries.is_empty() {
            println!("_No claims, fulfilments or notes recorded on this atom yet._");
            return;
        }
        for entry in &self.entries {
            let date = entry.occurred_at.get(..10).unwrap_or("unknown");
            let via = entry
                .via
                .as_ref()
                .map(|v| format!("via {v} · "))
                .unwrap_or_default();
            println!(
                "- {date} · {} · {} · {via}{}",
                entry.kind, entry.actor, entry.text
            );
        }
    }
}
