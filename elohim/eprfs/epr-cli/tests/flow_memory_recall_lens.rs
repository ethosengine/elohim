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

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Task 1.2: `Render` per lens with the honesty and content floors
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// `minimal` collapses the orientation to two lines (intent, worthwhile finish) and folds the
/// honesty floor's five fields — recipe CID, lens CID, selection rule, omissions, receipts —
/// onto ONE line (rule 3: never spelled out as five, never dropped), then hands exactly
/// `choice_count` (1, at `minimal`) Linked choices: a scaffold that hands one command, not a
/// menu. `--scope tooling` matches no declared source root here, so this exercises the
/// whole-scope door (no `first_screen`) — the leaner of the two doors, and the harder one to
/// keep under budget since nothing bounds "Linked choices" but the lens itself.
#[test]
fn minimal_prints_the_five_floor_fields_on_one_line_and_at_most_one_choice() {
    let dir = repo();
    claim_actor(dir.path(), "agent:reader@claude-haiku-4-5", "lens-d");
    let text = text_in(
        dir.path(),
        "lens-d",
        &[
            "open",
            "--need",
            "which command rebuilds the stale index",
            "--scope",
            "tooling",
        ],
    );
    assert!(text.len() < 1600, "{}: {text}", text.len());
    assert!(
        text.contains("recipe bafk")
            && text.contains("lens bafk")
            && text.contains("selection:")
            && text.contains("omissions:")
            && text.contains("receipts:"),
        "{text}"
    );
    assert_eq!(
        text.matches("\n  epr flow memory recall ").count(),
        1,
        "one handed command at minimal:\n{text}"
    );
}

/// A `content_class: correction` candidate is UNFILTERABLE — the anti-bubble content floor — and
/// renders at every lens, from `minimal`'s one-command budget to `detail`'s full candidate list.
/// (The brief's own fixture named this file `tooling/correction.md`; `tooling/` is outside this
/// fixture's declared `source_roots` (`["docs"]`, see `common::contract_value`), which would
/// leave it undiscoverable by either lens rather than proving the floor — so it is written under
/// `docs/`, the fixture's one declared source root, with the scope named to match.)
#[test]
fn a_correction_candidate_renders_at_every_lens() {
    let dir = repo();
    // Two ordinary candidates that outrank the correction on term coverage (both "stale" and
    // "index" DECLARED, worth 2x a body hit) — the correction only declares "stale" and carries
    // "index" in its body only, so it scores strictly lower and ranks past `minimal`'s
    // `choice_count` of 1 (fix round 1's scenario: a floor candidate ranked past the cut, not
    // merely a lone candidate that would have been kept by rank alone).
    for name in ["rank1", "rank2"] {
        write(
            dir.path(),
            &format!("docs/{name}.md"),
            &format!(
                "---\ntitle: {name}\ndescription: stale index leader\n---\nordinary candidate\n"
            ),
        );
    }
    write(
        dir.path(),
        "docs/correction.md",
        "---\ntitle: correction\ndescription: correction of the stale claim\ncontent_class: correction\n---\nthe index command changed\n",
    );
    for (model, level) in [
        ("claude-haiku-4-5", "minimal"),
        ("claude-fable-5-1", "detail"),
    ] {
        let session = format!("lens-e-{level}");
        claim_actor(dir.path(), &format!("agent:reader@{model}"), &session);
        let text = text_in(
            dir.path(),
            &session,
            &["open", "--need", "stale index", "--scope", "docs"],
        );
        assert!(text.contains("docs/correction.md"), "{level}: {text}");
        if level == "minimal" {
            // Fix round 1: at `minimal` (`choice_count` 1) the correction ranks past the cut and
            // is kept only by the content floor — the anti-capture invariant requires its command
            // be one command away too, and a candidate the floor did NOT keep (`rank2.md`, which
            // outranks the correction but still falls outside `choice_count`) must never leak a
            // command into the list a reader is actually offered.
            let linked = text.split("Linked choices:").nth(1).unwrap_or_default();
            assert!(
                linked.contains("correction.md"),
                "a floor candidate ranked past choice_count must still get a Linked choice:\n{text}"
            );
            assert!(
                !linked.contains("rank2.md"),
                "a candidate the content floor did not keep must never appear as a Linked choice:\n{text}"
            );
        }
    }
}

