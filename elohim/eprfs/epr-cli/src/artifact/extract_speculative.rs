//! Test-only risk probe. Speculative instructions never contribute to extraction or recording.
use super::*;
use host_contract::{names::axis, Predicate, Requires};
use serde_json::{json, Value};

#[derive(Default, Serialize)]
struct Use {
    count: u64,
    in_privileged_runs: u64,
    examples: Vec<Value>,
}

#[derive(Default, Serialize)]
struct Measurement {
    unvisited_bytes: u64,
    valid_instruction_bytes: u64,
    invalid_bytes_skipped: u64,
    valid_runs: u64,
    privileged_runs: u64,
    features_already_required: BTreeMap<String, u64>,
    features_not_in_requires: BTreeMap<String, Use>,
}

pub(in crate::artifact::extract) fn measure(
    file: &Elf<'_>,
    requires: &Requires,
) -> ArtifactResult<Value> {
    let mut required: BTreeSet<String> = BASELINE.iter().map(|s| (*s).into()).collect();
    for clause in requires.clauses().filter(|c| c.axis().0 == axis::ISA_X86) {
        if let Predicate::Subset(features) = clause.predicate() {
            required.extend(features.iter().cloned());
        }
    }
    for (feature, implied) in LEVEL_IMPLICATIONS {
        if required.contains(*feature) {
            required.insert((*implied).into());
        }
    }
    let mut report = Measurement::default();
    let mut error = None;
    scan_ranges_observing(file, |code, start, visited, symbol| {
        if error.is_none() {
            error = report
                .partial(code, start, visited, symbol, &required)
                .err();
        }
    })?;
    if let Some(error) = error {
        return Err(error);
    }
    Ok(serde_json::to_value(report)?)
}

impl Measurement {
    fn partial(
        &mut self,
        code: &[u8],
        start: u64,
        visited: &[bool],
        symbol: Option<&str>,
        required: &BTreeSet<String>,
    ) -> ArtifactResult<()> {
        self.unvisited_bytes += visited.iter().filter(|&&v| !v).count() as u64;
        let mut offset = 0;
        while offset < code.len() {
            if visited[offset] {
                offset += 1;
                continue;
            }
            let from = offset;
            while offset < code.len() && !visited[offset] {
                offset += 1;
            }
            // Each unvisited span is consumed once, at instruction strides. Invalid decode
            // advances ONE byte (iced can consume more), then starts a fresh valid run.
            // Never decode across visited bytes, or count overlapping speculative starts.
            let mut decoder = Decoder::with_ip(
                64,
                &code[from..offset],
                start + from as u64,
                DecoderOptions::NONE,
            );
            let mut run = Vec::new();
            while decoder.can_decode() {
                let position = decoder.position();
                let instruction = decoder.decode();
                if instruction.is_invalid() {
                    self.run(&run, start, symbol, required)?;
                    run.clear();
                    self.invalid_bytes_skipped += 1;
                    decoder.set_position(position + 1).map_err(unrecorded)?;
                    decoder.set_ip(start + (from + position + 1) as u64);
                } else {
                    self.valid_instruction_bytes += instruction.len() as u64;
                    run.push(instruction);
                }
            }
            self.run(&run, start, symbol, required)?;
        }
        Ok(())
    }

    fn run(
        &mut self,
        instructions: &[Instruction],
        start: u64,
        symbol: Option<&str>,
        required: &BTreeSet<String>,
    ) -> ArtifactResult<()> {
        if instructions.is_empty() {
            return Ok(());
        }
        self.valid_runs += 1;
        let privileged = instructions.iter().any(Instruction::is_privileged);
        self.privileged_runs += u64::from(privileged);
        for instruction in instructions {
            let mut features = BTreeSet::new();
            for &cpuid in instruction.cpuid_features() {
                if cpuid == CpuidFeature::CET_IBT
                    && matches!(
                        instruction.mnemonic(),
                        Mnemonic::Endbr32 | Mnemonic::Endbr64
                    )
                {
                    continue; // Same hint-space exemption as the real extractor.
                }
                if let Some(name) = cpu_name(cpuid).map_err(unrecorded)? {
                    features.insert(name);
                }
            }
            if matches!(instruction.mnemonic(), Mnemonic::Lahf | Mnemonic::Sahf) {
                features.insert("lahf_lm");
            }
            features.extend(prerequisites(instruction));
            for feature in features.into_iter().filter(|f| !BASELINE.contains(f)) {
                if required.contains(feature) {
                    *self
                        .features_already_required
                        .entry(feature.into())
                        .or_default() += 1;
                    continue;
                }
                let usage = self
                    .features_not_in_requires
                    .entry(feature.into())
                    .or_default();
                usage.count += 1;
                usage.in_privileged_runs += u64::from(privileged);
                if usage.examples.len() < 3 {
                    usage.examples.push(json!({
                        "symbol": symbol, "range_start": start,
                        "offset": instruction.ip() - start,
                        "address": instruction.ip(),
                        "instruction": format!("{:?}", instruction.code()),
                        "privileged_run": privileged,
                    }));
                }
            }
        }
        Ok(())
    }
}
