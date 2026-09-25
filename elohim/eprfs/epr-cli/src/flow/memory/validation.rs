use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use cid::Cid;
use elohim_epr_rea::{
    atom_cid, parse_agent_ref, parse_participant_ref, ActorStore, FlowRecord, FlowStore,
    ParticipantRef, SidecarActorStore, SidecarFlowStore, ROOT_COLLECTIVE_DECLARATION,
};
use eprfs_agent::memory::{
    Affiliation, AffiliationStanding, Collective, Contribution, Feedback, FileRef, Locality,
    MemberKind, ProjectionRequest,
};
use eprfs_core::BlobCid;
use serde_json::{json, Value};

use super::{refused, FlowResult};

/// The repository-root collective's declaration. Every other declaration is a CHILD, found by
/// walking up from a path to the nearest `.epr-meta/collective.json`.
pub const COLLECTIVE_PATH: &str = ROOT_COLLECTIVE_DECLARATION;
/// Where affiliations live locally: the pre-image of the notarized Qahal `Membership` links.
pub const AFFILIATIONS_PATH: &str = ".eprfs/status/affiliations.jsonl";
const DECLARATION_TAIL: &str = ".epr-meta/collective.json";
/// How deep a chain of child collectives may nest before the walk is refused.
const MAX_NESTING: usize = 8;
const MAX_BYTES: usize = 262144;
const MAX_FILES: usize = 32;

#[derive(Clone)]
pub struct ReadFile {
    pub reference: FileRef,
    pub text: String,
}

/// One resolved collective: its pinned declaration and its current affiliations.
///
/// Stewardship is read here and nowhere else. The declaration names no steward; the stewards
/// are the affiliations with `role: Steward` that are still standing, and a collective with
/// none is refused before any operation runs under it.
#[derive(Clone)]
pub struct Governance {
    pub reference: FileRef,
    pub declaration: Collective,
    /// The current affiliation per member (the last sidecar line wins), with its line CID.
    pub affiliations: Vec<(String, Affiliation)>,
    /// Sidecar lines whose CID or shape did not verify. They count for nothing, anywhere.
    pub invalid_lines: usize,
}

impl Governance {
    pub fn is_root(&self) -> bool {
        self.reference.path == COLLECTIVE_PATH
    }

    /// Stewards still on record.
    pub fn stewards(&self) -> impl Iterator<Item = &(String, Affiliation)> {
        self.affiliations
            .iter()
            .filter(|(_, a)| a.is_active_steward())
    }

    /// The standing (never withdrawn) affiliation that stands for `participant`, if any.
    pub fn affiliation_of(&self, participant: &str) -> Option<&(String, Affiliation)> {
        self.affiliations
            .iter()
            .filter(|(_, a)| a.withdrawn.is_none())
            .find(|(_, a)| a.names(participant))
    }

    /// Whom an affiliation acts for: its declared `acts_for`, else the repository agent on the
    /// root collective (the role `REPO_AGENT` kept after it stopped being the steward test), else
    /// the child collective itself.
    pub fn acts_for(&self, affiliation: &Affiliation) -> String {
        affiliation.acts_for.clone().unwrap_or_else(|| {
            if self.is_root() {
                crate::flow::REPO_AGENT.to_string()
            } else {
                self.declaration.id.clone()
            }
        })
    }

    /// The party a contribution filed in this collective answers to when it names nobody else:
    /// the default `acts_for` of the collective's affiliations.
    pub fn default_steward(&self) -> String {
        if self.is_root() {
            crate::flow::REPO_AGENT.to_string()
        } else {
            self.declaration.id.clone()
        }
    }

    /// Whether `steward` may stand in a contribution's `steward` slot: a Steward member, or the
    /// party some standing affiliation acts for. Nobody else answers for this collective.
    pub fn answers_for(&self, steward: &str) -> bool {
        self.affiliations
            .iter()
            .filter(|(_, a)| a.withdrawn.is_none())
            .any(|(_, a)| {
                (a.is_active_steward() && a.member == steward) || self.acts_for(a) == steward
            })
    }

    /// The Stewards on record, as a report: who, which affiliation line, and whether each stands
    /// for real or is a fixture co-steward.
    pub fn steward_report(&self) -> Value {
        Value::Array(
            self.stewards()
                .map(|(cid, a)| {
                    json!({"member":a.member,"memberKind":a.member_kind,"affiliation":cid,
                        "standing":a.standing,"actsFor":self.acts_for(a),
                        "validatedAt":validated_at(a.standing)})
                })
                .collect(),
        )
    }
}

