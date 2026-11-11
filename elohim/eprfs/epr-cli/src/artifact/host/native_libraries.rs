//! Derive std's native libraries from the recorded compiler for each key, without a cache.
//! Libraries added by dependency build scripts (`cargo:rustc-link-lib=...`, e.g. `-lssl`)
//! are outside std's set; closing that limit needs the verified link trace of K6 slice 2.

use std::collections::BTreeSet;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Stdio};

use super::{ArtifactError, ArtifactResult, MethodDescriptor};

pub(super) fn probe(descriptor: &MethodDescriptor, path: &str) -> ArtifactResult<BTreeSet<String>> {
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
    let output = Command::new(rustc)
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
        .current_dir(scratch.path())
        .stdin(Stdio::null())
        .env_clear()
        .envs(&descriptor.non_path_inputs)
        .env("PATH", path)
        .env("LANG", "C")
        .env("RUSTUP_TOOLCHAIN", toolchain)
        .env("TMPDIR", scratch.path())
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
