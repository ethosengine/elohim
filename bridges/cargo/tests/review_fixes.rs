//! One test per fix from the two independent reviews of the bridge (code-reviewer and Codex):
//! privacy beyond `authors` (H1), an offer address that binds its disclosures and not its runner
//! (H2), a first-observation time that survives history rewrites (M1), a relative `--lockfile`
//! (M2), metadata/lockfile mismatch (M4), an unusable repository URL (M5), and the LOW items that
//! are observable here (verify never creates a store, old lockfile formats refuse).

use std::path::Path;
use std::process::Command;

use cargo_bridge::{observe, reliance, verify, Config, Inputs};
use elohim_epr_rea::FlowRecord;

const METADATA: &str = include_str!("fixtures/ws/metadata.json");
const LOCKFILE: &str = include_str!("fixtures/ws/Cargo.lock");
const AT: &str = "2026-10-15T00:00:00Z";

fn config() -> Config {
    Config::for_repository("fixture-ws", "/fixture/repo", AT)
}

fn inputs(metadata: &str, lockfile: &str) -> Inputs {
    Inputs {
        metadata: metadata.to_string(),
        lockfile: lockfile.as_bytes().to_vec(),
    }
}

/// Every string a run could publish: the offer document and every record.
fn published(r: &cargo_bridge::Reliance) -> String {
    let mut all = r.offer.text().to_string();
    for record in r.records() {
        all.push('\n');
        all.push_str(&serde_json::to_string(&record).unwrap());
    }
    all
}

/// The fixture with credentials, ports, queries, fragments and an email-shaped home directory
/// planted OUTSIDE `authors`: in repository URLs, a git source and a path dependency's location.
fn leaky_metadata() -> String {
    METADATA
        .replace(
            "\"repository\": \"https://github.com/serde-rs/serde\"",
            "\"repository\": \"https://user:s3cr3t-token@github.com/serde-rs/serde?ref=main#frag\"",
        )
        .replace(
            "\"repository\": null,\n      \"homepage\": null,\n      \"documentation\": null,\n      \"edition\": \"2018\"",
            "\"repository\": \"https://deploy:ghp_leaked@git.example.org:8443/team/itoa?token=abc\",\n      \"homepage\": null,\n      \"documentation\": null,\n      \"edition\": \"2018\"",
        )
        .replace(
            "git+https://github.com/example/gitcrate?rev=0123456#0123456789abcdef0123456789abcdef01234567",
            "git+https://ci-bot:glpat-secret@github.com/example/gitcrate?rev=0123456#0123456789abcdef0123456789abcdef01234567",
        )
        .replace(
            "/fixture/repo/libs/localutil",
            "/home/alice@example.com/lib/localutil",
        )
}

#[test]
fn h1_no_userinfo_token_port_query_or_home_path_reaches_any_record_or_the_offer() {
    let metadata = leaky_metadata();
    for planted in ["s3cr3t", "ghp_leaked", "glpat-secret", "alice@example.com"] {
        assert!(metadata.contains(planted), "fixture must plant `{planted}`");
    }
    let r = reliance(&inputs(&metadata, LOCKFILE), &config()).unwrap();
    let all = published(&r);
    for leak in [
        "s3cr3t",
        "user:",
        "ghp_leaked",
        "deploy:",
        "glpat-secret",
        "ci-bot",
        "8443",
        "?ref=",
        "token=",
        "#frag",
        "alice",
        "/home/",
        "example.com",
    ] {
        assert!(
            !all.contains(leak),
            "`{leak}` leaked into a record or the offer"
        );
    }
    // The normalized locations still name the projects.
    let inflows: Vec<&str> = r
        .offer
        .declaration()
        .disclosure
        .inflows
        .iter()
        .map(|i| i.presence.as_str())
        .collect();
    assert!(inflows.contains(&"presence:github-com/serde-rs/serde"));
    assert!(inflows.contains(&"presence:git-example-org/team/itoa"));
    assert!(inflows.contains(&"presence:github-com/example/gitcrate"));
    // The path dependency outside the repository is named by a hash, never its path.
    let outside = r
        .cited
        .iter()
        .find(|c| c.coordinate.starts_with("path:"))
        .unwrap();
    assert!(
        outside.coordinate.starts_with("path:external/"),
        "{}",
        outside.coordinate
    );
    // Every `@` left anywhere is a `name@version` coordinate or an agent ref, never userinfo.
    for (i, _) in all.match_indices('@') {
        let after = &all[i + 1..];
        assert!(
            after.starts_with(|c: char| c.is_ascii_digit()) || after.starts_with("claude-"),
            "unexpected `@` before `{}`",
            &after[..after.len().min(24)]
        );
    }
}

