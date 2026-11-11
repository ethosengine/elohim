//! Complete executable-section scan. Symbols label evidence only; they never narrow it.
use std::collections::{BTreeMap, BTreeSet};

use host_contract::{X86_64_V2, X86_64_V3};
use iced_x86::{CpuidFeature, Decoder, DecoderOptions, Instruction, Mnemonic};
use object::{elf, Object, ObjectSection, ObjectSymbol, SectionFlags, SymbolKind};
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
    sections: Vec<String>,
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

pub(super) fn scan(file: &Elf<'_>) -> ArtifactResult<Scan> {
    let mut scan = Scan {
        used: BTreeMap::new(),
        instructions: 0,
        sections: Vec::new(),
    };
    // Merge regular and dynamic symbols for sized and zero-sized labels. Only evidence
    // uses names; decoding uses section bytes.
    let mut symbols: Vec<_> = file
        .symbols()
        .chain(file.dynamic_symbols())
        .filter(|symbol| symbol.kind() == SymbolKind::Text && symbol.is_definition())
        .filter_map(|symbol| {
            Some((
                symbol.address(),
                symbol.size(),
                symbol.section_index()?,
                symbol.name().ok()?,
            ))
        })
        .collect();
    symbols.sort_unstable_by_key(|s| (s.0, s.1, s.2 .0, s.3));
    symbols.dedup();
    let mut instruction = Instruction::default();
    for section in file.sections() {
        if !matches!(section.flags(), SectionFlags::Elf { sh_flags } if sh_flags & u64::from(elf::SHF_EXECINSTR) != 0)
        {
            continue;
        }
        let data = section.data().map_err(unrecorded)?;
        if data.is_empty() {
            continue;
        }
        let name = section.name().map_err(unrecorded)?;
        scan.sections.push(name.into());
        let local_symbols: Vec<_> = symbols
            .iter()
            .filter(|symbol| symbol.2 == section.index())
            .collect();
        let mut decoder = Decoder::with_ip(64, data, section.address(), DecoderOptions::NONE);
        while decoder.can_decode() {
            decoder.decode_out(&mut instruction);
            if instruction.is_invalid() {
                return Err(unrecorded(format!(
                    "isa.x86: invalid instruction in {name} at 0x{:x}",
                    instruction.ip()
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
                if usage.examples.len() >= 3 {
                    continue;
                }
                let index = local_symbols.partition_point(|symbol| symbol.0 <= instruction.ip());
                let symbol = index
                    .checked_sub(1)
                    .and_then(|index| local_symbols.get(index))
                    .filter(|symbol| symbol.1 == 0 || instruction.ip() - symbol.0 < symbol.1)
                    .map(|symbol| symbol.3);
                // Prefer distinct containing symbols, so the three examples describe code
                // locations instead of three adjacent instructions in the same function.
                if usage
                    .examples
                    .iter()
                    .any(|example| example.symbol.as_deref() == symbol)
                {
                    continue;
                }
                usage.examples.push(Example {
                    address: instruction.ip(),
                    section: name.into(),
                    symbol: symbol.map(str::to_owned),
                });
            }
        }
    }
    if scan.sections.is_empty() {
        return Err(unrecorded("isa.x86: no readable executable sections"));
    }
    Ok(scan)
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
