//! Hand-built ELF and frame fixtures: no assembler, compiler, or new dependencies.
use super::*;
use object::{ObjectSection, ObjectSymbol};

// DWARF32 CIE v1, absolute 64-bit PC, zR augmentation; then one FDE.
pub(super) fn frame_bytes(pc: u64, size: u64) -> Vec<u8> {
    let mut bytes = 13u32.to_le_bytes().to_vec();
    bytes.extend([0, 0, 0, 0, 1, b'z', b'R', 0, 1, 0x78, 16, 1, 0]);
    bytes.extend(21u32.to_le_bytes());
    bytes.extend(21u32.to_le_bytes()); // CIE pointer field offset, relative back to 0.
    bytes.extend(pc.to_le_bytes());
    bytes.extend(size.to_le_bytes());
    bytes.push(0); // FDE augmentation length
    bytes.extend(0u32.to_le_bytes());
    bytes
}

pub(super) fn section_offset(bytes: &[u8], index: usize) -> usize {
    u64::from_le_bytes(bytes[40..48].try_into().unwrap()) as usize + index * 64
}

pub(super) fn no_frames(bytes: &mut [u8]) {
    let frame = section_offset(bytes, 5);
    bytes[frame + 32..frame + 40].copy_from_slice(&0u64.to_le_bytes());
}

pub(super) fn add_symbols(bytes: &mut Vec<u8>, functions: &[(u64, u64, &str)], dynamic: bool) {
    let old = section_offset(bytes, 0);
    let count = u16::from_le_bytes(bytes[60..62].try_into().unwrap()) as usize;
    let mut headers = bytes[old..old + count * 64].to_vec();
    let count = headers.len() / 64;
    let mut strings = vec![0];
    let mut symbols = vec![0; 24];
    for (offset, size, name) in functions {
        symbols.extend((strings.len() as u32).to_le_bytes());
        symbols.extend([elf::STB_GLOBAL << 4 | elf::STT_FUNC, 0]);
        symbols.extend(1u16.to_le_bytes());
        symbols.extend((0x400078 + offset).to_le_bytes());
        symbols.extend(size.to_le_bytes());
        strings.extend(name.as_bytes());
        strings.push(0);
    }
    for (kind, data, link, entsize) in [
        (elf::SHT_STRTAB, strings, 0, 0u64),
        (
            if dynamic {
                elf::SHT_DYNSYM
            } else {
                elf::SHT_SYMTAB
            },
            symbols,
            count as u32,
            24,
        ),
    ] {
        let mut header = [0u8; 64];
        header[4..8].copy_from_slice(&kind.to_le_bytes());
        header[24..32].copy_from_slice(&(bytes.len() as u64).to_le_bytes());
        header[32..40].copy_from_slice(&(data.len() as u64).to_le_bytes());
        header[40..44].copy_from_slice(&link.to_le_bytes());
        header[44..48].copy_from_slice(&1u32.to_le_bytes());
        header[48..56].copy_from_slice(&8u64.to_le_bytes());
        header[56..64].copy_from_slice(&entsize.to_le_bytes());
        bytes.extend(data);
        headers.extend(header);
    }
    let offset = bytes.len() as u64;
    bytes[40..48].copy_from_slice(&offset.to_le_bytes());
    bytes[60..62].copy_from_slice(&((count + 2) as u16).to_le_bytes());
    bytes.extend(headers);
}

fn sized(code: &[u8], size: u64) -> Vec<u8> {
    let mut bytes = fixture(elf::EM_X86_64, elf::ELFOSABI_LINUX, code, None, None);
    no_frames(&mut bytes);
    add_symbols(&mut bytes, &[(0, size, "sized_function")], false);
    bytes
}

