//! Thin command adapter for the device-runtime library.
use elohim_device_runtime::{
    bin_store::{self, IngestOptions, PeerServing, Refusal, Registry, Store, Transition},
    manifest_store::ManifestStore,
    session,
};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

fn argument(detail: impl Into<String>) -> Refusal {
    Refusal {
        kind: "arguments".into(),
        detail: detail.into(),
    }
}
fn land_transition(
    flags: &BTreeSet<&str>,
    options: &BTreeMap<&str, &str>,
    expected: &str,
) -> Result<Transition, Refusal> {
    let rebase = flags.contains("--rebase-lineage");
    let revert = flags.contains("--revert");
    if rebase && revert {
        return Err(argument(
            "--rebase-lineage and --revert are mutually exclusive",
        ));
    }
    if rebase {
        if expected == "none" {
            return Err(argument(
                "--rebase-lineage requires --expect <current-cid>, not none",
            ));
        }
        let reason = options
            .get("--reason")
            .filter(|reason| !reason.trim().is_empty())
            .ok_or_else(|| argument("--rebase-lineage requires --reason <text>"))?;
        return Ok(Transition::RebaseLineage {
            reason: (*reason).to_owned(),
        });
    }
    Ok(if revert {
        Transition::Revert
    } else {
        Transition::Forward
    })
}
/// Parse, call the device library, and render one bin command. Returns a process exit code.
pub fn run(args: &[String]) -> i32 {
    match execute(args) {
        Ok(text) => {
            println!("{text}");
            0
        }
        Err(e) => {
            eprintln!("{e}");
            2
        }
    }
}
pub(crate) fn execute(args: &[String]) -> Result<String, Refusal> {
    let mut options = BTreeMap::new();
    let mut flags = BTreeSet::new();
    let mut positional = Vec::new();
    let mut rest = Vec::new();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--" {
            rest.extend(iter.cloned());
            break;
        }
        match arg.as_str() {
            "--repair"|"--json"|"--revert"|"--rebase-lineage"|"--current"|"--pin"|"--provisional"=>{ flags.insert(arg.as_str()); },
            "--tool"|"--registry"|"--store"|"--repo"|"--source-commit"|"--gate-run"|"--tree-clean"|"--commit"|"--expect"|"--reason"|"--invoked"|"--cid"|"--closure"|"--launcher"|"--destination"=>{ options.insert(arg.as_str(),iter.next().ok_or_else(|| argument(format!("{arg} needs a value")))?.as_str()); },
            "--help"|"-h"=>return Ok("epr bin <ingest|land|pin|resolve|status|gc|bootstrap|follow|shadows|source-state|launch|install-launcher|migrate> --tool <name> [--registry PATH] [--store PATH] [--json]; land requires --expect <cid|none> [--revert | --rebase-lineage --reason <text>]".into()),
            _ if arg.starts_with('-')=>return Err(argument(format!("unknown flag {arg}"))),
            _=>positional.push(arg.as_str()),
        }
    }
    let verb = *positional
        .first()
        .ok_or_else(|| argument("missing bin verb"))?;
    let registry = if let Some(path) = options.get("--registry") {
        Registry::load(Path::new(path))?
    } else {
        Registry::discover()?
    };
    if verb == "launch" {
        bin_store::launch(
            &registry,
            Path::new(
                options
                    .get("--invoked")
                    .ok_or_else(|| argument("launch requires --invoked"))?,
            ),
            &rest,
        )?;
        return Ok(String::new());
    }
    let tool = options
        .get("--tool")
        .ok_or_else(|| argument("--tool is required"))?;
    let store = if let Some(path) = options.get("--store") {
        Store::new(path, tool, &registry)?
    } else {
        Store::from_environment(tool, &registry)?
    };
    let store = store.with_runner(std::sync::Arc::new(crate::git::DeviceRunner));
    // Only the verbs that land read the device manifest for the landing policy.
    let store = if matches!(verb, "land" | "follow" | "bootstrap") {
        store.with_peer_serving(peer_serving_policy(
            crate::device::declaration::store(None, None, None),
            tool,
        ))
    } else {
        store
    };
    let repo = || store.repo_root(options.get("--repo").map(Path::new));
    let identity = session::resolve();
    let closure = options.get("--closure").map(Path::new);
    let value: Value = match verb {
        "ingest" => {
            let source = closure
                .or_else(|| positional.get(1).map(Path::new))
                .ok_or_else(|| argument("ingest requires binary or --closure PATH"))?;
            let clean = options
                .get("--tree-clean")
                .map(|v| match v.to_ascii_lowercase().as_str() {
                    "true" | "1" | "yes" => Ok(true),
                    "false" | "0" | "no" => Ok(false),
                    _ => Err(argument("--tree-clean expects true or false")),
                })
                .transpose()?;
            json!(store.ingest(
                source,
                &IngestOptions {
                    repo: options.get("--repo").map(PathBuf::from),
                    source_commit: options.get("--source-commit").map(|s| s.to_string()),
                    gate_run: options.get("--gate-run").map(|s| s.to_string()),
                    tree_clean: clean,
                    closure: closure.is_some()
                }
            )?)
        }
        "land" => {
            let expected = options
                .get("--expect")
                .copied()
                .ok_or_else(|| argument("land requires --expect <cid|none>"))?;
            let transition = land_transition(&flags, &options, expected)?;
            json!(store.land_expected(
                positional
                    .get(1)
                    .or_else(|| options.get("--cid"))
                    .ok_or_else(|| argument("land requires cid"))?,
                options
                    .get("--commit")
                    .ok_or_else(|| argument("land requires --commit"))?,
                &repo()?,
                transition,
                (expected != "none").then_some(expected)
            )?)
        }
        "pin" => json!(store.pin(
            &identity,
            options.get("--cid").copied(),
            flags.contains("--provisional"),
            options.get("--repo").map(Path::new)
        )?),
        "resolve" => {
            let result = if flags.contains("--current") {
                store.resolve_current(None)?
            } else {
                store.resolve(None, Some(&identity), &|k| std::env::var(k).ok(), false)?
            };
            if !flags.contains("--json") {
                return Ok(result.path.display().to_string());
            }
            json!(result)
        }
        "status" => {
            if flags.contains("--repair") {
                store.repair_heads()?;
            }
            let status = store.status(
                &identity,
                options.get("--commit").copied(),
                flags.contains("--pin"),
                &|k| std::env::var(k).ok(),
            )?;
            if !flags.contains("--json") {
                return Ok(format!(
                    "session: {} cid={}\nhead: {}\nstate: {}\nresolved: {}",
                    status.session.as_deref().unwrap_or("none"),
                    status.session_cid.as_deref().unwrap_or("none"),
                    status.head.as_deref().unwrap_or("none"),
                    if status.behind {
                        "behind"
                    } else {
                        &status.source
                    },
                    status
                        .resolved
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "none".into())
                ));
            }
            json!(status)
        }
        "gc" => store.gc(&session::proc_root(), None)?,
        "shadows" => json!(store.shadows(&std::env::var("PATH").unwrap_or_default())),
        "source-state" => store.source_state(&repo()?)?,
        "bootstrap" => json!(store.bootstrap(&repo()?, closure)?),
        "follow" => {
            let head = crate::device::declaration::store(None, None, None)
                .map_err(argument)?
                .read()?;
            match head
                .follows
                .iter()
                .find(|f| f.channel == format!("bin:{tool}"))
            {
                Some(follow) => store.follow_declared(&repo()?, closure, follow)?,
                None => {
                    serde_json::json!({"followed":false,"why":"device does not follow this tool"})
                }
            }
        }
        "migrate" => store.migrate()?,
        "install-launcher" => {
            if !options.contains_key("--registry") {
                let bins = store
                    .root()
                    .parent()
                    .map(|root| root.join("bin/bin-tools.json"));
                crate::device::declaration::store_with_bins(None, None, None, bins)
                    .map_err(argument)?
                    .apply()?;
            }
            let native = match options.get("--launcher") {
                Some(path) => PathBuf::from(path),
                None => store.landed_launcher()?,
            };
            let destination = options
                .get("--destination")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    PathBuf::from(std::env::var("CARGO_HOME").unwrap_or_else(|_| {
                        format!("{}/.cargo", std::env::var("HOME").unwrap_or_default())
                    }))
                    .join("bin")
                });
            store.install_launcher(&native, &destination, &registry, &repo()?)?
        }
        _ => return Err(argument(format!("unknown bin verb {verb}"))),
    };
    serde_json::to_string(&value).map_err(|e| argument(e.to_string()))
}

