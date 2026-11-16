//! Thin adapters for the device-private manifest chain.
use ark_core::declaration::{self, FollowMode, ManifestAct, RunLaunch};
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
    let mut mode_given = false;
    let mut pin = None;
    let mut stop = false;
    let mut unfollow = false;
    let mut render = None;
    let mut launch = RunLaunch::default();
    let mut json = false;
    let mut preview = false;
    let mut replace_processes = false;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            // Everything after `--` is the tool's own argv, appended after `{artifact}`.
            "--" if verb == "run" => {
                launch.args = args[i + 1..].to_vec();
                break;
            }
            "--stop" => stop = true,
            "--unfollow" => unfollow = true,
            "--observed" => observed = true,
            "--json" => json = true,
            "--preview" => preview = true,
            "--replace-processes" => replace_processes = true,
            "--channels" | "--root" | "--runtime-config" | "--base" | "--mode" | "--pin"
            | "--render" | "--ready-line" | "--restart" | "--name" | "--env" => {
                let flag = &args[i];
                i += 1;
                let value = args.get(i).ok_or_else(|| format!("{flag} needs a value"))?;
                match flag.as_str() {
                    "--channels" => channels = Some(value.clone()),
                    "--root" => root = Some(PathBuf::from(value)),
                    "--runtime-config" => runtime = Some(PathBuf::from(value)),
                    "--base" => base = Some(PathBuf::from(value)),
                    "--mode" => {
                        mode = FollowMode::parse(value)?;
                        mode_given = true;
                    }
                    "--pin" => pin = Some(value.clone()),
                    "--render" => render = Some(value.as_str()),
                    "--ready-line" => launch.ready_line = Some(value.clone()),
                    "--restart" => launch.restart = Some(declaration::parse_restart(value)?),
                    "--name" if launch.name.is_some() => {
                        return Err("--name is declared twice".into())
                    }
                    "--name" => launch.name = Some(value.clone()),
                    "--env" => {
                        let (key, val) = value.split_once('=').ok_or("--env needs KEY=VALUE")?;
                        if launch.env.insert(key.to_owned(), val.to_owned()).is_some() {
                            return Err(format!("--env {key} is declared twice"));
                        }
                    }
                    _ => unreachable!(),
                }
            }
            value if !value.starts_with('-') && name.is_none() => name = Some(value.to_owned()),
            value => return Err(format!("unexpected argument {value}")),
        }
        i += 1;
    }
    if verb != "run" && launch != RunLaunch::default() {
        return Err("--name, --env, --ready-line, --restart and `-- <args>` belong to run".into());
    }
    let act_flags = stop || unfollow || channels.is_some() || mode_given || pin.is_some();
    if preview && (verb != "apply" || observed || render.is_some() || act_flags) {
        return Err(
            "--preview belongs to apply, without --observed, --render, --stop, \
                    --unfollow, --channels, --mode or --pin"
                .into(),
        );
    }
    if replace_processes && !matches!(verb.as_str(), "follow" | "run" | "apply") {
        return Err("--replace-processes belongs to follow, run and apply".into());
    }
    if render.is_some_and(|r| !matches!(r, "runtime-config" | "bin-tools" | "manifest")) {
        return Err("render must be runtime-config, bin-tools or manifest".into());
    }
    let observed_bins = root.clone();
    let store = store(root, runtime, base)?.replace_processes(replace_processes);
    if preview {
        let preview = store.preview().map_err(|e| e.to_string())?;
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&preview).map_err(|e| e.to_string())?
            );
        } else {
            print!("{}", render_preview(&preview));
        }
        // A dry run that apply would refuse still shows what refuses, then fails.
        return Ok(if preview.conflicts.is_empty() {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        });
    }
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
            launch,
        }),
        "show" => store.read(),
        "apply" => store.apply_carrying().map(|applied| {
            if !applied.carried.is_empty() {
                eprintln!(
                    "carried (device-owned, not in base): {}",
                    applied.carried.join(", ")
                );
            }
            applied.manifest
        }),
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

/// Human reading of an apply preview; `--json` prints the structured preview instead.
fn render_preview(preview: &manifest_store::ManifestPreview) -> String {
    let unchanged = if preview.changed { "" } else { " (unchanged)" };
    let mut lines = vec![
        "epr device apply --preview (nothing written)".to_owned(),
        format!("  current  {}", preview.current_cid),
        format!("  next     {}{unchanged}", preview.next_cid),
    ];
    for (label, diff) in [
        ("follows", &preview.follows),
        ("tools", &preview.tools),
        ("processes", &preview.processes),
    ] {
        let items: Vec<String> = diff
            .added
            .iter()
            .map(|name| format!("  + {name}"))
            .chain(diff.removed.iter().map(|name| format!("  - {name}")))
            .chain(
                diff.changed
                    .iter()
                    .map(|c| format!("  ~ {} ({})", c.name, c.fields.join(", "))),
            )
            .collect();
        if items.is_empty() {
            lines.push(format!("{label}: unchanged"));
        } else {
            lines.push(format!("{label}:"));
            lines.extend(items);
        }
    }
    if !preview.carried.is_empty() {
        lines.push(format!(
            "carried (device-owned, not in base): {}",
            preview.carried.join(", ")
        ));
    }
    if !preview.conflicts.is_empty() {
        lines.push(format!(
            "conflicts (the base declares these differently; apply refuses): {}",
            preview.conflicts.join(", ")
        ));
        lines.push(
            "  cure: rerun with --replace-processes to accept the base's processes, or \
             re-declare them after apply with `epr device run`"
                .to_owned(),
        );
    }
    lines.join("\n") + "\n"
}
