//! # placement-core — the placement contract's floor and ceiling, as pure functions
//!
//! Interfaces spec: `genesis/docs/superpowers/specs/2026-10-20-placement-contract-interfaces-design.md`
//! §2.D (the floor) and §2.E (the ceiling); design spec: `…/2026-10-20-what-every-app-inherits-
//! placement-contract-design.md` §7.
//!
//! Inputs go in, a plan comes out, an adapter applies it, in the shape of ark-core's
//! `lifecycle::step`. Decisions live only here; an adapter (the custody `Ports` in elohim-storage,
//! the task kind in ark-core) translates rows into these types and applies the plan as effects.
//!
//! | Module | Owns |
//! |---|---|
//! | [`kind`] | `KindId`, `GrantScope`, `PlacementKind`, `KindRegistry`, `MethodRegistry` |
//! | [`need`] | `Need`, `NeedKey`, `Demand`, `Requirements`, `PrivateNeed`, `NeedSource` |
//! | [`offer`] | `Posture`, `OfferTerms`, `DeviceFacts` (never serialized), `Candidate`, `Consent` |
//! | [`eligibility`] | `admit` → `Eligible` (unforgeable), `Ineligible`, the `Governor` face |
//! | [`rank`] | `MethodId`, `Ranking` and the four built-ins, `diversify`, the ranking contract |
//! | [`adopt`] | `CommitmentDraft`, `Carrying`, `Release`, `RealEvidence`, `settle` |
//! | [`decide`] | `Charter`, `Snapshot`, `Plan`, `decide`, `decide_private` |
//! | [`floor`] | `Trigger`, `Ports`, `Floor` |
//! | [`explain`] | `Explanation`, `explain` |
//! | [`params`] | `ParameterSet`, `HeadDeclaration`, `InForce` (the only form the floor accepts), `select_in_force` |
//! | [`ceiling`] | `Draft`, `Pressure`, `within_band`, the adapter traits |
//! | [`manifest`] | `ManifestNeeds`, `needs_from_manifest` |
//!
//! The rails of the spec's §1.3 that live here are tests in this crate: R-P1 and R-P2 in
//! `boundary` below, R-P3 and R-P7 there too, and R-P4, R-P5 and R-P6 as `compile_fail` doctests on
//! `offer`, `eligibility` and `params::InForce`.

#![forbid(unsafe_code)]
// R7 (interfaces spec §2.0) returns every refusal by value with its three parts, so the spec's
// signatures (`admit`, `InForce::from_head`, `within_band`, `RatificationCheck::verify`) carry an
// `Unanswered<R>` in their `Err`. Boxing it would change those signatures for a size lint.
#![allow(
    clippy::result_large_err,
    reason = "Interfaces ruling R7 returns all three Unanswered refusal parts by value; boxing would change those public signatures"
)]

pub mod adopt;
pub mod ceiling;
pub mod decide;
pub mod eligibility;
pub mod explain;
pub mod floor;
pub mod kind;
pub mod manifest;
pub mod need;
pub mod offer;
pub mod params;
pub mod rank;

