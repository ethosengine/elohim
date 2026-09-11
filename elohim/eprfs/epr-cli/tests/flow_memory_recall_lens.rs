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

// ── governed-discovery station 2.1: `open --purpose bootstrap` ────────────────────────────────────

/// `--purpose bootstrap` with no `--need`: the session's intent is minted from the register's own
/// top red habit — `<id>: <first check>` — never a term match, and the view carries the
/// `ProjectionRequest`-shaped envelope (`purpose`/`audience`/`inputs`/`omissions`) around it.
#[test]
fn bootstrap_purpose_carries_the_top_red_as_intent_and_declares_its_inputs() {
    let dir = repo();
    write(
        dir.path(),
        "genesis/manifests/habits.yaml",
        "habits:\n- id: alpha\n  status: red\n  active: true\n  checks: ['a2o @concern:alpha']\n  invariant: alpha holds\n",
    );
    let v = view_in(dir.path(), "boot", &["open", "--purpose", "bootstrap"]);
    assert!(v["orientation"]["intent"]
        .as_str()
        .unwrap()
        .starts_with("alpha"));
    assert_eq!(v["projection"]["purpose"], "bootstrap");
    assert_eq!(v["projection"]["audience"], "private");
    assert!(!v["projection"]["inputs"].as_array().unwrap().is_empty());
}

// ── station 2.1, fix round 1 (controller review) ───────────────────────────────────────────────────

/// The floor line's own `omissions: N` field: extracts the integer following the label, so an
/// assertion on it survives the label text itself changing.
fn floor_omissions_count(text: &str) -> usize {
    let line = text
        .lines()
        .find(|line| line.contains("omissions:"))
        .unwrap_or_else(|| panic!("no floor line in rendering:\n{text}"));
    line.split("omissions:")
        .nth(1)
        .and_then(|rest| rest.trim().split(|c: char| !c.is_ascii_digit()).next())
        .and_then(|digits| digits.parse().ok())
        .unwrap_or_else(|| panic!("floor line has no parseable omissions count: {line}"))
}

/// Finding 1: `projection.omissions` were carried in `--json` only — invisible in the human
/// rendering, and not counted in the floor line's own `omissions: N`. An absent `flows.jsonl`
/// (removed here even though `repo()`'s fixture edges create one by default) now renders a bullet
/// naming it directly under the `Bootstrap:` line, and the floor's `omissions:` count grows to
/// include it.
#[test]
fn an_absent_flows_sidecar_renders_a_bullet_and_counts_on_the_floor() {
    let dir = repo();
    write(
        dir.path(),
        "genesis/manifests/habits.yaml",
        "habits:\n- id: alpha\n  status: red\n  active: true\n  checks: ['a2o @concern:alpha']\n  invariant: alpha holds\n",
    );
    let flows_path = dir.path().join(".eprfs/status/flows.jsonl");
    if flows_path.exists() {
        std::fs::remove_file(&flows_path).expect("remove flows sidecar");
    }
    let text = text_in(dir.path(), "boot-omit", &["open", "--purpose", "bootstrap"]);
    assert!(
        text.contains("· .eprfs/status/flows.jsonl is absent"),
        "no bullet naming the absent flows sidecar:\n{text}"
    );
    assert!(
        floor_omissions_count(&text) >= 1,
        "floor omissions must count the projection's own omission:\n{text}"
    );
}

