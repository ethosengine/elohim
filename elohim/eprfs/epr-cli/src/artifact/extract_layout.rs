//! Check the loader's file view before trusting section metadata or decoding bytes.
use std::ops::Range;

use object::elf;
use object::read::elf::{Dyn, ProgramHeader, SectionHeader};
use serde_json::{json, Value};

use super::{text, unrecorded, ArtifactResult, Elf};

pub(super) struct Budget {
    remaining: usize,
    kind: &'static str,
}

impl Budget {
    pub(super) fn new(bytes: usize, minimum_entry: usize, kind: &'static str) -> Self {
        Self {
            remaining: bytes / minimum_entry,
            kind,
        }
    }

    pub(super) fn take(&mut self, count: usize) -> ArtifactResult<()> {
        self.remaining = self
            .remaining
            .checked_sub(count)
            .ok_or_else(|| unrecorded(format!("metadata budget exhausted: {}", self.kind)))?;
        Ok(())
    }
}

pub(super) fn name(value: &str) -> &str {
    let mut end = value.len().min(256);
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}

pub(super) fn trim_evidence(value: &mut Value) {
    match value {
        Value::String(s) => s.truncate(name(s).len()),
        Value::Array(a) => a.iter_mut().for_each(trim_evidence),
        Value::Object(o) => o.values_mut().for_each(trim_evidence),
        _ => {}
    }
}

// Serialize to a counting sink, refusing before a second oversized document is allocated.
pub(super) fn evidence_limit(value: &impl serde::Serialize) -> ArtifactResult<()> {
    struct Limited(usize);
    impl std::io::Write for Limited {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self.0.checked_sub(bytes.len()).ok_or_else(|| {
                std::io::Error::other("evidence-size-limit: exceeds 1048576 bytes")
            })?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Limited(1 << 20), value).map_err(unrecorded)
}

fn range(offset: u64, size: u64, file_size: usize, label: &str) -> ArtifactResult<Range<u64>> {
    let end = offset
        .checked_add(size)
        .filter(|end| *end <= file_size as u64)
        .ok_or_else(|| unrecorded(format!("{label}: file range past EOF or overflow")))?;
    Ok(offset..end)
}

pub(super) fn validate(file: &Elf<'_>, bytes: &[u8]) -> ArtifactResult<Value> {
    let endian = file.endian();
    let mut loads = Vec::new();
    for p in file.elf_program_headers() {
        if p.p_type(endian) == elf::PT_LOAD {
            let r = range(
                p.p_offset(endian),
                p.p_filesz(endian),
                bytes.len(),
                "PT_LOAD",
            )?;
            if p.p_filesz(endian) > p.p_memsz(endian)
                || p.p_vaddr(endian).checked_add(p.p_memsz(endian)).is_none()
            {
                return Err(unrecorded("PT_LOAD invalid memory range"));
            }
            loads.push((p, r));
        }
    }
    let table = file.elf_section_table();
    let mut executable = Vec::new();
    let mut symbols = Budget::new(bytes.len(), 24, "symbols");
    let mut dynamic = Budget::new(bytes.len(), 16, "dynamic entries / DT_NEEDED");
    let mut notes = Budget::new(bytes.len(), 1, "section notes");
    for s in table.iter() {
        let kind = s.sh_type(endian);
        // Aggregate physical table sizes before any iterator can revisit aliased tables.
        let size = usize::try_from(s.sh_size(endian)).map_err(unrecorded)?;
        match kind {
            elf::SHT_SYMTAB | elf::SHT_DYNSYM => symbols.take(size / 24)?,
            elf::SHT_DYNAMIC => dynamic.take(size / 16)?,
            elf::SHT_NOTE => notes.take(size)?,
            _ => {}
        }
        if s.sh_flags(endian) & u64::from(elf::SHF_EXECINSTR) == 0 || kind == elf::SHT_NOBITS {
            continue;
        }
        let label =
            name(text(table.section_name(endian, s).map_err(unrecorded)?)?.as_str()).to_owned();
        let r = range(s.sh_offset(endian), s.sh_size(endian), bytes.len(), &label)?;
        if !loads.iter().any(|(p, load)| {
            p.p_flags(endian) & elf::PF_X != 0
                && r.start >= load.start
                && r.end <= load.end
                && s.sh_addr(endian).checked_sub(p.p_vaddr(endian))
                    == r.start.checked_sub(load.start)
        }) {
            return Err(unrecorded(format!(
                "executable section {label}: contradicts executable PT_LOAD mapping"
            )));
        }
        if !r.is_empty() {
            executable.push((r, label));
        }
    }
    executable.sort_unstable_by_key(|(r, _)| (r.start, r.end));
    for pair in executable.windows(2) {
        if pair[1].0.start < pair[0].0.end {
            return Err(unrecorded(format!(
                "executable section overlap: {} and {}",
                pair[0].1, pair[1].1
            )));
        }
    }
    let mut count = 0u64;
    let mut examples = Vec::new();
    for (p, load) in &loads {
        if p.p_flags(endian) & elf::PF_X == 0 {
            continue;
        }
        let first = executable.partition_point(|(r, _)| r.end <= load.start);
        let covered = &executable[first..];
        if covered.first().is_none_or(|(r, _)| r.start >= load.end) {
            return Err(unrecorded("executable PT_LOAD has no executable section; function boundaries and code bytes unknown"));
        }
        let mut cursor = load.start;
        for (r, _) in covered.iter().take_while(|(r, _)| r.start < load.end) {
            gap(cursor, r.start.max(load.start), &mut count, &mut examples);
            cursor = cursor.max(r.end.min(load.end));
        }
        gap(cursor, load.end, &mut count, &mut examples);
    }
    validate_dynamic(file, bytes)?;
    Ok(json!({"segment-uncovered": {"bytes": count, "examples": examples}}))
}

