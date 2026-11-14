//! Host admission against a synthetic observed host: requirement-side effects only,
//! the own-build rule's conditions, and the no-host-values wire goldens. Candidate
//! grants are tested separately from core authentication.

use std::collections::BTreeSet;

mod support;
use ark_supervisor::admission::AdmissionRecord;
use ark_supervisor::admission::{admit_artifact, admit_recorded_requirements, UNPROBED_X86};
use cid::Cid;
use elohim_epr::cid::compute_cid;
use elohim_epr::verdict::Decision;
use host_contract::{AxisId, AxisValue, Clause, ClauseSource, Predicate, Provides, Requires};

#[path = "support/candidate.rs"]
mod candidate;

#[test]
fn authenticated_content_needs_a_matching_admitted_record() {
    use ark_core::candidate::CandidateRefusal;
    use ark_supervisor::admission::{admit_candidate, PlatformRefusal};
    let authenticated = candidate::authenticated();
    let output = authenticated.content().candidate.executable;
    let requires = native_requires(vec![isa_clause(ClauseSource::Extracted, &["avx2"])]).unwrap();
    let recorded = support::resolve(output, Some(&requires), true).unwrap();
    let admitted = admit_candidate(
        authenticated.clone(),
        recorded,
        &synthetic_host(),
        &BTreeSet::new(),
    )
    .unwrap();
    assert_eq!(admitted.authenticated(), &authenticated);
    assert_eq!(admitted.executable(), output);
    assert_eq!(admitted.verdict().decision, Decision::Permit);
    assert_eq!(admitted.own_build_rule().len(), 1);
    assert_eq!(
        admitted.own_produce(),
        Some(compute_cid(b"this device's Produce"))
    );
    let record = admitted.record();
    let wire = serde_json::to_string(&record).unwrap();
    assert_eq!(
        serde_json::from_str::<AdmissionRecord>(&wire).unwrap(),
        record
    );
    for (other_output, own, withdrawn) in [
        (compute_cid(b"other"), true, BTreeSet::new()),
        (output, false, BTreeSet::new()),
        (output, true, BTreeSet::from([output])),
    ] {
        let recorded = support::resolve(other_output, Some(&requires), own).unwrap();
        let refusal = admit_candidate(
            authenticated.clone(),
            recorded,
            &synthetic_host(),
            &withdrawn,
        )
        .unwrap_err();
        assert_eq!(refusal.label(), "candidate_wrong_platform");
        if other_output != output {
            assert!(matches!(
                refusal,
                CandidateRefusal::WrongPlatform(
                    PlatformRefusal::RequirementForOtherExecutable { .. }
                )
            ));
        } else {
            assert!(matches!(
                refusal,
                CandidateRefusal::WrongPlatform(PlatformRefusal::Refused(_))
            ));
        }
    }
    let missing = support::resolve(output, Some(&Requires::default()), false).unwrap();
    assert!(matches!(
        admit_candidate(
            authenticated.clone(),
            missing,
            &synthetic_host(),
            &BTreeSet::new()
        ),
        Err(CandidateRefusal::WrongPlatform(_))
    ));
    let plain = support::resolve(output, Some(&native_requires(vec![]).unwrap()), false).unwrap();
    assert!(
        admit_candidate(authenticated, plain, &synthetic_host(), &BTreeSet::new())
            .unwrap()
            .own_build_rule()
            .is_empty()
    );
}

