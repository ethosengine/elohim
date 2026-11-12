//! Check Cargo dep-info target/freshness and listed inputs; a build receipt must bind bytes.
use super::*;

pub(super) fn check(options: &Options, descriptor: &MethodDescriptor, missing: &mut Vec<String>) {
    // Cargo writes <bin>.d alongside <bin>. The mesh passes its pool slot's dev/debug/bin
    // to put after verifying its installed copy. Never search another build's target tree.
    let Some(binary) = &options.bytes else { return };
    let dep_info = binary.with_extension("d");
    let Ok(text) = std::fs::read_to_string(&dep_info) else {
        missing.push("missing build-inputs: compiler inputs unverifiable (no dep-info)".into());
        return;
    };
    let Some((target, sources)) = text.split_once(": ") else {
        missing
            .push("missing build-inputs: compiler inputs unverifiable (invalid dep-info)".into());
        return;
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
    for source in sources {
        match source.canonicalize() {
            Ok(p) if p.starts_with(&options.repo) => {
                cargo_project::source_in_view(&options.repo, descriptor, &p, missing)
            }
            Ok(p)
                if registry.as_ref().is_some_and(|r| p.starts_with(r))
                    || toolchain.as_ref().is_some_and(|r| p.starts_with(r)) => {}
            _ => missing.push(format!(
                "missing build-inputs: compiler input outside captured view: {}",
                source.display()
            )),
        }
    }
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
    #[test]
    fn dep_info_escaped_paths() {
        assert_eq!(
            super::words(r"/a/b\ c /a/\#d /a/$$e /a/back\\slash"),
            ["/a/b c", "/a/#d", "/a/$e", r"/a/back\slash"]
        );
    }
}
