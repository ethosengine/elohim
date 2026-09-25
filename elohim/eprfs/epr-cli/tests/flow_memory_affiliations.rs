//! Plural stewardship on record: a collective declaration names no steward; Stewards are
//! affiliation records (the local pre-image of Qahal `Membership`); a child collective pins its
//! parent; a claim binds its collective of record; graduation needs a distinct Steward.
//! Synthetic roots own every write, except the read-only checks of the repository's own seed.
use std::path::Path;

use elohim_epr_cli::{
    actor,
    flow::{memory, note},
};
use eprfs_agent::memory::{
    requires_non_fixture_stewards, Affiliation, AffiliationStanding, FileRef, MemberKind,
    MembershipRole,
};
use serde_json::{json, Value};
use tempfile::TempDir;

const ROOT: &str = ".epr-meta/collective.json";
const CHILD: &str = "genesis/concern/.epr-meta/collective.json";

fn write(root: &Path, path: &str, value: &Value) {
    let file = root.join(path);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}
fn pin(root: &Path, path: &str) -> Value {
    memory::execute(root, "pin", Some(path), None).unwrap()["resource"].clone()
}
fn declaration() -> Value {
    serde_json::from_str(include_str!("../../../../.epr-meta/collective.json")).unwrap()
}
fn affiliation(
    root: &Path,
    collective: &str,
    member: &str,
    kind: MemberKind,
    role: MembershipRole,
    standing: AffiliationStanding,
) -> Affiliation {
    let bytes = std::fs::read(root.join(collective)).unwrap();
    Affiliation {
        version: 1,
        collective: FileRef {
            path: collective.into(),
            cid: eprfs_core::BlobCid::compute_raw(&bytes).to_string(),
        },
        member: member.into(),
        member_kind: kind,
        role,
        sponsor: None,
        acts_for: None,
        standing,
        since: "2026-09-25T00:00:00Z".into(),
        withdrawn: None,
    }
}
fn append(root: &Path, record: &Affiliation) {
    use std::io::Write;
    let path = root.join(memory::AFFILIATIONS_PATH);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap();
    writeln!(file, "{}", memory::affiliation_line(record).unwrap()).unwrap();
}
/// A standing affiliation with the root collective: human:matthew's is the genesis Steward line;
/// every other member is sponsored by him (the sponsorship chain, `flow_memory_sponsorship.rs`).
fn affiliate(root: &Path, member: &str, kind: MemberKind, role: MembershipRole) {
    let mut record = affiliation(
        root,
        ROOT,
        member,
        kind,
        role,
        AffiliationStanding::Standing,
    );
    record.sponsor = (member != "human:matthew").then(|| "human:matthew".to_string());
    append(root, &record);
}
fn git(root: &Path, args: &[&str]) {
    let status = elohim_epr_cli::process::build_command("git", args, root, &[])
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.test")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.test")
        .output()
        .unwrap();
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
}