#[test]
fn reader_gaps_and_output_mismatch_never_admit() {
    use ark_supervisor::admission::ResolveGap;
    use serde_json::json;
    let case = fixture();
    let base = json!({"output":case.executable.to_string(), "self":false,
        "produce":null, "known":true, "requires":null, "path":null});
    for (status, extra, diagnostic, expected) in [
        (
            0,
            json!({"unmeasured":"no recorded requirement"}),
            "",
            ResolveGap::NotRecorded,
        ),
        (
            2,
            json!({"known":false}),
            "",
            ResolveGap::UnknownOutput(case.executable),
        ),
        (
            3,
            json!({"unavailable":[compute_cid(b"absent requirement").to_string()]}),
            "",
            ResolveGap::Unavailable,
        ),
        (
            3,
            json!({"gap":"unreadable env:requires slot"}),
            "",
            ResolveGap::ReaderGap("unreadable env:requires slot".into()),
        ),
        (
            4,
            json!({"conflict":"incompatible clauses"}),
            "",
            ResolveGap::Conflict,
        ),
        (
            2,
            json!({}),
            "error: unrecognized subcommand 'requires'",
            ResolveGap::NoReader,
        ),
        (
            1,
            json!({}),
            "reader failed",
            ResolveGap::ReaderGap("artifact reader failed".into()),
        ),
    ] {
        let mut reply = base.clone();
        reply
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        if !diagnostic.is_empty() {
            // A missing verb or process failure has no K13 JSON answer.
            reply = json!(null);
        }
        assert_eq!(
            support::stub(case.executable, None, reply.clone(), status, diagnostic).unwrap_err(),
            expected
        );
        if diagnostic.is_empty() {
            reply["output"] = json!(compute_cid(b"other output").to_string());
            assert_eq!(
                support::stub(case.executable, None, reply, status, "").unwrap_err(),
                ResolveGap::Malformed("output mismatch".into())
            );
        }
    }
    for (status, extra) in [
        (0, json!({})),
        (0, json!({"known":false})),
        (2, json!({})),
        (3, json!({})),
        (3, json!({"unavailable":[]})),
        (4, json!({})),
    ] {
        let mut reply = base.clone();
        reply
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        assert!(matches!(
            support::stub(case.executable, None, reply, status, ""),
            Err(ResolveGap::Malformed(_))
        ));
    }
    let mut reply = json!({"known":true, "sources":[], "output":compute_cid(b"different output").to_string(), "requires":compute_cid(b"requires").to_string(), "self":true, "produce":compute_cid(b"produce").to_string(), "path":"WRITE_PATH"});
    assert_eq!(
        support::stub(
            case.executable,
            case.requires.as_ref(),
            reply.clone(),
            0,
            ""
        )
        .unwrap_err(),
        ResolveGap::Malformed("output mismatch".into())
    );
    reply["requires"] = json!(elohim_epr::cid::compute_bytes_cid(
        &serde_json::to_vec(&case.requires).unwrap()
    )
    .to_string());
    reply["output"] = json!(case.executable.to_string());
    reply["produce"] = json!("not a CID");
    assert!(matches!(
        support::stub(
            case.executable,
            case.requires.as_ref(),
            reply.clone(),
            0,
            ""
        ),
        Err(ResolveGap::Malformed(_))
    ));
    reply["self"] = json!(false);
    assert!(support::stub(
        case.executable,
        case.requires.as_ref(),
        reply.clone(),
        0,
        ""
    )
    .is_ok());
    reply["self"] = json!(true);
    reply["produce"] = json!(null);
    let recorded = support::stub(case.executable, case.requires.as_ref(), reply, 0, "").unwrap();
    assert_eq!(
        admit_artifact(&recorded, &case.provides, &BTreeSet::new()).own_produce(),
        None
    );
}

fn set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|s| (*s).to_owned()).collect()
}

/// Admission inputs as a candidate caller resolves them.
#[derive(Clone)]
struct Case {
    executable: Cid,
    requires: Option<Requires>,
    provides: Provides,
    own_produce: Option<Cid>,
    withdrawn_artifacts: BTreeSet<Cid>,
}

fn fixture() -> Case {
    // Raw CID parsed from an external, fixed empty-blob fixture.
    let executable: Cid = "bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenxquvyku"
        .parse()
        .unwrap();
    Case {
        executable,
        requires: Some(
            native_requires(vec![isa_clause(ClauseSource::Declared, &["sse3"])]).unwrap(),
        ),
        provides: synthetic_host(),
        own_produce: None,
        withdrawn_artifacts: BTreeSet::new(),
    }
}

fn admit(case: &Case) -> AdmissionRecord {
    let recorded = support::resolve(
        case.executable,
        case.requires.as_ref(),
        case.own_produce == Some(case.executable),
    )
    .unwrap();
    admit_artifact(&recorded, &case.provides, &case.withdrawn_artifacts)
}

fn isa_clause(source: ClauseSource, features: &[&str]) -> Clause {
    Clause {
        axis: AxisId("isa.x86".into()),
        predicate: Predicate::Subset(set(features)),
        source,
        narrowing: None,
    }
}

