//! The reach gate of a device's tier: only the person's other admitted devices are answered.

use std::sync::{Arc, RwLock};

use eprfs_core::{BlobCid, KeyCustody, ReachGate, ReachReason, ReachRefusal, Requester};
use iroh::NodeId;

use crate::admission::Admission;

/// Admission shared between the tier gate and whoever reloads it. A [`Admission::reload`] through
/// the lock takes effect at the next request, on every open connection, with no restart.
pub type SharedAdmission = Arc<RwLock<Admission>>;

/// Admits a [`Requester`] only when it names a device whose node id [`Admission`] admits now.
///
/// The requester is built here, by [`Self::requester`], from the node id iroh authenticated for
/// the connection — never from anything the peer sends. An unadmitted node id becomes
/// [`Requester::anonymous`], which this gate refuses, so the endpoint answers it with its uniform
/// refusal: a stranger cannot tell a held root from an unknown one. Every root in the device's
/// store is admitted alike to the person's own devices; what the store holds is the crossing
/// allow-list's decision, not the gate's (spec §3.3).
///
/// Fail closed: a poisoned admission lock admits nobody.
#[derive(Debug, Clone)]
pub struct StewardDevicesGate {
    admission: SharedAdmission,
}

impl StewardDevicesGate {
    pub fn new(admission: SharedAdmission) -> StewardDevicesGate {
        StewardDevicesGate { admission }
    }

    /// The requester an authenticated `node` is: its device, held on the device, when admitted;
    /// otherwise anonymous.
    pub fn requester(&self, node: &NodeId) -> Requester {
        let device = self
            .admission
            .read()
            .ok()
            .and_then(|admission| admission.admits(node).map(str::to_string));
        match device {
            Some(device) => Requester::authenticated(device, KeyCustody::DeviceHeld),
            None => Requester::anonymous(),
        }
    }

    /// Re-read at every call, so a device dropped by a reload is refused at its next request.
    fn admit(&self, who: &Requester) -> Result<(), ReachRefusal> {
        let admitted = who.custody() == KeyCustody::DeviceHeld
            && who.agent().is_some_and(|device| {
                self.admission
                    .read()
                    .is_ok_and(|admission| admission.admits_device(device))
            });
        if admitted {
            Ok(())
        } else {
            Err(ReachRefusal::new(ReachReason::OutsideStanding, None))
        }
    }
}

impl ReachGate for StewardDevicesGate {
    fn admit_root(&self, who: &Requester, _root: &BlobCid) -> Result<(), ReachRefusal> {
        self.admit(who)
    }

    fn admit_object(&self, who: &Requester, _cid: &BlobCid) -> Result<(), ReachRefusal> {
        self.admit(who)
    }
}
