//! The sponsorship chain: membership is the first place anti-self-election holds.
//!
//! The first Steward line of a collective is its genesis; every later line names a sponsor who,
//! at that point in the fold (file order), is an ACTIVE Steward and is not the line's member; a
//! fixture Steward may sponsor only a Fixture or Contributor line. `epr flow memory affiliate` is
//! the in-band act, sponsored by the session's claimed participant, signed when this device is
//! enrolled for the sponsor. Every root is a committed tempdir; every key is a temp file.
use std::path::{Path, PathBuf};
use std::process::Command;

use elohim_epr_cli::{
    actor,
    device_key::{self, DeviceKey},
    flow::memory::{self, Options},
};
use eprfs_agent::memory::{Affiliation, AffiliationStanding, FileRef, MemberKind, MembershipRole};
use serde_json::{json, Value};
use tempfile::TempDir;

const ROOT: &str = ".epr-meta/collective.json";

fn write(root: &Path, path: &str, value: &Value) {
    let file = root.join(path);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}
fn git(root: &Path, args: &[&str]) {
    let out = elohim_epr_cli::process::build_command("git", args, root, &[])
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.test")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.test")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// One line's record: `sponsor` names who sponsored it (`None` only for a genesis).
fn line(
    root: &Path,
    member: &str,
    kind: MemberKind,
    role: MembershipRole,
    standing: AffiliationStanding,
    sponsor: Option<&str>,
) -> Affiliation {
    let bytes = std::fs::read(root.join(ROOT)).unwrap();
    Affiliation {
        version: 1,
        collective: FileRef {
            path: ROOT.into(),
            cid: eprfs_core::BlobCid::compute_raw(&bytes).to_string(),
        },
        member: member.into(),
        member_kind: kind,
        role,
        sponsor: sponsor.map(str::to_string),
        acts_for: None,
        standing,
        since: "2026-09-25T00:00:00Z".into(),
        withdrawn: None,
    }
}
fn append_raw(root: &Path, text: &str) {
    use std::io::Write;
    let path = root.join(memory::AFFILIATIONS_PATH);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap();
    writeln!(file, "{text}").unwrap();
}
fn append(root: &Path, record: &Affiliation) {
    append_raw(root, &memory::affiliation_line(record).unwrap());
}
fn person(root: &Path, member: &str, role: MembershipRole, sponsor: Option<&str>) -> Affiliation {
    line(
        root,
        member,
        MemberKind::Person,
        role,
        AffiliationStanding::Standing,
        sponsor,
    )
}
fn steward(root: &Path, member: &str, sponsor: Option<&str>) {
    append(
        root,
        &person(root, member, MembershipRole::Steward, sponsor),
    );
}
fn withdraw(root: &Path, member: &str, sponsor: Option<&str>) {
    let mut record = person(root, member, MembershipRole::Steward, sponsor);
    record.withdrawn = Some("2026-09-26T00:00:00Z".into());
    append(root, &record);
}

/// A root collective with human:matthew as its genesis Steward, human:adam a fixture Steward
/// sponsored by matthew, and the investigator package a Contributor sponsored by matthew.
fn fixture() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        ROOT,
        &serde_json::from_str(include_str!("../../../../.epr-meta/collective.json")).unwrap(),
    );
    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "fixture"]);
    steward(root, "human:matthew", None);
    append(
        root,
        &line(
            root,
            "human:adam",
            MemberKind::Person,
            MembershipRole::Steward,
            AffiliationStanding::Fixture,
            Some("human:matthew"),
        ),
    );
    append(
        root,
        &line(
            root,
            "agent:investigator",
            MemberKind::ElohimAgent,
            MembershipRole::Contributor,
            AffiliationStanding::Standing,
            Some("human:matthew"),
        ),
    );
    actor::claim(root, "human:matthew", "matthew").unwrap();
    actor::claim(root, "human:adam", "adam").unwrap();
    actor::claim(root, "agent:investigator@fixture", "contributor").unwrap();
    dir
}

