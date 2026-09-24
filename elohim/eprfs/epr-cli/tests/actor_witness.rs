//! Integration tests for the participant plane's commands — `epr actor witness | contest | device`
//! and the standing-human read `current` and `flow note` make from them.
//!
//! Every key here is a temp file: library calls take the `DeviceKey` explicitly, and the tests that
//! drive the `epr` binary set `ELOHIM_DEVICE_KEY_FILE` on the CHILD only, so no test ever touches
//! this device's real key in `${XDG_CONFIG_HOME:-$HOME/.config}/elohim/device/`, and no test mutates
//! the process environment other tests share. Every root is a committed tempdir repo, so claims and
//! witnesses have a real HEAD to be dated against — never the real sidecar.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use elohim_epr_cli::actor::{
    self, claim_with_device, contest, device_authorize, device_bind, device_enroll, witness,
    ActorError, RosterState,
};
use elohim_epr_cli::device_key::{self, DeviceKey};
use elohim_epr_rea::{
    standing_human, ActorRecord, ActorStore, AgentRef, ParticipantRow, SidecarActorStore,
    SidecarRoster,
};
use tempfile::TempDir;

const SUBJECT: &str = "human:matthew";
const WITNESS: &str = "agent:orchestrator@claude-fable-5-1";
const SESSION: &str = "witness-session";
const BASIS: &str = "operator of this stewarded device; present in this session";

fn git(root: &Path, args: &[&str]) {
    let out = elohim_epr_cli::process::build_command("git", args, root, &[])
        .env("GIT_AUTHOR_NAME", "Fixture Author")
        .env("GIT_AUTHOR_EMAIL", "author@example.test")
        .env("GIT_COMMITTER_NAME", "Fixture Author")
        .env("GIT_COMMITTER_EMAIL", "author@example.test")
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?} failed");
}

/// A committed tempdir repo — a HEAD to date records against, and nothing else.
fn fixture() -> TempDir {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("README.md"), "fixture\n").unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["add", "-A"]);
    git(dir.path(), &["commit", "-q", "-m", "fixture"]);
    dir
}

/// A device key in its own temp directory (never the config-home key).
fn device(keys: &TempDir, name: &str) -> (PathBuf, DeviceKey) {
    let path = keys.path().join(name).join("ed25519.seed");
    let key = DeviceKey::load_or_generate(&path).unwrap();
    (path, key)
}

fn records(root: &Path) -> Vec<ActorRecord> {
    SidecarActorStore::open(root)
        .unwrap()
        .records()
        .unwrap()
        .into_iter()
        .map(|(_, r)| r)
        .collect()
}

fn roster_rows(root: &Path, handle: &str) -> Vec<ParticipantRow> {
    SidecarRoster::open(root, handle)
        .unwrap()
        .read()
        .unwrap()
        .rows()
        .iter()
        .map(|(_, r)| r.clone())
        .collect()
}

fn standing_on(root: &Path, key: &DeviceKey) -> Option<AgentRef> {
    let roster = SidecarRoster::open(root, "matthew")
        .unwrap()
        .read()
        .unwrap();
    standing_human(&roster, &records(root), &key.did_key(), &device_key::verify)
}

/// Run the built `epr` with the device key named on the child alone; the harness's own session
/// variables are removed so the child resolves only what the test hands it.
fn epr(root: &Path, key_file: &Path, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_epr"));
    command
        .args(args)
        .arg("--root")
        .arg(root)
        .env(device_key::DEVICE_KEY_ENV, key_file)
        .env_remove("CLAUDE_CODE_SESSION_ID")
        .env_remove("CLAUDE_SESSION_ID")
        .env_remove("ELOHIM_SESSION_ID");
    command.output().expect("epr runs")
}

// ── witness ──────────────────────────────────────────────────────────────────────────────────

