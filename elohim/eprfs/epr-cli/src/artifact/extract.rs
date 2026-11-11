//! Byte-only ELF requirements and reconstructable, local extraction evidence (K8).
//! No host observations and no dispatch exemptions: those belong to A2 and K10.
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use host_contract::{
    names::{axis, line},
    AxisId, Clause, ClauseSource, Predicate, Requires, Version,
};
use object::read::elf::{Dyn, ElfFile64, FileHeader, NoteIterator, ProgramHeader, SectionHeader};
use object::{elf, Endianness, Object};
use serde_json::{json, Value};

use super::{unrecorded, ArtifactResult};

#[path = "extract_frames.rs"]
mod frames;
#[path = "extract_isa.rs"]
mod isa;
#[path = "extract_layout.rs"]
mod layout;
#[cfg(test)]
#[path = "extract_tests.rs"]
mod tests;
#[path = "extract_versions.rs"]
mod versions;

type Elf<'a> = ElfFile64<'a, Endianness>;

pub(super) struct Extraction {
    pub requires: Requires,
    pub evidence: BTreeMap<String, Value>,
}

impl Extraction {
    fn insert(
        &mut self,
        axis: &str,
        predicate: Predicate,
        mut evidence: Value,
    ) -> ArtifactResult<()> {
        layout::trim_evidence(&mut evidence);
        self.requires
            .insert(Clause {
                axis: AxisId(axis.into()),
                predicate,
                source: ClauseSource::Extracted,
                narrowing: None,
            })
            .map_err(|e| unrecorded(format!("MergeError: {e}")))?;
        self.evidence.insert(axis.into(), evidence);
        Ok(())
    }
}

/// All parse failures name the output before the recording tail can store anything.
pub(super) fn extract(path: &Path, bytes: &[u8]) -> ArtifactResult<Extraction> {
    extract_elf(bytes)
        .map_err(|e| unrecorded(format!("{}: Requires extraction: {e}", path.display())))
}

