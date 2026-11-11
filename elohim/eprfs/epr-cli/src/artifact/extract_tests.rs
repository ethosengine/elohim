use super::*;
#[path = "extract_hardening_tests.rs"]
mod hardening;
#[path = "extract_ranges_tests.rs"]
mod ranges;
use crate::artifact::requires::{canonical, documents};
use host_contract::{admit, AxisValue, Provides, X86_64_V2, X86_64_V3};
use ranges::{add_symbols, frame_bytes};

fn names(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|s| (*s).into()).collect()
}

fn predicate<'a>(requires: &'a Requires, axis: &str) -> &'a Predicate {
    requires
        .clauses()
        .find(|clause| clause.axis().0 == axis)
        .unwrap()
        .predicate()
}

#[test]
fn artifact_isa_level_selection() {
    let v2 = names(X86_64_V2.features);
    let v3 = names(X86_64_V3.features);
    let mut v4 = v3.clone();
    v4.extend(names(&[
        "avx512f", "avx512bw", "avx512cd", "avx512dq", "avx512vl",
    ]));
    for (used, expected) in [
        (vec!["sse2", "cmov", "fpu"], None),
        (vec!["sse4_1"], Some(v2)),
        (vec!["avx2"], Some(v3)),
        (vec!["avx512f"], Some(v4.clone())),
        (vec!["aes"], Some(names(&["aes"]))),
        (
            vec!["avx512f", "aes"],
            Some({
                v4.insert("aes".into());
                v4
            }),
        ),
    ] {
        assert_eq!(isa::level_features(&names(&used)), expected, "{used:?}");
    }
}

#[test]
fn artifact_endbr_is_evidence_not_a_requirement() {
    for opcode in [0xfa, 0xfb] {
        // ENDBR64 and ENDBR32
        let mut bytes = fixture(
            elf::EM_X86_64,
            elf::ELFOSABI_LINUX,
            &[0xf3, 0x0f, 0x1e, opcode],
            None,
            None,
        );
        add_symbols(&mut bytes, &[(0, 4, "hint_target")], false);
        let result = extract(Path::new("hint.elf"), &bytes).unwrap();
        assert!(!result
            .requires
            .clauses()
            .any(|c| c.axis().0 == axis::ISA_X86));
        let scan = &result.evidence[axis::ISA_X86];
        assert_eq!(scan["instructions"], 1);
        assert!(scan["used"].as_object().unwrap().is_empty());
        assert_eq!(
            scan["notRequired"],
            json!([{
                "feature": "ibt", "reason": "executes as NOP where unsupported", "count": 1,
                "examples": [{"address": 0x400078, "section": ".text", "symbol": "hint_target"}]
            }])
        );
    }
}

#[test]
fn artifact_xgetbv_is_extra_only_below_osxsave_level() {
    for (suffix, expected) in [
        (&[0xc3][..], names(&["xsave"])),
        (
            &[0xc5, 0xfd, 0xef, 0xc0, 0xc3][..],
            names(X86_64_V3.features),
        ),
    ] {
        let code = [&[0x0f, 0x01, 0xd0][..], suffix].concat(); // XGETBV
        let bytes = fixture(elf::EM_X86_64, elf::ELFOSABI_LINUX, &code, None, None);
        let result = extract(Path::new("xgetbv.elf"), &bytes).unwrap();
        assert_eq!(
            predicate(&result.requires, axis::ISA_X86),
            &Predicate::Subset(expected)
        );
        assert_eq!(result.evidence[axis::ISA_X86]["used"]["xsave"]["count"], 1);
        assert_eq!(result.evidence[axis::ISA_X86]["notRequired"], json!([]));
    }
}

