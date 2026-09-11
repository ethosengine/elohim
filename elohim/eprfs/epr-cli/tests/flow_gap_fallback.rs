//! The gap-items FALLBACK STORE — its native home, its precedence, and the one-shot adoption.
//!
//! Station two of `genesis/docs/superpowers/plans/2026-09-10-memory-kit-replacement-finish.md`,
//! round two. The station's headline claim is that stations come from the documents; the honest
//! qualifier the review extracted is that they do not come from ALL of them. Measured on the live
//! tree on 2026-09-10: of 241 gap-items records with stations, **61 documents / 649 stations** were
//! produced by an agent-supplied method (`agent-decomposition`, `agent`, `agent-extracted`,
//! `delivery-stations`) against a document that still exists. Those stations exist nowhere in their
//! documents' own bytes — a prose spec a human or an agent decomposed by hand has no checkboxes to
//! read — and they survive only through the "the document did not speak" fallback.
//!
//! So the cache directory is a store to RELOCATE, not one to delete. `.eprfs/status/gap-items/` is
//! its native home; the kit path is read second and only for names the native home does not hold.
//! After adoption the kit copy is inert while still on disk, which is what makes the deletion
//! station's job a removal of something nothing reads.

use std::path::Path;

use elohim_epr_cli::flow::project::{adopt_gap_items, fallback_gap_files, FALLBACK_GAP_DIR};
use tempfile::TempDir;

const KIT_DIR: &str = ".claude/memory-kit/gap-items";

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

/// A hand-decomposed record: the shape whose stations exist only in the cache.
fn agent_record(doc: &str, items: usize) -> String {
    let items: Vec<String> = (1..=items)
        .map(|n| format!("{{\"id\":\"x#{n}\",\"state\":\"OPEN\",\"text\":\"station {n}\"}}"))
        .collect();
    format!(
        "{{\"doc\":\"{doc}\",\"slug\":\"x\",\"method\":\"agent-decomposition\",\"items\":[{}]}}",
        items.join(",")
    )
}

fn fixture() -> TempDir {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    // A prose spec: no checkboxes, so its stations live only in the cache.
    write(
        root,
        "genesis/docs/superpowers/specs/prose.md",
        "---\nid: prose\n---\n\n# Prose\n\nNo machine-extractable structure at all.\n",
    );
    write(
        root,
        &format!("{KIT_DIR}/specs__prose.json"),
        &agent_record("genesis/docs/superpowers/specs/prose.md", 3),
    );
    // An orphan: its document is gone.
    write(
        root,
        &format!("{KIT_DIR}/specs__vanished.json"),
        &agent_record("genesis/docs/superpowers/specs/vanished.md", 5),
    );
    // Not a gap-items record at all.
    write(root, &format!("{KIT_DIR}/notes.json"), "[1, 2, 3]\n");
    tmp
}

#[test]
fn adoption_copies_only_records_whose_document_still_exists() {
    let tmp = fixture();
    let root = tmp.path();
    let summary = adopt_gap_items(root, &root.join(KIT_DIR)).expect("adoption runs");

    assert_eq!(summary.scanned, 3);
    assert_eq!(summary.adopted.len(), 1);
    assert_eq!(summary.adopted[0].name, "specs__prose.json");
    assert_eq!(
        summary.adopted[0].doc,
        "genesis/docs/superpowers/specs/prose.md"
    );
    assert_eq!(summary.adopted[0].items, 3);
    assert_eq!(summary.orphans, vec!["specs__vanished.json".to_string()]);
    assert_eq!(summary.unreadable, vec!["notes.json".to_string()]);
    assert!(summary.already_present.is_empty());

    // The orphan is LEFT WHERE IT IS. Its stations describe work on a document that no longer
    // exists; carrying them forward would keep a ledger of promises about nothing.
    assert!(!root
        .join(FALLBACK_GAP_DIR)
        .join("specs__vanished.json")
        .exists());
}

#[test]
fn the_copy_is_byte_identical_so_its_cid_addresses_both_files() {
    let tmp = fixture();
    let root = tmp.path();
    let summary = adopt_gap_items(root, &root.join(KIT_DIR)).expect("adoption runs");

    let source = std::fs::read(root.join(KIT_DIR).join("specs__prose.json")).unwrap();
    let copy = std::fs::read(root.join(FALLBACK_GAP_DIR).join("specs__prose.json")).unwrap();
    assert_eq!(
        source, copy,
        "the copy must be byte-for-byte, or the recorded CID addresses only one of the two"
    );
    // The recorded CID is the address of those very bytes — the adoption is verifiable, not asserted.
    let recomputed = eprfs_core::BlobCid::compute_raw(&copy).to_string();
    assert_eq!(summary.adopted[0].cid, recomputed);
}