fn view(root: &Path) -> Value {
    memory::execute(root, "collective", None, None).unwrap()
}
fn stewards(root: &Path) -> Vec<String> {
    view(root)["stewards"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["member"].as_str().unwrap().to_string())
        .collect()
}
/// The refusal reasons the fold named, one per refused line.
fn refusals(root: &Path) -> Vec<String> {
    view(root)["affiliations"]["refused"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["reason"].as_str().unwrap().to_string())
        .collect()
}
fn lines(root: &Path) -> usize {
    std::fs::read_to_string(root.join(memory::AFFILIATIONS_PATH))
        .unwrap()
        .lines()
        .count()
}

// ── the fold ────────────────────────────────────────────────────────────────────────────────

#[test]
fn the_genesis_steward_needs_no_sponsor_and_every_later_steward_names_its_chain() {
    let dir = fixture();
    let root = dir.path();
    let view = view(root);
    assert_eq!(view["affiliations"]["invalidLines"], 0, "{view}");
    let stewards = view["stewards"].as_array().unwrap();
    assert_eq!(stewards[0]["member"], "human:matthew");
    assert!(stewards[0]["sponsor"].is_null());
    assert_eq!(stewards[0]["sponsorChain"], json!([]));
    assert_eq!(stewards[0]["signature"]["status"], "unsigned");
    assert_eq!(stewards[1]["member"], "human:adam");
    assert_eq!(stewards[1]["sponsor"], "human:matthew");
    let chain = stewards[1]["sponsorChain"].as_array().unwrap();
    assert_eq!(chain.len(), 1);
    assert_eq!(chain[0]["member"], "human:matthew");
    assert_eq!(chain[0]["genesis"], true);
    assert_eq!(chain[0]["affiliation"], stewards[0]["affiliation"]);
}

#[test]
fn a_steward_line_with_no_sponsor_after_genesis_is_refused_and_does_not_stand() {
    let dir = fixture();
    let root = dir.path();
    steward(root, "human:ruth", None);
    assert_eq!(stewards(root), ["human:matthew", "human:adam"]);
    let refused = refusals(root);
    assert_eq!(refused.len(), 1, "{refused:?}");
    assert!(refused[0].contains("names no sponsor"), "{refused:?}");
    let named = &view(root)["affiliations"]["refused"][0];
    assert_eq!(named["member"], "human:ruth");
    assert_eq!(named["line"], 4);
    // A line ahead of any genesis is refused too: nobody could have sponsored it.
    let fresh = fixture();
    let root = fresh.path();
    std::fs::remove_file(root.join(memory::AFFILIATIONS_PATH)).unwrap();
    append(
        root,
        &person(root, "human:ruth", MembershipRole::Contributor, None),
    );
    steward(root, "human:matthew", None);
    assert_eq!(stewards(root), ["human:matthew"]);
    assert!(refusals(root)[0].contains("precedes the collective's genesis"));
}

#[test]
fn self_sponsorship_is_refused_including_a_sibling_build_of_the_same_agent_role() {
    let dir = fixture();
    let root = dir.path();
    // matthew re-affiliates himself (a role change) on his own sponsorship.
    append(
        root,
        &person(
            root,
            "human:matthew",
            MembershipRole::Contributor,
            Some("human:matthew"),
        ),
    );
    assert!(
        refusals(root)[0].contains("self-sponsorship"),
        "{:?}",
        refusals(root)
    );
    assert_eq!(stewards(root)[0], "human:matthew");
    // An agent build made Steward by matthew cannot sponsor a sibling build of its own role.
    let agent = |member: &str, sponsor: &str| {
        line(
            root,
            member,
            MemberKind::ElohimAgent,
            MembershipRole::Steward,
            AffiliationStanding::Standing,
            Some(sponsor),
        )
    };
    append(root, &agent("agent:reviewer@opus-5", "human:matthew"));
    append(
        root,
        &agent("agent:reviewer@sonnet-5", "agent:reviewer@opus-5"),
    );
    append(root, &agent("agent:reviewer", "agent:reviewer@opus-5"));
    let refused = refusals(root);
    assert_eq!(refused.len(), 3, "{refused:?}");
    assert!(
        refused[1..].iter().all(|r| r.contains("self-sponsorship")),
        "{refused:?}"
    );
    // …while a build of another role, sponsored by it, stands.
    append(
        root,
        &agent("agent:rust-architect@opus-5", "agent:reviewer@opus-5"),
    );
    assert!(stewards(root).contains(&"agent:rust-architect@opus-5".to_string()));
}

