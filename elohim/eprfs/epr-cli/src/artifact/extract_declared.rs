//! The isa.x86 requirement a host is judged against is what the build DECLARED (steward ruling,
//! native-throughput sprint, 2026-11-18): the level in `target.isa` and the `RUSTFLAGS`
//! `-C target-cpu` / `-C target-feature`, the x86-64 baseline when none is given.
//!
//! The extractor reads every instruction in the bytes, including code reached only behind a
//! runtime CPUID check (aes/sha/memchr-style multiversioning in dependencies), so its union
//! over-states what a host must have: a baseline `epr` built on a Zen 2 host records vaes and
//! fma4 and still runs there. What it finds beyond the declaration is kept as the clause's
//! narrowing provenance and in the evidence document, never in the admitted subset.
//!
//! Residual, recorded rather than solved: a C dependency compiled with its own `-m` flags (or
//! hand-written assembly without a CPUID guard) is outside what `RUSTFLAGS` declares.
use std::collections::{BTreeMap, BTreeSet};

use host_contract::{
    names::axis, AxisId, Clause, ClauseSource, Narrowing, Predicate, Requires, X86_64_V2, X86_64_V3,
};
use serde_json::{json, Value};

use super::{isa, unrecorded, ArtifactResult};

/// What a build's descriptor says about its x86 instruction set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Declared {
    /// The declared features above the x86-64 baseline; empty means the baseline itself.
    Features(BTreeSet<String>),
    /// `target-cpu=native` or a CPU or level this reader cannot name: the bytes' extraction
    /// governs, as before the ruling.
    Undeclarable(String),
}

/// The declaration in a method descriptor's non-path inputs.
pub(crate) fn declared(non_path_inputs: &BTreeMap<String, String>) -> Declared {
    let mut level = 1;
    let mut extras = BTreeSet::new();
    if let Some(isa) = non_path_inputs.get("target.isa") {
        match level_of(isa.trim()) {
            Some(declared) => level = level.max(declared),
            None => return Declared::Undeclarable(format!("target.isa={}", isa.trim())),
        }
    }
    let flags = non_path_inputs.get("RUSTFLAGS").map_or("", String::as_str);
    for (key, value) in codegen(flags) {
        match key {
            "target-cpu" => match level_of(value) {
                Some(declared) => level = level.max(declared),
                None => return Declared::Undeclarable(format!("target-cpu={value}")),
            },
            "target-feature" => extras.extend(
                value
                    .split(',')
                    .filter_map(|feature| feature.trim().strip_prefix('+'))
                    .map(feature_name),
            ),
            _ => {}
        }
    }
    let mut features: BTreeSet<String> = match level {
        1 => BTreeSet::new(),
        2 => names(X86_64_V2.features),
        3 => names(X86_64_V3.features),
        _ => names(X86_64_V3.features)
            .into_iter()
            .chain(names(isa::V4))
            .collect(),
    };
    features.extend(extras);
    Declared::Features(features)
}

fn names(features: &[&str]) -> BTreeSet<String> {
    features.iter().map(|name| (*name).to_owned()).collect()
}

fn level_of(name: &str) -> Option<u8> {
    match name {
        "x86-64" => Some(1),
        "x86-64-v2" => Some(2),
        "x86-64-v3" => Some(3),
        "x86-64-v4" => Some(4),
        _ => None,
    }
}

/// `-C key=value`, `-Ckey=value`, `--codegen key=value` and `--codegen=key=value`, in order.
fn codegen(flags: &str) -> Vec<(&str, &str)> {
    let mut found = Vec::new();
    let mut tokens = flags.split_whitespace();
    while let Some(token) = tokens.next() {
        let option = match token {
            "-C" | "--codegen" => tokens.next(),
            _ => token
                .strip_prefix("--codegen=")
                .or_else(|| token.strip_prefix("-C")),
        };
        if let Some((key, value)) = option.and_then(|option| option.split_once('=')) {
            found.push((key, value));
        }
    }
    found
}

/// rustc's feature spelling to the contract's (psABI and Linux flag names).
fn feature_name(rust: &str) -> String {
    match rust.to_ascii_lowercase().as_str() {
        "sha" => "sha_ni".into(),
        "lahfsahf" => "lahf_lm".into(),
        "cmpxchg16b" => "cx16".into(),
        other => other.replace('.', "_"),
    }
}

/// Replace the extracted isa.x86 clause with the declared one. Features the bytes use beyond
/// the declaration become the clause's [`Narrowing`] (`evidence` names the method whose
/// declaration justified it) and the evidence document's `dispatched` list. A baseline
/// declaration requires nothing above the floor, so no clause is kept (spec §3.1 rule 4) and
/// the observation stays in the evidence document.
pub(super) fn govern(
    requires: &mut Requires,
    evidence: &mut BTreeMap<String, Value>,
    declared: &Declared,
    method: &str,
) -> ArtifactResult<()> {
    let Declared::Features(features) = declared else {
        return Ok(());
    };
    let observed: BTreeSet<String> = requires
        .clauses()
        .find(|clause| clause.axis().0 == axis::ISA_X86)
        .and_then(|clause| match clause.predicate() {
            Predicate::Subset(set) => Some(set.clone()),
            _ => None,
        })
        .unwrap_or_default();
    let required = isa::level_features(features).unwrap_or_default();
    let dispatched: BTreeSet<String> = observed.difference(&required).cloned().collect();
    let clauses: Vec<Clause> = serde_json::from_value(serde_json::to_value(&*requires)?)?;
    let mut rebuilt = Requires::default();
    for clause in clauses
        .into_iter()
        .filter(|clause| clause.axis.0 != axis::ISA_X86)
    {
        rebuilt
            .insert(clause)
            .map_err(|e| unrecorded(format!("MergeError: {e}")))?;
    }
    if !required.is_empty() {
        rebuilt
            .insert(Clause {
                axis: AxisId(axis::ISA_X86.into()),
                predicate: Predicate::Subset(required.clone()),
                source: ClauseSource::Declared,
                narrowing: (!dispatched.is_empty()).then(|| Narrowing {
                    removed: dispatched.clone(),
                    evidence: method.to_owned(),
                }),
            })
            .map_err(|e| unrecorded(format!("MergeError: {e}")))?;
    }
    *requires = rebuilt;
    let entry = evidence
        .entry(axis::ISA_X86.to_owned())
        .or_insert_with(|| json!({}));
    if let Value::Object(entry) = entry {
        entry.insert("declared".into(), json!(required));
        entry.insert("dispatched".into(), json!(dispatched));
    }
    Ok(())
}

#[cfg(test)]
#[path = "extract_declared_tests.rs"]
mod tests;