/// How far an approval resting on a Steward of this standing validates anything. A fixture
/// co-steward's approval runs the real primitive at Bootstrap stakes and must never read as
/// peer validation.
pub fn validated_at(standing: AffiliationStanding) -> &'static str {
    match standing {
        AffiliationStanding::Standing => "local (steward on record)",
        AffiliationStanding::Fixture => "bootstrap (fixture co-steward)",
    }
}

pub struct Reader {
    pub root: PathBuf,
    files: BTreeMap<String, ReadFile>,
    bytes: usize,
    records: Option<Vec<(Cid, FlowRecord)>>,
    sidecar_bytes: u64,
}

impl Reader {
    pub fn new(root: &Path) -> FlowResult<Self> {
        Ok(Self {
            root: root.canonicalize()?,
            files: BTreeMap::new(),
            bytes: 0,
            records: None,
            sidecar_bytes: 0,
        })
    }

    fn path(&self, path: &str) -> FlowResult<PathBuf> {
        let rel = normalized(path)?;
        let mut full = self.root.clone();
        for part in rel.components() {
            full.push(part);
            let meta = std::fs::symlink_metadata(&full)?;
            if meta.file_type().is_symlink() {
                return Err(refused("symlink source paths are refused"));
            }
        }
        Ok(full)
    }

