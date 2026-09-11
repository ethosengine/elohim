//! `epr flow report placement` — ledger parity against `placement-audit.py --ledger`.
//!
//! Station two of `genesis/docs/superpowers/plans/2026-09-10-memory-kit-replacement-finish.md`.
//!
//! The corpus below is a SYNTHETIC tempdir of about thirty documents, one per state the vocabulary
//! can produce, rather than the live tree. Two reasons, and they are the same reason twice: the live
//! corpus is authored by everyone and changes hourly, so a test asserting counts against it would be
//! measuring the repository's editing cadence; and a synthetic corpus can contain the states the
//! live tree happens not to have today (`REGRESSED`, `BLOCKED-BY-ENV`, an empty pressure dir), which
//! are exactly the arms a port is most likely to get wrong.
//!
//! Parity is asserted TWICE over the same fixture:
//!
//! 1. against `EXPECTED_BALANCE`, pinned constants — so the test says something even where `python3`
//!    is absent, and so it is never the vacuous "zero equals zero";
//! 2. against `placement-audit.py --ledger --json <tmpdir>` when the interpreter and the script are
//!    both present — the actual oracle. Station six deletes that script; from then on the pinned
//!    constants are the surviving witness, and the oracle leg skips with a reason rather than failing.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use elohim_epr_cli::flow::placement::placement;
use elohim_epr_cli::flow::scope::focus_baseline;
use tempfile::TempDir;

/// The state histogram this corpus produces. Every arm of `budget_state` is represented.
const EXPECTED_BALANCE: [(&str, usize); 11] = [
    ("ACTIVE", 4),
    ("BLOCKED-BY-ENV", 3),
    ("CLAIMED-ONLY", 4),
    ("LINKED", 1),
    ("MEM-UNLINKED", 1),
    ("NEEDS-TRIAGE", 4),
    ("REGRESSED", 2),
    ("SETTLED", 3),
    ("SUPERSEDED", 2),
    ("UNKNOWN-STATUS", 3),
    ("VERIFIED-STABLE", 2),
];

const EXPECTED_TOTAL: usize = 29;
const EXPECTED_PRESSURE: usize = 16;
const EXPECTED_HELD: usize = 3;
const EXPECTED_SETTLED: usize = 10;

const CLUSTER_STATE: &str = r#"updated: 2026-09-10
resources:
  household-nodes:
    role: the household mesh
    available: true
    provides_node_types: [household]
  absent-cap:
    role: a capability that is down
    available: false
    provides_node_types: [remote]
"#;

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

/// A doc with `status:` frontmatter plus any extra frontmatter lines.
fn doc(status: &str, extra: &str) -> String {
    format!("---\ntitle: fixture\nstatus: {status}\n{extra}---\n\n# Fixture\n\nProse.\n")
}