/// The red-team's "standing is a shape, not a score" invariant, made a test: no rendered view —
/// at any lens — ever prints a numeric `standing`/`score` token. `render()` prints WHERE a term
/// hit and HOW OFTEN (a line range, a per-term hit count); it never sums a candidate's provenance
/// into a ranking number a reader could mistake for authority.
#[test]
fn no_rendered_view_prints_a_numeric_standing_or_score_token() {
    let dir = repo();
    for (model, level) in [
        ("claude-haiku-4-5", "minimal"),
        ("claude-sonnet-5", "simple"),
        ("claude-opus-5", "standard"),
        ("claude-fable-5-1", "detail"),
    ] {
        let session = format!("print-never-sum-{level}");
        claim_actor(dir.path(), &format!("agent:reader@{model}"), &session);
        let text = text_in(
            dir.path(),
            &session,
            &[
                "open",
                "--need",
                "which command rebuilds the stale index",
                "--scope",
                "docs",
            ],
        );
        assert!(
            !contains_numeric_field(&text, "standing"),
            "{level}: rendered a numeric `standing` token:\n{text}"
        );
        assert!(
            !contains_numeric_field(&text, "score"),
            "{level}: rendered a numeric `score` token:\n{text}"
        );
    }
}

/// A tiny stand-in for `regex::is_match(&format!("{name}[:=]\\s*\\d"))` — this workspace declares
/// no `regex` dependency, and the check is narrow enough not to need one.
///
/// Fix round 1: the original version matched `name[:=]\s*\d` only, which missed a quoted
/// JSON-shaped key (`"match_score": 3`, the generic key-dump path's `serde_json::
/// to_string_pretty` fallback for an unhandled object value) — the closing quote sat where a
/// `:`/`=` was expected, so the scan moved past without matching. Also matches `"name":\s*\d` now
/// (skips one closing `"` immediately after `name`, if present, before the `:`/`=` check).
fn contains_numeric_field(text: &str, name: &str) -> bool {
    let bytes = text.as_bytes();
    let mut start = 0;
    while let Some(offset) = text[start..].find(name) {
        let mut cursor = start + offset + name.len();
        if bytes.get(cursor) == Some(&b'"') {
            cursor += 1;
        }
        if matches!(bytes.get(cursor), Some(b':') | Some(b'=')) {
            cursor += 1;
            while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
                cursor += 1;
            }
            if bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
                return true;
            }
        }
        start += offset + 1;
        if start >= text.len() {
            break;
        }
    }
    false
}

/// The lens's `density_bytes` is a SEPARATE soft cap from `choice_count` and the content floor:
/// even a candidate the choice-count cut would have kept can still be dropped once the rendered
/// candidate block would exceed the byte budget — but a `[floor]`-marked candidate never is, and
/// the block names exactly how many more exist at a wider lens. `simple`'s `density_bytes` is
/// overridden to an unrealistic `1` so the drop is deterministic to assert on, rather than
/// depending on the exact byte length of a rendered line.
#[test]
fn density_drops_ordinary_candidates_but_never_a_floor_one() {
    let dir = repo();
    let mut contract = contract_value(false);
    contract["lens_table"]["levels"]["simple"]["density_bytes"] = json!(1);
    // `discovery.rs`'s own `limits.search_results` (3, live contract) already caps how many
    // candidates a question ever SEES before a lens gets a say — raised here so six equally
    // scored candidates all reach `first_screen.candidates`, and the truncation this test
    // actually asserts on is the lens's, not discovery's.
    contract["limits"]["search_results"] = json!(10);
    save_contract(dir.path(), contract);
    for name in ["w1", "w2", "w3", "w4", "w5"] {
        write(
            dir.path(),
            &format!("docs/{name}.md"),
            &format!(
                "---\ntitle: widget {name}\ndescription: widget candidate\n---\nwidget body\n"
            ),
        );
    }
    // Alphabetically last among the seven equally-scored candidates (path is the final tie-break
    // in `discover_scored`), so it lands past `simple`'s `choice_count` of 3 and only the content
    // floor — never an accident of rank — is why it still renders.
    write(
        dir.path(),
        "docs/zzz-correction.md",
        "---\ntitle: widget correction\ndescription: widget candidate\ncontent_class: correction\n---\nwidget body\n",
    );
    claim_actor(dir.path(), "agent:reader@claude-sonnet-5", "lens-density");
    let text = text_in(
        dir.path(),
        "lens-density",
        &["open", "--need", "widget", "--scope", "docs"],
    );
    let candidates_section = text
        .split("Candidate sources:")
        .nth(1)
        .and_then(|rest| rest.split("\nLinked choices:").next())
        .unwrap_or_default();
    assert!(
        candidates_section.contains("docs/zzz-correction.md")
            && candidates_section.contains("[floor]"),
        "the floor candidate must still render even under a 1-byte density cap:\n{candidates_section}"
    );
    assert!(
        !candidates_section.contains("docs/w1.md"),
        "an ordinary candidate within choice_count must still be dropped by the density cap:\n{candidates_section}"
    );
    assert!(
        candidates_section.contains("more candidate(s) at a wider lens (--lens standard)"),
        "{candidates_section}"
    );
}
