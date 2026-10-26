//! Arrival is a reconstructed local view (C); it creates no peer facts or DHT heads.
use crate::{
    actor, device_key,
    flow::memory::recall::{embedder, index},
    git,
};
use elohim_device_runtime::{
    bin_store::{Registry, Store},
    session,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fmt, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

/// Whether a condition works, lacks an enhancement, prevents work, or cannot be observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    /// The condition is met.
    Ok,
    /// Local work remains possible without this enhancement.
    Degraded,
    /// The device cannot safely perform its work.
    Refused,
    /// The fact cannot currently be observed (never assumed false).
    Unknown,
}
/// One arrival condition and its actionable remedy, in dependency order.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    /// Stable human-readable condition name.
    pub name: String,
    /// Observed condition.
    pub state: State,
    /// Evidence or the reason it could not be obtained.
    pub detail: String,
    /// Command or action that restores the condition.
    pub cure: Option<String>,
}
impl Check {
    fn new(name: &str, state: State, detail: impl Into<String>, cure: Option<String>) -> Self {
        Self {
            name: name.into(),
            state,
            detail: detail.into(),
            cure,
        }
    }
}
impl fmt::Display for Check {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = match self.state {
            State::Ok => "ok",
            State::Degraded => "degraded",
            State::Refused => "REFUSED",
            State::Unknown => "unknown",
        };
        write!(f, "{state} {}: {}", self.name, self.detail)?;
        if let Some(cure) = &self.cure {
            write!(f, " — {cure}")?;
        }
        Ok(())
    }
}

/// Read this device's arrival conditions; never enroll, install, or build during a status read.
pub fn status() -> Vec<Check> {
    let here = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    status_at(&git::checkout(&here).unwrap_or(here))
}

pub(crate) fn status_at(root: &Path) -> Vec<Check> {
    let identity = session::resolve();
    let registry = Registry::discover();
    let env = |key: &str| std::env::var(key).ok();
    let binary = |tool| {
        match &registry {
        Ok(registry) => match Store::from_environment(tool, registry) {
            Ok(store) => tool_check(tool, &store, &identity, &env),
            Err(error) => Check::new(
                &format!("{tool} binary"),
                State::Refused,
                error.to_string(),
                Some(format!("epr bin bootstrap --tool {tool}")),
            ),
        },
        Err(error) => Check::new(
            &format!("{tool} binary"),
            State::Refused,
            error.to_string(),
            Some("restore genesis/orchestrator/manifests/runtime/workspace-device.manifest.json or set EPR_BIN_TOOLS".into()),
        ),
    }
    };
    let key = device_key::resolve_path();
    let handle = sole_handle(root).unwrap_or_else(|| "<handle>".into());
    let key_check = match &key {
        Ok(path) => key_check(path, &handle),
        Err(error) => Check::new(
            "device key",
            State::Refused,
            error.to_string(),
            Some("set XDG_CONFIG_HOME to this device's persistent config directory".into()),
        ),
    };
    vec![
        binary("epr"),
        binary("brit"),
        key_check,
        roster_check(root, key.as_deref().ok(), &handle),
        model_check(root),
        fold_check(
            index::status(root, index::EmbedderChoice::Pinned),
            lag_limit(root),
        ),
        berth_check(&berth_dir(), identity.session.as_deref()),
        repository_check(root),
        peer_daemon_check(),
        supervisor_check(root),
    ]
}

/// Whether ark runs the device's declared manifest at its head with every child ready.
/// Absence degrades and never refuses: local work goes on without a supervisor, losing only
/// restart and witnessed deaths (local-first). Like every arrival line, it is a read.
fn supervisor_check(root: &Path) -> Check {
    let (ok, detail, cure) = super::supervise::check(root);
    Check::new(
        "device supervisor",
        if ok { State::Ok } else { State::Degraded },
        detail,
        cure,
    )
}

fn peer_daemon_check() -> Check {
    let observed = (|| {
        let store = crate::bin::daemon_store()?;
        let leases = store.live_daemons(&crate::bin::daemon_proc_root())?;
        let head = store.resolve_current(None).ok().and_then(|r| r.cid);
        Ok::<_, elohim_device_runtime::bin_store::Refusal>((leases, head))
    })();
    match observed {
        Ok((leases, head)) => {
            let stale: Vec<_> = leases
                .into_iter()
                .map(|pin| pin.running_cid.unwrap_or(pin.cid))
                .filter(|cid| Some(cid) != head.as_ref())
                .collect();
            if stale.is_empty() {
                Check::new(
                    "peer daemon",
                    State::Ok,
                    "at current or no daemon running",
                    None,
                )
            } else {
                Check::new(
                    "peer daemon",
                    State::Degraded,
                    format!(
                        "running {}; current {}",
                        stale.join(", "),
                        head.as_deref().unwrap_or("absent")
                    ),
                    Some("restart epr peer serve to follow current".into()),
                )
            }
        }
        Err(error) => Check::new(
            "peer daemon",
            State::Unknown,
            error.to_string(),
            Some("restore access to the epr bin store".into()),
        ),
    }
}