fn corpus() -> TempDir {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    write(root, "genesis/manifests/cluster-state.yaml", CLUSTER_STATE);

    let specs = "genesis/docs/superpowers/specs";
    write(
        root,
        &format!("{specs}/a-proposed.md"),
        &doc("proposed", ""),
    );
    write(
        root,
        &format!("{specs}/b-landed-verified.md"),
        &doc("landed", "verified_by: [ci-run-42]\n"),
    );
    write(
        root,
        &format!("{specs}/c-landed-bare.md"),
        &doc("landed", ""),
    );
    write(
        root,
        &format!("{specs}/d-superseded.md"),
        &doc("superseded", ""),
    );
    write(
        root,
        &format!("{specs}/e-nostatus.md"),
        "# No frontmatter\n\nProse.\n",
    );
    write(root, &format!("{specs}/f-weird.md"), &doc("marinating", ""));
    write(
        root,
        &format!("{specs}/g-living.md"),
        &doc("living document", ""),
    );
    write(
        root,
        &format!("{specs}/h-blocked.md"),
        &doc("proposed", "requires_env: [absent-cap]\n"),
    );
    write(
        root,
        &format!("{specs}/i-regressed.md"),
        &doc("regressed", ""),
    );
    write(
        root,
        &format!("{specs}/j-claimed.md"),
        &doc("claimed-not-verified", ""),
    );
    write(
        root,
        &format!("{specs}/k-mdstatus.md"),
        "# Markdown status\n\n**Status:** landed\n",
    );
    // Structural files: never rows.
    write(root, &format!("{specs}/INDEX.md"), "# Index\n");
    write(root, &format!("{specs}/CLAUDE.md"), "# Gospel\n");

    let plans = "genesis/docs/superpowers/plans";
    write(
        root,
        &format!("{plans}/p-proposed.md"),
        &doc("proposed", ""),
    );
    write(
        root,
        &format!("{plans}/p-landed-commit.md"),
        &doc("landed", "landed_commit: abc1234\n"),
    );
    write(
        root,
        &format!("{plans}/p-empty-status.md"),
        "---\ntitle: fixture\nstatus:\n---\n\n# Empty status\n",
    );

    write(
        root,
        "genesis/docs/plans/legacy-draft.md",
        &doc("draft", ""),
    );

    let canon = "genesis/docs/content/elohim-protocol/architecture";
    write(
        root,
        &format!("{canon}/canon-landed.md"),
        &doc("accepted", ""),
    );
    write(
        root,
        &format!("{canon}/canon-living.md"),
        &doc("architecture pattern", ""),
    );
    write(
        root,
        &format!("{canon}/canon-nostatus.md"),
        "# Seed\n\nProse.\n",
    );
    write(root, &format!("{canon}/README.md"), "# Readme\n");

    let history = "genesis/docs/content/elohim-protocol/history";
    write(
        root,
        &format!("{history}/hist-landed.md"),
        &doc("landed", ""),
    );
    write(
        root,
        &format!("{history}/hist-dead.md"),
        &doc("retired", ""),
    );

    write(
        root,
        "genesis/docs/superpowers/notes/note-draft.md",
        &doc("wip", ""),
    );
    write(
        root,
        "genesis/docs/research/research-unknown.md",
        &doc("exploratory", ""),
    );

    // Pressure dirs: the HOME is authoritative over the frontmatter.
    write(
        root,
        "genesis/docs/_state/regression/reg-doc.md",
        &doc("landed", ""),
    );
    write(
        root,
        "genesis/docs/_state/blockers/blk-doc.md",
        &doc("proposed", ""),
    );
    write(
        root,
        "genesis/docs/_state/unverified/unv-doc.md",
        &doc("proposed", ""),
    );
    write(
        root,
        "genesis/docs/_state/needs-triage/tri-doc.md",
        &doc("landed", ""),
    );

    write(
        root,
        ".claude/memory/m-linked.md",
        "---\nname: linked\ncites:\n  - \"a | b | sha256:0000000000000000\"\n---\n\nBody.\n",
    );
    write(
        root,
        ".claude/memory/m-unlinked.md",
        "---\nname: unlinked\n---\n\nBody.\n",
    );
    write(
        root,
        ".claude/memory/m-blocked.md",
        "---\nname: blocked\nrequires_env: [absent-cap]\n---\n\nBody.\n",
    );
    write(root, ".claude/memory/MEMORY.md", "# Index\n");

    tmp
}

fn repo_root() -> Option<std::path::PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .map(Path::to_path_buf)
}

#[test]
fn the_balance_matches_the_pinned_histogram() {
    let tmp = corpus();
    let report = placement(tmp.path(), None).expect("placement derives");
    let expected: BTreeMap<String, usize> = EXPECTED_BALANCE
        .iter()
        .map(|(k, v)| (k.to_string(), *v))
        .collect();
    assert_eq!(report.balance, expected);
    assert_eq!(report.total_files, EXPECTED_TOTAL);
    assert_eq!(report.pressure, EXPECTED_PRESSURE);
    assert_eq!(report.held, EXPECTED_HELD);
    assert_eq!(report.settled, EXPECTED_SETTLED);
    // The balance SUMS to the total: nothing hides.
    assert_eq!(report.balance.values().sum::<usize>(), report.total_files);
}

#[test]
fn structural_files_are_never_rows() {
    let tmp = corpus();
    let report = placement(tmp.path(), None).expect("placement derives");
    for skipped in ["INDEX.md", "CLAUDE.md", "README.md", "MEMORY.md"] {
        assert!(
            !report.rows.iter().any(|r| r.path.ends_with(skipped)),
            "{skipped} must not be a ledger row"
        );
    }
}

