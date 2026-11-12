//! Retry only invalid linear ranges by walking reachable instructions, bounded by bytes.
use iced_x86::{Decoder, DecoderOptions, FlowControl, Instruction, Mnemonic};

pub(super) struct Reached {
    pub starts: Vec<bool>,
    pub bytes: Vec<bool>,
}

/// Validate before recording features: inline data from a failed linear attempt must
/// contribute neither requirements nor counts. Clean ranges keep their linear scan.
pub(super) fn validate(
    data: &[u8],
    start: u64,
    allow_fallback: bool,
) -> Result<Option<Reached>, u64> {
    let mut decoder = Decoder::with_ip(64, data, start, DecoderOptions::NONE);
    let mut instruction = Instruction::default();
    while decoder.can_decode() {
        decoder.decode_out(&mut instruction);
        if instruction.is_invalid() || instruction.next_ip() > start + data.len() as u64 {
            return if allow_fallback {
                walk(data, start).map(Some)
            } else {
                Err(instruction.ip())
            };
        }
    }
    Ok(None)
}

fn walk(data: &[u8], start: u64) -> Result<Reached, u64> {
    let end = start + data.len() as u64;
    let mut reached = Reached {
        starts: vec![false; data.len()],
        bytes: vec![false; data.len()],
    };
    let mut pending = vec![start];
    let mut instruction = Instruction::default();
    while let Some(address) = pending.pop() {
        let mut decoder = Decoder::with_ip(
            64,
            &data[(address - start) as usize..],
            address,
            DecoderOptions::NONE,
        );
        while decoder.can_decode() {
            let offset = (decoder.ip() - start) as usize;
            if reached.starts[offset] {
                break;
            }
            reached.starts[offset] = true;
            decoder.decode_out(&mut instruction);
            if instruction.is_invalid() || instruction.next_ip() > end {
                return Err(instruction.ip());
            }
            // Count byte union, including when branch targets overlap instructions.
            reached.bytes[offset..offset + instruction.len()].fill(true);
            let flow = instruction.flow_control();
            if matches!(
                flow,
                FlowControl::ConditionalBranch
                    | FlowControl::UnconditionalBranch
                    | FlowControl::Call
            ) {
                let target = instruction.near_branch_target();
                if (start..end).contains(&target) && !reached.starts[(target - start) as usize] {
                    pending.push(target);
                }
            }
            if matches!(
                flow,
                FlowControl::Return
                    | FlowControl::UnconditionalBranch
                    | FlowControl::IndirectBranch
                    | FlowControl::IndirectCall
            ) || matches!(
                instruction.mnemonic(),
                Mnemonic::Ud2 | Mnemonic::Int3 | Mnemonic::Hlt
            ) {
                break;
            }
        }
    }
    Ok(reached)
}
