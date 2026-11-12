//! Check Cargo dep-info target/freshness and listed inputs; a build receipt must bind bytes.
use super::*;

/// `build_scripts` is true when the closure declares the `build-inputs.build-scripts` gap: then a
/// file a build script generated into its OUT_DIR (this build's `<profile>/build/<pkg>-<hash>/out/`)
/// is that declared gap, not a missing fact. Cargo also merges build-script watch paths into
/// dep-info: absent watches contributed no bytes and are returned as preflight evidence only
/// under the same declared gap. Without the gap either kind of path still refuses.
pub(super) fn check(
    options: &Options,
    descriptor: &MethodDescriptor,
    build_scripts: bool,
    missing: &mut Vec<String>,
) -> Vec<PathBuf> {
    let mut watched_absent = Vec::new();
    // Cargo writes <bin>.d alongside <bin>. The mesh passes its pool slot's dev/debug/bin
    // to put after verifying its installed copy. Never search another build's target tree.
    let Some(binary) = &options.bytes else {
        return watched_absent;
    };
    let dep_info = binary.with_extension("d");
    let Ok(text) = std::fs::read_to_string(&dep_info) else {
        missing.push("missing build-inputs: compiler inputs unverifiable (no dep-info)".into());
        return watched_absent;
    };
    let Some((target, sources)) = text.split_once(": ") else {
        missing
            .push("missing build-inputs: compiler inputs unverifiable (invalid dep-info)".into());
        return watched_absent;
    };
    let crate_path = options.repo.join(
        descriptor
            .non_path_inputs
            .get(CRATE_INPUT)
            .map_or(".", String::as_str),
    );
    let paths = |s: &str| {
        words(s)
            .into_iter()
            .map(|s| {
                let p = PathBuf::from(s);
                if p.is_absolute() {
                    p
                } else {
                    crate_path.join(p)
                }
            })
            .collect::<Vec<_>>()
    };
    let targets = paths(target);
    if !targets
        .iter()
        .any(|p| p.canonicalize().ok() == binary.canonicalize().ok() && p.is_file())
    {
        missing.push(format!(
            "missing build-inputs: dep-info {} names target {target}, not recorded binary {}",
            dep_info.display(),
            binary.display()
        ));
    }
    let modified = |path: &Path| std::fs::metadata(path).and_then(|m| m.modified());
    match (modified(binary), modified(&dep_info)) {
        (Ok(binary_time), Ok(dep_time)) if binary_time <= dep_time => {}
        (Ok(_), Ok(_)) => missing.push(format!(
            "missing build-inputs: binary {} is newer than dep-info {}",
            binary.display(),
            dep_info.display()
        )),
        _ => missing.push(format!(
            "missing build-inputs: cannot compare mtimes of binary {} and dep-info {}",
            binary.display(),
            dep_info.display()
        )),
    }
    let sources = sources.replace("\\\n", "");
    let sources = paths(sources.lines().next().unwrap_or(""));
    if sources.is_empty() {
        missing.push("missing build-inputs: compiler inputs unverifiable (empty dep-info)".into());
    }
    let registry = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".cargo")))
        .and_then(|p| p.join("registry/src").canonicalize().ok());
    let toolchain = host::compiler_root(descriptor).ok();
    let script_outputs = binary
        .canonicalize()
        .ok()
        .and_then(|b| b.parent().map(|profile| profile.join("build")));
    for source in sources {
        match source.canonicalize() {
            Ok(p) if p.starts_with(&options.repo) => {
                cargo_project::source_in_view(&options.repo, descriptor, &p, missing)
            }
            Ok(p)
                if registry.as_ref().is_some_and(|r| p.starts_with(r))
                    || toolchain.as_ref().is_some_and(|r| p.starts_with(r)) => {}
            Ok(p) if build_scripts && is_script_output(&p, script_outputs.as_deref()) => {}
            // Only definite absence qualifies. Permission errors, symlink loops and existing
            // files outside the view remain refusals even with the build-script gap.
            Err(error) if build_scripts && error.kind() == std::io::ErrorKind::NotFound => {
                watched_absent.push(source);
            }
            _ => missing.push(format!(
                "missing build-inputs: compiler input outside captured view: {}",
                source.display()
            )),
        }
    }
    watched_absent.sort();
    watched_absent.dedup();
    watched_absent
}

