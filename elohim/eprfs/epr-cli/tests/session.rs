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