/// Whether `manifest` declares a supervised `peer serve` running `tool`'s landed release: a
/// process on the tool's channel whose arguments begin `peer serve` (not `--check`).
pub(crate) fn declares_peer_serve(manifest: &ark_core::RuntimeManifest, tool: &str) -> bool {
    let channel = format!("bin:{tool}");
    manifest.processes.iter().any(|process| {
        matches!(&process.artifact, ark_core::ArtifactRef::Channel { channel_id } if *channel_id == channel)
            && process.argv.get(1..3) == Some(&["peer".to_owned(), "serve".to_owned()][..])
            && !process.argv.iter().any(|arg| arg == "--check")
    })
}

/// The landing policy the device's current manifest head sets for `tool`. An unreadable head
/// keeps the tolerance and says so: landing never fails on the manifest it cannot read.
pub(crate) fn peer_serving_policy(store: Result<ManifestStore, String>, tool: &str) -> PeerServing {
    match store.and_then(|store| store.read().map_err(|error| error.to_string())) {
        Ok(head) if declares_peer_serve(&head, tool) => PeerServing::Required,
        Ok(_) => PeerServing::Tolerated,
        Err(detail) => {
            eprintln!(
                "{tool} landing: device manifest unreadable ({detail}); a candidate whose peer \
                 check is unsupported or finds no device still lands with a note"
            );
            PeerServing::Tolerated
        }
    }
}

