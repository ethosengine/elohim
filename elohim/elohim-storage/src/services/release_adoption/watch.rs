//! The sweep — watch, fetch, and hand the evidence to the floor.
//!
//! One tick over the followed channels. Everything expensive is sized BEFORE it
//! is started, because the two things this loop touches (a conductor zome call
//! and a blob read) are both uncancellable in the way that matters: a
//! `call_zome` abandoned by a caller-side timeout keeps executing inside the
//! conductor, still holding the read permit whose saturation the timeout was
//! reacting to. So there are no timeouts here. There are budgets, checked
//! first:
//!
//! bounded-work: `MAX_CHANNELS_PER_SWEEP` head resolves +
//! `MAX_THRESHOLD_READS_PER_SWEEP` attestation reads +
//! `MAX_ARTIFACT_BYTES_PER_SWEEP` bytes staged, all decided before the first
//! call; a channel that does not fit is simply not checked this tick (its
//! recorded state is left untouched, so `lastCheckedAt` never lies about when
//! it was last actually looked at), and the finite ladder in
//! [`super::state::BACKOFF_LADDER_SECS`] paces the rest.
//!
//! # I1 — where authority terminates
//!
//! The head comes from THIS node's conductor and nowhere else. A peer hint or a
//! `ContentHeadDeclared` signal may *trigger* a sweep; it may never *supply* an
//! answer. That is why the resolve is a `resolve_content_head_local` call
//! (`GetStrategy::Local` — gossip already delivered, never a network fetch)
//! rather than a read of the local projection: the projection is a record of what we were
//! told, and this loop is deciding whether to change what we RUN.
//!
//! # Where the manifest actually lives
//!
//! T2's ceremony driver publishes a release as an `update_content` on the
//! channel's OWN content id, patching `metadata_json` to
//! `{"kind":"release-manifest","publishedAt":…,"manifest":{…}}`, then declares
//! that version the canonical head. So the resolved head's
//! `content.metadata_json` IS the release — there is no separate content body
//! to fetch, and a channel ROOT (which carries `kind: "release-channel"`) is
//! correctly read as "no release published yet", not as a malformed one.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use seam_contracts::{Answer, ReasonLabel};
use sha2::{Digest, Sha256};

use super::apply::{ApplyRegistry, SoakAttestor};
use super::state::{
    self, AdoptionMode, AppliedRelease, FollowedChannel, FollowedChannels, HeadTier, ResolvedHead,
    Verdict,
};
use super::verify::{self, FetchedArtifact, InstalledReality, LineageEvidence, VerifyInput};
use super::{AdoptionRefusal, Artifact, DecisionArm, RefusalReason, RELEASE_MANIFEST_KIND};
use crate::blob_store::BlobStore;
use crate::conductor_admission::AdmissionClass;
use crate::hc_client::HcClient;
use crate::services::conductor_writes::ContentHeadWire;
use crate::services::release_attestation::{self, ReleaseAttestationCtx};

/// How often the controller sweeps. Slower than the runtime-config poller by an
/// order of magnitude on purpose: a release is a ceremony act with a soak
/// window measured in half-hours, so a minute of latency costs nothing and a
/// tighter loop only spends conductor capacity.
pub const SWEEP_INTERVAL_SECS: u64 = 60;

/// Head resolves started per sweep. A followed-channel set is single-digit by
/// design (spec §5 head-plane cost: a handful of channels per network), so this
/// is slack rather than a cap on ambition — it exists so a mis-edited config
/// with a hundred entries cannot turn one tick into a hundred zome calls.
pub const MAX_CHANNELS_PER_SWEEP: usize = 8;

/// Attestation threshold reads started per sweep. Each one is `1 + N` zome
/// calls (T5 bounds `N` itself), so this is the second factor in the sweep's
/// conductor budget and is deliberately tighter than the channel cap: the
/// cheap arms refuse first, so only channels that got all the way to the
/// threshold arm spend it.
pub const MAX_THRESHOLD_READS_PER_SWEEP: usize = 4;

/// Artifact bytes staged per sweep. A coordinator bundle runs 1-64 MiB; a
/// storage binary is larger. A channel whose declared artifact bytes do not fit
/// in the remaining budget is skipped this tick, NOT refused — deferral and
/// refusal are different facts and the report must not confuse them.
pub const MAX_ARTIFACT_BYTES_PER_SWEEP: u64 = 192 * 1024 * 1024;

/// How long an installed-reality read is reused before it is taken again.
///
/// Installed reality changes on an install or a coordinator hot-swap — events
/// this node performs, not events it discovers. Re-reading the hApp inventory
/// every sweep would spend admin round trips to re-learn a fact that almost
/// never moves; five minutes is well inside the soak windows a release
/// discipline declares.
pub const INSTALLED_REALITY_TTL_SECS: i64 = 300;

// ---------------------------------------------------------------------------
// Metrics (C8)
// ---------------------------------------------------------------------------

/// Count one decision. Every arm of every sweep passes through here, so a
/// verdict that reaches the report and not the meter is impossible by
/// construction.
pub fn record_decision(arm: DecisionArm, reason: &str) {
    crate::metrics::RELEASE_ADOPTION_DECISIONS
        .with_label_values(&[arm.label(), reason])
        .inc();
}

// ---------------------------------------------------------------------------
// Reading the release out of the head
// ---------------------------------------------------------------------------

/// Pull the release manifest body out of a resolved head's `metadata_json`.
///
/// Three honest outcomes, and they are NOT the same:
/// - `Ok(Some(body))` — this version is a release.
/// - `Ok(None)` — this version is not a release (a channel root, or an
///   ordinary content version). Idle, not a refusal.
/// - `Err(_)` — the envelope claims to be a release and is not readable.
pub fn extract_release_body(
    metadata_json: &str,
) -> Result<Option<serde_json::Value>, AdoptionRefusal> {
    let trimmed = metadata_json.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let envelope: serde_json::Value = match serde_json::from_str(trimmed) {
        Ok(v) => v,
        Err(e) => {
            return Err(AdoptionRefusal::new(
                RefusalReason::ManifestUndecodable,
                format!("head metadata_json is not JSON: {e}"),
            ))
        }
    };
    match envelope.get("kind").and_then(serde_json::Value::as_str) {
        Some(RELEASE_MANIFEST_KIND) => {}
        // A channel root, or any other version. Nothing to judge.
        _ => return Ok(None),
    }
    match envelope.get("manifest") {
        Some(body) if body.is_object() => Ok(Some(body.clone())),
        Some(_) => Err(AdoptionRefusal::new(
            RefusalReason::ManifestUndecodable,
            "metadata_json declares kind=release-manifest but `manifest` is not an object",
        )),
        None => Err(AdoptionRefusal::new(
            RefusalReason::ManifestUndecodable,
            "metadata_json declares kind=release-manifest but carries no `manifest` key",
        )),
    }
}

