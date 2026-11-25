//! `epr flow context <dir> --participate` — the participation read, cut 1
//! (`genesis/docs/superpowers/specs/2026-11-22-participation-read-design.md` §5 cut 1, P14–P16).
//!
//! **One caller (P14).** A session on this device reading this device's own stores, for whom
//! every local record is `self`. Nothing here serves any other caller; the first printed line
//! says so.
//!
//! **Opens nothing that creates (P15).** The flow log is read as bytes and decoded with
//! [`decode_flow_records`]; `SidecarFlowStore::open` is never called. Habits come through
//! [`registers::read_habits`]. Standing is read from captured roster and actor-log bytes
//! ([`crate::actor::device_state_snapshot`]), writing no root pin. Capacity is the device's own
//! `berth room cargo --json`, which is read-only and store-free. Moorings and sibling moorings
//! have no read-only JSON surface in berth (`berth status` prints text only), so they are
//! reported unavailable rather than parsed from text. Covenant scope is `unknown` because no
//! non-initialising reader exists.
//!
//! **Each source keeps its own meaning of open (P6)** and its own order (P8): nothing is scored
//! and nothing is sorted by worth. A commitment's bound is a ceiling and is never printed as
//! worth (P7). Role sets are sets, and an unresolved one prints `unknown` with its reason (P4).
//! The read records nothing, claims nothing and contacts no one (§6).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use cid::Cid;
use elohim_epr_rea::{decode_flow_records, CommitmentState, FlowEvent, FlowRecord};
use serde::{Deserialize, Serialize};

use super::intend::{own_state, OwnState, INTENT_TAG};
use super::read::{is_note, note_view, NoteView};
use super::registers::{self, HabitEntry};
use super::{body_cid_of_file, confine_under, rel_to_root, short_cid, FlowError, FlowResult};

/// How many items each source group prints before it says `… N more in <source>`, unless the
/// caller passes `--limit`.
pub const DEFAULT_LIMIT: usize = 8;

/// The largest `--limit` the read accepts.
pub const MAX_LIMIT: usize = 200;

/// The longest any one berth read may take before its source is reported unavailable.
const BERTH_TIMEOUT: Duration = Duration::from_secs(5);
/// The marker a berth carries when its `status --json` is the read-only presence surface. An
/// older berth builds a store on any `status` call, so one without the marker is never run.
const BERTH_STATUS_CONTRACT: &str = "status-json@1";

/// The one caller cut 1 serves (P14), as the first line names it.
pub const CALLER: &str = "this device's own records, for its steward's session";

const FLOW_LOG_REL: &str = ".eprfs/status/flows.jsonl";
const LABELS_REL: &str = ".eprfs/status/labels.json";
const BERTH_REL: &str = "genesis/agentic/bin/berth";
const NOT_CLAIMABLE: &str = "not applicable (presence is a signal, not claimable work)";
const EXCLUDED_SCOPE: &str = "counts of in-scope records this caller can see";
const COVENANT_UNKNOWN: &str = "no non-initialising reader";
const REACHABILITY: &str = "reachability undeclared";

pub const PRESENCE: &str = "presence";
pub const STATIONS: &str = "stations";
pub const COMMITMENTS: &str = "commitments";
pub const HABITS: &str = "habits";
pub const LANDING_INTENTS: &str = "landing intents";
pub const NOTES: &str = "notes";

/// A field the evidence establishes, or `unknown` with the reason it does not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Known<T> {
    Known(T),
    Unknown {
        unknown: String,
    },
    /// The field has no meaning for this kind of item: presence is a signal, not claimable work.
    NotApplicable {
        #[serde(rename = "notApplicable")]
        not_applicable: String,
    },
}

impl<T> Known<T> {
    fn unknown(reason: impl Into<String>) -> Self {
        Known::Unknown {
            unknown: reason.into(),
        }
    }
}

/// Where the read looked, and for whom.
#[derive(Debug, Serialize)]
pub struct Scope {
    /// The directory, repo-relative; `.` for the root.
    pub path: String,
    pub caller: String,
}

/// One source and whether this read could consume it. `more` counts only items this caller
/// sees that the item cap left unprinted.
#[derive(Debug, Serialize)]
pub struct SourceView {
    pub source: String,
    /// `available`, `partial` or `unavailable`.
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub more: usize,
    /// On `sibling moorings`: each of the steward's other devices that is silent or has not
    /// acknowledged this device's records, as `berth status --json` read it.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub devices: Vec<DeviceNote>,
}

/// One of the steward's other devices whose exchange with this one is not current.
#[derive(Debug, Clone, Serialize)]
pub struct DeviceNote {
    pub device: String,
    pub note: String,
}

/// The device's own capacity answer (`berth room cargo --json`).
#[derive(Debug, Clone, Serialize)]
pub struct Capacity {
    pub verdict: String,
    pub reason: Option<String>,
    pub axis: Option<String>,
    pub owner: Option<String>,
}

/// Stewards, gifters and receivers: each a set, never a single beneficiary (P4).
#[derive(Debug, Clone, Serialize)]
pub struct Roles {
    pub stewards: Known<Vec<String>>,
    pub gifters: Known<Vec<String>>,
    pub receivers: Known<Vec<String>>,
}

/// One recorded unit, with its source and time (P7). The value is the recorded slot verbatim.
#[derive(Debug, Clone, Serialize)]
pub struct Worth {
    pub unit: String,
    pub value: String,
    pub source: String,
    pub at: String,
}

/// What has been said about an item, by its own author.
#[derive(Debug, Clone, Serialize)]
pub struct Testimony {
    pub author: String,
    #[serde(rename = "ref")]
    pub reference: String,
    pub at: String,
    /// Run wall-clock time for gate-verdict testimony. Legacy gate verdicts say `unknown`;
    /// testimony of every other kind omits this field.
    #[serde(rename = "observedAt", skip_serializing_if = "Option::is_none")]
    pub observed_at: Option<String>,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub says: Option<String>,
}

/// Filters and annotations with reasons (P8); never a merit number.
#[derive(Debug, Clone, Serialize)]
pub struct Fit {
    pub covenant: Known<String>,
    pub capacity: Known<String>,
    pub graph: Known<String>,
}

/// Human presence (P3, P9, P11): only the effect, never a reason, and never capacity terms.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub ready_for: Known<String>,
}

/// One candidate, in its source's own terms (P16).
#[derive(Debug, Clone, Serialize)]
pub struct Item {
    #[serde(rename = "ref")]
    pub reference: String,
    pub state: String,
    pub serves: Known<Vec<String>>,
    pub roles: Roles,
    pub standing: Known<String>,
    pub worth: Known<Vec<Worth>>,
    pub testimony: Vec<Testimony>,
    pub taking: String,
    pub fit: Fit,
    pub person: Person,
    pub agent: Known<String>,
}