/// Open the epr custody store shared by daemon and device status.
pub(crate) fn daemon_store(
) -> Result<elohim_device_runtime::bin_store::Store, elohim_device_runtime::bin_store::Refusal> {
    elohim_device_runtime::bin_store::Store::from_environment(
        "epr",
        &elohim_device_runtime::bin_store::Registry::discover()?,
    )
}

/// Process table used to observe daemon owner lifetimes.
pub(crate) fn daemon_proc_root() -> PathBuf {
    session::proc_root()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags(values: &[&'static str]) -> BTreeSet<&'static str> {
        values.iter().copied().collect()
    }

    fn options(values: &[(&'static str, &'static str)]) -> BTreeMap<&'static str, &'static str> {
        values.iter().copied().collect()
    }

    #[test]
    fn land_arguments_refuse_rebase_lineage_with_revert() {
        let error = land_transition(
            &flags(&["--rebase-lineage", "--revert"]),
            &options(&[("--reason", "history was rewritten")]),
            "bafk-current",
        )
        .unwrap_err();

        assert_eq!(error.kind, "arguments");
        assert!(error.detail.contains("mutually exclusive"));
    }

    #[test]
    fn land_arguments_refuse_rebase_lineage_without_prior() {
        let error = land_transition(
            &flags(&["--rebase-lineage"]),
            &options(&[("--reason", "history was rewritten")]),
            "none",
        )
        .unwrap_err();

        assert_eq!(error.kind, "arguments");
        assert!(error.detail.contains("--expect"));
    }

    #[test]
    fn land_arguments_refuse_rebase_lineage_without_reason() {
        let error = land_transition(&flags(&["--rebase-lineage"]), &options(&[]), "bafk-current")
            .unwrap_err();

        assert_eq!(error.kind, "arguments");
        assert!(error.detail.contains("--reason"));
    }

    #[test]
    fn land_arguments_build_rebase_lineage_transition() {
        assert_eq!(
            land_transition(
                &flags(&["--rebase-lineage"]),
                &options(&[("--reason", "history was rewritten")]),
                "bafk-current",
            )
            .unwrap(),
            Transition::RebaseLineage {
                reason: "history was rewritten".into()
            }
        );
    }

    fn device_manifest(root: &Path) -> ManifestStore {
        crate::device::declaration::store(Some(root.to_path_buf()), None, None).unwrap()
    }

    fn run_epr(store: &ManifestStore, name: &str, args: &[&str]) {
        store
            .act(ark_core::declaration::ManifestAct::Run {
                tool: "epr".into(),
                stop: false,
                launch: ark_core::declaration::RunLaunch {
                    name: Some(name.into()),
                    args: args.iter().map(|arg| (*arg).to_owned()).collect(),
                    ..Default::default()
                },
            })
            .unwrap();
    }

    #[test]
    fn landing_requires_peer_serving_only_when_the_head_declares_a_peer_process() {
        let tmp = tempfile::tempdir().unwrap();
        let store = device_manifest(tmp.path());
        assert_eq!(
            peer_serving_policy(Ok(device_manifest(tmp.path())), "epr"),
            PeerServing::Tolerated,
            "a head with no processes keeps the tolerance"
        );

        run_epr(&store, "flow-report", &["flow", "report", "scope"]);
        assert_eq!(
            peer_serving_policy(Ok(device_manifest(tmp.path())), "epr"),
            PeerServing::Tolerated,
            "an epr process that does not serve keeps the tolerance"
        );

        run_epr(&store, "peer", &["peer", "serve", "--listen", "0.0.0.0:0"]);
        assert_eq!(
            peer_serving_policy(Ok(device_manifest(tmp.path())), "epr"),
            PeerServing::Required
        );
        // The policy is per tool: brit's landing does not judge epr's peer process.
        assert_eq!(
            peer_serving_policy(Ok(device_manifest(tmp.path())), "brit"),
            PeerServing::Tolerated
        );
    }

    #[test]
    fn an_unreadable_manifest_keeps_the_landing_tolerance() {
        let tmp = tempfile::tempdir().unwrap();
        let store = device_manifest(tmp.path());
        run_epr(&store, "peer", &["peer", "serve"]);
        std::fs::write(tmp.path().join("manifest/head"), b"not a head").unwrap();

        assert_eq!(
            peer_serving_policy(
                crate::device::declaration::store(Some(tmp.path().to_path_buf()), None, None),
                "epr"
            ),
            PeerServing::Tolerated
        );
        assert_eq!(
            peer_serving_policy(Err("no device key".into()), "epr"),
            PeerServing::Tolerated
        );
    }
}