#[test]
fn a_contributor_or_a_withdrawn_steward_cannot_sponsor() {
    let dir = fixture();
    let root = dir.path();
    // The investigator package is a Contributor: its builds sponsor nothing.
    steward(root, "human:ruth", Some("agent:investigator@fixture"));
    assert!(refusals(root)[0].contains("not an active Steward"));
    assert!(refusals(root)[0].contains("Contributor"));
    // ruth stands (sponsored by matthew), is withdrawn by matthew, then sponsors nobody.
    steward(root, "human:ruth", Some("human:matthew"));
    withdraw(root, "human:ruth", Some("human:matthew"));
    append(
        root,
        &person(
            root,
            "human:carol",
            MembershipRole::Contributor,
            Some("human:ruth"),
        ),
    );
    let refused = refusals(root);
    assert_eq!(refused.len(), 2, "{refused:?}");
    assert!(refused[1].contains("withdrawn"), "{refused:?}");
    assert_eq!(stewards(root), ["human:matthew", "human:adam"]);
}

#[test]
fn a_fixture_steward_sponsors_contributors_and_fixtures_never_standing_stewards() {
    let dir = fixture();
    let root = dir.path();
    steward(root, "human:ruth", Some("human:adam"));
    let refused = refusals(root);
    assert!(
        refused[0].contains("fixture Steward human:adam"),
        "{refused:?}"
    );
    append(
        root,
        &person(
            root,
            "human:carol",
            MembershipRole::Contributor,
            Some("human:adam"),
        ),
    );
    append(
        root,
        &line(
            root,
            "human:dan",
            MemberKind::Person,
            MembershipRole::Steward,
            AffiliationStanding::Fixture,
            Some("human:adam"),
        ),
    );
    // Nor may a fixture withdraw real authority.
    withdraw(root, "human:matthew", Some("human:adam"));
    let view = view(root);
    assert_eq!(view["affiliations"]["invalidLines"], 2, "{view}");
    assert_eq!(stewards(root), ["human:matthew", "human:adam", "human:dan"]);
    assert_eq!(view["affiliations"]["current"], 5);
}

#[test]
fn a_withdrawal_names_a_sponsor_and_leaving_yourself_is_never_gated() {
    let dir = fixture();
    let root = dir.path();
    steward(root, "human:ruth", Some("human:matthew"));
    withdraw(root, "human:ruth", None);
    assert_eq!(refusals(root).len(), 1);
    assert!(stewards(root).contains(&"human:ruth".to_string()));
    // Self-withdrawal passes: the line names its own member as sponsor.
    withdraw(root, "human:ruth", Some("human:ruth"));
    assert!(!stewards(root).contains(&"human:ruth".to_string()));
    let admitted = &view(root)["affiliations"];
    assert_eq!(admitted["invalidLines"], 1, "{admitted}");
    // A rejoin is sponsored like any line…
    steward(root, "human:ruth", Some("human:matthew"));
    assert!(stewards(root).contains(&"human:ruth".to_string()));
    // …and a self-join never stands: only leaving is ungated.
    append(
        root,
        &person(
            root,
            "human:carol",
            MembershipRole::Contributor,
            Some("human:carol"),
        ),
    );
    let refused = refusals(root);
    assert!(
        refused.last().unwrap().contains("self-sponsorship"),
        "{refused:?}"
    );
    assert!(
        refused.last().unwrap().contains("only leaving is ungated"),
        "{refused:?}"
    );
}

// ── stewardless ─────────────────────────────────────────────────────────────────────────────