fn synthetic_host() -> Provides {
    let mut host = Provides::default();
    host.observe(
        AxisId("isa.x86".into()),
        AxisValue::Names(set(&["sse3", "private-host-marker"])),
    );
    host.observe(AxisId("arch".into()), AxisValue::Scalar("x86_64".into()));
    host.observe(AxisId("os".into()), AxisValue::Scalar("linux".into()));
    host
}

fn platform_clause(axis: &str, value: &str) -> Clause {
    Clause {
        axis: AxisId(axis.into()),
        predicate: Predicate::Equal(value.into()),
        source: ClauseSource::Declared,
        narrowing: None,
    }
}

fn native_requires(mut clauses: Vec<Clause>) -> Result<Requires, host_contract::MergeError> {
    for (axis, value) in [("arch", "x86_64"), ("os", "linux")] {
        if !clauses.iter().any(|c| c.axis.0 == axis) {
            clauses.push(platform_clause(axis, value));
        }
    }
    Requires::try_from(clauses)
}

fn local_output(executable: Cid) -> Cid {
    executable
}

fn summary_of<'a>(effect: &'a AdmissionRecord, suffix: &str) -> &'a str {
    &effect
        .verdict()
        .witness
        .checks
        .iter()
        .find(|c| c.check_id.ends_with(suffix))
        .unwrap()
        .summary
}

#[test]
fn observed_requirements_admit_for_the_named_executable() {
    let case = fixture();
    let result = admit(&case);
    assert_eq!(result.verdict().decision, Decision::Permit);
    assert!(result.permits());
    assert!(result.own_build_rule().is_empty());
    assert_eq!(result.executable(), case.executable);
    assert_eq!(
        result.verdict().subject.as_deref(),
        Some(case.executable.to_string().as_str())
    );
}

#[test]
fn unmet_isa_names_required_axis_without_host_values() {
    let mut case = fixture();
    case.requires =
        Some(native_requires(vec![isa_clause(ClauseSource::Extracted, &["avx2"])]).unwrap());
    let result = admit(&case);
    assert_eq!(result.verdict().decision, Decision::Refuse);
    let wire = serde_json::to_value(&result).unwrap();
    let check = wire["verdict"]["witness"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["checkId"] == "host-compatibility:isa.x86")
        .unwrap();
    assert_eq!(check["summary"], "unmet");
    assert_eq!(check["observed"]["required"]["axis"], "isa.x86");
    assert_eq!(
        check["observed"]["required"]["predicate"]["value"],
        serde_json::json!(["avx2"])
    );
    assert!(!wire.to_string().contains("private-host-marker"));
    assert!(!wire.to_string().contains("private-architecture-marker"));
    assert!(!wire.to_string().contains("\"host\""));
    assert!(!wire.to_string().contains("provides"));
}

#[test]
fn unknown_and_unobserved_refuse_even_own_builds() {
    let mut case = fixture();
    case.own_produce = Some(local_output(case.executable));
    case.requires =
        Some(native_requires(vec![isa_clause(ClauseSource::Extracted, &["avx2"])]).unwrap());
    case.provides = Provides::default();
    for reason in ["unknown-axis", "unobserved"] {
        if reason == "unobserved" {
            case.provides.mark_unobserved(AxisId("isa.x86".into()));
        }
        let result = admit(&case);
        assert_eq!(result.verdict().decision, Decision::Refuse);
        assert_eq!(summary_of(&result, ":isa.x86"), reason);
        assert!(result.own_build_rule().is_empty());
    }
    let clause = Clause {
        axis: AxisId("future.axis".into()),
        predicate: Predicate::Equal("required".into()),
        source: ClauseSource::Declared,
        narrowing: None,
    };
    case.requires = Some(native_requires(vec![clause]).unwrap());
    assert_eq!(summary_of(&admit(&case), ":future.axis"), "unknown-axis");
}