fn extract_elf(bytes: &[u8]) -> ArtifactResult<Extraction> {
    if object::FileKind::parse(bytes).map_err(unrecorded)? == object::FileKind::Elf32 {
        let file = object::read::elf::ElfFile32::<Endianness>::parse(bytes).map_err(unrecorded)?;
        return Err(unrecorded(format!(
            "unsupported ELF32 e_machine {}",
            file.elf_header().e_machine(file.endian())
        )));
    }
    let file = Elf::parse(bytes).map_err(unrecorded)?;
    let endian = file.endian();
    let header = file.elf_header();
    let machine = header.e_machine(endian);
    let arch = match machine {
        elf::EM_X86_64 => "x86_64",
        elf::EM_AARCH64 => "aarch64",
        _ => return Err(unrecorded(format!("unsupported e_machine {machine}"))),
    };
    if !matches!(header.e_type(endian), elf::ET_EXEC | elf::ET_DYN) {
        return Err(unrecorded("ELF is not an executable or shared output"));
    }
    if !file.is_little_endian() {
        return Err(unrecorded(
            "big-endian ELF unsupported; byte order is not yet an axis",
        ));
    }
    // Without section metadata we cannot prove that we decoded all executable sections or
    // read the version needs. Refuse instead of interpreting missing tables as empty needs.
    if file.elf_section_table().is_empty() {
        return Err(unrecorded(
            "ELF section table missing; executable sections and version needs unreadable",
        ));
    }
    let loader_evidence = layout::validate(&file, bytes)?;
    let mut result = Extraction {
        requires: Requires::default(),
        evidence: BTreeMap::new(),
    };
    result
        .evidence
        .insert("elf.loader-view".into(), loader_evidence);
    result.insert(
        axis::ARCH,
        Predicate::Equal(arch.into()),
        json!({"e_machine": machine}),
    )?;
    let mut loader = None;
    let mut abi_tags = BTreeSet::new();
    let mut dynamic_segment = false;
    for segment in file.elf_program_headers() {
        if let Some(interpreter) = segment.interpreter(endian, bytes).map_err(unrecorded)? {
            let name = text(interpreter)?;
            if name.is_empty() || loader.replace(name).is_some() {
                return Err(unrecorded("empty or duplicate PT_INTERP"));
            }
        }
        dynamic_segment |= segment.p_type(endian) == elf::PT_DYNAMIC;
        if let Some(notes) = segment.notes(endian, bytes).map_err(unrecorded)? {
            read_tags(notes, endian, &mut abi_tags)?;
        }
    }
    for section in file.elf_section_table().iter() {
        if let Some(notes) = section.notes(endian, bytes).map_err(unrecorded)? {
            read_tags(notes, endian, &mut abi_tags)?;
        }
    }
    if abi_tags.len() > 1 {
        return Err(unrecorded("conflicting NT_GNU_ABI_TAG notes"));
    }
    let abi_tag = abi_tags.first();
    let osabi = header.e_ident().os_abi;
    let os = if let Some(tag) = abi_tag {
        match tag[0] {
            0 => "linux",
            1 => "hurd",
            2 => "solaris",
            3 => "freebsd",
            value => return Err(unrecorded(format!("unknown NT_GNU_ABI_TAG OS {value}"))),
        }
    } else {
        os_from_loader(osabi, loader.as_deref())?
    };
    result.insert(
        axis::OS,
        Predicate::Equal(os.into()),
        json!({
            "NT_GNU_ABI_TAG": abi_tag, "ELF_OSABI": osabi, "PT_INTERP": loader,
        }),
    )?;
    if let Some(tag) = abi_tag.filter(|tag| tag[0] == 0) {
        result.insert(
            axis::KERNEL_LINUX,
            floor(line::LINUX, tag[1..].to_vec()),
            json!({"NT_GNU_ABI_TAG": tag}),
        )?;
    }
    if let Some(loader) = &loader {
        result.insert(
            axis::ABI_LOADER,
            Predicate::Present([loader.clone()].into()),
            json!({"PT_INTERP": loader}),
        )?;
    }
    let needed = needed_libraries(&file, bytes, dynamic_segment)?;
    if !needed.is_empty() {
        result.insert(
            axis::LIB,
            Predicate::Present(needed.clone()),
            json!({"DT_NEEDED": needed}),
        )?;
    }
    let glibc = versions::glibc_versions(&file, bytes)?;
    let musl = loader.as_deref().is_some_and(|name| {
        name.rsplit('/')
            .next()
            .unwrap_or(name)
            .starts_with("ld-musl-")
    });
    if musl {
        if !glibc.is_empty() {
            return Err(unrecorded(
                "musl PT_INTERP conflicts with GLIBC version needs on libc.so.6",
            ));
        }
        // Empty components mean the musl line with no provable version floor, not a host version.
        result.insert(
            axis::ABI_LIBC,
            floor(line::MUSL, vec![]),
            json!({
                "PT_INTERP": loader, "version_floor": null,
                "reason": "musl loader identifies the line, not a version",
            }),
        )?;
    } else if let Some(version) = glibc.iter().max_by(|a, b| a.1.cmp(b.1)) {
        result.insert(
            axis::ABI_LIBC,
            floor(line::GLIBC, version.1.clone()),
            json!({
                "library": "libc.so.6", "GLIBC_versions": glibc.keys().collect::<Vec<_>>(),
            }),
        )?;
    } else if needed.contains("libc.so.6") {
        return Err(unrecorded(
            "libc.so.6 has no readable GLIBC_x.y version needs; abi.libc floor unknown",
        ));
    }
    if machine == elf::EM_X86_64 {
        let scan = isa::scan(&file)?;
        if let Some(features) = isa::level_features(&scan.used.keys().cloned().collect()) {
            result.insert(
                axis::ISA_X86,
                Predicate::Subset(features),
                serde_json::to_value(&scan)?,
            )?;
        } else {
            result
                .evidence
                .insert(axis::ISA_X86.into(), serde_json::to_value(&scan)?);
        }
    }
    for value in result.evidence.values_mut() {
        layout::trim_evidence(value);
    }
    layout::evidence_limit(&result.evidence)?;
    Ok(result)
}