#[test]
fn artifact_data_after_sized_function_is_uncovered() {
    let bytes = sized(&[0xc3, 0x06, 0x0f, 0xff], 1);
    let result = extract(Path::new("data.elf"), &bytes).unwrap();
    let scan = &result.evidence[axis::ISA_X86];
    assert_eq!(scan["instructions"], 1);
    assert_eq!(
        scan["sections"][0],
        json!({
            "section": ".text", "total_bytes": 4, "covered_bytes": 1, "uncovered_bytes": 3,
            "uncovered_examples": [{"start": 0x400079, "length": 3, "nearest_preceding_symbol": "sized_function"}]
        })
    );
}

#[test]
fn artifact_invalid_or_truncated_instruction_names_containing_symbol() {
    for code in [&[0x06][..], &[0x0f][..]] {
        let error = extract(Path::new("invalid.elf"), &sized(code, 1))
            .err()
            .unwrap()
            .to_string();
        assert!(
            error.contains("sized_function")
                && error.contains("0x400078")
                && error.contains("invalid instruction"),
            "{error}"
        );
    }
    // A valid instruction extending into uncovered bytes must still refuse.
    assert!(extract(Path::new("truncated.elf"), &sized(&[0x0f, 0x1f, 0], 1)).is_err());
}

#[test]
fn artifact_unknown_function_boundaries_names_section() {
    let mut bytes = fixture(elf::EM_X86_64, elf::ELFOSABI_LINUX, &[0xc3], None, None);
    no_frames(&mut bytes);
    for symbols in [false, true] {
        if symbols {
            add_symbols(&mut bytes, &[(0, 0, "zero_size")], false);
        }
        let error = extract(Path::new("unknown.elf"), &bytes)
            .err()
            .unwrap()
            .to_string();
        assert!(
            error.contains(".text") && error.contains("function boundaries are unknown"),
            "{error}"
        );
    }
}

pub(super) fn rename_text(bytes: &mut Vec<u8>, name: &str) {
    let strings = section_offset(bytes, 4);
    let offset = bytes.len();
    bytes.extend([b"\0", name.as_bytes(), b"\0"].concat());
    bytes[strings + 24..strings + 32].copy_from_slice(&(offset as u64).to_le_bytes());
    bytes[strings + 32..strings + 40].copy_from_slice(&((name.len() + 2) as u64).to_le_bytes());
    // Only .text needs a name in this fixture; the other sections are not executable.
    for index in 2..6 {
        let header = section_offset(bytes, index);
        bytes[header..header + 4].fill(0);
    }
}

#[test]
fn artifact_abi_stubs_without_ranges_decode_whole_section() {
    for name in [".init", ".fini", ".plt", ".plt.got", ".plt.sec", ".iplt"] {
        // vpxor ymm0,ymm0,ymm0; ret. No symbols or FDE ranges.
        let mut bytes = fixture(
            elf::EM_X86_64,
            elf::ELFOSABI_LINUX,
            &[0xc5, 0xfd, 0xef, 0xc0, 0xc3],
            None,
            None,
        );
        no_frames(&mut bytes);
        rename_text(&mut bytes, name);
        let result = extract(Path::new("stub.elf"), &bytes).unwrap();
        assert_eq!(
            predicate(&result.requires, axis::ISA_X86),
            &Predicate::Subset(names(X86_64_V3.features))
        );
        let scan = &result.evidence[axis::ISA_X86];
        assert_eq!(scan["instructions"], 2);
        assert_eq!(scan["sections"][0]["covered_bytes"], 5);
        assert_eq!(scan["sections"][0]["uncovered_bytes"], 0);
        assert_eq!(scan["used"]["avx2"]["examples"][0]["section"], name);

        bytes[0x78] = 0x06; // Invalid in 64-bit mode, including ABI stubs.
        let error = extract(Path::new("invalid-stub.elf"), &bytes)
            .err()
            .unwrap()
            .to_string();
        assert!(
            error.contains(name) && error.contains("invalid instruction"),
            "{error}"
        );
    }
    // A similar name is not a policy exception.
    let mut bytes = fixture(elf::EM_X86_64, elf::ELFOSABI_LINUX, &[0xc3], None, None);
    no_frames(&mut bytes);
    rename_text(&mut bytes, ".plt.custom");
    assert!(extract(Path::new("custom.elf"), &bytes)
        .err()
        .unwrap()
        .to_string()
        .contains("function boundaries are unknown"));
}

