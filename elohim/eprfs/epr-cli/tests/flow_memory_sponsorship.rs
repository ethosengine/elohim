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
fn a_withdrawal_needs_a_sponsor_who_is_not_the_leaver() {
    let dir = fixture();
    let root = dir.path();
    steward(root, "human:ruth", Some("human:matthew"));
    withdraw(root, "human:ruth", None);
    withdraw(root, "human:ruth", Some("human:ruth"));
    assert_eq!(refusals(root).len(), 2);
    assert!(stewards(root).contains(&"human:ruth".to_string()));
    withdraw(root, "human:ruth", Some("human:matthew"));
    assert!(!stewards(root).contains(&"human:ruth".to_string()));
    // A rejoin is sponsored like any line.
    steward(root, "human:ruth", Some("human:matthew"));
    assert!(stewards(root).contains(&"human:ruth".to_string()));
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
