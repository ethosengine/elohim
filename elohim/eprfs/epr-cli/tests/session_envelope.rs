//! The shipped CLI and hooks share the a2o fixture, including real flow claims and closure.
fn run_fixture_stage(base: &std::path::Path, stage: &str) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let result = std::process::Command::new("python3")
        .arg(root.join("genesis/agentic/session_envelope_test.py"))
        .args(["--stage", stage, "--base"])
        .arg(base)
        .env("EPR_BIN", env!("CARGO_BIN_EXE_epr"))
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

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
        run_fixture_stage(fixture.path(), stage);
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

#[test]
fn gc_continues_after_one_dead_owner_record_cannot_close() {
    let fixture = tempfile::tempdir().unwrap();
    run_fixture_stage(fixture.path(), "setup");
    run_fixture_stage(fixture.path(), "acquire");

    let sessions = fixture.path().join("config-home/elohim/device/sessions");
    let failed_session = "claude-code-4101-77";
    let closed_session = "codex-4102-77";
    let failed_path = sessions.join(format!("{failed_session}.json"));
    let mut failed_record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&failed_path).unwrap()).unwrap();
    let non_repository = fixture.path().join("not-a-repository");
    std::fs::create_dir(&non_repository).unwrap();
    failed_record["repository"] = serde_json::json!(non_repository);
    std::fs::write(&failed_path, serde_json::to_vec(&failed_record).unwrap()).unwrap();
    std::fs::remove_dir_all(fixture.path().join("proc/4101")).unwrap();
    std::fs::remove_dir_all(fixture.path().join("proc/4102")).unwrap();

    let root = fixture.path().join("repo");
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_epr"));
    command
        .args(["session", "gc", "--json"])
        .current_dir(&root)
        .env("EPR_UNDER_TEST", "1")
        .env(
            "EPR_AGENT_RUNTIMES",
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../genesis/agentic/agent-runtimes.json"),
        )
        .env("BIN_STORE", fixture.path().join("stores"))
        .env("EPR_PROC_ROOT", fixture.path().join("proc"))
        .env("XDG_CONFIG_HOME", fixture.path().join("config-home"))
        .env("CLAUDE_PROJECT_DIR", &root)
        .env("CLAUDE_CONFIG_DIR", fixture.path().join("config"))
        .env("BERTH_DIR", fixture.path().join("berth"))
        .env("BERTH_UNDER_TEST", "1")
        .env("BRIT_BUILD_REF_BIN", fixture.path().join("brit-stub"))
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1");
    let output = command.output().unwrap();

    assert!(!output.status.success());
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(result["closed"]
        .as_array()
        .unwrap()
        .iter()
        .any(|record| record["identity"]["session"] == closed_session));
    assert!(result["failed"].as_array().unwrap().iter().any(|failure| {
        failure["session"] == failed_session
            && failure["error"]
                .as_str()
                .is_some_and(|error| !error.is_empty())
    }));
    let closed_record: serde_json::Value = serde_json::from_slice(
        &std::fs::read(sessions.join(format!("{closed_session}.json"))).unwrap(),
    )
    .unwrap();
    assert!(closed_record["closedAt"].is_string());
}
