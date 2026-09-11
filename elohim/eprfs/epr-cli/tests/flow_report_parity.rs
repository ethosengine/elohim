//! `epr flow report parity --inventory <path>` — the retirement ledger folded against a tree.
//!
//! Station six of `genesis/docs/superpowers/plans/2026-09-10-memory-kit-replacement-finish.md`.
//! Every assertion below is a way the fold could have lied and does not: a prose cell passing
//! itself, a negation's incidental backtick reading as a replacement, a verb's existence standing
//! in for a flag's, a self-reference deadlocking the ledger, a pipe inside code shifting a row's
//! columns.
//!
//! The verb probe is pointed at `CARGO_BIN_EXE_epr` rather than at `current_exe()`, which under
//! `cargo test` is the harness. That indirection is the whole reason `parity_with_binary` exists.

use std::path::Path;

use elohim_epr_cli::flow::parity::parity_with_binary;
use elohim_epr_cli::flow::report::OutcomeStatus;
use eprfs_core::BlobCid;
use tempfile::TempDir;

const EPR: &str = env!("CARGO_BIN_EXE_epr");

fn write(root: &Path, rel: &str, contents: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

/// A tempdir carrying the surfaces the fixture inventory's rows point at.
fn fixture() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(
        root,
        ".claude/scripts/_lib/__tests__/kept_test.py",
        "# test\n",
    );
    write(
        root,
        ".claude/scripts/_lib/__tests__/sibling_test.py",
        "# test\n",
    );
    write(
        root,
        ".epr-meta/elohim/lenses/relocated-lens.py",
        "# lens\n",
    );
    // A live consumer that still executes one kit script, and a kit file that names another.
    write(
        root,
        ".claude/hooks/live-consumer.py",
        "run('.claude/scripts/memory-kit/still-wired.py --status')\n",
    );
    write(
        root,
        ".claude/scripts/memory-kit/placement-audit.py",
        "# imports focus-baseline.py\n",
    );
    // Derived, gitignored run output naming a script that once ran — a record, not a consumer.
    write(
        root,
        "genesis/a2o/reports/cucumber-report.json",
        "{\"cmd\": \"recall-ceremony.py --session x\"}\n",
    );
    // Two DATED records: a shift objective and a timeline entry. Both are tracked, authored
    // statements about a tree that existed on the day they were written.
    write(
        root,
        ".claude/shifts/2026-08-14T02-42-some-shift.objective.json",
        "{\"inputs\": [\".claude/scripts/memory-kit/dated-only.py\"]}\n",
    );
    write(
        root,
        "genesis/data/timeline/backlog/some-entry.yaml",
        "note: .claude/scripts/memory-kit/dated-only.py was the tool that day\n",
    );
    dir
}

/// Rows are written as one table so the fold's table parsing is exercised end to end.
fn inventory(rows: &[&str]) -> String {
    let mut text = String::from(
        "---\nid: fixture\n---\n\n## Scripts\n\n\
         | artifact | capability | consumers | writes | native replacement today | parity | tests |\n\
         |---|---|---|---|---|---|---|\n",
    );
    for row in rows {
        text.push_str(row);
        text.push('\n');
    }
    text
}

fn fold(dir: &TempDir, rows: &[&str]) -> elohim_epr_cli::flow::parity::ParityPayload {
    let text = inventory(rows);
    write(dir.path(), "inv.md", &text);
    parity_with_binary(dir.path(), Path::new("inv.md"), Some(Path::new(EPR)))
        .expect("the fold runs over a table with an artifact column")
}

fn row<'a>(
    payload: &'a elohim_epr_cli::flow::parity::ParityPayload,
    artifact: &str,
) -> &'a elohim_epr_cli::flow::parity::ParityRow {
    payload
        .tables
        .iter()
        .flat_map(|t| &t.rows)
        .find(|r| r.artifact == artifact)
        .unwrap_or_else(|| panic!("no row for {artifact}"))
}

#[test]
fn a_row_passes_only_when_all_three_legs_pass() {
    let dir = fixture();
    let payload = fold(
        &dir,
        &["| `clean.py` | a capability | none | none | `epr flow status` | native | `_lib/__tests__/kept_test.py`, `sibling_test.py` |"],
    );
    let clean = row(&payload, "clean.py");
    assert_eq!(clean.outcome, OutcomeStatus::Passed, "{}", clean.summary);
    assert_eq!(clean.replacement.status, OutcomeStatus::Passed);
    assert_eq!(
        clean.tests.checks.len(),
        2,
        "the in-cell directory carry must resolve the bare sibling token: {:?}",
        clean.tests
    );
    assert_eq!(clean.tests.status, OutcomeStatus::Passed);
    assert_eq!(clean.references.status, OutcomeStatus::Passed);
    assert_eq!(payload.totals.passed, 1);
}