#[test]
#[cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]
fn artifact_elf_extracts_and_round_trips_against_v2() {
    let path = std::path::PathBuf::from("round-trip.elf");
    let mut bytes = with_dynamic(fixture(
        elf::EM_X86_64,
        0,
        &[0xc5, 0xfd, 0xef, 0xc0, 0xc3],
        Some("/lib64/ld-linux-x86-64.so.2"),
        Some([0, 3, 2, 0]),
    ));
    add_symbols(&mut bytes, &[(0, 5, "round_trip")], false);
    let extraction = extract(&path, &bytes).unwrap();
    assert_eq!(
        predicate(&extraction.requires, axis::ARCH),
        &Predicate::Equal("x86_64".into())
    );
    assert_eq!(
        predicate(&extraction.requires, axis::OS),
        &Predicate::Equal("linux".into())
    );
    assert!(
        matches!(predicate(&extraction.requires, axis::ABI_LIBC), Predicate::AtLeast(Version { line: version_line, components }) if version_line == line::GLIBC && components.len() >= 2)
    );
    assert!(
        matches!(predicate(&extraction.requires, axis::LIB), Predicate::Present(libs) if libs.contains("libc.so.6"))
    );
    assert!(
        matches!(predicate(&extraction.requires, axis::ABI_LOADER), Predicate::Present(loaders) if loaders.iter().any(|p| p.contains("ld-linux")))
    );
    let (document, evidence) = documents(&bytes, &path, None, false).unwrap();
    let (cid, bytes) = document.unwrap();
    assert_eq!(cid, eprfs_core::BlobCid::compute_bytes(&bytes));
    let decoded: Requires = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(decoded, extraction.requires);
    let originals: Vec<Clause> = decoded.clone().into();
    assert!(originals
        .iter()
        .all(|c| c.source == ClauseSource::Extracted && c.narrowing.is_none()));
    assert_eq!(canonical(&decoded).unwrap().1, bytes);
    let evidence: Value = serde_json::from_slice(&evidence.unwrap().1).unwrap();
    for feature in evidence[axis::ISA_X86]["used"]
        .as_object()
        .unwrap()
        .values()
    {
        let examples = feature["examples"].as_array().unwrap();
        assert!(!examples.is_empty() && examples.len() <= 3);
        assert!(examples.iter().all(|e| e["address"].as_u64().is_some()));
    }
    assert!(evidence[axis::ISA_X86]["used"]
        .as_object()
        .unwrap()
        .values()
        .any(|f| f["examples"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["symbol"].is_string())));
    assert_refused_by_v2(&decoded);
}

fn assert_refused_by_v2(requires: &Requires) {
    let vector = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../../crates/elohim-host-contract/tests/vectors/ethosengine-epr-on-synthetic-v2.json",
    );
    let vector: Value = serde_json::from_slice(&std::fs::read(vector).unwrap()).unwrap();
    let mut provides = Provides::default();
    for (axis, value) in vector["provides"]["observed"].as_object().unwrap() {
        provides.observe(
            AxisId(axis.clone()),
            serde_json::from_value::<AxisValue>(value.clone()).unwrap(),
        );
    }
    let verdict = serde_json::to_value(admit(requires, &provides)).unwrap();
    let text = serde_json::to_string(&verdict).unwrap();
    assert!(text.contains("refuse"), "{text}");
    assert!(
        text.contains(axis::ISA_X86) && text.contains("missing") && text.contains("avx2"),
        "{text}"
    );
}

