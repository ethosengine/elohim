use std::collections::BTreeSet;

use elohim_epr::cbor;
use elohim_epr::verdict::Decision;
use host_contract::{
    admit, AxisId, AxisValue, Clause, ClauseSource, Predicate, Provides, Requires, X86_64_V2,
    X86_64_V3,
};

fn clause(predicate: Predicate, source: ClauseSource) -> Clause {
    Clause {
        axis: AxisId("test.axis".into()),
        predicate,
        source,
        narrowing: None,
    }
}

#[test]
fn incompatible_merges_are_atomic_and_cannot_enter_through_serde() {
    for (left, right) in [
        (
            Predicate::Equal("linux".into()),
            Predicate::Equal("darwin".into()),
        ),
        (
            Predicate::Equal("linux".into()),
            Predicate::Present(BTreeSet::new()),
        ),
        (
            Predicate::AtLeast(host_contract::Version {
                line: "glibc".into(),
                components: vec![2, 34],
            }),
            Predicate::AtLeast(host_contract::Version {
                line: "musl".into(),
                components: vec![1, 2],
            }),
        ),
    ] {
        let original = clause(left, ClauseSource::Declared);
        let incoming = clause(right, ClauseSource::Extracted);
        let mut requires = Requires::try_from(vec![original.clone()]).unwrap();
        let before = requires.clone();
        assert!(requires.insert(incoming.clone()).is_err());
        assert_eq!(requires, before);
        let other = Requires::try_from(vec![incoming.clone()]).unwrap();
        assert!(requires.merge(&other).is_err());
        assert!(other.merge(&requires).is_err());
        let json = serde_json::to_value(vec![original, incoming]).unwrap();
        assert!(serde_json::from_value::<Requires>(json).is_err());
    }
}

#[test]
fn merging_preserves_sources_and_disagreements_through_later_evidence() {
    let declared = clause(
        Predicate::Subset(BTreeSet::from(["sse3".into()])),
        ClauseSource::Declared,
    );
    let extracted = clause(
        Predicate::Subset(BTreeSet::from(["avx2".into()])),
        ClauseSource::Extracted,
    );
    let proven = clause(
        Predicate::Subset(BTreeSet::from(["sse3".into(), "avx2".into()])),
        ClauseSource::Proven,
    );
    let orders = [
        vec![declared.clone(), extracted.clone(), proven.clone()],
        vec![proven.clone(), declared.clone(), extracted.clone()],
        vec![extracted.clone(), proven.clone(), declared.clone()],
    ];
    let expected = Requires::try_from(orders[0].clone()).unwrap();
    for order in orders {
        let requires = Requires::try_from(order).unwrap();
        assert_eq!(requires, expected);
        assert_eq!(requires.disagreements().len(), 1);
        let disagreement = requires.disagreements().into_iter().next().unwrap();
        assert_eq!(disagreement.declared, declared);
        assert_eq!(disagreement.extracted, extracted);
        assert_eq!(requires.clauses().next().unwrap().evidence().len(), 3);
        assert_eq!(requires.merge(&requires).unwrap(), requires);
    }
}

#[test]
fn observations_replace_stale_values_and_only_check_required_axes() {
    let axis = AxisId("test.axis".into());
    let requires = Requires::try_from(vec![clause(
        Predicate::Equal("linux".into()),
        ClauseSource::Declared,
    )])
    .unwrap();
    let mut provides = Provides::default();
    provides.observe(axis.clone(), AxisValue::Scalar("linux".into()));
    provides.observe(
        AxisId("private.unrelated".into()),
        AxisValue::Scalar("secret".into()),
    );
    let verdict = admit(&requires, &provides);
    assert_eq!(verdict.decision, Decision::Permit);
    assert!(!serde_json::to_string(&verdict).unwrap().contains("secret"));
    provides.mark_unobserved(axis.clone());
    assert!(provides.observed(&axis).is_none());
    assert_eq!(admit(&requires, &provides).decision, Decision::Refuse);
    provides.observe(axis.clone(), AxisValue::Scalar("linux".into()));
    assert!(!provides.is_unobserved(&axis));
    assert_eq!(admit(&requires, &provides).decision, Decision::Permit);
}