/// One source's items, in that source's own order.
#[derive(Debug, Serialize)]
pub struct Group {
    pub source: String,
    pub items: Vec<Item>,
}

/// In-scope items this caller sees but that are not open by their source's meaning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Excluded {
    pub source: String,
    pub reason: String,
    pub count: usize,
}

/// The whole read. Keys are exactly the ones P16 lists.
#[derive(Debug, Serialize)]
pub struct ParticipationResult {
    pub scope: Scope,
    pub completeness: Known<String>,
    pub sources: Vec<SourceView>,
    pub capacity: Known<Capacity>,
    pub grant: Known<String>,
    pub covenant: Known<String>,
    pub groups: Vec<Group>,
    pub excluded: Vec<Excluded>,
    pub outcome: String,
}

/// The process-wide inputs the read takes from its environment, named so tests can supply them.
#[derive(Debug, Clone, Default)]
pub struct Probes {
    /// The berth executable; `None` resolves `BERTH_BIN`, then `<root>/genesis/agentic/bin/berth`.
    pub berth: Option<PathBuf>,
    /// The device key file; `None` resolves it the way every command does.
    pub device_key: Option<PathBuf>,
}

/// `epr flow context <dir> --participate [--limit N]`, reading the process environment.
pub fn participate(root: &Path, target: &str, limit: usize) -> FlowResult<ParticipationResult> {
    participate_with(root, target, limit, &Probes::default())
}

/// [`participate`] with its environment inputs named. `limit` is the per-source item count.
pub fn participate_with(
    root: &Path,
    target: &str,
    limit: usize,
    probes: &Probes,
) -> FlowResult<ParticipationResult> {
    if !(1..=MAX_LIMIT).contains(&limit) {
        return Err(FlowError::InvalidArguments(format!(
            "--limit takes a count from 1 to {MAX_LIMIT}, not {limit}"
        )));
    }
    let canonical_root = std::fs::canonicalize(root).map_err(|source| FlowError::Read {
        path: root.to_path_buf(),
        source,
    })?;
    let abs = if Path::new(target).is_absolute() {
        PathBuf::from(target)
    } else {
        canonical_root.join(target)
    };
    if !abs.is_dir() {
        return Err(FlowError::InvalidArguments(format!(
            "--participate takes a directory under the root; `{target}` is not one"
        )));
    }
    let confined = confine_under(&canonical_root, &abs)?;
    let dir = rel_to_root(&canonical_root, &confined);
    let scope = Scoper::new(&dir);

    let standing = standing(&canonical_root, probes);
    let capacity = capacity(&canonical_root, probes);
    let capacity_fit = match &capacity {
        Known::Known(answer) => Known::Known(match &answer.reason {
            Some(reason) => format!("{} — {reason} (this device's own answer)", answer.verdict),
            None => format!("{} (this device's own answer)", answer.verdict),
        }),
        other => known_reason(other),
    };
    let presence = presence(&canonical_root, probes);
    let agent = match &presence {
        Ok(status) => match status.gap("moorings") {
            Some(reason) => Known::unknown(format!("moorings unavailable: {reason}")),
            None => Known::Known(format!(
                "running: {} session(s) on this device, {} on siblings",
                status.moorings.len(),
                status
                    .gap("siblings")
                    .map_or_else(|| status.siblings.len().to_string(), |_| "unknown".into())
            )),
        },
        Err(reason) => Known::unknown(format!(
            "no running or invocable evidence: moorings are unavailable ({reason})"
        )),
    };
    let base = ItemBase {
        standing,
        fit: Fit {
            covenant: Known::unknown(COVENANT_UNKNOWN),
            capacity: capacity_fit,
            graph: Known::unknown("shared/upstream/downstream relation is not composed in cut 1"),
        },
        agent,
    };

    let mut sources = Vec::new();
    let mut groups = Vec::new();
    let mut excluded = Vec::new();

    // Presence first: it answers "where can I participate". A session is a signal, never a
    // person's presence (P3, P11).
    match &presence {
        Ok(status) => {
            sources.push(match status.gap("moorings") {
                Some(reason) => unavailable("moorings", reason),
                None => available("moorings"),
            });
            let mut siblings = match status.gap("siblings") {
                Some(reason) => unavailable("sibling moorings", reason),
                None => available("sibling moorings"),
            };
            siblings.devices = status.device_notes();
            sources.push(siblings);
            for gap in &status.unavailable {
                if gap.source != "moorings" && gap.source != "siblings" {
                    sources.push(unavailable(
                        &format!("berth {}", gap.source),
                        gap.reason.clone(),
                    ));
                }
            }
            groups.push(Group {
                source: PRESENCE.into(),
                items: presence_items(status, &scope, &base),
            });
        }
        Err(reason) => {
            sources.push(unavailable("moorings", reason.clone()));
            sources.push(unavailable("sibling moorings", reason.clone()));
        }
    }

    let flows = read_flows(&canonical_root);
    let labels = read_labels(&canonical_root);
    let repo_scope = super::repository_scope(&canonical_root).ok();

    // Flow-backed sources.
    match &flows {
        Err(reason) => {
            for name in [STATIONS, COMMITMENTS, LANDING_INTENTS, NOTES] {
                sources.push(unavailable(name, reason.clone()));
            }
        }
        Ok((records, partial)) => {
            let status = if partial.is_some() {
                "partial"
            } else {
                "available"
            };
            let resolver = Resolver::new(&canonical_root, records, &labels, repo_scope);
            let ctx = FlowCtx {
                records,
                resolver: &resolver,
                scope: &scope,
                base: &base,
            };
            let mut labelled = |name: &str, group: Option<(Vec<Item>, Vec<Excluded>)>| match group {
                Some((items, excl)) => {
                    sources.push(SourceView {
                        source: name.to_string(),
                        status: status.to_string(),
                        reason: partial.clone(),
                        more: 0,
                        devices: Vec::new(),
                    });
                    excluded.extend(excl);
                    groups.push(Group {
                        source: name.to_string(),
                        items,
                    });
                }
                None => sources.push(unavailable(
                    name,
                    labels
                        .as_ref()
                        .err()
                        .cloned()
                        .unwrap_or_else(|| "subject paths are not recoverable".into()),
                )),
            };
            // Stations and commitments name their subject by a scope CID, recoverable only
            // through the label map; without it their paths are not recoverable from a snapshot.
            let path_bearing = labels.is_ok() || records.is_empty();
            labelled(STATIONS, path_bearing.then(|| stations(&ctx)));
            labelled(COMMITMENTS, path_bearing.then(|| commitments(&ctx)));
            labelled(LANDING_INTENTS, Some(landing_intents(&ctx)));
            labelled(NOTES, Some(notes(&ctx)));
        }
    }

    // Habits: the habit reader, independent of the flow log. Inserted in source-table order.
    let notes_for_habits: Vec<(Cid, FlowEvent)> = match &flows {
        Ok((records, _)) => records
            .iter()
            .filter_map(|(cid, record)| match record {
                FlowRecord::Event(event) if is_note(event) => Some((*cid, event.clone())),
                _ => None,
            })
            .collect(),
        Err(_) => Vec::new(),
    };
    let habit_position = sources
        .iter()
        .position(|s| s.source == LANDING_INTENTS)
        .unwrap_or(sources.len());
    match registers::read_habits(&canonical_root) {
        Ok(rows) => {
            let (items, excl) = habits(&rows, &scope, &base, &notes_for_habits);
            sources.insert(habit_position, available(HABITS));
            let group_position = groups
                .iter()
                .position(|g| g.source == LANDING_INTENTS)
                .unwrap_or(groups.len());
            groups.insert(
                group_position,
                Group {
                    source: HABITS.into(),
                    items,
                },
            );
            excluded.extend(excl);
        }
        Err(error) => sources.insert(habit_position, unavailable(HABITS, error.to_string())),
    }

    sources.push(match &capacity {
        Known::Known(_) => available("capacity"),
        Known::Unknown { unknown }
        | Known::NotApplicable {
            not_applicable: unknown,
        } => unavailable("capacity", unknown.clone()),
    });

    // The cap, per source and in source order: nothing is moved ahead of anything by worth.
    for group in &mut groups {
        let shown = group.items.len().min(limit);
        let more = group.items.len() - shown;
        group.items.truncate(shown);
        if let Some(view) = sources.iter_mut().find(|s| s.source == group.source) {
            view.more = more;
        }
    }

    let any = groups.iter().any(|g| !g.items.is_empty());
    let shown_dir = if dir.is_empty() { ".".into() } else { dir };
    Ok(ParticipationResult {
        scope: Scope {
            path: shown_dir,
            caller: CALLER.into(),
        },
        completeness: Known::unknown(
            "only this device's own stores are read; other devices and unavailable sources are \
             not composed",
        ),
        sources,
        capacity,
        grant: Known::unknown("no grant reference is composed in cut 1"),
        covenant: Known::unknown(COVENANT_UNKNOWN),
        groups,
        excluded,
        outcome: if any {
            "candidates".into()
        } else {
            "no visible candidate established".into()
        },
    })
}

