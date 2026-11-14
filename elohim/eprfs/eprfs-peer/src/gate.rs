//! The reach gate of a device's tier: only the person's other admitted devices are answered, and
//! each only for what this device serves them.

use std::sync::{Arc, RwLock};

use eprfs_core::{BlobCid, KeyCustody, ReachGate, ReachReason, ReachRefusal, Requester};
use iroh::NodeId;

use crate::admission::Admission;
use crate::serve_decision::{
    deny_reason, wire_refusal, Basis, Deny, ObjectKind, ReachClaim, Relationship, WireRefusal,
};

/// Admission shared between the tier gate and whoever reloads it. A [`Admission::reload`] through
/// the lock takes effect at the next request, on every open connection, with no restart.
pub type SharedAdmission = Arc<RwLock<Admission>>;

/// What a root asked of this device's tier is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Served {
    /// A root of the channel tree this device projects and serves.
    Channel,
    /// The output CID of a derivation artifact this device holds custody of: its latest custody
    /// record is indexed under the method named `method`, and its own custody events give that
    /// record the reach claim `claim`. Served only as [`crate::serve_decision`] decides.
    Artifact { method: String, claim: ReachClaim },
    /// Anything else, whether held here or not. Never served.
    Unknown,
}

/// What this device's tier serves: its channel roots, the artifacts its mount indexes, and the
/// decision that says to whom (artifact-store design §1.3, K12). Address resolution is not
/// authorization: [`Self::classify`] and [`Self::object_of`] only say what a CID is here;
/// [`Self::decide`] alone says who may have an object of a record.
pub trait ServeScope: Send + Sync {
    /// What `root` is on this device.
    fn classify(&self, root: &BlobCid) -> Served;
    /// Whether `cid` is the manifest of `artifact`, or one of the chunks it names.
    fn object_of(&self, artifact: &BlobCid, cid: &BlobCid) -> bool;
    /// The method names of the indexed records `cid` is an object of whose own decision serves
    /// that object to an admitted device of the person under the policy now (admission itself is
    /// not judged here).
    fn methods_of(&self, cid: &BlobCid) -> Vec<String>;
    /// Whether the `object` of a record with `claim` and method `method_name` may be served to
    /// `device` (a did:key, judged against admission now; `None` for a requester that is not a
    /// device). Existing scopes default to deny: no policy is loaded.
    ///
    /// # Errors
    ///
    /// The [`Deny`] that refuses it.
    fn decide(
        &self,
        _device: Option<&str>,
        _claim: &ReachClaim,
        _method_name: &str,
        _object: ObjectKind,
    ) -> Result<Basis, Deny> {
        Err(Deny::PolicyUnavailable)
    }
    /// Whether some indexed record `cid` is an object of is one whose own decision serves that
    /// object to `device` now. Existing scopes default to deny.
    fn admits_object(&self, _device: &str, _cid: &BlobCid) -> bool {
        false
    }
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
}

/// A refusal of one object of a selected record: what the requester is told, and the true reason
/// this device records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordRefusal {
    pub(crate) wire: WireRefusal,
    pub(crate) local: String,
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
///   and an artifact is admitted when [`ServeScope::decide`] serves its latest record's manifest
///   to the device. A refusal names the standing the device lacks (`serve-blob:<method>`,
///   `serve-reach:ladder-unbuilt`), except that a `private` or unreadable record is refused
///   exactly as an unknown root is: `OutsideStanding` with no label.
/// - [`ReachGate::admit_object`]: the requester must be admitted; then an object is admitted when
///   some indexed record naming it serves it to the device ([`ServeScope::admits_object`]),
///   anything else is refused `NotUnderRoot`. Membership in the requested artifact is the
///   caller's to check after.
/// - [`StewardDevicesGate::admit_record`]: one object of one selected record, decided on that
///   record's own claim; the artifact answer path uses it for every request shape.
///
/// **Labels narrow `reach.rs`'s normalization rule, deliberately.** eprfs-core says a serving
/// endpoint normalizes every refusal to `OutsideStanding` with no label before replying. The tree
/// endpoint still does. The artifact mount replies with these typed refusals, label included, to
/// the person's own admitted devices only: they are the same person, and the label names a method
/// of their own policy, not a group they are outside of. To anyone else every refusal is the
/// uniform form, byte-identical to the tree endpoint's. A record declared `private` (or with a
/// reach slot the gate cannot read) is the exception: it is answered as absent even to the
/// person's own devices, which are outside its reach ([`crate::wire_refusal`]).
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

    /// Decide `object` of the selected custody record (claim `claim`, method `method_name`) for
    /// `who`: the record's own claim and method, never the output's latest record's.
    pub(crate) fn admit_record(
        &self,
        who: &Requester,
        claim: &ReachClaim,
        method_name: &str,
        object: ObjectKind,
    ) -> Result<Basis, RecordRefusal> {
        let device = self.admitted(who);
        let relationship = if device.is_some() {
            Relationship::OwnDevice
        } else {
            Relationship::None
        };
        self.scope
            .decide(device, claim, method_name, object)
            .map_err(|deny| RecordRefusal {
                wire: wire_refusal(deny, relationship, claim, method_name),
                local: deny_reason(deny, claim, method_name),
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
            Served::Artifact { method, claim } => self
                .scope
                .decide(Some(device), &claim, &method, ObjectKind::ManifestOrChunk)
                .map(|_| ())
                .map_err(|deny| {
                    match wire_refusal(deny, Relationship::OwnDevice, &claim, &method) {
                        WireRefusal::AsAbsent => {
                            ReachRefusal::new(ReachReason::OutsideStanding, None)
                        }
                        WireRefusal::Standing(standing) => {
                            ReachRefusal::new(ReachReason::OutsideStanding, Some(standing))
                        }
                    }
                }),
            Served::Unknown => Err(ReachRefusal::new(ReachReason::OutsideStanding, None)),
        }
    }

    fn admit_object(&self, who: &Requester, cid: &BlobCid) -> Result<(), ReachRefusal> {
        let device = self.admit(who)?;
        if self.scope.admits_object(device, cid) {
            Ok(())
        } else {
            Err(ReachRefusal::new(ReachReason::NotUnderRoot, None))
        }
    }
}
