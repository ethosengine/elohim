//! Brit-facing Git adapter: existing acceptance grants one pinned, local fast-forward.
//! No new authority store: the flow append lock fences cooperative acceptance writers.
use std::{
    io::Write,
    path::Path,
    process::{ExitCode, Stdio},
};

use cid::Cid;
use elohim_epr_rea::{FlowRecord, FlowStore, SidecarFlowStore};
use serde::{Deserialize, Serialize};

use crate::{
    doctor,
    flow::{self, acceptance, reconciliation},
    git, process, ready, Error, Result,
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Integration {
    source_ref: String,
    source_commit: String,
    target_ref: String,
    target_commit: String,
    gate: Vec<String>,
    gate_timeout_seconds: u64,
}

#[derive(Deserialize)]
struct Report {
    revision: String,
    integration: Integration,
}

fn refused(message: impl Into<String>) -> Error {
    Error::InvalidArguments(format!("integration refused: {}", message.into()))
}

fn flow_error(error: impl std::fmt::Display) -> Error {
    refused(error.to_string())
}

fn accepted(root: &Path, selected: &Cid, records: &[(Cid, FlowRecord)]) -> Result<Integration> {
    let event = records
        .iter()
        .find_map(|(id, record)| match record {
            FlowRecord::Event(event) if id == selected => Some(event),
            _ => None,
        })
        .ok_or_else(|| refused("acceptance CID is not an existing event"))?;
    acceptance::validate_record(root, records, event).map_err(flow_error)?;
    if acceptance::slot(event, "purpose:") != Some("acceptance")
        || acceptance::slot(event, "verdict:") != Some("approved")
    {
        return Err(refused("selected record is not approved acceptance"));
    }
    let selected_text = selected.to_string();
    if records.iter().any(|(_, record)| {
        matches!(record, FlowRecord::Event(later)
        if acceptance::slot(later, "supersedes:") == Some(selected_text.as_str())
            && acceptance::validate_structure(root, records, later).is_ok())
    }) {
        return Err(refused("selected acceptance is superseded"));
    }
    let commitment = records
        .iter()
        .find_map(|(id, record)| match record {
            FlowRecord::Commitment(commitment) if *id == event.resource => Some(commitment),
            _ => None,
        })
        .ok_or_else(|| refused("acceptance commitment missing"))?;
    let view = reconciliation::reconcile(root, &commitment.in_scope_of, None, records);
    let commitment_id = event.resource.to_string();
    if !view.assertions.iter().any(|assertion| {
        assertion.commitments.iter().any(|evidence| {
            evidence.commitment == commitment_id
                && evidence.acceptance == "accepted"
                && evidence.issues.is_empty()
                && evidence.acceptance_records.contains(&selected_text)
        })
    }) {
        return Err(refused(
            "native reconciliation does not currently accept this commitment",
        ));
    }
    let path = acceptance::slot(event, "report-path:")
        .ok_or_else(|| refused("acceptance report missing"))?;
    let path = flow::confine_under(root, &root.join(path)).map_err(flow_error)?;
    let bytes = std::fs::read_to_string(&path).map_err(|source| Error::Read { path, source })?;
    // Validate the exact bytes we deserialize too, rather than trusting a previous file read.
    let expected =
        acceptance::slot(event, "report-cid:").ok_or_else(|| refused("report pin missing"))?;
    if flow::body_cid(&bytes).to_string() != expected {
        return Err(refused("acceptance report changed while reading"));
    }
    let report: Report = serde_json::from_str(&bytes)?;
    if report.revision != report.integration.source_commit {
        return Err(refused(
            "report revision differs from integrated source commit",
        ));
    }
    Ok(report.integration)
}

fn validate_git(root: &Path, plan: &Integration) -> Result<()> {
    let hook = process::require(
        "git",
        &["rev-parse", "--git-path", "hooks/reference-transaction"],
        root,
    )?
    .stdout;
    let hook = root.join(hook);
    match std::fs::metadata(&hook) {
        Ok(metadata) => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if metadata.is_file() && metadata.permissions().mode() & 0o111 != 0 {
                    return Err(refused("executable reference-transaction hook is unsupported while the flow append lock is held; use an explicitly reviewed Git integration workflow"));
                }
            }
        }
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
            ) => {}
        Err(error) => {
            return Err(Error::Read {
                path: hook,
                source: error,
            })
        }
    }
    if plan.source_ref == plan.target_ref
        || plan.gate.is_empty()
        || plan.gate[0].trim().is_empty()
        || !(1..=3600).contains(&plan.gate_timeout_seconds)
        || plan.gate.iter().any(|arg| arg.contains('\0'))
    {
        return Err(refused(
            "distinct refs and a nonempty executable argv gate are required",
        ));
    }
    for (reference, oid) in [
        (&plan.source_ref, &plan.source_commit),
        (&plan.target_ref, &plan.target_commit),
    ] {
        if !reference.starts_with("refs/heads/")
            || ![40, 64].contains(&oid.len())
            || !oid
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(refused(
                "full branch refs and lowercase full commit OIDs are required",
            ));
        }
        process::require("git", &["check-ref-format", reference], root)?;
        if process::run("git", &["symbolic-ref", "-q", reference], root)?.success {
            return Err(refused("source and target must be direct refs"));
        }
        if process::require("git", &["rev-parse", "--verify", reference], root)?.stdout != *oid
            || process::require("git", &["cat-file", "-t", oid], root)?.stdout != "commit"
        {
            return Err(refused(format!(
                "{reference} moved or does not name the pinned commit"
            )));
        }
    }
    if process::require("git", &["symbolic-ref", "-q", "HEAD"], root)?.stdout != plan.source_ref
        || process::require("git", &["rev-parse", "HEAD"], root)?.stdout != plan.source_commit
        || !git::working_tree_changes(root)?.is_empty()
    {
        return Err(refused(
            "current checkout must be clean and HEAD must name the pinned source ref",
        ));
    }
    let worktrees = process::require("git", &["worktree", "list", "--porcelain", "-z"], root)?;
    if worktrees
        .stdout
        .split('\0')
        .any(|field| field == format!("branch {}", plan.target_ref))
    {
        return Err(refused("target is checked out in a worktree"));
    }
    if !process::run(
        "git",
        &[
            "merge-base",
            "--is-ancestor",
            &plan.target_commit,
            &plan.source_commit,
        ],
        root,
    )?
    .success
    {
        return Err(refused(
            "target-to-source update is not a proven fast-forward",
        ));
    }
    Ok(())
}