#[test]
fn absent_and_empty_native_requirements_refuse() {
    let mut case = fixture();
    assert_eq!(
        support::resolve(case.executable, None, true).unwrap_err(),
        ark_supervisor::admission::ResolveGap::NotRecorded
    );
    case.requires = Some(Requires::default());
    assert_eq!(admit(&case).verdict().decision, Decision::Refuse);
    // The recorded-requirement judgement fails closed the same way.
    assert_eq!(
        admit_recorded_requirements(None, &case.provides).decision,
        Decision::Refuse
    );
}

#[test]
fn recorded_requirements_never_use_the_own_build_rule() {
    let case = fixture();
    let requires = native_requires(vec![isa_clause(ClauseSource::Extracted, &["avx2"])]).unwrap();
    let verdict = admit_recorded_requirements(Some(&requires), &case.provides);
    assert_eq!(verdict.decision, Decision::Refuse);
    assert!(verdict
        .witness
        .checks
        .iter()
        .all(|c| c.summary != "permit-under-own-build-rule"));
    let requires = case.requires.as_ref().unwrap();
    assert_eq!(
        admit_recorded_requirements(Some(requires), &case.provides).decision,
        Decision::Permit
    );
}

#[test]
fn own_build_permit_names_every_waived_clause() {
    let mut case = fixture();
    case.own_produce = Some(local_output(case.executable));
    let waived = vec![
        isa_clause(ClauseSource::Extracted, &["avx2"]),
        isa_clause(ClauseSource::Extracted, &["bmi2"]),
    ];
    let mut clauses = waived.clone();
    clauses.push(isa_clause(ClauseSource::Declared, &["sse3"]));
    case.requires = Some(native_requires(clauses).unwrap());
    let result = admit(&case);
    assert_eq!(result.verdict().decision, Decision::Permit);
    // The host-contract-free waiver spells each clause exactly as the contract does.
    assert_eq!(
        serde_json::to_value(result.own_build_rule()).unwrap(),
        serde_json::to_value(&waived).unwrap()
    );
    assert_eq!(
        result
            .verdict()
            .witness
            .checks
            .iter()
            .filter(|c| c.summary == "permit-under-own-build-rule")
            .count(),
        2
    );
    assert!(!serde_json::to_string(&result)
        .unwrap()
        .contains("private-host-marker"));
}

#[test]
fn declared_proven_non_isa_sibling_and_withdrawn_refuse() {
    let base = fixture();
    for source in [ClauseSource::Declared, ClauseSource::Proven] {
        let mut case = base.clone();
        case.own_produce = Some(local_output(case.executable));
        case.requires = Some(
            native_requires(vec![
                isa_clause(ClauseSource::Extracted, &["bmi2"]),
                isa_clause(source, &["avx2"]),
            ])
            .unwrap(),
        );
        let result = admit(&case);
        assert_eq!(result.verdict().decision, Decision::Refuse);
        assert!(result.own_build_rule().is_empty());
    }
    let mut case = base;
    case.own_produce = Some(local_output(case.executable));
    case.requires = Some(
        native_requires(vec![Clause {
            axis: AxisId("arch".into()),
            predicate: Predicate::Equal("aarch64".into()),
            source: ClauseSource::Extracted,
            narrowing: None,
        }])
        .unwrap(),
    );
    assert_eq!(admit(&case).verdict().decision, Decision::Refuse);
    case.requires =
        Some(native_requires(vec![isa_clause(ClauseSource::Extracted, &["avx2"])]).unwrap());
    case.own_produce = None;
    assert_eq!(admit(&case).verdict().decision, Decision::Refuse);
    case.own_produce = Some(local_output(case.executable));
    case.withdrawn_artifacts.insert(case.executable);
    assert_eq!(admit(&case).verdict().decision, Decision::Refuse);
    case.withdrawn_artifacts.clear();
    case.withdrawn_artifacts
        .insert(compute_cid(b"another artifact"));
    assert_eq!(admit(&case).verdict().decision, Decision::Permit);
}

#[test]
fn native_candidates_require_both_arch_and_os() {
    for clauses in [
        vec![],
        vec![platform_clause("arch", "x86_64")],
        vec![platform_clause("os", "linux")],
    ] {
        let mut case = fixture();
        case.requires = Some(Requires::try_from(clauses.clone()).unwrap());
        case.own_produce = Some(local_output(case.executable));
        let result = admit(&case);
        assert_eq!(result.verdict().decision, Decision::Refuse);
        for axis in ["arch", "os"] {
            if !clauses.iter().any(|c| c.axis.0 == axis) {
                assert!(result
                    .verdict()
                    .witness
                    .checks
                    .iter()
                    .any(|c| c.check_id == format!("host-compatibility:{axis}")
                        && c.summary == "missing-requirement"));
            }
        }
    }
}

