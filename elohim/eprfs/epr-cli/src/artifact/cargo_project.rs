//! Read-only Cargo source discovery. Metadata is locked and offline; it never repairs a lock.
use super::*;

pub(super) const LOCK: &str = "missing recipe.source-lock: Cargo.lock";
pub(super) const COMMIT: &str = "missing recipe.source-commit";

fn lock_path(repo: &Path, crate_path: &Path) -> Option<PathBuf> {
    crate_path
        .ancestors()
        .take_while(|p| p.starts_with(repo))
        .find_map(|p| {
            let lock = p.join("Cargo.lock");
            lock.is_file().then_some(lock)
        })
}

pub(super) fn source_prerequisites(repo: &Path, crate_path: &Path, missing: &mut Vec<String>) {
    if git_ok(repo, &["rev-parse", "HEAD^{commit}"]).is_err()
        || !git_ok(repo, &["rev-parse", "--show-toplevel"])
            .is_ok_and(|root| Path::new(root.trim()) == repo)
    {
        missing.push(COMMIT.into());
    }
    let locked = lock_path(repo, crate_path).is_some_and(|path| {
        let relative = path.strip_prefix(repo).unwrap_or(&path).to_string_lossy();
        git_ok(repo, &["cat-file", "-e", &format!("HEAD:{relative}")]).is_ok()
    });
    if !locked {
        missing.push(LOCK.into());
    }
}

pub(super) fn tracked_files(repo: &Path) -> ArtifactResult<Vec<String>> {
    Ok(git_ok(repo, &["ls-files", "-z"])?
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect())
}

pub(super) fn metadata(
    crate_path: &Path,
    host: &str,
    features: Option<&str>,
) -> ArtifactResult<serde_json::Value> {
    let mut args = vec![
        "metadata",
        "--format-version",
        "1",
        "--locked",
        "--offline",
        "--filter-platform",
        host,
    ];
    if let Some(features) = features {
        args.extend(["--features", features]);
    }
    serde_json::from_str(&tool_line("cargo", &args, crate_path)?).map_err(Into::into)
}

// K6's pinned build closure must enumerate script/proc-macro inputs and capture registry
// source trees to close these two gaps. K12, blocked on H12, closes reach.
pub(super) const SCRIPTS_GAP: &str = "build-inputs.build-scripts";
pub(super) const SOURCES_GAP: &str = "build-inputs.dependency-sources";
// Close only when put runs the build, or an ark build receipt binds compiler inputs to the
// output bytes. Target/mtime checks alone cannot detect edited dep-info, for any Cargo recipe.
pub(super) const COMPILER_GAP: &str = "build-inputs.compiler-inputs-unattested";

pub(super) fn covered(descriptor: &MethodDescriptor, file: &str) -> bool {
    descriptor.view.iter().any(|p| {
        p.to_slash_string()
            .is_ok_and(|p| file == p || file.starts_with(&format!("{p}/")))
    })
}

pub(super) fn source_in_view(
    repo: &Path,
    descriptor: &MethodDescriptor,
    path: &Path,
    missing: &mut Vec<String>,
) {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let Ok(relative) = path.strip_prefix(repo) else {
        missing.push(format!(
            "missing build-inputs: path dependency outside captured view: {}",
            path.display()
        ));
        return;
    };
    let file = relative.to_string_lossy();
    if !covered(descriptor, &file)
        || git_ok(repo, &["cat-file", "-e", &format!("HEAD:{file}")]).is_err()
    {
        missing.push(format!(
            "missing build-inputs: compiler/source input not committed in view: {file}"
        ));
    }
}

