//! Payload-only observations: linker-script operands, ELF GNU properties and glibc headers.
//! No host library lookup, command execution, or assumption that absent notes imply v2.
use super::{
    limits::EvidenceBudget, limits::Limit, refused, ArtifactError, ArtifactResult, Members,
};
use object::read::elf::{ElfFile, FileHeader, NoteIterator, ProgramHeader, SectionHeader};
use std::collections::BTreeSet;

pub(super) fn isa(
    bytes: &[u8],
    budget: &mut EvidenceBudget,
) -> ArtifactResult<Option<Vec<String>>> {
    if !bytes.starts_with(b"\x7fELF") {
        return Ok(None);
    }
    let bits = match object::File::parse(bytes).map_err(refused)? {
        object::File::Elf32(file) => elf_isa(&file, bytes)?,
        object::File::Elf64(file) => elf_isa(&file, bytes)?,
        _ => return Err(refused("ELF parser returned another format")),
    };
    let mut needed = Vec::new();
    for (bit, name) in [
        (1, "x86-64-baseline"),
        (2, "x86-64-v2"),
        (4, "x86-64-v3"),
        (8, "x86-64-v4"),
    ] {
        if bits & bit != 0 {
            needed.push(budget.retain(name)?);
        }
    }
    if bits & !15 != 0 {
        needed.push(budget.retain(&format!("unknown-x86-isa-bits:0x{:08x}", bits & !15))?);
    }
    Ok(Some(needed))
}

fn elf_isa<'a, Elf: FileHeader>(file: &ElfFile<'a, Elf>, bytes: &'a [u8]) -> ArtifactResult<u32> {
    let mut bits = 0;
    for header in file.elf_program_headers() {
        if let Some(notes) = header.notes(file.endian(), bytes).map_err(refused)? {
            bits |= note_bits(notes, file.endian())?;
        }
    }
    for header in file.elf_section_table().iter() {
        if let Some(notes) = header.notes(file.endian(), bytes).map_err(refused)? {
            bits |= note_bits(notes, file.endian())?;
        }
    }
    Ok(bits)
}

fn note_bits<Elf: FileHeader>(
    notes: NoteIterator<'_, Elf>,
    endian: Elf::Endian,
) -> ArtifactResult<u32> {
    let mut bits = 0;
    for note in notes {
        if let Some(properties) = note.map_err(refused)?.gnu_properties(endian) {
            for property in properties {
                let property = property.map_err(refused)?;
                if property.pr_type() == object::elf::GNU_PROPERTY_X86_ISA_1_NEEDED {
                    if property.pr_data().len() != 4 {
                        return Err(refused("invalid x86 ISA needed property size"));
                    }
                    bits |= property.data_u32(endian).map_err(refused)?;
                }
            }
        }
    }
    Ok(bits)
}

pub(super) fn libc_version(
    bytes: &[u8],
    budget: &mut EvidenceBudget,
) -> ArtifactResult<Option<String>> {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Ok(None);
    };
    let define = |name: &str| {
        text.lines().find_map(|line| {
            let mut words = line.trim_start().strip_prefix('#')?.split_whitespace();
            if words.next()? != "define" || words.next()? != name {
                return None;
            }
            words.next()?.parse::<u32>().ok()
        })
    };
    let (Some(major), Some(minor)) = (define("__GLIBC__"), define("__GLIBC_MINOR__")) else {
        return Ok(None);
    };
    Ok(Some(budget.retain(&format!("{major}.{minor}"))?))
}

/// Borrow tokens on demand: no token vector proportional to the untrusted file size.
fn token<'a>(rest: &mut &'a str) -> ArtifactResult<Option<&'a str>> {
    loop {
        *rest = rest.trim_start();
        if rest.is_empty() {
            return Ok(None);
        }
        if let Some(comment) = rest.strip_prefix("/*") {
            *rest = &comment[comment
                .find("*/")
                .ok_or_else(|| refused("unterminated script comment"))?
                + 2..];
        } else if rest.starts_with('#') {
            *rest = rest.split_once('\n').map_or("", |(_, tail)| tail);
        } else {
            break;
        }
    }
    let (value, consumed) = if rest.starts_with(['(', ')', ',', ';']) {
        (&rest[..1], 1)
    } else if rest.starts_with(['\'', '"']) {
        let quote = rest.as_bytes()[0] as char;
        let end = rest[1..]
            .find(quote)
            .ok_or_else(|| refused("unterminated script quote"))?
            + 1;
        (&rest[1..end], end + 1)
    } else {
        let end = rest
            .find(|c: char| c.is_whitespace() || "(),;".contains(c))
            .unwrap_or(rest.len());
        let end = rest[..end]
            .find("/*")
            .map_or(end, |comment| end.min(comment));
        (&rest[..end], end)
    };
    *rest = &rest[consumed..];
    Ok(Some(value))
}

fn normalize(path: &str) -> Option<String> {
    let mut result = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                result.pop()?;
            }
            _ => result.push(part),
        }
    }
    let path = result.join("/");
    super::valid_path(&path).then_some(path)
}

