//! Payload-only observations: linker-script operands, ELF GNU properties and glibc headers.
//! No host library lookup, command execution, or assumption that absent notes imply v2.
use super::{refused, ArtifactResult, Members};
use object::read::elf::{ElfFile, FileHeader, NoteIterator, ProgramHeader, SectionHeader};
use std::collections::BTreeSet;

pub(super) fn isa(bytes: &[u8]) -> ArtifactResult<Option<Vec<String>>> {
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
            needed.push(name.into());
        }
    }
    if bits & !15 != 0 {
        needed.push(format!("unknown-x86-isa-bits:0x{:08x}", bits & !15));
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

pub(super) fn libc_version(bytes: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(bytes).ok()?;
    let define = |name: &str| {
        text.lines().find_map(|line| {
            let mut words = line.trim_start().strip_prefix('#')?.split_whitespace();
            if words.next()? != "define" || words.next()? != name {
                return None;
            }
            words.next()?.parse::<u32>().ok()
        })
    };
    Some(format!(
        "{}.{}",
        define("__GLIBC__")?,
        define("__GLIBC_MINOR__")?
    ))
}

/// One shared, borrowing tokenizer for detection and validation. Non-script headers stop after
/// the first two tokens rather than allocating a token vector for their entire contents.
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

fn resolve(script: &str, operand: &str, members: &Members) -> ArtifactResult<String> {
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
        .find(|path| matches!(members.get(path), Some(Some(_))))
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
) -> ArtifactResult<Option<Vec<String>>> {
    if bytes.starts_with(b"\x7fELF") || bytes.starts_with(b"!<arch>\n") {
        return Ok(None);
    }
    let extension = std::path::Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str());
    let named_script = matches!(extension, Some("ld" | "lds" | "so"));
    let Ok(text) = std::str::from_utf8(bytes) else {
        if named_script {
            return Err(refused("non-UTF-8 linker script"));
        }
        return Ok(None);
    };
    let mut rest = text;
    let mut first = token(&mut rest)?.unwrap_or("");
    while first == ";" {
        first = token(&mut rest)?.unwrap_or("");
    }
    let second = token(&mut rest)?;
    let directive = first.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
        && first.bytes().all(|b| b.is_ascii_uppercase() || b == b'_');
    // C headers also start with uppercase function-like macros after their # lines. Known
    // linker directives still identify scripts regardless of filename; unknown macros in a
    // header alone do not. Explicit script filenames are always validated.
    let candidate = matches!(
        first,
        "GROUP"
            | "INPUT"
            | "AS_NEEDED"
            | "OUTPUT_FORMAT"
            | "OUTPUT_ARCH"
            | "SEARCH_DIR"
            | "SECTIONS"
            | "MEMORY"
            | "PHDRS"
            | "VERSION"
            | "INCLUDE"
            | "ENTRY"
            | "EXTERN"
            | "OUTPUT"
            | "TARGET"
            | "STARTUP"
            | "ASSERT"
            | "PROVIDE"
            | "PROVIDE_HIDDEN"
            | "REGION_ALIAS"
            | "INSERT"
    ) || (directive
        && matches!(second, Some("(" | "{"))
        && !matches!(extension, Some("h" | "hpp")))
        || named_script
        || text.contains("/* GNU ld script");
    if !candidate {
        return Ok(None);
    }
    let mut tokens = vec![first];
    tokens.extend(second);
    while let Some(next) = token(&mut rest)? {
        tokens.push(next);
    }
    let mut i = 0;
    let mut operands = BTreeSet::new();
    while i < tokens.len() {
        if tokens[i] == ";" {
            i += 1;
            continue;
        }
        let directive = tokens[i];
        let inputs = matches!(directive, "GROUP" | "INPUT");
        if !inputs && !matches!(directive, "OUTPUT_FORMAT" | "OUTPUT_ARCH") {
            return Err(refused(format!(
                "script {path}: unsupported directive {directive}"
            )));
        }
        i += 1;
        if tokens.get(i).copied() != Some("(") {
            return Err(refused("script directive missing parenthesis"));
        }
        i += 1;
        let mut counts = vec![0usize];
        while !counts.is_empty() {
            let token = tokens
                .get(i)
                .ok_or_else(|| refused("unclosed script directive"))?;
            i += 1;
            match *token {
                ")" => {
                    if counts.pop() == Some(0) {
                        return Err(refused("script directive has no operand"));
                    }
                }
                "," => {}
                "AS_NEEDED" if inputs => {
                    if tokens.get(i).copied() != Some("(") {
                        return Err(refused("AS_NEEDED missing parenthesis"));
                    }
                    i += 1;
                    *counts.last_mut().expect("inside directive") += 1;
                    counts.push(0);
                }
                "(" | ";" => return Err(refused("unsupported script syntax")),
                _ => {
                    if tokens.get(i).copied() == Some("(") {
                        return Err(refused(format!(
                            "script {path}: unsupported directive {token}"
                        )));
                    }
                    *counts.last_mut().expect("inside directive") += 1;
                    if inputs {
                        operands.insert(resolve(path, token, members)?);
                    }
                }
            }
        }
    }
    Ok(Some(operands.into_iter().collect()))
}