pub use adopt::{
    commitment_id, fulfilment_id, fulfilment_promise, settle, Carrying, CommitmentDraft,
    FulfilmentDraft, Lease, Period, RealEvidence, RecordPlane, Release, ReleaseReason,
};
pub use ceiling::{
    within_band, Draft, DraftInput, ParameterHead, Pressure, PressureInput, PressureKind,
    RatificationCheck, TuningSink,
};
pub use decide::{
    chosen, decide, decide_private, Charter, DeviceParams, ForwardingRule, LossDomain, Plan,
    PrivatePlan, PrivateSnapshot, Snapshot,
};
pub use eligibility::{
    admit, Candidacy, Eligibility, Eligible, Ineligible, IneligibleKind, IneligibleReason,
};
pub use explain::{explain, Explanation, NotCarried, NotCarriedKind, NotCarriedReason, Outcome};
pub use floor::{affected, Floor, PassOutcome, PortError, Ports, Trigger};
pub use kind::{
    required_scope, Charge, GrantScope, KindId, KindRegistry, MethodRegistry, MishpatAction,
    PlacementKind,
};
pub use manifest::{
    needs_from_manifest, ClosureKey, ComputeBounds, ComputeNeed, GrantRequest, ManifestContext,
    ManifestNeeds, ManifestNeedsOut, WorksOffline,
};
pub use need::{
    AgentRefKey, Demand, Depth, Expresser, ExpresserKind, Isolation, Need, NeedClass, NeedKey,
    NeedSource, NeedTerms, PrivateNeed, Requirements,
};
pub use offer::{
    Candidate, Capacity, CidString, Consent, DeviceFacts, EpochSecs, EprScope, FailureDomain,
    OfferTerms, PersonChoices, Posture,
};
pub use params::{
    builtin_set, select_in_force, HeadDeclaration, InForce, ParamFallback, ParamRefusal,
    ParamRefusalKind, ParameterSet, PlaceError, RatificationKind, RatificationRef,
    RatificationRule, SetSource,
};
pub use rank::{
    diversify, DiversityAware, Factor, MethodId, NearestBytes, RankInput, RankKey, Ranking,
    Scratch, WeightedCapacity, Weights, XorDistance,
};

#[cfg(test)]
mod boundary {
    /// Every module's source, read at compile time. `scan_covers_every_module` keeps it honest.
    const SOURCES: &[(&str, &str)] = &[
        ("adopt.rs", include_str!("adopt.rs")),
        ("ceiling.rs", include_str!("ceiling.rs")),
        ("decide.rs", include_str!("decide.rs")),
        ("eligibility.rs", include_str!("eligibility.rs")),
        ("explain.rs", include_str!("explain.rs")),
        ("floor.rs", include_str!("floor.rs")),
        ("kind.rs", include_str!("kind.rs")),
        ("lib.rs", include_str!("lib.rs")),
        ("manifest.rs", include_str!("manifest.rs")),
        ("need.rs", include_str!("need.rs")),
        ("offer.rs", include_str!("offer.rs")),
        ("params.rs", include_str!("params.rs")),
        ("rank.rs", include_str!("rank.rs")),
    ];

    /// The `[dependencies]` table's package names, in file order.
    fn dependency_lines() -> Vec<(String, String)> {
        parse_dependencies(include_str!("../Cargo.toml")).unwrap_or_else(|e| panic!("R-P1: {e}"))
    }

    /// Every normal dependency a manifest declares, as (name, spec). A `[dependencies.<name>]`
    /// table is the dependency `<name>` (its body joined as the spec). Any other table that declares
    /// non-dev dependencies (`[target.<cfg>.dependencies]`, `[build-dependencies]`, and their dotted
    /// forms) is refused: each adds a dependency R-P1's set must see, and the rail reads only the
    /// forms it can name exactly. `[dev-dependencies]` and its dotted form are skipped.
    fn parse_dependencies(toml: &str) -> Result<Vec<(String, String)>, String> {
        enum Table {
            Deps,
            Dotted(usize),
            Other,
        }
        let mut table = Table::Other;
        let mut out: Vec<(String, String)> = Vec::new();
        for line in toml.lines() {
            let t = line.trim();
            if t.starts_with('[') {
                let header = t.trim_start_matches('[').trim_end_matches(']').trim();
                let dev = header == "dev-dependencies" || header.starts_with("dev-dependencies.");
                table = if header == "dependencies" {
                    Table::Deps
                } else if let Some(name) = header.strip_prefix("dependencies.") {
                    out.push((name.trim_matches('"').to_string(), String::new()));
                    Table::Dotted(out.len() - 1)
                } else if !dev && header.contains("dependencies") {
                    return Err(format!(
                        "`[{header}]` declares dependencies the exact set cannot see; \
                         name them in `[dependencies]`"
                    ));
                } else {
                    Table::Other
                };
                continue;
            }
            if t.is_empty() || t.starts_with('#') {
                continue;
            }
            match table {
                Table::Other => {}
                Table::Deps => {
                    let (name, spec) = t
                        .split_once('=')
                        .ok_or_else(|| format!("a dependency line is `name = spec`: {t:?}"))?;
                    out.push((
                        name.trim().trim_end_matches(".workspace").to_string(),
                        spec.trim().to_string(),
                    ));
                }
                Table::Dotted(i) => {
                    let spec = &mut out[i].1;
                    if !spec.is_empty() {
                        spec.push_str(", ");
                    }
                    spec.push_str(t);
                }
            }
        }
        Ok(out)
    }

