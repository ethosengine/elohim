//! Local collective memory contracts. Authored references are local B-class inputs;
//! projections are reconstructible C-class context, never membership or acceptance proofs.
//! These specialize existing content/observation grammar, not core EPR kinds.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileRef {
    pub path: String,
    /// Exact raw file BlobCid, including any metadata; unlike a document body CID.
    pub cid: String,
}

/// Where a passage may travel INSIDE this repository: a placement tier, never an audience.
///
/// "Reach" is the network's audience vocabulary (schema 8: self … commons); the reach/locality
/// split spec (2026-07-22) keeps the two apart so a repository placement tier can never be
/// misread at the network boundary as an audience grant. The wire spellings are unchanged
/// (`private|workspace|repository`), so every contribution written before the rename still reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Locality {
    Private,
    Workspace,
    Repository,
}

/// A local collective declaration: `<dir>/.epr-meta/collective.json`.
///
/// It names NO steward. Stewardship is plural and lives in [`Affiliation`] records (the local
/// pre-image of the notarized Qahal `Membership`), so a declaration cannot elect its own
/// steward, and a collective with nobody on record as Steward is refused by its reader. A
/// declaration below the repository root is a CHILD collective: its `parent` pins the enclosing
/// declaration by path and raw CID. Existing actor claims own mutable session attribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Collective {
    pub version: u32,
    pub id: String,
    pub display_name: String,
    pub charter: String,
    pub participation: String,
    /// The enclosing declaration, pinned. `None` only for the repository-root collective.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<FileRef>,
    /// Where this collective stands in the collectives registry's vocabulary
    /// (`genesis/data/collectives/collectives.json`): declared terms, never a copied fixture.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry: Option<RegistryTerms>,
    /// The lineage of this declaration: the raw CIDs of the earlier declarations at the SAME
    /// path that this one amends, newest first. An amendment prepends the CID it replaces.
    ///
    /// It is carried as a list, not a single back-pointer, because a superseded declaration's
    /// bytes do not stay in the tree: a lone pointer could not be walked offline past its first
    /// hop. Work pinned to any CID in this chain was contributed under this collective's earlier
    /// charter and keeps its standing (reported as lineage, never hidden); a repeated CID is a
    /// cycle and is refused.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supersedes: Vec<String>,
    pub source_rules: Vec<SourceRule>,
}

/// A collective's registration, spoken in the collectives registry's own vocabulary.
///
/// `reach` HERE is correct usage: the registry is the network audience vocabulary, and this is
/// the audience the collective would stand at when it crosses. It is never a source rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegistryTerms {
    /// The registry file these terms are checked against, by repository-relative path.
    pub catalog: String,
    /// An existing registry collective id this collective appeals to.
    pub constitutional_parent_id: String,
    /// One of the registry schema's `governanceLayer` values.
    pub governance_layer: String,
    /// One of the registry's `reachConstraints` keys.
    pub reach: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceRule {
    pub path: String,
    pub max_locality: Locality,
}

/// Which kind of member an [`Affiliation`] names: `qahal::MemberKind`, variant for variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemberKind {
    Person,
    Collective,
    ElohimAgent,
}

/// The member's role in the collective: `qahal::MembershipRole`, variant for variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MembershipRole {
    Steward,
    Contributor,
    Observer,
}

/// Whether a member stands for real or is a test fixture.
///
/// NO counterpart in `qahal::Membership`: a local-only field. A `Fixture` member is a test human
/// (e.g. `human:adam`) who co-stewards at Bootstrap stakes so the real primitives run end to end.
/// Every surface that reports an approval resting on a `Fixture` member must say so, and
/// [`requires_non_fixture_stewards`] is the gate that refuses such approvals wherever real peer
/// validation is owed (settlement, an external offer). Serde-strict: an unknown spelling is
/// refused, never defaulted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AffiliationStanding {
    Standing,
    Fixture,
}

