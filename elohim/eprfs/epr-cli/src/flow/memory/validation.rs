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
    agent_role, Affiliation, AffiliationStanding, Collective, Contribution, Feedback, FileRef,
    Locality, MemberKind, ProjectionRequest,
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
    /// The current affiliation per member (the last ADMITTED sidecar line wins), with its CID.
    pub affiliations: Vec<(String, Affiliation)>,
    /// Sidecar lines that count for nothing, anywhere: every entry of [`Self::refused`].
    pub invalid_lines: usize,
    /// Every refused line, named: one whose CID, shape or signature did not verify, or one of
    /// this collective's lines the sponsorship chain refused (see [`Fold::admit`]).
    pub refused: Vec<RefusedLine>,
    /// Every admitted line of this collective by CID: its sponsor's line and its signature.
    pub admitted: BTreeMap<String, Admitted>,
    /// `Some(declaration CID)` when the collective is STEWARDLESS: its last Steward left under
    /// that declaration. Contributions still flow; every act needing a Steward refuses.
    pub stewardless_since: Option<String>,
    /// A child collective's parent Stewards (active), who may sponsor its re-founding.
    pub parent_stewards: Vec<(String, Affiliation)>,
}

/// One sidecar line that counts for nothing, and why — never silently dropped.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefusedLine {
    /// 1-based line number in the sidecar.
    pub line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member: Option<String>,
    pub reason: String,
}

/// One admitted line: the record, the admitted line its sponsor stood on (`None` = the genesis
/// Steward), and how it is signed.
#[derive(Clone, Debug)]
pub struct Admitted {
    pub record: Affiliation,
    pub sponsor_line: Option<String>,
    pub kind: AdmissionKind,
    pub signature: LineSignature,
}

impl Governance {
    pub fn is_root(&self) -> bool {
        self.reference.path == COLLECTIVE_PATH
    }

    /// Whether the collective has no active Steward (its last Steward left).
    pub fn is_stewardless(&self) -> bool {
        self.stewardless_since.is_some()
    }

    /// `stewardless` or `stewarded`: the word every surface prints.
    pub fn stewardship(&self) -> &'static str {
        if self.is_stewardless() {
            "stewardless"
        } else {
            "stewarded"
        }
    }

    /// Refuses, naming the state, when an act needs a Steward and the collective has none.
    pub fn require_stewarded(&self, act: &str) -> FlowResult<()> {
        match &self.stewardless_since {
            None => Ok(()),
            Some(since) => Err(refused(format!(
                "{act} needs a Steward, and collective {} is stewardless (its last Steward left                  under declaration {since}); contributions still flow, but no Steward's verdict                  or sponsorship can stand until it is re-founded",
                self.declaration.id
            ))),
        }
    }

    /// How this collective may be re-founded should it be stewardless.
    pub fn refounding(&self) -> Refounding {
        if self.is_root() {
            Refounding::Root {
                current: self.reference.cid.clone(),
                supersedes: self.declaration.supersedes.clone(),
            }
        } else {
            Refounding::Child {
                parent: self.parent_stewards.clone(),
            }
        }
    }

    /// Stewards still on record.
    pub fn stewards(&self) -> impl Iterator<Item = &(String, Affiliation)> {
        self.affiliations
            .iter()
            .filter(|(_, a)| a.is_active_steward())
    }

    /// The standing (never withdrawn) affiliation that stands for `participant`, if any.
    /// An exact member match wins over a package-level agent affiliation that also names the
    /// participant, so a build-specific Steward is found even beside its role's Contributor line.
    pub fn affiliation_of(&self, participant: &str) -> Option<&(String, Affiliation)> {
        let standing = || {
            self.affiliations
                .iter()
                .filter(|(_, a)| a.withdrawn.is_none())
        };
        standing()
            .find(|(_, a)| a.member == participant)
            .or_else(|| standing().find(|(_, a)| a.names(participant)))
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

    /// The Stewards on record, as a report: who, which affiliation line, whether each stands
    /// for real or is a fixture co-steward, who sponsored it back to the genesis Steward, and
    /// whether each line of that chain is `signed` or `unsigned`.
    pub fn steward_report(&self) -> Value {
        Value::Array(
            self.stewards()
                .map(|(cid, a)| {
                    json!({"member":a.member,"memberKind":a.member_kind,"affiliation":cid,
                        "standing":a.standing,"actsFor":self.acts_for(a),
                        "validatedAt":validated_at(a.standing),
                        "sponsor":a.sponsor,
                        "signature":self.signature_report(cid),
                        "sponsorChain":self.sponsor_chain(cid)})
                })
                .collect(),
        )
    }

    /// `signed` (with its device) or the honest literal `unsigned`, for one admitted line.
    pub fn signature_report(&self, cid: &str) -> Value {
        match self.admitted.get(cid).map(|a| &a.signature) {
            Some(LineSignature::Signed { signer, .. }) => {
                json!({"status":"signed","signer":signer})
            }
            _ => json!({"status":"unsigned"}),
        }
    }

    /// The sponsors of one admitted line, nearest first, ending at the genesis Steward: each hop
    /// is the exact line the sponsor stood on when it sponsored, never its later record.
    pub fn sponsor_chain(&self, cid: &str) -> Vec<Value> {
        let mut chain = Vec::new();
        let mut seen = BTreeSet::new();
        let mut next = self.admitted.get(cid).and_then(|a| a.sponsor_line.clone());
        while let Some(hop) = next {
            if !seen.insert(hop.clone()) {
                break;
            }
            let Some(line) = self.admitted.get(&hop) else {
                break;
            };
            chain.push(json!({"member":line.record.member,"affiliation":hop,
                "role":line.record.role,"standing":line.record.standing,
                "genesis":line.sponsor_line.is_none(),"admittedAs":line.kind,
                "signature":self.signature_report(&hop)["status"]}));
            next = line.sponsor_line.clone();
        }
        chain
    }
}