    pub fn read(&mut self, path: &str) -> FlowResult<ReadFile> {
        if let Some(file) = self.files.get(path) {
            return Ok(file.clone());
        }
        if self.files.len() >= MAX_FILES {
            return Err(refused("source file budget exhausted"));
        }
        let full = self.path(path)?;
        let metadata = std::fs::metadata(&full)?;
        if !metadata.is_file() {
            return Err(refused("source is not a regular file"));
        }
        let remaining = MAX_BYTES.saturating_sub(self.bytes);
        if metadata.len() > remaining as u64 {
            return Err(refused("source byte budget exhausted"));
        }
        let mut bytes = Vec::new();
        File::open(&full)?
            .take(remaining as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > remaining {
            return Err(refused("source grew beyond byte budget"));
        }
        let text = String::from_utf8(bytes.clone()).map_err(|_| refused("source is not UTF-8"))?;
        let file = ReadFile {
            reference: FileRef {
                path: path.into(),
                cid: BlobCid::compute_raw(&bytes).to_string(),
            },
            text,
        };
        self.bytes += bytes.len();
        self.files.insert(path.into(), file.clone());
        Ok(file)
    }

    pub fn pinned(&mut self, reference: &FileRef) -> FlowResult<ReadFile> {
        let file = self.read(&reference.path)?;
        if file.reference != *reference {
            return Err(refused(format!(
                "source version changed: {}",
                reference.path
            )));
        }
        Ok(file)
    }

    pub fn unchanged(&self, reference: &FileRef) -> FlowResult<()> {
        let path = self.path(&reference.path)?;
        let mut bytes = Vec::new();
        File::open(path)?
            .take(MAX_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > MAX_BYTES || BlobCid::compute_raw(&bytes).to_string() != reference.cid {
            return Err(refused("source changed at write boundary"));
        }
        Ok(())
    }

    pub fn sidecar(&self, path: &str) -> FlowResult<()> {
        let full = self.path(path)?;
        if std::fs::metadata(full)?.len() > 33554432 {
            return Err(refused("native sidecar exceeds 32MiB local read budget"));
        }
        Ok(())
    }

    pub fn records(&mut self) -> FlowResult<&[(Cid, FlowRecord)]> {
        if self.records.is_none() {
            self.sidecar(".eprfs/status/flows.jsonl")?;
            self.sidecar_bytes =
                std::fs::metadata(self.root.join(".eprfs/status/flows.jsonl"))?.len();
            self.records = Some(SidecarFlowStore::open(&self.root)?.records()?);
        }
        Ok(self.records.as_deref().unwrap_or(&[]))
    }

    /// The repository-root collective.
    pub fn collective(&mut self) -> FlowResult<Governance> {
        self.governance(COLLECTIVE_PATH)
    }

    /// The collective of record for `path`: the nearest `.epr-meta/collective.json` walking up
    /// from it (from `path` itself when it is a directory), ending at the repository root.
    pub fn nearest_declaration(&self, path: &str) -> FlowResult<String> {
        let rel = normalized(path)?;
        let start = if self.root.join(rel).is_dir() {
            Some(rel)
        } else {
            rel.parent()
        };
        self.nearest_from(start)
    }

    fn nearest_from(&self, mut dir: Option<&Path>) -> FlowResult<String> {
        while let Some(d) = dir {
            if d.as_os_str().is_empty() {
                break;
            }
            let candidate = d.join(DECLARATION_TAIL);
            if is_plain_file(&self.root.join(&candidate)) {
                return Ok(candidate.to_string_lossy().into_owned());
            }
            dir = d.parent();
        }
        if is_plain_file(&self.root.join(COLLECTIVE_PATH)) {
            return Ok(COLLECTIVE_PATH.to_string());
        }
        Err(refused(format!(
            "no collective is declared: {COLLECTIVE_PATH} is absent"
        )))
    }

    /// The collective a declaration path names: the declaration read and checked, its parent
    /// chain verified, and its Stewards on record required.
    pub fn governance(&mut self, declaration_path: &str) -> FlowResult<Governance> {
        self.governance_at(declaration_path, 0)
    }

    fn governance_at(&mut self, path: &str, depth: usize) -> FlowResult<Governance> {
        if depth > MAX_NESTING {
            return Err(refused(format!(
                "collective nesting exceeds {MAX_NESTING}; stop or narrow the chain"
            )));
        }
        let dir = declaration_dir(path).ok_or_else(|| {
            refused(format!(
                "`{path}` is not a collective declaration (…/{DECLARATION_TAIL})"
            ))
        })?;
        let file = self.read(path)?;
        let raw: Value = serde_json::from_str(&file.text)?;
        retired_vocabulary(&raw, path)?;
        let declaration: Collective = serde_json::from_value(raw)?;
        version(declaration.version)?;
        if !declaration.id.starts_with("collective:")
            || declaration.participation != "registered-local-session"
        {
            return Err(refused(
                "unsupported collective relationship or participation policy",
            ));
        }
        bounded_text(&declaration.display_name, "displayName", 256)?;
        bounded_text(&declaration.charter, "charter", 4000)?;
        if declaration.source_rules.is_empty() || declaration.source_rules.len() > 16 {
            return Err(refused("requires 1..16 source rules"));
        }
        let mut rules = BTreeSet::new();
        for rule in &declaration.source_rules {
            if !rules.insert(&rule.path) {
                return Err(refused("duplicate source rule path"));
            }
            if rule.path.is_empty()
                || Path::new(&rule.path)
                    .components()
                    .any(|c| !matches!(c, Component::Normal(_)))
            {
                return Err(refused("source rule must be a normalized relative path"));
            }
        }
        match (&declaration.parent, path == COLLECTIVE_PATH) {
            (Some(_), true) => {
                return Err(refused(
                    "the repository-root collective has no parent; remove `parent`",
                ))
            }
            (None, false) => {
                return Err(refused(format!(
                    "child collective {path} must pin its enclosing collective as `parent`"
                )))
            }
            (Some(parent), false) => {
                let enclosing = self.nearest_from(Path::new(&dir).parent())?;
                if parent.path != enclosing {
                    return Err(refused(format!(
                        "{path} pins parent {} but its enclosing collective is {enclosing}",
                        parent.path
                    )));
                }
                if self.read(&parent.path)?.reference != *parent {
                    return Err(refused(format!(
                        "{path}'s parent pin is stale: {} changed since it was pinned; re-pin it",
                        parent.path
                    )));
                }
                let upper = self.governance_at(&parent.path, depth + 1)?;
                for rule in &declaration.source_rules {
                    if !Path::new(&rule.path).starts_with(&dir) {
                        return Err(refused(format!(
                            "child source rule {} lies outside its collective's directory {dir}",
                            rule.path
                        )));
                    }
                    if rule.max_locality > self.policy_locality(&rule.path, &upper.declaration)? {
                        return Err(refused(format!(
                            "child source rule {} widens its parent's locality",
                            rule.path
                        )));
                    }
                }
            }
            (None, true) => {}
        }
        let (affiliations, invalid_lines) = self.affiliations_for(&file.reference)?;
        let governance = Governance {
            reference: file.reference,
            declaration,
            affiliations,
            invalid_lines,
        };
        if governance.stewards().next().is_none() {
            return Err(refused(format!(
                "collective {} ({path}) has no Steward on record — stewards are affiliation \
                 records (role Steward) in {AFFILIATIONS_PATH}, never a declaration field",
                governance.declaration.id
            )));
        }
        Ok(governance)
    }

    /// Every current affiliation with the collective whose declaration is at `reference.path`.
    ///
    /// Matched by declaration PATH: a charter amendment re-pins the declaration's CID, and a
    /// member's standing does not lapse because the text they affiliated under was amended — the
    /// CID each member affiliated under stays on their line as the record of it.
    fn affiliations_for(
        &self,
        reference: &FileRef,
    ) -> FlowResult<(Vec<(String, Affiliation)>, usize)> {
        if !self.root.join(AFFILIATIONS_PATH).exists() {
            return Ok((Vec::new(), 0));
        }
        self.sidecar(AFFILIATIONS_PATH)?;
        let text = std::fs::read_to_string(self.path(AFFILIATIONS_PATH)?)?;
        let mut order: Vec<String> = Vec::new();
        let mut current: BTreeMap<String, (String, Affiliation)> = BTreeMap::new();
        let mut invalid = 0;
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let Some((cid, affiliation)) = verify_affiliation_line(line) else {
                invalid += 1;
                continue;
            };
            if affiliation.collective.path != reference.path {
                continue;
            }
            if !current.contains_key(&affiliation.member) {
                order.push(affiliation.member.clone());
            }
            current.insert(affiliation.member.clone(), (cid, affiliation));
        }
        let affiliations = order
            .into_iter()
            .filter_map(|member| current.remove(&member))
            .collect();
        Ok((affiliations, invalid))
    }

    /// A session's claim binds its collective of record; an unbound claim is bound to the root.
    /// Refuses a write whose collective differs from the one the acting claim is bound to.
    pub fn require_bound(&self, session: Option<&str>, governance: &Governance) -> FlowResult<()> {
        let Some(session) = session else {
            return Ok(());
        };
        if !self.root.join(".eprfs/status/actors.jsonl").exists() {
            return Ok(());
        }
        self.sidecar(".eprfs/status/actors.jsonl")?;
        let Some((_, claim)) = SidecarActorStore::open(&self.root)?.current_for(session)? else {
            return Ok(());
        };
        let bound = claim.collective_of_record();
        if bound != governance.reference.path {
            return Err(refused(format!(
                "session {session} is bound to the collective at {bound}, but this act's \
                 collective of record is {} ({}); re-claim with `epr actor claim --under <path>` \
                 inside that collective",
                governance.reference.path, governance.declaration.id
            )));
        }
        Ok(())
    }

    pub fn require_collective(&self, supplied: &FileRef, actual: &FileRef) -> FlowResult<()> {
        if supplied != actual {
            return Err(refused(
                "collective declaration version/path differs; re-read current governance",
            ));
        }
        Ok(())
    }

    /// `path` must lie in the collective `governance` names, never in a nearer child.
    pub fn owned_by(&self, path: &str, governance: &Governance) -> FlowResult<()> {
        let owner = self.nearest_declaration(path)?;
        if owner != governance.reference.path {
            return Err(refused(format!(
                "{path} belongs to the collective at {owner}, not to {} ({})",
                governance.reference.path, governance.declaration.id
            )));
        }
        Ok(())
    }

    pub fn allowed(
        &self,
        path: &str,
        locality: Locality,
        collective: &Collective,
    ) -> FlowResult<()> {
        if locality > self.policy_locality(path, collective)? {
            return Err(refused(format!(
                "source policy forbids requested locality: {path}"
            )));
        }
        Ok(())
    }

    pub fn policy_locality(&self, path: &str, collective: &Collective) -> FlowResult<Locality> {
        // Most specific component-prefix wins; duplicate rule paths are refused.
        collective
            .source_rules
            .iter()
            .filter(|r| Path::new(path).starts_with(&r.path))
            .max_by_key(|r| Path::new(&r.path).components().count())
            .map(|r| r.max_locality)
            .ok_or_else(|| {
                refused(format!(
                    "source is outside declared collective policy: {path}"
                ))
            })
    }

    pub fn effective_locality(
        &mut self,
        reference: &FileRef,
        collective: &Collective,
        depth: usize,
    ) -> FlowResult<Locality> {
        if depth > 8 {
            return Err(refused(
                "feedback provenance depth exceeds 8; stop or narrow the chain",
            ));
        }
        let file = self.pinned(reference)?;
        let mut locality = self.policy_locality(&reference.path, collective)?;
        if let Ok(assertion) = serde_json::from_str::<Contribution>(&file.text) {
            locality = locality.min(assertion.reach);
        }
        if let Ok(request) = serde_json::from_str::<ProjectionRequest>(&file.text) {
            locality = locality.min(request.audience);
        }
        if let Ok(feedback) = serde_json::from_str::<Feedback>(&file.text) {
            locality =
                locality.min(self.effective_locality(&feedback.target, collective, depth + 1)?);
        }
        if let Ok(value) = serde_json::from_str::<Value>(&file.text) {
            match value.get("operation").and_then(Value::as_str) {
                Some("project") => {
                    let audience: Locality =
                        serde_json::from_value(value["receipt"]["audience"].clone())?;
                    let request: FileRef =
                        serde_json::from_value(value["receipt"]["request"].clone())?;
                    locality = locality
                        .min(audience)
                        .min(self.policy_locality(&request.path, collective)?);
                }
                Some("feedback" | "contribute" | "graduate") => {
                    let inherited: Locality =
                        serde_json::from_value(value["effectiveReach"].clone()).map_err(|_| {
                            refused("saved native receipt has no verifiable inherited locality")
                        })?;
                    locality = locality.min(inherited);
                    if value["operation"] == "feedback" {
                        let target: FileRef = serde_json::from_value(value["target"].clone())?;
                        let request: FileRef = serde_json::from_value(value["resource"].clone())?;
                        locality = locality
                            .min(self.effective_locality(&target, collective, depth + 1)?)
                            .min(self.policy_locality(&request.path, collective)?);
                    }
                }
                Some("pin") => locality = Locality::Private,
                _ => {}
            }
        }
        Ok(locality)
    }

    pub fn contribution(&mut self, c: &Contribution, governance: &Governance) -> FlowResult<()> {
        let collective = &governance.declaration;
        version(c.version)?;
        self.require_collective(&c.collective, &governance.reference)?;
        if !["session", "workspace", "repository"].contains(&c.scope.as_str()) {
            return Err(refused("contribution steward or scope is not declared"));
        }
        if !governance.answers_for(&c.steward) {
            return Err(refused(format!(
                "contribution steward or scope is not declared: steward `{}` answers for nobody \
                 on record in {}; name a Steward member or the party an affiliation acts for \
                 (default {})",
                c.steward,
                collective.id,
                governance.default_steward()
            )));
        }
        // Either participant kind may author a contribution — the human as themselves, never
        // as a persona the substrate minted for them.
        elohim_epr_rea::parse_participant_ref(&c.author)?;
        bounded_text(&c.concern, "concern", 256)?;
        bounded_text(&c.claim, "claim", 1000)?;
        strings(&c.uncertainty, "uncertainty", 8, 300)?;
        if c.sources.is_empty()
            || c.sources.len() > 8
            || c.supersedes.len() > 8
            || c.contradicts.len() > 8
        {
            return Err(refused(
                "contribution needs 1..8 sources and at most 8 links of each kind",
            ));
        }
        for source in &c.sources {
            // The collective of record is resolved from where the evidence lives, so a source
            // inside a child collective cannot be filed under its parent (or the reverse).
            self.owned_by(&source.resource.path, governance)?;
            self.allowed(&source.resource.path, source.reach, collective)?;
            if c.reach > source.reach {
                return Err(refused("contribution cannot widen source restriction"));
            }
            let file = self.pinned(&source.resource)?;
            if c.reach > self.effective_locality(&file.reference, collective, 0)? {
                return Err(refused(
                    "source or receipt metadata retains its inherited locality restriction",
                ));
            }
        }
        for link in c.supersedes.iter().chain(&c.contradicts) {
            self.allowed(&link.path, c.reach, collective)?;
            if c.reach > self.effective_locality(link, collective, 0)? {
                return Err(refused(
                    "linked contribution metadata retains its locality restriction",
                ));
            }
        }
        Ok(())
    }

    /// Whether a declaration's registry terms are spoken in the collectives registry's vocabulary:
    /// the parent id exists there, the reach is one of its `reachConstraints`, and the governance
    /// layer is one its schema enumerates. A report, never a refusal: the terms are the pre-image
    /// of the crossing, not a local governance gate, so a fixture tree without the catalog still
    /// governs.
    pub fn registry_report(&mut self, declaration: &Collective) -> Value {
        let Some(terms) = &declaration.registry else {
            return json!({"declared": false});
        };
        let mut problems = Vec::new();
        let catalog = self
            .read(&terms.catalog)
            .ok()
            .and_then(|f| serde_json::from_str::<Value>(&f.text).ok());
        match &catalog {
            None => problems.push(format!("catalog {} is absent or unreadable", terms.catalog)),
            Some(catalog) => {
                let known = catalog["collectives"].as_array().is_some_and(|all| {
                    all.iter()
                        .any(|c| c["id"].as_str() == Some(&terms.constitutional_parent_id))
                });
                if !known {
                    problems.push(format!(
                        "constitutionalParentId `{}` is no collective in {}",
                        terms.constitutional_parent_id, terms.catalog
                    ));
                }
                if catalog["reachConstraints"].get(&terms.reach).is_none() {
                    problems.push(format!(
                        "reach `{}` is not one of {}'s reachConstraints",
                        terms.reach, terms.catalog
                    ));
                }
                let schema = catalog["$schema"].as_str().and_then(|rel| {
                    let path = Path::new(&terms.catalog)
                        .parent()?
                        .join(rel.trim_start_matches("./"));
                    let file = self.read(&path.to_string_lossy()).ok()?;
                    serde_json::from_str::<Value>(&file.text).ok()
                });
                let layers = schema.as_ref().and_then(|s| enum_of(s, "governanceLayer"));
                match layers {
                    Some(layers) if layers.iter().any(|l| l == &terms.governance_layer) => {}
                    Some(_) => problems.push(format!(
                        "governanceLayer `{}` is not in the registry schema's enum",
                        terms.governance_layer
                    )),
                    None => problems
                        .push("the registry schema's governanceLayer enum is unreadable".into()),
                }
            }
        }
        json!({"declared": true, "terms": terms, "verified": problems.is_empty(), "problems": problems})
    }

    pub fn usage(&self) -> Value {
        json!({"sourceFiles":self.files.len(),"sourceBytes":self.bytes,"nativeSidecarBytes":self.sidecar_bytes,
            "accounting":"Unique input bytes; write-boundary fingerprint reread and existing native sidecar evaluation are additional bounded work."})
    }
}

/// Refuse the vocabulary a declaration no longer speaks, naming what replaced it — a bare
/// "unknown field" would teach a caller nothing about where stewardship went.
fn retired_vocabulary(raw: &Value, path: &str) -> FlowResult<()> {
    if raw.get("steward").is_some() {
        return Err(refused(format!(
            "{path}: unknown field `steward` — stewards are affiliation records (role Steward) \
             in {AFFILIATIONS_PATH}, never a declaration field"
        )));
    }
    let rules = raw.get("sourceRules").and_then(Value::as_array);
    if rules.is_some_and(|rules| rules.iter().any(|r| r.get("maxReach").is_some())) {
        return Err(refused(format!(
            "{path}: source rule field `maxReach` was renamed `maxLocality` — reach is the \
             network's audience vocabulary; where a passage travels inside the repository is \
             its locality"
        )));
    }
    Ok(())
}

/// The directory a declaration governs (`""` for the root), or `None` when `path` is not a
/// normalized `…/.epr-meta/collective.json` path.
fn declaration_dir(path: &str) -> Option<String> {
    normalized(path).ok()?;
    if path == COLLECTIVE_PATH {
        return Some(String::new());
    }
    path.strip_suffix(&format!("/{DECLARATION_TAIL}"))
        .map(str::to_string)
}

fn normalized(path: &str) -> FlowResult<&Path> {
    let rel = Path::new(path);
    if rel.as_os_str().is_empty() || rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err(refused(
            "path must be relative with no dot, parent or root components",
        ));
    }
    Ok(rel)
}

