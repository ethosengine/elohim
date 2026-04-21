//! Triggers ts-rs to emit TypeScript bindings into
//! elohim/sdk/epr-ts/src/generated/ via the #[ts(export_to)] attributes.

use elohim_epr::kind::CouplingLeg;
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
}