#[test]
fn witness_appends_witness_signed_and_genesis() {
    let dir = fixture();
    let keys = TempDir::new().unwrap();
    let root = dir.path();
    let (_, key) = device(&keys, "a");

    let outcome = witness(root, SUBJECT, WITNESS, SESSION, BASIS, false, &key).expect("witnesses");

    assert_eq!(outcome.subject, SUBJECT);
    assert_eq!(outcome.witness, WITNESS);
    assert_eq!(outcome.device, key.did_key());
    assert_eq!(outcome.roster, RosterState::Genesis);
    assert_eq!(
        outcome.roster_path,
        ".eprfs/status/participants/matthew.jsonl"
    );

    // The sidecar: the witness, then the device's signature over it.
    let recs = records(root);
    assert_eq!(recs.len(), 2, "witness + signed: {recs:?}");
    let ActorRecord::Witness(w) = &recs[0] else {
        panic!("first record is the witness: {recs:?}")
    };
    assert_eq!(w.subject.0, SUBJECT);
    assert_eq!(w.witness.0, WITNESS);
    assert_eq!(w.basis, BASIS);
    assert_eq!(recs[0].cid().unwrap().to_string(), outcome.record_cid);
    let ActorRecord::Signed(sig) = &recs[1] else {
        panic!("second record is the signature: {recs:?}")
    };
    assert_eq!(sig.claim_cid, outcome.record_cid);
    assert_eq!(sig.signer, key.did_key());
    assert!(
        sig.verify(&device_key::verify),
        "the device's own signature verifies"
    );

    // The roster: one genesis row, chain root = this device, signed by it.
    let rows = roster_rows(root, "matthew");
    assert_eq!(rows.len(), 1);
    let ParticipantRow::Genesis {
        chain_root,
        record_cid,
        signer,
        ..
    } = &rows[0]
    else {
        panic!("genesis row expected: {rows:?}")
    };
    assert_eq!(chain_root, &key.did_key());
    assert_eq!(signer, &key.did_key());
    assert_eq!(record_cid, &outcome.record_cid);

    // And it stands: this device now speaks for the handle.
    assert_eq!(standing_on(root, &key), Some(AgentRef(SUBJECT.into())));

    // Public material only in the tracked roster: no role@model, no email.
    let text = std::fs::read_to_string(root.join(&outcome.roster_path)).unwrap();
    assert!(!text.contains('@'), "{text}");
}

#[test]
fn witness_refuses_human_as_and_empty_basis() {
    let dir = fixture();
    let keys = TempDir::new().unwrap();
    let root = dir.path();
    let (_, key) = device(&keys, "a");

    let err = witness(root, SUBJECT, "human:matthew", SESSION, BASIS, false, &key)
        .expect_err("a human's own act is a claim, not a witness");
    assert!(err.to_string().contains("not an agent"), "got: {err}");

    for basis in ["", "   "] {
        let err = witness(root, SUBJECT, WITNESS, SESSION, basis, false, &key)
            .expect_err("a witness names what it knows");
        assert!(err.to_string().contains("basis"), "got: {err}");
    }

    let err = witness(
        root,
        "agent:scribe@opus-5",
        WITNESS,
        SESSION,
        BASIS,
        false,
        &key,
    )
    .expect_err("only a human is witnessed");
    assert!(err.to_string().contains("not a human"), "got: {err}");

    assert!(
        !root.join(".eprfs").exists(),
        "a refused witness opens neither the sidecar nor a roster"
    );
}

