//! Governable algorithm-content, following the existing bounded-recall artifact convention.
//! Method bytes and application policy are independently named, never inferred from a label.
use super::declarations::{identity, read};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Algorithm {
    id: String,
    version: u32,
    artifact_type: String,
    governance: Value,
    composition: Vec<String>,
    executor: String,
    application: Application,
}
#[derive(Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Application {
    mode: String,
    review: String,
    max_candidates: usize,
    packet_bytes: usize,
    deadline_ms: usize,
    inference: String,
    persistence: String,
    contributions: String,
}

pub fn load(root: &Path, path: &str) -> Result<Value, String> {
    let raw = read(root, path)?;
    let method: Algorithm =
        serde_json::from_str(&raw).map_err(|e| format!("invalid classifier algorithm: {e}"))?;
    let app = &method.application;
    let stages = [
        "normalize",
        "resolve",
        "detect",
        "verify-sources",
        "bound-packet",
        "agent-review",
        "validate-review",
    ];
    if method.id.is_empty()
        || method.version == 0
        || method.artifact_type != "algorithm-content"
        || method.executor != "native-seam-v1"
        || method.composition != stages
        || method.governance["owner"] != "governance-plane-single-evaluator"
        || app.mode != "advisory"
        || app.review != "active-agent"
        || app.inference != "none"
        || app.persistence != "none"
        || app.contributions != "explicit-only"
        || !(1..=3).contains(&app.max_candidates)
        || !(1024..=8192).contains(&app.packet_bytes)
        || app.deadline_ms != 2000
    {
        return Err("unsupported classifier method or application policy; native executor must implement declared stages and bounds".into());
    }
    // Name the actual compiled method, not whatever a possibly newer checkout happens to hold.
    let implementation = [
        ("adapters.rs", include_str!("adapters.rs")),
        ("classify.rs", include_str!("classify.rs")),
        ("declarations.rs", include_str!("declarations.rs")),
        ("method.rs", include_str!("method.rs")),
        ("mod.rs", include_str!("mod.rs")),
        ("review.rs", include_str!("review.rs")),
    ]
    .map(|(path, body)| json!({"module": path, "sourceId": identity(body.as_bytes())}));
    let application = serde_json::to_value(app).expect("application serializes");
    Ok(
        json!({"path": path, "algorithmId": identity(raw.as_bytes()), "executor": method.executor,
        "implementationId": identity(&serde_json::to_vec(&implementation).unwrap()),
        "applicationId": identity(&serde_json::to_vec(&application).unwrap()),
        "application": application}),
    )
}