#[test]
fn artifact_ranges_union_merges_overlaps_but_not_adjacent_functions() {
    let mut bytes = sized(&[0x90, 0xc3, 0x06], 1);
    // The FDE overlaps the symbol and extends it by one byte: decode the union once.
    let frame = section_offset(&bytes, 5);
    bytes[frame + 32..frame + 40].copy_from_slice(&46u64.to_le_bytes());
    let data = u64::from_le_bytes(bytes[frame + 24..frame + 32].try_into().unwrap()) as usize;
    bytes[data + 33..data + 41].copy_from_slice(&2u64.to_le_bytes());
    let result = extract(Path::new("union.elf"), &bytes).unwrap();
    assert_eq!(result.evidence[axis::ISA_X86]["instructions"], 2);
    assert_eq!(
        result.evidence[axis::ISA_X86]["sections"][0]["covered_bytes"],
        2
    );

    let mut adjacent = fixture(
        elf::EM_X86_64,
        elf::ELFOSABI_LINUX,
        &[0x0f, 0x1f, 0],
        None,
        None,
    );
    no_frames(&mut adjacent);
    add_symbols(&mut adjacent, &[(0, 1, "first"), (1, 2, "second")], false);
    assert!(extract(Path::new("adjacent.elf"), &adjacent).is_err());
}

#[test]
fn artifact_dynamic_symbols_are_fallback_only() {
    let mut bytes = fixture(
        elf::EM_X86_64,
        elf::ELFOSABI_LINUX,
        &[0xc3, 0x06],
        None,
        None,
    );
    no_frames(&mut bytes);
    add_symbols(&mut bytes, &[(0, 1, "dynamic_function")], true);
    assert!(extract(Path::new("dynamic.elf"), &bytes).is_ok());
    add_symbols(&mut bytes, &[(1, 1, "regular_function")], false);
    let error = extract(Path::new("regular.elf"), &bytes)
        .err()
        .unwrap()
        .to_string();
    assert!(error.contains("regular_function"), "{error}");
}

#[test]
fn artifact_fde_reader_covers_current_main() {
    let bytes = std::fs::read(std::env::current_exe().unwrap()).unwrap();
    let file = Elf::parse(&*bytes).unwrap();
    let main = file.symbols().find(|s| s.name() == Ok("main")).unwrap();
    let frames = super::super::frames::ranges(&file).unwrap();
    assert!(frames
        .iter()
        .any(|r| r.start <= main.address() && r.end >= main.address() + main.size()));
    assert!(!frames.is_empty());
    let section = file.section_by_name(".eh_frame").unwrap();
    assert!(section.size() > 0);
}

#[test]
fn artifact_fde_reader_rejects_malformed_records() {
    let bytes = frame_bytes(0x400078, 1);
    assert_eq!(
        super::super::frames::read(&bytes, 0).unwrap(),
        vec![0x400078..0x400079]
    );
    for end in 1..bytes.len() - 4 {
        // A complete CIE alone is valid; every other prefix cuts a record.
        if end == 17 {
            continue;
        }
        assert!(
            super::super::frames::read(&bytes[..end], 0).is_err(),
            "prefix {end}"
        );
    }
    let mut invalid = bytes.clone();
    invalid[21..25].copy_from_slice(&20u32.to_le_bytes());
    assert!(super::super::frames::read(&invalid, 0).is_err());
    let mut unsupported = bytes;
    unsupported[16] = 0x3b; // data-relative base is unspecified in .eh_frame alone.
    assert!(super::super::frames::read(&unsupported, 0).is_err());
}