#[test]
fn prose_and_absence_are_skipped_and_never_passed() {
    let dir = fixture();
    let payload = fold(
        &dir,
        &[
            "| `no-tests.py` | c | none | none | `epr flow status` | partial | fixture legs: 40 chars refused, 80 accepted |",
            "| `no-replacement.py` | c | none | none | none | missing | `_lib/__tests__/kept_test.py` |",
            "| `negated.py` | c | none | none | none (`epr flow status`/`epr check` gate writes, not this) | missing | `_lib/__tests__/kept_test.py` |",
        ],
    );
    let prose = row(&payload, "no-tests.py");
    assert_eq!(prose.outcome, OutcomeStatus::Skipped);
    assert_eq!(prose.tests.status, OutcomeStatus::Skipped);
    assert!(
        prose.tests.detail.contains("no checkable test path"),
        "a skipped leg must name what is missing: {}",
        prose.tests.detail
    );

    assert_eq!(
        row(&payload, "no-replacement.py").outcome,
        OutcomeStatus::Skipped
    );

    // The load-bearing one: a cell that says "none" and then mentions a real verb in passing must
    // NOT read as a replacement. This is a live inventory shape (`claude-md-audit.py`).
    let negated = row(&payload, "negated.py");
    assert_eq!(
        negated.replacement.status,
        OutcomeStatus::Skipped,
        "a negation's incidental backtick is prose about what does NOT cover the row: {}",
        negated.replacement.detail
    );
    assert!(negated
        .replacement
        .detail
        .contains("declares no replacement"));
    assert_eq!(payload.totals.passed, 0);
}

#[test]
fn a_retirement_needs_its_reason_and_carries_the_replacement_leg() {
    let dir = fixture();
    let payload = fold(
        &dir,
        &[
            "| `retired.py` | c | none | none | none | **retired 2026-09-10** — script deleted; staleness not ported | `_lib/__tests__/kept_test.py` |",
            "| `bare-retire.py` | c | none | none | none | retire | `_lib/__tests__/kept_test.py` |",
        ],
    );
    let retired = row(&payload, "retired.py");
    assert_eq!(retired.replacement.status, OutcomeStatus::Passed);
    assert_eq!(retired.outcome, OutcomeStatus::Passed);
    assert_eq!(
        retired.declared_parity,
        "retired 2026-09-10 — script deleted; staleness not ported"
    );

    let bare = row(&payload, "bare-retire.py");
    assert_eq!(
        bare.replacement.status,
        OutcomeStatus::Skipped,
        "`retire` with no ground is an assertion, not evidence"
    );
}

#[test]
fn a_provenance_citation_does_not_gate_but_a_procedure_pin_does() {
    // The registry's own contract (`.claude/epr-meta/measures.yaml`): `provenance:` CITES the
    // constant a row replaces and may name a line in a script that is gone — the whole point of
    // lifting a hard-coded constant into a declared row is that the row outlives the script.
    // `procedure:` names the producer that must exist, so a deleted script there is a dangling
    // method pin and a real finding. One registry file, one of each, opposite outcomes.
    let dir = fixture();
    write(
        dir.path(),
        ".claude/epr-meta/measures.yaml",
        concat!(
            "measures:\n",
            "  - id: cited-only\n",
            "    procedure: \".epr-meta/elohim/lenses/relocated-lens.py — the live producer\"\n",
            "    provenance: \".claude/scripts/memory-kit/cited-only.py:49 MIN_CHARS=80\"\n",
            "  - id: pinned-to-a-ghost\n",
            "    procedure: \".claude/scripts/memory-kit/pinned-producer.py — byte length\"\n",
            "    provenance: \"some other note\"\n",
        ),
    );
    let payload = fold(
        &dir,
        &[
            "| `cited-only.py` | c | none | none | `epr flow status` | retire — lifted to a declared row | `_lib/__tests__/kept_test.py` |",
            "| `pinned-producer.py` | c | none | none | `epr flow status` | retire — lifted to a declared row | `_lib/__tests__/kept_test.py` |",
        ],
    );

    let cited = row(&payload, "cited-only.py");
    assert_eq!(
        cited.references.status,
        OutcomeStatus::Passed,
        "a provenance citation is not a consumer: {}",
        cited.references.detail
    );
    assert!(cited.residue.is_empty(), "residue: {:?}", cited.residue);
    assert_eq!(cited.cited_references.len(), 1);
    assert_eq!(
        cited.cited_references[0].path,
        ".claude/epr-meta/measures.yaml"
    );
    assert!(cited.references.detail.contains("1 cited"));
    assert_eq!(cited.outcome, OutcomeStatus::Passed);

    let pinned = row(&payload, "pinned-producer.py");
    assert_eq!(
        pinned.references.status,
        OutcomeStatus::Failed,
        "a procedure naming a deleted script is a dangling method pin"
    );
    assert_eq!(pinned.residue.len(), 1);
    assert!(pinned.residue[0].text.starts_with("procedure:"));
    assert!(pinned.cited_references.is_empty());
    assert_eq!(pinned.outcome, OutcomeStatus::Failed);

    assert!(payload
        .omissions
        .iter()
        .any(|o| o.contains("citedReferences") && o.contains("procedure:")));
}