/// Both Stewards of the fixture leave themselves: the collective reads stewardless.
fn stewardless(root: &Path) {
    let mut adam = line(
        root,
        "human:adam",
        MemberKind::Person,
        MembershipRole::Steward,
        AffiliationStanding::Fixture,
        Some("human:adam"),
    );
    adam.withdrawn = Some("2026-09-26T00:00:00Z".into());
    append(root, &adam);
    withdraw(root, "human:matthew", Some("human:matthew"));
}

#[test]
fn the_last_steward_may_leave_and_only_steward_acts_refuse_naming_stewardless() {
    let dir = fixture();
    let root = dir.path();
    write(
        root,
        "genesis/evidence.json",
        &json!({"observed":"Only narrow claims are supported"}),
    );
    stewardless(root);
    let view = view(root);
    assert_eq!(view["stewardship"], "stewardless", "{view}");
    assert_eq!(view["stewards"], json!([]));
    assert_eq!(view["affiliations"]["invalidLines"], 0);
    // Value still flows: a contribution files.
    let pin =
        |path: &str| memory::execute(root, "pin", Some(path), None).unwrap()["resource"].clone();
    write(
        root,
        "genesis/assertion.json",
        &json!({"version":1,"collective":pin(ROOT),
        "author":"agent:investigator@fixture","steward":"repo:ethosengine/elohim",
        "scope":"workspace","reach":"repository","concern":"stewardless evidence",
        "claim":"Only the measured scope is supported","uncertainty":["Wider use remains untested"],
        "sources":[{"resource":pin("genesis/evidence.json"),"reach":"repository"}],"supersedes":[],"contradicts":[]}),
    );
    memory::execute(
        root,
        "contribute",
        Some("genesis/assertion.json"),
        Some("contributor"),
    )
    .unwrap();
    // Graduation needs a Steward's verdict: refused, naming the state.
    write(
        root,
        "genesis/graduation.json",
        &json!({"version":1,"collective":pin(ROOT),"contribution":pin("genesis/assertion.json"),
        "review":"bafyreigh2akiscaildc4ecb5fxd4osdmnjoxzmvhrrkqbzsicgqj3bnzfy","audience":"repository"}),
    );
    let err = memory::execute(root, "graduate", Some("genesis/graduation.json"), None)
        .unwrap_err()
        .to_string();
    assert!(err.contains("stewardless"), "{err}");
    // Sponsorship refuses, naming the state — for a former Steward and a Contributor alike.
    for session in ["matthew", "contributor"] {
        let err = affiliate(root, session, "human:ruth", "contributor", |_| {}).unwrap_err();
        assert!(err.contains("stewardless"), "{err}");
    }
    // Leaving still works while stewardless: the Contributor withdraws itself through the verb.
    let out = affiliate(
        root,
        "contributor",
        "agent:investigator",
        "contributor",
        |o| {
            o.kind = Some("elohim-agent");
            o.withdraw = true;
        },
    )
    .unwrap();
    assert_eq!(out["admittedAs"], "self-withdrawal");
    assert_eq!(out["appended"]["record"]["sponsor"], "agent:investigator");
}

#[test]
fn a_root_collective_is_refounded_only_through_its_declaration() {
    let dir = fixture();
    let root = dir.path();
    stewardless(root);
    let before = memory::execute(root, "pin", Some(ROOT), None).unwrap()["resource"]["cid"]
        .as_str()
        .unwrap()
        .to_string();
    // A new genesis pinned to the same declaration is a raw edit: refused.
    steward(root, "human:matthew", None);
    let refused = refusals(root);
    assert!(
        refused[0].contains("re-founded only through its declaration"),
        "{refused:?}"
    );
    assert_eq!(view(root)["stewardship"], "stewardless");
    // A reviewed amendment whose lineage names the stewardless declaration re-founds it.
    let mut declaration: Value =
        serde_json::from_slice(&std::fs::read(root.join(ROOT)).unwrap()).unwrap();
    declaration["charter"] = json!("Re-founded after the last Steward left.");
    declaration["supersedes"] = json!([before]);
    write(root, ROOT, &declaration);
    steward(root, "human:matthew", None);
    let view = view(root);
    assert_eq!(view["stewardship"], "stewarded", "{view}");
    assert_eq!(view["stewards"][0]["member"], "human:matthew");
    // The earlier refused line stays refused; the re-founding line stands.
    assert_eq!(view["affiliations"]["invalidLines"], 1);
    // …and the sponsor chain of a line it sponsors reads the re-founding.
    steward(root, "human:ruth", Some("human:matthew"));
    let chain = &view_of(root, "human:ruth")["sponsorChain"][0];
    assert_eq!(chain["admittedAs"], "refounded-by-declaration");
}