/// One member's affiliation with one collective: a `{cid, record}` line in
/// `.eprfs/status/affiliations.jsonl`.
///
/// The local PRE-IMAGE of the notarized `imagodei_integrity::qahal::Membership`, so crossing is a
/// mint rather than a translation: `member` ↔ `member_cid`, `member_kind` ↔ `member_kind`,
/// `collective` ↔ `collective_cid`, `role` ↔ `role`, `sponsor` ↔ `sponsor_cid`, `since` ↔
/// `joined_at_block_height`, `withdrawn` ↔ `withdrawn_at_block_height`. `version`, `acts_for` and
/// `standing` are local-only (no Membership field carries them); the crossing must decide how
/// each is carried before it mints. The sidecar is append-only: the LAST line for a
/// `(collective path, member)` pair is that member's current affiliation, and a line with
/// `withdrawn` set ends it without rewriting history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Affiliation {
    pub version: u32,
    /// The collective's declaration, by path and the raw CID the member affiliated under.
    pub collective: FileRef,
    /// `human:<handle>`, `agent:<role>[@<model>]` or `collective:<id>`; never an email.
    pub member: String,
    pub member_kind: MemberKind,
    pub role: MembershipRole,
    /// Who sponsored a Steward pending counter-attestation (a participant ref), if anyone.
    #[serde(default)]
    pub sponsor: Option<String>,
    /// On whose behalf the member acts. `None` on a root-collective affiliation means the
    /// repository agent (`repo:<owner>/<name>`).
    #[serde(default)]
    pub acts_for: Option<String>,
    pub standing: AffiliationStanding,
    /// When the affiliation began (RFC3339, the tree it was made against).
    pub since: String,
    /// When the affiliation was withdrawn, if it was.
    #[serde(default)]
    pub withdrawn: Option<String>,
}

impl Affiliation {
    /// A Steward still on record: role Steward and never withdrawn.
    pub fn is_active_steward(&self) -> bool {
        self.role == MembershipRole::Steward && self.withdrawn.is_none()
    }

    /// Whether this affiliation stands for `participant`: the exact participant, or, for an
    /// agent affiliation named at package level (`agent:<role>`), any build of that role.
    pub fn names(&self, participant: &str) -> bool {
        if self.member == participant {
            return true;
        }
        self.member_kind == MemberKind::ElohimAgent
            && !self.member.contains('@')
            && participant
                .split_once('@')
                .is_some_and(|(role, _)| role == self.member)
    }

    /// Whether this affiliation's member is the SAME author as `author` for anti-self-election.
    ///
    /// Wider than [`Self::names`] on purpose, and used only for that refusal: when either side is
    /// an agent ref, both are normalized to their role first, so any build of a role counts as
    /// the author of work by any other build of it (`agent:code-reviewer@opus-5` may not approve
    /// `agent:code-reviewer@sonnet-5`). Standing lookup keeps the narrower [`Self::names`], so a
    /// build-specific Steward affiliation never lends its standing to a sibling build.
    pub fn same_author_as(&self, author: &str) -> bool {
        self.names(author)
            || matches!(
                (agent_role(&self.member), agent_role(author)),
                (Some(mine), Some(theirs)) if mine == theirs
            )
    }
}

/// The role an agent ref names — `agent:<role>` or `agent:<role>@<model>` — else `None`.
pub fn agent_role(participant: &str) -> Option<&str> {
    participant
        .strip_prefix("agent:")
        .map(|rest| rest.split_once('@').map_or(rest, |(role, _)| role))
}