fn unavailable(name: &str, reason: String) -> SourceView {
    SourceView {
        source: name.into(),
        status: "unavailable".into(),
        reason: Some(reason),
        more: 0,
        devices: Vec::new(),
    }
}

fn available(name: &str) -> SourceView {
    SourceView {
        source: name.into(),
        status: "available".into(),
        reason: None,
        more: 0,
        devices: Vec::new(),
    }
}

/// A field that is not `Known`, carried over with its own reason.
fn known_reason<T, U>(value: &Known<T>) -> Known<U> {
    match value {
        Known::Known(_) => Known::unknown("established"),
        Known::Unknown { unknown } => Known::unknown(unknown.clone()),
        Known::NotApplicable { not_applicable } => Known::NotApplicable {
            not_applicable: not_applicable.clone(),
        },
    }
}

/// The per-item fields that do not vary by item in cut 1.
struct ItemBase {
    standing: Known<String>,
    fit: Fit,
    agent: Known<String>,
}

impl ItemBase {
    fn item(&self, parts: Parts) -> Item {
        let Parts(reference, state, serves, roles, worth, testimony, taking) = parts;
        Item {
            reference,
            state,
            serves,
            roles,
            standing: self.standing.clone(),
            worth,
            testimony,
            taking,
            fit: self.fit.clone(),
            person: Person {
                ready_for: Known::unknown(REACHABILITY),
            },
            agent: self.agent.clone(),
        }
    }
}

/// The per-item fields each source supplies, in the order an item prints them: reference,
/// state, serves, roles, worth, testimony, taking.
struct Parts(
    String,
    String,
    Known<Vec<String>>,
    Roles,
    Known<Vec<Worth>>,
    Vec<Testimony>,
    String,
);

/// Whether a repo-relative subject path lies under the scope directory.
struct Scoper {
    dir: String,
}

impl Scoper {
    fn new(dir: &str) -> Self {
        Self {
            dir: dir.trim_end_matches('/').to_string(),
        }
    }

    fn covers(&self, path: &str) -> bool {
        if path.starts_with('/') || path.split('/').any(|part| part == "..") {
            return false;
        }
        self.dir.is_empty() || path == self.dir || path.starts_with(&format!("{}/", self.dir))
    }
}

/// The flow log's records, read as bytes (P15). `Ok((records, Some(note)))` when an unterminated
/// tail line (an append in progress) was left unread; `Err` names why the log is unavailable.
type FlowSnapshot = Result<(Vec<(Cid, FlowRecord)>, Option<String>), String>;

fn read_flows(root: &Path) -> FlowSnapshot {
    let text = match std::fs::read_to_string(root.join(FLOW_LOG_REL)) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(format!("no flow store at {FLOW_LOG_REL}"))
        }
        Err(error) => return Err(format!("{FLOW_LOG_REL} could not be read: {error}")),
    };
    let (complete, partial) = if text.is_empty() || text.ends_with('\n') {
        (text.as_str(), None)
    } else {
        let end = text.rfind('\n').map_or(0, |at| at + 1);
        (
            &text[..end],
            Some("an unterminated last line (an append in progress) was not read".to_string()),
        )
    };
    decode_flow_records(complete)
        .map(|records| (records, partial))
        .map_err(|error| format!("{FLOW_LOG_REL} could not be decoded: {error}"))
}

fn read_labels(root: &Path) -> Result<super::Labels, String> {
    let text = match std::fs::read_to_string(root.join(LABELS_REL)) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(format!(
                "subject paths are not recoverable: no label map at {LABELS_REL}"
            ))
        }
        Err(error) => {
            return Err(format!(
                "subject paths are not recoverable: {LABELS_REL} could not be read: {error}"
            ))
        }
    };
    serde_json::from_str(&text).map_err(|error| {
        format!("subject paths are not recoverable: {LABELS_REL} does not parse: {error}")
    })
}

