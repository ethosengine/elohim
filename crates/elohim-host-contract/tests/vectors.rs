use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::Path;

use elohim_epr::verdict::{CheckOutcome, Decision};
use host_contract::{admit, AxisId, AxisValue, Predicate, Provides, Refusal, Requires};
use serde::Deserialize;

/// Test-only synthetic host input; production Provides never implements serde.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SyntheticProvides {
    observed: BTreeMap<AxisId, AxisValue>,
    unobserved: BTreeSet<AxisId>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Vector {
    requires: Requires,
    provides: SyntheticProvides,
    expected: Decision,
    #[serde(default)]
    merge: Option<Requires>,
    #[serde(default)]
    expected_refusals: Vec<Refusal>,
    #[serde(default)]
    expected_merged: BTreeMap<AxisId, Predicate>,
    #[serde(default)]
    expected_disagreements: usize,
}

#[test]
fn every_json_vector() -> Result<(), Box<dyn Error>> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors");
    let mut paths = fs::read_dir(directory)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|path| {
        path.extension()
            .is_some_and(|extension| extension == "json")
    });
    paths.sort();
    assert!(
        !paths.is_empty(),
        "the conformance directory must not be empty"
    );
    for path in paths {
        let vector: Vector = serde_json::from_slice(&fs::read(&path)?)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        let mut requires = vector.requires;
        if let Some(other) = vector.merge {
            let reverse = other.merge(&requires)?;
            requires = requires.merge(&other)?;
            assert_eq!(requires, reverse, "merge order: {}", path.display());
        }
        assert_eq!(
            requires.disagreements().len(),
            vector.expected_disagreements,
            "disagreements: {}",
            path.display()
        );
        for (axis, expected) in vector.expected_merged {
            let clause = requires
                .clauses()
                .find(|clause| clause.axis() == &axis)
                .unwrap();
            assert_eq!(clause.predicate(), &expected, "merge: {}", path.display());
        }
        let mut provides = Provides::default();
        for axis in vector.provides.unobserved {
            assert!(
                !vector.provides.observed.contains_key(&axis),
                "ambiguous fixture axis"
            );
            provides.mark_unobserved(axis);
        }
        for (axis, value) in vector.provides.observed {
            provides.observe(axis, value);
        }
        let verdict = admit(&requires, &provides);
        assert_eq!(verdict.decision, vector.expected, "{}", path.display());
        assert_eq!(verdict.witness.checks.len(), requires.clauses().len());
        assert_eq!(admit(&requires, &provides), verdict, "deterministic repeat");
        let mut refusals = Vec::new();
        for (check, clause) in verdict.witness.checks.iter().zip(requires.clauses()) {
            assert_eq!(
                check.check_id,
                format!("host-compatibility:{}", clause.axis().0)
            );
            let observed = check.observed.as_ref().unwrap();
            assert_eq!(
                observed["required"],
                serde_json::to_value(clause.predicate())?
            );
            assert_eq!(
                observed["evidence"],
                serde_json::to_value(clause.evidence())?
            );
            assert_eq!(
                observed["host"],
                serde_json::to_value(provides.observed(clause.axis()))?
            );
            let refusal: Option<Refusal> = serde_json::from_value(observed["refusal"].clone())?;
            if let Some(refusal) = refusal {
                assert_eq!(check.outcome, CheckOutcome::Failed);
                assert!(check.summary.contains(refusal.label()));
                refusals.push(refusal);
            } else {
                assert_eq!(check.outcome, CheckOutcome::Passed);
            }
        }
        assert_eq!(
            refusals,
            vector.expected_refusals,
            "refusals: {}",
            path.display()
        );
        let encoded = serde_json::to_vec(&requires)?;
        assert_eq!(serde_json::from_slice::<Requires>(&encoded)?, requires);
    }
    Ok(())
}