/// How one admitted line was admitted — what its sponsor chain reads at that hop.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AdmissionKind {
    /// The collective's first Steward line: it names no sponsor.
    Genesis,
    /// Sponsored by a distinct active Steward of the collective.
    Sponsored,
    /// The member withdrew themselves. Leaving is never gated.
    SelfWithdrawal,
    /// A stewardless ROOT collective re-founded through a reviewed amendment of its declaration.
    RefoundedByDeclaration,
    /// A stewardless CHILD collective re-founded by a Steward of its parent collective.
    RefoundedByParent,
}

/// What [`Fold::admit`] admits a line as: the admitted line its sponsor stood on, and how.
#[derive(Clone, Debug)]
pub struct Admission {
    pub sponsor_line: Option<String>,
    pub kind: AdmissionKind,
}

/// How a stewardless collective may be founded again.
#[derive(Clone, Default)]
pub enum Refounding {
    /// A root collective: only through its declaration. `current` is the declaration's raw CID
    /// now, `supersedes` its lineage (newest first).
    Root {
        current: String,
        supersedes: Vec<String>,
    },
    /// A child collective: a Steward of the parent collective sponsors the new genesis.
    Child { parent: Vec<(String, Affiliation)> },
    /// No re-founding route (a fold read without its collective's context).
    #[default]
    Closed,
}

/// The sponsorship chain, folded in FILE ORDER over one collective's sidecar lines.
///
/// The first Steward line of a collective is its genesis: it names no sponsor and is never a
/// Fixture (a fixture never mints real authority). Joining, rejoining and a role change name a
/// `sponsor` who, at that point in the fold, is an ACTIVE Steward of the collective and is not
/// the line's member (by [`Affiliation::same_author_as`], so a sibling build of the same agent
/// role cannot sponsor it); a Fixture Steward may sponsor only a Fixture or a Contributor line.
/// LEAVING IS NEVER GATED: a withdrawal naming its own member as sponsor is always admitted, so
/// the last Steward may leave, and the collective then reads STEWARDLESS — contributions still
/// flow, every act that needs a Steward refuses naming the state, and only a re-founding
/// ([`Refounding`]) admits a new genesis. Membership is the first place the anti-self-election
/// rule holds: a Steward added by a raw file edit without a valid sponsor does not stand.
#[derive(Default)]
pub struct Fold {
    order: Vec<String>,
    current: BTreeMap<String, (String, Affiliation)>,
    genesis: bool,
    /// The declaration CID pinned by the line that left the collective with no active Steward.
    stewardless_since: Option<String>,
    refounding: Refounding,
    admitted: BTreeMap<String, Admitted>,
}

impl Fold {
    /// An empty fold for a collective re-founded by `refounding` should it ever go stewardless.
    pub fn new(refounding: Refounding) -> Self {
        Self {
            refounding,
            ..Self::default()
        }
    }

