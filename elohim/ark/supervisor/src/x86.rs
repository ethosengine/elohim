//! Linux x86-64 usable ISA observation. Names match epr-cli's cpu_name table.
//! CPUID bits: Linux arch/x86/include/asm/cpufeatures.h; state gates are separate.

use std::arch::x86_64::{__cpuid_count, _xgetbv};
use std::collections::BTreeSet;

macro_rules! std_probes {
    ($($rust:tt => $name:literal),* $(,)?) => {
        const STD_PROBES: &[(&str, fn() -> bool)] = &[$(
            ($name, || std::is_x86_feature_detected!($rust)),
        )*];
    };
}

// Use std's OS-aware detectors, retaining extractor spellings rather than Rust's.
std_probes!(
    "tsc" => "tsc", "mmx" => "mmx", "sse" => "sse", "sse2" => "sse2",
    "fxsr" => "fxsr", "cmpxchg16b" => "cx16", "popcnt" => "popcnt",
    "sse3" => "sse3", "sse4.1" => "sse4_1", "sse4.2" => "sse4_2", "ssse3" => "ssse3",
    "sse4a" => "sse4a", "avx" => "avx", "avx2" => "avx2", "bmi1" => "bmi1",
    "bmi2" => "bmi2", "f16c" => "f16c", "fma" => "fma", "lzcnt" => "lzcnt",
    "movbe" => "movbe", "aes" => "aes", "pclmulqdq" => "pclmulqdq",
    "adx" => "adx", "sha" => "sha_ni", "gfni" => "gfni", "vaes" => "vaes",
    "vpclmulqdq" => "vpclmulqdq", "rdrand" => "rdrand", "rdseed" => "rdseed",
    "rtm" => "rtm", "xsave" => "xsave", "xsavec" => "xsavec",
    "xsaveopt" => "xsaveopt", "xsaves" => "xsaves",
    "avx512f" => "avx512f", "avx512bw" => "avx512bw", "avx512cd" => "avx512cd",
    "avx512dq" => "avx512dq", "avx512vl" => "avx512vl", "avx512er" => "avx512er",
    "avx512pf" => "avx512pf", "avx512ifma" => "avx512ifma", "avx512vbmi" => "avx512vbmi",
    "avx512vbmi2" => "avx512_vbmi2", "avx512vnni" => "avx512_vnni",
    "avx512bitalg" => "avx512_bitalg", "avx512vpopcntdq" => "avx512_vpopcntdq",
    "avx512fp16" => "avx512_fp16", "avx512vp2intersect" => "avx512_vp2intersect",
);

// (extractor name, leaf, subleaf, register, bit). Only register-only extensions
// or features with an additional usability gate below belong here. Privileged
// modes and AMX are explicitly UNPROBED_X86 in admission, never CPUID-only promises.
const CPUID_PROBES: &[(&str, u32, u32, char, u32)] = &[
    ("fpu", 1, 0, 'd', 0),
    ("cx8", 1, 0, 'd', 8),
    ("cmov", 1, 0, 'd', 15),
    ("clflush", 1, 0, 'd', 19),
    ("sep", 1, 0, 'd', 11),
    ("syscall", 0x8000_0001, 0, 'd', 11),
    ("rdtscp", 0x8000_0001, 0, 'd', 27),
    ("3dnowext", 0x8000_0001, 0, 'd', 30),
    ("3dnow", 0x8000_0001, 0, 'd', 31),
    ("lahf_lm", 0x8000_0001, 0, 'c', 0),
    ("3dnowprefetch", 0x8000_0001, 0, 'c', 8),
    ("xop", 0x8000_0001, 0, 'c', 11),
    ("fma4", 0x8000_0001, 0, 'c', 16),
    ("tbm", 0x8000_0001, 0, 'c', 21),
    ("clzero", 0x8000_0008, 0, 'b', 0),
    ("rdpru", 0x8000_0008, 0, 'b', 4),
    ("fsgsbase", 7, 0, 'b', 0),
    ("hle", 7, 0, 'b', 4),
    ("clflushopt", 7, 0, 'b', 23),
    ("clwb", 7, 0, 'b', 24),
    ("prefetchwt1", 7, 0, 'c', 0),
    ("pku", 7, 0, 'c', 3),
    ("waitpkg", 7, 0, 'c', 5),
    ("rdpid", 7, 0, 'c', 22),
    ("cldemote", 7, 0, 'c', 25),
    ("movdiri", 7, 0, 'c', 27),
    ("movdir64b", 7, 0, 'c', 28),
    ("avx512_4vnniw", 7, 0, 'd', 2),
    ("avx512_4fmaps", 7, 0, 'd', 3),
    ("serialize", 7, 0, 'd', 14),
    ("tsxldtrk", 7, 0, 'd', 16),
    // Linux arch/x86/include/asm/cpufeatures.h, word 12: CPUID.(7,1).EAX.
    // Keep all subleaf-1 features out of std's unbounded subleaf query.
    ("sha512", 7, 1, 'a', 0),
    ("sm3", 7, 1, 'a', 1),
    ("sm4", 7, 1, 'a', 2),
    ("avx_vnni", 7, 1, 'a', 4),
    ("avx512_bf16", 7, 1, 'a', 5),
    ("cmpccxadd", 7, 1, 'a', 7),
    ("avx_ifma", 7, 1, 'a', 23),
    // Intel SDM, Vol. 2A, CPUID structured extended feature subleaf 1, EDX.
    ("avx_vnni_int8", 7, 1, 'd', 4),
    ("avx_ne_convert", 7, 1, 'd', 5),
    ("avx_vnni_int16", 7, 1, 'd', 10),
    ("prefetchiti", 7, 1, 'd', 14),
    ("ptwrite", 0x14, 0, 'b', 4),
];