#[test]
fn artifact_fde_reader_pc_relative_v3_and_dwarf64() {
    let mut bytes = 13u32.to_le_bytes().to_vec();
    bytes.extend([0, 0, 0, 0, 3, b'z', b'R', 0, 1, 0x78, 16, 1, 0x1b]);
    bytes.extend(13u32.to_le_bytes());
    bytes.extend(21u32.to_le_bytes());
    bytes.extend((-0x1019i32).to_le_bytes()); // field at 0x5019 -> PC 0x4000
    bytes.extend(16u32.to_le_bytes());
    bytes.push(0);
    assert_eq!(
        super::super::frames::read(&bytes, 0x5000).unwrap(),
        vec![0x4000..0x4010]
    );
    let mut bad = bytes.clone();
    bad[8] = 4; // Unsupported CIE version, not guessed from the .debug_frame layout.
    assert!(super::super::frames::read(&bad, 0x5000).is_err());
    let mut indirect = bytes;
    indirect[16] = 0x9b;
    assert!(super::super::frames::read(&indirect, 0x5000).is_err());

    let mut wide = u32::MAX.to_le_bytes().to_vec();
    wide.extend(13u64.to_le_bytes());
    wide.extend(0u32.to_le_bytes());
    wide.extend([3, b'z', b'R', 0, 1, 0x78, 16, 1, 0]);
    wide.extend(u32::MAX.to_le_bytes());
    wide.extend(21u64.to_le_bytes());
    wide.extend(37u32.to_le_bytes());
    wide.extend(0x4000u64.to_le_bytes());
    wide.extend(16u64.to_le_bytes());
    wide.push(0);
    assert_eq!(
        super::super::frames::read(&wide, 0).unwrap(),
        vec![0x4000..0x4010]
    );
    let end = wide.len();
    wide[end - 9..end - 1].copy_from_slice(&u64::MAX.to_le_bytes());
    assert!(super::super::frames::read(&wide, 0).is_err());
}

#[test]
fn artifact_inline_table_fallback_counts_avx2_and_unreached_bytes() {
    // nop; jmp over a four-byte relative offset; vpxor ymm0,ymm0,ymm0; ret; data.
    let code = [0x90, 0xeb, 4, 6, 0, 0, 0, 0xc5, 0xfd, 0xef, 0xc0, 0xc3, 6];
    let result = extract(Path::new("inline.elf"), &sized(&code, code.len() as u64)).unwrap();
    assert_eq!(
        predicate(&result.requires, axis::ISA_X86),
        &Predicate::Subset(names(X86_64_V3.features))
    );
    let scan = &result.evidence[axis::ISA_X86];
    assert_eq!(scan["instructions"], 4);
    assert_eq!(scan["used"]["avx2"]["count"], 1);
    assert_eq!(scan["used"]["osxsave"]["count"], 1);
    assert_eq!(
        scan["partialFunctions"],
        json!({
            "count": 1, "unreachedBytes": 5,
            "examples": [{"symbol": "sized_function", "rangeStart": 0x400078,
                "rangeLength": 13, "decodedBytes": 8, "unreachedBytes": 5}]
        })
    );
    assert_eq!(scan["sections"][0]["covered_bytes"], 8);
    assert_eq!(scan["sections"][0]["uncovered_bytes"], 5);
    assert_eq!(
        scan["sections"][0]["uncovered_examples"],
        json!([
            {"start": 0x40007b, "length": 4, "nearest_preceding_symbol": "sized_function"},
            {"start": 0x400084, "length": 1, "nearest_preceding_symbol": "sized_function"}
        ])
    );
}

#[test]
fn artifact_fallback_reached_invalid_target_still_refuses() {
    // The first invalid byte triggers fallback; the jump reaches the second one.
    let code = [0xeb, 1, 6, 6];
    let error = extract(Path::new("reached.elf"), &sized(&code, 4))
        .err()
        .unwrap()
        .to_string();
    assert!(
        error.contains("sized_function") && error.contains("0x40007b"),
        "{error}"
    );
}

