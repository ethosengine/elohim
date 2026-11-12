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
            // Intel SDM Vol. 2: CALL, JMP, Jcc/LOOP, RET/IRET, INT, UD, HLT,
            // XBEGIN/XABORT/XEND. This is a bounded static walk, not CPU emulation.
            // Keep exhaustive: a new iced flow class must get an explicit policy.
            let (follow_target, fall_through) = match instruction.flow_control() {
                // Follow the taken target and fall through for the not-taken case.
                FlowControl::ConditionalBranch => (true, true),
                // Follow the target, then stop: JMP has no sequential continuation.
                FlowControl::UnconditionalBranch => (true, false),
                // Follow an in-range direct target and fall through for a return.
                // iced also puts system/VM calls here; they have no near operand.
                FlowControl::Call => (true, true),
                // Stop: the return destination comes from runtime state.
                FlowControl::Return => (false, false),
                // Stop: the jump target needs register/memory state we do not resolve.
                FlowControl::IndirectBranch => (false, false),
                // The callee is unresolved, but CALL returns to the next instruction,
                // as a direct call does: fall through. Stopping here would leave the
                // continuation unreached and its features uncounted (a weaker clause).
                FlowControl::IndirectCall => (false, true),
                // INT saves a continuation for IRET: conservatively fall through.
                // INT3 remains an explicit breakpoint stop under this contract.
                FlowControl::Interrupt => (false, instruction.mnemonic() != Mnemonic::Int3),
                // iced assigns only XBEGIN to this class: follow its abort target
                // AND fall through into the transaction on successful entry.
                FlowControl::XbeginXabortXend => (true, true),
                // Stop at UD0/UD1/UD2: #UD faults, with no normal fallthrough.
                // Invalid decodes already refused above, before this match.
                FlowControl::Exception => (false, false),
                // Fall through ordinary instructions (also XEND, and XABORT,
                // a NOP outside RTM; XBEGIN already queues the abort edge).
                // HLT stays a contract stop despite possible interrupt resumption.
                FlowControl::Next => (false, instruction.mnemonic() != Mnemonic::Hlt),
            };
            if follow_target {
                let target = instruction.near_branch_target();
                if (start..end).contains(&target) && !reached.starts[(target - start) as usize] {
                    pending.push(target);
                }
            }
            if !fall_through {
                break;
            }
        }
    }
    Ok(reached)
}