fn cpuid(leaf: u32, sub: u32) -> [u32; 4] {
    cpuid_with(leaf, sub, |leaf, sub| {
        let r = __cpuid_count(leaf, sub);
        [r.eax, r.ebx, r.ecx, r.edx]
    })
}

fn cpuid_with(leaf: u32, sub: u32, mut read: impl FnMut(u32, u32) -> [u32; 4]) -> [u32; 4] {
    // CPUID exists on x86-64. Unsupported leaves must not alias the last leaf.
    let limit = read(if leaf >= 0x8000_0000 { 0x8000_0000 } else { 0 }, 0)[0];
    if leaf > limit || (leaf == 7 && sub > read(7, 0)[0]) {
        return [0; 4];
    }
    read(leaf, sub)
}

// (prerequisite, dependents). Remove unsupported claims; never infer a missing
// prerequisite from an extension bit. Apply to a fixed point for transitive loss.
const PREREQUISITES: &[(&str, &[&str])] = &[
    (
        "avx",
        &[
            "avx2",
            "avx512f",
            "vaes",
            "vpclmulqdq",
            "f16c",
            "fma",
            "fma4",
            "xop",
        ],
    ),
    (
        "avx2",
        &[
            "avx_vnni",
            "avx_ifma",
            "avx_vnni_int8",
            "avx_ne_convert",
            "avx_vnni_int16",
            "sha512",
            "sm3",
            "sm4",
        ],
    ),
    (
        "avx512f",
        &[
            "avx512bw",
            "avx512cd",
            "avx512dq",
            "avx512vl",
            "avx512er",
            "avx512pf",
            "avx512ifma",
            "avx512vbmi",
            "avx512_vbmi2",
            "avx512_vnni",
            "avx512_bitalg",
            "avx512_vpopcntdq",
            "avx512_bf16",
            "avx512_fp16",
            "avx512_vp2intersect",
            "avx512_4vnniw",
            "avx512_4fmaps",
        ],
    ),
];

fn close_prerequisites(features: &mut BTreeSet<String>) {
    loop {
        let before = features.len();
        for &(prerequisite, dependents) in PREREQUISITES {
            if !features.contains(prerequisite) {
                for &name in dependents {
                    features.remove(name);
                }
            }
        }
        if features.len() == before {
            break;
        }
    }
}

#[derive(Default)]
struct State {
    osxsave: bool,
    xcr0: u64,
    fsgsbase: bool,
    ospke: bool,
}

impl State {
    fn permits(&self, name: &str) -> bool {
        let avx = self.osxsave && self.xcr0 & 6 == 6;
        if name.starts_with("avx512") {
            return avx && self.xcr0 & 0xe0 == 0xe0;
        }
        if name.starts_with("avx")
            || matches!(
                name,
                "fma" | "fma4" | "f16c" | "xop" | "vaes" | "vpclmulqdq" | "sha512" | "sm3" | "sm4"
            )
        {
            return avx;
        }
        match name {
            "xsave" | "xsavec" | "xsaveopt" | "xsaves" => self.osxsave,
            "fsgsbase" => self.fsgsbase,
            "pku" => self.ospke,
            _ => true,
        }
    }
}

