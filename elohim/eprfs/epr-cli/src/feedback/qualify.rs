//! Exercise the actual recorder/producer boundary in a disposable private store.
//! A witness is a rebuildable check, never a release, participant claim or admission.
use super::{load_policy, policy_root, Result};
use crate::flow::memory::recall::{bounded_command, ProcessOutcome};
use elohim_epr_rea::{feedback::Policy, FlowRecord, FlowStore, SidecarFlowStore};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::Command,
};

const CHECK: &str = "private-feedback-loop";
const PRIVATE_ARGUMENT: &str = "--feedback-probe-private-argument";
const OUTPUT_LIMIT: usize = 1024 * 1024;
const COMMAND_SECONDS: f64 = 10.0;

struct Failure {
    outcome: &'static str,
    summary: &'static str,
}
type Checked<T> = std::result::Result<T, Failure>;
fn failed(summary: &'static str) -> Failure {
    Failure {
        outcome: "failed",
        summary,
    }
}
fn absent(summary: &'static str) -> Failure {
    Failure {
        outcome: "skipped",
        summary,
    }
}

pub(super) fn run(options: &BTreeMap<String, String>, machine: bool) -> Result<()> {
    let recorder_only = options.contains_key("recorder-only");
    let mode = if recorder_only {
        "recorder-only"
    } else {
        "pair"
    };
    let checked = qualify(options, recorder_only);
    let (outcome, summary, observed) = match checked {
        Ok(evidence) => (
            "passed",
            if recorder_only {
                "Recorder qualified in isolation; the Brit/EPR pair remains unqualified."
            } else {
                "Selected pair retained private outcomes and proposed a native intent without admitting repair."
            },
            evidence,
        ),
        Err(error) => (error.outcome, error.summary, json!({})),
    };
    let mut observed = observed;
    observed["mode"] = json!(mode);
    observed["fixtureOnly"] = json!(true);
    observed["pairQualified"] = json!(outcome == "passed" && !recorder_only);
    let witness = json!({"checkId":CHECK,"outcome":outcome,"summary":summary,"observed":observed});
    if machine {
        println!("{}", serde_json::to_string(&witness)?);
    } else {
        println!("{CHECK}: {outcome} — {summary}");
    }
    if outcome != "passed" {
        return Err("feedback qualification did not pass".into());
    }
    Ok(())
}

fn qualify(options: &BTreeMap<String, String>, recorder_only: bool) -> Checked<Value> {
    if recorder_only == options.contains_key("brit") {
        return Err(failed(
            "Choose exactly one of --brit PATH or --recorder-only.",
        ));
    }
    if options
        .keys()
        .any(|key| !["root", "brit", "recorder-only"].contains(&key.as_str()))
    {
        return Err(failed("Unsupported qualification option."));
    }
    let root = policy_root(options.get("root").map(String::as_str))
        .map_err(|_| absent("Contextual feedback policy is unavailable; capture is unmeasured."))?;
    if !root.join(".epr-meta/feedback-policy.json").is_file() {
        return Err(absent(
            "Contextual feedback policy is unavailable; capture is unmeasured.",
        ));
    }
    let policy = load_policy(&root)
        .map_err(|_| failed("Contextual feedback policy or capacity binding is invalid."))?;
    if !policy.automatic_capture {
        return Err(failed(
            "Contextual policy does not authorize automatic feedback capture.",
        ));
    }
    if !policy.producers.iter().any(|producer| producer == "brit") {
        return Err(failed(
            "Contextual policy does not enable Brit's feedback producer.",
        ));
    }
    if !policy.deliberate_evidence || policy.response == elohim_epr_rea::feedback::Response::Report
    {
        return Err(failed(
            "Contextual policy does not authorize feedback proposals.",
        ));
    }
    if policy.capacity.is_none() {
        return Err(absent(
            "Version-pinned feedback capacity policy is required for qualification.",
        ));
    }
    let recorder =
        std::env::current_exe().map_err(|_| absent("Recorder executable is unavailable."))?;
    let brit = options
        .get("brit")
        .map(|path| executable(Path::new(path)))
        .transpose()?;
    let fixture = tempfile::Builder::new()
        .prefix("brit-feedback-qualify-")
        .tempdir()
        .map_err(|_| absent("Private qualification fixture could not be created."))?;
    prepare(fixture.path(), &policy, &recorder)?;
    let probe = Probe {
        root: fixture.path(),
        recorder: &recorder,
    };
    probe.feedback(&["migrate", "--legacy-writers-quiesced"])?;
    for (id, outcome, args, expected) in [
        ("success", "passed", vec!["--version"], 0),
        ("refusal", "refused", vec!["log", PRIVATE_ARGUMENT], 2),
    ] {
        if let Some(brit) = &brit {
            probe.brit(brit, id, &args, expected, fixture.path())?;
        } else {
            probe.feedback(&[
                "capture",
                "--producer",
                "brit",
                "--operation",
                "log",
                "--outcome",
                outcome,
                "--invocation",
                id,
            ])?;
        }
    }
    let list = probe.feedback(&["list"])?;
    let (occurrences, refusal) = check_outcomes(&list)?;
    if probe.feedback(&["list"])? != list {
        return Err(failed("Reopening changed retained experiences."));
    }
    check_private_bytes(fixture.path())?;
    let proposal = probe.feedback(&[
        "propose",
        "--id",
        &refusal,
        "--as",
        "agent:probe@fixture",
        "--objective",
        "Restore the refused fixture operation.",
    ])?;
    let intent = proposal["intent"]
        .as_str()
        .ok_or_else(|| failed("Proposal did not identify a native intent."))?;
    if proposal["admitted"] != false {
        return Err(failed("Proposal improperly admitted repair."));
    }
    check_proposal(fixture.path(), intent, &refusal)?;
    let triage = probe.feedback(&["triage", "--response", "repair"])?;
    check_triage(&triage, &occurrences)?;
    if let Some(brit) = &brit {
        let before = probe.feedback(&["list"])?;
        for (id, args, expected) in [
            ("recorder-success", vec!["--version"], 0),
            ("recorder-refusal", vec!["log", PRIVATE_ARGUMENT], 2),
        ] {
            let output =
                probe.brit(brit, id, &args, expected, &fixture.path().join("no-policy"))?;
            if !String::from_utf8_lossy(&output.stderr).contains("feedback capture gap") {
                return Err(failed("Recorder failure was not visible."));
            }
        }
        if before != probe.feedback(&["list"])? {
            return Err(failed("Recorder failure altered retained evidence."));
        }
    }
    Ok(
        json!({"occurrences":occurrences,"proposal":intent,"admitted":false,
        "recorderFailurePreservesExit":brit.is_some(),"recorder":recorder,"brit":brit,
        "policy":policy.id,"capacity":policy.capacity}),
    )
}