/// Where a record's subject lies.
enum Subject {
    /// No recorded path, or a path outside the scope directory: it moves nothing in this read.
    Outside,
    /// Under the directory, scoped to the file's current version (or to the repository itself).
    Current(String),
    /// Under the directory, scoped to a version of the file that is no longer its contents.
    Superseded,
}

/// Subject paths from the snapshot: the label map, the repository scope, and each labelled
/// file's current body CID (computed once per path, and only for paths under the directory).
struct Resolver<'a> {
    root: &'a Path,
    labels: Option<&'a super::Labels>,
    repo_scope: Option<Cid>,
    current: std::cell::RefCell<HashMap<String, Option<Cid>>>,
    /// Record CID → the scope CID of the intent or commitment it names, for notes on records.
    record_scope: HashMap<Cid, Cid>,
}

impl<'a> Resolver<'a> {
    fn new(
        root: &'a Path,
        records: &'a [(Cid, FlowRecord)],
        labels: &'a Result<super::Labels, String>,
        repo_scope: Option<Cid>,
    ) -> Self {
        let record_scope = records
            .iter()
            .filter_map(|(cid, record)| match record {
                FlowRecord::Intent(intent) => Some((*cid, intent.in_scope_of)),
                FlowRecord::Commitment(commitment) => Some((*cid, commitment.in_scope_of)),
                _ => None,
            })
            .collect();
        Self {
            root,
            labels: labels.as_ref().ok(),
            repo_scope,
            current: std::cell::RefCell::new(HashMap::new()),
            record_scope,
        }
    }

    fn path_of(&self, scope: &Cid) -> Option<String> {
        if Some(*scope) == self.repo_scope {
            return Some(String::new());
        }
        self.labels?.get(&scope.to_string()).cloned()
    }

    fn subject(&self, scope: &Cid, scoper: &Scoper) -> Subject {
        let Some(path) = self.path_of(scope) else {
            return Subject::Outside;
        };
        if path.is_empty() {
            return if scoper.dir.is_empty() {
                Subject::Current(".".into())
            } else {
                Subject::Outside
            };
        }
        if !scoper.covers(&path) {
            return Subject::Outside;
        }
        let current = *self
            .current
            .borrow_mut()
            .entry(path.clone())
            .or_insert_with(|| body_cid_of_file(&self.root.join(&path)));
        if current == Some(*scope) {
            Subject::Current(path)
        } else {
            Subject::Superseded
        }
    }

    /// A note's subject path: its recorded subject slot when that is a path, else the path of
    /// the record or file it was recorded on. No version check: testimony stays testimony.
    fn note_path(&self, event: &FlowEvent, view: &NoteView) -> Option<String> {
        if !view.subject.is_empty() && view.subject.parse::<Cid>().is_err() {
            return Some(view.subject.clone());
        }
        let scope = self
            .record_scope
            .get(&event.resource)
            .copied()
            .unwrap_or(event.resource);
        self.path_of(&scope)
            .or_else(|| self.path_of(&event.in_scope_of))
    }
}

struct FlowCtx<'a> {
    records: &'a [(Cid, FlowRecord)],
    resolver: &'a Resolver<'a>,
    scope: &'a Scoper,
    base: &'a ItemBase,
}

impl FlowCtx<'_> {
    /// Notes recorded ON `resource`, oldest first, by their own authors.
    fn testimony_on(&self, resource: &Cid) -> Vec<Testimony> {
        self.records
            .iter()
            .filter_map(|(cid, record)| match record {
                FlowRecord::Event(event) if is_note(event) && event.resource == *resource => {
                    Some(testimony(cid, event))
                }
                _ => None,
            })
            .collect()
    }

    fn discharged(&self) -> HashSet<Cid> {
        self.records
            .iter()
            .filter_map(|(_, record)| match record {
                FlowRecord::Event(event) => Some(event.fulfills.iter().copied()),
                _ => None,
            })
            .flatten()
            .collect()
    }
}

fn testimony(cid: &Cid, event: &FlowEvent) -> Testimony {
    let view = note_view(cid, event);
    let observed_at = super::read::gate_verdict_observed_at(&view)
        .map(|value| value.unwrap_or("unknown").to_string());
    Testimony {
        author: view.actor,
        reference: cid.to_string(),
        at: view.occurred_at,
        observed_at,
        kind: view.kind,
        says: view.reason.or(view.verdict),
    }
}

/// Count an in-scope record that is not open, under its reason.
fn exclude(tally: &mut BTreeMap<String, usize>, order: &mut Vec<String>, reason: &str) {
    if !tally.contains_key(reason) {
        order.push(reason.to_string());
    }
    *tally.entry(reason.to_string()).or_insert(0) += 1;
}

fn excluded_rows(source: &str, tally: &BTreeMap<String, usize>, order: &[String]) -> Vec<Excluded> {
    order
        .iter()
        .map(|reason| Excluded {
            source: source.into(),
            reason: reason.clone(),
            count: tally[reason],
        })
        .collect()
}

fn no_link(role: &str, what: &str) -> Known<Vec<String>> {
    Known::unknown(format!("no recorded {role} link on this {what}"))
}

const SUPERSEDED: &str = "scoped to a superseded version of its document";

/// Stations (P6): an unticked intent is open; a ticked one is a claim, not a fulfilment.
fn stations(ctx: &FlowCtx<'_>) -> (Vec<Item>, Vec<Excluded>) {
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    let (mut tally, mut order) = (BTreeMap::new(), Vec::new());
    for (cid, record) in ctx.records {
        let FlowRecord::Intent(intent) = record else {
            continue;
        };
        let Some(tag) = intent.resource_spec.classified_as.first() else {
            continue;
        };
        let Some(state) = tag.strip_prefix("gap:") else {
            continue;
        };
        if !seen.insert(*cid) {
            continue;
        }
        let path = match ctx.resolver.subject(&intent.in_scope_of, ctx.scope) {
            Subject::Outside => continue,
            Subject::Superseded => {
                exclude(&mut tally, &mut order, SUPERSEDED);
                continue;
            }
            Subject::Current(path) => path,
        };
        if state != "open" {
            let reason = if state == "claimed" {
                "ticked: claimed, not fulfilled — its claim commitment is read under commitments"
                    .to_string()
            } else {
                format!("station state gap:{state}")
            };
            exclude(&mut tally, &mut order, &reason);
            continue;
        }
        let claimants: Vec<String> = ctx
            .records
            .iter()
            .filter_map(|(_, record)| match record {
                FlowRecord::Commitment(c) if c.satisfies.contains(cid) => {
                    Some(c.provider.0.clone())
                }
                _ => None,
            })
            .collect();
        let gap_id = intent.resource_spec.classified_as.get(1).cloned();
        let mut state_text = format!("open — unticked intent raised by {}", intent.raised_by.0);
        if !claimants.is_empty() {
            state_text.push_str(&format!("; claimed by {}", claimants.join(", ")));
        }
        let mut serves = vec![path];
        serves.extend(gap_id.clone());
        items.push(ctx.base.item(Parts(
            gap_id.clone().unwrap_or_else(|| cid.to_string()),
            state_text,
            Known::Known(serves),
            Roles {
                stewards: no_link("steward", "station"),
                gifters: no_link("gifter", "station"),
                receivers: no_link("receiver", "station"),
            },
            Known::unknown("no recorded unit on a station"),
            ctx.testimony_on(cid),
            format!(
                "a claim commitment on intent {} (`epr flow claim --on {}`) — a claim, not a \
                 fulfilment",
                short_cid(cid),
                gap_id.unwrap_or_else(|| cid.to_string())
            ),
        )));
    }
    (items, excluded_rows(STATIONS, &tally, &order))
}

