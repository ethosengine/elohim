//! Governed-discovery station 1: the reader lens. Every view resolves and prints WHO is
//! reading — a stated tier from the actor sidecar's claim, mapped by the recipe's declared
//! `lens_table`, widened on an explicit `--lens <level>` request or narrowed/widened by this
//! reader's own revealed journey evidence — and the lens is content-addressed so a reader can
//! tell on sight whether two views were rendered under the same one.
mod common;
use common::*;
use serde_json::json;

/// A claimed `sonnet` reader resolves to the recipe's declared `simple` tier, with its stated
/// provenance naming the model and a content-addressed lens CID.
#[test]
fn a_sonnet_claim_resolves_to_simple_with_provenance_printed() {
    let dir = repo();
    claim_actor(dir.path(), "agent:reader@claude-sonnet-5", "lens-a");
    let v = view_in(
        dir.path(),
        "lens-a",
        &[
            "open",
            "--need",
            "which command rebuilds the stale index",
            "--scope",
            "tooling",
        ],
    );
    assert_eq!(v["lens"]["level"], "simple", "{v}");
    assert!(
        v["lens"]["provenance"]["stated"][0]
            .as_str()
            .unwrap()
            .contains("claude-sonnet-5"),
        "{v}"
    );
    assert!(
        v["lens"]["cid"].as_str().unwrap().starts_with("bafk"),
        "{v}"
    );
}

/// A session with no registered claim resolves to the recipe's declared defaults, and the
/// provenance says so honestly — `"none"`, not a guessed tier.
#[test]
fn an_unknown_reader_gets_the_defaults_labelled_stated_none() {
    let dir = repo();
    let v = view_in(dir.path(), "lens-b", &["open", "--need", "orient"]);
    assert_eq!(v["lens"]["level"], "standard", "{v}");
    assert_eq!(v["lens"]["provenance"]["stated"], json!(["none"]), "{v}");
}

/// `--lens` widens or narrows on request and is NEVER refused, even past what the reader's own
/// stated tier would offer — and the override is named, not silently substituted.
#[test]
fn a_requested_wider_lens_is_never_refused_and_is_recorded_as_requested() {
    let dir = repo();
    claim_actor(dir.path(), "agent:reader@claude-haiku-4-5", "lens-c");
    let v = view_in(
        dir.path(),
        "lens-c",
        &["open", "--need", "orient", "--lens", "detail"],
    );
    assert_eq!(v["lens"]["level"], "detail", "{v}");
    assert_eq!(
        v["lens"]["provenance"]["stated"][0],
        "requested: detail (stated claude-haiku-4-5 → minimal)",
        "{v}"
    );
}

/// The human rendering prints the lens on EXACTLY one line, right after the guiding context it
/// bounds — the honesty floor requires the lens be named on every view, not buried in JSON a
/// human rendering never shows, and not scattered across the generic key dump either.
#[test]
fn the_human_rendering_prints_exactly_one_lens_line_after_guiding_context() {
    let dir = repo();
    claim_actor(dir.path(), "agent:reader@claude-opus-5", "lens-d");
    let text = text_in(
        dir.path(),
        "lens-d",
        &["open", "--need", "orient", "--scope", "tooling"],
    );
    let lens_lines: Vec<&str> = text
        .lines()
        .filter(|line| line.starts_with("lens: "))
        .collect();
    assert_eq!(
        lens_lines.len(),
        1,
        "expected exactly one `lens:` line:\n{text}"
    );
    let lens_line = lens_lines[0];
    assert!(lens_line.contains("standard"), "{lens_line}");
    assert!(lens_line.contains("claude-opus-5"), "{lens_line}");
    assert!(lens_line.contains("cid bafk"), "{lens_line}");
    assert!(lens_line.contains("renew: none"), "{lens_line}");

    let guiding_index = text
        .lines()
        .position(|line| line.starts_with("Guiding context"))
        .unwrap_or_else(|| panic!("no `Guiding context` line in rendering:\n{text}"));
    let lens_index = text
        .lines()
        .position(|line| line.starts_with("lens: "))
        .expect("already asserted exactly one lens line exists above");
    assert!(
        lens_index > guiding_index,
        "lens line must follow the last guiding context line:\n{text}"
    );
}

