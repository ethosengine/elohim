//! Device-local loader search directories. No cache subprocess or persisted inventory.
use std::{
    collections::BTreeSet,
    ffi::{CStr, CString},
    fs,
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
};

pub(super) fn default_dirs(arch: &str) -> BTreeSet<PathBuf> {
    let mut dirs = ["/lib64", "/usr/lib64", "/lib", "/usr/lib"]
        .into_iter()
        .map(PathBuf::from)
        .collect::<BTreeSet<_>>();
    let multiarch = match arch {
        "x86_64" => Some("x86_64-linux-gnu"),
        "i386" | "i486" | "i586" | "i686" => Some("i386-linux-gnu"),
        "aarch64" => Some("aarch64-linux-gnu"),
        "riscv64" => Some("riscv64-linux-gnu"),
        _ => None,
    };
    if let Some(multiarch) = multiarch {
        for base in ["/lib", "/usr/lib"] {
            dirs.insert(Path::new(base).join(multiarch));
        }
    }
    dirs
}

pub(super) fn observe(mut dirs: BTreeSet<PathBuf>, config: &Path) -> Option<BTreeSet<String>> {
    read_config(config, &mut dirs, &mut BTreeSet::new());
    let mut names = BTreeSet::new();
    let mut readable = false;
    for dir in dirs {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        readable = true;
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            // Follow symlinks to establish resolvability, but keep the link's name.
            if (name.ends_with(".so") || name.contains(".so.")) && entry.path().is_file() {
                names.insert(name.into());
            }
        }
    }
    readable.then_some(names)
}

fn read_config(path: &Path, dirs: &mut BTreeSet<PathBuf>, visited: &mut BTreeSet<PathBuf>) {
    let Ok(canonical) = fs::canonicalize(path) else {
        return;
    };
    if !visited.insert(canonical) {
        return;
    }
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or_default().trim();
        let mut words = line.split_whitespace();
        match words.next() {
            Some("include") => {
                for pattern in words {
                    let pattern = path.parent().unwrap_or(Path::new("/")).join(pattern);
                    for included in expand(&pattern) {
                        read_config(&included, dirs, visited);
                    }
                }
            }
            Some(_) if Path::new(line).is_absolute() => {
                dirs.insert(PathBuf::from(line));
            }
            _ => {}
        }
    }
}

// libc glob implements ld.so.conf's include patterns (including directory components)
// without launching a command or adding a second pattern language.
fn expand(pattern: &Path) -> Vec<PathBuf> {
    let Ok(pattern) = CString::new(pattern.as_os_str().as_bytes()) else {
        return Vec::new();
    };
    // SAFETY: glob initializes the zeroed result, the pattern is NUL terminated,
    // and no callback is supplied. Read only gl_pathc initialized entries on success;
    // copy their NUL-terminated paths before releasing glob's allocation on every exit.
    unsafe {
        let mut result: libc::glob_t = std::mem::zeroed();
        let status = libc::glob(pattern.as_ptr(), 0, None, &mut result);
        let mut paths = Vec::new();
        if status == 0 {
            for index in 0..result.gl_pathc {
                let bytes = CStr::from_ptr(*result.gl_pathv.add(index)).to_bytes();
                paths.push(PathBuf::from(std::ffi::OsString::from_vec(bytes.to_vec())));
            }
        }
        libc::globfree(&mut result);
        paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elohim_epr::verdict::Decision;
    use host_contract::{admit, names::axis, AxisId, AxisValue, Provides, Requires};

    #[test]
    fn libraries_include_defaults_config_globs_and_symlink_names() {
        let root = tempfile::tempdir().unwrap();
        for dir in ["default", "extra", "nested", "conf.d"] {
            fs::create_dir(root.path().join(dir)).unwrap();
        }
        let config = root.path().join("ld.so.conf");
        fs::write(
            &config,
            format!(
                "# comment\n{} # inline comment\ninclude conf.d/*.conf\n",
                root.path().join("extra").display()
            ),
        )
        .unwrap();
        fs::write(
            root.path().join("conf.d/libs.conf"),
            format!(
                "{}\ninclude ../ld.so.conf\n",
                root.path().join("nested").display()
            ),
        )
        .unwrap();
        fs::write(
            root.path().join("conf.d/ignored.txt"),
            root.path().join("unwanted").display().to_string(),
        )
        .unwrap();
        for (dir, name) in [
            ("default", "libdefault.so"),
            ("extra", "libextra.so.1"),
            ("nested", "libnested.so.2"),
        ] {
            fs::write(root.path().join(dir).join(name), b"fixture").unwrap();
        }
        std::os::unix::fs::symlink("libextra.so.1", root.path().join("extra/libalias.so")).unwrap();
        std::os::unix::fs::symlink("missing", root.path().join("extra/libbroken.so")).unwrap();
        fs::create_dir(root.path().join("extra/libdirectory.so")).unwrap();
        fs::write(root.path().join("extra/not-a-library"), b"").unwrap();
        let names = observe([root.path().join("default")].into(), &config).unwrap();
        assert_eq!(
            names,
            [
                "libdefault.so",
                "libextra.so.1",
                "libnested.so.2",
                "libalias.so"
            ]
            .map(String::from)
            .into()
        );
        let mut provides = Provides::default();
        provides.observe(AxisId(axis::LIB.into()), AxisValue::Names(names));
        let debug = format!("{provides:?}");
        assert!(debug.contains("lib"));
        assert!(!debug.contains("libextra.so.1"));
        for (required, expected) in [
            (vec!["libalias.so"], Decision::Permit),
            (vec!["libalias.so", "libmissing.so.1"], Decision::Refuse),
        ] {
            let requires: Requires = serde_json::from_value(serde_json::json!([{
                "axis": "lib", "predicate": {"kind": "present", "value": required}, "source": "extracted"
            }])).unwrap();
            assert_eq!(admit(&requires, &provides).decision, expected);
        }
    }

    #[test]
    fn unreadable_search_is_unobserved_but_empty_directory_is_observed() {
        let root = tempfile::tempdir().unwrap();
        let missing = root.path().join("missing");
        assert_eq!(observe([missing.clone()].into(), &missing), None);
        assert_eq!(
            observe([root.path().into()].into(), &missing),
            Some(BTreeSet::new())
        );
        let dirs = default_dirs("x86_64");
        assert!(dirs.contains(Path::new("/lib/x86_64-linux-gnu")));
        assert!(dirs.contains(Path::new("/usr/lib/x86_64-linux-gnu")));
        assert!(!dirs.contains(Path::new("/lib/i386-linux-gnu")));
    }
}
