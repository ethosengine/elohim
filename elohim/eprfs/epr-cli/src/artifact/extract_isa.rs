//! Decode function/FDE ranges and ABI stub sections; report uncovered bytes.
use std::collections::{BTreeMap, BTreeSet};

use host_contract::{X86_64_V2, X86_64_V3};
use iced_x86::{CpuidFeature, Decoder, DecoderOptions, EncodingKind, Instruction, Mnemonic};
use object::{elf, Object, ObjectSection, ObjectSymbol, SectionFlags, SymbolFlags};
use serde::Serialize;

use super::{layout::name as bounded_name, unrecorded, ArtifactResult, Elf};

#[path = "extract_control_flow.rs"]
mod control_flow;
#[path = "extract_cpuid.rs"]
mod cpuid;
use cpuid::cpu_name;

#[cfg(test)]
#[path = "extract_speculative.rs"]
pub(super) mod speculative;

const V4: &[&str] = &["avx512f", "avx512bw", "avx512cd", "avx512dq", "avx512vl"];
const BASELINE: &[&str] = &[
    "fpu", "cx8", "cmov", "mmx", "fxsr", "sse", "sse2", "syscall", "clflush", "tsc", "msr", "cpuid",
];
// ELF gABI §4 Special Sections (.init/.fini) and x86-64 psABI §5.2 Procedure Linkage
// Table: these ABI-shaped code sections, including linker PLT variants, contain code
// from start to end. This whole-section rule does not apply to arbitrary executable sections.
const ABI_STUB_SECTIONS: &[&str] = &[".init", ".fini", ".plt", ".plt.got", ".plt.sec", ".iplt"];
// psABI v3+ requires OSXSAVE: its enabled XSAVE state already implies XSAVE support.
// Keep this implication scoped to level closure, not dispatch or symbol heuristics.
const LEVEL_IMPLICATIONS: &[(&str, &str)] = &[("osxsave", "xsave")];

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
    for (level_feature, implied) in LEVEL_IMPLICATIONS {
        if required.contains(*level_feature) {
            required.remove(*implied);
        }
    }
    (!required.is_empty()).then_some(required)
}

#[derive(Serialize)]
pub(super) struct Scan {
    pub used: BTreeMap<String, FeatureUse>,
    #[serde(rename = "notRequired")]
    not_required: Vec<NotRequired>,
    instructions: u64,
    sections: Vec<Coverage>,
    #[serde(rename = "partialFunctions")]
    partial_functions: PartialFunctions,
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct PartialFunctions {
    count: u64,
    unreached_bytes: u64,
    examples: Vec<PartialFunction>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PartialFunction {
    symbol: Option<String>,
    range_start: u64,
    range_length: u64,
    decoded_bytes: u64,
    unreached_bytes: u64,
}

#[derive(Serialize)]
struct NotRequired {
    feature: &'static str,
    reason: &'static str,
    #[serde(flatten)]
    usage: FeatureUse,
}

#[derive(Default, Serialize)]
pub(super) struct FeatureUse {
    count: u64,
    examples: Vec<Example>,
}

impl FeatureUse {
    fn record(&mut self, address: u64, section: &str, symbol: Option<&str>) {
        self.count += 1;
        if self.examples.len() < 3 && !self.examples.iter().any(|e| e.symbol.as_deref() == symbol) {
            self.examples.push(Example {
                address,
                section: bounded_name(section).into(),
                symbol: symbol.map(|s| bounded_name(s).to_owned()),
            });
        }
    }
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
}

// Disjoint intervals from a sweep of function starts/ends. Lookup is logarithmic even
// for nested aliases and a long uncovered tail (no per-instruction backward scan).
struct SymbolIntervals<'a>(Vec<(u64, u64, &'a str)>);

impl<'a> SymbolIntervals<'a> {
    fn new(symbols: &[&Symbol<'a>]) -> Self {
        let mut events = Vec::new();
        for (index, s) in symbols
            .iter()
            .enumerate()
            .filter(|(_, s)| s.function && s.size != 0)
        {
            events.push((s.start, true, index));
            events.push((s.start.saturating_add(s.size), false, index));
        }
        events.sort_unstable();
        let mut active = BTreeSet::new();
        let mut intervals = Vec::new();
        let mut previous = 0;
        for (address, begins, index) in events {
            if previous < address {
                if let Some(&i) = active.last() {
                    let s: &Symbol<'a> = symbols[i];
                    intervals.push((previous, address, s.name));
                }
            }
            if begins {
                active.insert(index);
            } else {
                active.remove(&index);
            }
            previous = address;
        }
        Self(intervals)
    }

    fn containing(&self, address: u64) -> Option<&'a str> {
        let index = self.0.partition_point(|(start, _, _)| *start <= address);
        index.checked_sub(1).and_then(|i| {
            let (_, end, name) = self.0[i];
            (address < end).then_some(name)
        })
    }
}