/// Commitments (P6): undischarged, possibly owned — the owner is shown and the item never hidden.
/// A declared bound is a ceiling, never worth (P7), so it is not printed at all.
fn commitments(ctx: &FlowCtx<'_>) -> (Vec<Item>, Vec<Excluded>) {
    let discharged = ctx.discharged();
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    let (mut tally, mut order) = (BTreeMap::new(), Vec::new());
    for (cid, record) in ctx.records {
        let FlowRecord::Commitment(commitment) = record else {
            continue;
        };
        if !seen.insert(*cid) {
            continue;
        }
        let path = match ctx.resolver.subject(&commitment.in_scope_of, ctx.scope) {
            Subject::Outside => continue,
            Subject::Superseded => {
                exclude(&mut tally, &mut order, SUPERSEDED);
                continue;
            }
            Subject::Current(path) => path,
        };
        match commitment.state {
            CommitmentState::Fulfilled => {
                exclude(&mut tally, &mut order, "fulfilled");
                continue;
            }
            CommitmentState::Revoked => {
                exclude(&mut tally, &mut order, "revoked");
                continue;
            }
            CommitmentState::Proposed | CommitmentState::Active => {}
        }
        if discharged.contains(cid) {
            exclude(&mut tally, &mut order, "discharged by a recorded event");
            continue;
        }
        let slots = &commitment.resource_spec.classified_as;
        let mut serves = vec![path];
        serves.extend(slots.get(1).cloned());
        serves.extend(
            slots
                .iter()
                .filter(|s| s.starts_with("habit:") || s.starts_with("brief:"))
                .cloned(),
        );
        serves.extend(
            commitment
                .satisfies
                .iter()
                .map(|intent| format!("intent {}", short_cid(intent))),
        );
        let provider = &commitment.provider.0;
        items.push(ctx.base.item(Parts(
            cid.to_string(),
            format!(
                "undischarged ({}); owned by {provider}",
                match commitment.state {
                    CommitmentState::Proposed => "proposed",
                    _ => "active",
                }
            ),
            Known::Known(serves),
            Roles {
                stewards: no_link("steward", "commitment"),
                gifters: Known::unknown(format!(
                    "provider {provider} is recorded; a provider is not a gifter by implication"
                )),
                receivers: Known::unknown(format!(
                    "receiver {} is recorded; a receiver field is not a receiver set by \
                     implication",
                    commitment.receiver.0
                )),
            },
            Known::unknown("no recorded unit joined to this commitment (a declared bound is a ceiling, not worth)"),
            ctx.testimony_on(cid),
            format!(
                "a fulfilment event discharging {} (`epr flow fulfill --on {}`), by its owner \
                 {provider}",
                short_cid(cid),
                cid
            ),
        )));
    }
    (items, excluded_rows(COMMITMENTS, &tally, &order))
}

/// Landing intents (P6): live until landed, withdrawn, dismissed or superseded. They are scoped
/// to the repository, so they are in scope only when the directory is the root.
fn landing_intents(ctx: &FlowCtx<'_>) -> (Vec<Item>, Vec<Excluded>) {
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    let (mut tally, mut order) = (BTreeMap::new(), Vec::new());
    for (cid, record) in ctx.records {
        let FlowRecord::Intent(intent) = record else {
            continue;
        };
        let slots = &intent.resource_spec.classified_as;
        if slots.first().map(String::as_str) != Some(INTENT_TAG) || !seen.insert(*cid) {
            continue;
        }
        if !matches!(
            ctx.resolver.subject(&intent.in_scope_of, ctx.scope),
            Subject::Current(_)
        ) {
            continue;
        }
        let state = own_state(ctx.records, cid);
        let word = match state {
            OwnState::Live => None,
            OwnState::Landed => Some("landed"),
            OwnState::Withdrawn => Some("withdrawn"),
            OwnState::Dismissed => Some("dismissed"),
            OwnState::Superseded => Some("superseded"),
        };
        if let Some(word) = word {
            exclude(&mut tally, &mut order, word);
            continue;
        }
        let mut serves: Vec<String> = slots.get(1).cloned().into_iter().collect();
        serves.extend(
            slots
                .iter()
                .filter(|s| s.starts_with("class:") || s.starts_with("closure:"))
                .cloned(),
        );
        let worth: Vec<Worth> = ctx
            .records
            .iter()
            .filter_map(|(event_cid, record)| match record {
                FlowRecord::Event(event)
                    if event.resource == *cid
                        && event.classified_as.first().map(String::as_str)
                            == Some("dev-landing:intent") =>
                {
                    event
                        .classified_as
                        .iter()
                        .find_map(|slot| slot.strip_prefix("env:gate_s="))
                        .map(|value| Worth {
                            unit: "gate seconds".into(),
                            value: value.to_string(),
                            source: event_cid.to_string(),
                            at: event.occurred_at.clone(),
                        })
                }
                _ => None,
            })
            .collect();
        items.push(ctx.base.item(Parts(
            cid.to_string(),
            format!(
                "live — until landed, withdrawn, dismissed or superseded; raised by {}",
                intent.raised_by.0
            ),
            Known::Known(serves),
            Roles {
                stewards: no_link("steward", "landing intent"),
                gifters: no_link("gifter", "landing intent"),
                receivers: no_link("receiver", "landing intent"),
            },
            if worth.is_empty() {
                Known::unknown("no gate seconds recorded on this intent")
            } else {
                Known::Known(worth)
            },
            ctx.testimony_on(cid),
            format!(
                "nothing another participant takes: its raiser settles it (`epr flow intend \
                 --land|--withdraw|--dismiss {}`)",
                short_cid(cid)
            ),
        )));
    }
    (items, excluded_rows(LANDING_INTENTS, &tally, &order))
}

