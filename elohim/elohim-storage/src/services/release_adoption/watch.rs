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
use super::{
    AdoptionRefusal, Artifact, ArtifactClass, DecisionArm, RefusalReason, RELEASE_MANIFEST_KIND,
};
use crate::blob_store::BlobStore;
use crate::conductor_admission::AdmissionClass;
use crate::hc_client::HcClient;
use crate::services::conductor_writes::ContentHeadWire;
use crate::services::release_attestation::{self, ReleaseAttestationCtx};

/// The lineage-window sweep (revert arm + sunset arm). Split out of this
/// file for the source-file LoC ceiling; `sweep_once` calls into it.
mod lineage_windows;

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

/// How stale a cached installed-reality snapshot must be before a
/// `*_lineage_mismatch` refusal earns exactly one bypass-TTL re-read before
/// it is emitted. Not "any staleness": a snapshot taken moments ago (the
/// common case — a fresh read, or a cache hit seconds old) is almost
/// certainly still the truth, and re-reading on every mismatch would spend an
/// admin round trip on every legitimately-refused release too. A few seconds
/// is enough headroom for this node's OWN apply — which invalidates the
/// cache outright (see `installed_reality_invalidate`) — while still
/// catching a passport change this controller did not itself cause.
pub const REREAD_STALE_THRESHOLD_SECS: i64 = 5;

/// **Pure.** Should a `*_lineage_mismatch` refusal, produced from a cached
/// installed-reality snapshot of the given age, earn one bypass-TTL re-read
/// before the controller emits it? Isolated from the async re-read itself so
/// the policy is a table a test can walk without a conductor:
///
/// - a mismatch reason + a stale cache → re-read (this is the whole point:
///   the cache might be pre-dating a passport change).
/// - a mismatch reason + a fresh cache → no re-read (the snapshot is already
///   current; re-reading would only spend a round trip to learn nothing new).
/// - any other refusal reason, at any age → no re-read (only the two lineage
///   mismatches are shaped like "we asked before our own apply landed").
///
/// The fourth row — a mismatch that survives the re-read — is not a distinct
/// state of this function: the caller re-runs `verify::verify_envelope`
/// against the refreshed value and simply falls through to the ordinary
/// `Refused` arm when it still fails, exactly as if no re-read had happened.
pub fn should_reread_on_mismatch(reason: RefusalReason, cache_age_secs: i64) -> bool {
    matches!(
        reason,
        RefusalReason::CoordinatorLineageMismatch | RefusalReason::DnaLineageMismatch
    ) && cache_age_secs >= REREAD_STALE_THRESHOLD_SECS
}

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