#[test]
fn the_pressure_dir_home_outranks_the_frontmatter() {
    let tmp = corpus();
    let report = placement(tmp.path(), None).expect("placement derives");
    let find = |needle: &str| {
        report
            .rows
            .iter()
            .find(|r| r.path.ends_with(needle))
            .unwrap_or_else(|| panic!("{needle} missing from the ledger"))
            .clone()
    };
    // `status: landed` in a regression dir is REGRESSED, not VERIFIED-STABLE.
    assert_eq!(find("reg-doc.md").state, "REGRESSED");
    assert_eq!(find("tri-doc.md").state, "NEEDS-TRIAGE");
    assert_eq!(find("unv-doc.md").state, "CLAIMED-ONLY");
    assert_eq!(find("blk-doc.md").state, "BLOCKED-BY-ENV");
}

#[test]
fn a_landed_doc_without_evidence_is_a_claim_not_a_verification() {
    let tmp = corpus();
    let report = placement(tmp.path(), None).expect("placement derives");
    let state = |needle: &str| {
        report
            .rows
            .iter()
            .find(|r| r.path.ends_with(needle))
            .map(|r| r.state.clone())
            .unwrap_or_default()
    };
    assert_eq!(state("c-landed-bare.md"), "CLAIMED-ONLY");
    assert_eq!(state("b-landed-verified.md"), "VERIFIED-STABLE");
    assert_eq!(state("p-landed-commit.md"), "VERIFIED-STABLE");
    // …but in CANONICAL / HISTORY the same status is the settled steady state.
    assert_eq!(state("canon-landed.md"), "SETTLED");
    assert_eq!(state("hist-landed.md"), "SETTLED");
}

#[test]
fn a_living_status_is_settled_only_in_its_canonical_home() {
    let tmp = corpus();
    let report = placement(tmp.path(), None).expect("placement derives");
    let state = |needle: &str| {
        report
            .rows
            .iter()
            .find(|r| r.path.ends_with(needle))
            .map(|r| r.state.clone())
            .unwrap_or_default()
    };
    assert_eq!(state("canon-living.md"), "SETTLED");
    assert_eq!(state("g-living.md"), "UNKNOWN-STATUS");
}

#[test]
fn an_unmeasured_corpus_reports_unknown_rather_than_zero() {
    // A corpus of pure prose has no extractable station. `0 OPEN` would read as "nothing left to
    // do"; `unknown` reads as "nobody has decomposed this", which is the true fact.
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        "genesis/manifests/cluster-state.yaml",
        CLUSTER_STATE,
    );
    write(
        tmp.path(),
        "genesis/docs/superpowers/specs/prose.md",
        &doc("proposed", ""),
    );
    let report = placement(tmp.path(), None).expect("placement derives");
    assert!(!report.gaps.known);
    assert_eq!(report.gaps.open, 0);
    assert_eq!(report.gaps.docs_with_items, 0);
}

#[test]
fn stations_in_the_corpus_roll_up_with_their_env_scope() {
    let tmp = corpus();
    write(
        tmp.path(),
        "genesis/docs/superpowers/plans/with-stations.md",
        "---\ntitle: fixture\nstatus: proposed\n---\n\n## Delivery stations\n\n\
         - [ ] household work\n- [x] a claimed one\n- [ ] blocked one @requires:absent-cap\n",
    );
    let report = placement(tmp.path(), None).expect("placement derives");
    assert!(report.gaps.known);
    assert_eq!(report.gaps.docs_with_items, 1);
    assert_eq!(report.gaps.open, 1);
    assert_eq!(report.gaps.claimed, 1);
    assert_eq!(report.gaps.blocked, 1);
}