fn executable(path: &Path) -> Checked<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let path = path
        .canonicalize()
        .map_err(|_| absent("Brit executable is unavailable; capture is unmeasured."))?;
    let metadata = path
        .metadata()
        .map_err(|_| absent("Brit executable is unavailable; capture is unmeasured."))?;
    if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
        return Err(absent(
            "Brit executable is unavailable; capture is unmeasured.",
        ));
    }
    if path.file_name().is_some_and(|name| name == "bin-launch") {
        return Err(absent("Resolve Brit to its immutable executable before qualification; a launcher can change session pins."));
    }
    Ok(path)
}

fn prepare(root: &Path, policy: &Policy, recorder: &Path) -> Checked<()> {
    let operation = || -> Result<()> {
        std::fs::create_dir(root.join(".epr-meta"))?;
        std::fs::create_dir(root.join("bin"))?;
        // The resolved snapshot carries the policy CID; no registry or live store is copied.
        std::fs::write(
            root.join(".epr-meta/feedback-policy.json"),
            serde_json::to_vec(policy)?,
        )?;
        std::os::unix::fs::symlink(recorder, root.join("bin/epr"))?;
        Ok(())
    };
    operation().map_err(|_| absent("Private qualification fixture could not be prepared."))
}

struct Probe<'a> {
    root: &'a Path,
    recorder: &'a Path,
}
impl Probe<'_> {
    fn command(&self, binary: &Path, args: &[&str]) -> Command {
        let mut command = Command::new(binary);
        command.args(args).current_dir(self.root);
        for key in [
            "EPR_BIN",
            "BRIT_BIN",
            "EPR_PIN_CID",
            "BRIT_PIN_CID",
            "EPR_REPO_ROOT",
        ]
        .into_iter()
        .chain(crate::process::GIT_LOCATION_ENV)
        {
            command.env_remove(key);
        }
        // Only the fixed fixture directory can supply the recorder; no inherited candidate wins.
        let paths = std::iter::once(self.root.join("bin"))
            .chain(std::env::split_paths(
                &std::env::var_os("PATH").unwrap_or_default(),
            ))
            .collect::<Vec<_>>();
        if let Ok(path) = std::env::join_paths(paths) {
            command.env("PATH", path);
        }
        command
            .env("EPR_SESSION_LAZY", "1")
            .env("EPR_FEEDBACK_ROOT", self.root);
        command
    }
    fn invoke(&self, command: Command) -> Checked<ProcessOutcome> {
        let output =
            bounded_command(command, OUTPUT_LIMIT, COMMAND_SECONDS, None).map_err(|_| {
                absent("Selected executable could not be started; coverage is unmeasured.")
            })?;
        if output.error.is_some() {
            return Err(absent(
                "Selected executable exceeded the qualification boundary; coverage is unmeasured.",
            ));
        }
        Ok(output)
    }
    fn feedback(&self, args: &[&str]) -> Checked<Value> {
        let mut command = self.command(self.recorder, &["feedback"]);
        command
            .args(args)
            .arg("--root")
            .arg(self.root)
            .arg("--json");
        let output = self.invoke(command)?;
        if output.status != Some(0) {
            return Err(failed(
                "Recorder refused the required native feedback interface.",
            ));
        }
        serde_json::from_slice(&output.stdout)
            .map_err(|_| failed("Recorder returned an invalid native response."))
    }
    fn brit(
        &self,
        brit: &Path,
        id: &str,
        args: &[&str],
        expected: i32,
        capture_root: &Path,
    ) -> Checked<ProcessOutcome> {
        let mut command = self.command(brit, args);
        command
            .env("EPR_FEEDBACK_INVOCATION", id)
            .env("EPR_FEEDBACK_ROOT", capture_root);
        let output = self.invoke(command)?;
        if output.status != Some(expected) {
            return Err(failed("Brit did not preserve the expected command exit."));
        }
        Ok(output)
    }
}

