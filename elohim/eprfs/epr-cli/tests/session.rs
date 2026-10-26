use serde_json::Value;
#[test]
fn session_cli_none_and_registry_absence_are_successful() {
    let tmp = tempfile::tempdir().unwrap();
    for args in [vec!["session", "id"], vec!["session", "id", "--json"]] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_epr"))
            .args(&args)
            .env_clear()
            .env("EPR_PROC_ROOT", tmp.path())
            .env("EPR_AGENT_RUNTIMES", tmp.path().join("missing"))
            .current_dir(tmp.path())
            .output()
            .unwrap();
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        if args.len() == 2 {
            assert_eq!(text, "session=none runtime=none source=none owner=-\n");
        } else {
            let value: Value = serde_json::from_str(&text).unwrap();
            assert_eq!(value["source"], "none");
        }
    }
}

#[test]
fn process_and_registry_overrides_work_without_test_opt_in() {
    let tmp = tempfile::tempdir().unwrap();
    let owner = std::process::id().to_string();
    // Production isolation can deliberately hide a live parent from resolution.
    for switch in [None, Some("0"), Some("1")] {
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_epr"));
        command
            .args(["session", "id", "--json"])
            .env_clear()
            .env("EPR_PROC_ROOT", tmp.path())
            .env("ELOHIM_SESSION_ID", "live-owner")
            .env("ELOHIM_SESSION_OWNER_PID", &owner)
            .current_dir(tmp.path());
        if let Some(value) = switch {
            command.env("EPR_UNDER_TEST", value);
        }
        let output = command.output().unwrap();
        assert!(output.status.success());
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(value["ownerPid"].is_null());
    }
    let comm = std::fs::read_to_string("/proc/self/comm").unwrap();
    let registry = tmp.path().join("runtimes.json");
    std::fs::write(
        &registry,
        serde_json::to_vec(&serde_json::json!({"runtimes": [{
            "id": "fixture-parent", "comm": [comm.trim()], "sessionEnv": ["FIXTURE_SESSION"]
        }]}))
        .unwrap(),
    )
    .unwrap();
    for switch in [None, Some("0"), Some("1")] {
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_epr"));
        command
            .args(["session", "id", "--json"])
            .env_clear()
            .env("EPR_AGENT_RUNTIMES", &registry)
            .env("FIXTURE_SESSION", "child-export")
            .current_dir(tmp.path());
        if let Some(value) = switch {
            command.env("EPR_UNDER_TEST", value);
        }
        let output = command.output().unwrap();
        assert!(output.status.success());
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["session"], "child-export");
        assert_eq!(value["source"], "runtimeAncestor");
    }
}