#[test]
fn witness_twice_on_one_device_needs_again() {
    let dir = fixture();
    let keys = TempDir::new().unwrap();
    let root = dir.path();
    let (_, key) = device(&keys, "a");

    let first = witness(root, SUBJECT, WITNESS, SESSION, BASIS, false, &key).unwrap();
    let err = witness(root, SUBJECT, WITNESS, "second-session", BASIS, false, &key)
        .expect_err("once per device");
    assert!(
        matches!(err, ActorError::InvalidArguments(_)),
        "got: {err:?}"
    );
    assert!(
        err.to_string().contains("--again"),
        "the refusal names the way through: {err}"
    );
    assert_eq!(records(root).len(), 2, "the refusal appended nothing");

    let again = witness(root, SUBJECT, WITNESS, "second-session", BASIS, true, &key)
        .expect("--again re-witnesses");
    assert_ne!(
        again.record_cid, first.record_cid,
        "a re-witness is a new record"
    );
    assert_eq!(
        again.roster,
        RosterState::Joined,
        "the roster already has its genesis"
    );
    assert_eq!(again.prior.as_deref(), Some(first.record_cid.as_str()));
    assert_eq!(records(root).len(), 4);
    assert_eq!(
        roster_rows(root, "matthew").len(),
        1,
        "one genesis per handle"
    );

    // Another device's witness of the same handle is not refused by this device's.
    let (_, other) = device(&keys, "b");
    let elsewhere = witness(root, SUBJECT, WITNESS, "b-session", BASIS, false, &other)
        .expect("a different device witnesses for itself");
    assert_eq!(elsewhere.roster, RosterState::Unbound);
    assert_eq!(
        standing_on(root, &other),
        None,
        "unbound until the ceremony"
    );
}

// ── claim ────────────────────────────────────────────────────────────────────────────────────

#[test]
fn claim_human_appends_signed_record_agents_unchanged() {
    let dir = fixture();
    let keys = TempDir::new().unwrap();
    let root = dir.path();
    let (_, key) = device(&keys, "a");

    let human = claim_with_device(root, SUBJECT, SESSION, Some(&key)).expect("a human claims");
    assert_eq!(human.signer.as_deref(), Some(key.did_key().as_str()));
    let recs = records(root);
    assert_eq!(recs.len(), 2, "claim + signed: {recs:?}");
    assert!(matches!(&recs[0], ActorRecord::Claim(c) if c.claimed.0 == SUBJECT));
    let ActorRecord::Signed(sig) = &recs[1] else {
        panic!("the claim is signed by the device: {recs:?}")
    };
    assert_eq!(sig.claim_cid, human.record_cid);
    assert!(sig.verify(&device_key::verify));

    // A re-claim at the same HEAD is a no-op, and so is its signature.
    let again = claim_with_device(root, SUBJECT, SESSION, Some(&key)).unwrap();
    assert!(!again.appended);
    assert_eq!(records(root).len(), 2);

    // An agent claim is unchanged: no signature, even with a device key in hand.
    let agent = claim_with_device(root, "agent:scribe@opus-5", "agent-session", Some(&key))
        .expect("an agent claims");
    assert_eq!(agent.signer, None);
    assert_eq!(records(root).len(), 3, "one claim record, no signature");
    let json = serde_json::to_string(&agent).unwrap();
    assert!(!json.contains("signer"), "agent payload unchanged: {json}");

    // And the plain library claim (no device) signs nothing — the floor.
    let bare = fixture();
    actor::claim(bare.path(), SUBJECT, SESSION).unwrap();
    assert_eq!(records(bare.path()).len(), 1);
}

// ── contest ──────────────────────────────────────────────────────────────────────────────────

#[test]
fn contest_appends_signed_contest_row() {
    let dir = fixture();
    let keys = TempDir::new().unwrap();
    let root = dir.path();
    let (_, key) = device(&keys, "a");

    // Nothing standing yet: nothing to contest.
    let err = contest(root, SUBJECT, "human:matthew", SESSION, "not me", &key)
        .expect_err("nothing stands");
    assert!(err.to_string().contains("nothing standing"), "got: {err}");

    let witnessed = witness(root, SUBJECT, WITNESS, SESSION, BASIS, false, &key).unwrap();
    assert!(standing_on(root, &key).is_some());

    let err = contest(root, SUBJECT, "human:matthew", SESSION, "  ", &key)
        .expect_err("a contest needs a basis");
    assert!(err.to_string().contains("basis"), "got: {err}");

    let outcome = contest(
        root,
        SUBJECT,
        "human:matthew",
        "contest-session",
        "that was not me at the keyboard",
        &key,
    )
    .expect("the human contests the witness");
    assert_eq!(outcome.target_cid, witnessed.record_cid);

    let rows = roster_rows(root, "matthew");
    assert_eq!(rows.len(), 2, "genesis + contest");
    let ParticipantRow::Contest {
        target_cid,
        by,
        basis,
        ..
    } = &rows[1]
    else {
        panic!("contest row expected: {rows:?}")
    };
    assert_eq!(target_cid, &witnessed.record_cid);
    assert_eq!(by, &key.did_key());
    assert_eq!(basis, "that was not me at the keyboard");
    assert_eq!(
        standing_on(root, &key),
        None,
        "a contested witness confers no standing"
    );
}