pub(super) fn checklist(
    repo: &Path,
    descriptor: &MethodDescriptor,
    missing: &mut Vec<String>,
) -> Vec<String> {
    let mut gaps = vec!["reach".to_string(), COMPILER_GAP.to_string()];
    let crate_path = repo.join(
        descriptor
            .non_path_inputs
            .get(CRATE_INPUT)
            .map_or(".", String::as_str),
    );
    let crate_path = crate_path.canonicalize().unwrap_or(crate_path);
    source_prerequisites(repo, &crate_path, missing);
    let host = descriptor
        .tools
        .get("rustc")
        .and_then(|r| r.lines().find_map(|l| l.strip_prefix("host: ")));
    let config_target = cargo_config_lines(repo, descriptor)
        .ok()
        .is_some_and(|(_, c)| c.get("build").and_then(|b| b.get("target")).is_some());
    if host != Some(host::target(descriptor))
        || descriptor
            .non_path_inputs
            .contains_key("CARGO_BUILD_TARGET")
        || config_target
    {
        missing.push("missing declared-target: build realization unsupported".into());
    }
    // Inspect lock entries independently of metadata, so a bad pin cannot hide behind a
    // resolution failure. Cargo's locked/offline resolver checks manifest/lock agreement.
    if let Some(lock) = lock_path(repo, &crate_path) {
        source_in_view(repo, descriptor, &lock, missing);
        match std::fs::read_to_string(&lock)
            .ok()
            .and_then(|s| s.parse::<toml::Value>().ok())
        {
            Some(lock) => {
                for package in lock
                    .get("package")
                    .and_then(toml::Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    let source = package
                        .get("source")
                        .and_then(toml::Value::as_str)
                        .unwrap_or("");
                    let name = package
                        .get("name")
                        .and_then(toml::Value::as_str)
                        .unwrap_or("unknown");
                    if source.starts_with("registry+") {
                        gaps.push(SOURCES_GAP.into());
                        if !package
                            .get("checksum")
                            .and_then(toml::Value::as_str)
                            .is_some_and(|s| {
                                s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
                            })
                        {
                            missing.push(format!("missing recipe.source-lock: registry dependency {name} has no lockfile checksum"));
                        }
                    } else if source.starts_with("git+")
                        && !source.rsplit_once('#').is_some_and(|(_, s)| {
                            s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit())
                        })
                    {
                        missing.push(format!("missing recipe.source-lock: git dependency {name} has no pinned commit"));
                    }
                }
            }
            None => missing.push(format!("{LOCK}: invalid lockfile")),
        }
    }
    let features = descriptor
        .flags
        .iter()
        .find_map(|f| f.strip_prefix("--features="));
    match metadata(&crate_path, host::target(descriptor), features) {
        Ok(metadata) => {
            let mut selected_bin = false;
            for package in metadata["packages"].as_array().into_iter().flatten() {
                let local = package["source"].is_null();
                if local {
                    if let Some(manifest) = package["manifest_path"].as_str() {
                        source_in_view(repo, descriptor, Path::new(manifest), missing);
                    }
                }
                for target in package["targets"].as_array().into_iter().flatten() {
                    let kinds = target["kind"].as_array();
                    if kinds
                        .is_some_and(|k| k.iter().any(|k| k == "custom-build" || k == "proc-macro"))
                    {
                        gaps.push(SCRIPTS_GAP.into());
                    }
                    if local {
                        if let Some(src) = target["src_path"].as_str() {
                            source_in_view(repo, descriptor, Path::new(src), missing);
                        }
                    }
                    let bin = descriptor
                        .flags
                        .iter()
                        .find_map(|f| f.strip_prefix("--bin="));
                    if package["manifest_path"]
                        .as_str()
                        .is_some_and(|p| Path::new(p) == crate_path.join("Cargo.toml"))
                        && kinds.is_some_and(|k| k.iter().any(|k| k == "bin"))
                        && target["name"].as_str() == bin
                    {
                        selected_bin = true;
                    }
                }
            }
            if !selected_bin {
                missing.push("missing recipe: selected Cargo binary absent".into());
            }
        }
        Err(e) => missing.push(format!(
            "missing build-inputs: dependency metadata unverifiable: {e}"
        )),
    }
    gaps.sort();
    gaps.dedup();
    gaps
}
