use ark_core::manifest::RuntimeManifest;
use k8s_bridge::{drift_verdict, quantity, render_envelope, verify, DriftVerdict};
use serde_json::{json, Value};
use std::{fs, path::PathBuf, process::Command};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn runtime() -> PathBuf {
    repo().join("genesis/orchestrator/manifests/runtime")
}
fn manifest(name: &str) -> RuntimeManifest {
    RuntimeManifest::from_json(
        &fs::read_to_string(runtime().join(format!("{name}.manifest.json"))).unwrap(),
    )
    .unwrap()
}
fn deployments() -> Value {
    serde_json::from_str(
        &fs::read_to_string(repo().join("genesis/orchestrator/data/deployments.json")).unwrap(),
    )
    .unwrap()
}
fn bash_split(args: &[&str]) -> String {
    let output = Command::new("bash")
        .arg(repo().join("scripts/ci/conductor-split-budget.sh"))
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}
// Explicit field order is the script's byte-level JSON contract.
fn split_json(r: &k8s_bridge::RenderedEnvelope) -> String {
    format!(
        "{{\"conductorMemoryRequest\":\"{}\",\"storageMemoryRequest\":\"{}\",\"conductorMemoryLimit\":\"{}\",\"storageMemoryLimit\":\"{}\",\"conductorCpuRequest\":\"{}\",\"storageCpuRequest\":\"{}\",\"conductorCpuLimit\":\"{}\",\"storageCpuLimit\":\"{}\"}}\n",
        r.conductor_memory_request, r.storage_memory_request, r.conductor_memory_limit, r.storage_memory_limit,
        r.conductor_cpu_request, r.storage_cpu_request, r.conductor_cpu_limit, r.storage_cpu_limit,
    )
}

#[test]
fn golden_split_matches_bash_for_every_archetype_and_adam() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../genesis/data/devices/archetype-resource-budgets.json"
    ))
    .unwrap();
    for (name, budget) in data["budgets"].as_object().unwrap() {
        let m = manifest(name.strip_prefix("device-").unwrap());
        let rendered = render_envelope(&m).unwrap();
        let args = ["memoryRequest", "memoryLimit", "cpuRequest", "cpuLimit"]
            .map(|f| budget[f].as_str().unwrap());
        assert_eq!(split_json(&rendered), bash_split(&args), "{name}");
    }
    assert_eq!(
        split_json(&render_envelope(&manifest("adam")).unwrap()),
        bash_split(&["2Gi", "8Gi", "1500m", "8000m"])
    );
}

#[test]
fn split_floors_odd_values_and_passes_absent_limits_through() {
    let mut m = manifest("family-node-base");
    let e = m.envelope.as_mut().unwrap();
    e.bound.memory_bytes = Some(8193 * 1024 * 1024 + 123);
    e.bound.cpu_millis = Some(4001);
    assert_eq!(
        split_json(&render_envelope(&m).unwrap()),
        bash_split(&["2Gi", "8193Mi", "1500m", "4001m"])
    );
    let e = m.envelope.as_mut().unwrap();
    e.bound.memory_bytes = None;
    e.bound.cpu_millis = None;
    assert_eq!(
        split_json(&render_envelope(&m).unwrap()),
        bash_split(&["2Gi", "", "1500m", ""])
    );
}

#[test]
fn pinned_fleet_is_fresh() {
    let verdicts = verify(
        &repo().join("genesis/orchestrator/data/deployments.json"),
        &runtime(),
    )
    .unwrap();
    assert_eq!(verdicts.len(), 7);
    assert!(verdicts.iter().all(|v| *v == DriftVerdict::Fresh));
}

fn temp_fleet() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("runtime");
    fs::create_dir(&dir).unwrap();
    for entry in fs::read_dir(runtime()).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "json") {
            fs::copy(&path, dir.join(path.file_name().unwrap())).unwrap();
        }
    }
    let deployments = temp.path().join("deployments.json");
    fs::write(
        &deployments,
        serde_json::to_string(&self::deployments()).unwrap(),
    )
    .unwrap();
    (temp, deployments, dir)
}

#[test]
fn perturbed_field_is_render_drift() {
    let (_temp, path, dir) = temp_fleet();
    let mut data = deployments();
    let human = &mut data["humans"][0];
    let name = human["name"].as_str().unwrap().to_string();
    human["edgenodeCpuLimit"] = json!("8001m");
    fs::write(&path, serde_json::to_string(&data).unwrap()).unwrap();
    assert_eq!(
        verify(&path, &dir).unwrap()[0],
        DriftVerdict::RenderDrift {
            human: name,
            field: "edgenodeCpuLimit".into(),
            pinned: "8001m".into(),
            rendered: "8000m".into()
        }
    );
}

#[test]
fn changed_manifest_bytes_are_pin_mismatch() {
    let (_temp, path, dir) = temp_fleet();
    let file = dir.join("adam.manifest.json");
    let bytes = fs::read_to_string(&file).unwrap();
    // Change semantic bytes, not whitespace: DAG-CBOR identity ignores JSON formatting.
    fs::write(&file, bytes.replace("8000", "8001")).unwrap();
    assert!(
        matches!(&verify(&path, &dir).unwrap()[0], DriftVerdict::PinMismatch { human, expected, actual } if human == "adam" && expected != actual)
    );
}

