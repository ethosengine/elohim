//! Host inputs observed before a build, and the linker witness checked before recording it.

use std::path::Path;
use std::process::{Command, Output, Stdio};

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
        .stdin(Stdio::null())
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

mod selections;
pub(super) use selections::{codegen_options, merge_configs, resolve, target, Selections};

fn tool_version(lines: &mut String, label: &str, tool: &Path, path: &str) -> ArtifactResult<()> {
    let version = BlobCid::compute_bytes(&output_bytes(probe(tool, &["--version"], path)?));
    lines.push_str(&format!("tool:{label}\t{}\t{version}\n", tool.display()));
    Ok(())
}

fn driver_defaults(lines: &mut String, label: &str, tool: &Path, path: &str) -> ArtifactResult<()> {
    let output = probe(tool, &["-E", "-dM", "-x", "c", "-"], path)?;
    let macros = String::from_utf8(output.stdout).map_err(|e| {
        ArtifactError::Gap(format!("{} macro dump is not UTF-8: {e}", tool.display()))
    })?;
    let mut macros: Vec<_> = macros.lines().collect();
    macros.sort_unstable();
    let defaults = BlobCid::compute_bytes(macros.join("\n").as_bytes());
    lines.push_str(&format!(
        "tool:{label}:target\t{}\t{defaults}\n",
        tool.display()
    ));
    Ok(())
}

fn link_input(lines: &mut String, label: &str, file: &Path, depth: usize) -> ArtifactResult<()> {
    if depth > 4 {
        return Err(ArtifactError::Gap(format!(
            "link input {label}: linker script recursion exceeds depth 4"
        )));
    }
    let resolved = std::fs::canonicalize(file).map_err(|e| {
        ArtifactError::Gap(format!(
            "link input {label} at {} does not resolve: {e}",
            file.display()
        ))
    })?;
    let bytes = std::fs::read(&resolved).map_err(|e| {
        ArtifactError::Gap(format!(
            "link input {label} at {} does not read: {e}",
            resolved.display()
        ))
    })?;
    lines.push_str(&format!(
        "link:{label}\t{}\t{}\n",
        resolved.display(),
        BlobCid::compute_bytes(&bytes)
    ));
    if let Ok(text) = std::str::from_utf8(&bytes) {
        if text.contains("GROUP") || text.contains("INPUT") || text.starts_with("/* GNU ld script")
        {
            // Remove C comments before looking for paths; paths in explanatory prose are not inputs.
            let mut uncommented = String::new();
            let mut rest = text;
            while let Some((before, comment)) = rest.split_once("/*") {
                uncommented.push_str(before);
                uncommented.push(' ');
                rest = comment.split_once("*/").map_or("", |(_, tail)| tail);
            }
            uncommented.push_str(rest);
            let referents: std::collections::BTreeSet<_> = uncommented
                .split(|c: char| {
                    c.is_whitespace() || matches!(c, '(' | ')' | ',' | ';' | '\'' | '"')
                })
                .filter(|word| word.starts_with('/'))
                .collect();
            for referent in referents {
                link_input(
                    lines,
                    &format!("{label}>{referent}"),
                    Path::new(referent),
                    depth + 1,
                )?;
            }
        }
    }
    Ok(())
}