// ── device ceremony ──────────────────────────────────────────────────────────────────────────

#[test]
fn device_enroll_authorize_bind_round_trip_with_two_keys() {
    let dir = fixture();
    let keys = TempDir::new().unwrap();
    let root = dir.path();
    let (_, a) = device(&keys, "a");
    let (_, b) = device(&keys, "b");
    assert_ne!(a.did_key(), b.did_key());

    // A: witnessed, the roster's genesis.
    witness(root, SUBJECT, WITNESS, SESSION, BASIS, false, &a).unwrap();
    // B: its present agent witnesses too — unbound until the ceremony.
    let on_b = witness(root, SUBJECT, WITNESS, "b-session", BASIS, false, &b).unwrap();
    assert_eq!(on_b.roster, RosterState::Unbound);
    assert_eq!(standing_on(root, &b), None);

    // B enrolls: a request carrying B's did:key and a nonce, the nonce kept on B.
    let request = device_enroll(root, "matthew", &b).expect("enroll");
    assert_eq!(request.controller, b.did_key());
    let nonce_file = root.join(".eprfs/status/participants/.enroll-matthew.json");
    assert!(nonce_file.is_file());
    let request_json = serde_json::to_string(&request).unwrap();
    assert!(!request_json.contains("seed"), "no key material crosses");

    // A authorizes (A is a member); the authorization travels as JSON, no key inside.
    let authorization = device_authorize(root, &request_json, &a).expect("authorize");
    assert_eq!(authorization.authorized_by, a.did_key());
    assert_eq!(authorization.chain_root, a.did_key());
    let authorization_json = serde_json::to_string(&authorization).unwrap();

    // B countersigns and appends the binding.
    let bound = device_bind(root, &authorization_json, &b).expect("bind");
    assert!(!nonce_file.exists(), "the nonce is spent");
    let rows = roster_rows(root, "matthew");
    let ParticipantRow::Binding {
        controller,
        authorized_by,
        sig_a,
        sig_b,
        ..
    } = rows.last().unwrap()
    else {
        panic!("binding row expected: {rows:?}")
    };
    assert_eq!(controller, &b.did_key());
    assert_eq!(authorized_by, &a.did_key());
    assert!(!sig_a.is_empty() && !sig_b.is_empty(), "both halves");
    assert_eq!(
        bound.row_cid,
        SidecarRoster::open(root, "matthew")
            .unwrap()
            .read()
            .unwrap()
            .rows()
            .last()
            .unwrap()
            .0
            .to_string()
    );

    // After bind, standing_human on device B resolves the handle.
    assert_eq!(standing_on(root, &b), Some(AgentRef(SUBJECT.into())));
    assert_eq!(standing_on(root, &a), Some(AgentRef(SUBJECT.into())));

    // The request and the authorization may also be passed as a file path.
    let (_, c) = device(&keys, "c");
    let request = device_enroll(root, "matthew", &c).unwrap();
    let request_path = keys.path().join("request.json");
    std::fs::write(&request_path, serde_json::to_string(&request).unwrap()).unwrap();
    let authorization = device_authorize(root, request_path.to_str().unwrap(), &b)
        .expect("a bound device may authorize the next");
    let authorization_path = keys.path().join("authorization.json");
    std::fs::write(
        &authorization_path,
        serde_json::to_string(&authorization).unwrap(),
    )
    .unwrap();
    device_bind(root, authorization_path.to_str().unwrap(), &c).expect("bind from a path");
}

