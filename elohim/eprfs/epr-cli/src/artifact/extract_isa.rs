//! Decode the union of sized function symbols and FDE ranges; report uncovered bytes.
use std::collections::{BTreeMap, BTreeSet};

use host_contract::{X86_64_V2, X86_64_V3};
use iced_x86::{CpuidFeature, Decoder, DecoderOptions, Instruction, Mnemonic};
use object::{elf, Object, ObjectSection, ObjectSymbol, SectionFlags, SymbolFlags};
use serde::Serialize;

use super::{unrecorded, ArtifactResult, Elf};

const V4: &[&str] = &["avx512f", "avx512bw", "avx512cd", "avx512dq", "avx512vl"];
const BASELINE: &[&str] = &[
    "fpu", "cx8", "cmov", "mmx", "fxsr", "sse", "sse2", "syscall", "clflush", "tsc", "msr", "cpuid",
];

/// Round up to the smallest psABI level covering used level features; retain every extension
/// outside those levels. Baseline features alone create no clause (spec §3.1 rule 4).
pub(super) fn level_features(used: &BTreeSet<String>) -> Option<BTreeSet<String>> {
    let mut required: BTreeSet<String> = used
        .iter()
        .filter(|name| !BASELINE.contains(&name.as_str()))
        .cloned()
        .collect();
    let level = if used.iter().any(|name| V4.contains(&name.as_str())) {
        4
    } else if used.iter().any(|name| {
        X86_64_V3.features.contains(&name.as_str()) && !X86_64_V2.features.contains(&name.as_str())
    }) {
        3
    } else if used
        .iter()
        .any(|name| X86_64_V2.features.contains(&name.as_str()))
    {
        2
    } else {
        1
    };
    if level >= 2 {
        required.extend(X86_64_V2.features.iter().map(|name| (*name).into()));
    }
    if level >= 3 {
        required.extend(X86_64_V3.features.iter().map(|name| (*name).into()));
    }
    if level >= 4 {
        required.extend(V4.iter().map(|name| (*name).into()));
    }
    (!required.is_empty()).then_some(required)
}

#[derive(Serialize)]
pub(super) struct Scan {
    pub used: BTreeMap<String, FeatureUse>,
    instructions: u64,
    sections: Vec<Coverage>,
}

#[derive(Default, Serialize)]
pub(super) struct FeatureUse {
    count: u64,
    examples: Vec<Example>,
}

#[derive(Serialize)]
struct Example {
    address: u64,
    section: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    symbol: Option<String>,
}

#[derive(Serialize)]
struct Coverage {
    section: String,
    total_bytes: u64,
    covered_bytes: u64,
    uncovered_bytes: u64,
    uncovered_examples: Vec<Uncovered>,
}

#[derive(Serialize)]
struct Uncovered {
    start: u64,
    length: u64,
    nearest_preceding_symbol: Option<String>,
}

struct Symbol<'a> {
    start: u64,
    size: u64,
    name: &'a str,
    function: bool,
    section: object::SectionIndex,
}

fn containing<'a>(symbols: &[&Symbol<'a>], address: u64) -> Option<&'a str> {
    let index = symbols.partition_point(|symbol| symbol.start <= address);
    symbols[..index]
        .iter()
        .rev()
        .find(|s| s.function && address - s.start < s.size)
        .map(|s| s.name)
}

