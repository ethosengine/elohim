//! One effective Cargo/rustc linker selection, shared by key probes and put authorization.

use std::collections::BTreeMap;

use super::{ArtifactError, ArtifactResult, MethodDescriptor};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Selections {
    pub driver: String,
    pub external_linker: Option<String>,
    pub bundled_lld_authorized: bool,
}

fn unsupported(flag: &str) -> ArtifactError {
    ArtifactError::Gap(format!(
        "{flag}: the key does not model that linker selection yet"
    ))
}

pub(crate) fn target(descriptor: &MethodDescriptor) -> &str {
    let mut target = None;
    let mut flags = descriptor.flags.iter();
    while let Some(flag) = flags.next() {
        if flag == "--target" {
            target = flags.next().map(String::as_str);
        } else if let Some(value) = flag.strip_prefix("--target=") {
            target = Some(value);
        }
    }
    target
        .or_else(|| {
            descriptor
                .tools
                .get("rustc")
                .and_then(|rustc| rustc.lines().find_map(|line| line.strip_prefix("host: ")))
        })
        // Old descriptors with no target-specific inputs can still use the default driver.
        .unwrap_or("")
}

fn words(value: &str) -> Vec<String> {
    value.split_whitespace().map(str::to_owned).collect()
}

fn config_flags(value: &toml::Value) -> ArtifactResult<Vec<String>> {
    match value {
        toml::Value::String(value) => Ok(words(value)),
        toml::Value::Array(values) => values
            .iter()
            .map(|value| {
                value.as_str().map(str::to_owned).ok_or_else(|| {
                    ArtifactError::Gap("cargo rustflags array contains a non-string".into())
                })
            })
            .collect(),
        _ => Err(ArtifactError::Gap(
            "cargo rustflags is not a string or array".into(),
        )),
    }
}

/// Resolve once, removing only inactive selection inputs from a *copy* of the descriptor's
/// environment. Otherwise those ignored values still perturb the method CID even without probes.
pub(crate) fn resolve(
    inputs: &mut BTreeMap<String, String>,
    config: &toml::Value,
    target: &str,
) -> ArtifactResult<Selections> {
    let prefix = format!("CARGO_TARGET_{}_", target.to_uppercase().replace('-', "_"));
    let target_flags = format!("{prefix}RUSTFLAGS");
    let target_linker = format!("{prefix}LINKER");
    if target.is_empty()
        && (inputs.keys().any(|key| key.starts_with("CARGO_TARGET_"))
            || config.get("target").is_some())
    {
        return Err(ArtifactError::Gap(
            "linker selection needs a recorded --target or rustc host triple".into(),
        ));
    }
    inputs.retain(|name, _| {
        !name.starts_with("CARGO_TARGET_")
            || !(name.ends_with("_RUSTFLAGS") || name.ends_with("_LINKER"))
            || name == &target_flags
            || name == &target_linker
    });
    let targets = config.get("target").and_then(toml::Value::as_table);
    let settings = targets.and_then(|targets| targets.get(target));
    let configured_linker = settings.and_then(|settings| settings.get("linker"));
    let mut driver = inputs.get(&target_linker).cloned().or_else(|| {
        configured_linker
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
    });
    if driver.is_none() {
        if let Some(targets) = targets {
            for (key, settings) in targets {
                if key.starts_with("cfg(") && settings.get("linker").is_some() {
                    return Err(unsupported(&format!(
                        "target.{key}.linker (unevaluated cfg)"
                    )));
                }
            }
        }
    }
    let flags = if let Some(encoded) = inputs.get("CARGO_ENCODED_RUSTFLAGS") {
        let flags = encoded.split('\u{1f}').map(str::to_owned).collect();
        inputs.remove("RUSTFLAGS");
        inputs.remove(&target_flags);
        inputs.remove("CARGO_BUILD_RUSTFLAGS");
        flags
    } else if let Some(flags) = inputs.get("RUSTFLAGS") {
        let flags = words(flags);
        inputs.remove(&target_flags);
        inputs.remove("CARGO_BUILD_RUSTFLAGS");
        flags
    } else {
        if let Some(targets) = targets {
            for (key, settings) in targets {
                if key.starts_with("cfg(") && settings.get("rustflags").is_some() {
                    return Err(unsupported(&format!(
                        "target.{key}.rustflags (unevaluated cfg)"
                    )));
                }
            }
        }
        let configured = settings.and_then(|settings| settings.get("rustflags"));
        if configured.is_some() || inputs.contains_key(&target_flags) {
            let mut flags = configured
                .map(config_flags)
                .transpose()?
                .unwrap_or_default();
            // Cargo appends the target environment flags to the configured target flags.
            if let Some(value) = inputs.get(&target_flags) {
                flags.extend(words(value));
            }
            inputs.remove("CARGO_BUILD_RUSTFLAGS");
            flags
        } else {
            let mut flags = config
                .get("build")
                .and_then(|build| build.get("rustflags"))
                .map(config_flags)
                .transpose()?
                .unwrap_or_default();
            if let Some(value) = inputs.get("CARGO_BUILD_RUSTFLAGS") {
                flags.extend(words(value));
            }
            flags
        }
    };
    let mut external_linker = None;
    let mut args = flags.iter();
    while let Some(arg) = args.next() {
        let option = match arg.as_str() {
            "-C" | "--codegen" | "-Z" => args.next().map(String::as_str),
            _ => arg
                .strip_prefix("--codegen=")
                .or_else(|| arg.strip_prefix("-C"))
                .or_else(|| arg.strip_prefix("-Z")),
        };
        let Some(option) = option else { continue };
        let (key, value) = option.split_once('=').unwrap_or((option, ""));
        match key {
            "link-self-contained" | "linker-flavor" | "linker-features" => {
                return Err(unsupported(&format!("{arg} {option}")));
            }
            "linker" => driver = Some(value.to_owned()),
            "link-arg" | "link-args" => {
                let driver_args = if key == "link-args" {
                    words(value)
                } else {
                    vec![value.to_owned()]
                };
                for arg in driver_args {
                    if let Some(flavor) = arg.strip_prefix("-fuse-ld=") {
                        external_linker = match flavor {
                            "lld" => None,
                            path if path.contains('/') => Some(path.to_owned()),
                            other => Some(format!("ld.{other}")),
                        };
                    }
                }
            }
            _ => {}
        }
    }
    Ok(Selections {
        driver: driver.unwrap_or_else(|| "cc".into()),
        bundled_lld_authorized: external_linker.is_none(),
        external_linker,
    })
}