    #[test]
    fn the_dependency_parser_sees_every_form_that_adds_a_dependency() {
        let names = |toml: &str| -> Result<Vec<String>, String> {
            parse_dependencies(toml).map(|d| d.into_iter().map(|(n, _)| n).collect())
        };
        assert_eq!(
            names("[dependencies]\ncid = { workspace = true }\nserde.workspace = true\n").unwrap(),
            ["cid", "serde"]
        );
        // The dotted-table form is a dependency named by its header.
        let dotted = parse_dependencies(
            "[dependencies]\ncid = \"1\"\n[dependencies.tokio]\nversion = \"1\"\nfeatures = [\"sync\"]\n",
        )
        .unwrap();
        assert_eq!(dotted[1].0, "tokio");
        assert_eq!(dotted[1].1, "version = \"1\", features = [\"sync\"]");
        // A target-specific or build dependency table is refused outright.
        for header in [
            "[target.'cfg(unix)'.dependencies]",
            "[target.'cfg(unix)'.dependencies.tokio]",
            "[build-dependencies]",
            "[build-dependencies.cc]",
        ] {
            let toml = format!("[dependencies]\ncid = \"1\"\n{header}\ntokio = \"1\"\n");
            assert!(names(&toml).is_err(), "{header}");
        }
        // Dev-dependencies are not the crate's set.
        assert_eq!(
            names(
                "[dependencies]\ncid = \"1\"\n[dev-dependencies]\nserde_json = \"1\"\n[dev-dependencies.x]\nversion = \"1\"\n"
            )
            .unwrap(),
            ["cid"]
        );
    }

