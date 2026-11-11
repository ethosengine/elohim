//! Recorded linker selections: refuse unsupported routing before any host probe or build.

use std::collections::BTreeMap;
use std::path::Path;

use super::{ArtifactError, ArtifactResult, MethodDescriptor};

#[derive(Default)]
pub(super) struct Selections {
    pub drivers: BTreeMap<String, String>,
    pub linkers: BTreeMap<String, String>,
    pub external: bool,
}

fn unsupported(flag: &str) -> ArtifactError {
    ArtifactError::Gap(format!(
        "{flag}: the key does not model that linker selection yet"
    ))
}

impl Selections {
    fn flags(&mut self, name: &str, value: &str) -> ArtifactResult<()> {
        let mut prefix = "";
        for word in value
            .split(|c: char| c.is_whitespace() || c == '\u{1f}')
            .filter(|w| !w.is_empty())
        {
            let option = word
                .strip_prefix("-C")
                .or_else(|| word.strip_prefix("-Z"))
                .or_else(|| (!prefix.is_empty()).then_some(word));
            if let Some(option) = option {
                let flag = option.split('=').next().unwrap_or(option);
                if matches!(
                    flag,
                    "link-self-contained" | "linker-features" | "linker-flavor"
                ) {
                    return Err(unsupported(&format!("{name}: {word}")));
                }
                if let Some(driver) = option.strip_prefix("linker=") {
                    self.drivers.insert(format!("{name}:linker"), driver.into());
                }
            }
            if let Some((_, linker)) = word.split_once("-fuse-ld=") {
                // Bare lld is rustc's sysroot gcc-ld shim, covered by rustc -vV.
                if linker != "lld" {
                    self.external = true;
                    let executable = if linker.contains('/') {
                        linker.to_string()
                    } else {
                        format!("ld.{linker}")
                    };
                    self.linkers.insert(format!("{name}:fuse-ld"), executable);
                }
            }
            prefix = if matches!(word, "-C" | "-Z") {
                word
            } else {
                ""
            };
        }
        Ok(())
    }
}

pub(super) fn selected(descriptor: &MethodDescriptor) -> ArtifactResult<Selections> {
    let mut selections = Selections::default();
    for (name, value) in &descriptor.non_path_inputs {
        if name.starts_with("CARGO_TARGET_") && name.ends_with("_LINKER") {
            selections.drivers.insert(name.clone(), value.clone());
        }
        if name == "RUSTFLAGS" || name.ends_with("_RUSTFLAGS") {
            selections.flags(name, value)?;
        }
    }
    Ok(selections)
}

pub(crate) fn check_cargo_config(path: &Path, bytes: &[u8]) -> ArtifactResult<()> {
    let config: toml::Value = std::str::from_utf8(bytes)
        .map_err(|e| ArtifactError::Gap(format!("cargo configuration {}: {e}", path.display())))?
        .parse()
        .map_err(|e| ArtifactError::Gap(format!("cargo configuration {}: {e}", path.display())))?;
    let Some(targets) = config.get("target").and_then(toml::Value::as_table) else {
        return Ok(());
    };
    for (target, settings) in targets {
        let name = format!("{} [target.{target}]", path.display());
        if settings.get("linker").is_some() {
            return Err(unsupported(&format!("{name} linker")));
        }
        if let Some(flags) = settings.get("rustflags") {
            let flags = match flags {
                toml::Value::String(value) => value.clone(),
                toml::Value::Array(values) => values
                    .iter()
                    .filter_map(toml::Value::as_str)
                    .collect::<Vec<_>>()
                    .join("\u{1f}"),
                _ => {
                    return Err(ArtifactError::Gap(format!(
                        "{name} rustflags is not a string or array"
                    )))
                }
            };
            let mut selections = Selections::default();
            selections.flags(&name, &flags)?;
            if !selections.drivers.is_empty()
                || !selections.linkers.is_empty()
                || flags.contains("-fuse-ld=")
            {
                return Err(unsupported(&format!("{name} rustflags {flags}")));
            }
        }
    }
    Ok(())
}