    /// The fold as it stands after a read collective: its current affiliations, past genesis
    /// (a collective read at all was founded), stewardless or not, with its re-founding route.
    pub fn from_governance(governance: &Governance) -> Self {
        let mut fold = Self {
            genesis: true,
            stewardless_since: governance.stewardless_since.clone(),
            refounding: governance.refounding(),
            ..Self::default()
        };
        for (cid, affiliation) in &governance.affiliations {
            fold.order.push(affiliation.member.clone());
            fold.current.insert(
                affiliation.member.clone(),
                (cid.clone(), affiliation.clone()),
            );
        }
        fold
    }

    /// Whether `line` may follow the fold so far, and as what; or the refusal naming why.
    pub fn admit(&self, line: &Affiliation) -> Result<Admission, String> {
        use eprfs_agent::memory::MembershipRole;
        // Leaving is never gated: a member may always withdraw themselves.
        if line.withdrawn.is_some() && line.sponsor.as_deref() == Some(line.member.as_str()) {
            return match self.current.get(&line.member) {
                Some((cid, a)) if a.withdrawn.is_none() => Ok(Admission {
                    sponsor_line: Some(cid.clone()),
                    kind: AdmissionKind::SelfWithdrawal,
                }),
                _ => Err(format!(
                    "{} holds no standing affiliation to withdraw",
                    line.member
                )),
            };
        }
        let founding = line.role == MembershipRole::Steward && line.withdrawn.is_none();
        if founding
            && line.standing == AffiliationStanding::Fixture
            && (!self.genesis || self.stewardless_since.is_some())
        {
            return Err(format!(
                "{} cannot be a genesis Steward as a Fixture: a fixture never mints real \
                 authority, so a collective is founded only by a standing Steward",
                line.member
            ));
        }
        if !self.genesis {
            if founding && line.sponsor.is_none() {
                return Ok(Admission {
                    sponsor_line: None,
                    kind: AdmissionKind::Genesis,
                });
            }
            return Err(format!(
                "{} precedes the collective's genesis Steward, so no Steward could sponsor it",
                line.member
            ));
        }
        if let Some(since) = &self.stewardless_since {
            return self.refound(line, since, founding);
        }
        let Some(sponsor) = line.sponsor.as_deref() else {
            return Err(format!(
                "{} names no sponsor: joining, rejoining and a role change need an active \
                 Steward's sponsorship (only leaving is ungated)",
                line.member
            ));
        };
        if agent_or_same(sponsor, &line.member) {
            return Err(format!(
                "self-sponsorship: {sponsor} may not sponsor its own membership line for {} — \
                 only leaving is ungated; joining, rejoining and a role change need a distinct \
                 active Steward",
                line.member
            ));
        }
        let Some((sponsor_line, steward)) = self.active_steward(sponsor) else {
            let held = self
                .current
                .values()
                .find(|(_, a)| a.member == sponsor)
                .or_else(|| self.current.values().find(|(_, a)| a.names(sponsor)))
                .map_or("no affiliation at all".to_string(), |(_, a)| {
                    if a.withdrawn.is_some() {
                        format!("a withdrawn {:?} affiliation", a.role)
                    } else {
                        format!("a {:?} affiliation", a.role)
                    }
                });
            return Err(format!(
                "sponsor {sponsor} is not an active Steward of this collective at this point in \
                 the fold ({held})"
            ));
        };
        if steward.same_author_as(&line.member) {
            return Err(format!(
                "self-sponsorship: sponsor {sponsor} (Steward line {}) is the same author as the \
                 member {} (agent refs compare by role)",
                steward.member, line.member
            ));
        }
        if steward.standing == AffiliationStanding::Fixture
            && line.standing != AffiliationStanding::Fixture
            && line.role != MembershipRole::Contributor
        {
            return Err(format!(
                "fixture Steward {sponsor} cannot sponsor a standing {:?} line for {}: a fixture \
                 may sponsor only a Fixture or Contributor line, never real authority",
                line.role, line.member
            ));
        }
        Ok(Admission {
            sponsor_line: Some(sponsor_line.to_string()),
            kind: AdmissionKind::Sponsored,
        })
    }