// Minimal ELF64 fixture: no host compiler, symbol convention or filesystem lookup. Instructions
// and their ELF metadata are independently authored, not copied from the extraction tables.
fn fixture(
    machine: u16,
    osabi: u8,
    code: &[u8],
    interp: Option<&str>,
    abi: Option<[u32; 4]>,
) -> Vec<u8> {
    let mut bytes = vec![0u8; 64 + 56];
    bytes[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    bytes[7] = osabi;
    bytes[16..18].copy_from_slice(&elf::ET_EXEC.to_le_bytes());
    bytes[18..20].copy_from_slice(&machine.to_le_bytes());
    bytes[20..24].copy_from_slice(&1u32.to_le_bytes());
    bytes[52..54].copy_from_slice(&64u16.to_le_bytes());
    bytes[54..56].copy_from_slice(&56u16.to_le_bytes());
    bytes[58..60].copy_from_slice(&64u16.to_le_bytes());
    let strings = b"\0.text\0.note.ABI-tag\0.interp\0.shstrtab\0.eh_frame\0";
    let mut sections = vec![(0u32, 0u32, 0u64, 0usize, 0usize)];
    for (name, kind, flags, data) in [
        (1, elf::SHT_PROGBITS, 6, code.to_vec()),
        (
            7,
            elf::SHT_NOTE,
            2,
            abi.map(|tag| {
                let mut note = Vec::new();
                for word in [4u32, 16, elf::NT_GNU_ABI_TAG] {
                    note.extend(word.to_le_bytes());
                }
                note.extend(b"GNU\0");
                for word in tag {
                    note.extend(word.to_le_bytes());
                }
                note
            })
            .unwrap_or_default(),
        ),
        (
            21,
            elf::SHT_PROGBITS,
            2,
            interp
                .map(|s| [s.as_bytes(), b"\0"].concat())
                .unwrap_or_default(),
        ),
        (29, elf::SHT_STRTAB, 0, strings.to_vec()),
        (
            39,
            elf::SHT_PROGBITS,
            2,
            frame_bytes(0x400078, code.len() as u64),
        ),
    ] {
        while !bytes.len().is_multiple_of(8) {
            bytes.push(0);
        }
        let start = bytes.len();
        bytes.extend(&data);
        sections.push((name, kind, flags, start, data.len()));
    }
    if interp.is_some() {
        bytes[32..40].copy_from_slice(&64u64.to_le_bytes());
        bytes[56..58].copy_from_slice(&1u16.to_le_bytes());
        bytes[64..68].copy_from_slice(&elf::PT_INTERP.to_le_bytes());
        bytes[72..80].copy_from_slice(&(sections[3].3 as u64).to_le_bytes());
        bytes[96..104].copy_from_slice(&(sections[3].4 as u64).to_le_bytes());
    }
    while !bytes.len().is_multiple_of(8) {
        bytes.push(0);
    }
    // Keep code at 0x78; put the program table before the section headers.
    let mut programs = if interp.is_some() {
        bytes[64..120].to_vec()
    } else {
        Vec::new()
    };
    programs.extend(hardening::program(
        elf::PT_LOAD,
        elf::PF_R | elf::PF_X,
        0x78,
        code.len() as u64,
        0x400078,
    ));
    let phoff = bytes.len() as u64;
    bytes[32..40].copy_from_slice(&phoff.to_le_bytes());
    bytes[56..58].copy_from_slice(&((programs.len() / 56) as u16).to_le_bytes());
    bytes.extend(programs);
    let shoff = bytes.len() as u64;
    bytes[40..48].copy_from_slice(&shoff.to_le_bytes());
    bytes[60..62].copy_from_slice(&(sections.len() as u16).to_le_bytes());
    bytes[62..64].copy_from_slice(&4u16.to_le_bytes());
    for (name, kind, flags, start, size) in sections {
        let mut section = [0u8; 64];
        section[..4].copy_from_slice(&name.to_le_bytes());
        section[4..8].copy_from_slice(&kind.to_le_bytes());
        section[8..16].copy_from_slice(&flags.to_le_bytes());
        section[16..24].copy_from_slice(&(0x400000 + start as u64).to_le_bytes());
        section[24..32].copy_from_slice(&(start as u64).to_le_bytes());
        section[32..40].copy_from_slice(&(size as u64).to_le_bytes());
        section[48..56].copy_from_slice(&4u64.to_le_bytes());
        bytes.extend(section);
    }
    bytes
}

#[test]
fn artifact_static_baseline_musl_and_aarch64() {
    let path = Path::new("fixture.elf");
    let static_elf = fixture(elf::EM_X86_64, 0, &[0xc3], None, Some([0, 3, 2, 0]));
    let static_result = extract(path, &static_elf).unwrap();
    let axes: Vec<_> = static_result
        .requires
        .clauses()
        .map(|c| c.axis().0.as_str())
        .collect();
    assert_eq!(axes, [axis::ARCH, axis::KERNEL_LINUX, axis::OS]);
    let musl = fixture(
        elf::EM_X86_64,
        0,
        &[0xc3],
        Some("/lib/ld-musl-x86_64.so.1"),
        None,
    );
    let musl = extract(path, &musl).unwrap();
    assert_eq!(
        predicate(&musl.requires, axis::ABI_LIBC),
        &floor(line::MUSL, vec![])
    );
    assert!(!musl
        .requires
        .clauses()
        .any(|c| c.axis().0 == axis::KERNEL_LINUX));
    let arm = fixture(
        elf::EM_AARCH64,
        elf::ELFOSABI_LINUX,
        &[0xc0, 3, 0x5f, 0xd6],
        None,
        None,
    );
    let arm = extract(path, &arm).unwrap();
    assert_eq!(
        predicate(&arm.requires, axis::ARCH),
        &Predicate::Equal("aarch64".into())
    );
    assert!(!arm.requires.clauses().any(|c| c.axis().0 == axis::ISA_X86));
}

#[test]
fn artifact_instruction_bytes_raise_isa_without_symbols() {
    // vpxor ymm0,ymm0,ymm0; aesenc xmm0,xmm0; lahf; ret
    let elf = fixture(
        elf::EM_X86_64,
        elf::ELFOSABI_LINUX,
        &[
            0xc5, 0xfd, 0xef, 0xc0, 0x66, 0x0f, 0x38, 0xdc, 0xc0, 0x9f, 0xc3,
        ],
        None,
        None,
    );
    let result = extract(Path::new("stripped.elf"), &elf).unwrap();
    let Predicate::Subset(features) = predicate(&result.requires, axis::ISA_X86) else {
        panic!()
    };
    assert!(features.contains("avx2") && features.contains("aes") && features.contains("lahf_lm"));
    assert_eq!(result.evidence[axis::ISA_X86]["instructions"], 4);
    assert_eq!(
        result.evidence[axis::ISA_X86]["used"]["avx2"]["examples"][0]["address"],
        0x400078u64
    );
    assert_refused_by_v2(&result.requires);
}

#[test]
fn artifact_caller_only_tightens_and_never_forges_provenance() {
    let dir = tempfile::tempdir().unwrap();
    let caller = dir.path().join("requires.json");
    let bytes = fixture(elf::EM_X86_64, 0, &[0xc3], None, Some([0, 3, 2, 0]));
    let clause = |axis: &str, predicate: Predicate| json!([{ "axis": axis, "predicate": predicate, "source": "proven" }]);
    for (predicate, okay) in [
        (floor(line::LINUX, vec![4, 0]), true),
        (floor(line::LINUX, vec![3, 2]), true),
        (floor(line::LINUX, vec![2, 6]), false),
        (floor(line::GLIBC, vec![4, 0]), false),
    ] {
        std::fs::write(
            &caller,
            serde_json::to_vec(&clause(axis::KERNEL_LINUX, predicate.clone())).unwrap(),
        )
        .unwrap();
        let result = documents(&bytes, Path::new("output.elf"), Some(&caller), false);
        if okay {
            let requires: Requires = serde_json::from_slice(&result.unwrap().0.unwrap().1).unwrap();
            assert!(Vec::<Clause>::from(requires)
                .iter()
                .any(|c| c.source == ClauseSource::Declared && c.predicate == predicate));
        } else {
            assert!(result.unwrap_err().to_string().contains("MergeError"));
        }
    }
    std::fs::write(
        &caller,
        serde_json::to_vec(&clause(axis::ARCH, Predicate::Equal("aarch64".into()))).unwrap(),
    )
    .unwrap();
    assert!(
        documents(&bytes, Path::new("output.elf"), Some(&caller), false)
            .unwrap_err()
            .to_string()
            .contains("MergeError")
    );
    std::fs::write(&caller, format!(r#"[{{"axis":"{}","predicate":{{"kind":"at-least","value":{{"line":"{}","components":[1e0]}}}}}}]"#, axis::KERNEL_LINUX, line::LINUX)).unwrap();
    assert!(matches!(
        documents(&bytes, Path::new("output.elf"), Some(&caller), false),
        Err(super::super::ArtifactError::RequiresNumber { .. })
    ));
}

#[test]
fn artifact_unreadable_requirements_refuse_with_output_name() {
    let path = Path::new("bad-output");
    for bytes in [
        b"not ELF".to_vec(),
        fixture(elf::EM_RISCV, elf::ELFOSABI_LINUX, &[0xc3], None, None),
        fixture(elf::EM_X86_64, 0, &[0xc3], None, None),
        fixture(elf::EM_X86_64, elf::ELFOSABI_LINUX, &[0x0f], None, None),
    ] {
        let error = documents(&bytes, path, None, false)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("bad-output") && error.contains("Requires extraction"),
            "{error}"
        );
    }
    assert!(
        extract(path, &fixture(elf::EM_RISCV, 3, &[0xc3], None, None))
            .err()
            .unwrap()
            .to_string()
            .contains("e_machine 243")
    );
    // sysroot put preserves its non-ELF archive path and optional opaque caller document.
    assert!(documents(b"archive validated by sysroot", path, None, true)
        .unwrap()
        .0
        .is_none());
}

#[test]
fn artifact_measure_requested_elf() {
    let Some(path) = std::env::var_os("EPR_EXTRACT_MEASURE") else {
        return;
    };
    let path = std::path::PathBuf::from(path);
    let started = std::time::Instant::now();
    let bytes = std::fs::read(&path).unwrap();
    let result = extract(&path, &bytes);
    let elapsed = started.elapsed();
    let mut report =
        json!({"path": path, "bytes": bytes.len(), "elapsed_ms": elapsed.as_secs_f64() * 1000.0});
    match &result {
        Ok(result) => {
            let (cid, document) = canonical(&result.requires).unwrap();
            report["requires"] = serde_json::from_slice(&document).unwrap();
            report["requires_cid"] = json!(cid.to_string());
            report["requires_canonical_json"] = json!(std::str::from_utf8(&document).unwrap());
            report["evidence"] = json!(result.evidence);
            let recorded: Requires = serde_json::from_slice(&document).unwrap();
            for level in ["v2", "v3"] {
                let vector_path = format!("../../../crates/elohim-host-contract/tests/vectors/ethosengine-epr-on-synthetic-{level}.json");
                let vector: Value = serde_json::from_slice(
                    &std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join(&vector_path))
                        .unwrap(),
                )
                .unwrap();
                let mut provides = Provides::default();
                for (axis, value) in vector["provides"]["observed"].as_object().unwrap() {
                    provides.observe(
                        AxisId(axis.clone()),
                        serde_json::from_value::<AxisValue>(value.clone()).unwrap(),
                    );
                }
                report["admissions"][level] = json!({"vector": vector_path, "provides": vector["provides"], "verdict": admit(&recorded, &provides)});
            }
        }
        Err(error) => {
            report["requires"] = Value::Null;
            report["error"] = json!(error.to_string());
            let file = Elf::parse(&*bytes).unwrap();
            let audit = isa::scan_ranges(&file).unwrap();
            report["known_range_audit"] = json!(audit);
            report["known_range_isa_features"] =
                json!(isa::level_features(&audit.used.keys().cloned().collect()));
        }
    }
    if let Some(path) = std::env::var_os("EPR_EXTRACT_REPORT") {
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    result.unwrap(); // A refusal is still a failing measurement, even though its evidence is saved.
}

/// Add independently encoded dynamic and GNU version-needs sections. The higher GLIBC need
/// belongs to libm, so an unscoped version scan would incorrectly require 9.99 instead of 2.39.
fn with_dynamic(mut bytes: Vec<u8>) -> Vec<u8> {
    let old_offset = u64::from_le_bytes(bytes[40..48].try_into().unwrap()) as usize;
    let mut headers = bytes[old_offset..].to_vec();
    let mut strings = vec![0u8];
    let mut offset = |name: &str| {
        let offset = strings.len() as u32;
        strings.extend(name.as_bytes());
        strings.push(0);
        offset
    };
    let libc = offset("libc.so.6");
    let libm = offset("libm.so.6");
    let low = offset("GLIBC_2.9");
    let high = offset("GLIBC_2.39");
    let unrelated = offset("GLIBC_9.99");
    let strings_start = (bytes.len() + 7) & !7;
    let dynamic_start = (strings_start + strings.len() + 7) & !7;
    // 2 NEEDED, STRTAB, STRSZ, VERNEED, VERNEEDNUM, NULL.
    let needs_start = dynamic_start + 7 * 16;
    let mut dynamic = Vec::new();
    for library in [libc, libm] {
        dynamic.extend(u64::from(elf::DT_NEEDED).to_le_bytes());
        dynamic.extend(u64::from(library).to_le_bytes());
    }
    for (tag, value) in [
        (elf::DT_STRTAB, 0x400000 + strings_start as u64),
        (elf::DT_STRSZ, strings.len() as u64),
        (elf::DT_VERNEED, 0x400000 + needs_start as u64),
        (elf::DT_VERNEEDNUM, 2),
    ] {
        dynamic.extend(u64::from(tag).to_le_bytes());
        dynamic.extend(value.to_le_bytes());
    }
    dynamic.extend([0u8; 16]);
    let mut needs = Vec::new();
    for (library, versions, next) in [(libc, vec![low, high], 48u32), (libm, vec![unrelated], 0)] {
        needs.extend(1u16.to_le_bytes());
        needs.extend((versions.len() as u16).to_le_bytes());
        needs.extend(library.to_le_bytes());
        needs.extend(16u32.to_le_bytes());
        needs.extend(next.to_le_bytes());
        for (index, version) in versions.iter().enumerate() {
            needs.extend([0u8; 8]);
            needs.extend(version.to_le_bytes());
            needs.extend(
                if index + 1 == versions.len() {
                    0u32
                } else {
                    16
                }
                .to_le_bytes(),
            );
        }
    }
    for (kind, data, entry_size) in [
        (elf::SHT_STRTAB, strings, 0u64),
        (elf::SHT_DYNAMIC, dynamic, 16),
        (elf::SHT_GNU_VERNEED, needs, 0),
    ] {
        while !bytes.len().is_multiple_of(8) {
            bytes.push(0);
        }
        let start = bytes.len();
        bytes.extend(&data);
        let mut header = [0u8; 64];
        header[4..8].copy_from_slice(&kind.to_le_bytes());
        header[16..24].copy_from_slice(&(0x400000 + start as u64).to_le_bytes());
        if kind == elf::SHT_GNU_VERNEED {
            header[44..48].copy_from_slice(&2u32.to_le_bytes());
        }
        header[24..32].copy_from_slice(&(start as u64).to_le_bytes());
        header[32..40].copy_from_slice(&(data.len() as u64).to_le_bytes());
        header[40..44].copy_from_slice(&6u32.to_le_bytes());
        header[48..56].copy_from_slice(&8u64.to_le_bytes());
        header[56..64].copy_from_slice(&entry_size.to_le_bytes());
        headers.extend(header);
    }
    while !bytes.len().is_multiple_of(8) {
        bytes.push(0);
    }
    let old_ph = u64::from_le_bytes(bytes[32..40].try_into().unwrap()) as usize;
    let ph_count = u16::from_le_bytes(bytes[56..58].try_into().unwrap()) as usize;
    let mut programs = bytes[old_ph..old_ph + ph_count * 56].to_vec();
    programs.extend(hardening::program(
        elf::PT_LOAD,
        elf::PF_R,
        strings_start as u64,
        (bytes.len() - strings_start) as u64,
        0x400000 + strings_start as u64,
    ));
    programs.extend(hardening::program(
        elf::PT_DYNAMIC,
        elf::PF_R,
        dynamic_start as u64,
        7 * 16,
        0x400000 + dynamic_start as u64,
    ));
    let phoff = bytes.len() as u64;
    bytes[32..40].copy_from_slice(&phoff.to_le_bytes());
    bytes[56..58].copy_from_slice(&((programs.len() / 56) as u16).to_le_bytes());
    bytes.extend(programs);
    let start = bytes.len();
    bytes[40..48].copy_from_slice(&(start as u64).to_le_bytes());
    bytes[60..62].copy_from_slice(&9u16.to_le_bytes());
    bytes.extend(headers);
    bytes
}

#[test]
fn artifact_glibc_floor_is_numeric_and_scoped_to_libc() {
    let bytes = with_dynamic(fixture(
        elf::EM_X86_64,
        elf::ELFOSABI_LINUX,
        &[0xc3],
        Some("/lib64/ld-linux-x86-64.so.2"),
        None,
    ));
    let result = extract(Path::new("versioned.elf"), &bytes).unwrap();
    assert_eq!(
        predicate(&result.requires, axis::ABI_LIBC),
        &floor(line::GLIBC, vec![2, 39])
    );
    assert_eq!(
        predicate(&result.requires, axis::LIB),
        &Predicate::Present(names(&["libc.so.6", "libm.so.6"]))
    );
    assert_eq!(
        result.evidence[axis::ABI_LIBC]["GLIBC_versions"],
        json!(["GLIBC_2.39", "GLIBC_2.9"])
    );
    let mut malformed = bytes.clone();
    let position = malformed
        .windows(10)
        .position(|s| s == b"GLIBC_2.39")
        .unwrap();
    malformed[position + 6] = b'x';
    assert!(extract(Path::new("bad-version.elf"), &malformed)
        .err()
        .unwrap()
        .to_string()
        .contains("unsupported libc.so.6 version"));
}
