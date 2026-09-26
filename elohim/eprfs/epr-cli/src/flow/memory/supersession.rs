//! Cross-author supersession: an umbrella entry folds other contributions out of the projected
//! index, and does so only when the collective's process says it may.
//!
//! A curator consolidating the index under its byte ceiling writes a NEW entry, the umbrella, whose
//! frontmatter declares `supersedes: [<entry-name>, …]`. On import each named entry resolves to its
//! CURRENT contribution (request path + raw CID), recorded in the umbrella's
//! `Contribution::supersedes`. The umbrella is authored by its writer, by the ordinary rules; the
//! members are never modified, never re-authored, and stay visible to recall and attribution.
//!
//! ## When a fold takes effect
//!
//! * **Same author** — every superseded contribution has the same author as the umbrella (the
//!   graduation rule's sense of "same": the exact participant, or two builds of one agent role).
//!   Folding your own work needs no one's leave: the fold is effective on import.
//! * **Another author's work** — only once an approving verdict on the umbrella's contribution
//!   (`epr flow note --on <its flow resource CID> --kind verdict --verdict approved`) comes from an
//!   active Steward of the collective who is not the curator: the distinct-Steward path graduation
//!   and offers use ([`super::steward_approval`]). A fixture co-steward's approval takes effect and
//!   reads `bootstrap (fixture co-steward)`; a later `changes-requested` verdict from an active
//!   Steward withdraws it; a stewardless collective can never approve.
//!
//! Until a fold is effective, the umbrella AND its members stay indexed, and the umbrella's row says
//! the fold is pending. Hiding someone else's contribution from what every session loads is a
//! collective act; a curator alone cannot perform it.
//!
//! ## What is refused at import
//!
//! Superseding an entry that is not contributed (there is no contribution to point at), an
//! umbrella superseding itself, and a cycle (a member whose own supersession chain reaches the
//! umbrella). Each refusal names the entry.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use eprfs_agent::memory::Contribution;
use eprfs_core::BlobCid;
use serde_json::{json, Value};

use super::validation::{validated_at, Reader};
use super::{steward_approval, ContributionActs};

/// The label the projected row of a pending umbrella carries, spelled once.
pub(super) const PENDING_LABEL: &str = "[supersession pending a Steward's verdict]";

/// Whether a fold's member and its umbrella share an author, for GRANTING the same-author
/// bypass: the exact participant only. [`eprfs_agent::memory::Affiliation::same_author_as`]'s
/// role-wide match (any build of a role is any other) is deliberately over-inclusive for
/// REFUSING self-approval; used to grant a fold with no Steward review it would let one agent
/// role fold another invocation's work unreviewed (review of 98decb42f).
pub(super) fn same_author(a: &str, b: &str) -> bool {
    a == b
}

/// The exact command a distinct Steward runs to approve the fold resting on `resource`.
pub(super) fn approval_command(resource: &str) -> String {
    format!(
        "epr flow note --on {resource} --kind verdict --verdict approved --reason \"<why this \
         fold keeps what its members said>\" --session <your registered session>"
    )
}

/// The frontmatter `supersedes:` list: entry names (a trailing `.md` is accepted), inline.
///
/// `Ok(None)` when the key is absent. The block-list form (`supersedes:` with the items on the
/// following lines) is refused by name rather than read as empty: the one frontmatter parser reads
/// top-level scalars only, and a declaration that silently read as nothing would fold nothing
/// while its author believed otherwise.
pub(super) fn declared(fields: &BTreeMap<String, String>) -> Result<Option<Vec<String>>, String> {
    let Some(raw) = fields.get("supersedes") else {
        return Ok(None);
    };
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(
            "`supersedes:` is empty or a block list — write it as one inline list: \
                    `supersedes: [entry_one, entry_two]`"
                .into(),
        );
    }
    let inner = raw
        .strip_prefix('[')
        .and_then(|r| r.strip_suffix(']'))
        .unwrap_or(raw);
    let mut names = Vec::new();
    for item in inner.split(',') {
        let name = item.trim().trim_matches('"').trim_matches('\'').trim();
        let name = name.strip_suffix(".md").unwrap_or(name);
        if name.is_empty() {
            continue;
        }
        if name.contains('/') || name.contains('\\') || name.starts_with('.') {
            return Err(format!(
                "`supersedes:` names `{name}` — name entries of the same directory by file stem"
            ));
        }
        if !names.iter().any(|n| n == name) {
            names.push(name.to_string());
        }
    }
    if names.is_empty() {
        return Err("`supersedes:` names no entry".into());
    }
    Ok(Some(names))
}