fn collect(
    probes: impl IntoIterator<Item = (&'static str, bool)>,
    state: &State,
) -> BTreeSet<String> {
    let mut features: BTreeSet<_> = probes
        .into_iter()
        .filter(|(name, present)| *present && state.permits(name))
        .map(|(name, _)| name.into())
        .collect();
    // OSXSAVE is an independent observation, not a synonym for CPU AVX support.
    if state.osxsave {
        features.insert("osxsave".into());
    }
    close_prerequisites(&mut features);
    features
}

pub(super) fn observe() -> BTreeSet<String> {
    let ecx = cpuid(1, 0)[2];
    let osxsave = ecx & (1 << 26) != 0 && ecx & (1 << 27) != 0;
    let state = State {
        osxsave,
        // SAFETY: XGETBV is called only after CPUID proves XSAVE and OSXSAVE.
        xcr0: if osxsave { unsafe { _xgetbv(0) } } else { 0 },
        // Linux HWCAP2_FSGSBASE proves CR4.FSGSBASE, unlike the CPUID bit alone.
        // SAFETY: getauxval reads the current process's auxiliary vector.
        fsgsbase: unsafe { libc::getauxval(libc::AT_HWCAP2) } & 2 != 0,
        ospke: cpuid(7, 0)[2] & (1 << 4) != 0,
    };
    let standard = STD_PROBES.iter().map(|(name, probe)| (*name, probe()));
    let direct = CPUID_PROBES.iter().map(|&(name, leaf, sub, reg, bit)| {
        let index = match reg {
            'a' => 0,
            'b' => 1,
            'c' => 2,
            'd' => 3,
            _ => unreachable!(),
        };
        (name, cpuid(leaf, sub)[index] & (1 << bit) != 0)
    });
    collect(standard.chain(direct).chain([("cpuid", true)]), &state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::admission::UNPROBED_X86;

    #[test]
    fn subleaf_zero_limit_never_queries_subleaf_one() {
        let mut calls = Vec::new();
        let result = cpuid_with(7, 1, |leaf, sub| {
            calls.push((leaf, sub));
            match (leaf, sub) {
                (0, 0) => [7, 0, 0, 0],
                // MD_CLEAR must never be interpreted as AVX-VNNI-INT16,
                // even on a CPU that aliases unsupported subleaf queries.
                (7, _) => [0, 0, 0, 1 << 10],
                _ => panic!("unexpected query"),
            }
        });
        assert_eq!(result, [0; 4]);
        assert_eq!(calls, [(0, 0), (7, 0)]);
    }

    #[test]
    fn supported_subleaf_is_read_and_unsupported_leaves_are_not() {
        assert_eq!(
            cpuid_with(7, 1, |leaf, sub| match (leaf, sub) {
                (0, 0) => [7, 0, 0, 0],
                (7, 0) => [1, 0, 0, 0],
                (7, 1) => [1 << 4, 0, 0, 1 << 10],
                _ => panic!("unexpected query"),
            }),
            [1 << 4, 0, 0, 1 << 10]
        );
        for leaf in [7, 0x8000_0001] {
            assert_eq!(
                cpuid_with(leaf, 1, |query, sub| {
                    assert_eq!(query, leaf & 0x8000_0000);
                    assert_eq!(sub, 0);
                    [query, 0, 0, 0]
                }),
                [0; 4]
            );
        }
    }

    fn all_probes() -> impl Iterator<Item = (&'static str, bool)> {
        STD_PROBES
            .iter()
            .map(|p| (p.0, true))
            .chain(CPUID_PROBES.iter().map(|p| (p.0, true)))
    }

    #[test]
    fn every_prerequisite_row_removes_only_unsupported_claims() {
        let state = State {
            osxsave: true,
            xcr0: 0xe6,
            ..State::default()
        };
        let complete = collect(all_probes(), &state);
        for &(prerequisite, dependents) in PREREQUISITES {
            let missing = collect(
                all_probes().filter(|(name, _)| *name != prerequisite),
                &state,
            );
            assert!(!missing.contains(prerequisite));
            assert!(missing.is_subset(&complete));
            assert!(missing.contains("aes"));
            for &name in dependents {
                assert!(complete.contains(name), "positive case: {name}");
                assert!(!missing.contains(name), "{name} requires {prerequisite}");
            }
        }
        // A leaked prerequisite cannot keep its own dependents alive either.
        let missing_avx = collect(all_probes().filter(|(name, _)| *name != "avx"), &state);
        for &(_, dependents) in PREREQUISITES {
            for &name in dependents {
                assert!(
                    !missing_avx.contains(name),
                    "transitive prerequisite: {name}"
                );
            }
        }
    }

    #[test]
    fn leaked_avx_vnni_int16_without_avx2_is_removed() {
        let features = collect(
            [("avx", true), ("avx2", false), ("avx_vnni_int16", true)],
            &State {
                osxsave: true,
                xcr0: 6,
                ..State::default()
            },
        );
        assert!(features.contains("avx"));
        assert!(!features.contains("avx2"));
        assert!(!features.contains("avx_vnni_int16"));
    }

    #[test]
    fn vector_extensions_keep_os_state_gates_and_prerequisite_coverage() {
        for (name, _) in all_probes().filter(|(name, _)| {
            name.starts_with("avx") || matches!(*name, "sha512" | "sm3" | "sm4")
        }) {
            if name != "avx" {
                let prerequisite = if name.starts_with("avx512") && name != "avx512f" {
                    "avx512f"
                } else if name.starts_with("avx_") || matches!(name, "sha512" | "sm3" | "sm4") {
                    "avx2"
                } else {
                    "avx"
                };
                assert!(
                    PREREQUISITES.iter().any(|&(required, dependents)| {
                        required == prerequisite && dependents.contains(&name)
                    }),
                    "missing prerequisite: {name}"
                );
            }
            for osxsave in [false, true] {
                for xcr0 in [0, 2, 6, 0xe6] {
                    let features = collect(
                        all_probes(),
                        &State {
                            osxsave,
                            xcr0,
                            ..State::default()
                        },
                    );
                    let mask = if name.starts_with("avx512") { 0xe6 } else { 6 };
                    assert_eq!(
                        features.contains(name),
                        osxsave && xcr0 & mask == mask,
                        "{name}"
                    );
                }
            }
        }
    }

    #[test]
    fn extractor_names_are_probed_or_explicitly_unprobed() {
        // Read the actual extractor table, not a second hand-maintained fixture.
        // gate-reads: elohim/eprfs/epr-cli/src/artifact/extract_cpuid.rs
        let source = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../eprfs/epr-cli/src/artifact/extract_cpuid.rs"
        ))
        .unwrap();
        let probed: BTreeSet<_> = STD_PROBES
            .iter()
            .map(|p| p.0)
            .chain(CPUID_PROBES.iter().map(|p| p.0))
            .chain(["cpuid", "osxsave"])
            .collect();
        let names: BTreeSet<_> = source
            .lines()
            .filter_map(|line| {
                let (_, tail) = line.split_once("=> \"")?;
                Some(tail.split_once('"').unwrap().0)
            })
            .collect();
        assert!(names.len() > 100, "extractor table format changed");
        for name in &names {
            assert!(
                probed.contains(name) ^ UNPROBED_X86.contains(name),
                "{name}"
            );
        }
        for name in UNPROBED_X86 {
            assert!(names.contains(name), "stale unprobed feature: {name}");
        }
    }

    #[test]
    fn synthetic_avx_requires_osxsave_and_vector_state() {
        let probes = [("avx", true), ("avx512f", true), ("avx512_4fmaps", true)];
        for osxsave in [false, true] {
            for xcr0 in [0, 2, 6, 0xe6] {
                let features = collect(
                    probes,
                    &State {
                        osxsave,
                        xcr0,
                        ..State::default()
                    },
                );
                assert_eq!(features.contains("avx"), osxsave && xcr0 & 6 == 6);
                assert!(!features.contains("avx") || features.contains("osxsave"));
                assert_eq!(features.contains("avx512f"), osxsave && xcr0 == 0xe6);
                assert_eq!(features.contains("avx512_4fmaps"), osxsave && xcr0 == 0xe6);
            }
        }
    }

    #[test]
    fn pku_and_fsgsbase_require_os_enablement() {
        let probes = [("pku", true), ("fsgsbase", true)];
        assert!(collect(probes, &State::default()).is_empty());
        assert_eq!(
            collect(
                probes,
                &State {
                    ospke: true,
                    fsgsbase: true,
                    ..State::default()
                }
            )
            .len(),
            2
        );
    }
}