#[test]
fn a_verbs_existence_does_not_stand_in_for_its_flags() {
    let dir = fixture();
    let payload = fold(
        &dir,
        &[
            "| `real-flag.py` | c | none | none | `epr flow concerns --corrections` | partial | `_lib/__tests__/kept_test.py` |",
            "| `absent-flag.py` | c | none | none | `epr flow concerns --stamp <doc>` | partial | `_lib/__tests__/kept_test.py` |",
            "| `absent-verb.py` | c | none | none | `epr flow nonesuch` | partial | `_lib/__tests__/kept_test.py` |",
        ],
    );
    assert_eq!(
        row(&payload, "real-flag.py").replacement.status,
        OutcomeStatus::Passed
    );

    let absent_flag = row(&payload, "absent-flag.py");
    assert_eq!(
        absent_flag.replacement.status,
        OutcomeStatus::Failed,
        "the chain is real and the flag is not — the row must fail naming the flag"
    );
    assert!(absent_flag.replacement.detail.contains("--stamp"));

    let absent_verb = row(&payload, "absent-verb.py");
    assert_eq!(absent_verb.replacement.status, OutcomeStatus::Failed);
    assert!(absent_verb
        .replacement
        .detail
        .contains("not a verb this binary has"));
}

#[test]
fn a_live_consumer_gates_and_an_intra_kit_reference_does_not() {
    let dir = fixture();
    let payload = fold(
        &dir,
        &[
            "| `still-wired.py` | c | none | none | `epr flow status` | partial | `_lib/__tests__/kept_test.py` |",
            "| `focus-baseline.py` | c | none | none | `epr flow status` | partial | `_lib/__tests__/kept_test.py` |",
        ],
    );
    let wired = row(&payload, "still-wired.py");
    assert_eq!(wired.references.status, OutcomeStatus::Failed);
    assert_eq!(wired.outcome, OutcomeStatus::Failed);
    assert_eq!(wired.residue.len(), 1);
    assert_eq!(wired.residue[0].path, ".claude/hooks/live-consumer.py");
    assert_eq!(wired.residue[0].line, 1);
    assert!(wired.residue[0].text.contains("still-wired.py"));

    // The deadlock guard: the kit citing itself must not block the deletion that removes both
    // ends, or no row can ever reach the state that authorises the act.
    let internal = row(&payload, "focus-baseline.py");
    assert_eq!(internal.references.status, OutcomeStatus::Passed);
    assert_eq!(internal.self_references.len(), 1);
    assert_eq!(
        internal.self_references[0].path,
        ".claude/scripts/memory-kit/placement-audit.py"
    );
    assert_eq!(internal.outcome, OutcomeStatus::Passed);
}