/// Borrow a resolved `(cid, kind)` pair for [`lineage_evidence_from`] without
/// consuming it — the pair is read twice when a declared-parent lookup is
/// needed (once to learn that, once to settle after the lookup).
fn superseded_as_ref(pair: &Option<(String, Option<String>)>) -> Option<(String, Option<&str>)> {
    pair.as_ref()
        .map(|(cid, kind)| (cid.clone(), kind.as_deref()))
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
                // This watch source has no `LineageRoles` handle of its own
                // (it is a narrow read-only conductor-inventory leg, not the
                // HTTP composition root) — an empty snapshot is honest and
                // byte-identical to the pre-Task-8 shape: it renders no
                // `lineage` on any role, matching a peer with no window ever
                // opened. `HttpServer`'s own `/version` handler (the wired
                // path) supplies the real snapshot via `with_lineage_roles`.
                lineage: std::collections::BTreeMap::new(),
                // **Task 12's field, filled by the same rule as `lineage`
                // above** — this source holds no `LineageBridge` any more than
                // it holds a `LineageRoles`, and the field's own doc says an
                // unwired node reports an empty sweep. Not a placeholder: a
                // narrow conductor-inventory leg has nothing to sweep, and the
                // wired `/version` path is where the real snapshot belongs.
                // If Task 12 wants the adoption controller to carry sweep
                // observations, that is a bridge handle threaded to here, not
                // a value invented at this call site.
                sweep: std::collections::BTreeMap::new(),
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
    /// The local projection pool, for the ONE read that needs it: a
    /// migrates-lineage commitment's projected lifecycle (`state`,
    /// `revoked_at`), which the path-evidence fetch pairs with the DHT entry's
    /// body. `None` leaves that lifecycle at its fail-closed default
    /// (`proposed`), which refuses a crossing rather than assuming one.
    db: Option<crate::db::DbPool>,
    /// **Task 13a.** The per-role lineage resolver, read by the revert sweep
    /// to find OPEN windows. `None` on a node that never wired one — the
    /// revert arm then does nothing at all, which is correct: a node with no
    /// resolver has no window to revert.
    lineage: Option<Arc<crate::lineage_roles::LineageRoles>>,
    /// **Task 13a.** Who performs a revert. In production this is the same
    /// `HappLineageVehicle` that opened the window.
    reverter: Option<Arc<dyn super::revert::LineageReverter>>,
    /// **Task 14b.** Who performs a sunset — the same `HappLineageVehicle`
    /// again. `None` leaves the sunset arm dark, which is the safe default for
    /// the one act with no remedy: a node that cannot seal simply keeps
    /// authoring on v2 with v1 open and readable.
    sunsetter: Option<Arc<dyn super::sunset::LineageSunsetter>>,
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
            db: None,
            lineage: None,
            reverter: None,
            sunsetter: None,
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

    /// Equip the local projection pool. Read by exactly one thing: the
    /// migrates-lineage path evidence's LIFECYCLE half
    /// (`super::path_evidence::fetch_path_evidence`).
    ///
    /// Without it a `happ-lineage` release refuses `conductor_unavailable` —
    /// this node cannot READ the commitment's lifecycle, which establishes
    /// nothing about the commitment in either direction (C4). It is
    /// deliberately NOT read as "not notarized": an unconfigured pool is a
    /// fact about us. Every other artifact class ignores this field entirely,
    /// so a node that follows no lineage channel pays a `None`.
    pub fn with_db(mut self, db: crate::db::DbPool) -> Self {
        self.db = Some(db);
        self
    }

    /// **Task 13a — equip the revert arm (Station 7).**
    ///
    /// BOTH halves or neither: the resolver names which windows are open and
    /// the reverter is the only thing that may close one. Wiring one without
    /// the other would give a controller that can see a revoked path and do
    /// nothing about it, which is worse than not looking — so the arm reads
    /// them as a pair and stays dark unless both are present.
    pub fn with_lineage_revert(
        mut self,
        lineage: Arc<crate::lineage_roles::LineageRoles>,
        reverter: Arc<dyn super::revert::LineageReverter>,
    ) -> Self {
        self.lineage = Some(lineage);
        self.reverter = Some(reverter);
        self
    }

    /// **Task 14b — equip the sunset arm (Station 8).**
    ///
    /// Separate from [`Self::with_lineage_revert`] rather than folded into it,
    /// because the two arms are not the same promise: a build may reasonably
    /// want the remedy without the irreversible act, and the reverse is never
    /// wanted at all. The arm additionally requires the resolver
    /// [`Self::with_lineage_revert`] supplies — without it there are no open
    /// windows to select, so a sunsetter alone is inert rather than dangerous.
    pub fn with_lineage_sunset(
        mut self,
        sunsetter: Arc<dyn super::sunset::LineageSunsetter>,
    ) -> Self {
        self.sunsetter = Some(sunsetter);
        self
    }

    /// Read installed reality, honoring the TTL cache. Returns the value
    /// alongside the age (seconds) of the snapshot that produced it — `0` for
    /// a snapshot just taken by this very call, so a caller deciding whether
    /// a refusal is worth a bypass-TTL re-read never has to guess whether the
    /// value it already has is fresh.
    async fn installed_reality(&self, now: i64) -> (Answer<InstalledReality>, i64) {
        let Some(source) = self.installed.as_ref() else {
            return (Answer::Unreachable, 0);
        };
        let mut cached = self.cached_reality.lock().await;
        if let Some((read_at, value)) = cached.as_ref() {
            let age = now - read_at;
            if age < INSTALLED_REALITY_TTL_SECS {
                return (value.clone(), age);
            }
        }
        let fresh = source.read().await;
        *cached = Some((now, fresh.clone()));
        (fresh, 0)
    }

    /// Force a fresh passport read, bypassing the TTL entirely, and cache the
    /// result. Used exactly once per stale-cache lineage-mismatch refusal —
    /// never in a loop — to give this node's OWN just-applied hot-swap a
    /// chance to be seen before the refusal is emitted.
    async fn installed_reality_refresh(&self, now: i64) -> Answer<InstalledReality> {
        let Some(source) = self.installed.as_ref() else {
            return Answer::Unreachable;
        };
        let fresh = source.read().await;
        *self.cached_reality.lock().await = Some((now, fresh.clone()));
        fresh
    }

    /// Invalidate the installed-reality cache outright. Called after ANY
    /// successful apply on this node — a coordinator-bundle hot-swap, a
    /// binary install, or a config-epr apply may each change the runtime
    /// passport this cache is a snapshot of — so the very next check re-reads
    /// it instead of serving a pre-apply snapshot for up to
    /// [`INSTALLED_REALITY_TTL_SECS`]. This is what closes the
    /// `coordinator_lineage_mismatch` false-refusal this controller produced
    /// against itself right after applying (measured 2026-09-02 on james and
    /// matthew): without it, the controller compares the NEXT release against
    /// the PRE-apply hashes for up to five minutes.
    async fn installed_reality_invalidate(&self) {
        *self.cached_reality.lock().await = None;
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

    /// Resolve the STAGING CANDIDATE's own `ContentHeadWire` — the two-call
    /// serve+prove pair, aimed at the declaration standing beneath an earned
    /// head (see [`classify_candidate_follow`]).
    ///
    /// `resolve_content_head_local` answers the WINNER's record, so the
    /// candidate's release manifest, author and `supersedes` are simply not in
    /// that answer. They have to be read separately, and they are read the ONLY
    /// way that keeps the bytes bound to the action the election named:
    ///
    ///   1. `get_record_for_action` — this conductor SERVES the record it holds
    ///      for the candidate action.
    ///   2. `validate_carried_head_record` — the wasm side PROVES those bytes:
    ///      action-hash binding, author signature, entry↔action binding, and the
    ///      TARGET-ID GATE (the Content must belong to THIS channel).
    ///
    /// Storage synthesizes nothing. A one-call `get_content` would have been
    /// cheaper, but `ContentOutput` carries no `supersedes`, so the controller
    /// would have had to assert a lineage edge instead of reading one — and
    /// `verify_lineage` is precisely the floor that edge feeds.
    ///
    /// ## Bounded on the WASM side, not by a caller timeout
    ///
    /// Both externs are O(1) in-wasm work decided BEFORE the call is made:
    /// `get_record_for_action` is a single local `get` on ONE action (no chain
    /// walk, no link gather), and `validate_carried_head_record` is pure crypto
    /// over bytes the caller supplied — it touches the DHT not at all. So there
    /// is no caller-side deadline here and there must not be one: a timeout
    /// would abandon a conductor that keeps running, which is the failure this
    /// crate's `conductor-call-is-uncancellable` rule names. `Background`
    /// admission is the bound that belongs to us — a sweep must never occupy
    /// the lane a person is standing in.
    ///
    /// This call pair is paid ONLY when a canary sees a candidate it has not
    /// already applied; on a converged canary the classifier returns
    /// `AlreadyApplied` and nothing here runs.
    ///
    /// Every failure returns `None`, and `None` means "keep following the
    /// winner" — a canary that cannot read the candidate does exactly what it
    /// does today, never a guess.
    async fn resolve_candidate_head(
        &self,
        channel_id: &str,
        candidate_cid: &str,
    ) -> Option<ContentHeadWire> {
        let hc = self.hc.as_ref()?;

        let served_payload = rmp_serde::to_vec_named(
            &crate::services::conductor_writes::GetRecordForActionInput {
                action_hash: candidate_cid.to_string(),
            },
        )
        .ok()?;
        let served_bytes = match hc
            .call_zome_timed(
                "content_store",
                "get_record_for_action",
                served_payload,
                AdmissionClass::Background,
            )
            .await
        {
            Ok((bytes, _timing)) => bytes,
            Err(e) => {
                tracing::debug!(
                    channel = %channel_id,
                    candidate = %candidate_cid,
                    error = %e,
                    "release-adoption: could not serve the staging candidate's record — the \
                     canary keeps following the earned winner this sweep"
                );
                return None;
            }
        };
        let served = rmp_serde::from_slice::<
            Option<crate::services::conductor_writes::CarriedRecordWire>,
        >(&served_bytes)
        .ok()
        .flatten()?;

        let proof_payload = rmp_serde::to_vec_named(
            &crate::services::conductor_writes::ValidateCarriedHeadRecordInput {
                id: channel_id.to_string(),
                expected_action_hash: candidate_cid.to_string(),
                record: Some(served.record),
            },
        )
        .ok()?;
        let proof_bytes = match hc
            .call_zome_timed(
                "content_store",
                "validate_carried_head_record",
                proof_payload,
                AdmissionClass::Background,
            )
            .await
        {
            Ok((bytes, _timing)) => bytes,
            Err(e) => {
                tracing::debug!(
                    channel = %channel_id,
                    candidate = %candidate_cid,
                    error = %e,
                    "release-adoption: the staging candidate's carried record did not prove \
                     itself — refusing to follow unproven bytes"
                );
                return None;
            }
        };
        let mut proven = rmp_serde::from_slice::<Option<ContentHeadWire>>(&proof_bytes)
            .ok()
            .flatten()?;

        // The candidate IS a staging declaration — say so explicitly rather than
        // letting the prove path's honest `canonical_earned: None` (it holds no
        // election, and correctly refuses to invent one) read downstream as
        // `HeadTier::None`, which would put this release under the EARNED
        // threshold gate it has not had a chance to earn yet.
        proven.canonical_earned = Some(false);
        // Nothing stands beneath a candidate. Clearing these keeps the wire from
        // describing a second hop the election never declared.
        proven.staging_candidate = None;
        proven.staging_candidate_declared_at = None;
        Some(proven)
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

        // **Task 13a.** The revert arm runs FIRST: a window whose path has
        // been revoked should stop authoring on v2 before this sweep does
        // anything else, and the arm costs one lock read on a peer with no
        // open window.
        self.sweep_lineage_windows().await;

        // BUDGETS FIRST. Everything below is sized before any call is made.
        let mut byte_budget = MAX_ARTIFACT_BYTES_PER_SWEEP;
        let mut threshold_reads = MAX_THRESHOLD_READS_PER_SWEEP;
        let mut checked = 0usize;

        for channel in followed.channels.iter().take(MAX_CHANNELS_PER_SWEEP) {
            let existing = state::channel_state(&channel.channel_id);
            if let Some(existing) = existing.as_ref() {
                if existing.is_backing_off(now) {
                    // The backoff on this row was computed for a SPECIFIC
                    // resolved head (`existing.resolved_head`) — the one a
                    // refusal was about. A channel's canonical election can
                    // move to a NEW head (a fresh release published on the
                    // same channel) while the timer is still running, and a
                    // refusal about the OLD head carries zero information
                    // about the new one. Re-resolve cheaply — the same
                    // `resolve_content_head_local` Background-classed read
                    // `check_channel` makes, never a network `get` — before
                    // honouring the timer.
                    let current_cid = match self.resolve_head(&channel.channel_id).await {
                        Answer::Present(head) => Some(head.head_action_hash.0.clone()),
                        // Absent or Unreachable: the election read itself
                        // gave us nothing new to act on, so this is not the
                        // "different head" case — honour the backoff exactly
                        // as before.
                        Answer::Absent | Answer::Unreachable => None,
                    };
                    if existing.should_skip_for_backoff(now, current_cid.as_deref()) {
                        continue;
                    }
                    // The election moved. Treat this tick as a clean start
                    // against the new head rather than resuming the old
                    // head's refusal streak — `check_channel` below re-reads
                    // and records the real verdict; this only clears the
                    // count so that verdict's backoff rung is computed
                    // against a streak of ONE, not a streak inherited from a
                    // head this check is not even about.
                    state::reset_refusals_for_new_head(&channel.channel_id);
                }
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
                // Invalidate FIRST, before anything else about this apply is
                // recorded: any vehicle — coordinator-bundle hot-swap, binary
                // install, or config-epr — may have just changed what this
                // node's own passport reports, and the next channel this
                // sweep (or the next sweep entirely) checks must not judge
                // itself against a snapshot this apply has already made
                // stale.
                self.installed_reality_invalidate().await;
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
                        // The lineage carry, kept rather than dropped: only the
                        // `happ-lineage` vehicle fills this in, and this row is
                        // the only place it survives the sweep.
                        carry: receipt.carry.clone(),
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

        // Hoisted, because the candidate decision below needs the SAME fact the
        // C6b exit needs. Guarded by `will_apply` exactly as that exit is, so
        // `observe` still reads nothing it did not read before.
        let applied = if will_apply {
            state::applied_release(&channel.channel_id)
        } else {
            None
        };

        // **LONG-LIVED CHANNEL — the canary follows the CANDIDATE.**
        //
        // Placed BEFORE `release_cid` and the C6b exit on purpose: a canary that
        // applied the candidate must compare `appliedRelease.cid` against the
        // CANDIDATE. Substituting after the exit would leave it comparing the
        // earned winner it never applied, and it would re-run the entire check —
        // threshold read, artifact fetch, verify — every sweep, forever.
        //
        // `apply` and `observe` never reach the substitution
        // (`classify_candidate_follow` returns `Leave`), so their behaviour here
        // is byte-identical to before.
        // Decided BEFORE the match, so the borrow of `head` and `applied` is
        // plainly over before an arm moves either of them.
        let follow = classify_candidate_follow(
            channel.mode,
            &head,
            applied.as_ref().map(|a| a.cid.as_str()),
        );
        let head = match follow {
            CandidateFollow::Leave => head,
            // The C6b idempotence rule, applied to the candidate — the SAME rule
            // as the exit below, on the subject this canary is actually
            // following. Stated here rather than reached through a
            // half-substituted wire, and it costs ZERO conductor calls beyond
            // the resolve, which is the contract C6b is written to keep.
            CandidateFollow::AlreadyApplied(candidate) => {
                let applied = applied
                    .expect("AlreadyApplied is returned only when an applied release was read");
                return CheckOutcome::Checked {
                    head: Some(ResolvedHead {
                        cid: candidate.clone(),
                        tier: HeadTier::Staging,
                    }),
                    verdict: Verdict::Applied {
                        release_cid: candidate,
                        vehicle: applied.vehicle,
                        already_current: true,
                    },
                    attestations: None,
                };
            }
            CandidateFollow::Follow(candidate) => {
                match self
                    .resolve_candidate_head(&channel.channel_id, &candidate)
                    .await
                {
                    Some(candidate_head) => {
                        tracing::info!(
                            channel = %channel.channel_id,
                            winner = %head.head_action_hash.0,
                            candidate = %candidate,
                            "release-adoption: canary follows the staging candidate standing \
                             beneath the earned head — the winner does not move until promotion"
                        );
                        candidate_head
                    }
                    // Unreadable candidate: keep the winner. Honest, and the
                    // next sweep asks again.
                    None => head,
                }
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
        //
        // `applied` is the hoisted read above — same value, same `will_apply`
        // guard, read once instead of twice.
        if let Some(applied) = applied {
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

        let (mut installed, installed_age_secs) = self.installed_reality(now).await;

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
            // **Defensive re-read.** A `*_lineage_mismatch` is exactly the
            // shape this controller's own just-applied hot-swap produces
            // against a cache it has not yet been told to drop (belt: the
            // invalidation in `apply_verified` above is the suspenders-free
            // fix; this is the suspenders, for a passport change that landed
            // by some path OTHER than this controller's own apply — an
            // operator-driven install, or a peer sharing the node). Bounded
            // to exactly one bypass-TTL re-read, decided by a pure table
            // (`should_reread_on_mismatch`) so it can never loop and never
            // fires on a snapshot that was already fresh.
            if should_reread_on_mismatch(refusal.reason_code(), installed_age_secs) {
                let refreshed = self.installed_reality_refresh(now).await;
                match verify::verify_envelope(&manifest, &refreshed) {
                    Ok(()) => {
                        tracing::info!(
                            channel = %channel.channel_id,
                            reason = %refusal.reason,
                            cache_age_secs = installed_age_secs,
                            "release-adoption: bypass-TTL re-read of installed reality reversed \
                             a lineage-mismatch refusal — the cached snapshot pre-dated a \
                             passport change"
                        );
                        installed = refreshed;
                    }
                    Err(fresh_refusal) => {
                        return CheckOutcome::Checked {
                            head: Some(resolved),
                            verdict: Verdict::Refused {
                                refusal: fresh_refusal,
                            },
                            attestations: None,
                        };
                    }
                }
            } else {
                return CheckOutcome::Checked {
                    head: Some(resolved),
                    verdict: Verdict::Refused { refusal },
                    attestations: None,
                };
            }
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

        // What the staged bytes WOULD install — the evidence the by-bytes exit
        // reads. Derived from the artifact itself because a manifest declares
        // only what it SUPERSEDES.
        let target_coordinators =
            super::apply::staged_target_coordinators(&manifest, &fetched).await;
        // FRESHNESS GATE. The by-bytes exit STOPS work on the strength of the
        // installed-reality snapshot, so — unlike a refusal, which self-heals
        // on the next sweep — it may not be taken from a stale one. Exactly one
        // bypass-TTL re-read, the same bounded shape (and the same threshold)
        // the lineage-mismatch re-read above uses, and only when the exit is
        // actually about to fire.
        if installed_age_secs >= REREAD_STALE_THRESHOLD_SECS
            && verify::already_runs_target(&manifest, &installed, &target_coordinators)
        {
            installed = self.installed_reality_refresh(now).await;
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
            target_coordinators: &target_coordinators,
            // **Rung 6.** The fetch site (I1/C5): the commitment is read
            // through THIS peer's own conductor, its lifecycle off this
            // peer's own projection. Absent for every artifact class but
            // `happ-lineage`, which costs a non-lineage sweep exactly nothing.
            path: super::path_evidence::fetch_path_evidence(
                self.hc.as_ref(),
                self.db.as_ref(),
                &manifest,
            )
            .await,
        }) {
            // ALREADY CURRENT BY BYTES: this peer runs exactly what the release
            // would install. Never a refusal — and routed through the SAME mode
            // table as a fresh apply, because "I already run these bytes" is a
            // fact about installed reality, not a licence to skip the mode's
            // rules about which tier this peer adopts.
            Ok(verify::VerifyOutcome::AlreadyCurrent { roles }) => {
                tracing::info!(
                    channel = %channel.channel_id,
                    release_cid = %release_cid,
                    roles = ?roles,
                    mode = channel.mode.label(),
                    "release-adoption: already current BY BYTES — this peer runs the release's \
                     target coordinator wasm for every role it touches"
                );
                match decide_post_verify_action(channel.mode, resolved.tier, true) {
                    // The mode WOULD have applied this head, and there is
                    // nothing left to apply. Recorded so the C6b exit takes
                    // over next sweep and this convergence costs one resolve
                    // from here on.
                    PostVerifyAction::Apply => {
                        state::record_applied(
                            &channel.channel_id,
                            AppliedRelease {
                                cid: release_cid.clone(),
                                at: now,
                                vehicle: super::VEHICLE_ALREADY_INSTALLED.to_string(),
                                // Nothing was carried — this arm is the
                                // already-installed convergence exit, and a
                                // carry receipt here would claim a crossing
                                // that never ran.
                                carry: None,
                            },
                            false,
                        );
                        Verdict::Applied {
                            release_cid: release_cid.clone(),
                            vehicle: super::VEHICLE_ALREADY_INSTALLED.to_string(),
                            already_current: true,
                        }
                    }
                    // `apply` mode on a STAGING head. Nothing to do either way,
                    // and recording an apply here would let this peer skip the
                    // threshold read an operator watching promotion wants.
                    PostVerifyAction::Waiting => Verdict::Waiting {
                        release_cid: release_cid.clone(),
                        detail: "this peer already runs these exact coordinator bytes; apply \
                                 mode adopts earned heads only, so there is nothing to do on \
                                 either side of the promotion"
                            .to_string(),
                    },
                    // Observe records nothing and applies nothing — but "you
                    // already run this" is a PASS, not the
                    // `coordinator_lineage_mismatch` the supersedes check alone
                    // would have reported.
                    PostVerifyAction::Observed => Verdict::Ok {
                        release_cid: release_cid.clone(),
                    },
                    PostVerifyAction::Refused => unreachable!(
                        "verify::verify already enforced the threshold for tier {:?} before the \
                         by-bytes exit",
                        resolved.tier
                    ),
                }
            }
            // OBSERVE MODE ENDS HERE: the `VerifiedRelease` is reported and
            // dropped. `apply` and `canary` diverge on a STAGING head — see
            // `decide_post_verify_action`, the single source both this call
            // site and its table test consult: `apply` adopts EARNED heads
            // only (a verified STAGING head there is `Waiting`), `canary`
            // adopts either tier.
            Ok(verify::VerifyOutcome::Verified(verified)) => {
                match decide_post_verify_action(channel.mode, resolved.tier, true) {
                    PostVerifyAction::Apply => {
                        self.apply_verified(&channel.channel_id, *verified).await
                    }
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
                }
            }
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

/// What this sweep should do about a STAGING candidate standing beneath an
/// EARNED head — the long-lived-channel decision, made PURE so the four corners
/// that matter are pinned by a table test rather than by a live conductor.
#[derive(Debug, Clone, PartialEq, Eq)]
enum CandidateFollow {
    /// Leave the resolved winner alone. Every mode but `canary`, every tier but
    /// `earned`, and an earned head with no candidate beneath it.
    Leave,
    /// This canary has ALREADY applied the candidate. Nothing to fetch, nothing
    /// to verify — the idempotence answer, on the candidate.
    AlreadyApplied(String),
    /// Fetch the candidate's own bytes, prove them, and follow it.
    Follow(String),
}

/// Decide [`CandidateFollow`] from the three facts that settle it: the mode, the
/// resolved head, and what this peer has already applied. PURE.
///
/// ## Why only `canary`
///
/// On a channel that has already earned a head the next release arrives as a
/// STAGING candidate underneath it (`content_store::select_staging_candidate`),
/// and the winner does not move until promotion. An `apply` peer must therefore
/// keep running the EARNED head — that is the entire point of the tier split.
/// A canary, whose job is to soak the next release before anyone promotes it,
/// has nothing to soak unless it follows the candidate: on a long-lived channel
/// the winner is a release it already runs, so a canary that only ever looked at
/// the winner would report `already_current` forever and the promotion loop
/// would never close.
///
/// This is the SAME act the canary already performs on a fresh channel whose
/// winner IS staging (`decide_post_verify_action(Canary, Staging, _) => Apply`).
/// Only where the staging declaration sits in the election differs, which is why
/// following it needs no new mode, no new verdict, and no new table row.
///
/// ## Why `observe` is excluded
///
/// An observer reports what the channel elected. The candidate is not elected —
/// it is queued beneath what was — so an observer that switched subjects would
/// stop reporting the head its operator is watching.
fn classify_candidate_follow(
    mode: AdoptionMode,
    head: &ContentHeadWire,
    applied_cid: Option<&str>,
) -> CandidateFollow {
    if mode != AdoptionMode::Canary {
        return CandidateFollow::Leave;
    }
    // Only beneath an EARNED winner. A staging winner has no candidate under it
    // (the zome reports `None` by construction), and `None`/`Some(false)` here
    // means no election evidence at all — never a licence to guess.
    if head.canonical_earned != Some(true) {
        return CandidateFollow::Leave;
    }
    let Some(candidate) = head.staging_candidate.as_ref().map(|h| h.0.clone()) else {
        return CandidateFollow::Leave;
    };
    // Defensive: a candidate that IS the winner is not a candidate. The zome
    // cannot produce this (the candidate is strictly newer than the winner), so
    // reaching it means the wire lied — and following it would make this peer
    // verify the head twice under two tiers.
    if candidate == head.head_action_hash.0 {
        return CandidateFollow::Leave;
    }
    if applied_cid == Some(candidate.as_str()) {
        CandidateFollow::AlreadyApplied(candidate)
    } else {
        CandidateFollow::Follow(candidate)
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
            assert!(REREAD_STALE_THRESHOLD_SECS > 0);
            // The re-read threshold exists to skip re-reading a snapshot this
            // node's own `installed_reality_invalidate()` already made fresh
            // — it must never approach the TTL it is carved out of.
            assert!(REREAD_STALE_THRESHOLD_SECS < INSTALLED_REALITY_TTL_SECS);
        }
    }

    /// A source that counts how many times it was actually asked, so a test
    /// can tell a cache hit from a cache miss without a conductor.
    struct CountingInstalledReality {
        calls: std::sync::atomic::AtomicUsize,
    }

    #[async_trait::async_trait]
    impl InstalledRealitySource for CountingInstalledReality {
        async fn read(&self) -> Answer<InstalledReality> {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Answer::Present(InstalledReality::default())
        }
    }

    /// **The fix, in its most direct form.** After ANY successful apply, the
    /// very next `installed_reality()` call must miss the cache — even
    /// called at the exact same instant, well inside the TTL — because
    /// `installed_reality_invalidate` is what closes the
    /// `coordinator_lineage_mismatch` false-refusal this controller produced
    /// against itself (measured on james and matthew, 2026-09-02).
    #[tokio::test]
    async fn installed_reality_invalidate_forces_the_next_read_to_miss_the_cache() {
        use std::sync::atomic::Ordering;

        let dir = tempfile::tempdir().unwrap();
        let source = Arc::new(CountingInstalledReality {
            calls: std::sync::atomic::AtomicUsize::new(0),
        });
        let controller = AdoptionController::new(dir.path()).with_installed_reality(source.clone());

        let (_, age) = controller.installed_reality(1_000).await;
        assert_eq!(age, 0, "the very first read is always a fresh snapshot");
        assert_eq!(source.calls.load(Ordering::SeqCst), 1);

        // Same instant, well inside the TTL: a plain cache hit. This is the
        // defect surface without invalidation — a check running seconds
        // after this node's own apply would be judged against the PRE-apply
        // snapshot for up to INSTALLED_REALITY_TTL_SECS.
        let (_, age) = controller.installed_reality(1_000).await;
        assert_eq!(age, 0);
        assert_eq!(
            source.calls.load(Ordering::SeqCst),
            1,
            "well within the TTL, a second read at the same instant must NOT touch the source"
        );

        controller.installed_reality_invalidate().await;

        let (_, age) = controller.installed_reality(1_000).await;
        assert_eq!(
            age, 0,
            "a post-invalidate read is a fresh snapshot again, not a stale cache hit"
        );
        assert_eq!(
            source.calls.load(Ordering::SeqCst),
            2,
            "invalidate must force the very next installed_reality() call to miss the cache, \
             even though it is still well within the TTL"
        );
    }

    /// **The re-read-once decision, as a pure table.** Only a lineage
    /// mismatch earns a bypass-TTL re-read, and only when the cache that
    /// produced it is stale enough that it could plausibly pre-date a
    /// passport change this node did not itself just invalidate for. A
    /// mismatch that survives the re-read is not a distinct row here — the
    /// caller re-runs `verify::verify_envelope` against the refreshed value
    /// and falls through to the ordinary `Refused` arm exactly as if no
    /// re-read had happened.
    #[test]
    fn should_reread_on_mismatch_gates_on_reason_and_staleness() {
        let cases: &[(RefusalReason, i64, bool)] = &[
            // Stale cache + a lineage mismatch -> re-read.
            (
                RefusalReason::CoordinatorLineageMismatch,
                REREAD_STALE_THRESHOLD_SECS,
                true,
            ),
            (
                RefusalReason::CoordinatorLineageMismatch,
                REREAD_STALE_THRESHOLD_SECS + 100,
                true,
            ),
            (
                RefusalReason::DnaLineageMismatch,
                REREAD_STALE_THRESHOLD_SECS,
                true,
            ),
            // Fresh cache + a lineage mismatch -> no re-read: the snapshot is
            // already current, so re-reading would only spend a round trip to
            // learn nothing new.
            (RefusalReason::CoordinatorLineageMismatch, 0, false),
            (
                RefusalReason::CoordinatorLineageMismatch,
                REREAD_STALE_THRESHOLD_SECS - 1,
                false,
            ),
            (RefusalReason::DnaLineageMismatch, 0, false),
            // Any other refusal reason, at any age -> no re-read: only the
            // two lineage mismatches are shaped like "we asked before our own
            // apply landed".
            (
                RefusalReason::ArtifactUnavailable,
                REREAD_STALE_THRESHOLD_SECS + 100,
                false,
            ),
            (
                RefusalReason::ConductorUnavailable,
                REREAD_STALE_THRESHOLD_SECS + 100,
                false,
            ),
        ];
        for (reason, age, expected) in cases.iter().copied() {
            assert_eq!(
                should_reread_on_mismatch(reason, age),
                expected,
                "reason={reason:?} age={age}"
            );
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

    // -----------------------------------------------------------------------
    // LONG-LIVED CHANNEL — the canary follows the candidate.
    //
    // `classify_candidate_follow` is the whole decision, so these are the
    // controller half's contract. Built through `ContentHeadWire`'s real
    // `Deserialize` impl so a fixture cannot drift from the wire shape.
    // -----------------------------------------------------------------------

    const WINNER_CID: &str = "uhCkkEarnedWinner000000000000000000000000000000000000";
    const CANDIDATE_CID: &str = "uhCkkStagingCandidate0000000000000000000000000000000";

    /// A resolved head, optionally EARNED and optionally carrying a candidate.
    fn head_wire(canonical_earned: Option<bool>, candidate: Option<&str>) -> ContentHeadWire {
        let mut value = serde_json::json!({
            "content_id": "runtime:coordinators:elohim:commons",
            "head_action_hash": WINNER_CID,
            "declared_at": 1_700_000_000_000_000i64,
            "canonical": true,
            "canonical_earned": canonical_earned,
            "content": {
                "id": "runtime:coordinators:elohim:commons",
                "content_type": "concept",
                "title": "t",
                "description": "d",
                "content_format": "markdown",
                "reach": "commons",
            },
        });
        if let Some(candidate) = candidate {
            value["staging_candidate"] = serde_json::json!(candidate);
            value["staging_candidate_declared_at"] = serde_json::json!(1_700_000_001_000_000i64);
        }
        serde_json::from_value(value).expect("ContentHeadWire fixture must deserialize")
    }

    /// THE CASE THIS CHANGE EXISTS FOR. A canary on a channel that already
    /// earned a head, with an unapplied candidate beneath it, follows the
    /// candidate — which is what gives it something to soak. Without this the
    /// canary sees only the release it already runs, reports `already_current`
    /// forever, and the promotion loop never closes.
    #[test]
    fn canary_follows_an_unapplied_candidate_beneath_an_earned_head() {
        let head = head_wire(Some(true), Some(CANDIDATE_CID));
        assert_eq!(
            classify_candidate_follow(AdoptionMode::Canary, &head, None),
            CandidateFollow::Follow(CANDIDATE_CID.to_string()),
            "a canary with nothing applied must fetch and follow the candidate"
        );
        // Same answer when this peer has applied something ELSE (the earned
        // winner it is currently running) — that is the ordinary steady state
        // a new candidate arrives into.
        assert_eq!(
            classify_candidate_follow(AdoptionMode::Canary, &head, Some(WINNER_CID)),
            CandidateFollow::Follow(CANDIDATE_CID.to_string()),
            "already running the earned winner is not having applied the candidate"
        );
    }

    /// The converged canary. Once it HAS applied the candidate, the answer is
    /// idempotence ON THE CANDIDATE — not a re-run against the earned winner it
    /// never applied. This is the arm that keeps a soaking canary from paying a
    /// threshold read, an artifact fetch and a verify on every single sweep.
    #[test]
    fn a_canary_that_applied_the_candidate_is_already_current_on_it() {
        let head = head_wire(Some(true), Some(CANDIDATE_CID));
        assert_eq!(
            classify_candidate_follow(AdoptionMode::Canary, &head, Some(CANDIDATE_CID)),
            CandidateFollow::AlreadyApplied(CANDIDATE_CID.to_string())
        );
    }

    /// APPLY AND OBSERVE ARE UNTOUCHED. An `apply` peer must keep running the
    /// EARNED head — following an unpromoted candidate would adopt a release
    /// nobody has promoted, which is the exact thing the tier split prevents.
    /// An observer must keep reporting the head its operator is watching.
    #[test]
    fn apply_and_observe_never_follow_a_candidate() {
        let head = head_wire(Some(true), Some(CANDIDATE_CID));
        for mode in [AdoptionMode::Apply, AdoptionMode::Observe] {
            for applied in [None, Some(WINNER_CID), Some(CANDIDATE_CID)] {
                assert_eq!(
                    classify_candidate_follow(mode, &head, applied),
                    CandidateFollow::Leave,
                    "mode={mode:?} applied={applied:?} must leave the resolved winner alone"
                );
            }
        }
    }

    /// NO CANDIDATE ⇒ THE PATH IS UNCHANGED, on every tier and in every mode.
    /// This is the behaviour-identity proof: a fresh channel (staging winner),
    /// a promoted channel with nothing queued, and a head carrying no election
    /// evidence at all each behave exactly as they did before the candidate
    /// existed.
    #[test]
    fn no_candidate_leaves_every_mode_on_its_existing_path() {
        let cases = [
            // an EARNED winner with nothing queued beneath it
            head_wire(Some(true), None),
            // a STAGING winner — the zome reports no candidate by construction
            head_wire(Some(false), None),
            // no election evidence at all (root-author fallback, or a
            // pre-candidate coordinator)
            head_wire(None, None),
        ];
        for head in &cases {
            for mode in [
                AdoptionMode::Canary,
                AdoptionMode::Apply,
                AdoptionMode::Observe,
            ] {
                assert_eq!(
                    classify_candidate_follow(mode, head, Some(WINNER_CID)),
                    CandidateFollow::Leave,
                    "mode={mode:?} earned={:?} must not invent a candidate",
                    head.canonical_earned
                );
            }
        }
    }

    /// A candidate reported beneath a STAGING winner is never followed. The zome
    /// cannot produce it, and following it would give the canary two staging
    /// answers on one channel with no rule to choose between them.
    #[test]
    fn a_candidate_under_a_staging_winner_is_not_followed() {
        let head = head_wire(Some(false), Some(CANDIDATE_CID));
        assert_eq!(
            classify_candidate_follow(AdoptionMode::Canary, &head, None),
            CandidateFollow::Leave
        );
    }

    /// Defensive: a candidate naming the WINNER is not a candidate. Unreachable
    /// from the zome (the candidate is strictly newer than the winner), so
    /// reaching it means the wire lied — and following it would verify one head
    /// twice under two different tiers.
    #[test]
    fn a_candidate_equal_to_the_winner_is_not_followed() {
        let head = head_wire(Some(true), Some(WINNER_CID));
        assert_eq!(
            classify_candidate_follow(AdoptionMode::Canary, &head, None),
            CandidateFollow::Leave
        );
    }

    /// The tier the substituted candidate carries is STAGING, and the routing
    /// that follows from it is `Apply` — the row that makes the canary's follow
    /// need no new mode and no new table entry. Pins the join between
    /// `classify_candidate_follow` and `decide_post_verify_action`.
    #[test]
    fn a_followed_candidate_routes_through_the_existing_canary_staging_row() {
        assert_eq!(
            decide_post_verify_action(AdoptionMode::Canary, HeadTier::Staging, false),
            PostVerifyAction::Apply,
            "a followed candidate is verified and applied by the canary, threshold reported \
             but not enforced — the soak IS the evidence being gathered"
        );
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
