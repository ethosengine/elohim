//! Triggers ts-rs to emit TypeScript bindings into
//! elohim/sdk/epr-ts/src/generated/ via the #[ts(export_to)] attributes.

use elohim_epr::kind::CouplingLeg;
use elohim_epr::witness::{
    ComputeSource, FrameClassification, FrameEvidence, FrameVerdict, Hop, Magnitude, ReaVerb,
    ResolutionProvenance, Span, WitnessId, WitnessedInteraction,
};
use elohim_epr::{Coupling, Envelope, EprKind, Reach, Signature};
use ts_rs::TS;

#[test]
fn export_bindings() {
    Coupling::export().unwrap();
    Envelope::export().unwrap();
    EprKind::export().unwrap();
    Reach::export().unwrap();
    Signature::export().unwrap();
    CouplingLeg::export().unwrap();

    // Frame/witness primitive (G1)
    WitnessedInteraction::export().unwrap();
    Magnitude::export().unwrap();
    FrameClassification::export().unwrap();
    FrameEvidence::export().unwrap();
    ResolutionProvenance::export().unwrap();
    Span::export().unwrap();
    FrameVerdict::export().unwrap();
    ComputeSource::export().unwrap();
    Hop::export().unwrap();
    ReaVerb::export().unwrap();
    WitnessId::export().unwrap();
}
