//! The steward's 2026-11-18 ruling: isa.x86 admission judges the declared build level.
use std::collections::{BTreeMap, BTreeSet};

use host_contract::{
    admit, names::axis, AxisId, AxisValue, Clause, ClauseSource, Predicate, Provides, Requires,
    Version,
};
use serde_json::Value;

use super::{declared, govern, Declared};

fn set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

fn inputs(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

/// What shem's and ethosengine's `epr` at 6616ce547 both recorded: the instruction union,
/// runtime-dispatched code included, beside a glibc floor.
fn extracted() -> Requires {
    let mut requires = Requires::default();
    for (name, predicate) in [
        (axis::ARCH, Predicate::Equal("x86_64".into())),
        (
            axis::ABI_LIBC,
            Predicate::AtLeast(Version {
                line: "glibc".into(),
                components: vec![2, 39],
            }),
        ),
        (
            axis::ISA_X86,
            Predicate::Subset(set(&[
                "adx",
                "aes",
                "avx",
                "avx2",
                "bmi1",
                "bmi2",
                "cx16",
                "f16c",
                "fma",
                "fma4",
                "lahf_lm",
                "lzcnt",
                "movbe",
                "osxsave",
                "pclmulqdq",
                "popcnt",
                "sha_ni",
                "sse3",
                "sse4_1",
                "sse4_2",
                "ssse3",
                "vaes",
                "vpclmulqdq",
            ])),
        ),
    ] {
        requires
            .insert(Clause {
                axis: AxisId(name.into()),
                predicate,
                source: ClauseSource::Extracted,
                narrowing: None,
            })
            .unwrap();
    }
    requires
}

/// A Zen 2 host like ethosengine's 3900X: x86-64-v3 plus adx, aes, sha_ni, pclmulqdq; no
/// fma4, vaes or vpclmulqdq. `v2_only` drops everything above x86-64-v2.
fn host(v2_only: bool) -> Provides {
    let mut features = set(&[
        "cx16", "lahf_lm", "popcnt", "sse3", "sse4_1", "sse4_2", "ssse3",
    ]);
    if !v2_only {
        features.extend(set(&[
            "avx",
            "avx2",
            "bmi1",
            "bmi2",
            "f16c",
            "fma",
            "lzcnt",
            "movbe",
            "osxsave",
            "adx",
            "aes",
            "pclmulqdq",
            "sha_ni",
        ]));
    }
    let mut provides = Provides::default();
    provides.observe(
        AxisId(axis::ARCH.into()),
        AxisValue::Scalar("x86_64".into()),
    );
    provides.observe(
        AxisId(axis::ABI_LIBC.into()),
        AxisValue::Version(Version {
            line: "glibc".into(),
            components: vec![2, 39],
        }),
    );
    provides.observe(AxisId(axis::ISA_X86.into()), AxisValue::Names(features));
    provides
}

fn governed(declaration: &Declared) -> (Requires, BTreeMap<String, Value>) {
    let mut requires = extracted();
    let mut evidence = BTreeMap::new();
    govern(&mut requires, &mut evidence, declaration, "bafymethod").unwrap();
    (requires, evidence)
}

fn isa_clause(requires: &Requires) -> Option<Clause> {
    let clauses: Vec<Clause> =
        serde_json::from_value(serde_json::to_value(requires).unwrap()).unwrap();
    clauses
        .into_iter()
        .find(|clause| clause.axis.0 == axis::ISA_X86)
}

#[test]
fn the_declaration_is_read_from_target_isa_and_rustflags() {
    assert_eq!(declared(&inputs(&[])), Declared::Features(BTreeSet::new()));
    assert_eq!(
        declared(&inputs(&[("RUSTFLAGS", "")])),
        Declared::Features(BTreeSet::new())
    );
    let Declared::Features(v3) = declared(&inputs(&[("RUSTFLAGS", "-C target-cpu=x86-64-v3")]))
    else {
        panic!("v3 is declarable")
    };
    assert!(
        v3.contains("avx2") && v3.contains("movbe") && !v3.contains("vaes"),
        "{v3:?}"
    );
    // The higher of target.isa and target-cpu stands, in any spelling of -C.
    for flags in [
        "-Ctarget-cpu=x86-64-v3",
        "--codegen target-cpu=x86-64-v3",
        "--codegen=target-cpu=x86-64-v3",
    ] {
        assert_eq!(
            declared(&inputs(&[
                ("target.isa", "x86-64-v2"),
                ("RUSTFLAGS", flags)
            ])),
            Declared::Features(v3.clone()),
            "{flags}"
        );
    }
    assert_eq!(
        declared(&inputs(&[(
            "RUSTFLAGS",
            "-C target-feature=+aes,+sha,+sse4.2,-avx,+crt-static"
        )])),
        Declared::Features(set(&["aes", "sha_ni", "sse4_2", "crt-static"])),
        "rustc spellings map to the contract's; removals add nothing"
    );
    for (flags, why) in [
        ("-C target-cpu=native", "target-cpu=native"),
        ("-C target-cpu=znver2", "target-cpu=znver2"),
    ] {
        assert_eq!(
            declared(&inputs(&[("RUSTFLAGS", flags)])),
            Declared::Undeclarable(why.into())
        );
    }
    assert_eq!(
        declared(&inputs(&[("target.isa", "armv8")])),
        Declared::Undeclarable("target.isa=armv8".into())
    );
}

#[test]
fn a_baseline_offer_whose_bytes_show_vaes_and_fma4_is_admitted_on_a_host_without_them() {
    // The receipt: the honest draw refused shem's baseline epr on ethosengine for fma4, vaes and
    // vpclmulqdq, which ethosengine's own baseline epr also contains and runs with.
    let before = admit(&extracted(), &host(false));
    assert_eq!(
        before.decision,
        elohim_epr::verdict::Decision::Refuse,
        "the defect it cures"
    );
    let (requires, evidence) = governed(&Declared::Features(BTreeSet::new()));
    assert!(
        isa_clause(&requires).is_none(),
        "the baseline requires nothing above the floor"
    );
    assert_eq!(
        admit(&requires, &host(false)).decision,
        elohim_epr::verdict::Decision::Permit
    );
    // Other axes are untouched, and what the bytes used is kept as evidence.
    assert_eq!(requires.clauses().count(), 2);
    assert_eq!(evidence[axis::ISA_X86]["declared"], serde_json::json!([]));
    let dispatched = evidence[axis::ISA_X86]["dispatched"].as_array().unwrap();
    assert!(dispatched.iter().any(|f| f == "vaes") && dispatched.iter().any(|f| f == "fma4"));
}

#[test]
fn a_declared_v3_offer_refuses_on_a_v2_host_and_keeps_the_rest_as_narrowing() {
    let Declared::Features(v3) = declared(&inputs(&[("target.isa", "x86-64-v3")])) else {
        panic!()
    };
    let (requires, _) = governed(&Declared::Features(v3));
    let clause = isa_clause(&requires).expect("a v3 declaration requires v3");
    assert_eq!(clause.source, ClauseSource::Declared);
    let Predicate::Subset(required) = &clause.predicate else {
        panic!()
    };
    assert!(required.contains("avx2") && !required.contains("vaes") && !required.contains("adx"));
    let narrowing = clause
        .narrowing
        .expect("what the bytes use beyond v3 is recorded");
    assert_eq!(narrowing.evidence, "bafymethod");
    assert!(narrowing.removed.contains("vaes") && narrowing.removed.contains("fma4"));
    assert!(
        !narrowing.removed.contains("avx2"),
        "a declared feature is required, not removed"
    );
    assert_eq!(
        admit(&requires, &host(true)).decision,
        elohim_epr::verdict::Decision::Refuse
    );
    assert_eq!(
        admit(&requires, &host(false)).decision,
        elohim_epr::verdict::Decision::Permit
    );
}

#[test]
fn a_native_build_keeps_the_extracted_requirement_and_refuses_on_a_v2_host() {
    let native = declared(&inputs(&[("RUSTFLAGS", "-C target-cpu=native")]));
    let (requires, evidence) = governed(&native);
    assert_eq!(
        requires,
        extracted(),
        "nothing declared that this reader can name"
    );
    assert!(evidence.is_empty());
    assert_eq!(
        admit(&requires, &host(true)).decision,
        elohim_epr::verdict::Decision::Refuse
    );
}