/// Whether following superseded contributions from `start` (a request path) reaches `target`.
/// Reads the contribution files on disk; an unreadable link ends that branch.
pub(super) fn reaches(root: &Path, start: &str, target: &str) -> bool {
    let mut stack = vec![start.to_string()];
    let mut seen = BTreeSet::new();
    while let Some(path) = stack.pop() {
        if path == target {
            return true;
        }
        if !seen.insert(path.clone()) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(root.join(&path)) else {
            continue;
        };
        let Ok(contribution) = serde_json::from_str::<Contribution>(&text) else {
            continue;
        };
        stack.extend(contribution.supersedes.into_iter().map(|r| r.path));
    }
    false
}

/// One superseded contribution, as the umbrella pinned it and as it stands now.
#[derive(Debug, Clone)]
pub(super) struct Member {
    pub path: String,
    pub cid: String,
    /// The member's entry file, from its contribution's provenance.
    pub file: Option<String>,
    pub author: Option<String>,
    /// Whether the member's contribution on disk is still the one the umbrella pinned. A member
    /// re-contributed since is a different contribution the fold never named: it is not hidden.
    pub current: bool,
}

#[derive(Debug, Clone)]
pub(super) enum State {
    Effective {
        /// `same-author` or `steward-verdict`.
        basis: &'static str,
        approval: Option<Value>,
    },
    Pending {
        reason: String,
    },
}

/// One umbrella and the fold it declares.
#[derive(Debug, Clone)]
pub(super) struct Fold {
    /// The umbrella's entry file.
    pub umbrella: String,
    /// The umbrella's contribution request path.
    pub request: String,
    /// The umbrella contribution's flow resource CID — what a verdict names with `--on`.
    pub resource: String,
    pub author: String,
    pub members: Vec<Member>,
    pub state: State,
}

impl Fold {
    pub fn effective(&self) -> bool {
        matches!(self.state, State::Effective { .. })
    }

    pub fn report(&self) -> Value {
        let (state, basis, approval, reason) = match &self.state {
            State::Effective { basis, approval } => {
                ("effective", Some(*basis), approval.clone(), None)
            }
            State::Pending { reason } => ("pending", None, None, Some(reason.clone())),
        };
        json!({
            "umbrella": self.umbrella,
            "contribution": self.request,
            "resource": self.resource,
            "author": self.author,
            "state": state,
            "basis": basis,
            "approval": approval,
            "reason": reason,
            "approve": (!self.effective()).then(|| approval_command(&self.resource)),
            "members": self.members.iter().map(|m| json!({
                "entry": m.file,
                "contribution": m.path,
                "cid": m.cid,
                "author": m.author,
                "current": m.current,
            })).collect::<Vec<_>>(),
        })
    }
}

/// Every attributed umbrella in the contributions directory, with its fold's standing.
pub(super) fn load(reader: &mut Reader, dir_rel: &Path, acts: &ContributionActs) -> Vec<Fold> {
    let root = reader.root.clone();
    let Ok(items) = std::fs::read_dir(root.join(dir_rel)) else {
        return Vec::new();
    };
    let mut paths: Vec<_> = items
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "json"))
        .collect();
    paths.sort();
    let mut folds = Vec::new();
    for path in paths {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(umbrella) = serde_json::from_str::<Contribution>(&text) else {
            continue;
        };
        if umbrella.supersedes.is_empty() || !acts.holds(&text, &umbrella) {
            continue;
        }
        let request = rel_str(&dir_rel.join(path.file_name().unwrap_or_default()));
        let members: Vec<Member> = umbrella
            .supersedes
            .iter()
            .map(|pinned| {
                let text = std::fs::read_to_string(root.join(&pinned.path)).ok();
                let current = text
                    .as_deref()
                    .is_some_and(|t| BlobCid::compute_raw(t.as_bytes()).to_string() == pinned.cid);
                let parsed = text
                    .as_deref()
                    .and_then(|t| serde_json::from_str::<Contribution>(t).ok());
                Member {
                    path: pinned.path.clone(),
                    cid: pinned.cid.clone(),
                    file: parsed
                        .as_ref()
                        .and_then(|c| c.imported.as_ref().map(|i| i.file.clone())),
                    author: parsed.map(|c| c.author),
                    current,
                }
            })
            .collect();
        let resource = crate::flow::body_cid(&text).to_string();
        let state = if reaches_any(&root, &umbrella, &request) {
            State::Pending {
                reason: "its supersession chain reaches back to it (a cycle); a cycle never \
                         takes effect"
                    .into(),
            }
        } else if members.iter().filter(|m| m.current).all(|m| {
            m.author
                .as_deref()
                .is_some_and(|a| same_author(a, &umbrella.author))
        }) {
            State::Effective {
                basis: "same-author",
                approval: None,
            }
        } else {
            standing(reader, &umbrella, &resource, acts)
        };
        folds.push(Fold {
            umbrella: umbrella
                .imported
                .as_ref()
                .map(|i| i.file.clone())
                .unwrap_or_else(|| request.clone()),
            request,
            resource,
            author: umbrella.author.clone(),
            members,
            state,
        });
    }
    folds
}