#[test]
fn h2_the_offer_address_binds_its_disclosures_and_not_its_runner() {
    let base = reliance(&inputs(METADATA, LOCKFILE), &config()).unwrap();

    // A different runner (author) leaves the offer, and so every record, where it was.
    let mut other_runner = config();
    other_runner.author = "human:someone-else".into();
    let same = reliance(&inputs(METADATA, LOCKFILE), &other_runner).unwrap();
    assert_eq!(same.offer_cid, base.offer_cid);
    assert_eq!(same.process_cid, base.process_cid);
    assert_ne!(
        same.offer.text(),
        base.offer.text(),
        "the author is still on record"
    );

    // A changed disclosure (itoa now declares a repository: a new inflow) re-addresses the offer.
    let disclosed = METADATA.replace(
        "\"repository\": null,\n      \"homepage\": null,\n      \"documentation\": null,\n      \"edition\": \"2018\"",
        "\"repository\": \"https://github.com/dtolnay/itoa\",\n      \"homepage\": null,\n      \"documentation\": null,\n      \"edition\": \"2018\"",
    );
    assert_ne!(disclosed, METADATA);
    let changed = reliance(&inputs(&disclosed, LOCKFILE), &config()).unwrap();
    assert_ne!(changed.offer_cid, base.offer_cid);

    // The disclosures are in the body the address is taken over, not only the frontmatter.
    let body = base.offer.text().split("\n---\n").nth(1).unwrap();
    assert!(body.contains("presence:github-com/serde-rs/serde"));
    assert!(body.contains("build:run-dependency-code"));
    assert!(
        !body.contains("agent:implementer"),
        "the author stays outside the address"
    );
}

#[test]
fn h2_observe_persists_the_offer_document_and_verify_checks_it() {
    let dir = tempfile::tempdir().unwrap();
    let outcome = observe(dir.path(), &inputs(METADATA, LOCKFILE), &config()).unwrap();
    let r = reliance(&inputs(METADATA, LOCKFILE), &config()).unwrap();
    let path = dir.path().join(r.offer.path());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), r.offer.text());
    assert_eq!(outcome.offer_document, path.display().to_string());
    assert!(verify(dir.path(), &inputs(METADATA, LOCKFILE), &config())
        .unwrap()
        .is_fresh());
    std::fs::write(&path, "edited").unwrap();
    let stale = verify(dir.path(), &inputs(METADATA, LOCKFILE), &config()).unwrap();
    assert!(!stale.is_fresh() && !stale.offer_document_current);
}

#[test]
fn m1_a_rewritten_commit_time_reuses_the_first_observation_and_appends_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let first = observe(dir.path(), &inputs(METADATA, LOCKFILE), &config()).unwrap();
    let flows = dir.path().join(".eprfs/status/flows.jsonl");
    let bytes = std::fs::read(&flows).unwrap();

    // Same lockfile, a different commit time (a rebase or squash moved it).
    let rebased = Config::for_repository("fixture-ws", "/fixture/repo", "2027-01-01T12:00:00Z");
    let again = observe(dir.path(), &inputs(METADATA, LOCKFILE), &rebased).unwrap();
    assert_eq!((again.appended, again.present), (0, first.appended));
    assert_eq!(
        again.occurred_at, AT,
        "the first observation's time is kept"
    );
    assert_eq!(std::fs::read(&flows).unwrap(), bytes);
    assert!(verify(dir.path(), &inputs(METADATA, LOCKFILE), &rebased)
        .unwrap()
        .is_fresh());
}

