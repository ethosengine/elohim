//! Thin adapters for the device-private manifest chain.
use ark_core::declaration::{self, FollowMode, ManifestAct};
use elohim_device_runtime::manifest_store::{self, ManifestStore, Projections};
use std::{path::PathBuf, process::ExitCode};

pub(crate) fn store(
    root: Option<PathBuf>,
    runtime: Option<PathBuf>,
    base: Option<PathBuf>,
) -> Result<ManifestStore, String> {
    store_with_bins(root, runtime, base, None)
}
pub(crate) fn store_with_bins(
    root: Option<PathBuf>,
    runtime: Option<PathBuf>,
    base: Option<PathBuf>,
    bins_override: Option<PathBuf>,
) -> Result<ManifestStore, String> {
    let explicit_root = root.is_some();
    let root = match root {
        Some(p) => p,
        None => crate::device_key::resolve_path()
            .map_err(|e| e.to_string())?
            .parent()
            .ok_or("device key has no parent")?
            .to_owned(),
    };
    let bins = if explicit_root {
        root.join("bin/bin-tools.json")
    } else {
        let bin_root = std::env::var_os("BIN_STORE")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/projects/.claude-config/tools/bin-store"));
        bin_root.join("bin/bin-tools.json")
    };
    let runtime_config = runtime
        .or_else(|| {
            (!explicit_root)
                .then(|| {
                    std::env::var_os("ELOHIM_RUNTIME_CONFIG_PATH")
                        .filter(|value| !value.is_empty())
                        .map(PathBuf::from)
                })
                .flatten()
        })
        .or_else(|| Some(root.join("runtime-config.toml")));
    let base = base.or_else(|| {
        (!explicit_root)
            .then(|| {
                std::env::var_os("ELOHIM_RUNTIME_MANIFEST_PATH")
                    .filter(|value| !value.is_empty())
                    .map(PathBuf::from)
            })
            .flatten()
    });
    ManifestStore::from_source(
        root,
        base.as_deref(),
        Projections {
            runtime_config,
            bin_tools: Some(bins_override.unwrap_or(bins)),
        },
    )
    .map_err(|e| e.to_string())
}

pub(super) fn apply_at_arrival() -> Result<(), String> {
    store(None, None, None)?
        .apply()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub(super) fn run(args: &[String]) -> Result<ExitCode, String> {
    let verb = args.first().ok_or("expected follow, run, show or apply")?;
    let mut root = None;
    let mut runtime = None;
    let mut base = None;
    let mut name = None;
    let mut observed = false;
    let mut channels = None;
    let mut mode = FollowMode::Observe;
    let mut pin = None;
    let mut stop = false;
    let mut unfollow = false;
    let mut render = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--stop" => stop = true,
            "--unfollow" => unfollow = true,
            "--observed" => observed = true,
            "--json" => {}
            "--channels" | "--root" | "--runtime-config" | "--base" | "--mode" | "--pin"
            | "--render" => {
                let flag = &args[i];
                i += 1;
                let value = args.get(i).ok_or_else(|| format!("{flag} needs a value"))?;
                match flag.as_str() {
                    "--channels" => channels = Some(value.clone()),
                    "--root" => root = Some(PathBuf::from(value)),
                    "--runtime-config" => runtime = Some(PathBuf::from(value)),
                    "--base" => base = Some(PathBuf::from(value)),
                    "--mode" => mode = FollowMode::parse(value)?,
                    "--pin" => pin = Some(value.clone()),
                    "--render" => render = Some(value.as_str()),
                    _ => unreachable!(),
                }
            }
            value if !value.starts_with('-') && name.is_none() => name = Some(value.to_owned()),
            value => return Err(format!("unexpected argument {value}")),
        }
        i += 1;
    }
    if render.is_some_and(|r| !matches!(r, "runtime-config" | "bin-tools" | "manifest")) {
        return Err("render must be runtime-config, bin-tools or manifest".into());
    }
    let observed_bins = root.clone();
    let store = store(root, runtime, base)?;
    let manifest = match verb.as_str() {
        "follow" if channels.is_some() => {
            store.follow_list(channels.as_deref().unwrap_or_default())
        }
        "follow" => store.act(ManifestAct::Follow {
            channel: name.ok_or("follow needs a channel")?,
            mode: if unfollow { None } else { Some(mode) },
            pin,
        }),
        "run" => store.act(ManifestAct::Run {
            tool: name.ok_or("run needs a tool")?,
            stop,
        }),
        "show" => store.read(),
        "apply" => store.apply(),
        _ => return Err("expected follow, run, show or apply".into()),
    }
    .map_err(|e| e.to_string())?;
    if observed {
        let observation = match observed_bins {
            Some(bins) => manifest_store::observe(&manifest, &bins, &Default::default()),
            None => manifest_store::observe_local(&manifest),
        }
        .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&observation).map_err(|e| e.to_string())?
        );
        return Ok(ExitCode::SUCCESS);
    }
    let output = match render {
        Some("runtime-config") => declaration::render_runtime_config(&manifest),
        Some("bin-tools") => declaration::render_bin_tools(&manifest).map_err(|e| e.to_string())?,
        Some("manifest") => declaration::render_manifest(&manifest).map_err(|e| e.to_string())?,
        Some(other) => return Err(format!("unknown render {other}")),
        None => serde_json::to_string_pretty(
            &manifest_store::view(&manifest).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?,
    };
    println!("{output}");
    Ok(ExitCode::SUCCESS)
}
