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

/// The human rendering prints the lens on ONE line, right after the guiding context it bounds —
/// the honesty floor requires the lens be named on every view, not buried in JSON a human
/// rendering never shows.
#[test]
fn the_human_rendering_prints_one_lens_line_after_guiding_context() {
    let dir = repo();
    claim_actor(dir.path(), "agent:reader@claude-opus-5", "lens-d");
    let text = text_in(
        dir.path(),
        "lens-d",
        &["open", "--need", "orient", "--scope", "tooling"],
    );
    let lens_line = text
        .lines()
        .find(|line| line.starts_with("lens: "))
        .unwrap_or_else(|| panic!("no `lens:` line in rendering:\n{text}"));
    assert!(lens_line.contains("standard"), "{lens_line}");
    assert!(lens_line.contains("claude-opus-5"), "{lens_line}");
    let guiding_index = text
        .lines()
        .position(|line| line.starts_with("Guiding context"));
    let lens_index = text.lines().position(|line| line.starts_with("lens: "));
    if let Some(guiding_index) = guiding_index {
        assert!(
            lens_index.unwrap() > guiding_index,
            "lens line must follow the last guiding context line:\n{text}"
        );
    }
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
