//! Local admission prints only requirements; Provides remains in memory.
use std::{fs, path::Path};

use ark_supervisor::admission::{admit_artifact, admit_recorded_requirements, OwnProduce};
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
pub fn admit(
    path: &Path,
    name: &str,
    requires_cid: Option<&str>,
    binary: Option<(&Path, &str)>,
    own_produce: Option<&str>,
) -> u8 {
    let result = (|| {
        if let Some((path, cid)) = binary {
            verified_bytes(path, Some(cid)).map_err(|e| {
                format!("output requires {cid} — {e}; rebuild or restore the recorded binary")
            })?;
        }
        let bytes = verified_bytes(path, requires_cid)
            .map_err(|e| format!("env:requires requires a verified local document — {e}; restore the recorded requirement"))?;
        let requires: Requires = serde_json::from_slice(&bytes).map_err(|_| {
            "env:requires requires a valid clause array — restore the recorded requirement"
                .to_string()
        })?;
        let provides = ark_supervisor::host::observe_host();
        let verdict = if let Some(produce) = own_produce {
            let (_, output) = binary.ok_or("own Produce requires a verified binary/output")?;
            let output = output.parse::<Cid>().map_err(|_| "invalid output CID")?;
            let produce = produce
                .parse::<Cid>()
                .map_err(|_| "invalid own Produce CID")?;
            let claim = OwnProduce::from_local_lookup(produce, output);
            let effect = admit_artifact(
                output,
                Some(&requires),
                &provides,
                Some(&claim),
                &Default::default(),
            );
            if effect.permits() && !effect.own_build_rule().is_empty() {
                let waived = effect
                    .own_build_rule()
                    .iter()
                    .map(|clause| {
                        let predicate: Predicate = serde_json::from_value(clause.predicate.clone())
                            .expect("admission preserves the requirement predicate");
                        format!("{} {}", clause.axis, value(&predicate))
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                return Ok(format!(
                    "admitted under the own-build rule (waived: {waived})"
                ));
            }
            effect.verdict().clone()
        } else {
            admit_recorded_requirements(Some(&requires), &provides)
        };
        if verdict.decision == Decision::Permit {
            Ok(format!("admitted ({} clauses)", requires.clauses().len()))
        } else {
            let failed = verdict
                .witness
                .checks
                .iter()
                .find(|c| c.outcome == CheckOutcome::Failed)
                .expect("refusal has a failed check");
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
    match result {
        Ok(detail) => {
            println!(
                "ok compatibility {}: {}",
                single_line(name),
                single_line(&detail)
            );
            0
        }
        Err(detail) => {
            println!(
                "REFUSED compatibility {}: {}",
                single_line(name),
                single_line(&detail)
            );
            1
        }
    }
}
