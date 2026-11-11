//! Derive std's native libraries from the recorded compiler for each key, without a cache.
//! Libraries added by dependency build scripts (`cargo:rustc-link-lib=...`, e.g. `-lssl`)
//! are outside std's set; closing that limit needs the verified link trace of K6 slice 2.

use std::collections::BTreeSet;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
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
    // Do not canonicalize rustc's executable: a rustup proxy dispatches by argv[0].
    let rustc = crate::artifact::resolve_tool_path(rustc, path).ok_or_else(|| {
        ArtifactError::Gap(format!(
            "rustc {rustc} does not resolve under recorded PATH"
        ))
    })?;
    let mut command = Command::new(rustc);
    command
        .current_dir(scratch.path())
        .stdin(Stdio::null())
        .env_clear()
        .envs(&descriptor.non_path_inputs)
        .env("PATH", path)
        .env("LANG", "C")
        .env("RUSTUP_TOOLCHAIN", toolchain)
        // Proxy invocation may only use an installed toolchain, never install/update one.
        .env("RUSTUP_AUTO_INSTALL", "0")
        .env("TMPDIR", scratch.path());
    // Location context is process-local, not descriptor/key material. Remove stale recorded
    // locations even when the caller has no replacement. Snapshot once for both invocations.
    for name in ["RUSTUP_HOME", "CARGO_HOME", "HOME"] {
        command.env_remove(name);
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
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