/// Refuses unless at least `n` DISTINCT non-fixture, still-standing Stewards approved.
///
/// The future settlement / external-offer gate: a value crossing to token or fiat, or an offer
/// the protocol makes outside itself, is a governance act a fixture co-steward can never carry.
/// Its first caller is a consuming-app offer with `crossesNetwork: true` (bridges/jenkins); the
/// future settlement Agreement is the second.
/// Distinctness is by `member`, so one steward approving twice counts once.
pub fn requires_non_fixture_stewards(approvals: &[&Affiliation], n: usize) -> Result<(), String> {
    let mut real: Vec<&str> = approvals
        .iter()
        .filter(|a| a.is_active_steward() && a.standing == AffiliationStanding::Standing)
        .map(|a| a.member.as_str())
        .collect();
    real.sort_unstable();
    real.dedup();
    if real.len() < n {
        let fixtures = approvals
            .iter()
            .filter(|a| a.standing == AffiliationStanding::Fixture)
            .count();
        return Err(format!(
            "requires {n} distinct non-fixture Steward approvals; found {} ({fixtures} fixture \
             approval(s) never count toward it)",
            real.len()
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Source {
    pub resource: FileRef,
    pub reach: Locality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Contribution {
    pub version: u32,
    pub collective: FileRef,
    pub author: String,
    pub steward: String,
    pub scope: String,
    pub reach: Locality,
    pub concern: String,
    pub claim: String,
    pub uncertainty: Vec<String>,
    pub sources: Vec<Source>,
    pub supersedes: Vec<FileRef>,
    pub contradicts: Vec<FileRef>,
    /// Present only when the claim was authored OUTSIDE the flow plane and imported verbatim.
    ///
    /// Additive and skipped when absent, so every contribution file written before this field
    /// existed still parses and still serializes to the same bytes — the projection receipts that
    /// embed a contribution are unchanged by its arrival.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imported: Option<Imported>,
}

/// The provenance of an imported entry: who wrote the bytes, where they live, and how the entry
/// declared itself.
///
/// This is deliberately NOT a second identity. `Contribution::author` stays the acting
/// participant's claim (the collective refuses anything else), and `Contribution::steward` stays
/// the collective's steward. `git_name` records the display name of the human whose commit the
/// pinned bytes came from — provenance of a source, never a claim of standing, never a minted
/// persona for a human who never asked for one, and never their email.
///
/// ## The identity reserve
///
/// The substrate copies a human into fruit only as a name the commit already publishes, or the
/// handle they claimed. The email is a cross-namespace key; it has no field here, so a tracked
/// contribution cannot carry one. The retired `gitAuthor` (`Name <email>`) is refused by
/// `deny_unknown_fields` rather than aliased: a store still carrying it is migrated by the one
/// attributed act `epr flow memory migrate-identity-reserve`, never silently read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Imported {
    /// The entry's own display name — its `title:`, else its `name:`, else its file stem.
    pub display: String,
    /// The entry's file name, which is the link target a projected index renders.
    pub file: String,
    /// The entry's repository-relative path: the scope the claim was authored for.
    pub scope_path: String,
    /// The display name (`%an`, no email) of the commit the pinned source bytes came from, or an
    /// honest absence such as `(untracked: no commit carries these bytes)`.
    pub git_name: String,
    /// The entry's declared `metadata.type`.
    pub entry_type: String,
    /// False when the entry declared `index: false` — it stays a contribution and stays out of
    /// the projected index, exactly as its author asked.
    pub indexed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectionRequest {
    pub version: u32,
    pub collective: FileRef,
    pub purpose: String,
    pub audience: Locality,
    pub inputs: Vec<FileRef>,
    pub omissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FeedbackKind {
    StaleSource,
    MisleadingProjection,
    OmittedContradiction,
    PoorSelection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Feedback {
    pub version: u32,
    pub collective: FileRef,
    pub target: FileRef,
    pub kind: FeedbackKind,
    pub passage: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Graduation {
    pub version: u32,
    pub collective: FileRef,
    pub contribution: FileRef,
    /// Exact existing native approved verdict event, never a claimed boolean.
    pub review: String,
    pub audience: Locality,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn steward(member: &str, standing: AffiliationStanding) -> Affiliation {
        Affiliation {
            version: 1,
            collective: FileRef {
                path: ".epr-meta/collective.json".into(),
                cid: "bafkreiexample".into(),
            },
            member: member.into(),
            member_kind: MemberKind::Person,
            role: MembershipRole::Steward,
            sponsor: None,
            acts_for: None,
            standing,
            since: "2026-09-25T00:00:00Z".into(),
            withdrawn: None,
        }
    }

    #[test]
    fn one_real_and_one_fixture_steward_do_not_make_two() {
        let matthew = steward("human:matthew", AffiliationStanding::Standing);
        let adam = steward("human:adam", AffiliationStanding::Fixture);
        let err = requires_non_fixture_stewards(&[&matthew, &adam], 2).unwrap_err();
        assert!(err.contains("found 1"), "{err}");
        assert!(err.contains("1 fixture"), "{err}");
        assert!(requires_non_fixture_stewards(&[&matthew, &adam], 1).is_ok());
    }

    #[test]
    fn distinct_real_stewards_pass_and_a_repeat_counts_once() {
        let matthew = steward("human:matthew", AffiliationStanding::Standing);
        let other = steward("human:ruth", AffiliationStanding::Standing);
        assert!(requires_non_fixture_stewards(&[&matthew, &other], 2).is_ok());
        assert!(requires_non_fixture_stewards(&[&matthew, &matthew], 2).is_err());
        let mut withdrawn = other.clone();
        withdrawn.withdrawn = Some("2026-09-26T00:00:00Z".into());
        assert!(requires_non_fixture_stewards(&[&matthew, &withdrawn], 2).is_err());
        let mut contributor = other;
        contributor.role = MembershipRole::Contributor;
        assert!(requires_non_fixture_stewards(&[&matthew, &contributor], 2).is_err());
    }

    #[test]
    fn standing_is_strict_and_unknown_affiliation_fields_are_refused() {
        let good =
            serde_json::to_value(steward("human:adam", AffiliationStanding::Fixture)).unwrap();
        assert_eq!(good["standing"], "Fixture");
        let mut bad = good.clone();
        bad["standing"] = serde_json::json!("fixture-ish");
        assert!(serde_json::from_value::<Affiliation>(bad).is_err());
        let mut missing = good.clone();
        missing.as_object_mut().unwrap().remove("standing");
        assert!(serde_json::from_value::<Affiliation>(missing).is_err());
        let mut extra = good;
        extra["email"] = serde_json::json!("x");
        assert!(serde_json::from_value::<Affiliation>(extra).is_err());
    }

    #[test]
    fn a_package_level_agent_affiliation_names_every_build_of_its_role() {
        let mut agent = steward("agent:reviewer", AffiliationStanding::Standing);
        agent.member_kind = MemberKind::ElohimAgent;
        assert!(agent.names("agent:reviewer@opus-5"));
        assert!(!agent.names("agent:reviewer-two@opus-5"));
        let person = steward("human:matthew", AffiliationStanding::Standing);
        assert!(person.names("human:matthew"));
        assert!(!person.names("human:matthew@x"));
    }

    #[test]
    fn same_author_normalizes_agent_refs_to_their_role_on_either_side() {
        let mut build = steward("agent:code-reviewer@opus-5", AffiliationStanding::Standing);
        build.member_kind = MemberKind::ElohimAgent;
        // A build-specific affiliation does not NAME a sibling build (no borrowed standing)…
        assert!(!build.names("agent:code-reviewer@sonnet-5"));
        // …but it is the same author for anti-self-election.
        assert!(build.same_author_as("agent:code-reviewer@sonnet-5"));
        assert!(build.same_author_as("agent:code-reviewer"));
        assert!(!build.same_author_as("agent:rust-architect@opus-5"));
        let person = steward("human:matthew", AffiliationStanding::Standing);
        assert!(!person.same_author_as("agent:matthew@opus-5"));
        assert!(person.same_author_as("human:matthew"));
    }
}
