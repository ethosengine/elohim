//! Checkbox-station extraction parity: the native reader against `decompose.py`'s recorded output.
//!
//! Station two of `genesis/docs/superpowers/plans/2026-09-10-memory-kit-replacement-finish.md`
//! replaces `.claude/memory-kit/gap-items/` with extraction from the document itself. "Replaces"
//! is only true if the ids and states are the SAME ids and states — a native reader that minted
//! `#1..#7` differently would silently orphan every claim, fulfilment and note already recorded
//! against the kit's spelling.
//!
//! Both halves are FIXTURES, copied on 2026-09-10 from the live tree:
//!
//! - `plans__<slug>.json` — the kit's recorded output, copied because station six deletes the
//!   directory it came from. After that deletion this file is the only surviving witness of what
//!   `decompose.py` said.
//! - `plans__<slug>.md` — the plan bytes that produced it, copied because the plans are live
//!   documents. Reading them from the tree instead would make this an ordinary plan edit's problem:
//!   ticking a station's box would red the eprfs gate while proving nothing about the extractor.
//!
//! Pinning both halves makes this a test of the ALGORITHM, which is what parity means here.

use std::path::PathBuf;

use elohim_epr_cli::flow::gaps::{decompose, GapState, Method};

const FIXTURES: [&str; 3] = [
    "plans__2026-09-09-collective-memory-integration",
    "plans__2026-09-10-agent-provenance-and-collective-affiliations",
    "plans__2026-09-10-memory-kit-replacement-finish",
];

/// The recorded station counts, so a fixture that silently emptied would be caught rather than
/// passing as "zero equals zero".
const EXPECTED_COUNTS: [(&str, usize); 3] = [
    ("plans__2026-09-09-collective-memory-integration", 3),
    (
        "plans__2026-09-10-agent-provenance-and-collective-affiliations",
        4,
    ),
    ("plans__2026-09-10-memory-kit-replacement-finish", 7),
];

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/gap-parity")
}

/// `(id, state)` pairs out of a recorded gap-items file, in file order.
fn recorded(slug: &str) -> (String, Vec<(String, String)>) {
    let path = fixtures_dir().join(format!("{slug}.json"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("fixture {} is unreadable: {e}", path.display()));
    let value: serde_json::Value = serde_json::from_str(&text).expect("gap-items fixture is JSON");
    let method = value
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let items = value
        .get("items")
        .and_then(|v| v.as_array())
        .expect("gap-items fixture has items")
        .iter()
        .map(|item| {
            (
                item.get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                item.get("state")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
            )
        })
        .collect();
    (method, items)
}

fn native(slug: &str) -> (String, Vec<(String, String)>) {
    let path = fixtures_dir().join(format!("{slug}.md"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("fixture {} is unreadable: {e}", path.display()));
    let decomposition = decompose(slug, &text, Vec::new());
    let items = decomposition
        .items
        .iter()
        .map(|g| (g.id.clone(), g.state.as_str().to_string()))
        .collect();
    (decomposition.method.as_str().to_string(), items)
}

#[test]
fn native_ids_and_states_equal_the_recorded_gap_items() {
    for slug in FIXTURES {
        let (kit_method, kit_items) = recorded(slug);
        let (native_method, native_items) = native(slug);
        assert_eq!(
            native_method, kit_method,
            "{slug}: extraction method diverged from the kit's"
        );
        assert_eq!(
            native_items, kit_items,
            "{slug}: native ids/states diverged from the recorded gap-items"
        );
    }
}

#[test]
fn the_recorded_counts_are_what_this_test_is_comparing() {
    // Without this, an emptied fixture on BOTH sides would pass vacuously.
    for (slug, expected) in EXPECTED_COUNTS {
        let (_, kit_items) = recorded(slug);
        let (_, native_items) = native(slug);
        assert_eq!(kit_items.len(), expected, "{slug}: recorded count moved");
        assert_eq!(native_items.len(), expected, "{slug}: native count moved");
    }
}

#[test]
fn every_fixture_decomposed_through_the_checkbox_arm() {
    // The delivery-station shape these plans use is the checkbox arm, never the bullet fallback.
    // If one of them ever fell back, the ids above would still match by luck of numbering while
    // naming entirely different assertions.
    for slug in FIXTURES {
        let path = fixtures_dir().join(format!("{slug}.md"));
        let text = std::fs::read_to_string(&path).expect("fixture readable");
        assert_eq!(
            decompose(slug, &text, Vec::new()).method,
            Method::CheckboxTasks,
            "{slug} must extract through checkbox-tasks"
        );
    }
}

#[test]
fn the_delivery_stations_heading_is_a_convention_not_a_parser_input() {
    // The plan names its stations under `## Delivery stations`; the extractor never reads the
    // heading. Same bytes with the heading removed decompose identically — which is what makes a
    // plan that names its stations differently still decompose.
    let path = fixtures_dir().join("plans__2026-09-10-memory-kit-replacement-finish.md");
    let text = std::fs::read_to_string(&path).expect("fixture readable");
    assert!(text.contains("## Delivery stations"));
    let without: String = text
        .lines()
        .filter(|l| l.trim() != "## Delivery stations")
        .collect::<Vec<_>>()
        .join("\n");
    let a = decompose("plans__x", &text, Vec::new());
    let b = decompose("plans__x", &without, Vec::new());
    assert_eq!(
        a.items.iter().map(|g| &g.text).collect::<Vec<_>>(),
        b.items.iter().map(|g| &g.text).collect::<Vec<_>>()
    );
}

#[test]
fn a_ticked_station_is_claimed_not_done() {
    let path = fixtures_dir().join("plans__2026-09-10-memory-kit-replacement-finish.md");
    let text = std::fs::read_to_string(&path).expect("fixture readable");
    let ticked = text.replacen("- [ ]", "- [x]", 1);
    let d = decompose("plans__x", &ticked, Vec::new());
    assert_eq!(d.items[0].state, GapState::Claimed);
    assert_eq!(d.claimed(), 1);
    assert_eq!(d.open(), d.items.len() - 1);
}