/// A root collective whose Stewards are human:matthew (standing) and human:adam (fixture), with
/// the investigator agent as a Contributor. Sessions: `author` (the investigator), `matthew`,
/// `adam`, `contributor` (another investigator build).
fn fixture() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, ROOT, &declaration());
    write(
        root,
        "genesis/evidence.json",
        &json!({"observed":"Only narrow claims are supported"}),
    );
    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "fixture"]);
    affiliate(
        root,
        "human:matthew",
        MemberKind::Person,
        MembershipRole::Steward,
    );
    let mut adam = affiliation(
        root,
        ROOT,
        "human:adam",
        MemberKind::Person,
        MembershipRole::Steward,
        AffiliationStanding::Fixture,
    );
    adam.sponsor = Some("human:matthew".into());
    append(root, &adam);
    affiliate(
        root,
        "agent:investigator",
        MemberKind::ElohimAgent,
        MembershipRole::Contributor,
    );
    actor::claim(root, "agent:investigator@fixture", "author").unwrap();
    actor::claim(root, "agent:investigator@other", "contributor").unwrap();
    actor::claim(root, "human:matthew", "matthew").unwrap();
    actor::claim(root, "human:adam", "adam").unwrap();
    dir
}
fn contribute(root: &Path) -> Value {
    write(
        root,
        "genesis/assertion.json",
        &json!({"version":1,"collective":pin(root,ROOT),
        "author":"agent:investigator@fixture","steward":"repo:ethosengine/elohim",
        "scope":"workspace","reach":"repository","concern":"qualified evidence",
        "claim":"Only the measured scope is supported","uncertainty":["Wider use remains untested"],
        "sources":[{"resource":pin(root,"genesis/evidence.json"),"reach":"repository"}],"supersedes":[],"contradicts":[]}),
    );
    memory::execute(
        root,
        "contribute",
        Some("genesis/assertion.json"),
        Some("author"),
    )
    .unwrap()
}
fn verdict(root: &Path, session: &str) -> String {
    note::note(
        root,
        "genesis/assertion.json",
        "verdict",
        "Approved only this narrow scope",
        None,
        Some("approved"),
        &note::NoteActor {
            as_ref: None,
            session: Some(session.into()),
        },
    )
    .unwrap()
    .record_cid
}
fn graduate(root: &Path, review: &str) -> Result<Value, String> {
    write(
        root,
        "genesis/graduation.json",
        &json!({"version":1,"collective":pin(root,ROOT),
        "contribution":pin(root,"genesis/assertion.json"),"review":review,"audience":"repository"}),
    );
    memory::execute(root, "graduate", Some("genesis/graduation.json"), None)
        .map_err(|e| e.to_string())
}
fn collective_err(root: &Path) -> String {
    memory::execute(root, "collective", None, None)
        .unwrap_err()
        .to_string()
}

#[test]
fn a_declaration_that_names_a_steward_is_refused_as_an_unknown_field() {
    let dir = fixture();
    let root = dir.path();
    let mut value = declaration();
    value["steward"] = json!("repo:ethosengine/elohim");
    write(root, ROOT, &value);
    let err = collective_err(root);
    assert!(err.contains("unknown field `steward`"), "{err}");
    assert!(err.contains("stewards are affiliation records"), "{err}");
}

#[test]
fn a_declaration_that_still_speaks_reach_is_refused_with_the_rename_named() {
    let dir = fixture();
    let root = dir.path();
    let mut value = declaration();
    value["sourceRules"][0] = json!({"path":".epr-meta","maxReach":"repository"});
    write(root, ROOT, &value);
    let err = collective_err(root);
    assert!(err.contains("`maxReach`"), "{err}");
    assert!(err.contains("`maxLocality`"), "{err}");
}

#[test]
fn a_collective_with_no_steward_on_record_is_refused() {
    let dir = fixture();
    let root = dir.path();
    // No sidecar at all.
    std::fs::remove_file(root.join(memory::AFFILIATIONS_PATH)).unwrap();
    assert!(collective_err(root).contains("has no Steward on record"));
    // Only a Contributor.
    affiliate(
        root,
        "agent:investigator",
        MemberKind::ElohimAgent,
        MembershipRole::Contributor,
    );
    assert!(collective_err(root).contains("has no Steward on record"));
    // A genesis Steward, then a second Steward it sponsors.
    affiliate(
        root,
        "human:matthew",
        MemberKind::Person,
        MembershipRole::Steward,
    );
    assert!(memory::execute(root, "collective", None, None).is_ok());
    affiliate(
        root,
        "human:ruth",
        MemberKind::Person,
        MembershipRole::Steward,
    );
    let withdrawal = |sponsor: &str| {
        let mut withdrawn = affiliation(
            root,
            ROOT,
            "human:matthew",
            MemberKind::Person,
            MembershipRole::Steward,
            AffiliationStanding::Standing,
        );
        withdrawn.sponsor = Some(sponsor.into());
        withdrawn.withdrawn = Some("2026-09-26T00:00:00Z".into());
        withdrawn
    };
    // A Steward cannot sponsor its own withdrawal: the line is refused, standing stays.
    append(root, &withdrawal("human:matthew"));
    let view = memory::execute(root, "collective", None, None).unwrap();
    assert_eq!(view["stewards"].as_array().unwrap().len(), 2);
    // Withdrawn by a later line another Steward sponsors: history stays, standing ends.
    append(root, &withdrawal("human:ruth"));
    let view = memory::execute(root, "collective", None, None).unwrap();
    let stewards = view["stewards"].as_array().unwrap();
    assert_eq!(stewards.len(), 1);
    assert_eq!(stewards[0]["member"], "human:ruth");
}