#[test]
fn adoption_is_idempotent() {
    let tmp = fixture();
    let root = tmp.path();
    adopt_gap_items(root, &root.join(KIT_DIR)).expect("first run");
    let second = adopt_gap_items(root, &root.join(KIT_DIR)).expect("second run");
    assert!(second.adopted.is_empty());
    assert_eq!(
        second.already_present,
        vec!["specs__prose.json".to_string()]
    );
    // Still exactly one file in the native home.
    let count = std::fs::read_dir(root.join(FALLBACK_GAP_DIR))
        .unwrap()
        .count();
    assert_eq!(count, 1);
}

#[test]
fn the_native_home_is_read_first_and_the_kit_path_second() {
    let tmp = fixture();
    let root = tmp.path();
    let recipe_paths = vec![format!("{KIT_DIR}/*.json")];

    // Before adoption: everything comes from the kit path.
    let before = fallback_gap_files(root, &recipe_paths);
    assert_eq!(before.len(), 3);
    assert!(before.iter().all(|p| p.starts_with(root.join(KIT_DIR))));

    adopt_gap_items(root, &root.join(KIT_DIR)).expect("adoption runs");

    // After adoption: the adopted name is read from the NATIVE home, once — not twice, and not from
    // the kit copy that is still on disk. That is what makes the old directory inert.
    let after = fallback_gap_files(root, &recipe_paths);
    assert_eq!(after.len(), 3, "one entry per NAME, never one per copy");
    let adopted = after
        .iter()
        .find(|p| p.file_name().unwrap() == "specs__prose.json")
        .expect("the adopted name is present");
    assert!(
        adopted.starts_with(root.join(FALLBACK_GAP_DIR)),
        "the adopted record must be read from its new home"
    );
    // The names the native home does not hold still come from the kit path.
    let orphan = after
        .iter()
        .find(|p| p.file_name().unwrap() == "specs__vanished.json")
        .expect("the un-adopted name is still reachable");
    assert!(orphan.starts_with(root.join(KIT_DIR)));
}

#[test]
fn adoption_refuses_a_directory_that_is_not_there() {
    let tmp = TempDir::new().unwrap();
    let err = adopt_gap_items(tmp.path(), &tmp.path().join("nowhere")).unwrap_err();
    assert!(format!("{err}").contains("--adopt-gap-items names no directory"));
}

#[test]
fn a_repository_with_no_fallback_store_at_all_yields_nothing() {
    // Honest absence, not an error: the whole point of the relocation is that the store's presence
    // is optional once every document speaks for itself.
    let tmp = TempDir::new().unwrap();
    assert!(fallback_gap_files(tmp.path(), &[]).is_empty());
}

// ── durability: the store is TRACKED, every other sidecar is not ────────────────────────────────

/// The three `git check-ignore` invariants the relocated store depends on.
///
/// `(path, must_be_ignored)`. The store is only a relocation if it is durable, and it is only safe
/// if making it durable did not un-ignore everything else named `.eprfs`.
const IGNORE_INVARIANTS: [(&str, bool); 3] = [
    // A NESTED sidecar stays ignored. `.eprfs/*` looks unanchored and is not — a pattern with an
    // interior slash anchors to the .gitignore's directory, so the first version of this block
    // silently un-ignored every nested sidecar in the repository. `**/.eprfs/` restores the
    // unanchored reading.
    ("elohim/elohim-storage/.eprfs/status/flows.jsonl", true),
    // The ROOT sidecar's derived records stay ignored: `epr flow project` reconstructs them.
    (".eprfs/status/flows.jsonl", true),
    // The gap-items store does NOT: 61 documents / 649 stations in it exist in no document's bytes
    // and no projection can rebuild them.
    (
        ".eprfs/status/gap-items/architecture__2026-06-04-qahal-epr-household-lattice-design.json",
        false,
    ),
];

#[test]
fn the_relocated_store_is_tracked_and_every_other_sidecar_is_not() {
    let Some(repo) = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(3) else {
        eprintln!("SKIP: could not resolve the repository root from CARGO_MANIFEST_DIR");
        return;
    };
    if !repo.join(".gitignore").is_file() {
        eprintln!("SKIP: {} carries no .gitignore", repo.display());
        return;
    }
    for (rel, must_be_ignored) in IGNORE_INVARIANTS {
        // `check-ignore` answers about a PATH, not about a file, so a path that does not exist is
        // still a legitimate question — which is what keeps this test honest on a tree where the
        // store has not been adopted yet.
        let out = match std::process::Command::new("git")
            .args(["check-ignore", "-q", rel])
            .current_dir(repo)
            .status()
        {
            Ok(status) => status,
            Err(err) => {
                eprintln!("SKIP: git is not runnable here ({err})");
                return;
            }
        };
        // git check-ignore -q: 0 = ignored, 1 = not ignored, >1 = error.
        let code = out.code().unwrap_or(2);
        assert!(
            code < 2,
            "git check-ignore errored on `{rel}` (exit {code})"
        );
        let ignored = code == 0;
        assert_eq!(
            ignored,
            must_be_ignored,
            "`{rel}` must {} ignored",
            if must_be_ignored { "be" } else { "NOT be" }
        );
    }
}
