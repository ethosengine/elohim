//! Private whole-tree input adapter for `task`: no Git lookup and no reusable derivation.
//! Canonical tree verification/capture stays EPRFS-owned; admission and process execution
//! stay in the existing Ark task path. Runtime substitutions never edit the checked spec.

use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use ark_core::{Berth, TaskCompletion, TaskOutcome, TaskSpec, TASK_CHILD_NAME, UNIDENTIFIED_AGENT};
use elohim_epr_rea::AgentRef;
use eprfs_core::{
    BlobCid, DerivationUnanswered, FetchPolicy, MethodDescriptor, ProjectedRoot, TreeLimits,
};
use eprfs_storage::DirectoryStorage;
use placement_core::GrantScope;
use seam_contracts::{ComeBack, Meanwhile, PendingStatus, Unanswered};

use crate::{
    task,
    task_files::{self as files, failure},
    Failure,
};

#[cfg(test)]
mod tests;

pub(crate) struct RunArgs {
    pub spec: Option<PathBuf>,
    pub request: Option<PathBuf>,
    pub root: Option<String>,
    pub store: PathBuf,
    pub run_dir: PathBuf,
    pub artifact: PathBuf,
    pub device: Option<String>,
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, Failure> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(failure)?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(failure)?;
    if bytes.len() > 1024 * 1024 {
        return Err(failure("request/spec exceeds 1 MiB"));
    }
    serde_json::from_slice(&bytes).map_err(failure)
}

fn from_request(path: &Path, root: BlobCid) -> Result<TaskSpec, Failure> {
    let request: task::TaskRequest = read_json(path)?;
    let method = MethodDescriptor {
        name: request.method.name,
        tools: request.method.tools,
        flags: request.method.flags,
        params: None,
        exec: Some(
            request
                .child
                .exec_cid(None, &request.outputs)
                .map_err(failure)?,
        ),
        step: request.method.step,
        embedder: None,
        view: eprfs_core::View::whole().paths().cloned().collect(),
        non_path_inputs: request.method.non_path_inputs,
        deterministic: request.method.deterministic,
    };
    TaskSpec::new(method, root, None, request.child, None, request.outputs).map_err(failure)
}

pub(crate) fn spec(request: &Path, root: &str, store: &Path) -> Result<u8, Failure> {
    let spec = from_request(request, BlobCid::parse(root).map_err(failure)?)?;
    let storage = DirectoryStorage::open(store).map_err(failure)?;
    let projected = futures::executor::block_on(ProjectedRoot::verify_whole_tree(
        &storage,
        spec.input().root(),
        &spec.input().view(),
        FetchPolicy::LocalOnly,
        &TreeLimits::default(),
    ))
    .map_err(failure)?;
    let key = spec.derivation_key(&projected).map_err(failure)?;
    println!(
        "{}",
        serde_json::json!({"spec": spec, "derivation": {"method": key.method, "input": key.input.cid()}})
    );
    Ok(0)
}