fn is_plain_file(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_file())
}

/// One sidecar line, `{cid, record}`, verified: the record parses strictly, its member and
/// parties are shaped as participants (never an email), and the CID re-derives from the record.
/// `None` is a line that counts for nothing.
pub fn verify_affiliation_line(line: &str) -> Option<(String, Affiliation)> {
    let value: Value = serde_json::from_str(line).ok()?;
    let object = value.as_object()?;
    if object.len() != 2 {
        return None;
    }
    let affiliation: Affiliation = serde_json::from_value(object.get("record")?.clone()).ok()?;
    validate_affiliation(&affiliation).ok()?;
    let cid = atom_cid(&affiliation).ok()?.to_string();
    (object.get("cid")?.as_str()? == cid).then_some((cid, affiliation))
}

/// The `{cid, record}` line an affiliation is appended as.
pub fn affiliation_line(affiliation: &Affiliation) -> FlowResult<String> {
    validate_affiliation(affiliation)?;
    let cid = atom_cid(affiliation)?.to_string();
    Ok(serde_json::to_string(
        &json!({"cid": cid, "record": affiliation}),
    )?)
}

pub fn validate_affiliation(a: &Affiliation) -> FlowResult<()> {
    version(a.version)?;
    if declaration_dir(&a.collective.path).is_none() {
        return Err(refused("affiliation collective is not a declaration path"));
    }
    let member_ok = match a.member_kind {
        MemberKind::Person => matches!(
            parse_participant_ref(&a.member),
            Ok(ParticipantRef::Human { .. })
        ),
        MemberKind::ElohimAgent => a
            .member
            .strip_prefix("agent:")
            .is_some_and(|rest| parse_agent_ref(&a.member).is_ok() || is_slug(rest)),
        MemberKind::Collective => a
            .member
            .strip_prefix("collective:")
            .is_some_and(is_path_slug),
    };
    if !member_ok {
        return Err(refused(format!(
            "affiliation member `{}` is not a {:?} participant ref",
            a.member, a.member_kind
        )));
    }
    // One meaning, one encoding: an absent `acts_for` on the root collective already means the
    // repository agent, so spelling it out would mint a second address for the same affiliation.
    if a.collective.path == COLLECTIVE_PATH
        && a.acts_for.as_deref() == Some(crate::flow::REPO_AGENT)
    {
        return Err(refused(format!(
            "affiliation acts_for `{}` is the root collective's default; omit it",
            crate::flow::REPO_AGENT
        )));
    }
    for party in a.sponsor.iter().chain(&a.acts_for) {
        let ok = match party.split_once(':') {
            Some(("human", _)) => parse_participant_ref(party).is_ok(),
            Some(("repo" | "collective", rest)) => is_path_slug(rest),
            _ => false,
        };
        if !ok {
            return Err(refused(format!(
                "affiliation party `{party}` is not a human:, repo: or collective: ref"
            )));
        }
    }
    bounded_text(&a.since, "since", 64)?;
    if let Some(withdrawn) = &a.withdrawn {
        bounded_text(withdrawn, "withdrawn", 64)?;
    }
    Ok(())
}

