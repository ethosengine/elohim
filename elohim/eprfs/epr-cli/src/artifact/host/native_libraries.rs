//! Derive std's native libraries from the recorded compiler for each key, without a cache.
//! Libraries added by dependency build scripts (`cargo:rustc-link-lib=...`, e.g. `-lssl`)
//! are outside std's set; closing that limit needs the verified link trace of K6 slice 2.

use std::collections::BTreeSet;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::{ArtifactError, ArtifactResult, MethodDescriptor};

pub(super) fn probe(descriptor: &MethodDescriptor, path: &str) -> ArtifactResult<BTreeSet<String>> {
    let recorded = descriptor
        .tools
        .get("rustc")
        .filter(|identity| {
            identity.starts_with("rustc ")
                && [
                    "binary",
                    "commit-hash",
                    "commit-date",
                    "host",
                    "release",
                    "LLVM version",
                ]
                .iter()
                .all(|field| {
                    identity.lines().any(|line| {
                        line.strip_prefix(&format!("{field}: "))
                            .is_some_and(|value| !value.is_empty())
                    })
                })
        })
        .ok_or_else(|| {
            ArtifactError::Gap(
                "native-static-libs needs a recorded full rustc -vV identity; describe again"
                    .into(),
            )
        })?;
    let target = super::target(descriptor);
    if target.is_empty() {
        return Err(ArtifactError::Gap(
            "native-static-libs needs a recorded target or rustc host triple".into(),
        ));
    }
    let toolchain = descriptor.tools.get("toolchain").ok_or_else(|| {
        ArtifactError::Gap("native-static-libs needs a recorded rustc toolchain".into())
    })?;
    // Use the process TMPDIR (or the platform default when unset), never the descriptor's.
    // Both the archive and rustc's scratch files stay inside this private, RAII-cleaned dir.
    let scratch = tempfile::Builder::new()
        .prefix("epr-native-libs-")
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .map_err(|e| ArtifactError::Gap(format!("native-static-libs scratch directory: {e}")))?;
    let rustc = descriptor
        .non_path_inputs
        .get("RUSTC")
        .map_or("rustc", String::as_str);
    let rustc = crate::artifact::resolve_tool_path(rustc, path).ok_or_else(|| {
        ArtifactError::Gap(format!(
            "rustc {rustc} does not resolve under recorded PATH"
        ))
    })?;
    let rustc = compiler(&rustc, descriptor)?;
    let mut command = Command::new(rustc);
    command
        .current_dir(scratch.path())
        .stdin(Stdio::null())
        .env_clear()
        .envs(&descriptor.non_path_inputs)
        .env("PATH", path)
        .env("LANG", "C")
        .env("RUSTUP_TOOLCHAIN", toolchain)
        // Defense in depth; compiler() never returns a rustup proxy.
        .env("RUSTUP_AUTO_INSTALL", "0")
        .env("TMPDIR", scratch.path());
    // Location context is process-local, not descriptor/key material. Remove stale recorded
    // locations even when the caller has no replacement. Snapshot once for both invocations.
    // A rustup or cargo home is passed through only when it already exists. Otherwise it points
    // inside the private scratch dir: a rustup proxy that compiler() did not recognise (a
    // standalone copy or hardlink named rustc) creates its home before honouring
    // RUSTUP_AUTO_INSTALL=0, and that write must land in scratch, never outside it. A real
    // compiler ignores both variables; a disguised proxy then fails verify_identity.
    for name in LOCATION_VARS {
        command.env_remove(name);
    }
    for (name, value) in location_env(scratch.path(), |name| std::env::var_os(name)) {
        command.env(name, value);
    }
    // Clone the prepared environment/cwd, keeping the identical resolved executable. Arguments
    // belong to each invocation so -vV cannot accidentally suppress the library probe.
    let mut identity = Command::new(command.get_program());
    identity
        .env_clear()
        .envs(command.get_envs().filter_map(|(k, v)| v.map(|v| (k, v))))
        .current_dir(scratch.path())
        .stdin(Stdio::null())
        .arg("-vV");
    verify_identity(&mut identity, recorded)?;
    let output = command
        .args([
            "--print",
            "native-static-libs",
            "--crate-type",
            "staticlib",
            "--target",
            target,
            "-o",
            "probe.a",
            "-",
        ])
        .output()
        .map_err(|e| ArtifactError::Gap(format!("rustc native-static-libs does not run: {e}")))?;
    if !output.status.success() {
        return Err(ArtifactError::Gap(format!(
            "rustc native-static-libs failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let stderr = std::str::from_utf8(&output.stderr)
        .map_err(|e| ArtifactError::Gap(format!("rustc native-static-libs is not UTF-8: {e}")))?;
    let mut notes = stderr
        .lines()
        .filter_map(|line| line.strip_prefix("note: native-static-libs: "));
    let note = notes.next().ok_or_else(|| {
        ArtifactError::Gap("rustc native-static-libs note is absent or unparseable".into())
    })?;
    let mut names = BTreeSet::new();
    for token in note.split_whitespace() {
        let name = token.strip_prefix("-l").filter(|name| {
            !name.is_empty()
                && name
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"_+.-".contains(&c))
        });
        let name = name.ok_or_else(|| {
            ArtifactError::Gap(format!(
                "rustc native-static-libs note is unparseable: {token}"
            ))
        })?;
        names.insert(name.to_owned());
    }
    if names.is_empty() || notes.next().is_some() {
        return Err(ArtifactError::Gap(
            "rustc native-static-libs note is empty or ambiguous".into(),
        ));
    }
    Ok(names)
}

/// Resolve an installed compiler without executing rustup, which creates its home even with
/// RUSTUP_AUTO_INSTALL=0. Location context stays process-local and outside the descriptor/key.
/// The toolchain-location variables the probe takes from this process, never from the descriptor.
const LOCATION_VARS: [&str; 3] = ["RUSTUP_HOME", "CARGO_HOME", "HOME"];

/// Location context for the probe. `HOME` passes through when set. `RUSTUP_HOME` and `CARGO_HOME`
/// pass through only when they name an existing directory; otherwise each points at a path inside
/// `scratch`, so a rustup proxy `compiler()` did not recognise (it creates its home before it reads
/// `RUSTUP_AUTO_INSTALL`) can only write inside the private scratch dir.
fn location_env(
    scratch: &Path,
    var: impl Fn(&str) -> Option<std::ffi::OsString>,
) -> Vec<(&'static str, std::ffi::OsString)> {
    let mut env = Vec::new();
    for name in LOCATION_VARS {
        match var(name) {
            Some(value) if name == "HOME" || Path::new(&value).is_dir() => env.push((name, value)),
            _ if name != "HOME" => env.push((
                name,
                scratch
                    .join(format!("{}-absent", name.to_ascii_lowercase()))
                    .into_os_string(),
            )),
            _ => {}
        }
    }
    env
}

pub(super) fn compiler(
    executable: &Path,
    descriptor: &MethodDescriptor,
) -> ArtifactResult<PathBuf> {
    let resolved = std::fs::canonicalize(executable).map_err(|e| {
        ArtifactError::Gap(format!(
            "rustc {} does not resolve: {e}",
            executable.display()
        ))
    })?;
    if !is_rustup_proxy(executable, &resolved) {
        return Ok(resolved);
    }
    let proxy = resolved;
    let toolchain = descriptor
        .tools
        .get("toolchain")
        .filter(|value| !value.is_empty() && value.as_str() != "unpinned")
        .ok_or_else(|| {
            ArtifactError::Gap("rustc proxy needs recorded RUSTUP_TOOLCHAIN; describe again".into())
        })?;
    let home = std::env::var_os("RUSTUP_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".rustup")))
        .ok_or_else(|| {
            ArtifactError::Gap("rustc proxy needs process RUSTUP_HOME or HOME".into())
        })?;
    if !home.is_dir() {
        return Err(ArtifactError::Gap(format!(
            "RUSTUP_HOME {} must be an existing directory; it will not be created",
            home.display()
        )));
    }
    // Use the compiler's host, never --target (which may name a cross-compilation target).
    let host = descriptor
        .tools
        .get("rustc")
        .and_then(|identity| {
            identity
                .lines()
                .find_map(|line| line.strip_prefix("host: "))
        })
        .filter(|host| !host.is_empty())
        .ok_or_else(|| ArtifactError::Gap("rustc proxy needs a recorded host triple".into()))?;
    let installed = if toolchain.ends_with(&format!("-{host}")) {
        toolchain.clone()
    } else {
        format!("{toolchain}-{host}")
    };
    // A rustup selector here must name one installed toolchain, not escape its directory.
    if Path::new(&installed).components().count() != 1
        || !matches!(
            Path::new(&installed).components().next(),
            Some(std::path::Component::Normal(_))
        )
    {
        return Err(ArtifactError::Gap(format!(
            "RUSTUP_TOOLCHAIN {toolchain} does not name an installed toolchain"
        )));
    }
    let executable = home.join("toolchains").join(installed).join("bin/rustc");
    if !std::fs::metadata(&executable)
        .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
    {
        return Err(ArtifactError::Gap(format!(
            "rustc toolchain not installed as a regular executable file: {}",
            executable.display()
        )));
    }
    let resolved = std::fs::canonicalize(&executable).map_err(|e| {
        ArtifactError::Gap(format!(
            "rustc {} does not resolve: {e}",
            executable.display()
        ))
    })?;
    if is_rustup_proxy(&executable, &resolved) || same_file(&resolved, &proxy) {
        return Err(ArtifactError::Gap(format!(
            "installed rustc {} is itself a rustup proxy",
            executable.display()
        )));
    }
    Ok(resolved)
}

fn is_rustup_proxy(executable: &Path, resolved: &Path) -> bool {
    if resolved.file_name().is_some_and(|name| name == "rustup") {
        return true;
    }
    // Rustup also installs hard-linked proxies: canonicalize alone preserves their rustc name.
    [executable, resolved]
        .iter()
        .any(|path| same_file(path, &path.with_file_name("rustup")))
}

fn same_file(left: &Path, right: &Path) -> bool {
    std::fs::metadata(left).is_ok_and(|left| {
        std::fs::metadata(right)
            .is_ok_and(|right| left.dev() == right.dev() && left.ino() == right.ino())
    })
}

fn verify_identity(command: &mut Command, recorded: &str) -> ArtifactResult<()> {
    let output = command
        .output()
        .map_err(|e| ArtifactError::Gap(format!("rustc identity -vV does not run: {e}")))?;
    if !output.status.success() {
        return Err(ArtifactError::Gap(format!(
            "rustc identity -vV failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    // describe uses process::run, which trims the outer whitespace of captured stdout.
    // Preserve that convention while comparing every field, including unknown future fields.
    let actual = std::str::from_utf8(&output.stdout)
        .map_err(|e| ArtifactError::Gap(format!("rustc identity -vV is not UTF-8: {e}")))?;
    if actual.trim() != recorded.trim() {
        let summary = |identity: &str| {
            ["release: ", "commit-hash: "]
                .map(|prefix| {
                    identity
                        .lines()
                        .find(|line| line.starts_with(prefix))
                        .map_or_else(|| format!("{prefix}<absent>"), str::to_owned)
                })
                .join(", ")
        };
        return Err(ArtifactError::Gap(format!(
            "rustc identity mismatch: recorded [{}]; resolved [{}]; full -vV output differs; describe again",
            summary(recorded), summary(actual)
        )));
    }
    Ok(())
}

pub(super) fn driver_file(
    driver: &Path,
    input: &str,
    path: &str,
) -> ArtifactResult<Option<String>> {
    let output = super::probe(driver, &[&format!("-print-file-name={input}")], path)?;
    let file = String::from_utf8(output.stdout).map_err(|e| {
        ArtifactError::Gap(format!(
            "{} returned a non-UTF-8 path for {input}: {e}",
            driver.display()
        ))
    })?;
    let file = file.trim();
    Ok((file != input).then(|| file.to_owned()))
}

#[cfg(test)]
mod tests;