pub(crate) fn run(args: RunArgs) -> Result<u8, Failure> {
    let spec = match (&args.spec, &args.request, &args.root) {
        (Some(path), None, root) => {
            let spec: TaskSpec = read_json(path)?;
            if let Some(root) = root {
                if BlobCid::parse(root).map_err(failure)? != *spec.input().root() {
                    return Err(failure("root differs from spec"));
                }
            }
            spec
        }
        (None, Some(request), Some(root)) => {
            from_request(request, BlobCid::parse(root).map_err(failure)?)?
        }
        _ => return Err(failure("select --spec or --request with --root")),
    };
    if spec.scope() != Some(GrantScope::OwnEnvelope) {
        return Err(failure(
            "native input requires the unchanged brit-gate OwnEnvelope method",
        ));
    }
    if spec.base_root().is_some() {
        return Err(failure("native input does not support base-root reuse"));
    }
    if !spec.child().readiness.is_empty() {
        return Err(failure(
            "native one-shot tasks do not support readiness probes",
        ));
    }
    validate_templates(&spec)?;
    let limits = TreeLimits::default();
    let storage = DirectoryStorage::open(&args.store).map_err(failure)?;
    let projected = futures::executor::block_on(ProjectedRoot::verify_whole_tree(
        &storage,
        spec.input().root(),
        &spec.input().view(),
        FetchPolicy::LocalOnly,
        &limits,
    ))
    .map_err(failure)?;
    let tree = futures::executor::block_on(eprfs_core::load_verified_tree(
        &storage,
        spec.input().root(),
        FetchPolicy::LocalOnly,
        &limits,
    ))
    .map_err(failure)?;
    let bytes = files::expanded_bytes(&tree, &limits)?;
    let expected = files::expected_input(&tree, &limits)?;
    drop(tree);
    let key = spec.derivation_key(&projected).map_err(failure)?;
    let task_id = spec.cid().map_err(failure)?;
    // Reject template-like directory names rather than let the driver's second template
    // pass reinterpret caller-chosen runtime path bytes.
    if args
        .run_dir
        .to_str()
        .is_none_or(|p| p.contains(['{', '}', '\0']))
    {
        return Err(failure(
            "run directory must be UTF-8 without template braces or NUL",
        ));
    }
    let run = files::new_run(&args.run_dir, &args.store)?;
    let status = task::StatusFile(run.join("status.jsonl"));
    task::install_interrupt_handlers()?;
    status.append(&PendingStatus::Queued)?;
    let result = execute(&args, &spec, &run, &storage, bytes, &expected, &status);
    let (exit, output) = match result {
        Ok(value) if task::interruption().is_none() => value,
        other => {
            status.append(&PendingStatus::Unanswered(Unanswered::new(
                DerivationUnanswered::NotCarried,
                ComeBack::OnEvent {
                    event: "rerun-requested-in-a-new-directory".into(),
                },
                Meanwhile::Nothing {
                    because: format!(
                        "no accepted output; retained run directory {}",
                        run.display()
                    ),
                },
            )))?;
            if let Some(signal) = task::interruption() {
                return Ok(128u8.saturating_add(signal as u8));
            }
            return match other {
                Err(error) => Err(error),
                Ok(_) => Err(failure("interrupted before result")),
            };
        }
    };
    let completion = TaskCompletion::Local {
        device: args.device.unwrap_or_else(|| UNIDENTIFIED_AGENT.into()),
        at: task::epoch_secs(),
    };
    let outcome =
        TaskOutcome::new(task_id, key, exit, output, None, completion).map_err(failure)?;
    #[cfg(test)]
    tests::before_answer(&run);
    // Check after capture and outcome construction, immediately before committing the
    // terminal answer. Never append an answered row and then retract it as interrupted.
    if let Some(signal) = task::interruption() {
        status.append(&PendingStatus::Unanswered(Unanswered::new(
            DerivationUnanswered::NotCarried,
            ComeBack::OnEvent {
                event: "rerun-requested".into(),
            },
            Meanwhile::Nothing {
                because: "interrupted before response".into(),
            },
        )))?;
        return Ok(128u8.saturating_add(signal as u8));
    }
    status.append(&PendingStatus::Answered(outcome.clone()))?;
    println!("{}", serde_json::to_string(&outcome).map_err(failure)?);
    Ok(task::exit_code(&exit))
}

