//! Hostile ELF regressions from K8 review, with independent byte-level mutations.
use super::*;
use ranges::{no_frames, rename_text, section_offset};

pub(super) fn program(kind: u32, flags: u32, offset: u64, size: u64, address: u64) -> [u8; 56] {
    let mut p = [0; 56];
    p[..4].copy_from_slice(&kind.to_le_bytes());
    p[4..8].copy_from_slice(&flags.to_le_bytes());
    p[8..16].copy_from_slice(&offset.to_le_bytes());
    p[16..24].copy_from_slice(&address.to_le_bytes());
    p[32..40].copy_from_slice(&size.to_le_bytes());
    p[40..48].copy_from_slice(&size.to_le_bytes());
    p[48..56].copy_from_slice(&1u64.to_le_bytes());
    p
}

fn refusal(bytes: &[u8], reason: &str) {
    let error = extract(Path::new("hostile.elf"), bytes)
        .err()
        .expect("must refuse")
        .to_string();
    assert!(error.contains(reason), "wanted {reason}: {error}");
}

fn phoff(bytes: &[u8]) -> usize {
    u64::from_le_bytes(bytes[32..40].try_into().unwrap()) as usize
}
fn data_offset(bytes: &[u8], index: usize) -> usize {
    let s = section_offset(bytes, index);
    u64::from_le_bytes(bytes[s + 24..s + 32].try_into().unwrap()) as usize
}

