//! Exhaustive iced-x86 1.22 CPUID -> host-contract spelling, with explicit refusals.
//! Linux references: arch/x86/include/asm/cpufeatures.h and arch/x86/kvm/reverse_cpuid.h
//! in https://github.com/torvalds/linux (inspected 2026-11-12). Kernel-internal/KVM names
//! such as avx_ifma and avx_vnni_int8 need an observer that actually supplies them;
//! a missing cpuinfo flag never grants support. sse3/lzcnt retain the contract's psABI aliases.
use super::{unrecorded, ArtifactResult};
use iced_x86::CpuidFeature;

/// Explicit decoder-to-contract spelling. Unknown extensions refuse rather than disappear or
/// invent a name. These are psABI names (sse3/lzcnt), with Linux flag names for other extensions.
pub(super) fn cpu_name(feature: CpuidFeature) -> ArtifactResult<Option<&'static str>> {
    use CpuidFeature::*;
    let name = match feature {
        // Architecture generations, NOP and PAUSE are part of this x86-64 baseline.
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
        SSE3 => "sse3", // Linux pni, normalized by the host observer to the psABI name.
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
        LZCNT => "lzcnt", // Linux abm, normalized by the host observer to the psABI name.
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
        // Linux kernel feature identifiers (some are internal, not printed in cpuinfo).
        IA64 => "ia64",
        LWP => "lwp",
        PCONFIG => "pconfig",
        RDPRU => "rdpru",
        SEV_ES => "sev_es",
        SEV_SNP => "sev_snp",
        INVLPGB => "invlpgb",
        FRED => "fred",
        LKGS => "lkgs",
        CMPCCXADD => "cmpccxadd",
        AVX_IFMA => "avx_ifma",
        AVX_NE_CONVERT => "avx_ne_convert",
        AVX_VNNI_INT8 => "avx_vnni_int8",
        WRMSRNS => "wrmsrns",
        AMX_COMPLEX => "amx_complex",
        SHA512 => "sha512",
        SM3 => "sm3",
        SM4 => "sm4",
        AVX_VNNI_INT16 => "avx_vnni_int16",
        PREFETCHITI => "prefetchiti",
        SGX1 => "sgx1",
        PADLOCK_ACE => "ace_en",
        PADLOCK_PHE => "phe_en",
        PADLOCK_PMM => "pmm_en",
        PADLOCK_RNG => "rng_en",
        INTEL8086_ONLY | INTEL286_ONLY | INTEL386_ONLY | INTEL386_A0_ONLY | INTEL486_A_ONLY
        | UMOV | MOV_TR => {
            return Err(refused(
                feature,
                "obsolete generation/stepping-only instructions have no Linux capability flag",
            ));
        }
        FPU287XL_ONLY | FPU387SL_ONLY => {
            return Err(refused(
                feature,
                "model-specific x87 instructions are not implied by the baseline fpu flag",
            ));
        }
        CYRIX_D3NOW | CYRIX_FPU | CYRIX_SMM | CYRIX_SMINT | CYRIX_SMINT_0F7E | CYRIX_SHR
        | CYRIX_DDI | CYRIX_EMMI | CYRIX_DMI => {
            return Err(refused(
                feature,
                "vendor-specific Cyrix instructions have no corresponding Linux capability flag",
            ));
        }
        HLE_or_RTM => {
            return Err(refused(
                feature,
                "HLE or RTM is a disjunction, not one Linux flag or a conjunctive subset",
            ));
        }
        SKINIT_or_SVM => {
            return Err(refused(
                feature,
                "SKINIT or SVM is a disjunction, not one Linux flag or a conjunctive subset",
            ));
        }
        INVEPT | INVVPID => {
            return Err(refused(
                feature,
                "VMX MSR capability has no exact cpuinfo flag; ept/vpid do not prove invalidation support",
            ));
        }
        CL1INVMB | MCOMMIT | PCOMMIT | OSS => {
            return Err(refused(
                feature,
                "no corresponding Linux capability flag for this instruction extension",
            ));
        }
        RDPMC => {
            return Err(refused(
                feature,
                "no exact Linux RDPMC flag; arch_perfmon is not equivalent and RDPMC is not in the level baseline",
            ));
        }
        CENTAUR_AIS | PADLOCK_GMI | PADLOCK_UNDOC => {
            return Err(refused(
                feature,
                "no corresponding Linux flag for this PadLock extension or its enabled state",
            ));
        }
        SMM => {
            return Err(refused(
                feature,
                "no Linux capability flag proving RSM/SMM instruction availability",
            ));
        }
        TDX => {
            return Err(refused(
                feature,
                "iced leaves the CPUID bit unspecified; tdx_guest/tdx_host_platform do not prove the instruction contract",
            ));
        }
        KL | AESKLE | WIDE_KL | UINTR | HRESET | RMPQUERY | RAO_INT | MSRLIST | TSE => {
            return Err(refused(
                feature,
                "no corresponding flag in Linux cpufeatures.h or KVM reverse_cpuid.h",
            ));
        }
        UDBG | KNC => {
            return Err(refused(
                feature,
                "undocumented debug/Knights Corner ISA has no corresponding Linux capability flag",
            ));
        }
    };
    Ok(Some(name))
}

fn refused(feature: CpuidFeature, why: &str) -> crate::artifact::ArtifactError {
    unrecorded(format!("unmapped CPUID feature {feature:?}: {why}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use host_contract::names::axis;
    use host_contract::{admit, AxisId, AxisValue, Predicate, Provides, Requires};

    #[test]
    fn every_mapped_cpuid_name_is_accepted_by_host_contract() {
        for feature in CpuidFeature::values() {
            match cpu_name(feature) {
                Ok(Some(name)) => {
                    // vocabulary.rs deliberately accepts an open BTreeSet<String>, not an enum.
                    let values = std::collections::BTreeSet::from([name.to_owned()]);
                    let requires: Requires = serde_json::from_value(serde_json::json!([{
                        "axis": axis::ISA_X86, "predicate": Predicate::Subset(values.clone()),
                        "source": "extracted"
                    }]))
                    .unwrap();
                    let mut host = Provides::default();
                    host.observe(AxisId(axis::ISA_X86.into()), AxisValue::Names(values));
                    assert_eq!(
                        admit(&requires, &host).decision,
                        elohim_epr::verdict::Decision::Permit,
                        "{feature:?}: {name}"
                    );
                    let wire = serde_json::to_vec(&requires).unwrap();
                    let decoded: Requires = serde_json::from_slice(&wire).unwrap();
                    assert_eq!(requires, decoded);
                }
                Ok(None) => {}
                Err(error) => {
                    let text = error.to_string();
                    assert!(text.contains(&format!("{feature:?}: ")), "{text}");
                }
            }
        }
    }
}