/// Probe the effective final-link driver and the independently selected build-script C compiler.
/// An explicit driver without fuse-ld also keys PATH's `ld` as an external linker. No selection,
/// or a final bare fuse-ld=lld, uses rustc's bundled linker identity instead.
pub(super) fn tool_lines(
    descriptor: &MethodDescriptor,
    selections: &Selections,
) -> ArtifactResult<String> {
    let Some(path) = descriptor.non_path_inputs.get(PATH_INPUT) else {
        if selections.driver != "cc" || selections.external_linker.is_some() {
            return Err(ArtifactError::Gap(
                "selected linker has no recorded PATH".into(),
            ));
        }
        return Ok(String::new());
    };
    let driver = resolve_tool(&selections.driver, path);
    if driver.is_none() && selections.driver != "cc" {
        return Err(ArtifactError::Gap(format!(
            "selected linker {} does not resolve under recorded PATH",
            selections.driver
        )));
    }
    let mut lines = String::new();
    if let Some(driver) = &driver {
        tool_version(&mut lines, "link-driver", driver, path)?;
        driver_defaults(&mut lines, "link-driver", driver, path)?;
        for input in LINK_INPUTS {
            let output = probe(driver, &[&format!("-print-file-name={input}")], path)?;
            let file = String::from_utf8(output.stdout).map_err(|e| {
                ArtifactError::Gap(format!(
                    "{} returned a non-UTF-8 path for {input}: {e}",
                    driver.display()
                ))
            })?;
            if file.trim() == input {
                lines.push_str(&format!("link:{input}\t-\n"));
            } else {
                link_input(&mut lines, input, Path::new(file.trim()), 0)?;
            }
        }
    } else {
        lines.push_str("tool:link-driver\t-\n");
    }
    let cc_name = descriptor
        .non_path_inputs
        .get("CC")
        .map_or("cc", String::as_str);
    let cc = resolve_tool(cc_name, path);
    if cc.is_none() && descriptor.non_path_inputs.contains_key("CC") {
        return Err(ArtifactError::Gap(format!(
            "build-script C compiler CC={cc_name} does not resolve under recorded PATH"
        )));
    }
    if let Some(cc) = cc {
        if driver.as_ref() != Some(&cc) {
            tool_version(&mut lines, "CC", &cc, path)?;
            driver_defaults(&mut lines, "CC", &cc, path)?;
        }
    }
    if let Some(name) = &selections.external_linker {
        let label = "external-linker";
        let resolved = resolve_tool(name, path).ok_or_else(|| {
            ArtifactError::Gap(format!(
                "selected linker {label}={name} does not resolve under recorded PATH"
            ))
        })?;
        tool_version(&mut lines, label, &resolved, path)?;
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
/// External GNU ld normally emits no linker witness. Its keyed --version proves which tool was
/// selected, not which tool produced these bytes: without a witness this build cannot be recorded.
/// Do not substitute the driver/compiler comment or the selected tool's version for that evidence.
pub(super) fn check_linker(
    descriptor: &MethodDescriptor,
    selections: &Selections,
    bytes: &[u8],
) -> ArtifactResult<()> {
    let kind = object::FileKind::parse(bytes).map_err(|e| {
        ArtifactError::Gap(format!(
            "artifact format does not read: {e}; no .comment Linker line"
        ))
    })?;
    if !matches!(kind, object::FileKind::Elf32 | object::FileKind::Elf64) {
        return Err(ArtifactError::Gap(format!(
            "artifact format {kind:?} is not ELF"
        )));
    }
    let elf = object::File::parse(bytes).map_err(|e| {
        ArtifactError::Gap(format!(
            "artifact ELF does not read: {e}; no .comment Linker line"
        ))
    })?;
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
    if let Some(version) = llvm.filter(|v| !v.is_empty() && selections.bundled_lld_authorized) {
        accounted.push(format!("LLD {version} (rustc LLVM)"));
    }
    if let Some(name) = &selections.external_linker {
        let label = "external-linker";
        let path = descriptor.non_path_inputs.get(PATH_INPUT).ok_or_else(|| {
            ArtifactError::Gap(format!(
                "selected linker {label}={name} has no recorded PATH"
            ))
        })?;
        let resolved = resolve_tool(name, path).ok_or_else(|| {
            ArtifactError::Gap(format!("selected linker {label}={name} does not resolve"))
        })?;
        let version =
            String::from_utf8_lossy(&output_bytes(probe(&resolved, &["--version"], path)?))
                .into_owned();
        if let Some((kind, version)) = linker_identity(&version) {
            accounted.push(format!("{kind} {version} ({label}={name})"));
        }
    }
    let missing_witness = || {
        ArtifactError::Gap(
            if accounted
                .iter()
                .any(|expected| matches!(linker_identity(expected), Some(("GNU ld", _))))
            {
                "an external GNU ld leaves no linker witness; this build cannot be recorded".into()
            } else {
                "artifact has no .comment Linker line".into()
            },
        )
    };
    let comment = elf
        .section_by_name(".comment")
        .ok_or_else(missing_witness)?
        .data()
        .map_err(|e| ArtifactError::Gap(format!("artifact .comment does not read: {e}")))?;
    let observed: Vec<&str> = comment
        .split(|byte| *byte == 0)
        .filter_map(|bytes| std::str::from_utf8(bytes).ok())
        .flat_map(str::lines)
        .filter_map(|line| {
            line.strip_prefix("Linker: ").or_else(|| {
                ["mold ", "GNU gold ", "GNU ld "]
                    .iter()
                    .any(|prefix| line.starts_with(prefix))
                    .then_some(line)
            })
        })
        .collect();
    if observed.is_empty() {
        return Err(missing_witness());
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
    use std::collections::BTreeMap;

    fn check_linker(descriptor: &MethodDescriptor, bytes: &[u8]) -> ArtifactResult<()> {
        let selection = resolve(
            &mut descriptor.non_path_inputs.clone(),
            &toml::Value::Table(Default::default()),
            target(descriptor),
        )?;
        super::check_linker(descriptor, &selection, bytes)
    }

    fn descriptor() -> MethodDescriptor {
        MethodDescriptor {
            name: "mesh-build".into(),
            tools: [("rustc".into(), "LLVM version: 22.1.2".into())].into(),
            flags: vec![],
            params: None,
            exec: None,
            step: None,
            embedder: None,
            view: vec![],
            non_path_inputs: BTreeMap::new(),
            deterministic: false,
        }
    }

    #[test]
    fn artifact_elf_class_and_endian_are_generic_and_other_formats_refuse_by_name() {
        for is64 in [false, true] {
            for little in [false, true] {
                let bytes = fixture::artifact_elf_format(b"Linker: LLD 22.1.2\0", is64, little);
                check_linker(&descriptor(), &bytes).unwrap();
            }
        }
        let mut macho = vec![0; 32];
        macho[..4].copy_from_slice(&[0xcf, 0xfa, 0xed, 0xfe]);
        let error = check_linker(&descriptor(), &macho).unwrap_err();
        assert!(error.to_string().contains("MachO64 is not ELF"), "{error}");
    }

    #[test]
    fn artifact_actual_linker_comment_formats_and_bundled_lld_authority() {
        use std::os::unix::fs::PermissionsExt;
        let tools = tempfile::tempdir().unwrap();
        let linker = tools.path().join("selected-linker");
        let mut descriptor = descriptor();
        descriptor
            .non_path_inputs
            .insert(PATH_INPUT.into(), tools.path().to_string_lossy().into());
        for (version, comment) in [
            (
                "mold 2.40.0 (compatible with GNU ld)",
                "mold 2.40.0 (compatible with GNU ld)",
            ),
            (
                "GNU gold (GNU Binutils 2.41) 1.16",
                "GNU gold (GNU Binutils 2.41) 1.16",
            ),
            ("GNU ld (GNU Binutils) 2.41", "GNU ld (GNU Binutils) 2.41"),
        ] {
            std::fs::write(&linker, format!("#!/bin/sh\necho '{version}'\n")).unwrap();
            std::fs::set_permissions(&linker, std::fs::Permissions::from_mode(0o755)).unwrap();
            descriptor.non_path_inputs.insert(
                "RUSTFLAGS".into(),
                format!("-C link-arg=-fuse-ld={}", linker.display()),
            );
            check_linker(&descriptor, &fixture::artifact_elf(comment.as_bytes())).unwrap();
            let bundled = fixture::artifact_elf(b"Linker: LLD 22.1.2\0");
            assert!(
                check_linker(&descriptor, &bundled).is_err(),
                "external selection cannot authorize bundled LLD"
            );
        }
        for flags in [
            "",
            "-C link-arg=-fuse-ld=lld",
            "-C link-arg=-fuse-ld=bfd -C link-arg=-fuse-ld=lld",
            "-Clinker=gcc -C link-arg=-fuse-ld=lld",
        ] {
            descriptor
                .non_path_inputs
                .insert("RUSTFLAGS".into(), flags.into());
            check_linker(&descriptor, &fixture::artifact_elf(b"Linker: LLD 22.1.2\0")).unwrap();
        }
        std::os::unix::fs::symlink(&linker, tools.path().join("ld")).unwrap();
        descriptor
            .non_path_inputs
            .insert("RUSTFLAGS".into(), "-Clinker=gcc".into());
        let bundled = fixture::artifact_elf(b"Linker: LLD 22.1.2\0");
        assert!(check_linker(&descriptor, &bundled).is_err());
        let comment_only = fixture::artifact_elf(b"GCC: 14.3.1\0");
        let mut no_comment = comment_only.clone();
        let name = no_comment
            .windows(8)
            .position(|bytes| bytes == b".comment")
            .unwrap();
        no_comment[name..name + 8].copy_from_slice(b".unknown");
        for bytes in [comment_only, no_comment] {
            let error = check_linker(&descriptor, &bytes).unwrap_err();
            assert!(matches!(error, ArtifactError::Gap(_)));
            assert!(
                error.to_string().contains(
                    "an external GNU ld leaves no linker witness; this build cannot be recorded"
                ),
                "{error}"
            );
        }
        descriptor.non_path_inputs.insert(
            "RUSTFLAGS".into(),
            "--codegen=link-self-contained=-linker".into(),
        );
        let error =
            check_linker(&descriptor, &fixture::artifact_elf(b"Linker: LLD 22.1.2\0")).unwrap_err();
        assert!(error.to_string().contains("link-self-contained"));
        descriptor.non_path_inputs.clear();
        assert!(check_linker(&descriptor, &fixture::artifact_elf(b"GCC: 14\0")).is_err());
    }

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