#[test]
fn artifact_overlapping_rx_loads_refuse_before_stub_decode() {
    // Two different file pages replace the same virtual page. The two baseline
    // stubs describe complete sections, but the later map puts AVX2 at the entry.
    let mut bytes = vec![0; 8960];
    bytes[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    bytes[7] = 3;
    bytes[16..18].copy_from_slice(&elf::ET_EXEC.to_le_bytes());
    bytes[18..20].copy_from_slice(&elf::EM_X86_64.to_le_bytes());
    bytes[20..24].copy_from_slice(&1u32.to_le_bytes());
    bytes[24..32].copy_from_slice(&0x400200u64.to_le_bytes());
    bytes[32..40].copy_from_slice(&64u64.to_le_bytes());
    bytes[40..48].copy_from_slice(&8704u64.to_le_bytes());
    for (offset, value) in [(52, 64u16), (54, 56), (56, 2), (58, 64), (60, 4), (62, 3)] {
        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }
    bytes[64..120].copy_from_slice(&program(elf::PT_LOAD, 5, 0, 4096, 0x400000));
    bytes[120..176].copy_from_slice(&program(elf::PT_LOAD, 5, 4096, 4096, 0x400000));
    bytes[512] = 0xc3;
    bytes[4608..4612].copy_from_slice(&[0xc5, 0xfd, 0xef, 0xc0]);
    bytes[4624] = 0xc3;
    bytes[8192..8217].copy_from_slice(b"\0.plt\0.plt.got\0.shstrtab\0");
    for (index, name, address, offset) in [(1, 1u32, 0x400200u64, 512u64), (2, 6, 0x400210, 4624)] {
        let s = 8704 + index * 64;
        bytes[s..s + 4].copy_from_slice(&name.to_le_bytes());
        bytes[s + 4..s + 8].copy_from_slice(&elf::SHT_PROGBITS.to_le_bytes());
        bytes[s + 8..s + 16].copy_from_slice(&6u64.to_le_bytes());
        bytes[s + 16..s + 24].copy_from_slice(&address.to_le_bytes());
        bytes[s + 24..s + 32].copy_from_slice(&offset.to_le_bytes());
        bytes[s + 32..s + 40].copy_from_slice(&1u64.to_le_bytes());
    }
    let s = 8896;
    bytes[s..s + 4].copy_from_slice(&15u32.to_le_bytes());
    bytes[s + 4..s + 8].copy_from_slice(&elf::SHT_STRTAB.to_le_bytes());
    bytes[s + 24..s + 32].copy_from_slice(&8192u64.to_le_bytes());
    bytes[s + 32..s + 40].copy_from_slice(&25u64.to_le_bytes());
    let mut disjoint = bytes.clone();
    disjoint[136..144].copy_from_slice(&0x401000u64.to_le_bytes());
    disjoint[8848..8856].copy_from_slice(&0x401210u64.to_le_bytes());
    let result = extract(Path::new("disjoint.elf"), &disjoint).unwrap();
    assert_eq!(result.requires.clauses().count(), 2);
    refusal(
        &bytes,
        "page-rounded virtual overlap: PT_LOAD[0] and PT_LOAD[1]",
    );

    // Disjoint byte ranges still share a rounded page, including RX/R and R/RX.
    for align in [0u64, 1, 4096, 8192] {
        for (first_flags, second_flags) in [(5u32, 5u32), (5, 4), (4, 5)] {
            let mut bad = bytes.clone();
            for (p, flags, address) in [
                (64, first_flags, 0x400200u64),
                (120, second_flags, 0x400800),
            ] {
                bad[p + 4..p + 8].copy_from_slice(&flags.to_le_bytes());
                bad[p + 16..p + 24].copy_from_slice(&address.to_le_bytes());
                bad[p + 32..p + 40].copy_from_slice(&4u64.to_le_bytes());
                bad[p + 40..p + 48].copy_from_slice(&4u64.to_le_bytes());
                bad[p + 48..p + 56].copy_from_slice(&align.to_le_bytes());
            }
            refusal(
                &bad,
                "page-rounded virtual overlap: PT_LOAD[0] and PT_LOAD[1]",
            );
        }
    }
}

#[test]
fn artifact_thousand_aliased_program_notes_exhaust_shared_budget() {
    let mut bytes = fixture(elf::EM_X86_64, 3, &[0xc3], None, None);
    let p = phoff(&bytes);
    let load = bytes[p..p + 56].to_vec();
    let note_offset = bytes.len() as u64;
    // 10,000 valid 16-byte notes: the old path revisited these ten million times.
    for _ in 0..10_000 {
        bytes.extend([4u32, 0, 0].into_iter().flat_map(u32::to_le_bytes));
        bytes.extend(b"GNU\0");
    }
    let note = program(elf::PT_NOTE, 4, note_offset, 160_000, 0);
    let new_ph = bytes.len() as u64;
    bytes[32..40].copy_from_slice(&new_ph.to_le_bytes());
    bytes[56..58].copy_from_slice(&1001u16.to_le_bytes());
    bytes.extend(load);
    for _ in 0..1000 {
        bytes.extend(note);
    }
    refusal(
        &bytes,
        "metadata budget exhausted: notes (program headers and sections)",
    );

    // One program-header payload fits; sharing it with a section must consume
    // the same budget, rather than getting a fresh allowance for each table.
    bytes.truncate(new_ph as usize + 2 * 56);
    bytes[56..58].copy_from_slice(&2u16.to_le_bytes());
    extract(Path::new("one-note-payload.elf"), &bytes).unwrap();
    let s = section_offset(&bytes, 2);
    bytes[s + 24..s + 32].copy_from_slice(&note_offset.to_le_bytes());
    bytes[s + 32..s + 40].copy_from_slice(&160_000u64.to_le_bytes());
    refusal(
        &bytes,
        "metadata budget exhausted: notes (program headers and sections)",
    );
}

#[test]
fn artifact_standalone_encoding_prerequisites_precede_level_selection() {
    for (code, extra, v4, vl) in [
        (&[0xc4, 0xe2, 0x7d, 0xdc, 0xc0][..], "vaes", false, false),
        (
            &[0xc4, 0xe3, 0x7d, 0x44, 0xc0, 0][..],
            "vpclmulqdq",
            false,
            false,
        ),
        (&[0xc4, 0xe3, 0xfd, 0x68, 0xc0, 0][..], "fma4", false, false),
        (&[0x8f, 0xe9, 0x78, 0x90, 0xc0][..], "xop", false, false),
        (
            &[0x62, 0xf2, 0x7d, 0x48, 0x8d, 0xc0][..],
            "avx512vbmi",
            true,
            false,
        ),
        (
            &[0x62, 0xf2, 0x7d, 0x28, 0x8d, 0xc0][..],
            "avx512vbmi",
            true,
            true,
        ),
        (
            &[0x62, 0xf2, 0x7d, 0x08, 0x8d, 0xc0][..],
            "avx512vbmi",
            true,
            true,
        ),
    ] {
        let bytes = fixture(elf::EM_X86_64, 3, code, None, None);
        let result = extract(Path::new(extra), &bytes).unwrap();
        let mut expected = names(X86_64_V3.features);
        if v4 {
            expected.extend(names(&[
                "avx512f", "avx512bw", "avx512cd", "avx512dq", "avx512vl",
            ]));
        }
        expected.insert(extra.into());
        assert_eq!(
            predicate(&result.requires, axis::ISA_X86),
            &Predicate::Subset(expected),
            "{extra}"
        );
        let used = &result.evidence[axis::ISA_X86]["used"];
        assert_eq!(used["osxsave"]["count"], 1);
        assert_eq!(used[if v4 { "avx512f" } else { "avx" }]["count"], 1);
        assert_eq!(used["avx512vl"].is_object(), vl);
    }
}

#[test]
fn artifact_partial_symbol_never_disables_whole_abi_stub_decode() {
    for section in [".init", ".fini", ".plt", ".plt.got", ".plt.sec", ".iplt"] {
        let mut bytes = fixture(
            elf::EM_X86_64,
            3,
            &[0xc3, 0xc5, 0xfd, 0xef, 0xc0],
            None,
            None,
        );
        no_frames(&mut bytes);
        rename_text(&mut bytes, section);
        add_symbols(&mut bytes, &[(0, 1, "ret_only")], false);
        let result = extract(Path::new(section), &bytes).unwrap();
        assert_eq!(
            predicate(&result.requires, axis::ISA_X86),
            &Predicate::Subset(names(X86_64_V3.features))
        );
        assert_eq!(
            result.evidence[axis::ISA_X86]["sections"][0]["uncovered_bytes"],
            0
        );
    }
}

#[test]
fn artifact_loader_decoy_and_past_eof_refuse() {
    // Exactly 544 bytes: actual AVX2 at entry 0x400078, decoy ret at another file offset.
    let mut bytes = vec![0; 544];
    bytes[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    bytes[7] = 3;
    bytes[16..18].copy_from_slice(&elf::ET_EXEC.to_le_bytes());
    bytes[18..20].copy_from_slice(&elf::EM_X86_64.to_le_bytes());
    bytes[20..24].copy_from_slice(&1u32.to_le_bytes());
    bytes[24..32].copy_from_slice(&0x400078u64.to_le_bytes());
    bytes[32..40].copy_from_slice(&64u64.to_le_bytes());
    bytes[40..48].copy_from_slice(&352u64.to_le_bytes());
    for (offset, value) in [(52, 64u16), (54, 56), (56, 1), (58, 64), (60, 3), (62, 2)] {
        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }
    bytes[64..120].copy_from_slice(&program(elf::PT_LOAD, 5, 120, 5, 0x400078));
    bytes[120..125].copy_from_slice(&[0xc5, 0xfd, 0xef, 0xc0, 0xc3]);
    bytes[128] = 0xc3;
    bytes[144..161].copy_from_slice(b"\0.text\0.shstrtab\0");
    let s = 416;
    bytes[s..s + 4].copy_from_slice(&1u32.to_le_bytes());
    bytes[s + 4..s + 8].copy_from_slice(&1u32.to_le_bytes());
    bytes[s + 8..s + 16].copy_from_slice(&6u64.to_le_bytes());
    bytes[s + 16..s + 24].copy_from_slice(&0x400078u64.to_le_bytes());
    bytes[s + 24..s + 32].copy_from_slice(&128u64.to_le_bytes());
    bytes[s + 32..s + 40].copy_from_slice(&1u64.to_le_bytes());
    let s = 480;
    bytes[s..s + 4].copy_from_slice(&7u32.to_le_bytes());
    bytes[s + 4..s + 8].copy_from_slice(&3u32.to_le_bytes());
    bytes[s + 24..s + 32].copy_from_slice(&144u64.to_le_bytes());
    bytes[s + 32..s + 40].copy_from_slice(&17u64.to_le_bytes());
    refusal(&bytes, "executable section .text");
    bytes[96..104].copy_from_slice(&16u64.to_le_bytes());
    bytes[104..112].copy_from_slice(&16u64.to_le_bytes());
    refusal(&bytes, "contradicts executable PT_LOAD mapping");
    let mut bytes = fixture(elf::EM_X86_64, 3, &[0xc3], None, None);
    let p = phoff(&bytes);
    bytes[p + 32..p + 40].copy_from_slice(&1_000_000u64.to_le_bytes());
    refusal(&bytes, "PT_LOAD: file range past EOF");
}

#[test]
fn artifact_segment_uncovered_is_evidence_and_missing_sections_refuse() {
    let mut bytes = fixture(elf::EM_X86_64, 3, &[0xc3], None, None);
    let p = phoff(&bytes);
    bytes[p + 32..p + 40].copy_from_slice(&4u64.to_le_bytes());
    bytes[p + 40..p + 48].copy_from_slice(&4u64.to_le_bytes());
    let result = extract(Path::new("padding.elf"), &bytes).unwrap();
    assert_eq!(
        result.evidence["elf.loader-view"]["segment-uncovered"],
        json!({"bytes":3,"examples":[{"file_offset":121,"length":3}]})
    );
    let s = section_offset(&bytes, 1);
    bytes[s + 8..s + 16].fill(0);
    refusal(&bytes, "executable PT_LOAD has no executable section");
    bytes[40..48].fill(0);
    bytes[60..64].fill(0);
    refusal(&bytes, "section table missing");
}

#[test]
fn artifact_aliasing_executable_sections_refuse_before_decode() {
    let mut bytes = fixture(elf::EM_X86_64, 3, &vec![0x90; 10_000], None, None);
    rename_text(&mut bytes, ".plt");
    let s = section_offset(&bytes, 1);
    let alias = bytes[s..s + 64].to_vec();
    let base = section_offset(&bytes, 0);
    let mut headers = bytes[base..base + 6 * 64].to_vec();
    for _ in 0..999 {
        headers.extend(&alias);
    }
    let offset = bytes.len() as u64;
    bytes[40..48].copy_from_slice(&offset.to_le_bytes());
    bytes[60..62].copy_from_slice(&1005u16.to_le_bytes());
    bytes.extend(headers);
    let started = std::time::Instant::now();
    refusal(&bytes, "executable section overlap");
    assert!(
        started.elapsed().as_secs() < 5,
        "must refuse before decoding aliased code"
    );
}

#[test]
fn artifact_big_endian_all_architectures_and_elf32_refuse() {
    // Minimal big-endian ELF64 headers parse, but endian refusal precedes any data scan.
    for machine in [elf::EM_AARCH64, elf::EM_X86_64] {
        let mut b = vec![0; 64];
        b[..7].copy_from_slice(b"\x7fELF\x02\x02\x01");
        b[7] = 3;
        b[16..18].copy_from_slice(&elf::ET_EXEC.to_be_bytes());
        b[18..20].copy_from_slice(&machine.to_be_bytes());
        b[20..24].copy_from_slice(&1u32.to_be_bytes());
        b[52..54].copy_from_slice(&64u16.to_be_bytes());
        refusal(&b, "big-endian ELF unsupported");
    }
    let mut b = vec![0; 52];
    b[..7].copy_from_slice(b"\x7fELF\x01\x01\x01");
    b[18..20].copy_from_slice(&elf::EM_386.to_le_bytes());
    b[20..24].copy_from_slice(&1u32.to_le_bytes());
    b[40..42].copy_from_slice(&52u16.to_le_bytes());
    refusal(&b, "unsupported ELF32");
}

#[test]
fn artifact_version_chains_validate_counts_progress_and_termination() {
    let good = with_dynamic(fixture(elf::EM_X86_64, 3, &[0xc3], None, None));
    let v = data_offset(&good, 8);
    for (offset, bytes, reason) in [
        (v, vec![2, 0], "vn_version"),
        (v + 16 + 12, vec![0; 4], "forward progress"), // vn_cnt=2, first vna_next=0
        (v + 32 + 12, 16u32.to_le_bytes().to_vec(), "exceeds vn_cnt"),
        (
            v + 8,
            0xfffffff0u32.to_le_bytes().to_vec(),
            "outside section",
        ),
        (v + 12, 16u32.to_le_bytes().to_vec(), "overlapping"),
        (
            v + 48 + 12,
            32u32.to_le_bytes().to_vec(),
            "exceeds declared count",
        ),
    ] {
        let mut bad = good.clone();
        bad[offset..offset + bytes.len()].copy_from_slice(&bytes);
        refusal(&bad, reason);
    }
}

#[test]
fn artifact_dynamic_loader_and_sections_must_agree() {
    let good = with_dynamic(fixture(elf::EM_X86_64, 3, &[0xc3], None, None));
    for (entry, reason) in [(2, "DT_STRTAB"), (4, "DT_VERNEED"), (5, "DT_VERNEEDNUM")] {
        let mut bad = good.clone();
        let d = data_offset(&bad, 7) + entry * 16 + 8;
        bad[d] ^= 1;
        refusal(&bad, reason);
    }
    let mut bad = good.clone();
    let ph = phoff(&bad) + 2 * 56;
    let d = data_offset(&bad, 7);
    let new = bad.len() as u64;
    let mut decoy = bad[d..d + 112].to_vec();
    decoy[8] ^= 1;
    bad.extend(decoy);
    bad[ph + 8..ph + 16].copy_from_slice(&new.to_le_bytes());
    refusal(&bad, "PT_DYNAMIC disagrees");
}

#[test]
fn artifact_metadata_and_evidence_budgets_and_name_truncation() {
    let mut budget = super::super::layout::Budget::new(48, 24, "symbols");
    budget.take(2).unwrap();
    assert!(budget
        .take(1)
        .unwrap_err()
        .to_string()
        .contains("metadata budget exhausted: symbols"));
    let mut bytes = fixture(elf::EM_X86_64, 3, &[0xc5, 0xfd, 0xef, 0xc0], None, None);
    let long = "é".repeat(200);
    add_symbols(&mut bytes, &[(0, 4, &long)], false);
    let result = extract(Path::new("name.elf"), &bytes).unwrap();
    assert_eq!(
        result.evidence[axis::ISA_X86]["used"]["avx2"]["examples"][0]["symbol"]
            .as_str()
            .unwrap()
            .len(),
        256
    );
    assert!(
        super::super::layout::evidence_limit(&json!({"names": "x".repeat(1<<20)}))
            .unwrap_err()
            .to_string()
            .contains("evidence-size-limit")
    );
}

#[test]
fn artifact_version_chains_allow_grouped_need_headers() {
    let mut bytes = with_dynamic(fixture(elf::EM_X86_64, 3, &[0xc3], None, None));
    let v = data_offset(&bytes, 8);
    let old = bytes[v..v + 80].to_vec();
    bytes[v..v + 16].copy_from_slice(&old[..16]);
    bytes[v + 16..v + 32].copy_from_slice(&old[48..64]);
    bytes[v + 32..v + 64].copy_from_slice(&old[16..48]);
    bytes[v + 64..v + 80].copy_from_slice(&old[64..80]);
    bytes[v + 8..v + 12].copy_from_slice(&32u32.to_le_bytes());
    bytes[v + 12..v + 16].copy_from_slice(&16u32.to_le_bytes());
    bytes[v + 24..v + 28].copy_from_slice(&48u32.to_le_bytes());
    let result = extract(Path::new("lld-layout.elf"), &bytes).unwrap();
    assert_eq!(
        predicate(&result.requires, axis::ABI_LIBC),
        &floor(line::GLIBC, vec![2, 39])
    );
}

#[test]
fn artifact_symbol_lookup_handles_nested_and_expired_aliases() {
    // Two functions overlap; the enclosing one still labels instructions after the inner ends.
    let mut bytes = fixture(
        elf::EM_X86_64,
        3,
        &[
            0xf3, 0x0f, 0xb8, 0xc0, 0xf3, 0x0f, 0xb8, 0xc0, 0xf3, 0x0f, 0xb8, 0xc0, 0xc3,
        ],
        None,
        None,
    );
    no_frames(&mut bytes);
    add_symbols(&mut bytes, &[(0, 13, "outer"), (4, 4, "inner")], false);
    let result = extract(Path::new("nested.elf"), &bytes).unwrap();
    assert_eq!(result.evidence[axis::ISA_X86]["used"]["popcnt"]["count"], 3);
    // The two names cover all three instructions, including the outer after the inner ends.
    let examples = result.evidence[axis::ISA_X86]["used"]["popcnt"]["examples"]
        .as_array()
        .unwrap();
    assert_eq!(examples.len(), 2);
    assert_eq!(examples[1]["symbol"], "inner");
    assert_eq!(
        result.evidence[axis::ISA_X86]["used"]["popcnt"]["examples"][0]["symbol"],
        "outer"
    );
}

#[test]
fn artifact_metadata_aliases_exhaust_file_sized_symbol_budget() {
    let mut bytes = fixture(elf::EM_X86_64, 3, &[0xc3], None, None);
    add_symbols(&mut bytes, &vec![(0, 1, "alias"); 1000], false);
    let base = section_offset(&bytes, 0);
    let s = section_offset(&bytes, 7);
    let alias = bytes[s..s + 64].to_vec();
    let mut headers = bytes[base..base + 8 * 64].to_vec();
    for _ in 0..10 {
        headers.extend(&alias);
    }
    let new = bytes.len() as u64;
    bytes[40..48].copy_from_slice(&new.to_le_bytes());
    bytes[60..62].copy_from_slice(&18u16.to_le_bytes());
    bytes.extend(headers);
    refusal(&bytes, "metadata budget exhausted: symbols");
}

#[test]
fn artifact_oversized_extraction_evidence_refuses() {
    // Distinct, legitimate stub ranges fit inside the file and metadata budget. Only
    // the aggregate evidence bound refuses this file, before it can be recorded.
    let mut bytes = fixture(elf::EM_X86_64, 3, &vec![0x90; 20_000], None, None);
    no_frames(&mut bytes);
    rename_text(&mut bytes, ".plt");
    let base = section_offset(&bytes, 0);
    let mut headers = bytes[base..base + 6 * 64].to_vec();
    let mut stub = headers[64..128].to_vec();
    stub[32..40].copy_from_slice(&1u64.to_le_bytes());
    headers[64..128].copy_from_slice(&stub);
    for i in 1..20_000u64 {
        stub[16..24].copy_from_slice(&(0x400078 + i).to_le_bytes());
        stub[24..32].copy_from_slice(&(0x78 + i).to_le_bytes());
        headers.extend(&stub);
    }
    let offset = bytes.len() as u64;
    bytes[40..48].copy_from_slice(&offset.to_le_bytes());
    bytes[60..62].copy_from_slice(&20005u16.to_le_bytes());
    bytes.extend(headers);
    refusal(&bytes, "evidence-size-limit");
}