#[test]
fn the_collective_view_names_each_steward_and_marks_the_fixture() {
    let dir = fixture();
    let root = dir.path();
    let view = memory::execute(root, "collective", None, None).unwrap();
    let stewards = view["stewards"].as_array().unwrap();
    assert_eq!(stewards.len(), 2);
    assert_eq!(stewards[0]["member"], "human:matthew");
    assert_eq!(stewards[0]["actsFor"], "repo:ethosengine/elohim");
    assert_eq!(stewards[0]["validatedAt"], "local (steward on record)");
    assert_eq!(stewards[1]["member"], "human:adam");
    assert_eq!(stewards[1]["standing"], "Fixture");
    assert_eq!(stewards[1]["validatedAt"], "bootstrap (fixture co-steward)");
    assert_eq!(view["affiliations"]["invalidLines"], 0);
    assert!(view["declaration"].get("steward").is_none());
}

#[test]
fn a_tampered_affiliation_line_counts_for_nothing() {
    let dir = fixture();
    let root = dir.path();
    contribute(root);
    let review = verdict(root, "contributor");
    // Promote the Contributor to Steward by rewriting its record without re-deriving its CID.
    let path = root.join(memory::AFFILIATIONS_PATH);
    let text = std::fs::read_to_string(&path).unwrap();
    let forged = text.replace(
        "\"member\":\"agent:investigator\",\"memberKind\":\"ElohimAgent\",\"role\":\"Contributor\"",
        "\"member\":\"agent:investigator\",\"memberKind\":\"ElohimAgent\",\"role\":\"Steward\"",
    );
    assert_ne!(text, forged, "the forgery must touch the Contributor line");
    std::fs::write(&path, forged).unwrap();
    let view = memory::execute(root, "collective", None, None).unwrap();
    assert_eq!(view["affiliations"]["invalidLines"], 1);
    assert!(graduate(root, &review)
        .unwrap_err()
        .contains("holds no Steward affiliation"));
}

#[test]
fn graduate_approved_by_the_authors_own_persona_is_refused() {
    let dir = fixture();
    let root = dir.path();
    contribute(root);
    let own = verdict(root, "author");
    assert!(graduate(root, &own).unwrap_err().contains("independent"));
}

#[test]
fn graduate_approved_by_a_contributor_is_refused() {
    let dir = fixture();
    let root = dir.path();
    contribute(root);
    let review = verdict(root, "contributor");
    let err = graduate(root, &review).unwrap_err();
    assert!(err.contains("holds no Steward affiliation"), "{err}");
    assert!(err.contains("Contributor"), "{err}");
}

#[test]
fn a_package_level_steward_cannot_approve_another_build_of_its_own_role() {
    let dir = fixture();
    let root = dir.path();
    // The investigator package itself is made a Steward: a later line supersedes the Contributor.
    affiliate(
        root,
        "agent:investigator",
        MemberKind::ElohimAgent,
        MembershipRole::Steward,
    );
    contribute(root);
    let review = verdict(root, "contributor");
    let err = graduate(root, &review).unwrap_err();
    assert!(err.contains("is the author"), "{err}");
}

/// A build-specific agent Steward: its session claims exactly that build.
fn build_steward(root: &Path, build: &str, session: &str) {
    affiliate(
        root,
        build,
        MemberKind::ElohimAgent,
        MembershipRole::Steward,
    );
    actor::claim(root, build, session).unwrap();
}

#[test]
fn a_build_specific_steward_cannot_approve_a_sibling_build_of_its_own_role() {
    let dir = fixture();
    let root = dir.path();
    // The author is agent:investigator@fixture; the Steward is another investigator build.
    build_steward(root, "agent:investigator@steward-build", "sibling");
    contribute(root);
    let review = verdict(root, "sibling");
    let err = graduate(root, &review).unwrap_err();
    assert!(err.contains("is the author"), "{err}");
    assert!(err.contains("agent:investigator@steward-build"), "{err}");
}

