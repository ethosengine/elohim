//! A thin handoff, not another admission engine. Native shift readiness owns session claims,
//! isolated worktrees, original authority, revocation, budgets, leases and exact-revision gates.
use super::{required, response::check_commitment, Result};
use cid::Cid;
use elohim_epr_rea::{
    feedback::{Feedback, Policy, Response},
    feedback_store::FeedbackStore,
};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path, process::Command};

pub(super) fn handoff(
    root: &Path,
    store: &FeedbackStore,
    policy: &Policy,
    options: &BTreeMap<String, String>,
) -> Result<Value> {
    if policy.response != Response::Repair || !policy.deliberate_evidence {
        return Err("waiting: contextual policy does not delegate repair".into());
    }
    let occurrence: Cid = required(options, "id")?.parse()?;
    let contract_path = root.join(required(options, "contract")?).canonicalize()?;
    if !contract_path.starts_with(root.canonicalize()?) {
        return Err("waiting: repair contract must belong to this workspace".into());
    }
    let bytes = std::fs::read(&contract_path)?;
    if bytes.len() as u64 > super::INPUT_LIMIT {
        return Err("repair contract exceeds input limit".into());
    }
    let contract: Value = serde_json::from_slice(&bytes)?;
    if contract["mode"] != "local-delivery" {
        return Err("waiting: feedback handoff requires explicit local-delivery mode".into());
    }
    let commitment: Cid = contract["commitment"]
        .as_str()
        .ok_or("waiting: contract commitment absent")?
        .parse()?;
    let existing = check_commitment(root, &commitment)?;
    let intent: Cid = contract["intent"]
        .as_str()
        .ok_or("waiting: contract intent absent")?
        .parse()?;
    if !existing.satisfies.contains(&intent) {
        return Err("waiting: contract does not name the commitment's intent".into());
    }
    let runtime = required(options, "runtime")?;
    if !["codex", "claude"].contains(&runtime) {
        return Err("unsupported shift runtime".into());
    }
    // One occurrence cannot acquire a second repair link. Repeated handoffs use the same
    // frozen native contract ID; the launcher itself refuses a duplicate live execution.
    store.append(
        Feedback::CommitmentLink {
            occurrence,
            commitment,
        },
        true,
    )?;
    let output = Command::new("just")
        .current_dir(root)
        .args(["dev", "shift", "start"])
        .arg(&contract_path)
        .args(["--runtime", runtime])
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "waiting: native shift admission exited {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(
        json!({"launcher":"just dev shift start","exitCode":output.status.code(),
        "handoffAccepted":output.status.success(),"recoveryObserved":false,
        "waiting":if output.status.success() {None} else {Some(String::from_utf8_lossy(&output.stderr).into_owned())}}),
    )
}
