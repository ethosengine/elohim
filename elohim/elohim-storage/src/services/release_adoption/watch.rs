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
//! answer. That is why the resolve is a `resolve_content_head` call rather than
//! a read of the local projection: the projection is a record of what we were
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

use super::state::{
    self, AdoptionMode, FollowedChannel, FollowedChannels, HeadTier, ResolvedHead, Verdict,
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

/// The observe-mode adoption controller.
///
/// Holds only what a sweep needs. There is no `Option<Box<dyn ApplyVehicle>>`
/// field: a controller that could hold a vehicle is one edit away from calling
/// it, and this rung's whole safety claim is that no such call site exists.
pub struct AdoptionController {
    hc: Option<Arc<HcClient>>,
    artifacts: Option<Arc<dyn ArtifactSource>>,
    installed: Option<Arc<dyn InstalledRealitySource>>,
    staging_root: PathBuf,
    cached_reality: tokio::sync::Mutex<Option<(i64, Answer<InstalledReality>)>>,
}

impl AdoptionController {
    pub fn new(staging_root: impl Into<PathBuf>) -> Self {
        Self {
            hc: None,
            artifacts: None,
            installed: None,
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

    /// Resolve one channel's canonical head through THIS node's conductor.
    ///
    /// Deliberately a `Background`-classed call: a controller sweep must never
    /// occupy the admission lane a person is standing in.
    /// [`crate::services::conductor_writes::call_resolve_content_head`] is the
    /// owner of this wire shape and its `ContentHeadWire` decode mirror is
    /// reused verbatim here; only the admission class differs. (Residual: that
    /// module wants a `_classed` variant the way its declare path already has
    /// one — it belongs to another lane, so this reuses the type rather than
    /// editing the file.)
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
                "resolve_content_head",
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
            match outcome {
                CheckOutcome::Skipped => {}
                CheckOutcome::Checked { head, verdict } => {
                    record_decision(verdict_arm(&verdict), verdict.reason_label());
                    state::record_check(&channel.channel_id, now, head, verdict);
                    checked += 1;
                }
            }
        }
        state::record_sweep(now);
        checked
    }

    async fn check_channel(
        &self,
        channel: &FollowedChannel,
        now: i64,
        byte_budget: &mut u64,
        threshold_reads: &mut usize,
    ) -> CheckOutcome {
        // Exhaustive on purpose, and NOT a wildcard: only `observe` exists, so
        // this compiles to nothing today — and the day T4 adds a mode this
        // match stops compiling. "Only observe is legal" must be true at the
        // point of ACTION, not only at the point of configuration, and a
        // forced edit here is the cheapest way to guarantee that.
        let mode_refusal: Option<AdoptionRefusal> = match channel.mode {
            AdoptionMode::Observe => None,
        };
        if let Some(refusal) = mode_refusal {
            return CheckOutcome::Checked {
                head: None,
                verdict: Verdict::Refused { refusal },
            };
        }

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
                };
            }
        };

        let release_cid = head.head_action_hash.0.clone();
        let resolved = ResolvedHead {
            cid: release_cid.clone(),
            tier: HeadTier::from_canonical_earned(head.canonical_earned),
        };

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
                }
            }
            Err(refusal) => {
                return CheckOutcome::Checked {
                    head: Some(resolved),
                    verdict: Verdict::Refused { refusal },
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
        let lineage = Answer::Present(LineageEvidence {
            supersedes: head.supersedes.as_ref().map(|h| h.0.clone()),
        });

        // Refuse on the cheap arms BEFORE spending a threshold read or a byte
        // of staging: an envelope that cannot match will not match after we pay
        // for evidence.
        if let Err(refusal) = verify::verify_envelope(&manifest, &installed) {
            return CheckOutcome::Checked {
                head: Some(resolved),
                verdict: Verdict::Refused { refusal },
            };
        }
        if let Err(refusal) = verify::verify_lineage(&manifest, &lineage) {
            return CheckOutcome::Checked {
                head: Some(resolved),
                verdict: Verdict::Refused { refusal },
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
        }) {
            // OBSERVE MODE ENDS HERE. A `VerifiedRelease` is minted, reported,
            // and dropped. There is no vehicle to hand it to, by construction.
            Ok(verified) => Verdict::Ok {
                release_cid: verified.release_cid,
            },
            Err(refusal) => Verdict::Refused { refusal },
        };

        CheckOutcome::Checked {
            head: Some(resolved),
            verdict,
        }
    }
}

enum CheckOutcome {
    /// Not looked at this sweep (backoff or budget). State untouched, so
    /// `lastCheckedAt` keeps telling the truth.
    Skipped,
    Checked {
        head: Option<ResolvedHead>,
        verdict: Verdict,
    },
}

fn verdict_arm(verdict: &Verdict) -> DecisionArm {
    match verdict {
        Verdict::Idle { .. } | Verdict::Ok { .. } => DecisionArm::Watch,
        Verdict::Refused { refusal } => refusal.reason_code().arm(),
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

/// Spawn the observe-mode sweep loop.
///
/// Returns `false` (and spawns nothing) when no channel is followed — a peer
/// that follows nothing pays nothing, exactly as the runtime-config watcher
/// does when no path is configured.
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
            "release-adoption: controller IDLE — no followed channels configured"
        );
        state::reconcile_followed(&followed);
        return false;
    }
    tracing::info!(
        channels = followed.channels.len(),
        refused = followed.refused.len(),
        sweep_secs = SWEEP_INTERVAL_SECS,
        "release-adoption: controller ACTIVE in OBSERVE mode — it will report verdicts and \
         apply nothing (no apply vehicle is compiled into this build)"
    );
    state::mark_controller_running(true);

    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(SWEEP_INTERVAL_SECS));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            // Re-read the config every tick: rung 4's whole point is that a
            // follow/unfollow lands on a RUNNING node.
            let followed = followed_from_runtime_config();
            let checked = controller.sweep_once(&followed).await;
            tracing::debug!(
                channels = followed.channels.len(),
                checked,
                "release-adoption: sweep complete"
            );
        }
    });
    true
}

/// Pre-touch every `(arm, reason)` series a real branch of this module can
/// reach, so a zero reads as a MEASURED zero rather than as an absent series.
///
/// The `apply` arm is deliberately NOT pre-touched: this build compiles no
/// vehicle, so a zero there would claim the apply arm ran and did nothing.
pub fn pretouch_metrics() {
    for arm in [DecisionArm::Watch, DecisionArm::Fetch, DecisionArm::Verify] {
        crate::metrics::RELEASE_ADOPTION_DECISIONS
            .with_label_values(&[arm.label(), super::REASON_OK])
            .inc_by(0);
    }
    crate::metrics::RELEASE_ADOPTION_DECISIONS
        .with_label_values(&[DecisionArm::Watch.label(), super::REASON_IDLE])
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
    }
}