/// Notes (P6): testimony, never a claimable commitment.
fn notes(ctx: &FlowCtx<'_>) -> (Vec<Item>, Vec<Excluded>) {
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    for (cid, record) in ctx.records {
        let FlowRecord::Event(event) = record else {
            continue;
        };
        if !is_note(event) || !seen.insert(*cid) {
            continue;
        }
        let view = note_view(cid, event);
        let Some(path) = ctx.resolver.note_path(event, &view) else {
            continue;
        };
        if !ctx.scope.covers(&path) {
            continue;
        }
        let slot = |prefix: &str| {
            view.extra
                .iter()
                .find_map(|s| s.strip_prefix(prefix).map(str::to_string))
        };
        let worth = match slot("value:") {
            Some(value) => Known::Known(vec![Worth {
                unit: slot("unit:")
                    .or_else(|| slot("measure:"))
                    .unwrap_or_else(|| "unnamed unit".into()),
                value,
                source: cid.to_string(),
                at: view.occurred_at.clone(),
            }]),
            None => Known::unknown("no recorded unit on this note"),
        };
        let stewards = match &view.steward {
            Some(steward) => Known::Known(vec![steward.clone()]),
            None => no_link("steward", "note"),
        };
        let mut serves = vec![path];
        if view.subject.parse::<Cid>().is_ok() {
            serves.push(view.subject.clone());
        }
        items.push(ctx.base.item(Parts(
            cid.to_string(),
            format!("testimony ({}) — not claimable", view.kind),
            Known::Known(serves),
            Roles {
                stewards,
                gifters: no_link("gifter", "note"),
                receivers: no_link("receiver", "note"),
            },
            worth,
            vec![testimony(cid, event)],
            "nothing: testimony is not claimable".into(),
        )));
    }
    (items, Vec::new())
}

/// Habits (P6): declared red or unwired is open. The declaration is read; evidence is not
/// joined in cut 1, so a disagreement between them is not read here and the state says so.
fn habits(
    rows: &[HabitEntry],
    scope: &Scoper,
    base: &ItemBase,
    notes: &[(Cid, FlowEvent)],
) -> (Vec<Item>, Vec<Excluded>) {
    let mut items = Vec::new();
    let (mut tally, mut order) = (BTreeMap::new(), Vec::new());
    for habit in rows {
        let Some(path) = habit.source.as_deref() else {
            continue;
        };
        if !scope.covers(path) {
            continue;
        }
        if habit.status != "red" && habit.status != "unwired" {
            let reason = if habit.status.is_empty() {
                "no declared status".to_string()
            } else {
                habit.status.clone()
            };
            exclude(&mut tally, &mut order, &reason);
            continue;
        }
        let mut serves = vec![format!("habit:{}", habit.id), path.to_string()];
        serves.extend(habit.checks.iter().map(|check| format!("check: {check}")));
        let testimony = notes
            .iter()
            .filter(|(cid, event)| note_view(cid, event).subject == path)
            .map(|(cid, event)| testimony(cid, event))
            .collect();
        items.push(base.item(Parts(
            habit.id.clone(),
            format!(
                "declared {}{} — evidence is not joined in cut 1",
                habit.status,
                if habit.active { ", active" } else { "" }
            ),
            Known::Known(serves),
            Roles {
                stewards: no_link("steward", "habit"),
                gifters: no_link("gifter", "habit"),
                receivers: no_link("receiver", "habit"),
            },
            Known::unknown("red mass is not read in cut 1"),
            testimony,
            format!(
                "a commitment carrying habit:{} (`epr flow claim --on <intent> --serves {}`)",
                habit.id, habit.id
            ),
        )));
    }
    (items, excluded_rows(HABITS, &tally, &order))
}

/// This device's standing, read without opening any store (P4, P15).
fn standing(root: &Path, probes: &Probes) -> Known<String> {
    let key = match &probes.device_key {
        Some(key) => key.clone(),
        None => match crate::device_key::resolve_path() {
            Ok(key) => key,
            Err(error) => return Known::unknown(format!("no device key resolves: {error}")),
        },
    };
    match crate::actor::device_state_snapshot(root, &key) {
        Ok(crate::actor::DeviceState::Standing(standing)) => Known::Known(format!(
            "{} by native standing — one handle, not authenticated membership in a space",
            standing.subject
        )),
        Ok(crate::actor::DeviceState::RootContested(contest)) => Known::unknown(format!(
            "{}'s roster roots elsewhere than this device pinned; standing is contested",
            contest.subject
        )),
        Ok(crate::actor::DeviceState::Unwitnessed) => {
            Known::unknown("this device stands on no roster here")
        }
        Err(reason) => Known::unknown(reason),
    }
}

/// The device's own capacity answer: `berth room cargo --json`, read-only and store-free.
fn berth_path(root: &Path, probes: &Probes) -> PathBuf {
    probes
        .berth
        .clone()
        .or_else(|| std::env::var_os("BERTH_BIN").map(PathBuf::from))
        .unwrap_or_else(|| root.join(BERTH_REL))
}

fn capacity(root: &Path, probes: &Probes) -> Known<Capacity> {
    let berth = berth_path(root, probes);
    if !berth.is_file() {
        return Known::unknown(format!("berth not found at {}", berth.display()));
    }
    let output = match run_bounded(&berth, &["room", "cargo", "--json"], BERTH_TIMEOUT) {
        Ok(output) => output,
        Err(reason) => return Known::unknown(reason),
    };
    // 0 admits and 3 refuses; both are the device's answer.
    if !matches!(output.0, Some(0) | Some(3)) {
        return Known::unknown(format!(
            "berth room cargo --json exited {}",
            output
                .0
                .map_or_else(|| "by signal".to_string(), |code| code.to_string())
        ));
    }
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&output.1) else {
        return Known::unknown("berth room cargo --json printed no JSON answer");
    };
    let text = |key: &str| value.get(key).and_then(|v| v.as_str()).map(str::to_string);
    match text("verdict") {
        Some(verdict) => Known::Known(Capacity {
            verdict,
            reason: text("reason"),
            axis: text("axis"),
            owner: text("owner"),
        }),
        None => Known::unknown("berth room cargo --json named no verdict"),
    }
}

/// What a bounded child printed: its exit code, stdout and stderr.
struct Ran(Option<i32>, Vec<u8>, Vec<u8>);