fn view_of(root: &Path, member: &str) -> Value {
    view(root)["stewards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["member"] == member)
        .unwrap()
        .clone()
}

const CHILD: &str = "genesis/concern/.epr-meta/collective.json";

#[test]
fn a_stewardless_child_is_refounded_by_a_steward_of_its_parent() {
    let dir = fixture();
    let root = dir.path();
    let parent = memory::execute(root, "pin", Some(ROOT), None).unwrap()["resource"].clone();
    write(
        root,
        CHILD,
        &json!({"version":1,"id":"collective:ethosengine/elohim/concern","displayName":"Concern",
        "charter":"A narrower collective inside the repository.","participation":"registered-local-session",
        "parent":parent,"sourceRules":[{"path":"genesis/concern","maxLocality":"repository"}]}),
    );
    let child_line = |member: &str, sponsor: Option<&str>, withdrawn: bool| {
        let bytes = std::fs::read(root.join(CHILD)).unwrap();
        let mut record = person(root, member, MembershipRole::Steward, sponsor);
        record.collective = FileRef {
            path: CHILD.into(),
            cid: eprfs_core::BlobCid::compute_raw(&bytes).to_string(),
        };
        record.withdrawn = withdrawn.then(|| "2026-09-26T00:00:00Z".to_string());
        record
    };
    let child_view = || memory::execute(root, "collective", Some("genesis/concern"), None).unwrap();
    append(root, &child_line("human:ruth", None, false));
    append(root, &child_line("human:ruth", Some("human:ruth"), true));
    assert_eq!(child_view()["stewardship"], "stewardless");
    // Nobody outside the parent's Stewards re-founds it — nor a sponsorless line.
    append(root, &child_line("human:carol", Some("human:dan"), false));
    append(root, &child_line("human:carol", None, false));
    assert_eq!(child_view()["stewardship"], "stewardless");
    // A Steward of the parent (human:matthew) sponsors the new genesis.
    append(
        root,
        &child_line("human:carol", Some("human:matthew"), false),
    );
    let view = child_view();
    assert_eq!(view["stewardship"], "stewarded", "{view}");
    assert_eq!(view["stewards"][0]["member"], "human:carol");
    assert_eq!(view["affiliations"]["invalidLines"], 2);
}

#[test]
fn a_fixture_genesis_is_refused() {
    let dir = fixture();
    let root = dir.path();
    std::fs::remove_file(root.join(memory::AFFILIATIONS_PATH)).unwrap();
    append(
        root,
        &line(
            root,
            "human:adam",
            MemberKind::Person,
            MembershipRole::Steward,
            AffiliationStanding::Fixture,
            None,
        ),
    );
    let err = memory::execute(root, "collective", None, None)
        .unwrap_err()
        .to_string();
    assert!(err.contains("never founded"), "{err}");
    steward(root, "human:matthew", None);
    let refused = refusals(root);
    assert!(
        refused[0].contains("a fixture never mints real authority"),
        "{refused:?}"
    );
    assert_eq!(stewards(root), ["human:matthew"]);
}

// ── the verb ────────────────────────────────────────────────────────────────────────────────

fn affiliate<'a>(
    root: &Path,
    session: &'a str,
    member: &'a str,
    role: &'a str,
    extra: impl FnOnce(&mut Options<'a>),
) -> Result<Value, String> {
    let mut opts = Options {
        session: Some(session),
        member: Some(member),
        kind: Some("person"),
        role: Some(role),
        ..Options::default()
    };
    extra(&mut opts);
    memory::execute_with(root, "affiliate", &opts).map_err(|e| e.to_string())
}

#[test]
fn the_verb_refuses_a_session_that_is_not_an_active_steward_and_appends_nothing() {
    let dir = fixture();
    let root = dir.path();
    let before = lines(root);
    let err = affiliate(root, "contributor", "human:ruth", "contributor", |_| {}).unwrap_err();
    assert!(err.contains("not an active Steward"), "{err}");
    let err = affiliate(root, "nobody", "human:ruth", "contributor", |_| {}).unwrap_err();
    assert!(err.contains("has no actor claim"), "{err}");
    // A fixture session may not mint a standing Steward.
    let err = affiliate(root, "adam", "human:ruth", "steward", |_| {}).unwrap_err();
    assert!(err.contains("fixture Steward"), "{err}");
    // Nor may a Steward sponsor itself.
    let err = affiliate(root, "matthew", "human:matthew", "contributor", |_| {}).unwrap_err();
    assert!(err.contains("self-sponsorship"), "{err}");
    assert_eq!(lines(root), before);
}

#[test]
fn the_verb_appends_exactly_one_unsigned_line_that_the_fold_admits() {
    let dir = fixture();
    let root = dir.path();
    let before = lines(root);
    let out = affiliate(root, "matthew", "human:ruth", "steward", |_| {}).unwrap();
    assert_eq!(lines(root), before + 1);
    assert_eq!(out["sponsor"], "human:matthew");
    assert_eq!(out["signature"]["status"], "unsigned");
    assert_eq!(out["appended"]["record"]["sponsor"], "human:matthew");
    let text = std::fs::read_to_string(root.join(memory::AFFILIATIONS_PATH)).unwrap();
    assert!(text
        .lines()
        .last()
        .unwrap()
        .contains("\"signature\":\"unsigned\""));
    assert!(stewards(root).contains(&"human:ruth".to_string()));
    // A fixture session sponsors a Contributor; a repeat of the same affiliation is a no-op refusal.
    affiliate(root, "adam", "human:carol", "contributor", |_| {}).unwrap();
    let err = affiliate(root, "adam", "human:carol", "contributor", |_| {}).unwrap_err();
    assert!(err.contains("nothing to append"), "{err}");
    // A withdrawal is one sponsored line too.
    affiliate(root, "matthew", "human:ruth", "steward", |o| {
        o.withdraw = true
    })
    .unwrap();
    assert!(!stewards(root).contains(&"human:ruth".to_string()));
    assert_eq!(lines(root), before + 3);
    assert_eq!(view(root)["affiliations"]["invalidLines"], 0);
}

// ── signatures ──────────────────────────────────────────────────────────────────────────────

fn device(keys: &TempDir, name: &str) -> (PathBuf, DeviceKey) {
    let path = keys.path().join(name).join("ed25519.seed");
    let key = DeviceKey::load_or_generate(&path).unwrap();
    (path, key)
}
/// Enrol `key` for human:matthew: a present agent witnesses him from this device (roster genesis).
fn enrol_matthew(root: &Path, key: &DeviceKey) {
    actor::witness(
        root,
        "human:matthew",
        "agent:orchestrator@fixture",
        "witness",
        "operator of this fixture device",
        false,
        key,
    )
    .unwrap();
}

#[test]
fn an_enrolled_sponsor_device_signs_and_a_tampered_signature_is_refused() {
    let dir = fixture();
    let root = dir.path();
    let keys = TempDir::new().unwrap();
    let (_, key) = device(&keys, "matthew");
    enrol_matthew(root, &key);
    let out = affiliate(root, "matthew", "human:ruth", "steward", |o| {
        o.device = Some(&key)
    })
    .unwrap();
    assert_eq!(out["signature"]["status"], "signed");
    assert_eq!(out["signature"]["signer"], key.did_key());
    let view = view(root);
    let ruth = view["stewards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["member"] == "human:ruth")
        .unwrap()
        .clone();
    assert_eq!(ruth["signature"]["status"], "signed");
    assert_eq!(ruth["sponsorChain"][0]["member"], "human:matthew");

    // Flip one hex digit of the signature: the line counts for nothing, ruth does not stand.
    let path = root.join(memory::AFFILIATIONS_PATH);
    let text = std::fs::read_to_string(&path).unwrap();
    let last = text.lines().last().unwrap().to_string();
    let mut value: Value = serde_json::from_str(&last).unwrap();
    let sig = value["signature"]["signature"]
        .as_str()
        .unwrap()
        .to_string();
    let flipped = format!(
        "{}{}",
        if sig.starts_with('0') { '1' } else { '0' },
        &sig[1..]
    );
    value["signature"]["signature"] = json!(flipped);
    std::fs::write(&path, text.replace(&last, &value.to_string())).unwrap();
    assert!(
        refusals(root)[0].contains("does not verify"),
        "{:?}",
        refusals(root)
    );
    assert!(!stewards(root).contains(&"human:ruth".to_string()));
}

#[test]
fn a_device_not_enrolled_for_the_sponsor_leaves_the_verb_unsigned_and_a_forged_line_refused() {
    let dir = fixture();
    let root = dir.path();
    let keys = TempDir::new().unwrap();
    let (_, stranger) = device(&keys, "stranger");
    // The verb never signs with a device the sponsor has not enrolled.
    let out = affiliate(root, "matthew", "human:ruth", "steward", |o| {
        o.device = Some(&stranger)
    })
    .unwrap();
    assert_eq!(out["signature"]["status"], "unsigned");
    // A line signed by that device (valid signature, wrong device) is refused on read.
    let record = person(
        root,
        "human:carol",
        MembershipRole::Contributor,
        Some("human:matthew"),
    );
    append_raw(
        root,
        &memory::affiliation_line_signed(&record, Some(&stranger)).unwrap(),
    );
    assert!(
        refusals(root)[0].contains("not enrolled"),
        "{:?}",
        refusals(root)
    );
}

#[test]
fn the_cli_verb_signs_with_the_enrolled_device_named_on_the_child() {
    let dir = fixture();
    let root = dir.path();
    let keys = TempDir::new().unwrap();
    let (key_file, key) = device(&keys, "matthew");
    enrol_matthew(root, &key);
    let out = Command::new(env!("CARGO_BIN_EXE_epr"))
        .args([
            "flow",
            "memory",
            "affiliate",
            "--member",
            "human:ruth",
            "--kind",
            "person",
            "--role",
            "contributor",
            "--session",
            "matthew",
            "--json",
            "--root",
        ])
        .arg(root)
        .env(device_key::DEVICE_KEY_ENV, &key_file)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["signature"]["status"], "signed");
    assert_eq!(value["signature"]["signer"], key.did_key());
}

// ── the repository's own seed ───────────────────────────────────────────────────────────────

#[test]
fn the_repository_seed_folds_with_no_refusal_and_every_later_line_is_sponsored_by_matthew() {
    let repository = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../.."));
    let view = memory::execute(repository, "collective", None, None).unwrap();
    assert_eq!(
        view["affiliations"]["invalidLines"], 0,
        "{}",
        view["affiliations"]
    );
    assert_eq!(view["affiliations"]["refused"], json!([]));
    let text = include_str!("../../../../.eprfs/status/affiliations.jsonl");
    let parsed: Vec<_> = text
        .lines()
        .map(|l| memory::parse_affiliation_line(l).unwrap())
        .collect();
    assert_eq!(parsed[0].record.member, "human:matthew");
    assert!(
        parsed[0].record.sponsor.is_none(),
        "matthew's line is the genesis"
    );
    assert!(parsed[1..]
        .iter()
        .all(|p| p.record.sponsor.as_deref() == Some("human:matthew")));
    assert!(parsed
        .iter()
        .all(|p| p.signature == memory::LineSignature::Unsigned));
    assert_eq!(
        view["stewards"][1]["sponsorChain"][0]["member"],
        "human:matthew"
    );
}