#[test]
fn artifact_fallback_xbegin_abort_target_counts_avx2_rel16_and_rel32() {
    for xbegin in [&[0x66, 0xc7, 0xf8, 5, 0][..], &[0xc7, 0xf8, 5, 0, 0, 0]] {
        // XBEGIN +5; XABORT 0; ret; invalid data; abort target: AVX2; ret.
        let mut code = xbegin.to_vec();
        code.extend([0xc6, 0xf8, 0, 0xc3, 6, 0xc5, 0xfd, 0xef, 0xc0, 0xc3]);
        let result = extract(Path::new("xbegin.elf"), &sized(&code, code.len() as u64)).unwrap();
        let mut required = names(X86_64_V3.features);
        required.insert("rtm".into());
        assert_eq!(
            predicate(&result.requires, axis::ISA_X86),
            &Predicate::Subset(required)
        );
        let scan = &result.evidence[axis::ISA_X86];
        for feature in ["avx2", "avx", "osxsave"] {
            assert_eq!(scan["used"][feature]["count"], 1, "{feature}");
        }
        assert_eq!(scan["used"]["rtm"]["count"], 2); // XBEGIN and fallthrough XABORT
        assert_eq!(scan["instructions"], 5);
        assert_eq!(scan["partialFunctions"]["unreachedBytes"], 1);
        assert_eq!(scan["sections"][0]["covered_bytes"], code.len() - 1);
    }
}

#[test]
fn artifact_fallback_xbegin_invalid_abort_target_refuses_rel16_and_rel32() {
    for xbegin in [&[0x66, 0xc7, 0xf8, 5, 0][..], &[0xc7, 0xf8, 5, 0, 0, 0]] {
        let mut code = xbegin.to_vec();
        code.extend([0xc6, 0xf8, 0, 0xc3, 6, 6]);
        let error = extract(
            Path::new("xbegin-invalid.elf"),
            &sized(&code, code.len() as u64),
        )
        .err()
        .unwrap()
        .to_string();
        let target = format!("0x{:x}", 0x400078 + xbegin.len() + 5);
        assert!(
            error.contains("sized_function") && error.contains(&target),
            "{error}"
        );
    }
}

#[test]
fn artifact_fallback_interrupt_return_continuation_is_reached() {
    // INT imm8 and INT1 can return; unlike the explicit INT3 stop, keep their continuation.
    for interrupt in [&[0xcd, 0x80][..], &[0xf1]] {
        let mut code = interrupt.to_vec();
        code.extend([0xc5, 0xfd, 0xef, 0xc0, 0xc3, 6]);
        let result = extract(Path::new("interrupt.elf"), &sized(&code, code.len() as u64)).unwrap();
        assert_eq!(result.evidence[axis::ISA_X86]["used"]["avx2"]["count"], 1);
    }
}

#[test]
fn artifact_clean_linear_function_keeps_unreachable_features() {
    // Even code after ret stays counted when the entire range decodes linearly.
    let code = [0xc3, 0xc5, 0xfd, 0xef, 0xc0];
    let result = extract(Path::new("linear.elf"), &sized(&code, 5)).unwrap();
    let scan = &result.evidence[axis::ISA_X86];
    assert_eq!(
        scan["partialFunctions"],
        json!({"count": 0, "unreachedBytes": 0, "examples": []})
    );
    assert_eq!(scan["instructions"], 2);
    assert_eq!(scan["sections"][0]["covered_bytes"], 5);
    assert_eq!(scan["used"]["avx2"]["count"], 1);
}

#[test]
fn artifact_stub_invalid_after_return_never_falls_back() {
    for name in [".init", ".fini", ".plt", ".plt.got", ".plt.sec", ".iplt"] {
        let mut bytes = sized(&[0xc3, 6], 2);
        rename_text(&mut bytes, name);
        let error = extract(Path::new("stub.elf"), &bytes)
            .err()
            .unwrap()
            .to_string();
        assert!(
            error.contains(name) && error.contains("0x400079"),
            "{error}"
        );
    }
}