    /// A line arriving while the collective is stewardless: only a re-founding genesis stands.
    fn refound(
        &self,
        line: &Affiliation,
        since: &str,
        founding: bool,
    ) -> Result<Admission, String> {
        let stewardless = |how: &str| {
            format!(
                "the collective is stewardless (its last Steward left under declaration \
                 {since}): no Steward can sponsor {}; {how}",
                line.member
            )
        };
        match &self.refounding {
            Refounding::Root {
                current,
                supersedes,
            } => {
                let how = "a root collective is re-founded only through its declaration — \
                           amend it by a reviewed edit whose `supersedes` names the stewardless \
                           declaration, then append a sponsorless standing genesis Steward line \
                           pinned to the amended declaration";
                if !founding || line.sponsor.is_some() {
                    return Err(stewardless(how));
                }
                let pinned = line.collective.cid.as_str();
                let position = |cid: &str| supersedes.iter().position(|c| c == cid);
                // Newest first: the pin must be the current declaration or an amendment that
                // came after the stewardless one in the lineage.
                let moved = pinned != since
                    && position(since).is_some()
                    && (pinned == current
                        || matches!((position(pinned), position(since)), (Some(p), Some(s)) if p < s));
                if !moved {
                    return Err(stewardless(how));
                }
                Ok(Admission {
                    sponsor_line: None,
                    kind: AdmissionKind::RefoundedByDeclaration,
                })
            }
            Refounding::Child { parent } => {
                let how = "a child collective is re-founded by a Steward of its parent \
                           collective sponsoring a standing genesis Steward line";
                let sponsor = line.sponsor.as_deref().filter(|_| founding);
                let steward = sponsor.and_then(|sponsor| {
                    let active = || parent.iter().filter(|(_, a)| a.is_active_steward());
                    active()
                        .find(|(_, a)| a.member == sponsor)
                        .or_else(|| active().find(|(_, a)| a.names(sponsor)))
                });
                let Some((cid, steward)) = steward else {
                    return Err(stewardless(how));
                };
                if steward.same_author_as(&line.member)
                    || steward.standing == AffiliationStanding::Fixture
                {
                    return Err(stewardless(
                        "the parent Steward sponsoring a re-founding must be standing and not \
                         the new genesis Steward itself",
                    ));
                }
                Ok(Admission {
                    sponsor_line: Some(cid.clone()),
                    kind: AdmissionKind::RefoundedByParent,
                })
            }
            Refounding::Closed => Err(stewardless("no re-founding route is known here")),
        }
    }

    /// The active Steward `participant` stands as: the exact member, else a package-level agent
    /// Steward naming the build (the same narrow lookup standing uses).
    fn active_steward(&self, participant: &str) -> Option<(&str, &Affiliation)> {
        let active = || self.current.values().filter(|(_, a)| a.is_active_steward());
        active()
            .find(|(_, a)| a.member == participant)
            .or_else(|| active().find(|(_, a)| a.names(participant)))
            .map(|(cid, a)| (cid.as_str(), a))
    }

    fn accept(&mut self, cid: String, line: Affiliation, admitted: Admitted) {
        let pinned = line.collective.cid.clone();
        if matches!(
            admitted.kind,
            AdmissionKind::Genesis
                | AdmissionKind::RefoundedByDeclaration
                | AdmissionKind::RefoundedByParent
        ) {
            self.genesis = true;
            self.stewardless_since = None;
        }
        if !self.current.contains_key(&line.member) {
            self.order.push(line.member.clone());
        }
        self.admitted.insert(cid.clone(), admitted);
        self.current.insert(line.member.clone(), (cid, line));
        if self.genesis
            && self.stewardless_since.is_none()
            && !self.current.values().any(|(_, a)| a.is_active_steward())
        {
            self.stewardless_since = Some(pinned);
        }
    }

    fn finish(mut self) -> FoldOutcome {
        let current = self
            .order
            .iter()
            .filter_map(|member| self.current.remove(member))
            .collect();
        FoldOutcome {
            current,
            admitted: self.admitted,
            founded: self.genesis,
            stewardless_since: self.stewardless_since,
        }
    }
}

/// Whether `sponsor` names the same participant as `member` (agent refs compare by role).
fn agent_or_same(sponsor: &str, member: &str) -> bool {
    sponsor == member
        || matches!((agent_role(sponsor), agent_role(member)), (Some(a), Some(b)) if a == b)
}