#[test]
fn a_build_specific_steward_of_a_different_role_may_approve() {
    let dir = fixture();
    let root = dir.path();
    build_steward(root, "agent:reviewer@steward-build", "other-role");
    contribute(root);
    let review = verdict(root, "other-role");
    let passed = graduate(root, &review).unwrap();
    assert_eq!(passed["allowed"], true);
    assert_eq!(passed["approver"]["member"], "agent:reviewer@steward-build");
}

#[test]
fn graduate_approved_by_a_distinct_steward_passes_and_names_its_affiliation() {
    let dir = fixture();
    let root = dir.path();
    contribute(root);
    let review = verdict(root, "matthew");
    let passed = graduate(root, &review).unwrap();
    assert_eq!(passed["allowed"], true);
    assert_eq!(passed["approver"]["member"], "human:matthew");
    assert_eq!(passed["approver"]["standing"], "Standing");
    assert!(passed["approver"]["affiliation"]
        .as_str()
        .unwrap()
        .starts_with("bafyrei"));
    assert_eq!(passed["validatedAt"], "local (steward on record)");
    assert!(!passed["standing"].as_str().unwrap().contains("FIXTURE"));
}

#[test]
fn a_fixture_co_stewards_approval_runs_the_primitive_and_says_so() {
    let dir = fixture();
    let root = dir.path();
    contribute(root);
    let review = verdict(root, "adam");
    let passed = graduate(root, &review).unwrap();
    assert_eq!(passed["allowed"], true);
    assert_eq!(passed["approver"]["member"], "human:adam");
    assert_eq!(passed["approver"]["standing"], "Fixture");
    assert_eq!(passed["validatedAt"], "bootstrap (fixture co-steward)");
    assert!(passed["standing"]
        .as_str()
        .unwrap()
        .contains("not peer validation"));
}

#[test]
fn one_real_and_one_fixture_steward_never_satisfy_a_two_steward_gate() {
    let dir = fixture();
    let root = dir.path();
    let matthew = affiliation(
        root,
        ROOT,
        "human:matthew",
        MemberKind::Person,
        MembershipRole::Steward,
        AffiliationStanding::Standing,
    );
    let adam = affiliation(
        root,
        ROOT,
        "human:adam",
        MemberKind::Person,
        MembershipRole::Steward,
        AffiliationStanding::Fixture,
    );
    assert!(requires_non_fixture_stewards(&[&matthew, &adam], 2).is_err());
    assert!(requires_non_fixture_stewards(&[&matthew, &adam], 1).is_ok());
}

/// A child collective under `genesis/concern`, pinned to the root, stewarded by human:matthew.
fn child(root: &Path) {
    write(
        root,
        CHILD,
        &json!({"version":1,"id":"collective:ethosengine/elohim/concern","displayName":"Concern",
        "charter":"A narrower collective inside the repository.","participation":"registered-local-session",
        "parent":pin(root,ROOT),
        "sourceRules":[{"path":"genesis/concern","maxLocality":"repository"}]}),
    );
    write(
        root,
        "genesis/concern/finding.json",
        &json!({"observed":"A finding that lives in the child"}),
    );
    append(
        root,
        &affiliation(
            root,
            CHILD,
            "human:matthew",
            MemberKind::Person,
            MembershipRole::Steward,
            AffiliationStanding::Standing,
        ),
    );
}

#[test]
fn a_child_collective_resolves_beneath_it_and_the_parent_resolves_for_a_sibling() {
    let dir = fixture();
    let root = dir.path();
    child(root);
    assert_eq!(
        memory::collective_of_record(root, "genesis/concern/finding.json").unwrap(),
        CHILD
    );
    assert_eq!(
        memory::collective_of_record(root, "genesis/concern/deeper/new.md").unwrap(),
        CHILD
    );
    assert_eq!(
        memory::collective_of_record(root, "genesis/concern").unwrap(),
        CHILD
    );
    assert_eq!(
        memory::collective_of_record(root, "genesis/evidence.json").unwrap(),
        ROOT
    );
    let view = memory::execute(
        root,
        "collective",
        Some("genesis/concern/finding.json"),
        None,
    )
    .unwrap();
    assert_eq!(
        view["declaration"]["id"],
        "collective:ethosengine/elohim/concern"
    );
    // A claim made under the child path binds the child.
    let bound = actor::claim_under(
        root,
        "agent:investigator@fixture",
        "bound",
        "genesis/concern/finding.json",
    )
    .unwrap();
    assert_eq!(bound.collective, CHILD);
    assert_eq!(
        actor::current(root, "bound")
            .unwrap()
            .claim
            .unwrap()
            .collective,
        CHILD
    );
}