#[test]
fn the_focus_baseline_reads_the_a2o_trees() {
    let tmp = corpus();
    let root = tmp.path();
    write(
        root,
        "genesis/a2o/features/lms/plain.feature",
        "Feature: plain\n  Scenario: x\n",
    );
    write(
        root,
        "genesis/a2o/features/dataplane/mixed.feature",
        "Feature: mixed\n  @requires:absent-cap\n  Scenario: x\n",
    );
    write(
        root,
        "genesis/a2o/held/features/dataplane/held.feature",
        "@requires:absent-cap\nFeature: held\n  Scenario: x\n",
    );
    let baseline = focus_baseline(root, None).expect("focus derives");
    let lms = baseline.subjects.get("lms").expect("lms subject");
    assert_eq!(lms.in_focus.len(), 1);
    assert!(lms.held.is_empty() && lms.mixed.is_empty());
    let dp = baseline
        .subjects
        .get("dataplane")
        .expect("dataplane subject");
    assert_eq!(dp.mixed.len(), 1);
    assert_eq!(dp.held.len(), 1);

    let rendered = baseline.render(None);
    assert!(rendered.contains("FAIR-GAME subjects (1)"));
    assert!(rendered.contains("NARROWED subjects (1)"));
    assert!(rendered.contains("### dataplane   ⚠ narrowed — absent-cap down"));
    assert!(rendered.contains("a capability that is down")); // the role supplies the WHY
                                                             // One subject only.
    let one = baseline.render(Some("dataplane"));
    assert!(one.contains("### dataplane"));
    assert!(!one.contains("FAIR-GAME subjects"));
}

