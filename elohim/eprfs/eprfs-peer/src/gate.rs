//! The reach gate of a device's tier: only the person's other admitted devices are answered, and
//! each only for what this device serves them.

use std::sync::{Arc, RwLock};

use eprfs_core::{BlobCid, KeyCustody, ReachGate, ReachReason, ReachRefusal, Requester};
use iroh::NodeId;

use crate::admission::Admission;

/// Admission shared between the tier gate and whoever reloads it. A [`Admission::reload`] through
/// the lock takes effect at the next request, on every open connection, with no restart.
pub type SharedAdmission = Arc<RwLock<Admission>>;

/// What a root asked of this device's tier is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Served {
    /// A root of the channel tree this device projects and serves.
    Channel,
    /// The output CID of a derivation artifact this device holds custody of, indexed under the
    /// method named `method`. Served only to a device the artifact grant admits for that name.
    Artifact { method: String },
    /// Anything else, whether held here or not. Never served.
    Unknown,
}

/// What this device's tier serves: its channel roots, the artifacts its mount indexes, and the
/// grant that says to whom (artifact-store design §1.3). Address resolution is not authorization:
/// [`Self::classify`] and [`Self::object_of`] only say what a CID is here; [`Self::grants`] alone
/// says who may have it.
pub trait ServeScope: Send + Sync {
    /// What `root` is on this device.
    fn classify(&self, root: &BlobCid) -> Served;
    /// Whether `cid` is the manifest of `artifact`, or one of the chunks it names.
    fn object_of(&self, artifact: &BlobCid, cid: &BlobCid) -> bool;
    /// The method names of every indexed artifact `cid` is the manifest or a chunk of.
    fn methods_of(&self, cid: &BlobCid) -> Vec<String>;
    /// Whether `device` (a did:key) may be served artifacts of the method named `method_name`.
    fn grants(&self, device: &str, method_name: &str) -> bool;
}

/// The scope of a tier serving one tree store and nothing else: every root is the store's to
/// answer (the store, not the gate, says whether it is held), and no artifact is served. This is
/// the scope of [`crate::TierProtocol::new`], the tier before the artifact mount.
#[derive(Debug, Clone, Copy, Default)]
pub struct WholeStoreScope;

impl ServeScope for WholeStoreScope {
    fn classify(&self, _root: &BlobCid) -> Served {
        Served::Channel
    }

    fn object_of(&self, _artifact: &BlobCid, _cid: &BlobCid) -> bool {
        false
    }

    fn methods_of(&self, _cid: &BlobCid) -> Vec<String> {
        Vec::new()
    }

    fn grants(&self, _device: &str, _method_name: &str) -> bool {
        false
    }
}

/// The standing label an admitted device lacks for an artifact of the method named `method`.
pub fn serve_blob_standing(method: &str) -> String {
    format!("serve-blob:{method}")
}

/// Admits a [`Requester`] only when it names a device whose node id [`Admission`] admits now, and
/// then only for what its [`ServeScope`] serves.
///
/// The requester is built here, by [`Self::requester`], from the node id iroh authenticated for
/// the connection — never from anything the peer sends. An unadmitted node id becomes
/// [`Requester::anonymous`], which this gate refuses, so the endpoint answers it with its uniform
/// refusal: a stranger cannot tell a held root from an unknown one, nor a granted artifact from
/// an ungranted one. What the channel store holds is the crossing allow-list's decision, not the
/// gate's (spec §3.3).
///
/// The rules, in order, every request (artifact-store design §1.3):
///
/// - [`ReachGate::admit_root`]: the requester must be admitted; then a channel root is admitted,
///   an artifact is admitted when [`ServeScope::grants`] its method to the device, else refused
///   `OutsideStanding` naming the standing `serve-blob:<method>`; anything else is refused
///   `OutsideStanding` with no label.
/// - [`ReachGate::admit_object`]: the requester must be admitted; then an object indexed under
///   some artifact whose method is granted to the device is admitted, anything else is refused
///   `NotUnderRoot`.
///
/// **Labels narrow `reach.rs`'s normalization rule, deliberately.** eprfs-core says a serving
/// endpoint normalizes every refusal to `OutsideStanding` with no label before replying. The tree
/// endpoint still does. The artifact mount replies with these typed refusals, label included, to
/// the person's own admitted devices only: they are the same person, and the label names a method
/// of their own policy, not a group they are outside of. To anyone else every refusal is the
/// uniform form, byte-identical to the tree endpoint's.
///
/// Fail closed: a poisoned admission lock admits nobody.
#[derive(Clone)]
pub struct StewardDevicesGate {
    admission: SharedAdmission,
    scope: Arc<dyn ServeScope>,
}

impl std::fmt::Debug for StewardDevicesGate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StewardDevicesGate")
            .field("admission", &self.admission)
            .finish_non_exhaustive()
    }
}

impl StewardDevicesGate {
    /// A gate over a [`WholeStoreScope`]: every root of the served store, no artifact.
    pub fn new(admission: SharedAdmission) -> StewardDevicesGate {
        StewardDevicesGate::with_scope(admission, Arc::new(WholeStoreScope))
    }

    /// A gate judging roots and objects by `scope`.
    pub fn with_scope(
        admission: SharedAdmission,
        scope: Arc<dyn ServeScope>,
    ) -> StewardDevicesGate {
        StewardDevicesGate { admission, scope }
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

    /// The admitted device `who` is, re-read at every call, so a device dropped by a reload is
    /// refused at its next request.
    pub fn admitted<'w>(&self, who: &'w Requester) -> Option<&'w str> {
        if who.custody() != KeyCustody::DeviceHeld {
            return None;
        }
        who.agent().filter(|device| {
            self.admission
                .read()
                .is_ok_and(|admission| admission.admits_device(device))
        })
    }

    fn admit<'w>(&self, who: &'w Requester) -> Result<&'w str, ReachRefusal> {
        // The device's own name is returned borrowed from `who`, never from the lock.
        self.admitted(who)
            .ok_or_else(|| ReachRefusal::new(ReachReason::OutsideStanding, None))
    }
}

impl ReachGate for StewardDevicesGate {
    fn admit_root(&self, who: &Requester, root: &BlobCid) -> Result<(), ReachRefusal> {
        let device = self.admit(who)?;
        match self.scope.classify(root) {
            Served::Channel => Ok(()),
            Served::Artifact { method } if self.scope.grants(device, &method) => Ok(()),
            Served::Artifact { method } => Err(ReachRefusal::new(
                ReachReason::OutsideStanding,
                Some(serve_blob_standing(&method)),
            )),
            Served::Unknown => Err(ReachRefusal::new(ReachReason::OutsideStanding, None)),
        }
    }

    fn admit_object(&self, who: &Requester, cid: &BlobCid) -> Result<(), ReachRefusal> {
        let device = self.admit(who)?;
        let granted = self
            .scope
            .methods_of(cid)
            .iter()
            .any(|method| self.scope.grants(device, method));
        if granted {
            Ok(())
        } else {
            Err(ReachRefusal::new(ReachReason::NotUnderRoot, None))
        }
    }
}