/// What folding one collective's lines yields.
pub struct FoldOutcome {
    pub current: Vec<(String, Affiliation)>,
    pub admitted: BTreeMap<String, Admitted>,
    pub founded: bool,
    pub stewardless_since: Option<String>,
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
        supersedes_chain(path, &file.reference.cid, &declaration.supersedes)?;
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
        let mut parent_stewards: Vec<(String, Affiliation)> = Vec::new();
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
                parent_stewards = upper.stewards().cloned().collect();
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
        let refounding = if path == COLLECTIVE_PATH {
            Refounding::Root {
                current: file.reference.cid.clone(),
                supersedes: declaration.supersedes.clone(),
            }
        } else {
            Refounding::Child {
                parent: parent_stewards.clone(),
            }
        };
        let (outcome, refused_lines) = self.affiliations_for(&file.reference, refounding)?;
        if !outcome.founded {
            return Err(refused(format!(
                "collective {} ({path}) has no Steward on record — it was never founded: \
                 stewards are affiliation records (role Steward) in {AFFILIATIONS_PATH}, never a \
                 declaration field, and a collective is founded by its genesis Steward line",
                declaration.id
            )));
        }
        Ok(Governance {
            reference: file.reference,
            declaration,
            affiliations: outcome.current,
            invalid_lines: refused_lines.len(),
            refused: refused_lines,
            admitted: outcome.admitted,
            stewardless_since: outcome.stewardless_since,
            parent_stewards,
        })
    }

    /// Every current affiliation with the collective whose declaration is at `reference.path`,
    /// folded through the sponsorship chain ([`Fold`]), with the lines that count for nothing.
    ///
    /// Matched by declaration PATH: a charter amendment re-pins the declaration's CID, and a
    /// member's standing does not lapse because the text they affiliated under was amended — the
    /// CID each member affiliated under stays on their line as the record of it.
    fn affiliations_for(
        &self,
        reference: &FileRef,
        refounding: Refounding,
    ) -> FlowResult<(FoldOutcome, Vec<RefusedLine>)> {
        let mut fold = Fold::new(refounding);
        if !self.root.join(AFFILIATIONS_PATH).exists() {
            return Ok((fold.finish(), Vec::new()));
        }
        self.sidecar(AFFILIATIONS_PATH)?;
        let text = std::fs::read_to_string(self.path(AFFILIATIONS_PATH)?)?;
        let mut refused = Vec::new();
        for (index, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let number = index + 1;
            let parsed = match parse_affiliation_line(line) {
                Ok(parsed) => parsed,
                Err(reason) => {
                    refused.push(RefusedLine {
                        line: number,
                        cid: None,
                        member: None,
                        reason,
                    });
                    continue;
                }
            };
            if parsed.record.collective.path != reference.path {
                continue;
            }
            let verdict = fold.admit(&parsed.record).and_then(|admission| {
                self.signer_enrolled(&parsed)?;
                Ok(admission)
            });
            match verdict {
                Ok(admission) => fold.accept(
                    parsed.cid,
                    parsed.record.clone(),
                    Admitted {
                        record: parsed.record,
                        sponsor_line: admission.sponsor_line,
                        kind: admission.kind,
                        signature: parsed.signature,
                    },
                ),
                Err(reason) => refused.push(RefusedLine {
                    line: number,
                    cid: Some(parsed.cid),
                    member: Some(parsed.record.member),
                    reason,
                }),
            }
        }
        Ok((fold.finish(), refused))
    }

    /// A signed line's signer must be a device enrolled for the party that signs it: the
    /// sponsor, or the member of a genesis line. Only a human has a device roster.
    fn signer_enrolled(&self, line: &AffiliationLine) -> Result<(), String> {
        let LineSignature::Signed { signer, .. } = &line.signature else {
            return Ok(());
        };
        let party = line
            .record
            .sponsor
            .as_deref()
            .unwrap_or(&line.record.member);
        let enrolled = match parse_participant_ref(party) {
            Ok(ParticipantRef::Human { handle }) => {
                crate::actor::device_enrolled(&self.root, &handle, signer)
            }
            _ => false,
        };
        if enrolled {
            Ok(())
        } else {
            Err(format!(
                "signed by {signer}, a device not enrolled for {party} in its participant roster"
            ))
        }
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

    /// A pinned collective must be the current declaration, or an earlier declaration of the
    /// SAME collective named in its `supersedes` chain. `Ok(None)` is the current declaration;
    /// `Ok(Some(line))` is lineage, and the line is what a surface prints instead of hiding it.
    pub fn require_collective(
        &self,
        supplied: &FileRef,
        governance: &Governance,
    ) -> FlowResult<Option<String>> {
        let actual = &governance.reference;
        if supplied == actual {
            return Ok(None);
        }
        if supplied.path == actual.path && governance.declaration.supersedes.contains(&supplied.cid)
        {
            return Ok(Some(lineage_line(&supplied.cid)));
        }
        Err(refused(format!(
            "collective declaration version/path differs; re-read current governance: pinned \
             {}@{} is neither the current declaration {}@{} nor in its supersedes chain",
            supplied.path, supplied.cid, actual.path, actual.cid
        )))
    }

    /// Like [`Self::require_collective`], but a NEW write must pin the current declaration: new
    /// work is never filed under a charter that has already been amended.
    pub fn require_current(&self, supplied: &FileRef, governance: &Governance) -> FlowResult<()> {
        match self.require_collective(supplied, governance)? {
            None => Ok(()),
            Some(_) => Err(refused(format!(
                "collective declaration version/path differs; re-read current governance: \
                 {}@{} is a superseded declaration — a new contribution pins the current one, {}",
                supplied.path, supplied.cid, governance.reference.cid
            ))),
        }
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

    /// Validates a contribution under `governance`. Returns the lineage line when the
    /// contribution is pinned to a superseded declaration of the same collective (see
    /// [`Self::require_collective`]); a caller filing NEW work refuses that with `require_current`.
    pub fn contribution(
        &mut self,
        c: &Contribution,
        governance: &Governance,
    ) -> FlowResult<Option<String>> {
        let collective = &governance.declaration;
        version(c.version)?;
        let lineage = self.require_collective(&c.collective, governance)?;
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
        elohim_epr_rea::parse_acting_participant(&c.author)?;
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
        Ok(lineage)
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

/// How many earlier declarations a lineage may name before the walk is refused.
const MAX_LINEAGE: usize = 64;

/// What a surface prints for work pinned to an earlier declaration of the same collective.
pub fn lineage_line(cid: &str) -> String {
    format!("pinned to superseded declaration {cid} (lineage ok)")
}

/// Walk a declaration's `supersedes` chain with a bound, refusing a cycle (a CID that recurs,
/// or the declaration naming its own CID), naming the CIDs involved.
fn supersedes_chain(path: &str, current: &str, chain: &[String]) -> FlowResult<()> {
    if chain.len() > MAX_LINEAGE {
        return Err(refused(format!(
            "{path}: supersedes chain exceeds {MAX_LINEAGE} declarations; stop or narrow it"
        )));
    }
    let mut seen = BTreeSet::new();
    for (hop, cid) in chain.iter().enumerate() {
        if cid.parse::<Cid>().is_err() {
            return Err(refused(format!(
                "{path}: supersedes[{hop}] `{cid}` is not a CID"
            )));
        }
        if cid == current {
            return Err(refused(format!(
                "{path}: supersedes chain is a cycle — it names the declaration's own CID {cid}"
            )));
        }
        if !seen.insert(cid.as_str()) {
            let first = chain.iter().position(|c| c == cid).unwrap_or(0);
            return Err(refused(format!(
                "{path}: supersedes chain is a cycle — {cid} recurs at hops {first} and {hop}"
            )));
        }
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

/// How an affiliation line is signed: the honest literal `unsigned`, or a DETACHED device
/// signature over the record's CID (a line field, never a record field, so the CID is unchanged).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LineSignature {
    Unsigned,
    Signed {
        /// The signing device, `did:key:…` — public material only.
        signer: String,
        /// The 64-byte ed25519 signature over [`affiliation_signing_message`], lowercase hex.
        signature: String,
    },
}

impl LineSignature {
    fn to_value(&self) -> Value {
        match self {
            LineSignature::Unsigned => json!("unsigned"),
            LineSignature::Signed { signer, signature } => {
                json!({"signer": signer, "signature": signature})
            }
        }
    }
}

/// One verified `{cid, record, signature}` sidecar line.
#[derive(Clone, Debug)]
pub struct AffiliationLine {
    pub cid: String,
    pub record: Affiliation,
    pub signature: LineSignature,
}

const AFFILIATION_SIGNING_DOMAIN: &str = "elohim:affiliation-sponsorship-signature:v1:";

/// The bytes a sponsor's device signs for one affiliation line: a domain tag and the record CID.
/// Its own domain, so an actor-record signature can never be replayed as a sponsorship.
pub fn affiliation_signing_message(record_cid: &str) -> Vec<u8> {
    format!("{AFFILIATION_SIGNING_DOMAIN}{record_cid}").into_bytes()
}

/// One sidecar line, `{cid, record, signature}`, verified: the record parses strictly, its
/// member and parties are shaped as participants (never an email), the CID re-derives from the
/// record, and a signature — when present — verifies as its signer's over that CID. The refusal
/// names why the line counts for nothing. Roster enrolment of the signer needs the tree and is
/// checked by the fold.
pub fn parse_affiliation_line(line: &str) -> Result<AffiliationLine, String> {
    let value: Value =
        serde_json::from_str(line).map_err(|e| format!("not a JSON object line: {e}"))?;
    let object = value
        .as_object()
        .ok_or_else(|| "not a JSON object line".to_string())?;
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    if keys != ["cid", "record", "signature"] {
        return Err(format!(
            "line keys are {keys:?}; an affiliation line is exactly {{cid, record, signature}}"
        ));
    }
    let affiliation: Affiliation = serde_json::from_value(object["record"].clone())
        .map_err(|e| format!("record does not parse: {e}"))?;
    validate_affiliation(&affiliation).map_err(|e| e.to_string())?;
    let cid = atom_cid(&affiliation)
        .map_err(|e| e.to_string())?
        .to_string();
    if object["cid"].as_str() != Some(cid.as_str()) {
        return Err(format!(
            "line CID does not re-derive from its record (record is {cid})"
        ));
    }
    let signature = match &object["signature"] {
        Value::String(s) if s == "unsigned" => LineSignature::Unsigned,
        Value::Object(sig) if sig.len() == 2 => {
            let signer = sig
                .get("signer")
                .and_then(Value::as_str)
                .ok_or("signature names no signer")?;
            let hex = sig
                .get("signature")
                .and_then(Value::as_str)
                .ok_or("signature carries no signature bytes")?;
            let verified = hex_decode(hex).is_some_and(|bytes| {
                crate::device_key::verify(signer, &affiliation_signing_message(&cid), &bytes)
            });
            if !verified {
                return Err(format!(
                    "signature does not verify as {signer}'s over {cid}"
                ));
            }
            LineSignature::Signed {
                signer: signer.to_string(),
                signature: hex.to_string(),
            }
        }
        _ => {
            return Err(
                "signature is neither the literal \"unsigned\" nor {signer, signature}".into(),
            )
        }
    };
    Ok(AffiliationLine {
        cid,
        record: affiliation,
        signature,
    })
}

/// [`parse_affiliation_line`], reduced to `(cid, record)`; `None` is a line that counts for
/// nothing.
pub fn verify_affiliation_line(line: &str) -> Option<(String, Affiliation)> {
    parse_affiliation_line(line)
        .ok()
        .map(|parsed| (parsed.cid, parsed.record))
}

/// The UNSIGNED `{cid, record, signature}` line an affiliation is appended as.
pub fn affiliation_line(affiliation: &Affiliation) -> FlowResult<String> {
    affiliation_line_signed(affiliation, None)
}

/// The line an affiliation is appended as, signed by `device` over its record CID when given,
/// else carrying the honest literal `unsigned`.
pub fn affiliation_line_signed(
    affiliation: &Affiliation,
    device: Option<&crate::device_key::DeviceKey>,
) -> FlowResult<String> {
    validate_affiliation(affiliation)?;
    let cid = atom_cid(affiliation)?.to_string();
    let signature = match device {
        Some(key) => LineSignature::Signed {
            signer: key.did_key(),
            signature: eprfs_meta::hex_lower(&key.sign(&affiliation_signing_message(&cid))),
        },
        None => LineSignature::Unsigned,
    };
    Ok(serde_json::to_string(
        &json!({"cid": cid, "record": affiliation, "signature": signature.to_value()}),
    )?)
}

fn hex_decode(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect()
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
        // A sponsor is whoever stood as an active Steward, and an agent build may be one.
        let agent_sponsor = a.sponsor.as_deref() == Some(party.as_str())
            && party
                .strip_prefix("agent:")
                .is_some_and(|rest| parse_agent_ref(party).is_ok() || is_slug(rest));
        if !ok && !agent_sponsor {
            return Err(refused(format!(
                "affiliation party `{party}` is not a human:, repo: or collective: ref (a \
                 sponsor may also be an agent: ref)"
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