fn check_outcomes(list: &Value) -> Checked<(Vec<String>, String)> {
    let rows = list["experiences"]
        .as_array()
        .filter(|rows| rows.len() == 2)
        .ok_or_else(|| failed("The pair did not retain both command outcomes."))?;
    let mut ids = Vec::new();
    for (run, outcome) in [("success", "passed"), ("refusal", "refused")] {
        let row = rows
            .iter()
            .find(|row| row["occurrence"]["source"]["run"] == run)
            .ok_or_else(|| failed("Retained invocation correlation does not match."))?;
        if row["occurrence"]["outcome"] != outcome
            || row["occurrence"]["count"] != 1
            || row["occurrence"]["source"]["producer"] != "brit"
        {
            return Err(failed("Retained command outcome does not match."));
        }
        if row["experiencedRecovery"] != false || row["commitments"] != json!([]) {
            return Err(failed(
                "Probe capture created a commitment or claimed recovery.",
            ));
        }
        ids.push(
            row["cid"]
                .as_str()
                .ok_or_else(|| failed("Retained occurrence has no address."))?
                .to_owned(),
        );
    }
    Ok((ids.clone(), ids[1].clone()))
}

fn check_private_bytes(root: &Path) -> Checked<()> {
    let files = std::fs::read_dir(root.join(".eprfs/status/feedback"))
        .map_err(|_| failed("Retained evidence is unavailable."))?;
    for entry in files {
        let path = entry
            .map_err(|_| failed("Retained evidence is unreadable."))?
            .path();
        if path
            .extension()
            .is_some_and(|extension| extension == "jsonl")
        {
            let bytes = std::fs::read_to_string(path)
                .map_err(|_| failed("Retained evidence is unreadable."))?;
            if bytes.contains(PRIVATE_ARGUMENT) {
                return Err(failed(
                    "Automatic capture retained the private command argument.",
                ));
            }
        }
    }
    Ok(())
}

fn check_proposal(root: &Path, intent: &str, occurrence: &str) -> Checked<()> {
    let records = SidecarFlowStore::open(root)
        .and_then(|store| store.records())
        .map_err(|_| failed("Native proposal evidence is unreadable."))?;
    let valid = match records.as_slice() {
        [(id, FlowRecord::Intent(record))] => {
            id.to_string() == intent && record.in_scope_of.to_string() == occurrence
        }
        _ => false,
    };
    if !valid {
        return Err(failed(
            "Native proposal did not produce exactly one intent scoped to the refused occurrence.",
        ));
    }
    Ok(())
}

fn check_triage(triage: &Value, occurrences: &[String]) -> Checked<()> {
    let rows = triage["rows"]
        .as_array()
        .filter(|rows| rows.len() == occurrences.len())
        .ok_or_else(|| failed("Triage did not retain the measured occurrences."))?;
    if !occurrences
        .iter()
        .all(|id| rows.iter().any(|row| row["experience"]["cid"] == *id))
        || !rows.iter().all(|row| {
            row["admitted"] == false
                && row["waiting"].as_array().is_some_and(|reasons| {
                    reasons.iter().any(|reason| {
                        reason
                            .as_str()
                            .is_some_and(|reason| reason.contains("commitment"))
                    })
                })
        })
    {
        return Err(failed("Feedback proposal improperly admitted repair."));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use elohim_epr_rea::{atom_cid, AgentRef, Intent, ReaVerb, ResourceSpec};

    #[test]
    fn proposal_scope_uses_the_native_field_and_rejects_extra_acts() {
        let root = tempfile::tempdir().expect("fixture");
        let occurrence = atom_cid(&"occurrence").expect("address");
        let other = atom_cid(&"other").expect("address");
        let mut store = SidecarFlowStore::open(root.path()).expect("store");
        let mut intent = Intent {
            action: ReaVerb::Produce,
            raised_by: AgentRef("agent:probe@fixture".into()),
            in_scope_of: occurrence,
            resource_spec: ResourceSpec {
                classified_as: vec![other.to_string()],
                quantity: None,
            },
        };
        let cid = store
            .append(FlowRecord::Intent(intent.clone()))
            .expect("intent");
        assert!(check_proposal(root.path(), &cid.to_string(), &occurrence.to_string()).is_ok());
        assert!(check_proposal(root.path(), &cid.to_string(), &other.to_string()).is_err());
        intent.in_scope_of = other;
        store.append(FlowRecord::Intent(intent)).expect("extra act");
        assert!(check_proposal(root.path(), &cid.to_string(), &occurrence.to_string()).is_err());
    }
}