/// Cargo merges strings nearest-first and arrays with the nearest items last. The caller supplies
/// active files from the keyed config walk, nearest first (`config` wins over `config.toml`).
pub(crate) fn merge_configs(
    configs: Vec<(std::path::PathBuf, Vec<u8>)>,
) -> ArtifactResult<toml::Value> {
    fn merge(lower: &mut toml::Value, higher: toml::Value) {
        match (lower, higher) {
            (toml::Value::Table(lower), toml::Value::Table(higher)) => {
                for (key, value) in higher {
                    if let Some(previous) = lower.get_mut(&key) {
                        merge(previous, value);
                    } else {
                        lower.insert(key, value);
                    }
                }
            }
            (toml::Value::Array(lower), toml::Value::Array(higher)) => lower.extend(higher),
            (lower, higher) => *lower = higher,
        }
    }
    let mut config = toml::Value::Table(Default::default());
    for (path, bytes) in configs.into_iter().rev() {
        let parsed = std::str::from_utf8(&bytes)
            .map_err(|e| {
                ArtifactError::Gap(format!("cargo configuration {}: {e}", path.display()))
            })?
            .parse::<toml::Value>()
            .map_err(|e| {
                ArtifactError::Gap(format!("cargo configuration {}: {e}", path.display()))
            })?;
        merge(&mut config, parsed);
    }
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TARGET: &str = "x86_64-unknown-linux-gnu";

    fn selection(flags: &str) -> Selections {
        resolve(
            &mut [("CARGO_ENCODED_RUSTFLAGS".into(), flags.into())].into(),
            &toml::Value::Table(Default::default()),
            TARGET,
        )
        .unwrap()
    }

    #[test]
    fn artifact_codegen_forms_preserve_order_and_link_args_boundaries() {
        let result = selection(concat!(
            "-Clinker=first\x1f-C\x1flinker=second\x1f--codegen\x1flinker=third\x1f",
            "--codegen=linker=last\x1f--codegen=link-args=-fuse-ld=bfd -fuse-ld=lld"
        ));
        assert_eq!(result.driver, "last");
        assert_eq!(result.external_linker, None);
        assert!(result.bundled_lld_authorized);
        let result = selection("-Clink-args=-fuse-ld=lld -fuse-ld=gold");
        assert_eq!(result.external_linker.as_deref(), Some("ld.gold"));
        assert!(!result.bundled_lld_authorized);
        let result = selection("--codegen=link-arg=-fuse-ld=/path with spaces/lld");
        assert_eq!(
            result.external_linker.as_deref(),
            Some("/path with spaces/lld")
        );
    }

    #[test]
    fn artifact_every_unsupported_codegen_form_refuses_even_before_override() {
        for flag in ["link-self-contained=-linker", "linker-flavor=gnu"] {
            for prefix in ["-C", "-C\x1f", "--codegen\x1f", "--codegen="] {
                let mut inputs = [(
                    "CARGO_ENCODED_RUSTFLAGS".into(),
                    format!("{prefix}{flag}\x1f-Clink-arg=-fuse-ld=lld"),
                )]
                .into();
                let error = resolve(&mut inputs, &toml::Value::Table(Default::default()), TARGET)
                    .unwrap_err();
                assert!(error.to_string().contains(flag));
            }
        }
        for prefix in ["-Z", "-Z\x1f"] {
            let mut inputs = [(
                "CARGO_ENCODED_RUSTFLAGS".into(),
                format!("{prefix}linker-features=-lld"),
            )]
            .into();
            assert!(resolve(&mut inputs, &toml::Value::Table(Default::default()), TARGET).is_err());
        }
    }

    #[test]
    fn artifact_target_config_and_environment_share_one_selection() {
        let config = "[target.x86_64-unknown-linux-gnu]\nlinker='configured'\nrustflags=['-Clinker=flag', '-Clink-arg=-fuse-ld=bfd']\n[target.aarch64-unknown-linux-gnu]\nrustflags=['-Clink-self-contained=no']\n[build]\nrustflags=['-Clinker-flavor=gnu']".parse().unwrap();
        let mut inputs = [
            (
                "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER".into(),
                "variable".into(),
            ),
            (
                "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS".into(),
                "--codegen linker=last -Clink-arg=-fuse-ld=lld".into(),
            ),
        ]
        .into();
        let result = resolve(&mut inputs, &config, TARGET).unwrap();
        assert_eq!(result.driver, "last");
        assert!(result.bundled_lld_authorized);
        inputs.insert("RUSTFLAGS".into(), String::new());
        let result = resolve(&mut inputs, &config, TARGET).unwrap();
        assert_eq!(result.driver, "variable");
        inputs.remove("CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER");
        assert_eq!(
            resolve(&mut inputs, &config, TARGET).unwrap().driver,
            "configured"
        );
    }

    #[test]
    fn artifact_unevaluated_cfg_refuses_only_when_flags_source_is_active() {
        let config = "[target.'cfg(unix)']\nrustflags=['-Copt-level=2']"
            .parse()
            .unwrap();
        assert!(resolve(&mut BTreeMap::new(), &config, TARGET)
            .unwrap_err()
            .to_string()
            .contains("cfg(unix)"));
        for name in ["RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS"] {
            let mut inputs = [(name.into(), String::new())].into();
            assert!(
                resolve(&mut inputs, &config, TARGET)
                    .unwrap()
                    .bundled_lld_authorized
            );
        }
    }

    #[test]
    fn artifact_config_hierarchy_merges_arrays_in_cargo_order() {
        let config = merge_configs(vec![
            ("near".into(), b"[build]\nrustflags=['-Clink-arg=-fuse-ld=lld']\n[target.x86_64-unknown-linux-gnu]\nlinker='near'".to_vec()),
            ("far".into(), b"[build]\nrustflags=['-Clink-arg=-fuse-ld=bfd']\n[target.x86_64-unknown-linux-gnu]\nlinker='far'".to_vec()),
        ]).unwrap();
        let result = resolve(&mut BTreeMap::new(), &config, TARGET).unwrap();
        assert_eq!(result.driver, "near");
        assert_eq!(result.external_linker, None);
        assert!(result.bundled_lld_authorized);
    }
}
