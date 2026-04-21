//! Coupling struct — ThreeLegCoupling attestation refs (spec §4.1).
//!
//! Every substantive EPR carries these; which are required is determined by
//! `EprKind::required_coupling()`.

use crate::kind::CouplingLeg;
use cid::Cid;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../sdk/epr-ts/src/generated/")]
pub struct Coupling {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[ts(type = "string | null", optional)]
    pub knowledge: Option<Cid>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[ts(type = "string | null", optional)]
    pub value: Option<Cid>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[ts(type = "string | null", optional)]
    pub governance: Option<Cid>,
}

impl Coupling {
    pub fn has(&self, leg: CouplingLeg) -> bool {
        match leg {
            CouplingLeg::Knowledge => self.knowledge.is_some(),
            CouplingLeg::Value => self.value.is_some(),
            CouplingLeg::Governance => self.governance.is_some(),
        }
    }
}