/// The oracle leg. Skips with a reason when `python3` or the kit script is absent — which is the
/// permanent state after station six.
#[test]
fn the_kit_agrees_with_the_native_balance_where_python_is_available() {
    let Some(repo) = repo_root() else {
        eprintln!("SKIP: could not resolve the repository root from CARGO_MANIFEST_DIR");
        return;
    };
    let script = repo.join(".claude/scripts/memory-kit/placement-audit.py");
    if !script.is_file() {
        eprintln!(
            "SKIP: {} is absent (expected once station six deletes the kit) — \
             the pinned EXPECTED_BALANCE remains the witness",
            script.display()
        );
        return;
    }
    let tmp = corpus();
    let output = match Command::new("python3")
        .arg(&script)
        .arg("--ledger")
        .arg("--json")
        .arg(tmp.path())
        .current_dir(&repo)
        .output()
    {
        Ok(out) if out.status.success() => out,
        Ok(out) => {
            eprintln!(
                "SKIP: the kit exited {:?}: {}",
                out.status.code(),
                String::from_utf8_lossy(&out.stderr)
            );
            return;
        }
        Err(err) => {
            eprintln!("SKIP: python3 is not runnable here ({err})");
            return;
        }
    };
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("the kit prints JSON under --json");
    let mut kit_balance: BTreeMap<String, usize> = BTreeMap::new();
    for row in payload
        .get("rows")
        .and_then(|v| v.as_array())
        .expect("the kit's payload carries rows")
    {
        let state = row
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        *kit_balance.entry(state).or_insert(0) += 1;
    }
    let report = placement(tmp.path(), None).expect("placement derives");
    assert_eq!(
        report.balance, kit_balance,
        "native balance diverged from placement-audit.py over the same corpus"
    );
    assert_eq!(
        report.total_files,
        payload
            .get("total_files")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default() as usize
    );
    // And the row-for-row reading, not just the histogram: same path, same state, same order.
    let kit_rows: Vec<(String, String, String)> = payload["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["path"].as_str().unwrap_or_default().to_string(),
                r["position"].as_str().unwrap_or_default().to_string(),
                r["state"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    let native_rows: Vec<(String, String, String)> = report
        .rows
        .iter()
        .map(|r| (r.path.clone(), r.position.clone(), r.state.clone()))
        .collect();
    assert_eq!(native_rows, kit_rows, "row-for-row placement diverged");
}

// ── station-two review round two ────────────────────────────────────────────────────────────────

#[test]
fn an_empty_evidence_list_is_absence_not_evidence() {
    // `verified_by: []` arrives from the native frontmatter parser as the non-empty STRING "[]".
    // Reading that as evidence moved `status: landed` docs out of the pressure queue and out of the
    // over-claim count `delivery-gate.py` steers on — the one number this station rewired.
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    write(root, "genesis/manifests/cluster-state.yaml", CLUSTER_STATE);
    let plans = "genesis/docs/superpowers/plans";
    write(
        root,
        &format!("{plans}/empty-list.md"),
        &doc("landed", "verified_by: []\n"),
    );
    write(
        root,
        &format!("{plans}/empty-spaced.md"),
        &doc("landed", "verified_by: [ ]\n"),
    );
    write(
        root,
        &format!("{plans}/empty-commit.md"),
        &doc("landed", "landed_commit: []\n"),
    );
    write(
        root,
        &format!("{plans}/real.md"),
        &doc("landed", "verified_by: [ci-42]\n"),
    );
    let report = placement(root, None).expect("placement derives");
    let state = |needle: &str| {
        report
            .rows
            .iter()
            .find(|r| r.path.ends_with(needle))
            .map(|r| r.state.clone())
            .unwrap_or_default()
    };
    assert_eq!(state("empty-list.md"), "CLAIMED-ONLY");
    assert_eq!(state("empty-spaced.md"), "CLAIMED-ONLY");
    assert_eq!(state("empty-commit.md"), "CLAIMED-ONLY");
    assert_eq!(state("real.md"), "VERIFIED-STABLE");
}

#[test]
fn an_empty_cites_list_leaves_a_memory_entry_unlinked() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    write(root, "genesis/manifests/cluster-state.yaml", CLUSTER_STATE);
    write(
        root,
        ".claude/memory/empty.md",
        "---\nname: e\ncites: []\n---\n\nBody.\n",
    );
    write(
        root,
        ".claude/memory/real.md",
        "---\nname: r\ncites:\n  - \"a | b | sha256:0000000000000000\"\n---\n\nBody.\n",
    );
    let report = placement(root, None).expect("placement derives");
    let state = |needle: &str| {
        report
            .rows
            .iter()
            .find(|r| r.path.ends_with(needle))
            .map(|r| r.state.clone())
            .unwrap_or_default()
    };
    assert_eq!(state("empty.md"), "MEM-UNLINKED");
    assert_eq!(state("real.md"), "LINKED");
}

#[test]
fn a_block_list_survives_a_blank_line_or_a_comment_inside_it() {
    // Closing the list on an interruption truncates it, and all three keys this reading depends on
    // change a document's state when they lose entries.
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    write(root, "genesis/manifests/cluster-state.yaml", CLUSTER_STATE);
    write(
        root,
        ".claude/memory/interrupted.md",
        "---\nname: i\ncites:\n\n  # why this one matters\n  - \"a | b | sha256:0000000000000000\"\n---\n\nBody.\n",
    );
    let report = placement(root, None).expect("placement derives");
    assert_eq!(
        report
            .rows
            .iter()
            .find(|r| r.path.ends_with("interrupted.md"))
            .map(|r| r.state.as_str()),
        Some("LINKED"),
        "the cite after the blank line and the comment is still part of the list"
    );
}

#[test]
fn a_non_utf8_document_stays_in_the_ledger() {
    // Dropping it would silently change the BALANCE denominator — the opposite of "sums to total,
    // nothing hides".
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    write(root, "genesis/manifests/cluster-state.yaml", CLUSTER_STATE);
    let path = root.join("genesis/docs/superpowers/specs/mojibake.md");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut bytes = b"---\ntitle: m\nstatus: proposed\n---\n\nbody ".to_vec();
    bytes.push(0xFF);
    bytes.push(b'\n');
    std::fs::write(&path, bytes).unwrap();
    let report = placement(root, None).expect("placement derives");
    assert_eq!(report.total_files, 1);
    assert_eq!(report.rows[0].state, "ACTIVE");
}

#[test]
fn the_payload_key_is_camel_case_and_pinned() {
    // The `--json` envelope renames the kit's `total_files` to `totalFiles`, matching every other
    // native payload. Pinned so the rename is a decision rather than an accident a consumer
    // discovers.
    let tmp = corpus();
    let report = placement(tmp.path(), None).expect("placement derives");
    let payload: serde_json::Value = serde_json::to_value(&report).unwrap();
    assert!(payload.get("totalFiles").is_some());
    assert!(payload.get("total_files").is_none());
    // `rows` keys are unchanged, which is what the one live consumer reads.
    let row = &payload["rows"][0];
    for key in ["path", "position", "state", "next"] {
        assert!(row.get(key).is_some(), "row key `{key}` must not move");
    }
}