#[test]
fn bind_refuses_nonce_mismatch_and_missing_half() {
    let dir = fixture();
    let keys = TempDir::new().unwrap();
    let root = dir.path();
    let (_, a) = device(&keys, "a");
    let (_, b) = device(&keys, "b");
    witness(root, SUBJECT, WITNESS, SESSION, BASIS, false, &a).unwrap();

    let request = device_enroll(root, "matthew", &b).unwrap();
    let authorization =
        device_authorize(root, &serde_json::to_string(&request).unwrap(), &a).unwrap();
    let rows_before = roster_rows(root, "matthew").len();

    // A replayed/forged nonce.
    let mut wrong_nonce = serde_json::to_value(&authorization).unwrap();
    wrong_nonce["nonce"] = serde_json::Value::String("not-the-nonce-b-minted".into());
    let err = device_bind(root, &wrong_nonce.to_string(), &b).expect_err("nonce mismatch");
    assert!(err.to_string().contains("nonce"), "got: {err}");

    // Half A missing.
    let mut no_half_a = serde_json::to_value(&authorization).unwrap();
    no_half_a["sigA"] = serde_json::Value::String(String::new());
    let err = device_bind(root, &no_half_a.to_string(), &b).expect_err("half A missing");
    assert!(err.to_string().contains("missing"), "got: {err}");

    // Half A present but not A's signature (B signed it itself).
    let mut forged = serde_json::to_value(&authorization).unwrap();
    forged["sigA"] = serde_json::Value::String("00".repeat(64));
    let err = device_bind(root, &forged.to_string(), &b).expect_err("half A must verify");
    assert!(err.to_string().contains("half A"), "got: {err}");

    // An authorizer that is not a member cannot authorize at all.
    let (_, stranger) = device(&keys, "stranger");
    let err = device_authorize(root, &serde_json::to_string(&request).unwrap(), &stranger)
        .expect_err("only a member authorizes");
    assert!(err.to_string().contains("not a member"), "got: {err}");

    // A device that never enrolled has no nonce to match.
    let (_, c) = device(&keys, "c");
    let err = device_bind(root, &serde_json::to_string(&authorization).unwrap(), &c)
        .expect_err("no enrolment on this device");
    assert!(err.to_string().contains("enrol"), "got: {err}");

    assert_eq!(
        roster_rows(root, "matthew").len(),
        rows_before,
        "every refusal appended nothing"
    );
    assert!(
        root.join(".eprfs/status/participants/.enroll-matthew.json")
            .is_file(),
        "a refused bind keeps the pending nonce"
    );
    assert_eq!(standing_on(root, &b), None);

    // The honest authorization still binds after the refusals.
    device_bind(root, &serde_json::to_string(&authorization).unwrap(), &b).expect("binds");
}

// ── current ──────────────────────────────────────────────────────────────────────────────────

#[test]
fn current_prints_standing_when_session_unclaimed() {
    let dir = fixture();
    let keys = TempDir::new().unwrap();
    let root = dir.path();
    let (key_file, key) = device(&keys, "a");
    witness(root, SUBJECT, WITNESS, SESSION, BASIS, false, &key).unwrap();

    let out = epr(
        root,
        &key_file,
        &["actor", "current", "--session", "a-fresh-session"],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("(no claim registered)"), "{text}");
    assert!(
        text.contains(&format!("standing {SUBJECT} (witnessed by {WITNESS} on ")),
        "{text}"
    );
    assert!(text.contains("device did:key:z6Mk"), "{text}");

    // The JSON payload carries it too, beside the explicit null claim.
    let out = epr(
        root,
        &key_file,
        &["actor", "current", "--session", "a-fresh-session", "--json"],
    );
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(json["claim"].is_null());
    assert_eq!(json["standing"]["subject"], SUBJECT);
    assert_eq!(json["standing"]["witnessedBy"], WITNESS);
    assert_eq!(json["standing"]["device"], key.did_key());

    // A session WITH a claim prints the claim as today.
    actor::claim(root, "agent:scribe@opus-5", "claimed-session").unwrap();
    let out = epr(
        root,
        &key_file,
        &["actor", "current", "--session", "claimed-session"],
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("agent:scribe@opus-5"), "{text}");
    assert!(!text.contains("standing"), "{text}");
}