#[test]
fn a_derived_run_receipt_is_not_a_consumer_and_does_not_gate() {
    // The counterpart of the intra-kit deadlock guard. `genesis/a2o/reports/` is gitignored with
    // zero tracked files; a receipt naming a retired script records a past run and cannot be
    // repaired by any act station six can take, so gating on it would make the row unpassable
    // for a reason nobody can address.
    let dir = fixture();
    let payload = fold(
        &dir,
        &["| `recall-ceremony.py` | c | none | none | `epr flow context` | partial | `_lib/__tests__/kept_test.py` |"],
    );
    let row = row(&payload, "recall-ceremony.py");
    assert_eq!(row.references.status, OutcomeStatus::Passed);
    assert!(row.residue.is_empty(), "residue: {:?}", row.residue);
    assert_eq!(row.outcome, OutcomeStatus::Passed);
    assert!(payload
        .omissions
        .iter()
        .any(|o| o.contains("genesis/a2o/reports/")));
}

#[test]
fn a_dated_record_is_evidence_not_a_consumer_and_does_not_gate() {
    // The operator's 2026-09-11 ruling, after this seat surfaced the class rather than deciding it:
    // a shift objective and a timeline entry are statements about a tree that existed. Editing one
    // to clear a gate would be falsifying a record, so gating on them would leave a row unpassable
    // for a reason nobody may act on. `genesis/data/timeline` was already the brief's; `.claude/shifts`
    // is the same kind of thing.
    let dir = fixture();
    let payload = fold(
        &dir,
        &["| `dated-only.py` | c | none | none | `epr flow status` | retire — history, not wiring | `_lib/__tests__/kept_test.py` |"],
    );
    let row = row(&payload, "dated-only.py");
    assert_eq!(
        row.references.status,
        OutcomeStatus::Passed,
        "residue: {:?}",
        row.residue
    );
    assert!(row.residue.is_empty());
    assert_eq!(row.outcome, OutcomeStatus::Passed);
    assert!(payload.scan.excluded.iter().any(|e| e == ".claude/shifts"));
    assert!(payload
        .omissions
        .iter()
        .any(|o| o.contains("Dated records are excluded") && o.contains(".claude/shifts/")));
}

#[test]
fn an_artifact_cell_naming_no_single_path_is_skipped_not_passed() {
    let dir = fixture();
    let payload = fold(
        &dir,
        &["| dated dirs `2026-05-10`…`2026-09-09` (40 dirs) | c | none | none | `epr flow status` | retire — derived, regenerable | `_lib/__tests__/kept_test.py` |"],
    );
    let dated = row(&payload, "dated dirs 2026-05-10…2026-09-09 (40 dirs)");
    assert!(
        dated.kit_tokens.is_empty(),
        "a date is not a path to search for"
    );
    assert_eq!(dated.references.status, OutcomeStatus::Skipped);
    assert_eq!(dated.outcome, OutcomeStatus::Skipped);
    assert!(dated.references.detail.contains("no single kit path"));
}

#[test]
fn the_payload_is_method_pinned_to_the_inventory_bytes() {
    let dir = fixture();
    let rows = ["| `clean.py` | c | none | none | `epr flow status` | native | `_lib/__tests__/kept_test.py` |"];
    let payload = fold(&dir, &rows);
    let bytes = std::fs::read(dir.path().join("inv.md")).unwrap();
    assert_eq!(
        payload.inventory_cid,
        BlobCid::compute_raw(&bytes).to_string(),
        "the fold's method pin IS the inventory bytes' raw CID"
    );
    assert_eq!(payload.inventory_bytes, bytes.len());
    assert!(
        payload.scan.files_scanned > 0,
        "an empty scan is not a clean scan"
    );
    assert!(payload
        .omissions
        .iter()
        .any(|o| o.contains("selfReferences")));
}

#[test]
fn an_inventory_with_no_artifact_table_is_refused_rather_than_read_as_all_clear() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "inv.md", "# no tables here\n\nprose only.\n");
    let err = parity_with_binary(dir.path(), Path::new("inv.md"), Some(Path::new(EPR)))
        .expect_err("an unreadable ledger must refuse, never render zero rows as success");
    assert!(err.to_string().contains("no markdown tables"));
}

#[test]
fn a_pipe_inside_backticks_does_not_shift_the_replacement_cell() {
    let dir = fixture();
    let payload = fold(
        &dir,
        &["| `piped.py` | c | `.claude/settings.json:206` — PostToolUse `Edit|Write` hook | writes | none | missing | `_lib/__tests__/kept_test.py` |"],
    );
    let piped = row(&payload, "piped.py");
    assert_eq!(
        piped.replacement.status,
        OutcomeStatus::Skipped,
        "the replacement cell is `none`; a shifted split would read `missing` or the writes cell"
    );
    assert_eq!(piped.declared_parity, "missing");
}