#[test]
fn bundle_identity_is_canonical_and_changes_with_every_document_field() {
    let v2 = X86_64_V2.to_bundle();
    let expected_features = [
        "cx16", "lahf_lm", "popcnt", "sse3", "sse4_1", "sse4_2", "ssse3",
    ];
    assert_eq!(
        v2.features,
        expected_features.into_iter().map(str::to_owned).collect()
    );
    let v3 = X86_64_V3.to_bundle();
    assert_eq!(
        v3.features
            .difference(&v2.features)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["avx", "avx2", "bmi1", "bmi2", "f16c", "fma", "lzcnt", "movbe", "osxsave"]
    );
    assert!(v2.features.is_subset(&v3.features));
    // Independent literal DAG-CBOR encoding: length-ordered map keys, sorted feature list.
    let expected = b"\xa3\x64axis\x67isa.x86\x64name\x69x86-64-v2\x68features\x87\x64cx16\x67lahf_lm\x66popcnt\x64sse3\x66sse4_1\x66sse4_2\x65ssse3";
    assert_eq!(v2.canonical_bytes().unwrap(), expected);
    assert!(cbor::decode_strict(expected).is_ok());
    let identity = v2.cid().unwrap();
    let mut reordered = v2.clone();
    reordered.features = v2.features.iter().rev().cloned().collect();
    assert_eq!(reordered.cid().unwrap(), identity);
    for mut changed in [v2.clone(), v2.clone(), v2.clone()] {
        changed.name.push('!');
        assert_ne!(changed.cid().unwrap(), identity);
        changed.name = v2.name.clone();
        changed.axis.0.push('!');
        assert_ne!(changed.cid().unwrap(), identity);
        changed.axis = v2.axis.clone();
        changed.features.insert("avx".into());
        assert_ne!(changed.cid().unwrap(), identity);
    }
}

#[test]
fn unknown_predicate_kinds_and_fields_are_rejected() {
    for json in [
        r#"[{"axis":"arch","source":"declared","predicate":{"kind":"future-kind","value":"x86_64"}}]"#,
        r#"[{"axis":"arch","source":"declared","predicate":{"kind":"equal","value":"x86_64"},"future":true}]"#,
    ] {
        assert!(serde_json::from_str::<Requires>(json).is_err());
    }
}

#[test]
fn narrowing_is_provenance_that_admit_ignores_and_the_wire_omits_when_absent() {
    use host_contract::Narrowing;
    let mut narrowed = clause(
        Predicate::Subset(["sse4_2".to_owned()].into()),
        ClauseSource::Extracted,
    );
    narrowed.narrowing = Some(Narrowing {
        removed: ["avx2".to_owned(), "bmi2".to_owned()].into(),
        evidence: "bafkreiexampledispatchevidence".to_owned(),
    });
    let requires = Requires::try_from(vec![narrowed.clone()]).expect("one clause merges");

    let wire = serde_json::to_value(&requires).expect("requires serializes");
    let decoded: Requires = serde_json::from_value(wire.clone()).expect("round trip decodes");
    assert_eq!(decoded, requires);
    assert!(wire.to_string().contains("\"narrowing\""));

    let plain = Requires::try_from(vec![clause(
        Predicate::Subset(["sse4_2".to_owned()].into()),
        ClauseSource::Extracted,
    )])
    .expect("one clause merges");
    let plain_wire = serde_json::to_value(&plain).expect("requires serializes");
    assert!(!plain_wire.to_string().contains("narrowing"));

    let mut host = Provides::default();
    host.observe(
        AxisId("test.axis".into()),
        AxisValue::Names(["sse4_2".to_owned()].into()),
    );
    assert_eq!(
        admit(&requires, &host).decision,
        admit(&plain, &host).decision
    );
}