fn tool_check(
    tool: &str,
    store: &Store,
    identity: &session::SessionIdentity,
    env: &dyn Fn(&str) -> Option<String>,
) -> Check {
    let name = format!("{tool} binary");
    let cure = Some(format!("epr bin bootstrap --tool {tool}"));
    let shadows = store.shadows(&env("PATH").unwrap_or_default());
    if !shadows.is_empty() {
        let paths: Vec<_> = shadows.iter().filter_map(|s| s["path"].as_str()).collect();
        return Check::new(
            &name,
            State::Refused,
            format!("PATH reaches {} before the launcher", paths.join(", ")),
            Some("epr device arrive; remove any remaining named PATH shadow".into()),
        );
    }
    match store.status(identity, None, false, env) {
        Ok(view)
            if view
                .resolved
                .as_ref()
                .is_some_and(|p| crate::doctor::is_executable(p)) =>
        {
            let running = if tool == "epr" {
                std::env::current_exe()
                    .ok()
                    .and_then(|path| store.executable_cid(&path).ok())
            } else {
                view.resolved_cid.clone()
            };
            let detail = format!(
                "{} vs head {} ({}{})",
                running.as_deref().unwrap_or("candidate"),
                view.head.as_deref().unwrap_or("none"),
                view.source,
                if view.behind { ", pinned behind" } else { "" }
            );
            let declared_run = running == view.resolved_cid
                && matches!(
                    view.source.as_str(),
                    "candidate" | "explicit-pin" | "session-pin"
                );
            let behind_unpinned = running != view.head && !declared_run;
            if !view.verified_current || behind_unpinned {
                Check::new(
                    &name,
                    State::Degraded,
                    detail,
                    Some(format!(
                        "epr device arrive (or declare {tool}'s candidate/pin)"
                    )),
                )
            } else {
                Check::new(&name, State::Ok, detail, None)
            }
        }
        Ok(_) => Check::new(
            &name,
            absent_state(tool),
            "no executable candidate, pin, or committed head",
            cure,
        ),
        Err(error) => Check::new(&name, absent_state(tool), error.to_string(), cure),
    }
}