fn uncovered(start: u64, end: u64, symbols: &[&Symbol<'_>], examples: &mut Vec<Uncovered>) {
    if start >= end || examples.len() >= 5 {
        return;
    }
    let index = symbols.partition_point(|symbol| symbol.start <= start);
    examples.push(Uncovered {
        start,
        length: end - start,
        nearest_preceding_symbol: index.checked_sub(1).map(|i| symbols[i].name.into()),
    });
}

pub(super) fn scan(file: &Elf<'_>) -> ArtifactResult<Scan> {
    let scan = scan_ranges(file)?;
    let unknown: Vec<_> = scan
        .sections
        .iter()
        .filter(|s| s.total_bytes != 0 && s.covered_bytes == 0)
        .map(|s| s.section.as_str())
        .collect();
    if !unknown.is_empty() {
        return Err(unrecorded(format!("isa.x86: executable sections {}: function boundaries are unknown (no sized function symbols or FDE ranges)", unknown.join(", "))));
    }
    Ok(scan)
}

// An audit can describe unknown coverage, but scan() must refuse before producing Requires.
pub(super) fn scan_ranges(file: &Elf<'_>) -> ArtifactResult<Scan> {
    let mut scan = Scan {
        used: BTreeMap::new(),
        instructions: 0,
        sections: Vec::new(),
    };
    // A present .symtab is authoritative; use .dynsym only when it is absent.
    let table = if file.elf_section_table().iter().any(|s| {
        use object::read::elf::SectionHeader;
        s.sh_type(file.endian()) == elf::SHT_SYMTAB
    }) {
        file.symbols()
    } else {
        file.dynamic_symbols()
    };
    let mut symbols: Vec<_> = table.filter(|s| s.is_definition()).filter_map(|s| {
        Some(Symbol {
            start: s.address(), size: s.size(), section: s.section_index()?,
            name: s.name().unwrap_or("<unnamed>"),
            function: matches!(s.flags(), SymbolFlags::Elf { st_info, .. } if st_info & 0xf == elf::STT_FUNC),
        })
    }).collect();
    symbols.sort_unstable_by_key(|s| (s.start, s.size, s.name));
    let frames = super::frames::ranges(file)?;
    for section in file.sections() {
        if !matches!(section.flags(), SectionFlags::Elf { sh_flags } if sh_flags & u64::from(elf::SHF_EXECINSTR) != 0)
        {
            continue;
        }
        let data = section.data().map_err(unrecorded)?;
        let name = section.name().map_err(unrecorded)?;
        let start = section.address();
        let end = start
            .checked_add(data.len() as u64)
            .ok_or_else(|| unrecorded("executable section address overflow"))?;
        let local: Vec<_> = symbols
            .iter()
            .filter(|s| s.section == section.index())
            .collect();
        let mut ranges = Vec::new();
        for symbol in local.iter().filter(|s| s.function && s.size != 0) {
            let symbol_end = symbol
                .start
                .checked_add(symbol.size)
                .ok_or_else(|| unrecorded("symbol range overflow"))?;
            if symbol.start < start || symbol_end > end {
                return Err(unrecorded(format!(
                    "function {} range outside executable section {name}",
                    symbol.name
                )));
            }
            ranges.push(symbol.start..symbol_end);
        }
        // An FDE is a virtual-address range. Intersect it with this section's bytes.
        ranges.extend(frames.iter().filter_map(|r| {
            let from = start.max(r.start);
            let to = end.min(r.end);
            (from < to).then_some(from..to)
        }));
        ranges.sort_unstable_by_key(|r| (r.start, r.end));
        let mut merged: Vec<std::ops::Range<u64>> = Vec::new();
        for range in ranges {
            if let Some(last) = merged.last_mut().filter(|last| range.start < last.end) {
                last.end = last.end.max(range.end);
            } else {
                merged.push(range);
            }
        }
        let mut coverage = Coverage {
            section: name.into(),
            total_bytes: data.len() as u64,
            covered_bytes: 0,
            uncovered_bytes: 0,
            uncovered_examples: Vec::new(),
        };
        let mut cursor = start;
        for range in merged {
            uncovered(
                cursor,
                range.start,
                &local,
                &mut coverage.uncovered_examples,
            );
            coverage.covered_bytes += range.end - range.start;
            cursor = range.end;
            let code = &data[(range.start - start) as usize..(range.end - start) as usize];
            decode(code, range.start, name, &local, &mut scan)?;
        }
        uncovered(cursor, end, &local, &mut coverage.uncovered_examples);
        coverage.uncovered_bytes = coverage.total_bytes - coverage.covered_bytes;
        scan.sections.push(coverage);
    }
    if !scan.sections.iter().any(|s| s.total_bytes != 0) {
        return Err(unrecorded("isa.x86: no readable executable sections"));
    }
    Ok(scan)
}

fn decode(
    data: &[u8],
    start: u64,
    section: &str,
    symbols: &[&Symbol<'_>],
    scan: &mut Scan,
) -> ArtifactResult<()> {
    let mut decoder = Decoder::with_ip(64, data, start, DecoderOptions::NONE);
    let mut instruction = Instruction::default();
    while decoder.can_decode() {
        decoder.decode_out(&mut instruction);
        let symbol = containing(symbols, instruction.ip());
        if instruction.is_invalid() || instruction.next_ip() > start + data.len() as u64 {
            return Err(unrecorded(format!(
                "isa.x86: invalid instruction or instruction past range end in {section} at 0x{:x}, containing symbol {}",
                instruction.ip(), symbol.unwrap_or("<unknown; FDE range>")
            )));
        }
        scan.instructions += 1;
        let mut features = Vec::new();
        for cpuid in instruction.cpuid_features() {
            if let Some(feature) = cpu_name(*cpuid)
                .map_err(|e| unrecorded(format!("isa.x86 at 0x{:x}: {e}", instruction.ip())))?
            {
                features.push(feature);
            }
        }
        // iced labels these X64, but LAHF/SAHF in long mode require the v2 LAHF_LM bit.
        if matches!(instruction.mnemonic(), Mnemonic::Lahf | Mnemonic::Sahf) {
            features.push("lahf_lm");
        }
        for feature in features {
            let usage = scan.used.entry(feature.into()).or_default();
            usage.count += 1;
            if usage.examples.len() >= 3
                || usage.examples.iter().any(|e| e.symbol.as_deref() == symbol)
            {
                continue;
            }
            usage.examples.push(Example {
                address: instruction.ip(),
                section: section.into(),
                symbol: symbol.map(str::to_owned),
            });
        }
    }
    Ok(())
}

/// Explicit decoder-to-contract spelling. Unknown extensions refuse rather than disappear or
/// invent a name. These are psABI names (sse3/lzcnt), with Linux flag names for other extensions.
fn cpu_name(feature: CpuidFeature) -> ArtifactResult<Option<&'static str>> {
    use CpuidFeature::*;
    let name = match feature {
        INTEL8086 | INTEL186 | INTEL286 | INTEL386 | INTEL486 | X64 | MULTIBYTENOP | PAUSE => {
            return Ok(None)
        }
        FPU | FPU287 | FPU387 => "fpu",
        CMOV => "cmov",
        CPUID => "cpuid",
        CX8 => "cx8",
        FXSR => "fxsr",
        MMX => "mmx",
        SSE => "sse",
        SSE2 => "sse2",
        SYSCALL => "syscall",
        CLFSH => "clflush",
        TSC => "tsc",
        MSR => "msr",
        CMPXCHG16B => "cx16",
        SSE3 => "sse3",
        SSSE3 => "ssse3",
        SSE4_1 => "sse4_1",
        SSE4_2 => "sse4_2",
        POPCNT => "popcnt",
        AVX => "avx",
        AVX2 => "avx2",
        BMI1 => "bmi1",
        BMI2 => "bmi2",
        F16C => "f16c",
        FMA => "fma",
        LZCNT => "lzcnt",
        MOVBE => "movbe",
        AVX512F => "avx512f",
        AVX512BW => "avx512bw",
        AVX512CD => "avx512cd",
        AVX512DQ => "avx512dq",
        AVX512VL => "avx512vl",
        AVX512ER => "avx512er",
        AVX512PF => "avx512pf",
        AVX512_4FMAPS => "avx512_4fmaps",
        AVX512_4VNNIW => "avx512_4vnniw",
        AVX512_BF16 => "avx512_bf16",
        AVX512_BITALG => "avx512_bitalg",
        AVX512_IFMA => "avx512ifma",
        AVX512_VBMI => "avx512vbmi",
        AVX512_VBMI2 => "avx512_vbmi2",
        AVX512_VNNI => "avx512_vnni",
        AVX512_VP2INTERSECT => "avx512_vp2intersect",
        AVX512_VPOPCNTDQ => "avx512_vpopcntdq",
        AVX512_FP16 => "avx512_fp16",
        AES => "aes",
        PCLMULQDQ => "pclmulqdq",
        SHA => "sha_ni",
        RDRAND => "rdrand",
        RDSEED => "rdseed",
        ADX => "adx",
        VAES => "vaes",
        VPCLMULQDQ => "vpclmulqdq",
        GFNI => "gfni",
        FSGSBASE => "fsgsbase",
        RDPID => "rdpid",
        RDTSCP => "rdtscp",
        XSAVE => "xsave",
        XSAVEC => "xsavec",
        XSAVEOPT => "xsaveopt",
        XSAVES => "xsaves",
        CET_IBT => "ibt",
        CET_SS => "user_shstk",
        CLDEMOTE => "cldemote",
        CLFLUSHOPT => "clflushopt",
        CLWB => "clwb",
        CLZERO => "clzero",
        FMA4 => "fma4",
        SSE4A => "sse4a",
        XOP => "xop",
        TBM => "tbm",
        D3NOW => "3dnow",
        D3NOWEXT => "3dnowext",
        MONITOR => "monitor",
        MONITORX => "mwaitx",
        MOVDIR64B => "movdir64b",
        MOVDIRI => "movdiri",
        MPX => "mpx",
        PKU => "pku",
        PREFETCHW => "3dnowprefetch",
        PREFETCHWT1 => "prefetchwt1",
        PTWRITE => "ptwrite",
        RTM => "rtm",
        HLE => "hle",
        SEP => "sep",
        SMAP => "smap",
        SMX => "smx",
        VMX => "vmx",
        SVM => "svm",
        SKINIT => "skinit",
        INVPCID => "invpcid",
        ENQCMD => "enqcmd",
        SERIALIZE => "serialize",
        WAITPKG => "waitpkg",
        WBNOINVD => "wbnoinvd",
        TSXLDTRK => "tsxldtrk",
        AMX_BF16 => "amx_bf16",
        AMX_TILE => "amx_tile",
        AMX_INT8 => "amx_int8",
        AMX_FP16 => "amx_fp16",
        AVX_VNNI => "avx_vnni",
        _ => return Err(unrecorded(format!("unmapped CPUID feature {feature:?}"))),
    };
    Ok(Some(name))
}
