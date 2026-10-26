//! The shipped CLI and hooks share the a2o fixture, including real flow claims and closure.
#[test]
fn runtime_child_resumes_without_inheriting_authority() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let result = std::process::Command::new("python3")
        .arg(root.join("genesis/agentic/session_envelope_test.py"))
        .arg("ResumeEnvelopeTest.test_runtime_child_resumes_without_inheriting_authority")
        .env("EPR_BIN", env!("CARGO_BIN_EXE_epr"))
        .output()
        .expect("native resume fixture starts");
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn session_envelope_native_facets_and_hook_pin_regressions() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let result = std::process::Command::new("python3")
        .arg(root.join("genesis/agentic/session_envelope_test.py"))
        .env("EPR_BIN", env!("CARGO_BIN_EXE_epr"))
        .output()
        .expect("python fixture starts");
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn native_session_views_match_the_wire_schema() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let fixture = tempfile::tempdir().unwrap();
    for stage in [
        "setup",
        "acquire",
        "joined",
        "close_claude",
        "collect",
        "resume_schema",
    ] {
        let result = std::process::Command::new("python3")
            .arg(root.join("genesis/agentic/session_envelope_test.py"))
            .args(["--stage", stage, "--base"])
            .arg(fixture.path())
            .env("EPR_BIN", env!("CARGO_BIN_EXE_epr"))
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../sdk/schemas/v1/views/session-envelope.schema.json"
    ))
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    for runtime in ["claude", "codex", "successor"] {
        let view: serde_json::Value = serde_json::from_slice(
            &std::fs::read(fixture.path().join(format!("view-{runtime}.json"))).unwrap(),
        )
        .unwrap();
        validator.validate(&view).unwrap();
        if runtime == "successor" {
            assert_eq!(view["envelope"]["generation"], 2);
            assert!(view["envelope"]["predecessor"]["recordCid"].is_string());
            assert!(view["envelope"]["predecessor"]["covenantSeal"].is_string());
            let mut malformed = view.clone();
            malformed["envelope"]["predecessor"]["recordCid"] = serde_json::json!(42);
            assert!(validator.validate(&malformed).is_err());
        }
    }
    for (verb, output) in [
        ("open", "open-claude.json"),
        ("open", "open-successor.json"),
        ("close", "close.json"),
        ("gc", "gc.json"),
    ] {
        let contract: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join(format!(
                "elohim/sdk/schemas/v1/views/session-envelope-{verb}.schema.json"
            )))
            .unwrap(),
        )
        .unwrap();
        let validator = jsonschema::options()
            .with_resource(
                "epr:schema:view:session-envelope",
                jsonschema::Resource::from_contents(schema.clone()).unwrap(),
            )
            .build(&contract)
            .unwrap();
        let actual: serde_json::Value =
            serde_json::from_slice(&std::fs::read(fixture.path().join(output)).unwrap()).unwrap();
        validator.validate(&actual).unwrap();
    }
}
