//! Bind feedback capacity to the existing pinned Precedent registry. No fallback defaults.
use super::{Policy, Result};
use elohim_epr_rea::feedback_capacity::Capacity;
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    policy: String,
    params: serde_json::Value,
}
pub(super) fn resolve(root: &Path, policy: &mut Policy) -> Result<()> {
    let path = root.join(".epr-meta/feedback-capacity.json");
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    let binding: Binding = serde_json::from_slice(&bytes)?;
    let snapshot = eprfs_meta::resolved_policy_snapshot(root, &binding.policy)
        .map_err(|e| format!("feedback capacity policy unavailable: {e}"))?;
    policy.capacity = Some(Capacity::resolve(binding.policy, snapshot, binding.params)?);
    // Legacy byte fields are historical decoding only; one contract owns all new decisions.
    policy.max_bytes = 0;
    policy.max_record_bytes = 0;
    Ok(())
}

pub(super) fn print(value: &serde_json::Value) {
    println!(
        "Private feedback capacity · {}",
        value["policy"]["policy"]
            .as_str()
            .unwrap_or("policy unknown")
    );
    if let Some(cid) = value["policy"]["policyCid"].as_str() {
        println!("Resolved policy: {cid}");
    }
    for (label, key) in [
        ("Retained evidence", "retainedBytes"),
        ("All managed file contents", "totalManagedFileBytes"),
        ("Eligible for reclamation", "reclaimableBytes"),
        ("Protected or waiting", "protectedBytes"),
        ("Selected for reclamation", "selectedBytes"),
    ] {
        match value[key].as_u64() {
            Some(n) => println!("{label}: {n} bytes"),
            None => println!("{label}: unknown"),
        }
    }
    println!(
        "Rejected captures: {}",
        value["rejectedCapture"]
            .as_u64()
            .map(|n| n.to_string())
            .unwrap_or_else(|| "unknown".into())
    );
    if let Some(waiting) = value["waiting"].as_array() {
        for reason in waiting {
            if let Some(reason) = reason.as_str() {
                println!("Waiting: {reason}");
            }
        }
    }
    if let Some(history) = value["history"].as_str() {
        println!("{history}");
    }
}
