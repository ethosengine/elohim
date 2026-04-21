//! Structural validation — coupling requirement enforcement (spec §7 stage 3).
//!
//! Payload schema validation (stage 4) requires the manifest resolver and is
//! deferred to Phase 2.

use crate::{
    envelope::Envelope,
    error::{EprError, Result},
};

pub fn validate_coupling(env: &Envelope) -> Result<()> {
    for leg in env.kind.required_coupling() {
        if !env.coupling.has(*leg) {
            return Err(EprError::Coupling(format!(
                "kind {:?} requires {:?} coupling leg",
                env.kind, leg
            )));
        }
    }
    Ok(())
}