#[test]
fn artifact_fallback_self_loops_and_many_branches_are_bounded() {
    let mut code = [0x75, 0xfe].repeat(4096); // jne self, with fallthrough to next branch
    code.extend([0xeb, 0xfe, 6]); // jmp self; invalid unreachable data
    let result = extract(Path::new("loops.elf"), &sized(&code, code.len() as u64)).unwrap();
    let scan = &result.evidence[axis::ISA_X86];
    assert_eq!(scan["instructions"], 4097);
    assert_eq!(scan["partialFunctions"]["unreachedBytes"], 1);
    assert_eq!(scan["sections"][0]["covered_bytes"], 8194);
}

#[test]
fn artifact_fallback_stops_at_each_terminator() {
    for terminator in [
        &[0xc3][..],
        &[0x0f, 0x0b],       // UD2
        &[0x0f, 0xff, 0xc0], // UD0
        &[0x0f, 0xb9, 0xc0], // UD1
        &[0xcc],
        &[0xf4],
        &[0xff, 0xe0],
        &[0xff, 0xd0],
    ] {
        let mut code = terminator.to_vec();
        code.push(6);
        let result = extract(Path::new("stop.elf"), &sized(&code, code.len() as u64)).unwrap();
        let scan = &result.evidence[axis::ISA_X86];
        assert_eq!(scan["instructions"], 1);
        assert_eq!(scan["partialFunctions"]["unreachedBytes"], 1);
    }
}

#[test]
fn artifact_fallback_follows_direct_calls_and_their_fallthrough() {
    // call +2; ENDBR at target; ret in fallthrough; invalid data separates paths.
    let code = [0xe8, 2, 0, 0, 0, 0xc3, 6, 0xf3, 0x0f, 0x1e, 0xfa, 0xc3];
    let result = extract(Path::new("call.elf"), &sized(&code, code.len() as u64)).unwrap();
    let scan = &result.evidence[axis::ISA_X86];
    assert_eq!(scan["instructions"], 4);
    assert_eq!(scan["notRequired"][0]["feature"], "ibt");
    assert_eq!(scan["notRequired"][0]["count"], 1);
    assert_eq!(scan["partialFunctions"]["unreachedBytes"], 1);
}

#[test]
fn artifact_fallback_discards_features_from_failed_linear_attempt() {
    let code = [0xeb, 5, 0xc5, 0xfd, 0xef, 0xc0, 6, 0xc3];
    let result = extract(Path::new("discard.elf"), &sized(&code, 8)).unwrap();
    assert!(result.evidence[axis::ISA_X86]["used"]
        .as_object()
        .unwrap()
        .is_empty());
    assert_eq!(
        result.evidence[axis::ISA_X86]["partialFunctions"]["unreachedBytes"],
        5
    );
}

#[test]
fn artifact_partial_function_evidence_is_bounded() {
    let mut bytes = fixture(
        elf::EM_X86_64,
        elf::ELFOSABI_LINUX,
        &[0xc3, 6].repeat(7),
        None,
        None,
    );
    no_frames(&mut bytes);
    let name = "é".repeat(200);
    let functions: Vec<_> = (0..7).map(|i| (i * 2, 2, name.as_str())).collect();
    add_symbols(&mut bytes, &functions, false);
    let result = extract(Path::new("bounded.elf"), &bytes).unwrap();
    let partial = &result.evidence[axis::ISA_X86]["partialFunctions"];
    assert_eq!(partial["count"], 7);
    assert_eq!(partial["unreachedBytes"], 7);
    assert_eq!(partial["examples"].as_array().unwrap().len(), 5);
    assert_eq!(
        partial["examples"][0]["symbol"].as_str().unwrap().len(),
        256
    );
}