    /// The value of `key` in a dependency spec (`{ a = 1, key = v }`, or a dotted table's joined
    /// body), up to the next `,` or `}` outside brackets; `None` when the key is absent.
    fn spec_value<'a>(spec: &'a str, key: &str) -> Option<&'a str> {
        let mut from = 0;
        while let Some(at) = spec[from..].find(key) {
            let start = from + at;
            let before = spec[..start].chars().next_back();
            let rest = spec[start + key.len()..].trim_start();
            if before.is_none_or(|c| c == '{' || c == ',' || c.is_whitespace()) {
                if let Some(value) = rest.strip_prefix('=') {
                    let value = value.trim_start();
                    let mut depth = 0usize;
                    let end = value
                        .char_indices()
                        .find(|(_, c)| match c {
                            '[' => {
                                depth += 1;
                                false
                            }
                            ']' => {
                                depth = depth.saturating_sub(1);
                                false
                            }
                            ',' | '}' => depth == 0,
                            _ => false,
                        })
                        .map_or(value.len(), |(i, _)| i);
                    return Some(value[..end].trim());
                }
            }
            from = start + key.len();
        }
        None
    }

    /// Every double-quoted string in `text`.
    fn quoted(text: &str) -> Vec<&str> {
        text.split('"').skip(1).step_by(2).collect()
    }

    /// R-P1's feature half: placement-core never turns on any elohim-compute feature, `runtime`
    /// (chrono, locks, concurrent maps) above all. Checks the dependency's own spec (no workspace
    /// inheritance, whose defaults this file cannot see; `default-features = false`; an empty or
    /// absent `features` list) and the crate's own `[features]` table (no `elohim-compute/…` or
    /// `elohim-compute?/…` forwarding). Codex review 2, finding 11: a substring test passed
    /// `default-features = false, features = ["runtime"]`.
    fn compute_feature_violations(toml: &str) -> Vec<String> {
        const DEP: &str = "elohim-compute";
        let deps = match parse_dependencies(toml) {
            Ok(d) => d,
            Err(e) => return vec![e],
        };
        let mut out = Vec::new();
        match deps.iter().find(|(n, _)| n == DEP) {
            None => out.push(format!("no `{DEP}` dependency")),
            Some((_, spec)) => {
                if spec.trim() == "true" || spec_value(spec, "workspace") == Some("true") {
                    out.push(format!("`{DEP}` inherits from the workspace"));
                }
                let defaults = spec_value(spec, "default-features")
                    .or_else(|| spec_value(spec, "default_features"));
                if defaults != Some("false") {
                    out.push(format!("`{DEP}` keeps its default features ({defaults:?})"));
                }
                if let Some(list) = spec_value(spec, "features") {
                    let named = quoted(list);
                    if !named.is_empty() {
                        out.push(format!("`{DEP}` turns on features {named:?}"));
                    }
                }
            }
        }
        let mut in_features = false;
        for line in toml.lines() {
            let t = line.trim();
            if t.starts_with('[') {
                in_features = t == "[features]";
                continue;
            }
            if !in_features || t.starts_with('#') {
                continue;
            }
            for f in quoted(t) {
                if f.starts_with(&format!("{DEP}/")) || f.starts_with(&format!("{DEP}?/")) {
                    out.push(format!("`[features]` forwards `{f}`"));
                }
            }
        }
        out
    }

    #[test]
    fn the_compute_feature_rail_reads_features_not_substrings() {
        let manifest = |dep: &str, features: &str| {
            format!("[features]\n{features}\n[dependencies]\ncid = \"1\"\nelohim-compute = {dep}\n")
        };
        let clean = "{ path = \"../c\", default-features = false }";
        assert!(compute_feature_violations(&manifest(clean, "")).is_empty());
        assert!(compute_feature_violations(&manifest(
            "{ path = \"../c\", default-features = false, features = [] }",
            "testing = [\"elohim-seam-contracts/harness\"]"
        ))
        .is_empty());
        for (dep, features) in [
            // The finding's fixture: the substring is there, and so is `runtime`.
            (
                "{ path = \"../c\", default-features = false, features = [\"runtime\"] }",
                "",
            ),
            ("{ path = \"../c\" }", ""),
            ("{ path = \"../c\", default-features = true }", ""),
            ("{ workspace = true, default-features = false }", ""),
            (clean, "x = [\"elohim-compute/runtime\"]"),
            (clean, "x = [\"elohim-compute?/runtime\"]"),
            (clean, "default = [\"dep:cid\", \"elohim-compute/default\"]"),
        ] {
            assert!(
                !compute_feature_violations(&manifest(dep, features)).is_empty(),
                "{dep} / {features}"
            );
        }
        // The dotted-table form is read the same way.
        let dotted = "[dependencies]\ncid = \"1\"\n[dependencies.elohim-compute]\npath = \"../c\"\ndefault-features = false\nfeatures = [\"runtime\"]\n";
        assert!(!compute_feature_violations(dotted).is_empty());
        let dotted_clean =
            "[dependencies]\ncid = \"1\"\n[dependencies.elohim-compute]\npath = \"../c\"\ndefault-features = false\n";
        assert!(compute_feature_violations(dotted_clean).is_empty());
    }

    /// R-P1: an exact set, not a deny-list. chrono, ts-rs and ed25519 arrive transitively through
    /// elohim-epr, as they do for ark-core; this rail governs what the crate names, R-P2 what it calls.
    #[test]
    fn exact_dependency_set() {
        let deps = dependency_lines();
        let mut names: Vec<&str> = deps.iter().map(|(n, _)| n.as_str()).collect();
        names.sort_unstable();
        assert_eq!(
            names,
            [
                "cid",
                "elohim-compute",
                "elohim-epr",
                "elohim-epr-rea",
                "elohim-seam-contracts",
                "serde",
                "serde_ipld_dagcbor",
                "sha2",
                "thiserror",
            ],
            "placement-core's direct dependency set moved; revise rail R-P1 and the interfaces spec §1.3 together"
        );
        let spec = |n: &str| {
            deps.iter()
                .find(|(name, _)| name == n)
                .map(|(_, s)| s.as_str())
                .unwrap()
        };
        assert!(
            spec("elohim-epr-rea").contains("default-features = false"),
            "epr-rea without its sidecar"
        );
        let violations = compute_feature_violations(include_str!("../Cargo.toml"));
        assert!(
            violations.is_empty(),
            "elohim-compute without `runtime`: {violations:?}"
        );
        assert!(
            spec("elohim-seam-contracts").contains("\"serde\""),
            "seam-contracts with serde"
        );
    }

    /// R-P2: no clock. Time arrives as `EpochSecs`. Needles are split literals because this module
    /// is one of the files it reads.
    #[test]
    fn no_clock() {
        let denied = [
            format!("Utc{}now", "::"),
            format!("SystemTime{}now", "::"),
            format!("Instant{}now", "::"),
        ];
        for (file, source) in SOURCES {
            for needle in &denied {
                assert!(
                    !source.contains(needle.as_str()),
                    "placement-core src/{file} reads a clock: `{needle}`"
                );
            }
        }
    }

    /// No I/O, threads or async in a pure core (§1.3: placement-core has no async at all).
    #[test]
    fn no_io_or_async() {
        let denied = [
            format!("std{}fs", "::"),
            format!("File{}", "::"),
            format!("std{}net", "::"),
            format!("std{}thread", "::"),
            format!("async {}", "fn"),
            format!(".{}", "await"),
        ];
        for (file, source) in SOURCES {
            for needle in &denied {
                assert!(
                    !source.contains(needle.as_str()),
                    "placement-core src/{file} names `{needle}`"
                );
            }
        }
    }

    /// R-P3: no floats and no partial ordering in ranking or parameters; weights are basis points.
    #[test]
    fn no_floats_in_ranking() {
        let denied = [
            format!("f{}", "32"),
            format!("f{}", "64"),
            format!("partial{}", "_cmp"),
        ];
        for (file, source) in SOURCES
            .iter()
            .filter(|(f, _)| *f == "rank.rs" || *f == "params.rs")
        {
            for needle in &denied {
                assert!(
                    !source.contains(needle.as_str()),
                    "placement-core src/{file} names `{needle}`"
                );
            }
        }
    }

    /// `src` with every comment removed and every string and char literal emptied, newlines kept:
    /// what the compiler executes, for the rails below. Line and nested block comments, plain, byte
    /// and raw strings, and char literals (a lifetime is kept) are recognised.
    fn code_only(src: &str) -> String {
        let c: Vec<char> = src.chars().collect();
        let ident = |i: usize| i < c.len() && (c[i].is_alphanumeric() || c[i] == '_');
        let mut out = String::with_capacity(src.len());
        let mut i = 0;
        while i < c.len() {
            let next = c.get(i + 1).copied();
            if c[i] == '/' && next == Some('/') {
                while i < c.len() && c[i] != '\n' {
                    i += 1;
                }
                continue;
            }
            if c[i] == '/' && next == Some('*') {
                let mut depth = 0usize;
                while i < c.len() {
                    if c[i] == '/' && c.get(i + 1) == Some(&'*') {
                        depth += 1;
                        i += 2;
                    } else if c[i] == '*' && c.get(i + 1) == Some(&'/') {
                        depth -= 1;
                        i += 2;
                        if depth == 0 {
                            break;
                        }
                    } else {
                        if c[i] == '\n' {
                            out.push('\n');
                        }
                        i += 1;
                    }
                }
                continue;
            }
            // A raw string: `r`, `br`, then `#`s and a quote, not inside an identifier.
            let raw_at = if c[i] == 'r' && (i == 0 || !ident(i - 1)) {
                Some(i + 1)
            } else if c[i] == 'b' && next == Some('r') && (i == 0 || !ident(i - 1)) {
                Some(i + 2)
            } else {
                None
            };
            if let Some(mut j) = raw_at {
                let mut hashes = 0;
                while c.get(j) == Some(&'#') {
                    hashes += 1;
                    j += 1;
                }
                if c.get(j) == Some(&'"') {
                    j += 1;
                    loop {
                        if j >= c.len() {
                            break;
                        }
                        if c[j] == '"' && (1..=hashes).all(|h| c.get(j + h) == Some(&'#')) {
                            j += 1 + hashes;
                            break;
                        }
                        if c[j] == '\n' {
                            out.push('\n');
                        }
                        j += 1;
                    }
                    out.push_str("\"\"");
                    i = j;
                    continue;
                }
            }
            if c[i] == '"' {
                i += 1;
                while i < c.len() && c[i] != '"' {
                    if c[i] == '\\' {
                        i += 1;
                    } else if c[i] == '\n' {
                        out.push('\n');
                    }
                    i += 1;
                }
                out.push_str("\"\"");
                i += 1;
                continue;
            }
            if c[i] == '\'' {
                if next == Some('\\') {
                    i += 3; // the quote, the backslash, and the escaped character
                    while i < c.len() && c[i] != '\'' {
                        i += 1;
                    }
                    out.push_str("' '");
                    i += 1;
                    continue;
                }
                if c.get(i + 2) == Some(&'\'') {
                    out.push_str("' '");
                    i += 3;
                    continue;
                }
            }
            out.push(c[i]);
            i += 1;
        }
        out
    }

    /// The bodies of the active `#[test]` functions in `code` (already [`code_only`]). A test whose
    /// attributes say `ignore` or carry any `cfg(` is not counted: it may not run.
    fn active_test_bodies(code: &str) -> Vec<&str> {
        let marker = "#[test]";
        let mut bodies = Vec::new();
        let mut from = 0;
        while let Some(at) = code[from..].find(marker).map(|p| p + from) {
            let item_start = code[..at].rfind(['}', ';', '{']).map_or(0, |p| p + 1);
            let Some(open) = code[at..].find('{').map(|p| p + at) else {
                break;
            };
            let attrs = &code[item_start..open];
            let mut depth = 0usize;
            let mut close = open;
            for (k, ch) in code[open..].char_indices() {
                match ch {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            close = open + k;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if !attrs.contains("ignore") && !attrs.contains("cfg(") {
                bodies.push(&code[open..=close]);
            }
            from = close.max(at + marker.len());
        }
        bodies
    }

    /// Code that disables what follows it under test, which the rails would otherwise read as live.
    fn disabling_cfgs() -> [String; 4] {
        [
            format!("cfg({}())", "any"),
            format!("cfg(not({}))", "test"),
            format!("cfg({})", "FALSE"),
            format!("cfg({})", "false"),
        ]
    }

    /// R-P7 over `sources`: every `impl Ranking for X` anywhere in the crate has an executable
    /// `assert_ranking_contract(&X…)` in an active `#[test]` body somewhere in the crate. Comments,
    /// strings and ignored or cfg-gated tests do not count. Returns each uncovered ranking.
    fn ranking_contract_gaps(sources: &[(&str, &str)]) -> Vec<String> {
        let code: Vec<(&str, String)> = sources.iter().map(|(f, s)| (*f, code_only(s))).collect();
        let marker = format!("impl {} for ", "Ranking");
        let mut gaps = Vec::new();
        for (file, src) in &code {
            for needle in disabling_cfgs() {
                if src.contains(needle.as_str()) {
                    gaps.push(format!("src/{file} disables code with `{needle}`"));
                }
            }
        }
        let bodies: Vec<&str> = code
            .iter()
            .flat_map(|(_, src)| active_test_bodies(src))
            .collect();
        for (file, src) in &code {
            for (at, _) in src.match_indices(marker.as_str()) {
                let name: String = src[at + marker.len()..]
                    .chars()
                    .take_while(|ch| ch.is_alphanumeric() || *ch == '_')
                    .collect();
                let call = format!("assert_ranking_contract(&{name}");
                let covered = bodies.iter().any(|b| {
                    b.match_indices(call.as_str()).any(|(k, _)| {
                        matches!(b[k + call.len()..].chars().next(), Some(')') | Some('('))
                    })
                });
                if !covered {
                    gaps.push(format!(
                        "`{name}` (src/{file}) implements Ranking but no active test runs `{call})`"
                    ));
                }
            }
        }
        gaps
    }

    /// R-P7: every `impl Ranking for X` in the crate runs the contract in an active test.
    #[test]
    fn every_ranking_runs_the_contract() {
        let gaps = ranking_contract_gaps(SOURCES);
        assert!(gaps.is_empty(), "{gaps:#?}");
        let marker = format!("impl {} for ", "Ranking");
        let seen: usize = SOURCES
            .iter()
            .map(|(_, s)| code_only(s).matches(marker.as_str()).count())
            .sum();
        assert!(
            seen >= 4,
            "the scan found {seen} rankings; the marker drifted"
        );
    }

    /// Codex review 1, finding 11: the rail fails when the contract call is missing, commented
    /// out, only in a string, outside a test, or in an ignored or cfg-gated test, and it sees an
    /// implementation in any module.
    #[test]
    fn the_ranking_rail_fails_without_an_executable_contract_call() {
        let imp = "impl Ranking for Mine {}\n";
        let ok = format!("{imp}#[test]\nfn t() {{ assert_ranking_contract(&Mine); }}\n");
        assert!(ranking_contract_gaps(&[("a.rs", &ok)]).is_empty());
        // The implementation in one module, its test in another.
        let test_only = "#[test]\nfn t() { assert_ranking_contract(&Mine(1)); }\n";
        assert!(ranking_contract_gaps(&[("kind.rs", imp), ("other.rs", test_only)]).is_empty());
        for (label, body) in [
            ("missing", "#[test]\nfn t() {}\n".to_string()),
            (
                "commented",
                "#[test]\nfn t() {\n    // assert_ranking_contract(&Mine);\n}\n".to_string(),
            ),
            (
                "block-commented",
                "#[test]\nfn t() { /* assert_ranking_contract(&Mine); */ }\n".to_string(),
            ),
            (
                "in a string",
                "#[test]\nfn t() { let _ = \"assert_ranking_contract(&Mine)\"; }\n".to_string(),
            ),
            (
                "not a test",
                "fn t() { assert_ranking_contract(&Mine); }\n".to_string(),
            ),
            (
                "ignored",
                "#[test]\n#[ignore]\nfn t() { assert_ranking_contract(&Mine); }\n".to_string(),
            ),
            (
                "ignored before",
                "#[ignore]\n#[test]\nfn t() { assert_ranking_contract(&Mine); }\n".to_string(),
            ),
            (
                "another type",
                "#[test]\nfn t() { assert_ranking_contract(&MineToo); }\n".to_string(),
            ),
        ] {
            let src = format!("{imp}{body}");
            assert!(
                !ranking_contract_gaps(&[("x.rs", &src)]).is_empty(),
                "{label}: the rail passed"
            );
        }
        let gated = format!(
            "{imp}#[{}]\nmod off {{\n#[test]\nfn t() {{ assert_ranking_contract(&Mine); }}\n}}\n",
            format_args!("cfg({}())", "any")
        );
        assert!(
            !ranking_contract_gaps(&[("x.rs", &gated)]).is_empty(),
            "cfg-gated module"
        );
    }

    /// Every file a `mod x;` declaration in `sources` compiles, resolved as rustc resolves it
    /// (`lib.rs` and `mod.rs` declare siblings; `foo.rs` declares `foo/x.rs`), that is not itself in
    /// `sources`; and every `#[path]` or `include!`, which the resolution cannot follow.
    fn unscanned_modules(sources: &[(&str, &str)]) -> Vec<String> {
        let names: Vec<&str> = sources.iter().map(|(n, _)| *n).collect();
        let path_attr = format!("#[{}", "path");
        let include = format!("include{}(", "!");
        let mut out = Vec::new();
        for (file, src) in sources {
            let code = code_only(src);
            for needle in [&path_attr, &include] {
                if code.contains(needle.as_str()) {
                    out.push(format!(
                        "src/{file} uses `{needle}`, which the scan cannot follow"
                    ));
                }
            }
            let dir = match file.rsplit_once('/') {
                Some((d, "mod.rs")) => format!("{d}/"),
                None if *file == "lib.rs" || *file == "mod.rs" => String::new(),
                _ => format!("{}/", file.trim_end_matches(".rs")),
            };
            for line in code.lines() {
                let t = line.trim();
                let rest = t
                    .strip_prefix("pub(crate) mod ")
                    .or_else(|| t.strip_prefix("pub mod "))
                    .or_else(|| t.strip_prefix("mod "));
                let Some(rest) = rest.filter(|r| r.ends_with(';')) else {
                    continue;
                };
                let m = rest.trim_end_matches(';').trim();
                let flat = format!("{dir}{m}.rs");
                let nested = format!("{dir}{m}/mod.rs");
                if !names.contains(&flat.as_str()) && !names.contains(&nested.as_str()) {
                    out.push(format!("src/{flat} is compiled but not scanned"));
                }
            }
        }
        out
    }

    /// Every compiled source is in `SOURCES`, nested modules included (Codex review 1, finding 12).
    #[test]
    fn scan_covers_every_module() {
        let missing = unscanned_modules(SOURCES);
        assert!(missing.is_empty(), "{missing:#?}");
    }

    #[test]
    fn the_module_scan_follows_nested_declarations() {
        let lib = "pub mod a;\nmod tests {\n}\n";
        assert!(unscanned_modules(&[("lib.rs", lib), ("a.rs", "")]).is_empty());
        // A child declared inside a listed file must be listed too.
        let a = "pub(crate) mod child;\n";
        assert_eq!(
            unscanned_modules(&[("lib.rs", lib), ("a.rs", a)]),
            ["src/a/child.rs is compiled but not scanned"]
        );
        assert!(unscanned_modules(&[("lib.rs", lib), ("a.rs", a), ("a/child.rs", "")]).is_empty());
        // A commented-out declaration compiles nothing.
        assert!(unscanned_modules(&[("lib.rs", "// mod gone;\n")]).is_empty());
        // Literals never hide or invent code: an escaped quote char, a raw string with a quote.
        assert_eq!(
            code_only("let q = '\\''; let r = r#\"mod z;\"#; mod w;"),
            "let q = ' '; let r = \"\"; mod w;"
        );
        // `#[path]` and `include!` are refused outright.
        let path = format!("#[{}= \"x.rs\"]\nmod y;\n", "path ");
        assert!(!unscanned_modules(&[("lib.rs", &path), ("y.rs", "")]).is_empty());
        let inc = format!("include{}(\"x.rs\");\n", "!");
        assert!(!unscanned_modules(&[("lib.rs", &inc)]).is_empty());
    }
}