fn gap(start: u64, end: u64, count: &mut u64, examples: &mut Vec<Value>) {
    if start < end {
        *count += end - start;
        if examples.len() < 5 {
            examples.push(json!({"file_offset": start, "length": end - start}));
        }
    }
}

fn mapped_offset(file: &Elf<'_>, address: u64, size: u64) -> ArtifactResult<u64> {
    let e = file.endian();
    let mut offset = None;
    for p in file
        .elf_program_headers()
        .iter()
        .filter(|p| p.p_type(e) == elf::PT_LOAD)
    {
        if let Some(delta) = address.checked_sub(p.p_vaddr(e)) {
            if delta
                .checked_add(size)
                .is_some_and(|end| end <= p.p_filesz(e))
            {
                let found = p.p_offset(e) + delta; // PT_LOAD bounds already checked.
                if offset.replace(found).is_some_and(|old| old != found) {
                    return Err(unrecorded(
                        "dynamic address has contradictory PT_LOAD mappings",
                    ));
                }
            }
        }
    }
    offset.ok_or_else(|| unrecorded("dynamic address outside file-backed PT_LOAD"))
}

fn dynamic_entries(
    entries: &[elf::Dyn64<object::Endianness>],
    e: object::Endianness,
) -> ArtifactResult<Vec<(u32, u64)>> {
    let mut result = Vec::new();
    for d in entries {
        let tag = d.tag32(e);
        if tag == Some(elf::DT_NULL) {
            return Ok(result);
        }
        if let Some(
            tag @ (elf::DT_NEEDED
            | elf::DT_STRTAB
            | elf::DT_STRSZ
            | elf::DT_VERNEED
            | elf::DT_VERNEEDNUM),
        ) = tag
        {
            if tag != elf::DT_NEEDED && result.iter().any(|(old, _)| *old == tag) {
                return Err(unrecorded("duplicate dynamic metadata tag"));
            }
            result.push((tag, d.d_val(e)));
        }
    }
    Err(unrecorded("dynamic table lacks DT_NULL termination"))
}

fn validate_dynamic(file: &Elf<'_>, bytes: &[u8]) -> ArtifactResult<()> {
    let e = file.endian();
    let table = file.elf_section_table();
    let sections: Vec<_> = table
        .iter()
        .filter(|s| s.sh_type(e) == elf::SHT_DYNAMIC)
        .collect();
    let segments: Vec<_> = file
        .elf_program_headers()
        .iter()
        .filter(|p| p.p_type(e) == elf::PT_DYNAMIC)
        .collect();
    let versions: Vec<_> = table
        .iter()
        .filter(|s| s.sh_type(e) == elf::SHT_GNU_VERNEED)
        .collect();
    if sections.is_empty() && segments.is_empty() && versions.is_empty() {
        return Ok(());
    }
    if sections.len() != 1 || segments.len() != 1 || versions.len() > 1 {
        return Err(unrecorded(
            "PT_DYNAMIC / dynamic or version sections missing or duplicated",
        ));
    }
    let s = sections[0];
    let p = segments[0];
    let (section_entries, strings_index) = table
        .dynamic(e, bytes)
        .map_err(unrecorded)?
        .ok_or_else(|| unrecorded("dynamic section unreadable"))?;
    let segment_entries = p
        .dynamic(e, bytes)
        .map_err(unrecorded)?
        .ok_or_else(|| unrecorded("PT_DYNAMIC unreadable"))?;
    Budget::new(bytes.len(), 16, "PT_DYNAMIC entries").take(segment_entries.len())?;
    let entries = dynamic_entries(section_entries, e)?;
    if entries != dynamic_entries(segment_entries, e)?
        || s.sh_offset(e) != p.p_offset(e)
        || s.sh_size(e) != p.p_filesz(e)
        || mapped_offset(file, p.p_vaddr(e), p.p_filesz(e))? != s.sh_offset(e)
    {
        return Err(unrecorded("PT_DYNAMIC disagrees with dynamic section"));
    }
    let value = |tag| entries.iter().find(|(t, _)| *t == tag).map(|(_, v)| *v);
    let strings = table.section(strings_index).map_err(unrecorded)?;
    let strtab = value(elf::DT_STRTAB).ok_or_else(|| unrecorded("PT_DYNAMIC missing DT_STRTAB"))?;
    if value(elf::DT_STRSZ) != Some(strings.sh_size(e))
        || strtab != strings.sh_addr(e)
        || mapped_offset(file, strtab, strings.sh_size(e))? != strings.sh_offset(e)
    {
        return Err(unrecorded(
            "DT_STRTAB / DT_NEEDED disagrees with section string table",
        ));
    }
    if let Some(v) = versions.first() {
        let address =
            value(elf::DT_VERNEED).ok_or_else(|| unrecorded("PT_DYNAMIC missing DT_VERNEED"))?;
        if value(elf::DT_VERNEEDNUM) != Some(u64::from(v.sh_info(e)))
            || address != v.sh_addr(e)
            || mapped_offset(file, address, v.sh_size(e))? != v.sh_offset(e)
            || v.sh_link(e) != s.sh_link(e)
        {
            return Err(unrecorded(
                "DT_VERNEED / DT_VERNEEDNUM disagrees with version section",
            ));
        }
    } else if value(elf::DT_VERNEED).is_some() || value(elf::DT_VERNEEDNUM).is_some() {
        return Err(unrecorded("PT_DYNAMIC version needs missing from sections"));
    }
    Ok(())
}