/// Run `program args…` with no stdin, capped at `timeout`.
fn run_bounded(program: &Path, args: &[&str], timeout: Duration) -> Result<Ran, String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("{} could not be started: {error}", program.display()))?;
    let drain = |pipe: Option<Box<dyn std::io::Read + Send>>| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            if let Some(mut pipe) = pipe {
                let _ = pipe.read_to_end(&mut bytes);
            }
            bytes
        })
    };
    let stdout = drain(
        child
            .stdout
            .take()
            .map(|p| Box::new(p) as Box<dyn std::io::Read + Send>),
    );
    let stderr = drain(
        child
            .stderr
            .take()
            .map(|p| Box::new(p) as Box<dyn std::io::Read + Send>),
    );
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let out = stdout.join().unwrap_or_default();
                let err = stderr.join().unwrap_or_default();
                return Ok(Ran(status.code(), out, err));
            }
            Ok(None) if started.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "{} did not answer within {} s",
                    program.display(),
                    timeout.as_secs()
                ));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(25)),
            Err(error) => {
                return Err(format!(
                    "{} could not be waited on: {error}",
                    program.display()
                ))
            }
        }
    }
}

fn known_text(value: &Known<String>) -> String {
    match value {
        Known::Known(text) => text.clone(),
        other => not_known_text(other),
    }
}

/// The text of a field that is not `Known`.
fn not_known_text<T>(value: &Known<T>) -> String {
    match value {
        Known::Known(_) => String::new(),
        Known::Unknown { unknown } => format!("unknown ({unknown})"),
        Known::NotApplicable { not_applicable } => not_applicable.clone(),
    }
}

fn set_text(value: &Known<Vec<String>>) -> String {
    match value {
        Known::Known(members) => format!("[{}]", members.join(", ")),
        other => not_known_text(other),
    }
}