#[test]
fn m4_metadata_and_lockfile_from_different_resolves_refuse_with_nothing_written() {
    // itoa is resolved by the metadata but absent from this lockfile.
    let older_lock = LOCKFILE.replace(
        "[[package]]\nname = \"itoa\"\nversion = \"1.0.11\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\nchecksum = \"d82a8b4f1f75b6422de6545986df3943d10fe75690461d441e98bc4d3fcb9419\"\n\n",
        "",
    );
    assert_ne!(older_lock, LOCKFILE);
    let dir = tempfile::tempdir().unwrap();
    let err = observe(dir.path(), &inputs(METADATA, &older_lock), &config()).unwrap_err();
    assert!(err.to_string().contains("disagree"), "{err}");
    assert!(err.to_string().contains("crates.io:itoa@1.0.11"), "{err}");
    assert!(!dir.path().join(".eprfs").exists());
    assert!(verify(dir.path(), &inputs(METADATA, &older_lock), &config()).is_err());
}

#[test]
fn m5_an_unusable_repository_url_falls_back_to_the_crate_and_never_blocks_the_workspace() {
    for bad in ["https://", "git@", "   "] {
        let metadata = METADATA.replace(
            "\"repository\": \"https://github.com/serde-rs/serde\"",
            &format!("\"repository\": \"{bad}\""),
        );
        let r = reliance(&inputs(&metadata, LOCKFILE), &config()).unwrap();
        let serde = r
            .events
            .iter()
            .find(|e| e.classified_as[1] == "crates.io:serde@1.0.200")
            .unwrap();
        assert_eq!(serde.provider.0, "presence:crates-io/serde", "{bad:?}");
        assert!(r
            .offer
            .declaration()
            .disclosure
            .inflows
            .iter()
            .all(|i| i.presence != "presence:"));
        assert_eq!(r.report.cited, 5);
    }
}

#[test]
fn low_a_lockfile_older_than_v3_refuses_naming_its_version() {
    let v2 = LOCKFILE.replace("version = 4\n", "");
    let err = reliance(&inputs(METADATA, &v2), &config()).unwrap_err();
    assert!(err.to_string().contains("v1 or v2"), "{err}");
    let v2_explicit = LOCKFILE.replace("version = 4\n", "version = 2\n");
    let err = reliance(&inputs(METADATA, &v2_explicit), &config()).unwrap_err();
    assert!(err.to_string().contains("v2"), "{err}");
}

#[test]
fn low_unvetted_licence_text_never_reaches_a_slot() {
    let metadata = METADATA.replace(
        "\"license\": \"MPL-2.0\"",
        "\"license\": \"see LICENSE <legal@fixture.example>\"",
    );
    let r = reliance(&inputs(&metadata, LOCKFILE), &config()).unwrap();
    for record in r.records() {
        if let FlowRecord::Event(e) = record {
            for slot in e.classified_as.iter().filter(|s| s.starts_with("license:")) {
                assert!(!slot.contains('<') && !slot.contains('@'), "{slot}");
            }
        }
    }
}

/// The CLI with a RELATIVE `--lockfile` (M2: the git timestamp lookup used to cd and pass the
/// relative path again) and `verify --store <missing>` (it must not create the directory).
#[test]
fn m2_and_low_cli_relative_lockfile_and_verify_never_creates_a_store() {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let scratch = tempfile::tempdir().unwrap();
    let metadata = scratch.path().join("metadata.json");
    std::fs::write(&metadata, METADATA).unwrap();
    let missing = scratch.path().join("no-store-here");
    let out = Command::new(env!("CARGO_BIN_EXE_cargo-bridge"))
        .current_dir(crate_dir)
        .args(["verify", "--metadata"])
        .arg(&metadata)
        .args(["--lockfile", "tests/fixtures/ws/Cargo.lock", "--store"])
        .arg(&missing)
        .args(["--root", "."])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "exit {:?}: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("notice:"));
    assert!(!missing.exists(), "verify created the store directory");
}
