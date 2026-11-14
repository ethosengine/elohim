//! Local admission prints only requirements; Provides remains in memory.
use std::{fs, path::Path};

use ark_supervisor::admission::{
    admit_artifact, admit_recorded_requirements, resolve_recorded, ResolveGap,
};
use cid::Cid;
use elohim_epr::verdict::{CheckOutcome, Decision};
use eprfs_core::BlobCid;
use host_contract::{Predicate, Requires};

fn value(predicate: &Predicate) -> String {
    match predicate {
        Predicate::Equal(value) => value.clone(),
        Predicate::Subset(values) | Predicate::Present(values) => {
            values.iter().cloned().collect::<Vec<_>>().join(", ")
        }
        Predicate::AtLeast(version) => format!(
            "{} >= {}",
            version.line,
            version
                .components
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(".")
        ),
    }
}

fn verified_bytes(path: &Path, cid: Option<&str>) -> Result<Vec<u8>, String> {
    let bytes = fs::read(path).map_err(|_| "recorded bytes are unavailable".to_string())?;
    if let Some(expected) = cid {
        let expected = BlobCid::parse(expected).map_err(|_| "invalid recorded CID")?;
        if BlobCid::compute_bytes(&bytes) != expected {
            return Err("recorded CID does not match bytes".into());
        }
    }
    Ok(bytes)
}

/// Each invocation prints exactly one line, including malformed/unavailable input.
pub fn admit(path: Option<&Path>, name: &str, binary: Option<(&Path, &str)>) -> u8 {
    let build = match name {
        "elohim-storage" => "storage",
        "relay-addr-beacon" => "beacon",
        other => other,
    };
    let result = (|| {
        let provides = ark_supervisor::host::observe_host();
        let verdict = if let Some((path, cid)) = binary {
            verified_bytes(path, Some(cid)).map_err(|e| {
                format!("output requires {cid} — {e}; rebuild or restore the recorded binary")
            })?;
            let output = cid.parse::<Cid>().map_err(|_| "invalid output CID")?;
            let recorded = match resolve_recorded(output) {
                Ok(recorded) => recorded,
                Err(ResolveGap::UnknownOutput(cid)) => {
                    return Err(format!(
                        "epr does not know output {cid} — rebuild with just mesh build {build}"
                    ));
                }
                Err(ResolveGap::Conflict) => {
                    return Err(format!(
                        "conflicting recorded requirements (MergeError) — rebuild with just mesh build {build}"
                    ));
                }
                Err(ResolveGap::Malformed(reason)) => {
                    return Err(format!("malformed requirement record: {reason}"))
                }
                Err(gap) => return Ok(format!("unmeasured ({gap})")),
            };
            let record = admit_artifact(&recorded, &provides, &Default::default());
            if record.permits() && !record.own_build_rule().is_empty() {
                let waived = record
                    .own_build_rule()
                    .iter()
                    .map(|clause| {
                        let predicate: Predicate = serde_json::from_value(clause.predicate.clone())
                            .map_err(|_| "invalid predicate in admission waiver")?;
                        Ok::<_, String>(format!("{} {}", clause.axis, value(&predicate)))
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .join("; ");
                return Ok(format!(
                    "admitted under the own-build rule (waived: {waived})"
                ));
            }
            record.verdict().clone()
        } else {
            let path = path.ok_or("env:requires requires a requirement file")?;
            let bytes = verified_bytes(path, None)
                .map_err(|e| format!("env:requires requires a local document — {e}"))?;
            let requires: Requires = serde_json::from_slice(&bytes).map_err(|_| {
                "env:requires requires a valid clause array — restore the requirement"
            })?;
            admit_recorded_requirements(Some(&requires), &provides)
        };
        if verdict.decision == Decision::Permit {
            Ok("admitted".into())
        } else {
            let failed = verdict
                .witness
                .checks
                .iter()
                .find(|c| c.outcome == CheckOutcome::Failed)
                .ok_or("admission declined without a failed check")?;
            let axis = failed
                .check_id
                .strip_prefix("host-compatibility:")
                .unwrap_or(&failed.check_id);
            let required = failed
                .observed
                .as_ref()
                .and_then(|v| v.get("required"))
                .and_then(|v| v.get("predicate"))
                .and_then(|v| serde_json::from_value::<Predicate>(v.clone()).ok())
                .map_or_else(|| "a recorded clause".into(), |p| value(&p));
            Err(format!("{axis} requires {required} — build for this device or use a compatible binary; the own-build rule can apply once a Produce recorded on this device is confirmed ({})", failed.summary))
        }
    })();
    // Labels and requirement values are caller input, so escape line/control separators.
    let single_line = |text: &str| {
        text.chars()
            .flat_map(char::escape_debug)
            .collect::<String>()
    };
    let diagnostic = if path.is_some() {
        " (diagnostic: requirement from file, not from the record)"
    } else {
        ""
    };
    match result {
        Ok(detail) => {
            println!(
                "ok compatibility {}: {}{}",
                single_line(name),
                single_line(&detail),
                diagnostic
            );
            0
        }
        Err(detail) => {
            println!(
                "REFUSED compatibility {}: {}{}",
                single_line(name),
                single_line(&detail),
                diagnostic
            );
            1
        }
    }
}