fn text(bytes: &[u8]) -> ArtifactResult<String> {
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(unrecorded)
}

fn floor(line: &str, components: Vec<u32>) -> Predicate {
    Predicate::AtLeast(Version {
        line: line.into(),
        components,
    })
}

fn read_tags(
    mut notes: NoteIterator<'_, elf::FileHeader64<Endianness>>,
    endian: Endianness,
    tags: &mut BTreeSet<[u32; 4]>,
) -> ArtifactResult<()> {
    while let Some(note) = notes.next().map_err(unrecorded)? {
        if note.name() != b"GNU" || note.n_type(endian) != elf::NT_GNU_ABI_TAG {
            continue;
        }
        if note.desc().len() != 16 {
            return Err(unrecorded("invalid NT_GNU_ABI_TAG size"));
        }
        let mut tag = [0; 4];
        for (value, bytes) in tag.iter_mut().zip(note.desc().chunks_exact(4)) {
            let bytes = <[u8; 4]>::try_from(bytes).map_err(unrecorded)?;
            *value = match endian {
                Endianness::Little => u32::from_le_bytes(bytes),
                Endianness::Big => u32::from_be_bytes(bytes),
            };
        }
        tags.insert(tag);
    }
    Ok(())
}

fn os_from_loader(osabi: u8, loader: Option<&str>) -> ArtifactResult<&'static str> {
    let from_loader = loader.and_then(|name| {
        let base = name.rsplit('/').next().unwrap_or(name);
        if base.starts_with("ld-linux-") || base.starts_with("ld-musl-") || base == "ld-linux.so.2"
        {
            Some("linux")
        } else if base == "ld-elf.so.1" {
            Some("freebsd")
        } else {
            None
        }
    });
    let from_abi = match osabi {
        elf::ELFOSABI_NONE => None,
        elf::ELFOSABI_LINUX => Some("linux"),
        elf::ELFOSABI_FREEBSD => Some("freebsd"),
        elf::ELFOSABI_NETBSD => Some("netbsd"),
        elf::ELFOSABI_OPENBSD => Some("openbsd"),
        elf::ELFOSABI_SOLARIS => Some("solaris"),
        _ => return Err(unrecorded(format!("unsupported ELF OSABI {osabi}"))),
    };
    if from_loader.is_some() && from_abi.is_some() && from_loader != from_abi {
        return Err(unrecorded("PT_INTERP and ELF OSABI disagree"));
    }
    from_loader.or(from_abi).ok_or_else(|| {
        unrecorded(format!(
            "os unknown: no NT_GNU_ABI_TAG, ELF OSABI {osabi}, PT_INTERP {loader:?}"
        ))
    })
}

fn needed_libraries(
    file: &Elf<'_>,
    bytes: &[u8],
    dynamic_segment: bool,
) -> ArtifactResult<BTreeSet<String>> {
    let table = file.elf_section_table();
    let mut needed = BTreeSet::new();
    if let Some((dynamic, strings)) = table.dynamic(file.endian(), bytes).map_err(unrecorded)? {
        let strings = table
            .strings(file.endian(), bytes, strings)
            .map_err(unrecorded)?;
        for entry in dynamic {
            if entry.tag32(file.endian()) == Some(elf::DT_NULL) {
                break;
            }
            if entry.tag32(file.endian()) == Some(elf::DT_NEEDED) {
                needed.insert(text(
                    entry.string(file.endian(), strings).map_err(unrecorded)?,
                )?);
            }
        }
    } else if dynamic_segment {
        return Err(unrecorded(
            "PT_DYNAMIC present but dynamic section unreadable; DT_NEEDED unknown",
        ));
    }
    Ok(needed)
}