#[test]
fn unprobed_features_refuse_without_an_own_build_waiver() {
    for feature in UNPROBED_X86 {
        let mut case = fixture();
        case.own_produce = Some(local_output(case.executable));
        case.requires =
            Some(native_requires(vec![isa_clause(ClauseSource::Extracted, &[feature])]).unwrap());
        // Even a supplied inventory cannot promote a feature ark cannot probe.
        for features in [vec![], vec![*feature]] {
            case.provides
                .observe(AxisId("isa.x86".into()), AxisValue::Names(set(&features)));
            let result = admit(&case);
            assert_eq!(result.verdict().decision, Decision::Refuse, "{feature}");
            assert!(result
                .verdict()
                .witness
                .checks
                .iter()
                .any(|c| c.check_id == "host-compatibility:isa.x86" && c.summary == "unobserved"));
            assert!(result.own_build_rule().is_empty());
            assert_eq!(result.own_produce(), None);
        }
    }
}

#[test]
fn mixed_axis_waives_only_the_failed_extracted_evidence() {
    for (extracted, declared, decision) in [
        ("avx2", "sse3", Decision::Permit),
        ("sse3", "avx2", Decision::Refuse),
    ] {
        let mut case = fixture();
        case.own_produce = Some(local_output(case.executable));
        case.requires = Some(
            native_requires(vec![
                isa_clause(ClauseSource::Extracted, &[extracted]),
                isa_clause(ClauseSource::Declared, &[declared]),
            ])
            .unwrap(),
        );
        let result = admit(&case);
        assert_eq!(result.verdict().decision, decision);
        assert_eq!(
            result.own_build_rule().len(),
            usize::from(decision == Decision::Permit)
        );
        assert_eq!(result.own_produce().is_some(), decision == Decision::Permit);
    }
}

#[test]
fn produce_for_another_executable_cannot_waive() {
    let mut case = fixture();
    case.requires =
        Some(native_requires(vec![isa_clause(ClauseSource::Extracted, &["avx2"])]).unwrap());
    case.own_produce = Some(local_output(compute_cid(b"other executable")));
    let result = admit(&case);
    assert_eq!(result.verdict().decision, Decision::Refuse);
    assert_eq!(result.own_produce(), None);
}

