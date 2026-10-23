use elohim_rust_style::{
    repository::{compare, Snapshot, POLICY},
    scan::scan,
};
use std::collections::BTreeMap;

fn tree(source: &str) -> Snapshot {
    Snapshot {
        files: BTreeMap::from([
            (
                "crate/Cargo.toml".into(),
                "[package]\nname='fixture'\nversion='0.1.0'\n".into(),
            ),
            ("crate/src/lib.rs".into(), source.into()),
            (
                POLICY.into(),
                r#"{"version":1,"rules":["RS-GOV-04"],"external_roots":[]}"#.into(),
            ),
        ]),
    }
}
fn verdict(before: &str, after: &str) -> Vec<String> {
    compare(&tree(before), &tree(after), "base".into(), "head".into())
        .unwrap()
        .failures
}
const DEBT: &str = "#[allow(dead_code)] fn old() {}";

#[test]
fn moving_debt_to_a_nested_cargo_package_is_not_paydown() {
    let before = tree(DEBT);
    let mut after = tree(DEBT);
    after.files.insert(
        "crate/src/Cargo.toml".into(),
        "[package]\nname='nested'\nversion='0.1.0'\n".into(),
    );
    let report = compare(&before, &after, "base".into(), "head".into()).unwrap();
    assert!(report.failures.iter().any(|s| s.contains("new")));
}

#[test]
fn new_descendants_cannot_shadow_an_existing_strict_binding() {
    let mut before = tree("pub fn clean() {}");
    before.files.insert("crate/.epr-meta".into(), "---\nepr-meta-version: 1\nid: strict\ncovers: subtree\nrules:\n  - id: strict\n    policy: rust-style-strict@1\n---\n".into());
    let mut after = Snapshot {
        files: before.files.clone(),
    };
    after.files.insert("crate/new/.epr-meta".into(), "---\nepr-meta-version: 1\nid: child\ncovers: subtree\nrules:\n  - id: strict\n    policy: unrelated@1\n---\n".into());
    after
        .files
        .insert("crate/new/mod.rs".into(), "pub fn also_clean() {}".into());
    let report = compare(&before, &after, "base".into(), "head".into()).unwrap();
    assert!(report.failures.iter().any(|s| s.contains("new descendant")));
}

#[test]
fn virtual_workspace_cargo_configuration_touches_descendant_crates() {
    let mut before = tree(DEBT);
    before.files.insert(
        "Cargo.toml".into(),
        "[workspace]\nmembers=['crate']\n".into(),
    );
    let mut after = Snapshot {
        files: before.files.clone(),
    };
    after
        .files
        .insert(".cargo/config.toml".into(), "[build]\njobs=1\n".into());
    let report = compare(&before, &after, "base".into(), "head".into()).unwrap();
    assert!(report
        .failures
        .iter()
        .any(|s| s.contains("remove at least one")));
}