/// Finding 2: a malformed/unparseable `habits.yaml` previously collapsed to the SAME `no red
/// habit; orient` an honest all-green register earns — fail-closed now: bootstrap REFUSES rather
/// than orienting a reader from a register it could not actually read, naming the path and
/// pointing at the projector.
#[test]
fn a_malformed_habit_register_refuses_rather_than_orienting() {
    let dir = repo();
    write(
        dir.path(),
        "genesis/manifests/habits.yaml",
        "habits:\n  - id: [this is not valid yaml\n",
    );
    let result = run_in(
        dir.path(),
        "boot-malformed",
        &["open", "--purpose", "bootstrap"],
    );
    assert_eq!(result.code, 2, "{}{}", result.stdout, result.stderr);
    let failure = result.json();
    let message = failure["unresolved"][0].as_str().unwrap_or_default();
    assert!(
        message.contains("cannot read the habit register")
            && message.contains("genesis/manifests/habits.yaml"),
        "{failure}"
    );
    let next = failure["next"].as_str().unwrap_or_default();
    assert!(
        next.contains("habits-project.py"),
        "next: must point at the projector: {next}"
    );
}

/// Finding 2, the other half: a register that PARSES fine and genuinely carries no red habit is
/// not a fault — it keeps the honest `no red habit; orient` intent with its own omission, never
/// the refusal a broken register now earns.
#[test]
fn an_all_green_register_still_orients_honestly() {
    let dir = repo();
    write(
        dir.path(),
        "genesis/manifests/habits.yaml",
        "habits:\n- id: alpha\n  status: green\n  active: false\n  checks: ['a2o @concern:alpha']\n  invariant: alpha holds\n",
    );
    let v = view_in(
        dir.path(),
        "boot-green",
        &["open", "--purpose", "bootstrap"],
    );
    assert_eq!(v["orientation"]["intent"], "no red habit; orient", "{v}");
    let omissions: Vec<&str> = v["projection"]["omissions"]
        .as_array()
        .expect("omissions array")
        .iter()
        .filter_map(|item| item.as_str())
        .collect();
    assert!(omissions.contains(&"no red habit; orient"), "{v}");
}

// ── fix round 2: an operation's own result renders at every lens, not just standard+ ──────────────
//
// A fresh reader found `read`, `source` and `history` printing NO primary payload at `simple`/
// `minimal` — only the collapsed orientation, floor line and Linked-choices boilerplate — so the
// reader had to guess `--lens detail` to see the passage it had just read. Ruling: density bounds
// candidate LISTS and secondary blocks only; an operation's own result renders at every lens, and
// so does the `lens:` provenance line (the floor line's compact `lens <cid>` token is a
// cross-reference to it, never a replacement).

/// `read`'s excerpt text and receipt key render at `simple`, not only at a wider lens.
#[test]
fn read_renders_its_excerpt_and_receipt_key_at_simple() {
    let dir = repo();
    claim_actor(dir.path(), "agent:reader@claude-sonnet-5", "fix2-read");
    view_in(dir.path(), "fix2-read", &["open", "--need", "orient"]);
    let text = text_in(
        dir.path(),
        "fix2-read",
        &["read", "--path", "docs/a.md", "--lines", "1:3"],
    );
    assert!(text.contains("title: Preserve uncertainty"), "{text}");
    assert!(text.contains("docs/a.md:1:3"), "{text}");
}

/// `source`'s outline — at least one heading — renders at `simple`, not only at a wider lens.
#[test]
fn source_renders_at_least_one_outline_heading_at_simple() {
    let dir = repo();
    claim_actor(dir.path(), "agent:reader@claude-sonnet-5", "fix2-source");
    view_in(dir.path(), "fix2-source", &["open", "--need", "orient"]);
    let text = text_in(
        dir.path(),
        "fix2-source",
        &["source", "--path", "docs/a.md"],
    );
    assert!(
        text.contains("Purpose") || text.contains("Evidence"),
        "{text}"
    );
}