fn quality_gate(root: &Path, plan: &Integration) -> Result<()> {
    use std::time::{Duration, Instant};
    let args: Vec<_> = plan.gate.iter().skip(1).map(String::as_str).collect();
    let mut command = process::build_command(&plan.gate[0], &args, root, &[]);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    // Platforms without a process-group termination contract fail closed.
    #[cfg(not(unix))]
    return Err(refused(
        "bounded integration gates currently require Unix process groups",
    ));
    #[cfg(unix)]
    {
        let mut child = command.spawn().map_err(|source| Error::Spawn {
            program: plan.gate[0].clone(),
            source,
        })?;
        let deadline = Instant::now() + Duration::from_secs(plan.gate_timeout_seconds);
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    return if status.success() {
                        Ok(())
                    } else {
                        Err(refused(format!("quality gate failed ({status})")))
                    }
                }
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(20))
                }
                result => {
                    let group = format!("-{}", child.id());
                    let _ = process::run("kill", &["-KILL", "--", &group], root);
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(refused(match result {
                        Err(error) => format!("quality gate wait failed: {error}"),
                        _ => "quality gate timed out".into(),
                    }));
                }
            }
        }
    }
}

/// Integrate one locally approved acceptance; only the target ref may be changed.
pub fn integrate(root: &Path, selected: &Cid) -> Result<serde_json::Value> {
    let root = git::discover_root(root)?;
    let store = SidecarFlowStore::open(&root).map_err(flow_error)?;
    let plan = accepted(&root, selected, &store.records().map_err(flow_error)?)?;
    validate_git(&root, &plan)?;
    let changes = git::changes_against(&root, &plan.target_commit)?;
    let readiness = ready::run(&root, &plan.target_commit, &changes, false)?;
    if !readiness.ready {
        return Err(refused(format!(
            "native ready gate failed: {}",
            serde_json::to_string(&readiness)?
        )));
    }
    quality_gate(&root, &plan)?;
    validate_git(&root, &plan)?;
    let readiness = ready::run(&root, &plan.target_commit, &changes, false)?;
    if !readiness.ready {
        return Err(refused("native ready gate failed after the quality gate"));
    }

    // Never run user gates while holding the append lock: gates may append evidence.
    // Reconciliation under this same lock observes their new records and refuses reds.
    let transaction = store.transaction().map_err(flow_error)?;
    let revalidated = accepted(&root, selected, &transaction.records().map_err(flow_error)?)?;
    if revalidated != plan {
        return Err(refused("integration report changed during gates"));
    }
    validate_git(&root, &plan)?;
    // Doctor is the existing non-mutating floor reader, not the potentially
    // script-running quality validators. Never execute those under the append lock.
    if !doctor::run_at(&root, &["HEAD", &plan.target_commit])?.ready {
        return Err(refused(
            "native governance floor changed before integration",
        ));
    }
    let mut command =
        process::build_command("git", &["update-ref", "--no-deref", "--stdin"], &root, &[]);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| Error::Spawn {
            program: "git update-ref".into(),
            source,
        })?;
    let input = format!(
        "start\nverify {} {}\nupdate {} {} {}\nprepare\ncommit\n",
        plan.source_ref,
        plan.source_commit,
        plan.target_ref,
        plan.source_commit,
        plan.target_commit
    );
    let written = child
        .stdin
        .take()
        .ok_or_else(|| refused("Git transaction stdin missing"))?
        .write_all(input.as_bytes());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while child
        .try_wait()
        .map_err(|source| Error::Spawn {
            program: "git update-ref wait".into(),
            source,
        })?
        .is_none()
    {
        if std::time::Instant::now() >= deadline {
            #[cfg(unix)]
            {
                let group = format!("-{}", child.id());
                let _ = process::run("kill", &["-KILL", "--", &group], &root);
            }
            let _ = child.kill();
            let _ = child.wait();
            return Err(refused("Git ref transaction timed out; outcome uncertain, inspect source/target refs before retrying (no rollback attempted)"));
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let output = child.wait_with_output().map_err(|source| Error::Spawn {
        program: "git update-ref".into(),
        source,
    })?;
    written.map_err(|source| Error::Spawn {
        program: "git update-ref stdin".into(),
        source,
    })?;
    if !output.status.success() {
        return Err(refused(format!(
            "atomic ref transaction failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    drop(transaction);
    Ok(
        serde_json::json!({"acceptance":selected.to_string(), "sourceCommit":plan.source_commit,
        "targetRef":plan.target_ref,"previousTargetCommit":plan.target_commit,"integrated":true,"pushed":false}),
    )
}

pub fn run(args: &[String]) -> Result<ExitCode> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("epr integrate --acceptance <CID> [--root DIR]\nLocally approved, target-bound fast-forward; runs pinned gates and never pushes.");
        return Ok(ExitCode::SUCCESS);
    }
    let mut root = std::env::current_dir().map_err(|source| Error::Read {
        path: ".".into(),
        source,
    })?;
    let mut selected = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let value = iter
            .next()
            .ok_or_else(|| refused("expected --acceptance CID [--root DIR]"))?;
        match arg.as_str() {
            "--root" => root = value.into(),
            "--acceptance" if selected.is_none() => {
                selected = Some(value.parse::<Cid>().map_err(flow_error)?)
            }
            _ => return Err(refused("expected --acceptance CID [--root DIR]")),
        }
    }
    let selected = selected.ok_or_else(|| refused("--acceptance CID is required"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&integrate(&root, &selected)?)?
    );
    Ok(ExitCode::SUCCESS)
}