#[test]
fn parser_covers_cfg_inner_attributes_and_macro_output_but_not_strings() {
    let code = r###"
        #![allow(unused)]
        #[cfg(any())] #[cfg_attr(unix, allow(dead_code))] fn disabled() {}
        macro_rules! generated { () => { #[allow(unused)] fn emitted() {} }; }
        macro_rules! conditional { ($cfg:meta) => { #[cfg_attr($cfg, allow(unused))] fn emitted() {} }; }
        const TEXT: &str = "#[allow(dead_code)]";
        // #[allow(dead_code)] is an example, not code.
    "###;
    assert_eq!(scan("src/lib.rs", code).unwrap().len(), 4);
}
#[test]
fn reasons_must_be_nonempty_literal_strings() {
    assert!(scan(
        "a.rs",
        "#[allow(dead_code, reason = \"used by a foreign entry point\")] fn f() {}"
    )
    .unwrap()
    .is_empty());
    assert_eq!(
        scan("a.rs", "#[expect(dead_code, reason = \"  \")] fn f() {}")
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn counts_cannot_trade_one_violation_for_another() {
    assert!(verdict(DEBT, "#[allow(unused)] fn replacement() {}")
        .iter()
        .any(|s| s.contains("new")));
}
#[test]
fn ordinary_touch_requires_real_paydown() {
    assert!(verdict(DEBT, &format!("{DEBT}\n// touched"))
        .iter()
        .any(|s| s.contains("remove at least one")));
    assert!(verdict(
        DEBT,
        "#[allow(dead_code, reason = \"foreign ABI entry point\")] fn old() {}"
    )
    .is_empty());
    assert!(verdict(DEBT, DEBT).is_empty());
}
#[test]
fn moving_lines_preserves_identity_and_duplicates_cannot_hide() {
    let a = scan("a.rs", DEBT).unwrap();
    let b = scan("a.rs", &format!("\n\n{DEBT}")).unwrap();
    assert_eq!(a[0].identity(), b[0].identity());
    assert!(
        verdict(DEBT, "#[allow(dead_code)] #[allow(dead_code)] fn old() {}")
            .iter()
            .any(|s| s.contains("new"))
    );
}
#[test]
fn strict_scopes_reject_legacy_debt_and_cannot_be_removed() {
    let mut before = tree(DEBT);
    let mut after = tree(DEBT);
    let strict="---\nepr-meta-version: 1\nid: fixture\nrules:\n  - id: strict\n    policy: rust-style-strict@1\n---\n";
    after.files.insert("crate/.epr-meta".into(), strict.into());
    assert!(compare(&before, &after, "base".into(), "head".into())
        .unwrap()
        .failures
        .iter()
        .any(|s| s.contains("strict")));
    before.files.insert("crate/.epr-meta".into(), strict.into());
    after.files.remove("crate/.epr-meta");
    assert!(compare(&before, &after, "base".into(), "head".into())
        .unwrap()
        .failures
        .iter()
        .any(|s| s.contains("weakened")));
}
#[test]
fn enrollment_preserves_no_new_but_does_not_demand_whole_repo_paydown() {
    let mut before = tree(DEBT);
    before.files.remove(POLICY);
    assert!(compare(
        &before,
        &tree(&format!("{DEBT}\n// touched")),
        "base".into(),
        "head".into()
    )
    .unwrap()
    .failures
    .is_empty());
    assert!(!compare(
        &before,
        &tree(&format!("{DEBT}\n#[allow(unused)] fn new() {{}}")),
        "base".into(),
        "head".into()
    )
    .unwrap()
    .failures
    .is_empty());
}
#[test]
fn missing_or_invalid_sources_never_report_clean() {
    assert!(scan("broken.rs", "fn broken(").is_err());
    let mut after = tree(DEBT);
    after.files.remove(POLICY);
    assert!(compare(&tree(DEBT), &after, "base".into(), "head".into()).is_err());
}
#[test]
fn documentation_does_not_trigger_paydown_and_source_deletion_does() {
    let before = tree(DEBT);
    let mut after = tree(DEBT);
    after.files.insert("crate/README.md".into(), "docs".into());
    assert!(compare(&before, &after, "base".into(), "head".into())
        .unwrap()
        .failures
        .is_empty());
    after.files.remove("crate/src/lib.rs");
    assert!(compare(&before, &after, "base".into(), "head".into())
        .unwrap()
        .failures
        .is_empty());
}
#[test]
fn exclusion_widening_is_refused() {
    let before = tree(DEBT);
    let mut after = tree(DEBT);
    after.files.insert(
        POLICY.into(),
        r#"{"version":1,"rules":["RS-GOV-04"],"external_roots":["crate"]}"#.into(),
    );
    assert!(compare(&before, &after, "base".into(), "head".into())
        .unwrap()
        .failures
        .iter()
        .any(|s| s.contains("exclusion")));
}

#[test]
fn lint_config_and_nested_policy_changes_touch_the_owning_crate() {
    for path in ["crate/clippy.toml", "crate/src/module/.epr-meta"] {
        let before = tree(DEBT);
        let mut after = tree(DEBT);
        after.files.insert(
            path.into(),
            if path.ends_with("clippy.toml") {
                "too-many-arguments-threshold = 8".into()
            } else {
                "---\nepr-meta-version: 1\nid: nested\n---\n".into()
            },
        );
        assert!(
            compare(&before, &after, "base".into(), "head".into())
                .unwrap()
                .failures
                .iter()
                .any(|s| s.contains("remove at least one")),
            "{path}"
        );
    }
}