fn resolve<'a>(script: &str, operand: &str, members: &'a Members) -> ArtifactResult<&'a str> {
    let operand = operand.strip_prefix('=').unwrap_or(operand);
    let parent = script.rsplit_once('/').map_or("", |(parent, _)| parent);
    let candidates = if operand.starts_with('/') {
        vec![operand.to_string()]
    } else if let Some(lib) = operand.strip_prefix("-l") {
        let names = if let Some(exact) = lib.strip_prefix(':') {
            vec![exact.to_string()]
        } else {
            vec![format!("lib{lib}.so"), format!("lib{lib}.a")]
        };
        [parent, "lib64", "usr/lib64", "lib", "usr/lib"]
            .into_iter()
            .flat_map(|dir| names.iter().map(move |name| format!("{dir}/{name}")))
            .collect()
    } else {
        vec![format!("{parent}/{operand}"), operand.to_string()]
    };
    candidates
        .iter()
        .filter_map(|path| normalize(path))
        .find_map(|path| {
            members
                .get_key_value(&path)
                .and_then(|(key, value)| value.as_ref().map(|_| key.as_str()))
        })
        .ok_or_else(|| {
            refused(format!(
                "script {script}: operand {operand} is absent inside the root"
            ))
        })
}

/// A deliberately bounded GNU script grammar: GROUP/INPUT, nested AS_NEEDED and format/arch
/// declarations. Unknown directives refuse instead of silently missing a dependency. Search
/// paths and INCLUDE require a later explicit grammar extension; no host search is attempted.
pub(super) fn script(
    path: &str,
    bytes: &[u8],
    members: &Members,
    budget: &mut EvidenceBudget,
) -> ArtifactResult<Option<Vec<String>>> {
    // Classification is entirely by location and magic, never suffix or first directive.
    // The sole data exception is *.pc directly in <library-dir>/pkgconfig: build
    // metadata, retained in the archive but not interpreted by ld. Libtool archives,
    // gconv lists and all other nonbinary library data refuse with the member path.
    let parent = path.rsplit_once('/').map_or("", |(parent, _)| parent);
    if !parent
        .split('/')
        .any(|part| matches!(part, "lib" | "lib64" | "lib32" | "libx32"))
        || bytes.starts_with(b"\x7fELF")
        || bytes.starts_with(b"!<arch>\n")
    {
        return Ok(None);
    }
    if path.ends_with(".pc")
        && parent.rsplit_once('/').is_some_and(|(library, leaf)| {
            leaf == "pkgconfig"
                && matches!(
                    library.rsplit('/').next(),
                    Some("lib" | "lib64" | "lib32" | "libx32")
                )
        })
    {
        return Ok(None);
    }
    parse_script(path, bytes, members, budget)
        .map(Some)
        .map_err(|error| match error {
            ArtifactError::Gap(reason) => ArtifactError::SysrootScript {
                path: path.into(),
                reason,
            },
            other => other,
        })
}

fn parse_script(
    path: &str,
    bytes: &[u8],
    members: &Members,
    budget: &mut EvidenceBudget,
) -> ArtifactResult<Vec<String>> {
    let mut rest = std::str::from_utf8(bytes).map_err(|_| refused("non-UTF-8 linker script"))?;
    let mut operands = BTreeSet::new();
    let mut operand_count = 0u64;
    let mut directives = 0;
    while let Some(directive) = token(&mut rest)? {
        if directive == ";" {
            continue;
        }
        let inputs = matches!(directive, "GROUP" | "INPUT");
        if !inputs && !matches!(directive, "OUTPUT_FORMAT" | "OUTPUT_ARCH") {
            return Err(refused(format!("unsupported directive {directive}")));
        }
        directives += 1;
        if token(&mut rest)? != Some("(") {
            return Err(refused("script directive missing parenthesis"));
        }
        let mut counts = vec![0usize];
        while !counts.is_empty() {
            let next = token(&mut rest)?.ok_or_else(|| refused("unclosed script directive"))?;
            match next {
                ")" => {
                    if counts.pop() == Some(0) {
                        return Err(refused("script directive has no operand"));
                    }
                }
                "," => {}
                "AS_NEEDED" if inputs => {
                    if token(&mut rest)? != Some("(") {
                        return Err(refused("AS_NEEDED missing parenthesis"));
                    }
                    Limit::Depth.check(counts.len() as u64 + 1)?;
                    *counts.last_mut().expect("inside directive") += 1;
                    counts.push(0);
                }
                "(" | ";" => return Err(refused("unsupported script syntax")),
                _ => {
                    // Peek through comments without retaining tokens. An unknown nested
                    // directive cannot masquerade as an OUTPUT_FORMAT/OUTPUT_ARCH operand.
                    let mut peek = rest;
                    if token(&mut peek)? == Some("(") {
                        return Err(refused(format!("unsupported directive {next}")));
                    }
                    if next.is_empty() {
                        return Err(refused("script directive has no operand"));
                    }
                    operand_count += 1;
                    Limit::ScriptOperands.check(operand_count)?;
                    *counts.last_mut().expect("inside directive") += 1;
                    if inputs {
                        let resolved = resolve(path, next, members)?;
                        // Charge on EVERY resolution, before cloning even a duplicate.
                        operands.insert(budget.retain(resolved)?);
                    }
                }
            }
        }
    }
    if directives == 0 {
        return Err(refused("script has no directive"));
    }
    Ok(operands.into_iter().collect())
}
