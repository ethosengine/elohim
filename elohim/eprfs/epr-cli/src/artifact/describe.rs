//! Cargo method discovery; shared mesh and standalone descriptor shape.
use super::*;

pub(super) fn describe(options: &Options) -> ArtifactResult<ExitCode> {
    let project = Options::required(&options.project, "--project")?;
    let bin = Options::required(&options.bin, "--bin")?;
    let rustflags = Options::required(&options.rustflags, "--rustflags")?;
    let sources = options.sources.as_deref().filter(|s| !s.is_empty());
    let crate_dir = options.crate_dir.as_deref().unwrap_or(".");
    let repo = &options.repo;
    let crate_path = repo.join(crate_dir);

    let mut missing = Vec::new();
    cargo_project::source_prerequisites(repo, &crate_path, &mut missing);
    preflight::refuse(missing)?;
    let mut non_path_inputs = BTreeMap::new();
    if let Ok(path) = std::env::var("PATH") {
        non_path_inputs.insert(PATH_INPUT.into(), path);
    }
    non_path_inputs.insert("RUSTFLAGS".to_string(), rustflags.to_string());
    // The crate, so the closure reads every Cargo configuration cargo reads for it.
    non_path_inputs.insert(CRATE_INPUT.to_string(), crate_dir.to_string());
    // Every native toolchain variable set here reaches cargo in the mesh build: recorded.
    for name in NATIVE_BUILD_ENV {
        if let Ok(value) = std::env::var(name) {
            non_path_inputs.insert(name.to_string(), value);
        }
    }
    for pair in &options.env {
        let (key, value) = pair.split_once('=').ok_or_else(|| {
            ArtifactError::InvalidArguments(format!("--env {pair}: expected K=V"))
        })?;
        if key == "RUSTUP_TOOLCHAIN" {
            if std::env::var(key).ok().as_deref() != Some(value) {
                return Err(ArtifactError::Gap(
                    "RUSTUP_TOOLCHAIN must match the discovery environment".into(),
                ));
            }
            continue; // tools["toolchain"] is the single recorded selector.
        }
        if RESOURCE_CAPS.contains(&key) {
            continue;
        }
        if non_path_inputs
            .insert(key.to_string(), value.to_string())
            .is_some()
            && build_input_class(key) == BuildInputClass::Target
        {
            return Err(ArtifactError::Gap(format!("{key} is declared twice")));
        }
    }
    for (name, value) in &options.target {
        if non_path_inputs
            .insert(name.clone(), value.clone())
            .is_some()
        {
            return Err(ArtifactError::Gap(format!("{name} is declared twice")));
        }
    }
    validate_target_inputs(&non_path_inputs)?;

    let rustc = tool_line("rustc", &["-vV"], &crate_path)?;
    let field = |name: &str| {
        rustc
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{name}: ")))
            .map(str::trim)
            .unwrap_or_default()
            .to_string()
    };
    let host = field("host");
    if host.is_empty() {
        return Err(ArtifactError::Gap("rustc -vV names no host triple".into()));
    }
    let cargo = tool_line("cargo", &["-V"], &crate_path)?;
    let mut tools = BTreeMap::new();
    tools.insert("rustc".to_string(), rustc.clone());
    tools.insert(
        "cargo".to_string(),
        cargo.lines().next().unwrap_or_default().to_string(),
    );
    tools.insert(
        "toolchain".to_string(),
        std::env::var("RUSTUP_TOOLCHAIN").unwrap_or_else(|_| "unpinned".into()),
    );

    let features = options.features.as_deref().filter(|f| !f.trim().is_empty());
    let mut flags = vec![format!("--bin={bin}")];
    if let Some(features) = features {
        flags.push(format!("--features={features}"));
    }
    flags.push("--profile=dev".into());
    flags.push(format!("--target={host}"));

    // The view: the build step's declared sources, the lock and cargo config beside the crate, any
    // toolchain file, and every path package cargo resolves for this build.
    let mut view: Vec<String> = Vec::new();
    if let Some(sources) = sources {
        let (manifest, step) = sources
            .split_once('#')
            .ok_or_else(|| ArtifactError::InvalidArguments("--sources is MANIFEST#STEP".into()))?;
        let manifest_json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(repo.join(manifest))
                .map_err(|e| ArtifactError::Gap(format!("{manifest} does not read: {e}")))?,
        )?;
        let globs = manifest_json["steps"][step]["inputs"]["sources"]
            .as_array()
            .ok_or_else(|| {
                ArtifactError::Gap(format!(
                    "{manifest} declares no steps.{step}.inputs.sources"
                ))
            })?;
        for glob in globs {
            let glob = glob.as_str().ok_or_else(|| {
                ArtifactError::Gap(format!("{manifest} {step}: a source is not a string"))
            })?;
            let prefix = glob_prefix(glob).ok_or_else(|| {
                ArtifactError::Gap(format!(
                "{manifest} {step}: the source `{glob}` covers the whole tree; no closure is keyed"
            ))
            })?;
            view.push(prefix);
        }
    }
    for extra in [
        "Cargo.lock",
        ".cargo/config.toml",
        "rust-toolchain",
        "rust-toolchain.toml",
    ] {
        if crate_path.join(extra).is_file() {
            view.push(format!("{crate_dir}/{extra}"));
        }
    }
    view.push(format!("{crate_dir}/Cargo.toml"));
    let metadata = cargo_project::metadata(&crate_path, &host, features)?;
    let root = repo.canonicalize().unwrap_or_else(|_| repo.clone());
    for package in metadata["packages"].as_array().into_iter().flatten() {
        if !package["source"].is_null() {
            continue;
        }
        let Some(dir) = package["manifest_path"]
            .as_str()
            .and_then(|p| Path::new(p).parent())
        else {
            continue;
        };
        let dir = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
        let relative = dir.strip_prefix(&root).map_err(|_| {
            ArtifactError::Gap(format!(
                "the path package {} is outside the checkout, so no closure is keyed",
                dir.display()
            ))
        })?;
        if relative.as_os_str().is_empty() {
            // A root package is a file view, never the forbidden empty/whole-tree view.
            view.extend(cargo_project::tracked_files(repo)?);
        } else {
            view.push(relative.to_string_lossy().into_owned());
        }
    }
    for path in &mut view {
        if let Some(relative) = path.strip_prefix("./") {
            *path = relative.to_string();
        }
    }
    let view = View::from_tree_paths(
        view.iter()
            .map(|path| TreePath::from_slash_bytes(path.as_bytes()).map_err(|e| e.to_string()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ArtifactError::Gap(format!("a view path is invalid: {e}")))?,
    );

    let descriptor = MethodDescriptor {
        name: MESH_BUILD.into(),
        tools,
        flags,
        params: None,
        exec: None,
        step: Some(format!("{MESH_BUILD}:{project}")),
        embedder: None,
        view: view.paths().cloned().collect(),
        non_path_inputs,
        deterministic: false,
    };
    let mut missing = Vec::new();
    let gaps = cargo_project::checklist(repo, &descriptor, &mut missing);
    preflight::print_gaps(&gaps);
    preflight::refuse(missing)?;
    let (_, config) = cargo_config_lines(&options.repo, &descriptor)?;
    let (effective, _) = effective_selection(&descriptor, &config)?;
    let text = serde_json::to_string_pretty(&descriptor)?;
    match &options.out {
        Some(out) => {
            if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(out, format!("{text}\n"))?;
            let method = effective
                .cid()
                .map_err(|e| ArtifactError::Gap(e.to_string()))?;
            println!("{method}");
        }
        None => println!("{text}"),
    }
    Ok(ExitCode::SUCCESS)
}