/// The first `enum` declared for property `key` anywhere in a JSON schema.
fn enum_of(schema: &Value, key: &str) -> Option<Vec<String>> {
    match schema {
        Value::Object(map) => {
            if let Some(values) = map.get(key).and_then(|p| p["enum"].as_array()) {
                return Some(
                    values
                        .iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect(),
                );
            }
            map.values().find_map(|v| enum_of(v, key))
        }
        Value::Array(items) => items.iter().find_map(|v| enum_of(v, key)),
        _ => None,
    }
}

fn is_slug(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn is_path_slug(s: &str) -> bool {
    !s.is_empty()
        && s.split('/').all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
                && part != "."
                && part != ".."
        })
}

pub fn version(version: u32) -> FlowResult<()> {
    if version != 1 {
        return Err(refused("unsupported contract version"));
    }
    Ok(())
}

pub fn bounded_text(value: &str, name: &str, max: usize) -> FlowResult<()> {
    if value.trim().is_empty() || value.len() > max {
        return Err(refused(format!(
            "{name} must be nonempty and <= {max} bytes"
        )));
    }
    Ok(())
}

pub fn strings(values: &[String], name: &str, count: usize, max: usize) -> FlowResult<()> {
    if values.len() > count {
        return Err(refused(format!("too many {name} values")));
    }
    for value in values {
        bounded_text(value, name, max)?;
    }
    Ok(())
}