fn reaches_any(root: &Path, umbrella: &Contribution, request: &str) -> bool {
    umbrella
        .supersedes
        .iter()
        .any(|m| reaches(root, &m.path, request))
}

/// A cross-author fold's standing, from the verdicts on the umbrella's contribution: the
/// distinct-Steward rule graduation uses, a later contrary verdict from an active Steward
/// withdrawing it, exactly as an offer's standing is folded.
fn standing(
    reader: &mut Reader,
    umbrella: &Contribution,
    resource: &str,
    acts: &ContributionActs,
) -> State {
    let governance = match reader.governance(&umbrella.collective.path) {
        Ok(g) => g,
        Err(e) => {
            return State::Pending {
                reason: format!("the umbrella's collective cannot be read: {e}"),
            }
        }
    };
    if let Err(e) = governance.require_stewarded("a cross-author supersession") {
        return State::Pending {
            reason: e.to_string(),
        };
    }
    let mut approval: Option<Value> = None;
    let mut ignored: Vec<String> = Vec::new();
    for verdict in acts.verdicts_on(resource) {
        if verdict.approved {
            match steward_approval(&governance, &verdict.provider, &umbrella.author) {
                Ok((affiliation_cid, affiliation)) => {
                    approval = Some(json!({
                        "verdict": verdict.record,
                        "approver": affiliation.member,
                        "affiliation": affiliation_cid,
                        "standing": affiliation.standing,
                        "validatedAt": validated_at(affiliation.standing),
                    }));
                }
                Err(why) => ignored.push(format!("{}: {why}", verdict.record)),
            }
        } else if governance
            .affiliation_of(&verdict.provider)
            .is_some_and(|(_, a)| a.is_active_steward())
        {
            approval = None;
            ignored.push(format!(
                "{}: a later changes-requested verdict by Steward {} withdrew any earlier approval",
                verdict.record, verdict.provider
            ));
        }
    }
    match approval {
        Some(approval) => State::Effective {
            basis: "steward-verdict",
            approval: Some(approval),
        },
        None => {
            let mut reason = format!(
                "it folds contributions by another author, so it needs an approving verdict from \
                 an active Steward of {} who is not its curator ({})",
                governance.declaration.id, umbrella.author
            );
            if let Some(last) = ignored.last() {
                reason.push_str(&format!("; not counted: {last}"));
            }
            State::Pending { reason }
        }
    }
}

/// The contributions an EFFECTIVE fold hides: request path → (pinned raw CID, umbrella entry).
/// Only a member still carrying the pinned bytes is hidden.
/// A member that is itself a PENDING umbrella is never hidden: its pending row and approval
/// advisory must stay visible, or a trivial same-author fold could quietly bury a cross-author
/// fold still waiting on a Steward (review of 98decb42f).
pub(super) fn hidden(folds: &[Fold]) -> BTreeMap<String, (String, String)> {
    let pending: std::collections::BTreeSet<&str> = folds
        .iter()
        .filter(|f| !f.effective())
        .map(|f| f.request.as_str())
        .collect();
    folds
        .iter()
        .filter(|f| f.effective())
        .flat_map(|f| {
            f.members
                .iter()
                .filter(|m| m.current && !pending.contains(m.path.as_str()))
                .map(move |m| (m.path.clone(), (m.cid.clone(), f.umbrella.clone())))
        })
        .collect()
}

fn rel_str(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