fn uncovered(start: u64, end: u64, symbols: &[&Symbol<'_>], examples: &mut Vec<Uncovered>) {
    if start >= end || examples.len() >= 5 {
        return;
    }
    let index = symbols.partition_point(|symbol| symbol.start <= start);
    examples.push(Uncovered {
        start,
        length: end - start,
        nearest_preceding_symbol: index
            .checked_sub(1)
            .map(|i| bounded_name(symbols[i].name).into()),
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
    // Monomorphized no-op: production retains no additional bytes or measurement state.
    scan_ranges_observing(file, |_, _, _, _| {})
}

fn scan_ranges_observing(
    file: &Elf<'_>,
    mut partial: impl FnMut(&[u8], u64, &[bool], Option<&str>),
) -> ArtifactResult<Scan> {
    let mut scan = Scan {
        used: BTreeMap::new(),
        not_required: Vec::new(),
        instructions: 0,
        sections: Vec::new(),
        partial_functions: PartialFunctions::default(),
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
    let mut symbols: BTreeMap<usize, Vec<Symbol<'_>>> = BTreeMap::new();
    for s in table.filter(|s| s.is_definition()) {
        if let Some(section) = s.section_index() {
            symbols.entry(section.0).or_default().push(Symbol {
                start: s.address(), size: s.size(), name: s.name().unwrap_or("<unnamed>"),
                function: matches!(s.flags(), SymbolFlags::Elf { st_info, .. } if st_info & 0xf == elf::STT_FUNC),
            });
        }
    }
    for local in symbols.values_mut() {
        local.sort_unstable_by_key(|s| (s.start, s.size, s.name));
    }
    let frames = merge_ranges(super::frames::ranges(file)?);
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
            .get(&section.index().0)
            .into_iter()
            .flatten()
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
        let first = frames.partition_point(|r| r.end <= start);
        ranges.extend(
            frames[first..]
                .iter()
                .take_while(|r| r.start < end)
                .filter_map(|r| {
                    let from = start.max(r.start);
                    let to = end.min(r.end);
                    (from < to).then_some(from..to)
                }),
        );
        if start < end && ABI_STUB_SECTIONS.contains(&name) {
            ranges.push(start..end);
        }
        let merged = merge_ranges(ranges);
        let mut coverage = Coverage {
            section: bounded_name(name).into(),
            total_bytes: data.len() as u64,
            covered_bytes: 0,
            uncovered_bytes: 0,
            uncovered_examples: Vec::new(),
        };
        let intervals = SymbolIntervals::new(&local);
        let mut cursor = start;
        for range in merged {
            uncovered(
                cursor,
                range.start,
                &local,
                &mut coverage.uncovered_examples,
            );
            cursor = range.end;
            let code = &data[(range.start - start) as usize..(range.end - start) as usize];
            let reached = control_flow::validate(code, range.start, !ABI_STUB_SECTIONS.contains(&name))
                .map_err(|address| unrecorded(format!(
                    "isa.x86: invalid instruction or instruction past range end in {name} at 0x{address:x}, containing symbol {}",
                    intervals.containing(address).unwrap_or("<unknown; FDE range>")
                )))?;
            let decoded_bytes = if let Some(reached) = &reached {
                let decoded = reached.bytes.iter().filter(|&&b| b).count() as u64;
                let unreached = code.len() as u64 - decoded;
                if unreached != 0 {
                    partial(
                        code,
                        range.start,
                        &reached.bytes,
                        intervals.containing(range.start),
                    );
                    scan.partial_functions.count += 1;
                    scan.partial_functions.unreached_bytes += unreached;
                    if scan.partial_functions.examples.len() < 5 {
                        scan.partial_functions.examples.push(PartialFunction {
                            symbol: intervals
                                .containing(range.start)
                                .map(|s| bounded_name(s).into()),
                            range_start: range.start,
                            range_length: code.len() as u64,
                            decoded_bytes: decoded,
                            unreached_bytes: unreached,
                        });
                    }
                    let mut offset = 0;
                    while offset < code.len() && coverage.uncovered_examples.len() < 5 {
                        if reached.bytes[offset] {
                            offset += 1;
                            continue;
                        }
                        let from = offset;
                        while offset < code.len() && !reached.bytes[offset] {
                            offset += 1;
                        }
                        uncovered(
                            range.start + from as u64,
                            range.start + offset as u64,
                            &local,
                            &mut coverage.uncovered_examples,
                        );
                    }
                }
                decoded
            } else {
                code.len() as u64
            };
            coverage.covered_bytes += decoded_bytes;
            decode(
                code,
                range.start,
                name,
                &intervals,
                reached.as_ref(),
                &mut scan,
            )?;
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

// Merge overlap, preserving adjacent function boundaries. Index FDEs once so neither
// duplicate FDEs nor many sections multiply metadata traversal or instruction decoding.
fn merge_ranges(mut ranges: Vec<std::ops::Range<u64>>) -> Vec<std::ops::Range<u64>> {
    ranges.sort_unstable_by_key(|r| (r.start, r.end));
    let mut merged: Vec<std::ops::Range<u64>> = Vec::new();
    for range in ranges {
        if let Some(last) = merged.last_mut().filter(|last| range.start < last.end) {
            last.end = last.end.max(range.end);
        } else {
            merged.push(range);
        }
    }
    merged
}

fn decode(
    data: &[u8],
    start: u64,
    section: &str,
    symbols: &SymbolIntervals<'_>,
    reached: Option<&control_flow::Reached>,
    scan: &mut Scan,
) -> ArtifactResult<()> {
    let mut decoder = Decoder::with_ip(64, data, start, DecoderOptions::NONE);
    let mut instruction = Instruction::default();
    while decoder.can_decode() {
        if let Some(reached) = reached {
            let position = decoder.position();
            let Some(next) = reached.starts[position..].iter().position(|&b| b) else {
                break;
            };
            decoder.set_position(position + next).map_err(unrecorded)?;
            decoder.set_ip(start + (position + next) as u64);
        }
        decoder.decode_out(&mut instruction);
        let symbol = symbols.containing(instruction.ip());
        if reached.is_some() {
            let next = (instruction.ip() - start) as usize + 1;
            decoder.set_position(next).map_err(unrecorded)?;
            decoder.set_ip(start + next as u64);
        }
        scan.instructions += 1;
        let mut features = Vec::new();
        for cpuid in instruction.cpuid_features() {
            // Intel SDM Vol. 2, ENDBR32/ENDBR64 (and Vol. 1 §17 CET): these
            // hint-space encodings execute as NOP on processors without CET.
            // No other CPUID feature is exempted by this rule.
            if *cpuid == CpuidFeature::CET_IBT
                && matches!(
                    instruction.mnemonic(),
                    Mnemonic::Endbr32 | Mnemonic::Endbr64
                )
            {
                if scan.not_required.is_empty() {
                    scan.not_required.push(NotRequired {
                        feature: "ibt",
                        reason: "executes as NOP where unsupported",
                        usage: FeatureUse::default(),
                    });
                }
                scan.not_required[0]
                    .usage
                    .record(instruction.ip(), section, symbol);
                continue;
            }
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
        features.extend(prerequisites(&instruction));
        features.sort_unstable();
        features.dedup();
        for feature in features {
            let usage = scan.used.entry(feature.into()).or_default();
            usage.record(instruction.ip(), section, symbol);
        }
    }
    Ok(())
}

// Intel SDM Vol. 2 §2.3 (VEX), §2.6 (EVEX), Vol. 1 §14 (AVX state),
// and AMD APM Vol. 6 (128/256-bit media instructions: XOP/FMA4 exceptions).
// CPUID lists describe extensions, not the complete state prerequisites. Apply this
// one encoding table to the used set BEFORE selecting a psABI level; retain extras.
const ENCODING_PREREQUISITES: &[(EncodingKind, bool, &[&str])] = &[
    (EncodingKind::VEX, false, &["avx", "osxsave"]), // Includes AMD FMA4.
    (EncodingKind::XOP, false, &["avx", "osxsave"]),
    (EncodingKind::EVEX, false, &["avx512f", "osxsave"]),
    (EncodingKind::EVEX, true, &["avx512vl"]), // 128/256-bit vectors only.
];

fn prerequisites(instruction: &Instruction) -> Vec<&'static str> {
    let encoding = instruction.encoding();
    // Use decoded vector width, not EVEX.L'L, whose bits also encode embedded
    // rounding for 512-bit operations. The widest operand defines vector length.
    let registers = || (0..instruction.op_count()).map(|i| instruction.op_register(i));
    let narrow = encoding == EncodingKind::EVEX
        && !registers().any(|r| r.is_zmm())
        && registers().any(|r| r.is_xmm() || r.is_ymm());
    ENCODING_PREREQUISITES
        .iter()
        .filter(|(kind, narrow_only, _)| *kind == encoding && (!narrow_only || narrow))
        .flat_map(|(_, _, features)| features.iter().copied())
        .collect()
}