#[test]
fn missing_pin_is_unpinned_and_suspended_humans_are_skipped() {
    let (_temp, path, dir) = temp_fleet();
    let mut data = deployments();
    data["humans"][0]
        .as_object_mut()
        .unwrap()
        .remove("runtimeManifest");
    fs::write(&path, serde_json::to_string(&data).unwrap()).unwrap();
    assert_eq!(
        verify(&path, &dir).unwrap()[0],
        DriftVerdict::Unpinned {
            human: "adam".into()
        }
    );
    data["humans"][0]["suspended"] = json!(true);
    fs::write(&path, serde_json::to_string(&data).unwrap()).unwrap();
    assert_eq!(verify(&path, &dir).unwrap().len(), 6);
}

#[test]
fn absent_envelope_renders_empty_object_and_keeps_live_cid() {
    let mut m = manifest("family-node-base");
    m.envelope = None;
    m.archetype = None;
    assert_eq!(
        serde_json::to_string(&render_envelope(&m).unwrap()).unwrap(),
        "{}"
    );
    assert_eq!(
        m.cid().unwrap(),
        "bafyreihagg75knog3e2fkiygpghcgqge35ovzka3vxken2zjleqnerdcaa"
    );
}

#[test]
fn unknown_schema_or_archetype_and_invalid_requests_refuse() {
    let mut m = manifest("family-node-base");
    m.schema = 2;
    assert!(render_envelope(&m).is_err());
    m.schema = 1;
    m.archetype = Some("missing".into());
    assert!(render_envelope(&m).is_err());
    m.archetype = None;
    assert!(render_envelope(&m).is_err());
    let mut m = manifest("jessica");
    m.envelope
        .as_mut()
        .unwrap()
        .requests
        .as_mut()
        .unwrap()
        .memory_bytes = Some(1);
    assert!(render_envelope(&m).is_err());
    m.envelope
        .as_mut()
        .unwrap()
        .requests
        .as_mut()
        .unwrap()
        .memory_bytes = Some(u64::MAX);
    assert!(render_envelope(&m).is_err());
}

#[test]
fn superseding_manifest_pins_request_override_and_archetype_lineage() {
    let m = manifest("jessica");
    assert_eq!(
        m.supersedes,
        Some(manifest("recycled-laptop").cid().unwrap())
    );
    assert_eq!(
        render_envelope(&m).unwrap().edgenode_memory_request,
        "1024Mi"
    );
    let mut changed = m.clone();
    changed.envelope.as_mut().unwrap().requests = None;
    assert_ne!(changed.cid().unwrap(), m.cid().unwrap());
    assert_eq!(
        render_envelope(&changed).unwrap().edgenode_memory_request,
        "768Mi"
    );
}

#[test]
fn quantity_equivalence_is_exact_and_malformed_quantities_refuse() {
    assert_eq!(
        quantity("8Gi", true).unwrap(),
        quantity("8192Mi", true).unwrap()
    );
    assert_eq!(
        quantity("1.5", false).unwrap(),
        quantity("1500m", false).unwrap()
    );
    for value in [
        "-1Mi",
        "NaNMi",
        "1e3Mi",
        "1.2.3Mi",
        "Mi",
        "8G",
        "18446744073709551615Ti",
    ] {
        assert!(quantity(value, true).is_err(), "{value}");
    }
    let data = deployments();
    let mut h = data["humans"][0].clone();
    h["deviceArchetype"] = json!("device-home-nuc");
    assert!(
        matches!(drift_verdict(&h, &manifest("adam")).unwrap(), DriftVerdict::RenderDrift { field, .. } if field == "deviceArchetype")
    );
}

#[test]
fn pin_path_cannot_escape_manifest_directory() {
    let (_temp, path, dir) = temp_fleet();
    let mut data = deployments();
    data["humans"][0]["runtimeManifest"]["path"] = json!("../adam.manifest.json");
    fs::write(&path, serde_json::to_string(&data).unwrap()).unwrap();
    assert!(verify(&path, &dir).is_err());
}

#[test]
fn cli_observe_is_explicitly_unimplemented_and_render_reports_cid() {
    let bin = env!("CARGO_BIN_EXE_k8s-bridge");
    let output = Command::new(bin).arg("observe").output().unwrap();
    assert_eq!(output.status.code(), Some(64));
    assert_eq!(output.stdout, b"not yet: Station 3b\n");
    let output = Command::new(bin)
        .args([
            "render",
            runtime().join("adam.manifest.json").to_str().unwrap(),
            "--human",
            "adam",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains(&manifest("adam").cid().unwrap()));
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap()["edgenodeCpuLimit"],
        "8000m"
    );
}

#[test]
fn cli_missing_pin_refuses() {
    let (_temp, path, dir) = temp_fleet();
    let mut data = deployments();
    data["humans"][0]
        .as_object_mut()
        .unwrap()
        .remove("runtimeManifest");
    fs::write(&path, serde_json::to_string(&data).unwrap()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_k8s-bridge"))
        .args([
            "verify",
            "--deployments",
            path.to_str().unwrap(),
            "--manifests-dir",
            dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(65));
    assert!(String::from_utf8(output.stdout)
        .unwrap()
        .contains("Unpinned"));
}