#[test]
fn a_child_that_widens_its_parent_or_holds_a_stale_parent_pin_is_refused() {
    let dir = fixture();
    let root = dir.path();
    child(root);
    let mut value: Value =
        serde_json::from_slice(&std::fs::read(root.join(CHILD)).unwrap()).unwrap();
    // The parent keeps `genesis` at repository locality; a child rule outside its own directory
    // is refused before any widening question arises.
    value["sourceRules"] = json!([{"path":"elohim","maxLocality":"repository"}]);
    write(root, CHILD, &value);
    assert!(
        memory::collective_of_record(root, "genesis/concern/finding.json")
            .unwrap_err()
            .to_string()
            .contains("lies outside")
    );
    // Narrow the parent's rule for the child's directory, then let the child widen it.
    let mut parent = declaration();
    parent["sourceRules"]
        .as_array_mut()
        .unwrap()
        .push(json!({"path":"genesis/concern","maxLocality":"workspace"}));
    write(root, ROOT, &parent);
    value["sourceRules"] = json!([{"path":"genesis/concern","maxLocality":"repository"}]);
    value["parent"] = pin(root, ROOT);
    write(root, CHILD, &value);
    assert!(
        memory::collective_of_record(root, "genesis/concern/finding.json")
            .unwrap_err()
            .to_string()
            .contains("widens its parent")
    );
    // A parent pin taken before the parent changed is stale.
    value["sourceRules"] = json!([{"path":"genesis/concern","maxLocality":"workspace"}]);
    write(root, CHILD, &value);
    parent["charter"] = json!("Amended charter.");
    write(root, ROOT, &parent);
    assert!(
        memory::collective_of_record(root, "genesis/concern/finding.json")
            .unwrap_err()
            .to_string()
            .contains("parent pin is stale")
    );
}

#[test]
fn a_claim_bound_to_a_different_collective_than_the_sources_is_refused_naming_both() {
    let dir = fixture();
    let root = dir.path();
    child(root);
    // `author` is unbound, so bound to the root; the evidence lives in the child.
    write(
        root,
        "genesis/concern/assertion.json",
        &json!({"version":1,"collective":pin(root,CHILD),
        "author":"agent:investigator@fixture","steward":"collective:ethosengine/elohim/concern",
        "scope":"workspace","reach":"repository","concern":"child evidence",
        "claim":"The child holds its own finding","uncertainty":["Only this finding"],
        "sources":[{"resource":pin(root,"genesis/concern/finding.json"),"reach":"repository"}],"supersedes":[],"contradicts":[]}),
    );
    let err = memory::execute(
        root,
        "contribute",
        Some("genesis/concern/assertion.json"),
        Some("author"),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains(ROOT), "{err}");
    assert!(err.contains(CHILD), "{err}");
    // Re-claimed under the child, the same session contributes.
    actor::claim_under(
        root,
        "agent:investigator@fixture",
        "author",
        "genesis/concern/finding.json",
    )
    .unwrap();
    let ok = memory::execute(
        root,
        "contribute",
        Some("genesis/concern/assertion.json"),
        Some("author"),
    )
    .unwrap();
    assert_eq!(ok["collectiveOfRecord"]["path"], CHILD);
    // A source in the child cannot be filed under the root collective.
    write(
        root,
        "genesis/misfiled.json",
        &json!({"version":1,"collective":pin(root,ROOT),
        "author":"agent:investigator@fixture","steward":"repo:ethosengine/elohim",
        "scope":"workspace","reach":"repository","concern":"misfiled",
        "claim":"Filed under the wrong collective","uncertainty":["Misfiled"],
        "sources":[{"resource":pin(root,"genesis/concern/finding.json"),"reach":"repository"}],"supersedes":[],"contradicts":[]}),
    );
    let err = memory::execute(
        root,
        "contribute",
        Some("genesis/misfiled.json"),
        Some("matthew"),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("belongs to the collective at"), "{err}");
}