fn execute(
    args: &RunArgs,
    spec: &TaskSpec,
    run: &Path,
    storage: &DirectoryStorage,
    bytes: u64,
    expected: &files::Inventory,
    status: &task::StatusFile,
) -> Result<(ark_core::ExitClass, Option<BlobCid>), Failure> {
    let limits = &TreeLimits::default();
    let input = run.join("input");
    let output = run.join("output");
    let home = run.join("home");
    let artifact = files::retain_artifact(&args.artifact, run, &spec.child().artifact)?;
    let manifest = task::device_manifest(run)?;
    let berth = Berth {
        manifest: manifest.cid().map_err(failure)?,
        data_root: run.to_owned(),
        artifacts: BTreeMap::from([(TASK_CHILD_NAME.to_owned(), artifact.clone())]),
        ..Berth::default()
    };
    let me = AgentRef(
        args.device
            .clone()
            .unwrap_or_else(|| UNIDENTIFIED_AGENT.into()),
    );
    if let Err(refused) = task::carried_here(spec, bytes, &me, &berth, run) {
        status.append(&PendingStatus::Unanswered(
            (*refused).map_why(|_| DerivationUnanswered::NotCarried),
        ))?;
        return Err(failure(
            "native input not carried by the local disk admission floor",
        ));
    }
    let mut child = spec.child_spec();
    let expand = |value: &str| expand(value, run, &input, &output, &artifact);
    child.argv = child
        .argv
        .iter()
        .map(|arg| expand(arg))
        .collect::<Result<_, _>>()?;
    child.env = child
        .env
        .iter()
        .map(|(k, v)| Ok((k.clone(), expand(v)?)))
        .collect::<Result<_, Failure>>()?;
    if child.env.get("HOME") != Some(&home.to_string_lossy().into_owned()) {
        return Err(failure(
            "explicit HOME must resolve to this run's private home directory",
        ));
    }
    futures::executor::block_on(eprfs_local::restore_exact_tree(
        storage,
        spec.input().root(),
        &input,
        FetchPolicy::LocalOnly,
        limits,
        &eprfs_host::HostProfile::current_platform(),
    ))
    .map_err(failure)?;
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&input, fs::Permissions::from_mode(0o700)).map_err(failure)?;
    }
    files::private_dir(&output)?;
    files::private_dir(&home)?;
    let before = files::inventory(&input, limits)?;
    if !files::matches_candidate(&before, expected) {
        return Err(failure(
            "restored input inventory differs from the verified candidate",
        ));
    }
    // Persist the ORIGINAL checked spec, not the expanded runtime child.
    fs::write(
        run.join("spec.json"),
        serde_json::to_vec(spec).map_err(failure)?,
    )
    .map_err(failure)?;
    if task::interruption().is_some() {
        return Err(failure("interrupted before spawn"));
    }
    let (exit, interrupted, descendants) = task::execute(&child, &berth, true)?;
    if interrupted.is_some() {
        return Err(failure("interrupted during execution"));
    }
    if descendants {
        return Err(failure(
            "task left surviving descendants; output not accepted",
        ));
    }
    if files::inventory(&input, limits)? != before {
        return Err(failure("whole input inventory changed during execution"));
    }
    if task::exit_code(&exit) != 0 {
        return Ok((exit, None));
    }
    let inventory = files::inventory(&output, limits)?;
    let selected = files::selected_outputs(&inventory, spec.outputs())?;
    let root = if selected.is_empty() {
        None
    } else {
        Some(
            futures::executor::block_on(eprfs_local::capture_tree(
                &output,
                &args.store,
                &selected,
                limits,
            ))
            .map_err(failure)?
            .root,
        )
    };
    if files::inventory(&input, limits)? != before
        || files::inventory(&output, limits)? != inventory
    {
        return Err(failure("input or output changed during capture"));
    }
    if task::interruption().is_some() {
        return Err(failure("interrupted during capture"));
    }
    Ok((exit, root))
}

fn validate_templates(spec: &TaskSpec) -> Result<(), Failure> {
    for key in ["PATH", "HOME"] {
        if !spec.child().env.contains_key(key) {
            return Err(failure(format!("explicit {key} is required")));
        }
    }
    for key in spec.child().env.keys() {
        if key.is_empty() || key.contains(['=', '\0', '{', '}']) {
            return Err(failure("invalid environment key"));
        }
    }
    for value in spec.child().argv.iter().chain(spec.child().env.values()) {
        expand(
            value,
            Path::new("/run"),
            Path::new("/run/input"),
            Path::new("/run/output"),
            Path::new("/run/artifact"),
        )?;
    }
    Ok(())
}

fn expand(
    value: &str,
    run: &Path,
    input: &Path,
    output: &Path,
    artifact: &Path,
) -> Result<String, Failure> {
    if value.contains('\0') {
        return Err(failure("NUL in native template"));
    }
    let mut result = String::new();
    let mut rest = value;
    while let Some(open) = rest.find('{') {
        let (plain, tail) = rest.split_at(open);
        if plain.contains('}') {
            return Err(failure("unmatched template brace"));
        }
        result.push_str(plain);
        let (key, after) = tail[1..]
            .split_once('}')
            .ok_or_else(|| failure("unterminated placeholder"))?;
        let path = match key {
            "input" => input,
            "output" => output,
            "data_root" => run,
            "artifact" => artifact,
            _ => return Err(failure(format!("unknown native placeholder {{{key}}}"))),
        };
        result.push_str(
            path.to_str()
                .ok_or_else(|| failure("runtime path must be UTF-8"))?,
        );
        rest = after;
    }
    if rest.contains(['}', '\0']) {
        return Err(failure("invalid template bytes"));
    }
    result.push_str(rest);
    Ok(result)
}