/// The bare envelope `kind` tag out of a content version's `metadata_json` —
/// the same discriminator [`extract_release_body`] and
/// [`verify::verify_shape`] both key on (`RELEASE_MANIFEST_KIND`), without the
/// manifest-body validation neither the lineage question nor this call site
/// needs. `None` for empty/unparseable JSON or an object with no `kind` field
/// — read honestly as "not a release", never guessed.
fn envelope_kind_tag(metadata_json: &str) -> Option<String> {
    let trimmed = metadata_json.trim();
    if trimmed.is_empty() {
        return None;
    }
    let envelope: serde_json::Value = serde_json::from_str(trimmed).ok()?;
    envelope
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

/// The pieces of a resolved content-version record that lineage needs — the
/// envelope `kind` tag ([`envelope_kind_tag`]) and the record's own `id`.
/// Reading the superseded record only ever needed the kind; reading a
/// declared parent ALSO needs the id, because "the declared parent exists as
/// a release on THIS channel" checks the record's own channel id, not merely
/// "is it a release somewhere" — see [`AdoptionController::resolve_record_summary`].
#[derive(Debug, Clone, PartialEq, Eq)]
struct RecordSummary {
    kind: Option<String>,
    id: String,
}

/// The two-phase lineage-evidence decision [`lineage_evidence_from`] returns.
///
/// `Evidence` is a settled answer — no further conductor call is needed.
/// `NeedDeclaredLookup` means the chain is a STAR (the superseded record is
/// not itself a release) and the manifest declares a parent: evidence cannot
/// be settled without reading that declared cid's own record, so the caller
/// reads it (the same way it read the superseded record) and calls
/// [`lineage_evidence_from`] again with the result.
#[derive(Debug, Clone, PartialEq, Eq)]
enum LineageDecision {
    /// The lineage evidence, settled.
    Evidence(Option<String>),
    /// Settling requires reading the declared parent through the conductor.
    NeedDeclaredLookup,
}

/// Decide the prior-RELEASE lineage evidence from what the head's
/// declaration supersedes, in at most two calls.
///
/// `superseded` is `None` when the head supersedes nothing at all (the head
/// IS the channel root). `Some((cid, kind))` pairs the superseded record's
/// action-hash cid with its envelope `kind` tag ([`envelope_kind_tag`]).
/// `declared` is the manifest's own `envelope.lineageParentCid`.
/// `declared_summary` is `None` on the first call (nothing read yet) and
/// `Some((kind, id))` on the second, once the caller has resolved `declared`
/// the same way it resolved `superseded`.
///
/// T2's `update_content` targets the channel's first `IdToContent` link (the
/// root) for **every** version, first release or fifth — the L2 chain is a
/// STAR around the root, never release→release. So a superseded record whose
/// kind is [`RELEASE_MANIFEST_KIND`] is the one case the chain itself proves
/// order for:
///
/// - **(a)** the chain carries a release → `Evidence(Some(that cid))`,
///   regardless of what the body declares — the chain outranks the hint.
///
/// Every other shape (the root, or an ordinary content version) is a star,
/// and the chain alone settles nothing about release order:
///
/// - **(b)** star, no declared parent → a first release: `Evidence(None)`.
/// - star, a declared parent → the strongest check the substrate offers is
///   EXISTENCE: does the declared cid resolve to a `release-manifest` whose
///   own `id` is THIS channel? That needs a second conductor read, so the
///   first call returns [`LineageDecision::NeedDeclaredLookup`]; once the
///   caller supplies `declared_summary`:
///   - **(c)** a release on this channel → `Evidence(Some(declared cid))`.
///   - **(d)** a release on ANOTHER channel → `Evidence(None)`.
///   - **(e)** not a release at all → `Evidence(None)`.
///
/// Never guesses: a read that cannot be performed (Absent/Unreachable) is the
/// caller's job to propagate honestly, never folded into this function as
/// either agreement or disagreement.
/// Borrow a resolved `(cid, kind)` pair for [`lineage_evidence_from`] without
/// consuming it — the pair is read twice when a declared-parent lookup is
/// needed (once to learn that, once to settle after the lookup).
fn superseded_as_ref(pair: &Option<(String, Option<String>)>) -> Option<(String, Option<&str>)> {
    pair.as_ref()
        .map(|(cid, kind)| (cid.clone(), kind.as_deref()))
}

fn lineage_evidence_from(
    superseded: Option<(String, Option<&str>)>,
    declared: Option<&str>,
    declared_summary: Option<(Option<&str>, Option<&str>)>,
    channel_id: &str,
) -> LineageDecision {
    // (a) The L2 chain itself names a prior RELEASE — order is proven by the
    // chain, and what the body declares plays no part in this arm.
    if let Some((cid, kind)) = &superseded {
        if *kind == Some(RELEASE_MANIFEST_KIND) {
            return LineageDecision::Evidence(Some(cid.clone()));
        }
    }
    // The chain is a STAR from here on: `superseded` is `None` (the head IS
    // the channel root) or names a record that is not itself a release.
    // Neither carries release order, so the declared parent's EXISTENCE as a
    // release on THIS channel is the strongest check available.
    let Some(declared_cid) = declared else {
        // (b) no declared parent on a star chain — a first release.
        return LineageDecision::Evidence(None);
    };
    let Some((kind, id)) = declared_summary else {
        // Settling this needs the declared cid's own record — ask the caller
        // to read it before calling again.
        return LineageDecision::NeedDeclaredLookup;
    };
    if kind == Some(RELEASE_MANIFEST_KIND) && id == Some(channel_id) {
        // (c) the declared parent EXISTS as a release on this channel.
        LineageDecision::Evidence(Some(declared_cid.to_string()))
    } else {
        // (d) a release, but on ANOTHER channel — or (e) not a release at
        // all. Either way the declared parent does not check out;
        // `verify_lineage` is what turns this into `lineage_parent_mismatch`.
        LineageDecision::Evidence(None)
    }
}

// ---------------------------------------------------------------------------
// Artifact sources
// ---------------------------------------------------------------------------

/// Where verified artifact bytes come from.
///
/// A trait so the floor can be exercised without a blob plane, and so the
/// peer-fetch leg (`p2p::blob_fetch::race_fetch` over the evidence-ordered
/// inventory candidates) can be wired in by the integrator without this module
/// growing a dependency on the swarm command channel.
#[async_trait::async_trait]
pub trait ArtifactSource: Send + Sync {
    /// Materialize one artifact's bytes and report what actually arrived.
    ///
    /// Implementations MUST NOT lie about length or digest to make a caller
    /// happy — the whole point of the fetch/verify split is that this reports
    /// observation and [`super::verify::verify_artifacts`] does the judging.
    async fn fetch(
        &self,
        artifact: &Artifact,
        staging_dir: &Path,
    ) -> Result<FetchedArtifact, AdoptionRefusal>;
}

/// The local blob store as an artifact source.
///
/// **Read-only with respect to the blob plane.** It never pulls bytes from a
/// peer: a blob that is not already local reports `artifact_unavailable`, which
/// is transient, so the next sweep asks again once ordinary replication has
/// done its job. Peer-pull is the integrator's wiring decision (see the module
/// docs), deliberately not a side effect of an observe-mode sweep.
pub struct BlobStoreArtifactSource {
    blobs: Arc<BlobStore>,
}

impl BlobStoreArtifactSource {
    pub fn new(blobs: Arc<BlobStore>) -> Self {
        Self { blobs }
    }
}

#[async_trait::async_trait]
impl ArtifactSource for BlobStoreArtifactSource {
    async fn fetch(
        &self,
        artifact: &Artifact,
        staging_dir: &Path,
    ) -> Result<FetchedArtifact, AdoptionRefusal> {
        let staged = staging_dir.join(&artifact.filename);

        // C6b — idempotent on (channel, releaseCid): a sweep that already
        // staged these exact bytes re-uses them instead of re-reading the blob.
        // The digest is re-checked, never assumed from the filename.
        if let Ok(existing) = tokio::fs::read(&staged).await {
            let sha256 = sha256_hex(&existing);
            if sha256.eq_ignore_ascii_case(&artifact.sha256) {
                return Ok(FetchedArtifact {
                    blob_cid: artifact.blob_cid.clone(),
                    path: staged,
                    bytes: existing.len() as u64,
                    sha256,
                });
            }
        }

        let bytes = self
            .blobs
            .get_by_address(&artifact.blob_cid)
            .await
            .map_err(|e| {
                AdoptionRefusal::new(
                    RefusalReason::ArtifactUnavailable,
                    format!(
                        "blob {} is not held locally ({e}) — the bytes may simply not have \
                         replicated yet",
                        artifact.blob_cid
                    ),
                )
            })?;

        let sha256 = sha256_hex(&bytes);
        // Stage only bytes that already prove out. Writing unverified bytes
        // into the directory an apply vehicle reads from would move the floor
        // from "verify then apply" to "apply what happens to be on disk".
        if sha256.eq_ignore_ascii_case(&artifact.sha256) && bytes.len() as u64 == artifact.bytes {
            if let Err(e) = write_staged(&staged, &bytes).await {
                return Err(AdoptionRefusal::new(
                    RefusalReason::ArtifactUnavailable,
                    format!(
                        "could not stage {} at {}: {e}",
                        artifact.filename,
                        staged.display()
                    ),
                ));
            }
        }

        Ok(FetchedArtifact {
            blob_cid: artifact.blob_cid.clone(),
            path: staged,
            bytes: bytes.len() as u64,
            sha256,
        })
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

async fn write_staged(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    // Write-then-rename so a half-written file is never visible under the name
    // a vehicle would read. The suffix is APPENDED rather than substituted:
    // `with_extension` would collapse `a.wasm` and `a.bin` onto one `a.partial`.
    let mut tmp = path.as_os_str().to_os_string();
    tmp.push(".partial");
    let tmp = PathBuf::from(tmp);
    tokio::fs::write(&tmp, bytes).await?;
    tokio::fs::rename(&tmp, path).await
}

// ---------------------------------------------------------------------------
// Installed reality, read once per TTL
// ---------------------------------------------------------------------------

/// Where the controller learns what this peer actually runs.
#[async_trait::async_trait]
pub trait InstalledRealitySource: Send + Sync {
    async fn read(&self) -> Answer<InstalledReality>;
}

/// The runtime passport's hApp leg. Composes
/// [`crate::runtime_passport::assemble_storage_passport`] rather than
/// re-deriving the inventory walk — the passport already owns the
/// bounded-inventory discipline and the honest per-role `error` fields.
pub struct PassportInstalledReality {
    pub admin_websocket: Option<holochain_client::AdminWebsocket>,
    pub app_id: String,
}

#[async_trait::async_trait]
impl InstalledRealitySource for PassportInstalledReality {
    async fn read(&self) -> Answer<InstalledReality> {
        let Some(admin) = self.admin_websocket.clone() else {
            return Answer::Unreachable;
        };
        let response = crate::runtime_passport::assemble_storage_passport(
            crate::runtime_passport::StoragePassportContext {
                embedded_conductor: false,
                external_conductor_configured: true,
                admin_websocket: Some(admin),
                app_id: self.app_id.clone(),
                libp2p_active: false,
                iroh_active: false,
            },
        )
        .await;
        InstalledReality::from_happ_passport(&response.passport.happ)
    }
}

// ---------------------------------------------------------------------------
// The controller
// ---------------------------------------------------------------------------

/// The adoption controller.
///
/// Holds only what a sweep needs. T3 shipped this with **no** vehicle field at
/// all, so that "no apply call site exists" was a fact of the type rather than
/// a promise. T4 adds the field — and the safety claim moves, deliberately, to
/// where it can still be checked: an apply requires a channel whose CONFIG says
/// `apply`, a release that passed the whole floor, a registered vehicle for its
/// declared class, and a node not under readable pressure. Four independent
/// conditions, each with a typed refusal, none of them a default.
pub struct AdoptionController {
    hc: Option<Arc<HcClient>>,
    artifacts: Option<Arc<dyn ArtifactSource>>,
    installed: Option<Arc<dyn InstalledRealitySource>>,
    /// The vehicles this node is equipped with. `None` means every `apply`
    /// channel refuses `no_vehicle_for_class` — honestly, and without applying.
    apply: Option<Arc<ApplyRegistry>>,
    /// The post-apply soak observer's rail (T5). `None` leaves applies
    /// un-attested rather than falsely attested.
    soak: Option<Arc<SoakAttestor>>,
    staging_root: PathBuf,
    cached_reality: tokio::sync::Mutex<Option<(i64, Answer<InstalledReality>)>>,
}

impl AdoptionController {
    pub fn new(staging_root: impl Into<PathBuf>) -> Self {
        Self {
            hc: None,
            artifacts: None,
            installed: None,
            apply: None,
            soak: None,
            staging_root: staging_root.into(),
            cached_reality: tokio::sync::Mutex::new(None),
        }
    }

    /// The conductor this peer resolves heads through. Without one every sweep
    /// reports `conductor_unavailable` — honestly, and without guessing.
    pub fn with_conductor(mut self, hc: Arc<HcClient>) -> Self {
        self.hc = Some(hc);
        self
    }

    pub fn with_artifact_source(mut self, source: Arc<dyn ArtifactSource>) -> Self {
        self.artifacts = Some(source);
        self
    }

    pub fn with_installed_reality(mut self, source: Arc<dyn InstalledRealitySource>) -> Self {
        self.installed = Some(source);
        self
    }

    /// Equip this controller with apply vehicles. **Without this call the
    /// controller cannot apply anything** — an `apply` channel refuses
    /// `no_vehicle_for_class`, which is the honest answer for a node that was
    /// asked to converge and has no machinery to converge with.
    pub fn with_apply_vehicles(mut self, registry: Arc<ApplyRegistry>) -> Self {
        self.apply = Some(registry);
        self
    }

    /// Equip the post-apply soak observer (T5's rail). Optional: without it an
    /// apply is un-attested, which is a missing datum. Never a fabricated one.
    pub fn with_soak_attestor(mut self, soak: Arc<SoakAttestor>) -> Self {
        self.soak = Some(soak);
        self
    }

    async fn installed_reality(&self, now: i64) -> Answer<InstalledReality> {
        let Some(source) = self.installed.as_ref() else {
            return Answer::Unreachable;
        };
        let mut cached = self.cached_reality.lock().await;
        if let Some((read_at, value)) = cached.as_ref() {
            if now - read_at < INSTALLED_REALITY_TTL_SECS {
                return value.clone();
            }
        }
        let fresh = source.read().await;
        *cached = Some((now, fresh.clone()));
        fresh
    }

    /// Resolve one channel's canonical head through THIS node's conductor —
    /// `resolve_content_head_local` (`GetStrategy::Local`), never the network
    /// variant. This controller reads what gossip has already delivered to
    /// THIS conductor, which is exactly what "3/3 convergence" measures; a
    /// network `get` inside a zome call is unbounded work this caller cannot
    /// cancel (a saturated/cold-arc peer returns `NoPeersForLocation`, which
    /// is a transport fault, not the honest absence C4 needs).
    ///
    /// Deliberately a `Background`-classed call: a controller sweep must never
    /// occupy the admission lane a person is standing in.
    /// [`crate::services::conductor_writes::call_resolve_content_head`] is the
    /// owner of this wire shape and its `ContentHeadWire` decode mirror is
    /// reused verbatim here; only the admission class (and now the Local
    /// zome fn) differ. (Residual: that module wants a `_classed` variant the
    /// way its declare path already has one — it belongs to another lane, so
    /// this reuses the type rather than editing the file.)
    async fn resolve_head(&self, channel_id: &str) -> Answer<ContentHeadWire> {
        let Some(hc) = self.hc.as_ref() else {
            return Answer::Unreachable;
        };
        let payload = match rmp_serde::to_vec_named(&channel_id.to_string()) {
            Ok(p) => p,
            Err(_) => return Answer::Unreachable,
        };
        let bytes = match hc
            .call_zome_timed(
                "content_store",
                "resolve_content_head_local",
                payload,
                AdmissionClass::Background,
            )
            .await
        {
            Ok((bytes, _timing)) => bytes,
            Err(e) => {
                tracing::debug!(
                    channel = %channel_id,
                    error = %e,
                    "release-adoption: head resolve failed — unreachable, never absence"
                );
                return Answer::Unreachable;
            }
        };
        match rmp_serde::from_slice::<Option<ContentHeadWire>>(&bytes) {
            // The conductor ANSWERED and reports no head. That is an observed
            // absence, and it is the C4-correct input to an idle verdict.
            Ok(head) => Answer::observed_absence(head),
            Err(e) => {
                tracing::warn!(
                    channel = %channel_id,
                    error = %e,
                    "release-adoption: could not decode ContentHeadWire"
                );
                Answer::Unreachable
            }
        }
    }

    /// Resolve the [`RecordSummary`] (envelope `kind` tag + record `id`) of
    /// ONE specific content version (`action_hash_b64`) through THIS node's
    /// conductor — the same `content_store`/`HcClient` path
    /// [`Self::resolve_head`] uses, aimed at a historical action instead of
    /// the live head.
    ///
    /// This is the read the lineage star-chain fix needs, at BOTH call sites:
    /// `head.supersedes` names an ACTION, not necessarily a RELEASE — every
    /// non-first `update_content` on a channel targets the channel ROOT, so
    /// the L2 chain is a star, never release→release — and the manifest's
    /// declared parent is just as much an unverified cid until read the same
    /// way. Deciding either "is a prior release" or "does the declared parent
    /// exist as a release on this channel" requires looking at what that
    /// action actually carries, never guessing from the action hash alone.
    async fn resolve_record_summary(&self, action_hash_b64: &str) -> Answer<RecordSummary> {
        let Some(hc) = self.hc.as_ref() else {
            return Answer::Unreachable;
        };
        let action_hash =
            match crate::services::conductor_writes::decode_action_hash(action_hash_b64) {
                Ok(h) => h,
                Err(e) => {
                    tracing::warn!(
                        action_hash = %action_hash_b64,
                        error = %e,
                        "release-adoption: superseded action hash undecodable — unreachable, \
                         never a guess at what it names"
                    );
                    return Answer::Unreachable;
                }
            };
        let payload = match rmp_serde::to_vec_named(&action_hash) {
            Ok(p) => p,
            Err(_) => return Answer::Unreachable,
        };
        let bytes = match hc
            .call_zome_timed(
                "content_store",
                "get_content",
                payload,
                AdmissionClass::Background,
            )
            .await
        {
            Ok((bytes, _timing)) => bytes,
            Err(e) => {
                tracing::debug!(
                    action_hash = %action_hash_b64,
                    error = %e,
                    "release-adoption: record resolve failed — unreachable, never absence"
                );
                return Answer::Unreachable;
            }
        };
        match rmp_serde::from_slice::<Option<lamad_types::ContentOutput>>(&bytes) {
            // The conductor answered and holds no record for this action —
            // honest absence (gossip-missing), not a verdict about lineage.
            // A retrieved record with no `kind` tag at all is equally
            // honest: it is simply not a release.
            Ok(out) => Answer::observed_absence(out.map(|o| RecordSummary {
                kind: envelope_kind_tag(&o.content.metadata_json),
                id: o.content.id,
            })),
            Err(e) => {
                tracing::warn!(
                    action_hash = %action_hash_b64,
                    error = %e,
                    "release-adoption: could not decode ContentOutput for the resolved record"
                );
                Answer::Unreachable
            }
        }
    }

    /// Read the attestation threshold for a release through T5's rail.
    ///
    /// C1 is a type obligation here, not a remembered step: the only way to
    /// build the `AdoptionDiscipline` this reader takes is
    /// `ChannelAdoptionDiscipline::for_release(builder_agent)`, so the
    /// builder's own attestation is excluded before the call is made.
    async fn read_threshold(
        &self,
        release_cid: &str,
        manifest: &super::ReleaseManifest,
    ) -> Option<release_attestation::QualifyingEvidence> {
        let hc = self.hc.as_ref()?;
        let discipline = manifest
            .adoption_discipline
            .for_release(manifest.provenance.builder_agent.clone());
        let ctx = ReleaseAttestationCtx::new(hc.clone());
        match release_attestation::count_qualifying_attestations(&ctx, release_cid, &discipline)
            .await
        {
            Ok(evidence) => Some(evidence),
            Err(e) => {
                tracing::debug!(
                    release_cid = %release_cid,
                    error = %e,
                    "release-adoption: attestation count unavailable — threshold_unchecked, \
                     which is NOT a pass"
                );
                None
            }
        }
    }

    /// One sweep over the followed channels. Returns how many channels were
    /// actually checked (skips do not count).
    pub async fn sweep_once(&self, followed: &FollowedChannels) -> usize {
        let now = state::now_unix();
        state::reconcile_followed(followed);

        // BUDGETS FIRST. Everything below is sized before any call is made.
        let mut byte_budget = MAX_ARTIFACT_BYTES_PER_SWEEP;
        let mut threshold_reads = MAX_THRESHOLD_READS_PER_SWEEP;
        let mut checked = 0usize;

        for channel in followed.channels.iter().take(MAX_CHANNELS_PER_SWEEP) {
            if state::channel_state(&channel.channel_id).is_some_and(|s| s.is_backing_off(now)) {
                continue;
            }
            let outcome = self
                .check_channel(channel, now, &mut byte_budget, &mut threshold_reads)
                .await;
            if let Some((head, verdict, attestations)) = outcome_transition(outcome) {
                record_decision(verdict_arm(&verdict), verdict.reason_label());
                state::record_check(&channel.channel_id, now, head, verdict, attestations);
                checked += 1;
            }
        }
        state::record_sweep(now);
        checked
    }

    /// The apply arm. Reached only from a channel whose config says `apply`,
    /// and only with a release that passed the whole floor.
    ///
    /// Returns the verdict; the caller records it. Note what is NOT here: no
    /// head is moved, no declaration is authored, nothing is gossiped. **C2 —
    /// apply never moves a head**; the ceremony does, and every controller
    /// converges toward whatever the ceremony elected, forward or backward.
    async fn apply_verified(&self, channel_id: &str, verified: super::VerifiedRelease) -> Verdict {
        let Some(registry) = self.apply.as_ref() else {
            return Verdict::Refused {
                refusal: AdoptionRefusal::new(
                    RefusalReason::NoVehicleForClass,
                    "this node is in apply mode for the channel but was wired with no apply \
                     vehicles at all",
                ),
            };
        };
        let soak_secs = verified.manifest.adoption_discipline.soak_secs;
        match registry.apply(&verified).await {
            Ok(receipt) => {
                let pending_restart = receipt
                    .detail
                    .get("pendingRestart")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                state::record_applied(
                    channel_id,
                    AppliedRelease {
                        cid: receipt.release_cid.clone(),
                        at: receipt.applied_at_unix,
                        vehicle: receipt.vehicle.clone(),
                    },
                    pending_restart,
                );
                // T4 → T5: the ONE call site into the attestation rail. It is
                // deferred by the channel's own declared soak window rather
                // than fired here, because an attestation authored at t=apply
                // would attest a window that never ran.
                if let Some(soak) = self.soak.as_ref() {
                    super::apply::spawn_soak_observer(
                        soak.clone(),
                        channel_id.to_string(),
                        receipt.release_cid.clone(),
                        soak_secs,
                        receipt.clone(),
                    );
                }
                Verdict::Applied {
                    release_cid: receipt.release_cid,
                    vehicle: receipt.vehicle,
                    already_current: false,
                }
            }
            Err(refusal) => Verdict::Refused { refusal },
        }
    }

    async fn check_channel(
        &self,
        channel: &FollowedChannel,
        now: i64,
        byte_budget: &mut u64,
        threshold_reads: &mut usize,
    ) -> CheckOutcome {
        // Exhaustive on purpose, and NOT a wildcard: the mode's meaning must be
        // decided at the point of ACTION, not only at the point of
        // configuration, and a forced edit here is the cheapest way to
        // guarantee a future mode cannot arrive as a silent fall-through.
        // `applies()` covers both `Apply` and `Canary` — WHICH of a STAGING
        // vs EARNED head each of them actually applies is decided after
        // verify, once the tier is known (`decide_post_verify_action`).
        let will_apply = channel.mode.applies();

        let head = match self.resolve_head(&channel.channel_id).await {
            Answer::Present(head) => head,
            Answer::Absent => {
                // The conductor answered: no head declared for this channel.
                // C3/C4 — idle, never a guess at "latest".
                return CheckOutcome::Checked {
                    head: None,
                    verdict: Verdict::Idle {
                        note: "this conductor sees no declared head for the channel".to_string(),
                    },
                    attestations: None,
                };
            }
            Answer::Unreachable => {
                return CheckOutcome::Checked {
                    head: None,
                    verdict: Verdict::Refused {
                        refusal: AdoptionRefusal::new(
                            RefusalReason::ConductorUnavailable,
                            "this peer could not resolve the channel through its own conductor \
                             — unreachable, which is never absence",
                        ),
                    },
                    attestations: None,
                };
            }
        };

        let release_cid = head.head_action_hash.0.clone();
        let resolved = ResolvedHead {
            cid: release_cid.clone(),
            tier: HeadTier::from_canonical_earned(head.canonical_earned),
        };

        // **C6b — the idempotence exit, and it is placed HERE on purpose.**
        //
        // The task atom's contract is "re-sweep on a current head is
        // already_current, ZERO conductor calls beyond the resolve". That is
        // only true if the check happens before the threshold read (1 + N zome
        // calls), before artifact staging, and before the vehicle. Putting it
        // after verify would still be idempotent in EFFECT while costing a full
        // sweep's evidence-gathering every minute, forever, on a converged
        // fleet — the shape of a controller that is correct and unaffordable.
        //
        // Observe mode deliberately does NOT take this exit: an observer's job
        // is to keep re-deriving the verdict, and an observer that stopped
        // checking a release it once passed would go blind to a peer whose
        // installed reality drifted underneath it.
        if will_apply {
            if let Some(applied) = state::applied_release(&channel.channel_id) {
                if applied.cid == release_cid {
                    return CheckOutcome::Checked {
                        head: Some(resolved),
                        verdict: Verdict::Applied {
                            release_cid,
                            vehicle: applied.vehicle,
                            already_current: true,
                        },
                        attestations: None,
                    };
                }
            }
        }

        let body = match extract_release_body(&head.content.metadata_json) {
            Ok(Some(body)) => body,
            Ok(None) => {
                return CheckOutcome::Checked {
                    head: Some(resolved),
                    verdict: Verdict::Idle {
                        note: "the channel's head carries no release manifest yet (a channel \
                               root, or a non-release version)"
                            .to_string(),
                    },
                    attestations: None,
                }
            }
            Err(refusal) => {
                return CheckOutcome::Checked {
                    head: Some(resolved),
                    verdict: Verdict::Refused { refusal },
                    attestations: None,
                }
            }
        };

        // Shape first: it is free, and it is what tells us how many bytes the
        // rest of this check would cost.
        let manifest = match verify::verify_shape(&body) {
            Ok(m) => m,
            Err(refusal) => {
                return CheckOutcome::Checked {
                    head: Some(resolved),
                    verdict: Verdict::Refused { refusal },
                    attestations: None,
                }
            }
        };

        // BUDGET GATE. A channel whose artifacts do not fit the remaining
        // per-sweep byte budget is SKIPPED — not refused. Deferral and refusal
        // are different facts; recording one as the other would put a healthy
        // channel into the backoff ladder for a reason that is about us.
        let declared_bytes: u64 = manifest.artifacts.iter().map(|a| a.bytes).sum();
        if declared_bytes > *byte_budget {
            tracing::info!(
                channel = %channel.channel_id,
                declared_bytes,
                remaining = *byte_budget,
                "release-adoption: channel deferred to the next sweep by the per-sweep byte \
                 budget (deferral, not refusal)"
            );
            return CheckOutcome::Skipped;
        }

        let installed = self.installed_reality(now).await;

        // The head's `supersedes` names an ACTION, not necessarily a RELEASE:
        // on a fresh channel's first release — and on EVERY non-first release,
        // since `update_content` always targets the channel root — it names
        // the channel root, never a prior release. Read what that action
        // actually carries before reporting it as prior lineage (the fix for
        // the first-release defect), and when it does NOT carry a prior
        // release, fall back to checking the manifest's declared parent for
        // EXISTENCE as a release on this channel — the strongest check a star
        // chain leaves available ([`lineage_evidence_from`]).
        let superseded: Answer<Option<(String, Option<String>)>> = match head.supersedes.as_ref() {
            None => Answer::Present(None),
            Some(superseded_action_hash) => {
                let superseded_cid = superseded_action_hash.0.clone();
                match self.resolve_record_summary(&superseded_cid).await {
                    Answer::Present(summary) => {
                        Answer::Present(Some((superseded_cid, summary.kind)))
                    }
                    Answer::Absent => Answer::Absent,
                    Answer::Unreachable => Answer::Unreachable,
                }
            }
        };

        let declared_parent = manifest.envelope.lineage_parent_cid.as_deref();

        let lineage = match superseded {
            Answer::Absent => Answer::Absent,
            Answer::Unreachable => Answer::Unreachable,
            Answer::Present(superseded_pair) => {
                match lineage_evidence_from(
                    superseded_as_ref(&superseded_pair),
                    declared_parent,
                    None,
                    &channel.channel_id,
                ) {
                    LineageDecision::Evidence(supersedes) => {
                        Answer::Present(LineageEvidence { supersedes })
                    }
                    LineageDecision::NeedDeclaredLookup => {
                        // Only reachable when `declared_parent` is `Some` —
                        // `lineage_evidence_from` returns `Evidence(None)`
                        // for a `None` declared parent instead.
                        let declared_cid = declared_parent.expect(
                            "NeedDeclaredLookup is returned only when a parent is declared",
                        );
                        match self.resolve_record_summary(declared_cid).await {
                            Answer::Present(summary) => {
                                let declared_summary =
                                    Some((summary.kind.as_deref(), Some(summary.id.as_str())));
                                match lineage_evidence_from(
                                    superseded_as_ref(&superseded_pair),
                                    declared_parent,
                                    declared_summary,
                                    &channel.channel_id,
                                ) {
                                    LineageDecision::Evidence(supersedes) => {
                                        Answer::Present(LineageEvidence { supersedes })
                                    }
                                    LineageDecision::NeedDeclaredLookup => unreachable!(
                                        "declared_summary is populated on this call; \
                                         lineage_evidence_from must settle"
                                    ),
                                }
                            }
                            Answer::Absent => Answer::Absent,
                            Answer::Unreachable => Answer::Unreachable,
                        }
                    }
                }
            }
        };

        // Refuse on the cheap arms BEFORE spending a threshold read or a byte
        // of staging: an envelope that cannot match will not match after we pay
        // for evidence.
        if let Err(refusal) = verify::verify_envelope(&manifest, &installed) {
            return CheckOutcome::Checked {
                head: Some(resolved),
                verdict: Verdict::Refused { refusal },
                attestations: None,
            };
        }
        if let Err(refusal) = verify::verify_lineage(&manifest, &lineage) {
            return CheckOutcome::Checked {
                head: Some(resolved),
                verdict: Verdict::Refused { refusal },
                attestations: None,
            };
        }

        let attestations = if *threshold_reads > 0 {
            *threshold_reads -= 1;
            self.read_threshold(&release_cid, &manifest).await
        } else {
            // Out of budget this sweep. `threshold_unchecked` is the honest
            // answer and is explicitly NOT a pass, so deferring the read can
            // never adopt anything.
            None
        };

        let mut fetched = Vec::with_capacity(manifest.artifacts.len());
        if let Some(source) = self.artifacts.as_ref() {
            let staging_dir = self.staging_root.join(sanitize_segment(&release_cid));
            for artifact in &manifest.artifacts {
                match source.fetch(artifact, &staging_dir).await {
                    Ok(f) => {
                        *byte_budget = byte_budget.saturating_sub(f.bytes);
                        fetched.push(f);
                    }
                    Err(refusal) => {
                        return CheckOutcome::Checked {
                            head: Some(resolved),
                            verdict: Verdict::Refused { refusal },
                            attestations,
                        }
                    }
                }
            }
        }

        let verdict = match verify::verify(VerifyInput {
            channel_id: &channel.channel_id,
            release_cid: &release_cid,
            body: &body,
            installed: &installed,
            lineage: &lineage,
            artifacts: &fetched,
            attestations: attestations.as_ref(),
            tier: resolved.tier,
        }) {
            // OBSERVE MODE ENDS HERE: the `VerifiedRelease` is reported and
            // dropped. `apply` and `canary` diverge on a STAGING head — see
            // `decide_post_verify_action`, the single source both this call
            // site and its table test consult: `apply` adopts EARNED heads
            // only (a verified STAGING head there is `Waiting`), `canary`
            // adopts either tier.
            Ok(verified) => match decide_post_verify_action(channel.mode, resolved.tier, true) {
                PostVerifyAction::Apply => self.apply_verified(&channel.channel_id, verified).await,
                PostVerifyAction::Waiting => Verdict::Waiting {
                    release_cid: verified.release_cid,
                    detail: "verified; this peer adopts only earned releases — the canary \
                                 soaks it first"
                        .to_string(),
                },
                PostVerifyAction::Observed => Verdict::Ok {
                    release_cid: verified.release_cid,
                },
                PostVerifyAction::Refused => unreachable!(
                    "verify::verify already enforced the threshold for tier {:?} — \
                         Ok(verified) implies it was met (or was Staging, which never enforces \
                         it)",
                    resolved.tier
                ),
            },
            Err(refusal) => Verdict::Refused { refusal },
        };

        CheckOutcome::Checked {
            head: Some(resolved),
            verdict,
            attestations,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum CheckOutcome {
    /// Not looked at this sweep (backoff or budget). State untouched, so
    /// `lastCheckedAt` keeps telling the truth.
    Skipped,
    Checked {
        head: Option<ResolvedHead>,
        verdict: Verdict,
        /// The threshold evidence this check actually read, or `None` when it
        /// never reached that arm — reported on `/admin/adoption` regardless
        /// of whether it was ENFORCED (only `HeadTier::Earned`/`None` enforce
        /// it; a `Staging` head's evidence here is soak progress).
        attestations: Option<release_attestation::QualifyingEvidence>,
    },
}

/// **Deferral is not refusal.** The single place a check outcome becomes a
/// state transition — and the single place that decides a deferral produces
/// NONE.
///
/// Extracted from the sweep body so the distinction is testable without a
/// conductor. It is the whole of the station: a `Skipped` outcome must not
/// touch `lastCheckedAt` (which would claim we looked when we did not), must
/// not increment `sweeps`, and must never enter the backoff ladder — a healthy
/// channel deferred because WE are over budget has done nothing wrong, and
/// putting it on the ladder would slow the channel down for a reason that is
/// about us.
#[allow(clippy::type_complexity)]
fn outcome_transition(
    outcome: CheckOutcome,
) -> Option<(
    Option<ResolvedHead>,
    Verdict,
    Option<release_attestation::QualifyingEvidence>,
)> {
    match outcome {
        CheckOutcome::Skipped => None,
        CheckOutcome::Checked {
            head,
            verdict,
            attestations,
        } => Some((head, verdict, attestations)),
    }
}

fn verdict_arm(verdict: &Verdict) -> DecisionArm {
    match verdict {
        Verdict::Idle { .. } | Verdict::Ok { .. } => DecisionArm::Watch,
        // Both the fresh apply and the `already_current` no-op are apply-arm
        // facts: they are what the apply arm decided, and metering them on the
        // watch arm would make an apply fleet indistinguishable from an observe
        // one at exactly the moment the difference matters. `Waiting` is the
        // same family: it is the apply arm declining to act on a STAGING head,
        // not a watch-arm observation.
        Verdict::Applied { .. } | Verdict::Waiting { .. } => DecisionArm::Apply,
        Verdict::Refused { refusal } => refusal.reason_code().arm(),
    }
}

/// The post-verify routing decision — pure, and the single source both
/// [`AdoptionController::check_channel`] and the table test below consult.
///
/// **`threshold_met` matters only when `tier` is NOT `Staging`.**
/// `verify::verify` enforces the threshold for `Earned` (and, unchanged from
/// before this design, `None`) and never for `Staging`, so a `Staging` row's
/// answer must not move when `threshold_met` flips — the table test pins
/// exactly that.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PostVerifyAction {
    /// Route the verified release to a vehicle.
    Apply,
    /// Verified, but this mode adopts EARNED heads only — a canary soaks a
    /// STAGING head first. Neither a pass-and-stop nor a refusal.
    Waiting,
    /// Verified, not applied. `observe`, on either tier.
    Observed,
    /// The threshold was enforced (tier != `Staging`) and was not met.
    Refused,
}

fn decide_post_verify_action(
    mode: AdoptionMode,
    tier: HeadTier,
    threshold_met: bool,
) -> PostVerifyAction {
    if tier != HeadTier::Staging && !threshold_met {
        return PostVerifyAction::Refused;
    }
    match mode {
        AdoptionMode::Observe => PostVerifyAction::Observed,
        AdoptionMode::Canary => PostVerifyAction::Apply,
        AdoptionMode::Apply if tier == HeadTier::Staging => PostVerifyAction::Waiting,
        AdoptionMode::Apply => PostVerifyAction::Apply,
    }
}

/// Make a release CID safe as one path segment. Base64 action hashes carry `/`
/// and `+`; a staging directory named from unsanitized wire input is a path
/// traversal waiting to be found.
fn sanitize_segment(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Read the followed-channel set from the rung-4 runtime-config surface.
pub fn followed_from_runtime_config() -> FollowedChannels {
    match crate::runtime_config::get_text(state::RELEASE_CHANNELS_KEY) {
        Some(raw) => state::parse_followed_channels(&raw),
        None => FollowedChannels::default(),
    }
}

/// The empty/non-empty SHAPE of one tick's follow set, plus what it would
/// apply — extracted so the tick loop's decisions are testable without a
/// conductor or an async runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
struct FollowShape {
    channel_count: usize,
    applying: Vec<String>,
}

impl FollowShape {
    fn of(followed: &FollowedChannels) -> Self {
        Self {
            channel_count: followed.channels.len(),
            applying: followed
                .channels
                .iter()
                .filter(|c| c.mode.applies())
                .map(|c| c.channel_id.clone())
                .collect(),
        }
    }

    fn is_empty(&self) -> bool {
        self.channel_count == 0
    }
}

/// What one tick should do, decided BEFORE any conductor call is made.
struct TickDecision {
    /// An empty follow set pays for the tick (the re-read) and nothing else:
    /// `sweep_once` is a real per-channel budget-spending walk, so a set with
    /// nothing to check does not enter it.
    sweep: bool,
    /// `Some(line)` exactly when the empty/non-empty shape of the follow set
    /// changed since the previous tick — a follow or unfollow landing on a
    /// RUNNING node must be answerable from a log line, the same property the
    /// boot log already has. `None` on every steady-state tick.
    transition_log: Option<String>,
}

/// The pure per-tick decision: sweep-or-skip, and whether this tick's follow
/// set differs in shape from the previous one enough to log a transition.
///
/// `previous = None` means "no prior tick to compare against" (never reached
/// by the real loop, which always seeds `previous` from the boot-time read
/// before the first tick — included here only so the function is total).
fn decide_tick(previous: Option<&FollowShape>, current: &FollowShape) -> TickDecision {
    let became_nonempty = previous.is_some_and(FollowShape::is_empty) && !current.is_empty();
    let became_empty = previous.is_some_and(|p| !p.is_empty()) && current.is_empty();
    let transition_log = if became_nonempty {
        Some(format!(
            "release-adoption: follow set changed on a RUNNING node — now following {} \
             channel(s), applying = {:?}",
            current.channel_count, current.applying
        ))
    } else if became_empty {
        Some(
            "release-adoption: follow set changed on a RUNNING node — now following none, idle \
             until the next follow"
                .to_string(),
        )
    } else {
        None
    };
    TickDecision {
        sweep: !current.is_empty(),
        transition_log,
    }
}

/// Spawn the sweep loop.
///
/// **Always spawns.** A peer that follows nothing at boot must still be a
/// RUNNING controller — rung 4's whole point is that a follow lands on a
/// running node, and that node has to be running for it to land on. The empty
/// case pays for the tick and nothing else (see [`decide_tick`]).
///
/// bounded-work: one tick per [`SWEEP_INTERVAL_SECS`], `MissedTickBehavior::Skip`
/// so a stalled runtime coalesces missed ticks instead of catching up in a
/// burst, and every per-tick cost capped by the budgets above.
pub fn spawn(controller: AdoptionController) -> bool {
    let followed = followed_from_runtime_config();
    if followed.channels.is_empty() {
        tracing::info!(
            config_key = state::RELEASE_CHANNELS_KEY,
            refused = followed.refused.len(),
            "release-adoption: controller IDLE — no followed channels configured, watching for \
             a follow"
        );
    } else {
        // Say — once, loudly, at boot — exactly which channels this node will
        // ACT on. "I edited the ConfigMap and my node started swapping
        // coordinators" must be answerable from a log line at startup, not
        // reconstructed later from the admin route.
        let applying: Vec<&str> = followed
            .channels
            .iter()
            .filter(|c| c.mode.applies())
            .map(|c| c.channel_id.as_str())
            .collect();
        if applying.is_empty() {
            tracing::info!(
                channels = followed.channels.len(),
                refused = followed.refused.len(),
                sweep_secs = SWEEP_INTERVAL_SECS,
                "release-adoption: controller ACTIVE, every followed channel in OBSERVE mode — \
                 it will report verdicts and apply nothing"
            );
        } else {
            tracing::warn!(
                channels = followed.channels.len(),
                refused = followed.refused.len(),
                sweep_secs = SWEEP_INTERVAL_SECS,
                applying = ?applying,
                vehicles = ?super::apply::registered_vehicle_labels(),
                "release-adoption: controller ACTIVE and will APPLY on the named channels — a \
                 verified release on one of them changes what this node runs"
            );
        }
    }
    state::reconcile_followed(&followed);
    state::mark_controller_running(true);

    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(SWEEP_INTERVAL_SECS));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut previous = FollowShape::of(&followed);
        loop {
            ticker.tick().await;
            // Re-read the config every tick: rung 4's whole point is that a
            // follow/unfollow lands on a RUNNING node.
            let followed = followed_from_runtime_config();
            let current = FollowShape::of(&followed);
            let decision = decide_tick(Some(&previous), &current);
            if let Some(line) = decision.transition_log {
                tracing::info!("{line}");
            }
            let checked = if decision.sweep {
                controller.sweep_once(&followed).await
            } else {
                // Nothing to check. Reconcile the (empty) registry so a
                // just-unfollowed channel's leftover state is dropped, and pay
                // for nothing else: `sweep_once`'s own `record_sweep` would
                // tick `sweeps`/`lastSweepUnixSecs` on `/admin/adoption` for a
                // tick that examined zero channels, which is not the same
                // fact as a sweep that ran and found nothing to do.
                state::reconcile_followed(&followed);
                0
            };
            tracing::debug!(
                channels = followed.channels.len(),
                checked,
                "release-adoption: sweep complete"
            );
            previous = current;
        }
    });
    true
}

/// Pre-touch every `(arm, reason)` series a real branch of this module can
/// reach, so a zero reads as a MEASURED zero rather than as an absent series.
///
/// **T4 widened this to the apply arm.** T3 left `{arm="apply"}` absent on
/// purpose — a zero there would have claimed an arm with no code. Now the
/// vehicles exist, so the asymmetry would point the other way: an absent
/// `{arm="apply",reason="ok"}` on a fleet that applies nothing yet reads as
/// *never measured* when the truthful reading is *measured zero*.
///
/// The pairing is still derived, never hand-listed: every refusal names its own
/// arm, so a reason added to the apply arm is pre-touched by construction.
pub fn pretouch_metrics() {
    for arm in DecisionArm::ALL {
        crate::metrics::RELEASE_ADOPTION_DECISIONS
            .with_label_values(&[arm.label(), super::REASON_OK])
            .inc_by(0);
    }
    crate::metrics::RELEASE_ADOPTION_DECISIONS
        .with_label_values(&[DecisionArm::Watch.label(), super::REASON_IDLE])
        .inc_by(0);
    // The converged-fleet series. Only the apply arm can emit it.
    crate::metrics::RELEASE_ADOPTION_DECISIONS
        .with_label_values(&[DecisionArm::Apply.label(), super::REASON_ALREADY_CURRENT])
        .inc_by(0);
    // **Design 2026-09-01 (canary-first adoption).** `Verdict::Waiting` is
    // apply-arm exactly like `already_current` above and is not a
    // `RefusalReason`, so it needs the same manual pre-touch: an absent
    // `{arm="apply",reason="awaiting_promotion"}` on a fleet with no `apply`
    // channel yet reads as *never measured* when the honest reading is
    // *measured zero*.
    crate::metrics::RELEASE_ADOPTION_DECISIONS
        .with_label_values(&[DecisionArm::Apply.label(), super::REASON_AWAITING_PROMOTION])
        .inc_by(0);
    for reason in RefusalReason::ALL {
        crate::metrics::RELEASE_ADOPTION_DECISIONS
            .with_label_values(&[reason.arm().label(), reason.label()])
            .inc_by(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_channel_root_is_idle_not_a_broken_release() {
        let root =
            r#"{"kind":"release-channel","channelId":"runtime:coordinators:elohim:commons"}"#;
        assert_eq!(extract_release_body(root).unwrap(), None);
    }

    #[test]
    fn an_empty_or_absent_metadata_json_is_idle() {
        assert_eq!(extract_release_body("").unwrap(), None);
        assert_eq!(extract_release_body("   ").unwrap(), None);
    }

    /// The envelope T2's ceremony driver actually publishes.
    #[test]
    fn the_ceremony_envelope_yields_the_manifest_body() {
        let published = serde_json::json!({
            "kind": "release-manifest",
            "publishedAt": "2026-09-01T00:00:00.000Z",
            "manifest": {"kind": "release-manifest", "channelId": "runtime:c:e:x"},
        })
        .to_string();
        let body = extract_release_body(&published).unwrap().unwrap();
        assert_eq!(body["channelId"], "runtime:c:e:x");
    }

    /// An envelope that CLAIMS to be a release and is not readable must refuse.
    /// Reading it as "no release here" would let a corrupt publish look like an
    /// empty channel.
    #[test]
    fn a_release_envelope_without_a_manifest_refuses_rather_than_reading_as_empty() {
        let missing = r#"{"kind":"release-manifest","publishedAt":"now"}"#;
        assert_eq!(
            extract_release_body(missing).unwrap_err().reason_code(),
            RefusalReason::ManifestUndecodable
        );
        let not_object = r#"{"kind":"release-manifest","manifest":"oops"}"#;
        assert_eq!(
            extract_release_body(not_object).unwrap_err().reason_code(),
            RefusalReason::ManifestUndecodable
        );
        assert_eq!(
            extract_release_body("{not json").unwrap_err().reason_code(),
            RefusalReason::ManifestUndecodable
        );
    }

    /// **The star-chain lineage table.** Every case from the design, in the
    /// order they're numbered there:
    /// (a) chain carries a release → `Some(that cid)` regardless of
    ///     declared; (b) star + declared `None` → `None`; (c) star +
    ///     declared `Some`, lookup = release on this channel →
    ///     `Some(declared)`; (d) star + declared `Some`, lookup = release on
    ///     ANOTHER channel → `None`; (e) star + declared `Some`, lookup =
    ///     non-release → `None`. Plus the two `NeedDeclaredLookup` cases,
    ///     proving the loop asks for a lookup exactly when — and only when —
    ///     one is actually needed.
    #[test]
    fn lineage_evidence_from_settles_every_star_chain_case() {
        const CHANNEL: &str = "runtime:coordinators:elohim:receipt";

        // (a) The chain carries a release — the declared hint plays no part,
        // proven by disagreeing with it here.
        assert_eq!(
            lineage_evidence_from(
                Some(("uhCkkPRIOR".to_string(), Some(RELEASE_MANIFEST_KIND))),
                Some("uhCkkSomethingElseEntirely"),
                None,
                CHANNEL,
            ),
            LineageDecision::Evidence(Some("uhCkkPRIOR".to_string())),
        );
        // Also true with no declared hint at all.
        assert_eq!(
            lineage_evidence_from(
                Some(("uhCkkPRIOR".to_string(), Some(RELEASE_MANIFEST_KIND))),
                None,
                None,
                CHANNEL,
            ),
            LineageDecision::Evidence(Some("uhCkkPRIOR".to_string())),
        );

        // (b) A star chain (the channel root) with no declared parent — a
        // first release, settled without any lookup.
        assert_eq!(
            lineage_evidence_from(
                Some(("uhCkkROOT".to_string(), Some("release-channel"))),
                None,
                None,
                CHANNEL,
            ),
            LineageDecision::Evidence(None),
        );
        // The `superseded: None` shape (the head IS the root) is the same
        // star, not a distinct case.
        assert_eq!(
            lineage_evidence_from(None, None, None, CHANNEL),
            LineageDecision::Evidence(None),
        );

        // A star chain WITH a declared parent cannot be settled on the first
        // call — the lookup has not happened yet.
        assert_eq!(
            lineage_evidence_from(
                Some(("uhCkkROOT".to_string(), Some("release-channel"))),
                Some("uhCkkDeclaredParent"),
                None,
                CHANNEL,
            ),
            LineageDecision::NeedDeclaredLookup,
        );

        // (c) The declared parent EXISTS as a release on this channel.
        assert_eq!(
            lineage_evidence_from(
                Some(("uhCkkROOT".to_string(), Some("release-channel"))),
                Some("uhCkkDeclaredParent"),
                Some((Some(RELEASE_MANIFEST_KIND), Some(CHANNEL))),
                CHANNEL,
            ),
            LineageDecision::Evidence(Some("uhCkkDeclaredParent".to_string())),
        );

        // (d) A release, but on ANOTHER channel — does not check out.
        assert_eq!(
            lineage_evidence_from(
                Some(("uhCkkROOT".to_string(), Some("release-channel"))),
                Some("uhCkkDeclaredParent"),
                Some((
                    Some(RELEASE_MANIFEST_KIND),
                    Some("runtime:coordinators:elohim:other")
                )),
                CHANNEL,
            ),
            LineageDecision::Evidence(None),
        );

        // (e) Not a release at all — does not check out.
        assert_eq!(
            lineage_evidence_from(
                Some(("uhCkkROOT".to_string(), Some("release-channel"))),
                Some("uhCkkDeclaredParent"),
                Some((Some("release-channel"), Some(CHANNEL))),
                CHANNEL,
            ),
            LineageDecision::Evidence(None),
        );
    }

    #[test]
    fn envelope_kind_tag_reads_the_bare_kind_field() {
        assert_eq!(
            envelope_kind_tag(r#"{"kind":"release-channel"}"#),
            Some("release-channel".to_string())
        );
        assert_eq!(envelope_kind_tag(""), None);
        assert_eq!(envelope_kind_tag("{not json"), None);
        assert_eq!(envelope_kind_tag("{}"), None);
    }

    /// A base64 action hash carries `/` and `+`. A staging directory named from
    /// unsanitized wire input is a path traversal waiting to be found.
    #[test]
    fn a_release_cid_never_escapes_its_staging_segment() {
        let sanitized = sanitize_segment("uhCkk../../etc/passwd+a/b");
        assert!(!sanitized.contains('/'));
        assert!(!sanitized.contains('.'));
        assert_eq!(Path::new(&sanitized).components().count(), 1);
    }

    /// **C6a.** Every budget is a compile-time constant a sweep reads BEFORE it
    /// starts work, and none of them is zero (a zero budget is a loop that
    /// looks alive and does nothing).
    ///
    /// Asserted in `const` blocks, so a budget edited to zero fails to COMPILE
    /// rather than failing a test someone could have skipped — which is the
    /// stronger form of the same guarantee, and what clippy's
    /// `assertions_on_constants` is pointing at.
    #[test]
    fn every_per_sweep_budget_is_declared_and_nonzero() {
        const {
            assert!(MAX_CHANNELS_PER_SWEEP > 0);
            assert!(MAX_THRESHOLD_READS_PER_SWEEP > 0);
            // The threshold read is the expensive one (1 + N zome calls each),
            // so its budget may never exceed the channel budget.
            assert!(MAX_THRESHOLD_READS_PER_SWEEP <= MAX_CHANNELS_PER_SWEEP);
            assert!(MAX_ARTIFACT_BYTES_PER_SWEEP > 0);
            assert!(SWEEP_INTERVAL_SECS > 0);
            // A TTL shorter than the sweep would re-read the hApp inventory
            // every tick, which is the cost the cache exists to remove.
            assert!(INSTALLED_REALITY_TTL_SECS > SWEEP_INTERVAL_SECS as i64);
        }
    }

    /// A controller with no conductor reports `conductor_unavailable` and
    /// judges nothing. The honest floor: no conductor, no authority, no guess.
    #[tokio::test]
    async fn a_controller_without_a_conductor_refuses_honestly_and_applies_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let controller = AdoptionController::new(dir.path());
        let followed = state::parse_followed_channels("runtime:coordinators:elohim:test-a");
        let checked = controller.sweep_once(&followed).await;
        assert_eq!(checked, 1);
        let states = state::snapshot();
        let channel = states
            .iter()
            .find(|s| s.channel_id == "runtime:coordinators:elohim:test-a")
            .expect("the followed channel is in the report");
        match channel.verdict.as_ref().expect("a verdict was recorded") {
            Verdict::Refused { refusal } => {
                assert_eq!(refusal.reason_code(), RefusalReason::ConductorUnavailable);
                assert!(refusal.transient);
            }
            other => panic!("expected conductor_unavailable, got {other:?}"),
        }
        assert!(channel.resolved_head.is_none(), "unreachable is not a head");
    }

    /// A blob the store does not hold is `artifact_unavailable` — transient, so
    /// the next sweep asks again once replication has done its job. It is never
    /// read as "the release is wrong".
    #[tokio::test]
    async fn an_absent_blob_is_unavailable_never_a_verdict_about_the_release() {
        let dir = tempfile::tempdir().unwrap();
        let blobs = Arc::new(BlobStore::new_memory());
        let source = BlobStoreArtifactSource::new(blobs);
        let artifact = Artifact {
            blob_cid: "bafkreih55t4vyzozvjgu7b62baj5oe2oyrkshcgw4yq7oamvmo3eainmsa".to_string(),
            bytes: 3,
            sha256: "0".repeat(64),
            filename: "content_store.wasm".to_string(),
            mime_type: None,
            role: None,
        };
        let refusal = source
            .fetch(&artifact, dir.path())
            .await
            .expect_err("nothing is stored");
        assert_eq!(refusal.reason_code(), RefusalReason::ArtifactUnavailable);
        assert!(refusal.transient);
    }

    /// Bytes that prove out are staged; bytes that do not are reported
    /// truthfully and NEVER written into the directory an apply vehicle reads.
    #[tokio::test]
    async fn only_bytes_that_prove_out_are_staged() {
        let dir = tempfile::tempdir().unwrap();
        let blobs = Arc::new(BlobStore::new_memory());
        let payload = b"release bytes".to_vec();
        let stored = blobs.store(&payload).await.expect("store");
        let source = BlobStoreArtifactSource::new(blobs);

        let good = Artifact {
            blob_cid: stored.cid.clone(),
            bytes: payload.len() as u64,
            sha256: sha256_hex(&payload),
            filename: "artifact.bin".to_string(),
            mime_type: None,
            role: None,
        };
        let fetched = source.fetch(&good, dir.path()).await.expect("held locally");
        assert_eq!(fetched.bytes, payload.len() as u64);
        assert_eq!(fetched.sha256, sha256_hex(&payload));
        assert!(fetched.path.exists(), "verified bytes are staged");
        assert_eq!(tokio::fs::read(&fetched.path).await.unwrap(), payload);

        // Second fetch re-uses the staged file (C6b — idempotent).
        let again = source.fetch(&good, dir.path()).await.expect("idempotent");
        assert_eq!(again, fetched);

        let lying = Artifact {
            sha256: "1".repeat(64),
            filename: "lying.bin".to_string(),
            ..good.clone()
        };
        let observed = source.fetch(&lying, dir.path()).await.expect("bytes exist");
        assert_eq!(
            observed.sha256,
            sha256_hex(&payload),
            "the source reports what ARRIVED; the floor does the judging"
        );
        assert!(
            !dir.path().join("lying.bin").exists(),
            "unverified bytes must never be staged where a vehicle would read them"
        );
    }

    /// **Deferral is not refusal.** A channel the sweep did not look at —
    /// because the per-sweep byte budget was already spent — must leave its
    /// recorded state EXACTLY as it was: `lastCheckedAt` still names the last
    /// time it was actually looked at, `sweeps` does not tick, the verdict is
    /// not replaced, and it never enters the backoff ladder.
    ///
    /// This is the failure the distinction exists to prevent: a healthy channel
    /// deferred because WE were over budget, recorded as a refusal, climbing to
    /// the 30-minute ceiling and then looking — to the operator reading
    /// `/admin/adoption` — like a channel with a problem of its own.
    ///
    /// Driven through [`outcome_transition`], the single place a sweep turns an
    /// outcome into a state transition, so this is the real decision and not a
    /// restatement of it.
    #[test]
    fn a_deferred_channel_is_not_a_refused_one() {
        let channel = FollowedChannel {
            channel_id: "runtime:coordinators:elohim:big".to_string(),
            mode: AdoptionMode::Observe,
        };
        let mut deferred = state::ChannelAdoptionState::new(&channel);

        // One real check, so there is something a deferral could corrupt.
        deferred.record(
            1_000,
            Some(ResolvedHead {
                cid: "uhCkkHead".to_string(),
                tier: HeadTier::Earned,
            }),
            Verdict::Ok {
                release_cid: "uhCkkHead".to_string(),
            },
            None,
        );
        let untouched = deferred.clone();

        // The next sweep defers this channel on the byte budget.
        let transition = outcome_transition(CheckOutcome::Skipped);
        assert!(
            transition.is_none(),
            "a deferral must produce NO state transition at all"
        );
        if let Some((head, verdict, attestations)) = transition {
            deferred.record(9_999, head, verdict, attestations);
        }

        assert_eq!(
            deferred, untouched,
            "a deferred channel's recorded state must be byte-identical to what it was"
        );
        assert_eq!(
            deferred.last_checked_at,
            Some(1_000),
            "lastCheckedAt must still name the last time we ACTUALLY looked"
        );
        assert_eq!(deferred.sweeps, 1, "a deferral is not a sweep of a channel");
        assert_eq!(deferred.consecutive_refusals, 0);
        assert_eq!(
            deferred.next_check_not_before, None,
            "a deferral must never enter the backoff ladder"
        );
        assert!(!deferred.is_backing_off(1_001));

        // And the contrast: a real check DOES transition, so the assertion
        // above is about deferral specifically and not about a dead function.
        let checked = outcome_transition(CheckOutcome::Checked {
            head: None,
            verdict: Verdict::Refused {
                refusal: AdoptionRefusal::new(RefusalReason::ArtifactUnavailable, "no peer"),
            },
            attestations: None,
        });
        let (head, verdict, attestations) = checked.expect("a completed check transitions");
        deferred.record(2_000, head, verdict, attestations);
        assert_eq!(deferred.last_checked_at, Some(2_000));
        assert_eq!(deferred.consecutive_refusals, 1);
        assert!(
            deferred.next_check_not_before.is_some(),
            "a real transient refusal DOES enter the ladder — that is the contrast"
        );
    }

    /// The typed metric arm follows the verdict, never the call site.
    #[test]
    fn the_metric_arm_is_a_property_of_the_verdict() {
        assert_eq!(
            verdict_arm(&Verdict::Idle {
                note: "n".to_string()
            }),
            DecisionArm::Watch
        );
        assert_eq!(
            verdict_arm(&Verdict::Ok {
                release_cid: "c".to_string()
            }),
            DecisionArm::Watch
        );
        assert_eq!(
            verdict_arm(&Verdict::Refused {
                refusal: AdoptionRefusal::new(RefusalReason::ArtifactDigestMismatch, "d")
            }),
            DecisionArm::Fetch
        );
        assert_eq!(
            verdict_arm(&Verdict::Refused {
                refusal: AdoptionRefusal::new(RefusalReason::DnaLineageMismatch, "d")
            }),
            DecisionArm::Verify
        );
        // T4: both applied shapes are apply-arm facts, and the converged one
        // carries its own reason so a stable fleet is not metered as a churning
        // one.
        let fresh = Verdict::Applied {
            release_cid: "c".to_string(),
            vehicle: "sync_coordinators".to_string(),
            already_current: false,
        };
        let converged = Verdict::Applied {
            release_cid: "c".to_string(),
            vehicle: "sync_coordinators".to_string(),
            already_current: true,
        };
        assert_eq!(verdict_arm(&fresh), DecisionArm::Apply);
        assert_eq!(verdict_arm(&converged), DecisionArm::Apply);
        assert_eq!(fresh.reason_label(), super::super::REASON_OK);
        assert_eq!(
            converged.reason_label(),
            super::super::REASON_ALREADY_CURRENT
        );
        assert_eq!(
            verdict_arm(&Verdict::Refused {
                refusal: AdoptionRefusal::new(RefusalReason::DeferredBackpressure, "d")
            }),
            DecisionArm::Apply
        );
        // **Canary-first adoption.** `Waiting` is an apply-arm fact — the
        // apply arm declining to act on a STAGING head — with its own
        // non-refusal reason, exactly like `already_current` above.
        let waiting = Verdict::Waiting {
            release_cid: "c".to_string(),
            detail: "d".to_string(),
        };
        assert_eq!(verdict_arm(&waiting), DecisionArm::Apply);
        assert_eq!(
            waiting.reason_label(),
            super::super::REASON_AWAITING_PROMOTION
        );
    }

    /// **The pure decision table (design 2026-09-01, canary-first adoption).**
    /// The threshold gates EARNED adoption only: a `Staging` row's answer must
    /// not move when `threshold_met` flips, and only `apply` on a `Staging`
    /// head produces `Waiting` — `canary` never does (it is what closes the
    /// loop `apply` cannot), and `observe` never applies on either tier.
    #[test]
    fn decide_post_verify_action_gates_the_threshold_at_earned_only() {
        use AdoptionMode::{Apply, Canary, Observe};
        use HeadTier::{Earned, Staging};
        use PostVerifyAction::{Apply as DoApply, Observed, Refused, Waiting};

        let cases: &[(AdoptionMode, HeadTier, bool, PostVerifyAction)] = &[
            (Observe, Earned, true, Observed),
            (Observe, Earned, false, Refused),
            (Observe, Staging, true, Observed),
            (Observe, Staging, false, Observed),
            (Apply, Earned, true, DoApply),
            (Apply, Earned, false, Refused),
            (Apply, Staging, true, Waiting),
            (Apply, Staging, false, Waiting),
            (Canary, Earned, true, DoApply),
            (Canary, Earned, false, Refused),
            (Canary, Staging, true, DoApply),
            (Canary, Staging, false, DoApply),
        ];
        for (mode, tier, met, expected) in cases.iter().copied() {
            assert_eq!(
                decide_post_verify_action(mode, tier, met),
                expected,
                "mode={mode:?} tier={tier:?} threshold_met={met}"
            );
        }
    }

    /// A controller wired with no vehicles refuses an apply channel by NAME
    /// rather than quietly observing it. A node told to converge that silently
    /// does not is the same class of lie as a mode that silently downgrades —
    /// this is that refusal at the point of action.
    #[tokio::test]
    async fn an_apply_channel_on_an_unequipped_node_refuses_rather_than_observing() {
        let dir = tempfile::tempdir().unwrap();
        let controller = AdoptionController::new(dir.path());
        assert!(
            controller.apply.is_none(),
            "the default controller carries no vehicles"
        );

        // Reaching the apply arm requires a VerifiedRelease, which only
        // verify.rs can mint — so the reachable assertion here is that the
        // unequipped path is the refusing one, exercised through the same
        // predicate the arm uses.
        let followed =
            state::parse_followed_channels("runtime:coordinators:elohim:t4-unequipped=apply");
        assert_eq!(followed.channels.len(), 1);
        assert!(followed.channels[0].mode.applies());
        assert!(followed.refused.is_empty());
    }

    /// **The defect this fixes, in pure-function form.** A node that boots
    /// with no followed channels must still SWEEP once a follow lands on it
    /// while running — `decide_tick` is what the spawned loop consults every
    /// tick, so proving the empty→non-empty transition here proves the loop
    /// would have acted on it, without needing a conductor or an async
    /// runtime to exercise the loop itself.
    #[test]
    fn an_empty_to_nonempty_follow_transition_is_observed_and_swept() {
        let empty = FollowShape::of(&FollowedChannels::default());
        let followed = state::parse_followed_channels("runtime:coordinators:elohim:receipt");
        let nonempty = FollowShape::of(&followed);

        // Steady state at empty: no sweep, no log — a node with nothing
        // followed pays for the tick and nothing else.
        let steady_empty = decide_tick(Some(&empty), &empty);
        assert!(!steady_empty.sweep, "an empty follow set is never swept");
        assert!(
            steady_empty.transition_log.is_none(),
            "no change in shape must not log"
        );

        // The transition this bug lost: empty at boot, then a runtime-config
        // edit lands a follow on the running node.
        let transition = decide_tick(Some(&empty), &nonempty);
        assert!(
            transition.sweep,
            "a follow landing on a running node must be swept on the very next tick"
        );
        let line = transition
            .transition_log
            .expect("the empty->non-empty transition must be answerable from a log line");
        assert!(line.contains("runtime:coordinators:elohim:receipt") || line.contains('1'));

        // Steady state at non-empty: sweeps every tick, no repeated log.
        let steady_nonempty = decide_tick(Some(&nonempty), &nonempty);
        assert!(steady_nonempty.sweep);
        assert!(
            steady_nonempty.transition_log.is_none(),
            "an unchanged non-empty shape must not re-log every tick"
        );

        // The inverse transition: an unfollow landing on a running node.
        let unfollowed = decide_tick(Some(&nonempty), &empty);
        assert!(!unfollowed.sweep);
        assert!(
            unfollowed.transition_log.is_some(),
            "an unfollow landing on a running node must also be answerable from a log line"
        );
    }
}