#[test]
fn current_prints_unwitnessed_on_a_bare_device() {
    let dir = fixture();
    let keys = TempDir::new().unwrap();
    let root = dir.path();
    let key_file = keys.path().join("never-minted.seed");

    let out = epr(root, &key_file, &["actor", "current", "--session", "s1"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("(unwitnessed)"), "{text}");
    assert!(!key_file.exists(), "a read never mints a device key");

    // A roster witnessed by ANOTHER device leaves this one unwitnessed.
    let (_, other) = device(&keys, "other");
    witness(root, SUBJECT, WITNESS, SESSION, BASIS, false, &other).unwrap();
    let (key_file, _) = device(&keys, "this");
    let out = epr(
        root,
        &key_file,
        &["actor", "current", "--session", "s1", "--json"],
    );
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(json["standing"].is_null(), "{json}");
}

// ── review findings (post-station-4 Lane P, rulings R-P15, R-P16, R-P19) ─────────────────────

/// Copy the tracked roster of `from` into `to` — what a fresh clone or worktree of a pushed
/// checkout holds: the roster, and none of the gitignored actor sidecar.
fn carry_roster(from: &Path, to: &Path, handle: &str) {
    let rel = format!(".eprfs/status/participants/{handle}.jsonl");
    std::fs::create_dir_all(to.join(".eprfs/status/participants")).unwrap();
    std::fs::copy(from.join(&rel), to.join(&rel)).unwrap();
}

fn current_json(root: &Path, key_file: &Path, extra: &[&str]) -> serde_json::Value {
    let mut args = vec!["actor", "current", "--json"];
    args.extend_from_slice(extra);
    let out = epr(root, key_file, &args);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn w2_root_pin_is_written_on_first_verified_read() {
    let dir = fixture();
    let keys = TempDir::new().unwrap();
    let root = dir.path();
    let (key_file, key) = device(&keys, "a");
    let pin = device_key::root_pin_path(&key_file, "matthew");
    assert_eq!(pin, keys.path().join("a/rosters/matthew.root"));

    witness(root, SUBJECT, WITNESS, SESSION, BASIS, false, &key).unwrap();
    assert!(!pin.exists(), "witnessing writes the roster, not the pin");

    let json = current_json(root, &key_file, &["--session", "fresh"]);
    assert_eq!(json["standing"]["subject"], SUBJECT);
    assert_eq!(
        std::fs::read_to_string(&pin).unwrap().trim(),
        key.did_key(),
        "the first verified read pins the chain root beside the key"
    );

    // A device that is not a member pins nothing.
    let (other_file, _) = device(&keys, "stranger");
    let json = current_json(root, &other_file, &["--session", "fresh"]);
    assert!(json["standing"].is_null());
    assert!(!device_key::root_pin_path(&other_file, "matthew").exists());
}

#[test]
fn w2_current_prints_contested_when_the_root_differs_from_the_pin() {
    let dir = fixture();
    let keys = TempDir::new().unwrap();
    let root = dir.path();
    let (key_file, key) = device(&keys, "a");
    let (_, eve) = device(&keys, "eve");
    witness(root, SUBJECT, WITNESS, SESSION, BASIS, false, &key).unwrap();
    // This device pinned ANOTHER root on an earlier read (the lineage it first saw).
    device_key::write_root_pin(&key_file, "matthew", &eve.did_key()).unwrap();

    let json = current_json(root, &key_file, &["--session", "fresh"]);
    assert!(
        json["standing"].is_null(),
        "contested, never standing: {json}"
    );
    assert_eq!(json["contested"]["subject"], SUBJECT);
    assert_eq!(json["contested"]["pinned"], eve.did_key());
    assert_eq!(json["contested"]["found"], key.did_key());
    assert_eq!(
        actor::standing_on_device(root, &key_file),
        None,
        "attribution sees no standing either"
    );

    let out = epr(root, &key_file, &["actor", "current", "--session", "fresh"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("contested (roster root differs from this device's pin)"),
        "{text}"
    );
    // The pin is never silently re-written to the roster's root.
    assert_eq!(
        device_key::read_root_pin(&key_file, "matthew").unwrap(),
        Some(eve.did_key())
    );
}

#[test]
fn w3_fresh_checkout_on_a_witnessed_device_is_standing() {
    let witnessed = fixture();
    let fresh = fixture();
    let keys = TempDir::new().unwrap();
    let (a_file, a) = device(&keys, "a");
    let (b_file, b) = device(&keys, "b");
    witness(
        witnessed.path(),
        SUBJECT,
        WITNESS,
        SESSION,
        BASIS,
        false,
        &a,
    )
    .unwrap();
    let request = device_enroll(witnessed.path(), "matthew", &b).unwrap();
    let auth = device_authorize(
        witnessed.path(),
        &serde_json::to_string(&request).unwrap(),
        &a,
    )
    .unwrap();
    device_bind(witnessed.path(), &serde_json::to_string(&auth).unwrap(), &b).unwrap();

    carry_roster(witnessed.path(), fresh.path(), "matthew");
    assert!(!fresh.path().join(".eprfs/status/actors.jsonl").exists());

    let json = current_json(fresh.path(), &a_file, &["--session", "worktree-session"]);
    assert_eq!(json["standing"]["subject"], SUBJECT, "{json}");
    assert_eq!(json["standing"]["roster"]["via"], "chain-root");
    let on_a = actor::standing_on_device(fresh.path(), &a_file).expect("A stands");
    assert_eq!(
        on_a.record_cid,
        roster_rows(witnessed.path(), "matthew")
            .iter()
            .find_map(|r| match r {
                ParticipantRow::Genesis { record_cid, .. } => Some(record_cid.clone()),
                _ => None,
            })
            .unwrap(),
        "the chain root stands on the founding record its genesis names"
    );

    let json = current_json(fresh.path(), &b_file, &["--session", "worktree-session"]);
    assert_eq!(json["standing"]["subject"], SUBJECT, "{json}");
    assert_eq!(json["standing"]["roster"]["via"], "bound");

    let out = epr(
        fresh.path(),
        &a_file,
        &["actor", "current", "--session", "worktree-session"],
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("standing human:matthew (by the tracked roster"),
        "{text}"
    );
    assert!(!text.contains("(unwitnessed)"), "{text}");
}

#[test]
fn w3_again_reads_the_roster() {
    let witnessed = fixture();
    let fresh = fixture();
    let keys = TempDir::new().unwrap();
    let (_, a) = device(&keys, "a");
    let first = witness(
        witnessed.path(),
        SUBJECT,
        WITNESS,
        SESSION,
        BASIS,
        false,
        &a,
    )
    .unwrap();
    carry_roster(witnessed.path(), fresh.path(), "matthew");

    let err = witness(fresh.path(), SUBJECT, WITNESS, "wt", BASIS, false, &a)
        .expect_err("the roster says this device is already witnessed");
    assert!(err.to_string().contains("--again"), "{err}");
    assert!(
        !fresh.path().join(".eprfs/status/actors.jsonl").exists()
            || records(fresh.path()).is_empty(),
        "the refusal appended nothing"
    );

    let again = witness(fresh.path(), SUBJECT, WITNESS, "wt", BASIS, true, &a).expect("--again");
    assert_eq!(again.prior.as_deref(), Some(first.record_cid.as_str()));
    assert_eq!(again.roster, RosterState::Joined);
    assert_eq!(roster_rows(fresh.path(), "matthew").len(), 1);
}

#[test]
fn m1_roster_failure_leaves_no_orphan_witness() {
    let dir = fixture();
    let keys = TempDir::new().unwrap();
    let root = dir.path();
    let (_, key) = device(&keys, "a");
    // A roster whose bytes no longer match their CIDs — unreadable, so the genesis decision
    // cannot be made.
    let rel = root.join(".eprfs/status/participants/matthew.jsonl");
    std::fs::create_dir_all(rel.parent().unwrap()).unwrap();
    std::fs::write(&rel, "{\"cid\":\"bafyreinotacid\",\"row\":{}}\n").unwrap();

    witness(root, SUBJECT, WITNESS, SESSION, BASIS, false, &key)
        .expect_err("an unreadable roster refuses the witness");
    let signed = if root.join(".eprfs/status/actors.jsonl").exists() {
        records(root).len()
    } else {
        0
    };
    assert_eq!(
        signed, 0,
        "no signed witness is left behind a roster failure"
    );
}

#[test]
fn m5_rewitness_after_contest_must_name_it() {
    let dir = fixture();
    let keys = TempDir::new().unwrap();
    let root = dir.path();
    let (key_file, key) = device(&keys, "a");
    let first = witness(root, SUBJECT, WITNESS, SESSION, BASIS, false, &key).unwrap();
    let contested = contest(root, SUBJECT, "human:matthew", "c", "not me", &key).unwrap();
    assert_eq!(actor::standing_on_device(root, &key_file), None);
    let before = records(root).len();

    let request = |answers: Option<&'static str>, again: bool| actor::WitnessRequest {
        subject: SUBJECT,
        witness_as: WITNESS,
        session: "rewitness",
        basis: BASIS,
        again,
        answers,
    };
    let err = actor::witness_answering(root, &request(None, true), &key)
        .expect_err("a re-witness after a contest names it");
    assert!(err.to_string().contains("--answers"), "{err}");
    assert!(
        err.to_string()
            .contains(&contested.row_cid[contested.row_cid.len() - 6..]),
        "the refusal names the open contest: {err}"
    );

    let not_a_contest: &'static str = Box::leak(first.record_cid.clone().into_boxed_str());
    let err = actor::witness_answering(root, &request(Some(not_a_contest), true), &key)
        .expect_err("--answers must name an open contest");
    assert!(err.to_string().contains("not an open contest"), "{err}");

    let row: &'static str = Box::leak(contested.row_cid.clone().into_boxed_str());
    let err = actor::witness_answering(root, &request(Some(row), false), &key)
        .expect_err("--answers is a re-witness flag");
    assert!(err.to_string().contains("--again"), "{err}");
    assert_eq!(
        records(root).len(),
        before,
        "every refusal appended nothing"
    );

    let answered = actor::witness_answering(root, &request(Some(row), true), &key)
        .expect("the answering re-witness");
    assert_eq!(answered.answers.as_deref(), Some(row));
    let ActorRecord::Witness(w) = records(root)
        .into_iter()
        .find(|r| r.cid().unwrap().to_string() == answered.record_cid)
        .unwrap()
    else {
        panic!("a witness record")
    };
    assert_eq!(
        w.answers.as_deref(),
        Some(row),
        "the record names what it answers"
    );
    assert!(
        actor::standing_on_device(root, &key_file).is_some(),
        "stands again"
    );

    // Answered: a later --again needs no --answers for that contest.
    witness(root, SUBJECT, WITNESS, "third", BASIS, true, &key).expect("nothing open");
}

#[test]
fn m8_current_device_reads_the_device_under_no_claimable_session() {
    let dir = fixture();
    let keys = TempDir::new().unwrap();
    let root = dir.path();
    let (key_file, key) = device(&keys, "a");
    witness(root, SUBJECT, WITNESS, SESSION, BASIS, false, &key).unwrap();
    // Whatever label a hook might have used, someone can claim it — and a claim hides the device.
    actor::claim(
        root,
        "agent:scribe@opus-5",
        "participant-standing:device-read",
    )
    .unwrap();
    let hidden = current_json(
        root,
        &key_file,
        &["--session", "participant-standing:device-read"],
    );
    assert!(
        hidden["standing"].is_null(),
        "a claimed label hides the device"
    );

    let json = current_json(root, &key_file, &["--device"]);
    assert!(json["session"].is_null(), "{json}");
    assert!(json["claim"].is_null());
    assert_eq!(json["standing"]["subject"], SUBJECT);

    let both = epr(
        root,
        &key_file,
        &["actor", "current", "--device", "--session", "s"],
    );
    assert!(!both.status.success(), "--device consults no session");
}