// ── declaration lineage: an amendment supersedes, it never orphans ─────────────────────────

/// Amend the root declaration's charter, declaring `supersedes` (newest first).
fn amend(root: &Path, supersedes: Vec<String>) -> String {
    let mut value: Value =
        serde_json::from_slice(&std::fs::read(root.join(ROOT)).unwrap()).unwrap();
    value["charter"] = json!("An amended charter.");
    value["supersedes"] = json!(supersedes);
    write(root, ROOT, &value);
    pin(root, ROOT)["cid"].as_str().unwrap().to_string()
}

#[test]
fn a_contribution_pinned_to_the_prior_declaration_can_feedback_and_graduate() {
    let dir = fixture();
    let root = dir.path();
    contribute(root);
    let prior = pin(root, ROOT)["cid"].as_str().unwrap().to_string();
    let review = verdict(root, "matthew");
    amend(root, vec![prior.clone()]);
    let expected = format!("pinned to superseded declaration {prior} (lineage ok)");

    let passed = graduate(root, &review).unwrap();
    assert_eq!(passed["allowed"], true);
    assert_eq!(passed["collectivePin"], expected.as_str());

    write(
        root,
        "genesis/feedback.json",
        &json!({"version":1,"collective":pin(root,ROOT),
        "target":pin(root,"genesis/assertion.json"),"kind":"stale-source",
        "passage":"Only the measured scope is supported","reason":"Recheck under the amended charter"}),
    );
    let feedback = memory::execute(
        root,
        "feedback",
        Some("genesis/feedback.json"),
        Some("matthew"),
    )
    .unwrap();
    assert_eq!(feedback["targetCollectivePin"], expected.as_str());

    write(
        root,
        "genesis/project-request.json",
        &json!({"version":1,"collective":pin(root,ROOT),"purpose":"Read under lineage",
        "audience":"workspace","inputs":[pin(root,"genesis/assertion.json")],"omissions":["none"]}),
    );
    let projected =
        memory::execute(root, "project", Some("genesis/project-request.json"), None).unwrap();
    assert_eq!(
        projected["receipt"]["items"][0]["collectivePin"],
        expected.as_str()
    );

    // New work is never filed under the superseded charter.
    let err = memory::execute(
        root,
        "contribute",
        Some("genesis/assertion.json"),
        Some("author"),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("superseded declaration"), "{err}");
}

#[test]
fn a_contribution_pinned_outside_the_chain_is_refused_naming_both() {
    let dir = fixture();
    let root = dir.path();
    contribute(root);
    let prior = pin(root, ROOT)["cid"].as_str().unwrap().to_string();
    let review = verdict(root, "matthew");
    // An unrelated (well-formed) CID in the chain: the contribution's pin is not in it.
    let unrelated = pin(root, "genesis/evidence.json")["cid"]
        .as_str()
        .unwrap()
        .to_string();
    let current = amend(root, vec![unrelated]);
    let err = graduate(root, &review).unwrap_err();
    assert!(err.contains(&prior), "{err}");
    assert!(err.contains(&current), "{err}");
    assert!(err.contains("supersedes chain"), "{err}");
}

#[test]
fn a_cyclic_supersedes_chain_is_refused_naming_the_cid() {
    let dir = fixture();
    let root = dir.path();
    let a = pin(root, ROOT)["cid"].as_str().unwrap().to_string();
    let b = pin(root, "genesis/evidence.json")["cid"]
        .as_str()
        .unwrap()
        .to_string();
    amend(root, vec![a.clone(), b, a.clone()]);
    let err = collective_err(root);
    assert!(err.contains("cycle"), "{err}");
    assert!(err.contains(&a), "{err}");
    // A chain naming something that is not a CID is refused too.
    amend(root, vec!["not-a-cid".into()]);
    assert!(collective_err(root).contains("is not a CID"));
}