impl ParticipationResult {
    /// The P16 text block. Same distinctions and unknowns as the JSON.
    pub fn render_text(&self) -> String {
        let mut out = String::new();
        let mut line = |text: String| {
            out.push_str(&text);
            out.push('\n');
        };
        line(format!("Participation: {CALLER}; completeness unknown"));
        let sources: Vec<String> = self
            .sources
            .iter()
            .map(|s| match &s.reason {
                Some(reason) => format!("{} {} ({reason})", s.source, s.status),
                None => format!("{} {}", s.source, s.status),
            })
            .collect();
        line(format!("Sources: {}", sources.join("; ")));
        for source in &self.sources {
            for device in &source.devices {
                line(format!("  device {}: {}", device.device, device.note));
            }
        }
        let capacity = match &self.capacity {
            Known::Known(answer) => match &answer.reason {
                Some(reason) => format!("{} — {reason}", answer.verdict),
                None => answer.verdict.clone(),
            },
            other => not_known_text(other),
        };
        line(format!(
            "Capacity: {capacity}; grant: {}",
            known_text(&self.grant)
        ));
        line(format!("Covenant scope: {}", known_text(&self.covenant)));
        if self.outcome != "candidates" {
            line("No visible candidate established".into());
        }
        for group in &self.groups {
            let more = self
                .sources
                .iter()
                .find(|s| s.source == group.source)
                .map_or(0, |s| s.more);
            if group.items.is_empty() && more == 0 {
                continue;
            }
            line(capitalise(&group.source));
            for item in &group.items {
                line(format!("  {} — {}", item.reference, item.state));
                line(format!("  Serves: {}", set_or_unknown(&item.serves)));
                line(format!(
                    "  For whom: stewards {}; gifters {}; receivers {}",
                    set_text(&item.roles.stewards),
                    set_text(&item.roles.gifters),
                    set_text(&item.roles.receivers)
                ));
                line(format!("  Standing: {}", known_text(&item.standing)));
                line(format!(
                    "  Worth: {}",
                    match &item.worth {
                        Known::Known(rows) => rows
                            .iter()
                            .map(|w| format!("{} {} ({} at {})", w.value, w.unit, w.source, w.at))
                            .collect::<Vec<_>>()
                            .join("; "),
                        other => not_known_text(other),
                    }
                ));
                line(format!(
                    "  Testimony: {}",
                    if item.testimony.is_empty() {
                        "none recorded".to_string()
                    } else {
                        item.testimony
                            .iter()
                            .map(|t| {
                                let observed = t
                                    .observed_at
                                    .as_ref()
                                    .map(|value| format!(", observed {value}"))
                                    .unwrap_or_default();
                                format!(
                                    "{} {} at {}{} ({})",
                                    t.author, t.reference, t.at, observed, t.kind
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("; ")
                    }
                ));
                line(format!("  Taking would record: {}", item.taking));
                line(format!(
                    "  Fit: covenant {}; capacity {}; graph {}",
                    known_text(&item.fit.covenant),
                    known_text(&item.fit.capacity),
                    known_text(&item.fit.graph)
                ));
                line(match &item.person.ready_for {
                    Known::Known(when) => format!("  Person: ready for {when}"),
                    Known::Unknown { unknown } => {
                        format!("  Person: ready for unknown — {unknown}")
                    }
                    Known::NotApplicable { not_applicable } => {
                        format!("  Person: {not_applicable}")
                    }
                });
                line(format!("  Agent: {}", known_text(&item.agent)));
            }
            if more > 0 {
                line(format!("  … {more} more in {}", group.source));
            }
        }
        if self.excluded.is_empty() {
            line(format!("Excluded ({EXCLUDED_SCOPE}): none"));
        } else {
            let rows: Vec<String> = self
                .excluded
                .iter()
                .map(|e| format!("{}: {} {}", e.source, e.count, e.reason))
                .collect();
            line(format!("Excluded ({EXCLUDED_SCOPE}): {}", rows.join("; ")));
        }
        out
    }
}

fn set_or_unknown(value: &Known<Vec<String>>) -> String {
    match value {
        Known::Known(members) => members.join(", "),
        other => not_known_text(other),
    }
}

fn capitalise(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// `berth status --json`, exactly the contract berth publishes: live local sessions, live sessions
/// on the steward's other devices, the exchange with those devices, and what berth could not read.
#[derive(Debug, Clone, Deserialize)]
struct BerthStatus {
    moorings: Vec<LocalSession>,
    siblings: Vec<SiblingSession>,
    devices: Vec<DeviceRow>,
    unavailable: Vec<BerthGap>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocalSession {
    session: String,
    runtime: Option<String>,
    model: Option<String>,
    task: Option<String>,
    writes: Vec<String>,
    seen_s: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SiblingSession {
    device: String,
    session: String,
    runtime: Option<String>,
    task: Option<String>,
    writes: Vec<String>,
    seen_s: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct DeviceRow {
    device: String,
    #[serde(default)]
    silent: bool,
    unacked: Option<Unacked>,
}

#[derive(Debug, Clone, Deserialize)]
struct Unacked {
    reason: String,
    gap: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
struct BerthGap {
    source: String,
    reason: String,
}

impl BerthStatus {
    /// The reason berth gave for not reading `source`, when it gave one.
    fn gap(&self, source: &str) -> Option<String> {
        self.unavailable
            .iter()
            .find(|gap| gap.source == source)
            .map(|gap| gap.reason.clone())
    }

    /// One note per other device that is silent or has not acknowledged this device's records.
    fn device_notes(&self) -> Vec<DeviceNote> {
        self.devices
            .iter()
            .filter_map(|row| {
                let unacked = row.unacked.as_ref().map(|u| match (u.reason.as_str(), u.gap) {
                    ("never", _) => "has acknowledged none of this device's records".to_string(),
                    ("behind", Some(gap)) => format!(
                        "has acknowledged this device's records only to {gap} seq(s) behind its head"
                    ),
                    ("behind", None) => "is behind on this device's records".to_string(),
                    (other, _) => format!("has not acknowledged this device's records ({other})"),
                });
                let note = match (row.silent, unacked) {
                    (false, None) => return None,
                    (true, None) => "silent".to_string(),
                    (false, Some(unacked)) => unacked,
                    (true, Some(unacked)) => format!("silent; {unacked}"),
                };
                Some(DeviceNote {
                    device: crate::actor::short_did(&row.device),
                    note,
                })
            })
            .collect()
    }
}

/// This device's own presence read: `berth status --json`, capped and fail-open.
fn presence(root: &Path, probes: &Probes) -> Result<BerthStatus, String> {
    let berth = berth_path(root, probes);
    if !berth.is_file() {
        return Err(format!("berth not found at {}", berth.display()));
    }
    // Read before run: this read opens nothing that creates (P15), and that includes the tools
    // it spawns.
    let declares = std::fs::read_to_string(&berth)
        .map(|text| text.contains(BERTH_STATUS_CONTRACT))
        .unwrap_or(false);
    if !declares {
        return Err(format!(
            "this berth predates the read-only presence surface ({BERTH_STATUS_CONTRACT})"
        ));
    }
    let ran = run_bounded(&berth, &["status", "--json"], BERTH_TIMEOUT)?;
    if ran.0 != Some(0) {
        let said = String::from_utf8_lossy(&ran.2);
        let said = said.lines().next().unwrap_or("").trim();
        return Err(format!(
            "berth status --json exited {}{}",
            ran.0
                .map_or_else(|| "by signal".to_string(), |code| code.to_string()),
            if said.is_empty() {
                String::new()
            } else {
                format!(": {said}")
            }
        ));
    }
    serde_json::from_slice::<BerthStatus>(&ran.1)
        .map_err(|error| format!("berth status --json printed no answer in its contract ({error})"))
}

/// One Presence item per live session whose declared writes meet the scope directory.
fn presence_items(status: &BerthStatus, scope: &Scoper, base: &ItemBase) -> Vec<Item> {
    let local = status.moorings.iter().map(|m| {
        (
            format!("session {}", short_session(&m.session)),
            m.runtime.as_deref(),
            m.model.as_deref(),
            m.task.as_deref(),
            &m.writes,
            m.seen_s,
        )
    });
    let siblings = status.siblings.iter().map(|m| {
        (
            format!(
                "session {} on {}",
                short_session(&m.session),
                crate::actor::short_did(&m.device)
            ),
            m.runtime.as_deref(),
            None,
            m.task.as_deref(),
            &m.writes,
            m.seen_s,
        )
    });
    local
        .chain(siblings)
        .filter_map(|(reference, runtime, model, task, writes, seen)| {
            let in_scope: Vec<String> = writes
                .iter()
                .filter(|write| paths_overlap(write, &scope.dir))
                .cloned()
                .collect();
            if in_scope.is_empty() {
                return None;
            }
            let who = match (runtime, model) {
                (Some(runtime), Some(model)) => format!("{runtime}, {model}"),
                (Some(one), None) | (None, Some(one)) => one.to_string(),
                (None, None) => "runtime undeclared".to_string(),
            };

            let mut item = base.item(Parts(
                reference,
                format!(
                    "live session ({who}); task: {}; declared in scope: {}; seen {} ago",
                    task.unwrap_or("none declared"),
                    in_scope.join(", "),
                    age(seen)
                ),
                not_applicable(),
                Roles {
                    stewards: Known::unknown("a session is presence, not a role-set member"),
                    gifters: Known::unknown("a session is presence, not a role-set member"),
                    receivers: Known::unknown("a session is presence, not a role-set member"),
                },
                not_applicable(),
                Vec::new(),
                NOT_CLAIMABLE.to_string(),
            ));
            item.agent = Known::Known(format!("running (seen {} ago)", age(seen)));
            Some(item)
        })
        .collect()
}

fn not_applicable<T>() -> Known<T> {
    Known::NotApplicable {
        not_applicable: NOT_CLAIMABLE.to_string(),
    }
}

fn short_session(session: &str) -> String {
    session.chars().take(8).collect()
}

fn age(seconds: i64) -> String {
    let seconds = seconds.max(0);
    if seconds < 120 {
        format!("{seconds}s")
    } else if seconds < 7200 {
        format!("{}m", seconds / 60)
    } else {
        format!("{}h", seconds / 3600)
    }
}

/// Whether two declared paths can name the same file: segment prefixes overlap, `**` meets
/// anything, and a segment glob meets what it matches (berth's own `paths_overlap` rule).
fn paths_overlap(left: &str, right: &str) -> bool {
    let parts = |path: &str| -> Vec<String> {
        path.split('/')
            .filter(|part| !part.is_empty() && *part != ".")
            .map(str::to_string)
            .collect()
    };
    let wild = |part: &str| part.contains(['*', '?', '[']);
    for (a, b) in parts(left).iter().zip(parts(right).iter()) {
        if a == "**" || b == "**" {
            return true;
        }
        if a == b || glob_match(a, b) || glob_match(b, a) {
            continue;
        }
        if wild(a) && wild(b) {
            continue;
        }
        return false;
    }
    true
}

/// `*` and `?` over one path segment.
fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti, mut star, mut mark) = (0, 0, None, 0);
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if let Some(at) = star {
            pi = at + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|c| *c == '*')
}