/// A device cannot work without its own `epr`; every other followed tool is an enhancement
/// (brit's absence costs the gate its `rakia affected` oracle, which falls back to path
/// selection), so its absence degrades instead of refusing (local-first, ground rule 2).
fn absent_state(tool: &str) -> State {
    if tool == "epr" {
        State::Refused
    } else {
        State::Degraded
    }
}
fn key_check(path: &Path, handle: &str) -> Check {
    match device_key::DeviceKey::load(path) {
        Ok(_) => Check::new("device key", State::Ok, path.display().to_string(), None),
        Err(error) => Check::new(
            "device key",
            State::Refused,
            format!("{}: {error}", path.display()),
            Some(format!("epr actor device enroll --handle {handle}")),
        ),
    }
}
fn sole_handle(root: &Path) -> Option<String> {
    let mut handles: Vec<_> = fs::read_dir(root.join(actor::PARTICIPANTS_REL))
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            (path.extension()?.to_str()? == "jsonl")
                .then(|| path.file_stem()?.to_str().map(str::to_owned))
                .flatten()
        })
        .filter(|name| !name.starts_with('.'))
        .collect();
    (handles.len() == 1).then(|| handles.remove(0))
}
fn roster_check(root: &Path, key: Option<&Path>, handle: &str) -> Check {
    let current = key.map_or(actor::DeviceState::Unwitnessed, |key| {
        actor::device_state_read_only(root, key)
    });
    if let actor::DeviceState::Standing(standing) = &current {
        return Check::new("roster bound", State::Ok, standing.describe(), None);
    }
    let pending = root
        .join(actor::PARTICIPANTS_REL)
        .join(format!(".enroll-{handle}.json"))
        .is_file();
    let first = !root
        .join(actor::PARTICIPANTS_REL)
        .join(format!("{handle}.jsonl"))
        .is_file();
    let cure = if first {
        // A human claim alone does not create a roster; the present agent witnesses the first device.
        format!("first device here: epr actor claim --as human:{handle} --session <session>; then epr actor witness --subject human:{handle} --as agent:<role>@<model> --session <session> --basis '<human present>'; commit {}/{handle}.jsonl", actor::PARTICIPANTS_REL)
    } else {
        format!("epr actor device enroll --handle {handle} here; on a device already in {handle}'s roster run epr actor device authorize '<request>'; then here epr actor device bind '<authorization>' and commit {}/{handle}.jsonl", actor::PARTICIPANTS_REL)
    };
    Check::new(
        "roster bound",
        State::Refused,
        if matches!(current, actor::DeviceState::RootContested(_)) {
            "roster root contested; review the pinned root before enrollment"
        } else if pending {
            "enrollment pending"
        } else {
            "this device has no roster standing"
        },
        Some(cure),
    )
}
fn model_check(root: &Path) -> Check {
    let manifest = embedder::ModelManifest::load(&root.join(embedder::MODEL_MANIFEST_REL));
    let detail = match manifest {
        Ok(manifest) => match manifest.resolve_model_dir() {
            Some(path) if path.join("model.onnx").is_file() && path.join("tokenizer.json").is_file() => return Check::new("embed model", State::Ok, path.display().to_string(), None),
            _ => "no model.onnx + tokenizer.json at the model's resolved directory; semantic recall degrades".into(),
        },
        Err(error) => format!("model manifest unavailable: {error}; semantic recall degrades"),
    };
    Check::new(
        "embed model",
        State::Degraded,
        detail,
        Some("genesis/agentic/bin/embed-model-provision".into()),
    )
}
fn lag_limit(root: &Path) -> u64 {
    fs::read(root.join(".epr-meta/elohim/algorithms/recall-semantic-index.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|value| value["foldLag"]["limit"].as_u64())
        .unwrap_or(25)
}
fn fold_check(value: crate::flow::FlowResult<Value>, limit: u64) -> Check {
    let cure = "epr flow memory index fold --max-files 1";
    let value = match value {
        Ok(value) => value,
        Err(error) => {
            return Check::new(
                "fold attested",
                State::Degraded,
                error.to_string(),
                Some(cure.into()),
            )
        }
    };
    let state = value["last"]["state"]["state"]
        .as_str()
        .or_else(|| value["last"]["state"].as_str())
        .unwrap_or("unfolded");
    if state == "failed" {
        return Check::new(
            "fold attested",
            State::Degraded,
            format!(
                "last fold failed: {}",
                value["last"]["state"]["why"]
                    .as_str()
                    .unwrap_or("unknown reason")
            ),
            Some(format!("cure embed model, then {cure}")),
        );
    }
    match value["lag"].as_u64() {
        Some(lag) => Check::new(
            "fold attested",
            if state == "complete" && lag <= limit {
                State::Ok
            } else {
                State::Degraded
            },
            format!("{state}, {lag} files behind (limit {limit})"),
            if state == "complete" && lag <= limit {
                None
            } else {
                Some(format!(
                    "epr flow memory index fold --max-files {}",
                    if lag > limit {
                        lag.saturating_add(50).max(100)
                    } else {
                        1
                    }
                ))
            },
        ),
        None => Check::new(
            "fold attested",
            State::Degraded,
            "no attested fold lag; lexical recall remains available",
            Some(cure.into()),
        ),
    }
}
fn berth_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("CLAUDE_CONFIG_DIR").unwrap_or_else(|_| "/projects/.claude-config".into()),
    )
    .join("berth/moorings")
}
fn berth_check(dir: &Path, session: Option<&str>) -> Check {
    let records: Vec<Value> = fs::read_dir(dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|e| fs::read(e.path()).ok())
        .filter_map(|bytes| serde_json::from_slice(&bytes).ok())
        .collect();
    let found = records.iter().any(|value| {
        value["session"]
            .as_str()
            .is_some_and(|s| session.is_none_or(|expected| s == expected))
    });
    if found {
        Check::new(
            "berth moored",
            State::Ok,
            session.map_or_else(
                || format!("{} mooring(s), no session named", records.len()),
                |s| format!("session {s} moored"),
            ),
            None,
        )
    } else {
        Check::new(
            "berth moored",
            State::Degraded,
            "no mooring for this session; coordinated capacity unavailable",
            Some(format!(
                "berth moor --session {} --runtime {}",
                session.unwrap_or("<id>"),
                session::resolve().runtime
            )),
        )
    }
}
pub(crate) fn repository_check(root: &Path) -> Check {
    match git::accepted(root) {
        Ok(true) => Check::new(
            "repository trusted",
            State::Ok,
            format!(
                "git accepts {} natively or by this device's declaration",
                root.display()
            ),
            None,
        ),
        result => Check::new(
            "repository trusted",
            State::Refused,
            match result {
                Err(e) => e.to_string(),
                _ => format!("git does not accept {}", root.display()),
            },
            Some(format!(
                "epr device trust '{}'",
                root.display().to_string().replace('\'', "'\\''")
            )),
        ),
    }
}

