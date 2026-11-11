//! Host inputs observed before a build, and the linker witness checked before recording it.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Command, Output};

use object::{Object, ObjectSection};

use super::{resolve_tool, ArtifactError, ArtifactResult, BlobCid, MethodDescriptor, PATH_INPUT};

const LINK_INPUTS: [&str; 10] = [
    "crt1.o",
    "Scrt1.o",
    "crti.o",
    "crtn.o",
    "crtbeginS.o",
    "crtendS.o",
    "libgcc.a",
    "libgcc_s.so",
    "libc_nonshared.a",
    "libc.so",
];

fn probe(tool: &Path, args: &[&str], path: &str) -> ArtifactResult<Output> {
    let output = Command::new(tool)
        .args(args)
        .env_clear()
        .env("PATH", path)
        .env("LANG", "C")
        .output()
        .map_err(|e| {
            ArtifactError::Gap(format!(
                "{} {} does not run: {e}",
                tool.display(),
                args.join(" ")
            ))
        })?;
    if !output.status.success() {
        return Err(ArtifactError::Gap(format!(
            "{} {} failed: {}",
            tool.display(),
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(output)
}

fn output_bytes(output: Output) -> Vec<u8> {
    [output.stdout, output.stderr].concat()
}

/// Only recorded selections name system linkers. The ordinary rustc linker is accounted for by
/// its recorded compiler identity, not by whichever `ld` happens to be on PATH.
fn selected_linkers(descriptor: &MethodDescriptor) -> BTreeMap<String, String> {
    let mut selected = BTreeMap::new();
    for (name, value) in &descriptor.non_path_inputs {
        if name.starts_with("CARGO_TARGET_") && name.ends_with("_LINKER") {
            selected.insert(name.clone(), value.clone());
        }
        if name == "RUSTFLAGS"
            || name == "CARGO_ENCODED_RUSTFLAGS"
            || name.ends_with("_RUSTFLAGS")
            || name == "CFLAGS"
            || name == "CXXFLAGS"
            || name == "LDFLAGS"
        {
            let mut codegen = false;
            for word in value.split(|c: char| c.is_whitespace() || c == '\u{1f}') {
                if let Some(linker) = word
                    .strip_prefix("-Clinker=")
                    .or_else(|| codegen.then(|| word.strip_prefix("linker=")).flatten())
                {
                    selected.insert(format!("{name}:linker"), linker.to_string());
                }
                if let Some((_, linker)) = word.split_once("-fuse-ld=") {
                    let linker = if linker.contains('/') || linker.starts_with("ld") {
                        linker.to_string()
                    } else {
                        format!("ld.{linker}")
                    };
                    selected.insert(format!("{name}:fuse-ld"), linker);
                }
                codegen = word == "-C";
            }
        }
    }
    selected
}

/// Canonical tool paths and output CIDs under the recorded PATH, plus the driver's target defaults
/// and link input bytes. A driver echoing an unresolved input name contributes an explicit `-`.
pub(super) fn tool_lines(descriptor: &MethodDescriptor) -> ArtifactResult<String> {
    let Some(path) = descriptor.non_path_inputs.get(PATH_INPUT) else {
        return Ok(String::new());
    };
    let (label, cc) = descriptor
        .non_path_inputs
        .get("CC")
        .map_or(("cc", "cc"), |cc| ("CC", cc.as_str()));
    let mut names = selected_linkers(descriptor);
    names.insert(label.to_string(), cc.to_string());
    let mut lines = String::new();
    for (label, name) in names {
        let Some(resolved) = resolve_tool(&name, path) else {
            if label != "cc" && label != "CC" {
                return Err(ArtifactError::Gap(format!(
                    "selected linker {label}={name} does not resolve under recorded PATH"
                )));
            }
            lines.push_str(&format!("tool:{label}\t-\n"));
            continue;
        };
        let version =
            BlobCid::compute_bytes(&output_bytes(probe(&resolved, &["--version"], path)?));
        lines.push_str(&format!(
            "tool:{label}\t{}\t{version}\n",
            resolved.display()
        ));
        if label != "cc" && label != "CC" {
            continue;
        }
        let defaults = BlobCid::compute_bytes(&output_bytes(probe(
            &resolved,
            &["-Q", "--help=target"],
            path,
        )?));
        lines.push_str(&format!(
            "tool:{label}:target\t{}\t{defaults}\n",
            resolved.display()
        ));
        for input in LINK_INPUTS {
            let output = probe(&resolved, &[&format!("-print-file-name={input}")], path)?;
            let file = String::from_utf8(output.stdout).map_err(|e| {
                ArtifactError::Gap(format!("{cc} returned a non-UTF-8 path for {input}: {e}"))
            })?;
            let file = file.trim();
            if file == input {
                lines.push_str(&format!("link:{input}\t-\n"));
                continue;
            }
            let resolved = std::fs::canonicalize(file).map_err(|e| {
                ArtifactError::Gap(format!(
                    "link input {input} at {file} does not resolve: {e}"
                ))
            })?;
            let bytes = std::fs::read(&resolved).map_err(|e| {
                ArtifactError::Gap(format!(
                    "link input {input} at {} does not read: {e}",
                    resolved.display()
                ))
            })?;
            lines.push_str(&format!(
                "link:{input}\t{}\t{}\n",
                resolved.display(),
                BlobCid::compute_bytes(&bytes)
            ));
        }
    }
    Ok(lines)
}

fn linker_identity(text: &str) -> Option<(&str, &str)> {
    for (marker, kind) in [
        ("LLD ", "LLD"),
        ("GNU ld ", "GNU ld"),
        ("GNU gold ", "GNU gold"),
        ("mold ", "mold"),
    ] {
        if let Some((_, rest)) = text.split_once(marker) {
            // GNU's vendor annotation can precede its version number.
            let version = rest
                .split_whitespace()
                .find(|word| word.starts_with(|c: char| c.is_ascii_digit()))?;
            return Some((kind, version));
        }
    }
    None
}

/// Read only ELF section bytes. Missing, malformed or unaccounted linker evidence refuses before
/// `put` writes an attestation or a custody record; an old descriptor must be described again.
pub(super) fn check_linker(descriptor: &MethodDescriptor, bytes: &[u8]) -> ArtifactResult<()> {
    let elf = object::read::elf::ElfFile64::<object::Endianness>::parse(bytes).map_err(|e| {
        ArtifactError::Gap(format!(
            "artifact ELF does not read: {e}; no .comment Linker line"
        ))
    })?;
    let comment = elf
        .section_by_name(".comment")
        .ok_or_else(|| ArtifactError::Gap("artifact has no .comment Linker line".into()))?
        .data()
        .map_err(|e| ArtifactError::Gap(format!("artifact .comment does not read: {e}")))?;
    let observed: Vec<&str> = comment
        .split(|byte| *byte == 0)
        .filter_map(|bytes| std::str::from_utf8(bytes).ok())
        .flat_map(str::lines)
        .filter_map(|line| line.strip_prefix("Linker: "))
        .collect();
    if observed.is_empty() {
        return Err(ArtifactError::Gap(
            "artifact has no .comment Linker line".into(),
        ));
    }
    let llvm = descriptor
        .tools
        .get("rustc")
        .and_then(|rustc| {
            rustc
                .lines()
                .find_map(|line| line.strip_prefix("LLVM version: "))
        })
        .map(str::trim);
    let mut accounted = Vec::new();
    if let Some(version) = llvm.filter(|v| !v.is_empty()) {
        accounted.push(format!("LLD {version} (rustc LLVM)"));
    }
    for (label, name) in selected_linkers(descriptor) {
        let path = descriptor.non_path_inputs.get(PATH_INPUT).ok_or_else(|| {
            ArtifactError::Gap(format!(
                "selected linker {label}={name} has no recorded PATH"
            ))
        })?;
        let resolved = resolve_tool(&name, path).ok_or_else(|| {
            ArtifactError::Gap(format!("selected linker {label}={name} does not resolve"))
        })?;
        let version =
            String::from_utf8_lossy(&output_bytes(probe(&resolved, &["--version"], path)?))
                .into_owned();
        if let Some((kind, version)) = linker_identity(&version) {
            accounted.push(format!("{kind} {version} ({label}={name})"));
        }
    }
    for line in observed {
        let identity = linker_identity(line);
        if identity.is_none()
            || !accounted
                .iter()
                .any(|expected| linker_identity(expected) == identity)
        {
            return Err(ArtifactError::Gap(format!(
                "artifact Linker: {line}; keyed linker: {}",
                if accounted.is_empty() {
                    "none (no recorded rustc LLVM version or identifiable selected linker; describe again)".into()
                } else {
                    accounted.join(", ")
                }
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/support/artifact_elf.rs"]
mod fixture;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_linker_witness_matches_recorded_rustc_llvm() {
        let output = Command::new("rustc").arg("-vV").output().unwrap();
        assert!(output.status.success());
        let rustc = String::from_utf8(output.stdout).unwrap();
        let llvm = rustc
            .lines()
            .find_map(|line| line.strip_prefix("LLVM version: "))
            .unwrap()
            .to_string();
        let mut descriptor = MethodDescriptor {
            name: "mesh-build".into(),
            tools: [("rustc".into(), rustc)].into(),
            flags: vec![],
            params: None,
            exec: None,
            step: None,
            embedder: None,
            view: vec![],
            non_path_inputs: BTreeMap::new(),
            deterministic: false,
        };
        let valid = fixture::artifact_elf(format!("Linker: LLD {llvm} (toolchain)\0").as_bytes());
        check_linker(&descriptor, &valid).unwrap();
        for comment in ["Linker: GNU ld 2.41\0", "Linker: LLD 0.0.0\0"] {
            let err =
                check_linker(&descriptor, &fixture::artifact_elf(comment.as_bytes())).unwrap_err();
            assert!(matches!(err, ArtifactError::Gap(_)));
            let message = err.to_string();
            assert!(
                message.contains(comment.trim_end_matches('\0')),
                "{message}"
            );
            assert!(message.contains(&format!("LLD {llvm}")), "{message}");
        }
        for bytes in [
            fixture::artifact_elf(b"GCC: 14\0"),
            Vec::new(),
            valid[..100].to_vec(),
        ] {
            assert!(matches!(
                check_linker(&descriptor, &bytes),
                Err(ArtifactError::Gap(_))
            ));
        }
        let mixed =
            fixture::artifact_elf(format!("Linker: LLD {llvm}\0Linker: GNU ld 2.41\0").as_bytes());
        assert!(check_linker(&descriptor, &mixed).is_err());
        descriptor.tools.clear();
        assert!(check_linker(&descriptor, &valid)
            .unwrap_err()
            .to_string()
            .contains("describe again"));
    }
}