#[test]
fn admission_wire_goldens_disclose_effect_and_produce_only() {
    use host_contract::Version;
    use serde_json::json;
    let mut case = fixture();
    case.requires =
        Some(native_requires(vec![isa_clause(ClauseSource::Extracted, &["avx2"])]).unwrap());
    for (axis, value) in [
        (
            "abi.libc",
            AxisValue::Version(Version {
                line: "glibc".into(),
                components: vec![2, 987],
            }),
        ),
        (
            "kernel.linux",
            AxisValue::Version(Version {
                line: "linux".into(),
                components: vec![6, 987, 654],
            }),
        ),
        (
            "abi.loader",
            AxisValue::Names(set(&["/private/host/ld-linux.so.2"])),
        ),
    ] {
        case.provides.observe(AxisId(axis.into()), value);
    }
    let produce = compute_cid(b"this device's Produce");
    let arch_check = json!({"checkId":"host-compatibility:arch", "outcome":"passed",
        "summary":"satisfied", "observed":{"required":{"axis":"arch",
        "predicate":{"kind":"equal","value":"x86_64"},"source":"declared"}}});
    let os_check = json!({"checkId":"host-compatibility:os", "outcome":"passed",
        "summary":"satisfied", "observed":{"required":{"axis":"os",
        "predicate":{"kind":"equal","value":"linux"},"source":"declared"}}});
    let required = json!({"axis":"isa.x86","predicate":{"kind":"subset","value":["avx2"]},
        "source":"extracted"});
    for own in [false, true] {
        case.own_produce = own.then(|| local_output(case.executable));
        let result = admit(&case);
        let evidence = if own {
            json!({"required":required,"ownProduce":produce})
        } else {
            json!({"required":required})
        };
        let golden = json!({
            "executable": case.executable,
            "ownBuildRule": if own { vec![required.clone()] } else { vec![] },
            "ownProduce": if own { Some(produce) } else { None },
            "verdict": {"axis":"host-compatibility", "subject":case.executable.to_string(),
                "decision":{"type":if own {"permit"} else {"refuse"}}, "policyRef":null,
                "witness":{"checks":[arch_check, {
                    "checkId":"host-compatibility:isa.x86", "outcome":if own {"passed"} else {"failed"},
                    "summary":if own {"permit-under-own-build-rule"} else {"unmet"},
                    "observed":evidence}, os_check]}}
        });
        let wire = serde_json::to_value(&result).unwrap();
        assert_eq!(wire, golden);
        // The recorded effect round-trips through its own wire form.
        assert_eq!(
            serde_json::from_value::<AdmissionRecord>(wire.clone()).unwrap(),
            result
        );
        let wire = wire.to_string();
        for private in [
            "private-host-marker",
            "sse3",
            "glibc",
            "2.987",
            "987",
            "654",
            "6.987.654",
            "/private/host/ld-linux.so.2",
        ] {
            assert!(!wire.contains(private), "leaked {private}: {wire}");
        }
    }
    let debug = format!("{:?}", case.provides);
    for axis in [
        "arch",
        "os",
        "isa.x86",
        "abi.libc",
        "abi.loader",
        "kernel.linux",
    ] {
        assert!(debug.contains(axis));
    }
    for private in [
        "private-host-marker",
        "sse3",
        "glibc",
        "987",
        "654",
        "/private/host",
    ] {
        assert!(!debug.contains(private), "leaked {private}: {debug}");
    }
}

#[test]
fn written_requirement_is_bounded_and_matches_its_cid_including_merged_documents() {
    use ark_supervisor::admission::ResolveGap;
    use elohim_epr::cid::compute_bytes_cid;
    use serde_json::json;
    let output = compute_cid(b"output");
    let mut document = b"[]".to_vec();
    let mut reply = json!({"output":output.to_string(), "known":true, "self":false,
        "produce":null, "requires":compute_bytes_cid(&document).to_string(), "path":"WRITE_PATH",
        "sources":[compute_bytes_cid(b"source one").to_string(), compute_bytes_cid(b"source two").to_string()]});
    assert!(support::stub_bytes(output, Some(&document), reply.clone(), 0, "", "").is_ok());
    // Even harmless whitespace changes must match the exact written document's CID.
    document.push(b' ');
    assert_eq!(
        support::stub_bytes(output, Some(&document), reply.clone(), 0, "", "").unwrap_err(),
        ResolveGap::Malformed("requirement CID does not match written bytes".into())
    );
    document.resize(1024 * 1024, b' ');
    reply["requires"] = json!(compute_bytes_cid(&document).to_string());
    assert!(support::stub_bytes(output, Some(&document), reply.clone(), 0, "", "").is_ok());
    document.push(b' ');
    reply["requires"] = json!(compute_bytes_cid(&document).to_string());
    assert_eq!(
        support::stub_bytes(output, Some(&document), reply, 0, "", "").unwrap_err(),
        ResolveGap::Malformed("document exceeds 1 MiB".into())
    );
}

#[test]
fn sleeping_reader_is_killed_and_reaped_at_the_deadline() {
    use ark_supervisor::admission::ResolveGap;
    use serde_json::json;
    let tmp = tempfile::tempdir().unwrap();
    let pid_path = tmp.path().join("pid");
    let before = format!("echo $$ > '{}'; exec sleep 60", pid_path.display());
    let started = std::time::Instant::now();
    assert_eq!(
        support::stub_bytes(compute_cid(b"output"), None, json!(null), 0, "", &before).unwrap_err(),
        ResolveGap::ReaderGap("artifact reader timed out after 30 s".into())
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(45));
    let pid = std::fs::read_to_string(pid_path).unwrap();
    assert!(
        !std::path::Path::new(&format!("/proc/{}", pid.trim())).exists(),
        "reader was not reaped"
    );
}