/// The `lens:` provenance line renders EXACTLY once on `open`, `read` and `source` at `simple` —
/// never zero (the pre-fix bug) and never duplicated by folding it into the generic key dump.
#[test]
fn the_lens_line_renders_exactly_once_at_simple_for_open_read_and_source() {
    let dir = repo();

    claim_actor(dir.path(), "agent:reader@claude-sonnet-5", "fix2-lens-open");
    let open_text = text_in(dir.path(), "fix2-lens-open", &["open", "--need", "orient"]);
    assert_eq!(
        open_text
            .lines()
            .filter(|l| l.starts_with("lens: "))
            .count(),
        1,
        "{open_text}"
    );

    claim_actor(dir.path(), "agent:reader@claude-sonnet-5", "fix2-lens-read");
    view_in(dir.path(), "fix2-lens-read", &["open", "--need", "orient"]);
    let read_text = text_in(
        dir.path(),
        "fix2-lens-read",
        &["read", "--path", "docs/a.md", "--lines", "1:3"],
    );
    assert_eq!(
        read_text
            .lines()
            .filter(|l| l.starts_with("lens: "))
            .count(),
        1,
        "{read_text}"
    );

    claim_actor(
        dir.path(),
        "agent:reader@claude-sonnet-5",
        "fix2-lens-source",
    );
    view_in(
        dir.path(),
        "fix2-lens-source",
        &["open", "--need", "orient"],
    );
    let source_text = text_in(
        dir.path(),
        "fix2-lens-source",
        &["source", "--path", "docs/a.md"],
    );
    assert_eq!(
        source_text
            .lines()
            .filter(|l| l.starts_with("lens: "))
            .count(),
        1,
        "{source_text}"
    );
}

// ── fix round 3: a resumed session's recovered state renders at every lens ────────────────────────
//
// `resume`/`adopt` views that carry `concerns` (the whole-scope door, or a selected edge) are
// `is_open_shaped`, so at `minimal`/`simple` the early return that keeps the candidate block
// density-bounded was ALSO hiding `view["continuation"]` — the resumed session's recovered
// findings, evidence receipts and unresolved questions. Ruling: the continuation renders as one
// summary line at every open-shaped lens, and `resume`/`adopt` additionally get the latest
// finding and latest unresolved question — the reader's own recovered state is never
// density-bounded, even though the candidate list it sits beside still is.

/// `resume` at `simple` recovers a finding and a question remembered in a prior `remember` call —
/// both must be visible without asking for a wider lens.
#[test]
fn resume_renders_its_recovered_finding_and_question_at_simple() {
    let dir = repo();
    claim_actor(dir.path(), "agent:reader@claude-sonnet-5", "fix3-resume");
    view_in(dir.path(), "fix3-resume", &["open", "--need", "orient"]);
    view_in(
        dir.path(),
        "fix3-resume",
        &["select", "--edge", "1", "--need", "Check the dependency"],
    );
    view_in(
        dir.path(),
        "fix3-resume",
        &["read", "--path", "docs/source.md", "--lines", "6:7"],
    );
    view_in(
        dir.path(),
        "fix3-resume",
        &[
            "remember",
            "--finding",
            "Needs a qualified reading",
            "--question",
            "Does the narrow evidence support the wider claim?",
            "--next-action",
            "Seek independent review",
            "--evidence",
            "docs/source.md:6:7",
        ],
    );
    let resume_text = text_in(dir.path(), "fix3-resume", &["resume"]);
    assert!(
        resume_text.contains("Continuation:") && resume_text.contains("finding(s)"),
        "{resume_text}"
    );
    assert!(
        resume_text.contains("Needs a qualified reading"),
        "the latest finding must render at simple:\n{resume_text}"
    );
    assert!(
        resume_text.contains("Does the narrow evidence support the wider claim?"),
        "the latest unresolved question must render at simple:\n{resume_text}"
    );
}

/// `open` at `simple` carries the same one-line continuation summary exactly once — a fresh
/// ceremony recovers nothing, so it never grows the finding/question lines `resume`/`adopt` do.
#[test]
fn open_carries_the_continuation_summary_exactly_once_at_simple() {
    let dir = repo();
    claim_actor(dir.path(), "agent:reader@claude-sonnet-5", "fix3-open");
    let open_text = text_in(dir.path(), "fix3-open", &["open", "--need", "orient"]);
    assert_eq!(open_text.matches("Continuation:").count(), 1, "{open_text}");
}