/// `<profile>/build/<pkg>-<hash>/out/...`: a file a build script wrote into its OUT_DIR.
fn is_script_output(path: &Path, build_dir: Option<&Path>) -> bool {
    let Some(rest) = build_dir.and_then(|dir| path.strip_prefix(dir).ok()) else {
        return false;
    };
    let mut parts = rest.components();
    parts.next().is_some() && parts.next().is_some_and(|c| c.as_os_str() == "out")
}

// Cargo dep-info is make syntax: escaped spaces/backslashes/#, $$ for a literal dollar.
fn words(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(c) = chars.next() {
                    word.push(c);
                }
            }
            '$' if chars.peek() == Some(&'$') => {
                chars.next();
                word.push('$');
            }
            c if c.is_whitespace() => {
                if !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                }
            }
            c => word.push(c),
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input_check(
        build_scripts: bool,
        existing: bool,
    ) -> (Vec<String>, Vec<PathBuf>, Vec<PathBuf>) {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir(&repo).unwrap();
        let binary = dir.path().join("binary");
        std::fs::write(&binary, b"fixture").unwrap();
        let sources = vec![repo.join(".git/HEAD"), dir.path().join("outside.rs")];
        if existing {
            std::fs::write(&sources[1], b"outside captured view").unwrap();
        }
        std::fs::write(
            binary.with_extension("d"),
            format!(
                "{}: {} {}\n",
                binary.display(),
                sources[0].display(),
                sources[1].display()
            ),
        )
        .unwrap();
        let descriptor: MethodDescriptor = serde_json::from_value(serde_json::json!({
            "name": "mesh-build", "tools": {}, "flags": [], "params": null,
            "exec": null, "step": null, "embedder": null, "view": [],
            "nonPathInputs": {}, "deterministic": false
        }))
        .unwrap();
        let options = Options {
            repo,
            bytes: Some(binary),
            ..Default::default()
        };
        let mut missing = Vec::new();
        let absent = check(&options, &descriptor, build_scripts, &mut missing);
        (missing, absent, sources)
    }

    #[test]
    fn absent_watch_paths_with_build_script_gap_are_accepted_and_listed() {
        let (missing, mut absent, mut sources) = input_check(true, false);
        assert!(missing.is_empty(), "{missing:?}");
        absent.sort();
        sources.sort();
        assert_eq!(absent, sources);
    }

    #[test]
    fn absent_watch_paths_without_build_script_gap_refuse() {
        let (missing, absent, sources) = input_check(false, false);
        assert!(absent.is_empty());
        assert_eq!(missing.len(), 2, "{missing:?}");
        for path in sources {
            assert!(missing.iter().any(|m| m.contains(&format!(
                "compiler input outside captured view: {}",
                path.display()
            ))));
        }
    }

    #[test]
    fn existing_outside_input_still_refuses_with_build_script_gap() {
        let (missing, absent, sources) = input_check(true, true);
        assert_eq!(absent, vec![sources[0].clone()]);
        assert_eq!(
            missing,
            vec![format!(
                "missing build-inputs: compiler input outside captured view: {}",
                sources[1].display()
            )]
        );
    }

    #[test]
    fn dep_info_escaped_paths() {
        assert_eq!(
            super::words(r"/a/b\ c /a/\#d /a/$$e /a/back\\slash"),
            ["/a/b c", "/a/#d", "/a/$e", r"/a/back\slash"]
        );
    }

    #[test]
    fn build_script_out_dir_is_recognised_only_under_this_builds_profile() {
        use std::path::Path;
        let build = Path::new("/pool/dev/debug/build");
        let out = Path::new("/pool/dev/debug/build/elohim-chrome-asset-6e61/out/omni.js");
        assert!(super::is_script_output(out, Some(build)));
        // The package directory itself, or anything not under `out`, is not an OUT_DIR file.
        let script = Path::new("/pool/dev/debug/build/elohim-chrome-asset-6e61/build-script-build");
        assert!(!super::is_script_output(script, Some(build)));
        // Another build's tree is never accepted.
        let other = Path::new("/other/debug/build/pkg-1/out/x.js");
        assert!(!super::is_script_output(other, Some(build)));
        assert!(!super::is_script_output(out, None));
    }
}
