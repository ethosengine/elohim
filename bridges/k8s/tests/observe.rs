use k8s_bridge::{fold_observation, Observation};
use serde_json::{json, Value};
use std::process::Command;

const LEDGER: &str = include_str!("../../../genesis/data/rakia/compute-capacity.json");
const MIB: u64 = 1024 * 1024;
fn ledger() -> Value {
    serde_json::from_str(LEDGER).unwrap()
}
fn responses() -> [Value; 3] {
    let nodes = [
        ("shem", 24, 128751),
        ("ethosengine", 24, 64088),
        ("intel-nuc", 8, 15599),
        ("thinkc-p0h", 4, 7731),
        ("thinkc-p0t", 4, 7731),
        ("thinkc-p1s", 4, 7731),
        ("hp-micro10", 2, 15348),
    ];
    // The supplied whole-Mi figures total 246979. Canned sub-Mi remainders
    // exercise the byte-before-Mi aggregate, reproducing the stated 246980.
    [0,1,2].map(|metric| json!({"status":"success","data":{"resultType":"vector","result":nodes.iter().map(|(name,cpu,mem)| json!({"metric":{"node":name},"value":[1788634440, match metric {0 => cpu.to_string(), 1 => (mem * MIB + MIB / 4).to_string(), _ => "1".into()}]})).collect::<Vec<_>>()}}))
}
fn parse(r: &[Value; 3]) -> Observation {
    Observation::from_responses(
        &r[0],
        &r[1],
        &r[2],
        "2026-09-05T19:00:00Z".into(),
        "prometheus.example".into(),
    )
    .unwrap()
}
fn may() -> Value {
    let mut l = ledger();
    l["snapshotTimestamp"] = json!("2026-05-04T12:00:00Z");
    l["cluster"]["totalAllocatable"]["cpu_m"] = json!(46000);
    l["cluster"]["totalAllocatable"]["memory_Mi"] = json!(133726);
    l["cluster"]["totalHeadroom"]["cpu_m"] = json!(34820);
    l["cluster"]["totalHeadroom"]["memory_Mi"] = json!(110584);
    l["cluster"]["readyNodeCount"] = json!(6);
    l["cluster"]["notReadyNodes"] = json!(["shem"]);
    let remote = &mut l["cluster"]["nodeTypes"]["remote"];
    remote["nodes"][0]["ready"] = json!(false);
    remote["totals"]["cpu_m"] = json!(0);
    remote["totals"]["memory_Mi"] = json!(0);
    l
}
#[test]
fn canned_promotion_changes_only_observed_fields() {
    let mut l = may();
    let original = l.clone();
    let changes = fold_observation(&mut l, &parse(&responses())).unwrap();
    assert_eq!(l["cluster"]["totalAllocatable"]["cpu_m"], 70000);
    assert_eq!(l["cluster"]["totalAllocatable"]["memory_Mi"], 246980);
    assert_eq!(l["cluster"]["readyNodeCount"], 7);
    assert_eq!(
        l["cluster"]["nodeTypes"]["remote"]["nodes"][0]["ready"],
        true
    );
    assert_eq!(
        l["cluster"]["nodeTypes"]["remote"]["totals"]["cpu_m"],
        24000
    );
    assert_eq!(
        l["cluster"]["nodeTypes"]["remote"]["totals"]["memory_Mi"],
        128751
    );
    let mut expected = original.clone();
    expected["snapshotTimestamp"] = json!("2026-09-05T19:00:00Z");
    expected["snapshotMethod"] = json!("k8s-bridge observe from Prometheus host prometheus.example using kube_node_status_allocatable (CPU cores and memory bytes) and kube_node_status_condition (Ready) at 2026-09-05T19:00:00Z; commitments/actuals/ephemeral storage retained from 2026-05-04T12:00:00Z.");
    expected["cluster"]["actuals"]["shem"]["status"] = json!("Ready (observed 2026-09-05T19:00:00Z via Prometheus); utilization not re-sampled in this promotion");
    for (path, value) in [
        ("/cluster/totalAllocatable/cpu_m", 70000),
        ("/cluster/totalAllocatable/memory_Mi", 246980),
        ("/cluster/totalHeadroom/cpu_m", 58820),
        ("/cluster/totalHeadroom/memory_Mi", 223838),
        ("/cluster/readyNodeCount", 7),
    ] {
        *expected.pointer_mut(path).unwrap() = json!(value);
    }
    expected["cluster"]["notReadyNodes"] = json!([]);
    for (group, cpu, mem) in [
        ("remote", 24000, 128751),
        ("performance", 24000, 64088),
        ("operations", 8000, 15599),
        ("storage", 2000, 15348),
    ] {
        expected["cluster"]["nodeTypes"][group]["nodes"][0]["ready"] = json!(true);
        for key in ["allocatable", "totals"] {
            let bundle = if key == "totals" {
                &mut expected["cluster"]["nodeTypes"][group][key]
            } else {
                &mut expected["cluster"]["nodeTypes"][group]["nodes"][0][key]
            };
            bundle["cpu_m"] = json!(cpu);
            bundle["memory_Mi"] = json!(mem);
        }
    }
    assert_eq!(l, expected);
    let mut replay = original;
    for change in &changes {
        assert_eq!(replay.pointer(&change.field).unwrap(), &change.old);
        *replay.pointer_mut(&change.field).unwrap() = change.new.clone();
    }
    assert_eq!(replay, expected);
    assert!(changes.iter().any(|c| c.old == 46000 && c.new == 70000));
    assert!(changes.iter().any(|c| c.old == 133726 && c.new == 246980));
}
#[test]
fn missing_node_refuses_without_mutation() {
    for metric in 0..3 {
        let mut r = responses();
        r[metric]["data"]["result"]
            .as_array_mut()
            .unwrap()
            .remove(0);
        let mut l = ledger();
        let before = l.clone();
        assert!(fold_observation(&mut l, &parse(&r))
            .unwrap_err()
            .to_string()
            .contains("shem"));
        assert_eq!(l, before);
    }
}
#[test]
fn zero_and_nonfinite_allocatable_refuse() {
    for metric in 0..2 {
        for value in ["0", "NaN", "+Inf", "-1"] {
            let mut r = responses();
            r[metric]["data"]["result"][0]["value"][1] = json!(value);
            let error = Observation::from_responses(
                &r[0],
                &r[1],
                &r[2],
                "2026-09-05T19:00:00Z".into(),
                "test".into(),
            )
            .unwrap_err();
            assert!(error.to_string().contains("shem"));
        }
    }
    let mut obs = parse(&responses());
    obs.cpu_m.insert("shem".into(), 0);
    let mut l = ledger();
    let before = l.clone();
    assert!(fold_observation(&mut l, &obs)
        .unwrap_err()
        .to_string()
        .contains("shem"));
    assert_eq!(l, before);
}
#[test]
fn unknown_node_warns_without_adding_or_counting() {
    let mut r = responses();
    for response in &mut r {
        let mut extra = response["data"]["result"][0].clone();
        extra["metric"]["node"] = json!("new-node");
        response["data"]["result"]
            .as_array_mut()
            .unwrap()
            .push(extra);
    }
    let obs = parse(&r);
    let mut l = ledger();
    assert_eq!(
        obs.warnings(&l).unwrap(),
        vec!["warning: unknown node new-node; not added (operator promotion required)"]
    );
    fold_observation(&mut l, &obs).unwrap();
    assert_eq!(l["cluster"]["nodeCount"], 7);
    assert_eq!(l["cluster"]["totalAllocatable"]["cpu_m"], 70000);
    assert!(!serde_json::to_string(&l).unwrap().contains("new-node"));
}
#[test]
fn prometheus_error_status_refuses() {
    for metric in 0..3 {
        let mut r = responses();
        r[metric]["status"] = json!("error");
        assert!(Observation::from_responses(
            &r[0],
            &r[1],
            &r[2],
            "2026-09-05T19:00:00Z".into(),
            "test".into()
        )
        .unwrap_err()
        .to_string()
        .contains("status is not success"));
    }
}
#[test]
fn key_order_and_untouched_fields_round_trip_identically() {
    let mut l = ledger();
    let before = serde_json::to_string_pretty(&l).unwrap();
    let changes = fold_observation(&mut l, &parse(&responses())).unwrap();
    let serialized = serde_json::to_string_pretty(&l).unwrap();
    let mut roundtrip: Value = serde_json::from_str(&serialized).unwrap();
    for change in changes {
        *roundtrip.pointer_mut(&change.field).unwrap() = change.old;
    }
    assert_eq!(serde_json::to_string_pretty(&roundtrip).unwrap(), before);
    assert!(serialized.starts_with("{\n  \"version\": \"1.0\",\n  \"$schemaVersion\""));
}
#[test]
fn cli_missing_prometheus_url_refuses_65() {
    let output = Command::new(env!("CARGO_BIN_EXE_k8s-bridge"))
        .args(["observe", "--ledger", "does-not-exist.json"])
        .env_remove("PROMETHEUS_URL")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(65));
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("PROMETHEUS_URL"));
}
#[test]
fn not_ready_is_evidence_not_zero_capacity() {
    let mut r = responses();
    r[2]["data"]["result"][0]["value"][1] = json!("0");
    let mut l = ledger();
    fold_observation(&mut l, &parse(&r)).unwrap();
    assert_eq!(l["cluster"]["readyNodeCount"], 6);
    assert_eq!(l["cluster"]["notReadyNodes"], json!(["shem"]));
    assert_eq!(l["cluster"]["totalAllocatable"]["cpu_m"], 70000);
}