pub(super) fn run(args: &[String], arrive: bool) -> Result<ExitCode, crate::error::Error> {
    let mut root = None;
    let mut json = false;
    let mut strict = false;
    let mut worker = false;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--root" => {
                root = Some(PathBuf::from(iter.next().ok_or_else(|| {
                    crate::error::Error::InvalidArguments("--root needs a path".into())
                })?))
            }
            "--json" => json = true,
            "--strict" if arrive => strict = true,
            "--worker" if arrive => worker = true,
            _ => {
                return Err(crate::error::Error::InvalidArguments(format!(
                    "unknown device arrival flag {arg}"
                )))
            }
        }
    }
    let here = std::env::current_dir().map_err(|source| crate::error::Error::Read {
        path: PathBuf::from("."),
        source,
    })?;
    let root = root.unwrap_or_else(|| git::checkout(&here).unwrap_or(here));
    if arrive {
        let result = if worker {
            super::arrival_work::work(&root)
        } else {
            super::arrival_work::start(&root)
        };
        if let Err(error) = result {
            eprintln!("device arrival (continuing): {error}");
        }
        if worker {
            return Ok(ExitCode::SUCCESS);
        }
    }
    let checks = status_at(&root);
    if json {
        println!("{}", serde_json::to_string(&checks)?);
    } else {
        for check in &checks {
            println!("{check}");
        }
    }
    Ok(
        if (!arrive || strict) && checks.iter().any(|c| c.state == State::Refused) {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::os::unix::fs::PermissionsExt;

    // Parity: device_preflight_test.py -> native checks below:
    // seven_ok/order -> CLI schema/order; missing epr/brit + status -> candidates_and_absence;
    // no_key -> missing_and_invalid_keys; unbound/pending/first device -> roster_cures;
    // model absence/expansion -> absent_model_degrades + embedder::tests::resolve_*;
    // failed/lag/missing fold -> fold_states; missing berth -> moorings_use_recorded_session;
    // no epr no longer prevents library reads (CLI keyless test); dependencies are native calls.
    // guard registry fail-open -> CLI arrive_bad_registry; bootstrap/follow ordering -> arrival_work tests.
    // hand-installed bytes/shadows -> device-runtime bin_store::tests::test_guard_* and missing_launcher_*.
    #[test]
    fn missing_and_invalid_keys_refuse_without_minting() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("key/seed");
        let check = key_check(&path, "matthew");
        assert_eq!(check.state, State::Refused);
        assert!(check.cure.unwrap().contains("enroll --handle matthew"));
        assert!(!path.exists());
        fs::create_dir(path.parent().unwrap()).unwrap();
        fs::write(&path, b"bad").unwrap();
        assert_eq!(key_check(&path, "matthew").state, State::Refused);
        device_key::DeviceKey::load_or_generate(&dir.path().join("valid/seed")).unwrap();
        assert_eq!(
            key_check(&dir.path().join("valid/seed"), "matthew").state,
            State::Ok
        );
    }
    #[test]
    fn roster_cures_distinguish_first_unbound_and_pending_devices() {
        let dir = tempfile::tempdir().unwrap();
        let first = roster_check(dir.path(), None, "matthew");
        assert!(first.cure.unwrap().contains("actor witness"));
        let roster = dir.path().join(actor::PARTICIPANTS_REL);
        fs::create_dir_all(&roster).unwrap();
        fs::write(roster.join("matthew.jsonl"), "").unwrap();
        let unbound = roster_check(dir.path(), None, "matthew");
        for verb in ["enroll", "authorize", "bind"] {
            assert!(unbound.cure.as_ref().unwrap().contains(verb));
        }
        fs::write(roster.join(".enroll-matthew.json"), "{}").unwrap();
        assert!(roster_check(dir.path(), None, "matthew")
            .detail
            .contains("pending"));
    }
    #[test]
    fn absent_model_degrades_without_refusing_local_work() {
        let dir = tempfile::tempdir().unwrap();
        let check = model_check(dir.path());
        assert_eq!(check.state, State::Degraded);
        assert!(check.cure.unwrap().contains("embed-model-provision"));
    }
    #[test]
    fn fold_states_preserve_failure_reason_and_catch_up_size() {
        let check = |state: &str, lag: u64| {
            fold_check(
                Ok(json!({"lag":lag,"last":{"state":{"state":state,"why":"model unavailable"}}})),
                25,
            )
        };
        assert_eq!(check("complete", 25).state, State::Ok);
        assert_eq!(check("complete", 26).state, State::Degraded);
        assert!(check("failed", 400).detail.contains("model unavailable"));
        assert!(check("failed", 400).cure.unwrap().contains("embed model"));
        assert!(check("degraded", 400)
            .cure
            .unwrap()
            .contains("--max-files 450"));
        assert_eq!(fold_check(Ok(json!({})), 25).state, State::Degraded);
        assert_eq!(
            fold_check(
                Err(crate::flow::FlowError::Unavailable("unreadable".into())),
                25
            )
            .state,
            State::Degraded
        );
    }
    #[test]
    fn moorings_use_recorded_session_not_file_presence() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("empty.json"), "{}").unwrap();
        assert_eq!(berth_check(dir.path(), Some("s")).state, State::Degraded);
        fs::write(dir.path().join("record.json"), r#"{"session":"s"}"#).unwrap();
        assert_eq!(berth_check(dir.path(), Some("s")).state, State::Ok);
        assert_eq!(
            berth_check(dir.path(), Some("other")).state,
            State::Degraded
        );
        assert_eq!(berth_check(dir.path(), None).state, State::Ok);
    }
    #[test]
    fn candidates_and_absence_are_read_through_bin_store() {
        let dir = tempfile::tempdir().unwrap();
        let binary = dir.path().join("candidate");
        fs::write(&binary, b"candidate bytes").unwrap();
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).unwrap();
        let registry: Registry = serde_json::from_value(json!({"version":1,"tools":{
            "epr":{"bins":["epr"],"candidateEnv":{"epr":["EPR_BIN"]}},
            "brit":{"bins":["brit"],"candidateEnv":{"brit":["BRIT_BIN"]}}
        }}))
        .unwrap();
        let identity = session::SessionIdentity {
            runtime: "none".into(),
            session: None,
            owner_pid: None,
            owner_start_time: None,
            source: session::Source::None,
        };
        for tool in ["epr", "brit"] {
            let store = Store::new(dir.path().join(tool), tool, &registry).unwrap();
            let absent = tool_check(tool, &store, &identity, &|_| None);
            // Only the device's own epr is essential; brit's absence is an enhancement lost.
            let expected = if tool == "epr" {
                State::Refused
            } else {
                State::Degraded
            };
            assert_eq!(absent.state, expected, "{tool}");
            assert!(absent.cure.unwrap().contains(&format!("--tool {tool}")));
            let candidate = tool_check(tool, &store, &identity, &|k| {
                (k == format!("{}_BIN", tool.to_uppercase())).then(|| binary.display().to_string())
            });
            assert_eq!(candidate.state, State::Degraded);
        }
    }
    #[test]
    fn schema_contract_covers_every_state_and_only_declared_fields() {
        let schema: Value = serde_json::from_str(include_str!(
            "../../../../sdk/schemas/v1/views/device-status.schema.json"
        ))
        .unwrap();
        assert_eq!(schema["type"], "array");
        for state in [State::Ok, State::Degraded, State::Refused, State::Unknown] {
            let check = Check::new(
                "condition",
                state,
                "evidence",
                (state != State::Ok).then(|| "cure".into()),
            );
            let value = serde_json::to_value(&check).unwrap();
            assert_eq!(
                value.as_object().unwrap().len(),
                schema["items"]["properties"].as_object().unwrap().len()
            );
            for key in schema["items"]["required"].as_array().unwrap() {
                assert!(value.get(key.as_str().unwrap()).is_some());
            }
            assert!(schema["items"]["properties"]["state"]["enum"]
                .as_array()
                .unwrap()
                .contains(&value["state"]));
            assert!(value["name"].is_string() && value["detail"].is_string());
            assert!(value["cure"].is_null() || value["cure"].is_string());
            assert!(check.to_string().contains("condition: evidence"));
        }
    }
}