#[test]
fn every_repository_contribution_pins_the_current_declaration_or_its_lineage() {
    let view = memory::execute(repository(), "collective", None, None).unwrap();
    let current = view["resource"]["cid"].as_str().unwrap().to_string();
    let chain: Vec<String> =
        serde_json::from_value(view["declaration"]["supersedes"].clone()).unwrap();
    let dir = repository().join(".eprfs/status/memory/contributions");
    let mut checked = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let c: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let cid = c["collective"]["cid"].as_str().unwrap();
        assert_eq!(c["collective"]["path"], ROOT, "{}", path.display());
        assert!(
            cid == current || chain.iter().any(|c| c == cid),
            "{} pins {cid}, outside the declaration's lineage",
            path.display()
        );
        checked += 1;
    }
    assert!(checked > 0);
}

// ── the repository's own seed, read-only ────────────────────────────────────────────────────

fn repository() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../.."))
}

#[test]
fn the_repository_seed_verifies_line_by_line_and_carries_no_email() {
    let text = include_str!("../../../../.eprfs/status/affiliations.jsonl");
    assert!(!text.contains('@'), "an affiliation carries an `@`");
    let rows: Vec<Affiliation> = text
        .lines()
        .map(|line| {
            memory::verify_affiliation_line(line)
                .unwrap_or_else(|| panic!("line does not verify: {line}"))
                .1
        })
        .collect();
    let find = |member: &str| rows.iter().find(|a| a.member == member).unwrap();
    let matthew = find("human:matthew");
    assert!(matthew.is_active_steward());
    assert_eq!(matthew.standing, AffiliationStanding::Standing);
    let adam = find("human:adam");
    assert!(adam.is_active_steward());
    assert_eq!(adam.standing, AffiliationStanding::Fixture);
    assert_eq!(
        rows.iter()
            .filter(|a| a.is_active_steward() && a.standing == AffiliationStanding::Standing)
            .count(),
        1,
        "human:matthew is the only real human steward"
    );
    for package in std::fs::read_dir(repository().join(".epr-meta/elohim/packages/agents")).unwrap()
    {
        let name = package.unwrap().file_name().to_string_lossy().into_owned();
        let role = name.trim_end_matches(".json");
        let agent = find(&format!("agent:{role}"));
        assert_eq!(agent.member_kind, MemberKind::ElohimAgent);
        assert_eq!(agent.role, MembershipRole::Contributor);
        assert_eq!(agent.acts_for.as_deref(), Some("human:matthew"));
    }
    assert!(rows.iter().all(|a| a.collective.path == ROOT));
    // The sponsorship chain: matthew's line is the genesis; every other line he sponsored.
    assert!(matthew.sponsor.is_none());
    assert!(rows
        .iter()
        .filter(|a| a.member != "human:matthew")
        .all(|a| a.sponsor.as_deref() == Some("human:matthew")));
}

#[test]
fn the_repository_collective_reads_with_its_stewards_and_its_registry_terms_verify() {
    let view = memory::execute(repository(), "collective", None, None).unwrap();
    assert_eq!(view["affiliations"]["invalidLines"], 0);
    let members: Vec<&str> = view["stewards"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["member"].as_str().unwrap())
        .collect();
    assert_eq!(members, ["human:matthew", "human:adam"]);
    assert_eq!(view["registry"]["verified"], true, "{}", view["registry"]);
    assert_eq!(
        view["registry"]["terms"]["constitutionalParentId"],
        "org-ethosengine"
    );
}

#[test]
fn registry_terms_outside_the_catalogs_vocabulary_are_reported() {
    let dir = fixture();
    let root = dir.path();
    for file in [
        "genesis/data/collectives/collectives.json",
        "genesis/data/collectives/collectives.schema.json",
    ] {
        let target = root.join(file);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::copy(repository().join(file), target).unwrap();
    }
    assert_eq!(
        memory::execute(root, "collective", None, None).unwrap()["registry"]["verified"],
        true
    );
    let mut value = declaration();
    value["registry"]["constitutionalParentId"] = json!("org-nowhere");
    value["registry"]["reach"] = json!("galactic");
    value["registry"]["governanceLayer"] = json!("empire");
    write(root, ROOT, &value);
    let report = memory::execute(root, "collective", None, None).unwrap()["registry"].clone();
    assert_eq!(report["verified"], false);
    let problems = report["problems"].to_string();
    assert!(problems.contains("org-nowhere"), "{problems}");
    assert!(problems.contains("galactic"), "{problems}");
    assert!(problems.contains("empire"), "{problems}");
}