/// An agent claimed by a model the recipe's `lens_table.stated` does not name falls back to the
/// recipe default honestly — never a refusal, never a fabricated tier.
#[test]
fn an_unlisted_model_falls_back_to_the_recipe_default() {
    let dir = repo();
    claim_actor(dir.path(), "agent:reader@some-future-model-9", "lens-e");
    let v = view_in(dir.path(), "lens-e", &["open", "--need", "orient"]);
    assert_eq!(v["lens"]["level"], "standard", "{v}");
    let stated = v["lens"]["provenance"]["stated"][0].as_str().unwrap();
    assert!(stated.contains("some-future-model-9"), "{stated}");
    assert!(stated.contains("recipe default"), "{stated}");
}

/// A DECLARED-but-malformed `lens_table` must never refuse (a lens is orientation, not an
/// admission gate) but must never be silent either — falls back to the builtin table, names the
/// malformed reason in `view["unresolved"]`, and marks `provenance.defaults` so the fallback
/// reads as forced rather than as an ordinary declared default. Fix round 1, finding 1.
#[test]
fn a_malformed_declared_lens_table_falls_back_and_is_named_unresolved() {
    let dir = repo();
    let mut contract = contract_value(false);
    contract["lens_table"] = json!({"levels": "nope"});
    save_contract(dir.path(), contract);
    // Not `view_in`: the pushed `unresolved` entry means this view legitimately exits non-zero.
    let result = run_in(dir.path(), "lens-f", &["open", "--need", "orient"]);
    let v = result.json();
    assert_eq!(v["lens"]["level"], "standard", "{v}");
    assert_eq!(
        v["lens"]["provenance"]["defaults"], "builtin (declared lens_table malformed)",
        "{v}"
    );
    let unresolved: Vec<&str> = v["unresolved"]
        .as_array()
        .expect("unresolved array")
        .iter()
        .filter_map(|item| item.as_str())
        .collect();
    assert!(
        unresolved
            .iter()
            .any(|line| line.contains("lens_table declared but malformed")
                && line.contains("builtin defaults applied")),
        "{v}"
    );
}

/// A DECLARED-and-VALID `lens_table` wins over the builtin table — proven by a stated mapping
/// the builtin does not carry (`claude-sonnet-5 -> detail`, not the builtin's `simple`), so a
/// bug that silently fell through to `builtin_table()` would be caught here. Fix round 1,
/// finding 2.
#[test]
fn a_declared_lens_table_that_parses_wins_over_the_builtin() {
    let dir = repo();
    let mut contract = contract_value(false);
    contract["lens_table"]["stated"]["claude-sonnet-5"] = json!("detail");
    save_contract(dir.path(), contract);
    claim_actor(dir.path(), "agent:reader@claude-sonnet-5", "lens-g");
    let v = view_in(dir.path(), "lens-g", &["open", "--need", "orient"]);
    assert_eq!(v["lens"]["level"], "detail", "{v}");
    assert_eq!(v["lens"]["provenance"]["defaults"], "declared", "{v}");
}

/// An unrecognized `--lens` level refuses (a level that does not exist is not a wider lens) and
/// the refusal names the whole accepted set, not just "invalid". Fix round 1, finding 4.
#[test]
fn an_unrecognized_lens_level_refuses_and_names_the_six_accepted() {
    let dir = repo();
    let result = run_in(
        dir.path(),
        "lens-h",
        &["open", "--need", "orient", "--lens", "frobnicate"],
    );
    assert_eq!(result.code, 2, "{}{}", result.stdout, result.stderr);
    for level in ["minimal", "simple", "standard", "detail", "debug", "trace"] {
        assert!(
            result.stdout.contains(level),
            "refusal must name `{level}`: {}",
            result.stdout
        );
    }
}
