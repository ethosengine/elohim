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
        elohim_device_runtime::layout::resolve()
            .and_then(|layout| {
                layout
                    .bin_store()
                    .map(|root| root.join("bin/bin-tools.json"))
            })
            .map_err(|e| e.to_string())?
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
        "run" => {
            let tool = name.ok_or("run needs a tool")?;
            if !stop {
                // A run act always declares a `bin:<tool>` channel; refuse before it is written.
                let head = store.read().map_err(|e| e.to_string())?;
                let registry = manifest_store::registry(&head).map_err(|e| e.to_string())?;
                let process = launch.name.as_deref().unwrap_or(&tool);
                refuse_launcher_env(process, &tool, launch.env.keys(), &registry, false)?;
                let carry = carry_device_dirs(&mut launch.env, |key| std::env::var(key).ok());
                if !carry.carried.is_empty() {
                    eprintln!(
                        "carried from this shell into process {process}'s declared env: {} \
                         (ark clears every other variable; declare --env KEY=VALUE to choose \
                         another value)",
                        carry.carried.join(", ")
                    );
                }
                if !carry.braced.is_empty() {
                    eprintln!(
                        "not carried into process {process}: {} — the shell's value holds a \
                         brace, which a declared env value reads as a placeholder; declare it \
                         with --env KEY=VALUE",
                        carry.braced.join(", ")
                    );
                }
            }
            store.act(ManifestAct::Run { tool, stop, launch })
        }
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

/// The directories a `bin:<tool>` process finds this device by: its key and roster live under
/// `XDG_CONFIG_HOME`, its stores under the data, cache and state homes. Ark starts a child with
/// a cleared environment, so a peer serve declared without them dies "this device has no device
/// key" and restarts in a loop.
const DEVICE_DIR_KEYS: [&str; 5] = [
    "HOME",
    "XDG_CONFIG_HOME",
    "XDG_DATA_HOME",
    "XDG_CACHE_HOME",
    "XDG_STATE_HOME",
];

/// What [`carry_device_dirs`] did with each key the run left undeclared.
struct Carry {
    /// Declared from the shell, in [`DEVICE_DIR_KEYS`] order.
    carried: Vec<&'static str>,
    /// Left undeclared because the shell's value holds `{` or `}`, which ark reads as a
    /// placeholder; carrying it would refuse a run the steward never typed that value into.
    braced: Vec<&'static str>,
}

/// Declare each [`DEVICE_DIR_KEYS`] entry the run did not declare itself, from `lookup` (the
/// invoking shell). An explicit `--env` always wins; an unset or empty key stays undeclared.
fn carry_device_dirs(
    env: &mut std::collections::BTreeMap<String, String>,
    lookup: impl Fn(&str) -> Option<String>,
) -> Carry {
    let mut carry = Carry {
        carried: Vec::new(),
        braced: Vec::new(),
    };
    for key in DEVICE_DIR_KEYS {
        if env.contains_key(key) {
            continue;
        }
        match lookup(key).filter(|v| !v.is_empty()) {
            Some(value) if value.contains(['{', '}']) => carry.braced.push(key),
            Some(value) => {
                env.insert(key.to_owned(), value);
                carry.carried.push(key);
            }
            None => {}
        }
    }
    carry
}

/// The keys that steer the bin launcher to a raw candidate instead of the verified current:
/// `EPR_BIN` always, and every `candidateEnv` key the device registry declares for any tool.
fn launcher_env_keys(registry: &elohim_device_runtime::bin_store::Registry) -> Vec<String> {
    let mut keys = vec!["EPR_BIN".to_owned()];
    for tool in registry.tools.values() {
        let declared = tool["candidateEnv"]
            .as_object()
            .into_iter()
            .flat_map(|map| {
                map.values()
                    .filter_map(|v| v.as_array())
                    .flatten()
                    .filter_map(|v| v.as_str())
            });
        for key in declared {
            if !keys.iter().any(|k| k == key) {
                keys.push(key.to_owned());
            }
        }
    }
    keys
}

/// Refuse a launcher candidate key in the declared environment of a process whose artifact is
/// a `bin:<tool>` channel. A child started with one executes that raw candidate and reports its
/// file CID, not the release CID, so the device reads it as behind current. The key is named,
/// the value never. `at_supervise` words the refusal for a manifest that already carries it.
pub(super) fn refuse_launcher_env<'a>(
    process: &str,
    tool: &str,
    env_keys: impl IntoIterator<Item = &'a String>,
    registry: &elohim_device_runtime::bin_store::Registry,
    at_supervise: bool,
) -> Result<(), String> {
    let keys = launcher_env_keys(registry);
    let Some(key) = env_keys
        .into_iter()
        .find(|key| keys.iter().any(|k| k == *key))
    else {
        return Ok(());
    };
    Err(format!(
        "REFUSED launcher-env-declared: process {process} runs bin:{tool} and declares {key}, \
         a launcher candidate key; the child would execute that raw candidate and report its \
         file CID instead of the release CID, so the device reads it as behind current. \
         Launcher candidate keys ({}) are refused for every process whose artifact is a \
         bin:<tool> channel{}",
        keys.join(", "),
        if at_supervise {
            format!(
                "; this manifest already declares it, so nothing starts — redeclare it with \
                 `epr device run {tool} --name {process}` without --env {key}"
            )
        } else {
            String::new()
        }
    ))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn a_run_carries_the_device_dirs_it_did_not_declare_and_never_overrides_one() {
        let mut env = BTreeMap::from([("XDG_CONFIG_HOME".to_owned(), "/chosen".to_owned())]);
        let shell = |key: &str| match key {
            "HOME" => Some("/home/user".to_owned()),
            "XDG_CONFIG_HOME" => Some("/shell-config".to_owned()),
            "XDG_DATA_HOME" => Some("/nix/xdg/data".to_owned()),
            "XDG_CACHE_HOME" => Some(String::new()),
            "XDG_STATE_HOME" => Some("/odd/{dir}".to_owned()),
            _ => None,
        };
        let carry = carry_device_dirs(&mut env, shell);
        assert_eq!(carry.carried, ["HOME", "XDG_DATA_HOME"]);
        assert_eq!(
            carry.braced,
            ["XDG_STATE_HOME"],
            "a braced value is named, not carried"
        );
        assert_eq!(env["XDG_CONFIG_HOME"], "/chosen", "an explicit --env wins");
        assert_eq!(env["HOME"], "/home/user");
        assert!(
            !env.contains_key("XDG_CACHE_HOME"),
            "an empty value is not carried"
        );
        assert!(!env.contains_key("XDG_STATE_HOME"));
    }
}
