//! Fail-open arrival orchestration. Long custody work runs in a detached, device-locked worker.
use elohim_device_runtime::{
    bin_store::{Refusal, Registry, Store},
    session,
};
use std::{
    fs,
    os::unix::process::CommandExt,
    path::Path,
    process::{Command, Stdio},
};

type Result<T> = std::result::Result<T, Refusal>;
fn call(root: &Path, tool: &str, verb: &str) -> Result<String> {
    crate::bin::execute(&[
        verb.into(),
        "--tool".into(),
        tool.into(),
        "--repo".into(),
        root.display().to_string(),
    ])
}
fn each_tool(root: &Path, long: bool) -> Result<()> {
    let registry = Registry::discover()?;
    let manifest = super::declaration::store(None, None, None)
        .and_then(|s| s.read().map_err(|e| e.to_string()))
        .map_err(|detail| Refusal {
            kind: "manifest-read".into(),
            detail,
        })?;
    let skipped = std::env::var("BIN_STORE_SKIP_TOOLS").unwrap_or_default();
    for tool in registry
        .tools
        .keys()
        .filter(|tool| !skipped.split(',').any(|s| s == *tool))
    {
        let Some(follow) = manifest
            .follows
            .iter()
            .find(|f| f.channel == format!("bin:{tool}"))
        else {
            continue;
        };
        let result = (|| {
            let store = Store::from_environment(tool, &registry)?;
            if follow.mode == ark_core::declaration::FollowMode::Observe {
                if long {
                    store.follow_declared(root, None, follow)?;
                }
                return Ok(());
            }
            if !long
                && !store
                    .status(&session::resolve(), None, false, &|k| std::env::var(k).ok())?
                    .verified_current
            {
                return Ok(());
            }
            if follow.pin.is_some() {
                if long {
                    store.follow_declared(root, None, follow)?;
                }
                // A pinned apply/canary must install the launcher from the matching landed
                // release; bootstrap would be an unconstrained adoption before that check.
                call(root, tool, "install-launcher")?;
                store.status(&session::resolve(), None, true, &|k| std::env::var(k).ok())?;
                return Ok(());
            }
            if !maintain(long, &mut |verb| call(root, tool, verb), &mut |line| {
                eprintln!("{tool} arrival: {line}")
            })? {
                return Ok(());
            }
            store.status(&session::resolve(), None, true, &|k| std::env::var(k).ok())?;
            Ok::<_, Refusal>(())
        })();
        if let Err(error) = result {
            eprintln!("{tool} arrival (continuing): {error}");
        }
    }
    Ok(())
}
/// Heal already available heads before returning; queue builds without holding the hook's pipes.
pub(super) fn start(root: &Path) -> Result<()> {
    super::declaration::apply_at_arrival().map_err(|detail| Refusal {
        kind: "manifest-apply".into(),
        detail,
    })?;
    each_tool(root, false)?;
    let dir = worker_dir()?;
    fs::create_dir_all(&dir)?;
    let log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("arrival.log"))?;
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args(["device", "arrive", "--worker", "--root"])
        .arg(root)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log)
        .process_group(0);
    // Carry the original session's identity; a detached worker must not acquire a new TTY pin.
    let identity = session::resolve();
    if let Some(session) = identity.session {
        command.env("ELOHIM_SESSION_ID", session);
    }
    if let Some(owner) = identity.owner_pid {
        command.env("ELOHIM_SESSION_OWNER_PID", owner.to_string());
    }
    command.spawn()?;
    Ok(())
}
fn worker_dir() -> Result<std::path::PathBuf> {
    let key = crate::device_key::resolve_path()?;
    key.parent()
        .map(|p| p.join("arrival"))
        .ok_or_else(|| Refusal {
            kind: "arrival-path".into(),
            detail: "device key has no parent directory".into(),
        })
}
/// Serialize long work across runtimes on this device; kernel lock release survives worker death.
pub(super) fn work(root: &Path) -> Result<()> {
    let dir = worker_dir()?;
    fs::create_dir_all(&dir)?;
    let lock = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("worker.lock"))?;
    if rustix::fs::flock(&lock, rustix::fs::FlockOperation::NonBlockingLockExclusive).is_err() {
        eprintln!("device arrival: another worker is already following tools");
        return Ok(());
    }
    each_tool(root, true)
}

fn maintain(
    long: bool,
    run: &mut impl FnMut(&str) -> Result<String>,
    say: &mut impl FnMut(String),
) -> Result<bool> {
    if long {
        if let Err(error) = run("bootstrap") {
            say(format!(
                "no verified head; launcher left untouched: {error}"
            ));
            return Ok(false);
        }
        if let Err(error) = run("follow") {
            say(format!("could not follow; landed head stays: {error}"));
        }
    }
    run("install-launcher")?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn exercise(fail: &str) -> (Vec<String>, bool, Vec<String>) {
        let mut calls = Vec::new();
        let mut lines = Vec::new();
        let result = maintain(
            true,
            &mut |verb| {
                calls.push(verb.to_string());
                if verb == fail {
                    Err(Refusal {
                        kind: "fixture".into(),
                        detail: "unavailable".into(),
                    })
                } else {
                    Ok(String::new())
                }
            },
            &mut |line| lines.push(line),
        );
        (calls, result.unwrap_or(false), lines)
    }
    #[test]
    fn bootstrap_refusal_never_installs() {
        let (calls, installed, lines) = exercise("bootstrap");
        assert_eq!(calls, ["bootstrap"]);
        assert!(!installed);
        assert!(lines[0].contains("launcher left untouched"));
    }
    #[test]
    fn follow_failure_keeps_installation_and_all_steps_remain_native() {
        let (calls, installed, lines) = exercise("follow");
        assert_eq!(calls, ["bootstrap", "follow", "install-launcher"]);
        assert!(installed);
        assert!(lines[0].contains("landed head stays"));
    }
    #[test]
    fn installation_failure_is_reported_without_claiming_success() {
        assert!(!exercise("install-launcher").1);
    }
}
