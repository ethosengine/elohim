//! Adoption state — **Ephemeral (C)**, and deliberately so.
//!
//! Spec §5 classifies `AdoptionState` as Category C: node-local, in-memory,
//! reconstructable from a single sweep, surfaced on an admin route, **never
//! notarized and never gossiped as authority**. Nothing in this file is a
//! source of truth about a channel — it is a record of what THIS peer's
//! controller last saw and decided. Fleet visibility over these states stays
//! observational (`version-matrix --observed`), exactly as the runtime
//! passport's is.
//!
//! Two things live here that a naive controller would scatter:
//!
//! 1. **The followed-channel set** — parsed from the rung-4 runtime-config
//!    surface, so a peer can start or stop following a channel on a running
//!    node in seconds. `observe` and `apply` are the legal modes (T4 landed
//!    the second); `observe` is what a bare channel id means, so applying is
//!    always something an operator asked for by name. An illegal mode is
//!    recorded as a typed refusal rather than dropped — a channel that
//!    silently vanishes from the report because its mode was misspelled is the
//!    failure this module refuses to ship, and a misspelling that silently
//!    became `apply` would be the far worse half of the same bug.
//!
//! 2. **The backoff ladder** — finite by construction (C6a). A refusal that a
//!    later sweep could plausibly cure backs off along
//!    [`BACKOFF_LADDER_SECS`] and then holds at its last rung; a terminal
//!    refusal (the DNA line, a broken additive floor) goes straight to the
//!    ceiling, because only a NEW release can change it and re-asking the
//!    conductor about a decided question is pure cost.

use std::collections::BTreeMap;
use std::sync::{LazyLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};

use super::{AdoptionRefusal, RefusalReason};
use crate::services::release_attestation::QualifyingEvidence;

/// Runtime-config key naming the channels this peer follows.
///
/// Value is a comma- (or whitespace-) separated list of `channelId` or
/// `channelId=mode` entries, e.g.
///
/// ```text
/// ELOHIM_RELEASE_CHANNELS = "runtime:coordinators:elohim:canary-a=observe, runtime:config:elohim:commons"
/// ```
///
/// The key is the ENV VAR NAME, per the runtime-config module's one-vocabulary
/// rule: an operator who knows the flag knows the file key.
pub const RELEASE_CHANNELS_KEY: &str = "ELOHIM_RELEASE_CHANNELS";

/// Finite backoff ladder, in seconds. A channel that keeps refusing climbs it
/// and then **holds at the last rung** — it never grows without bound and never
/// stops asking entirely, which is the shape C6a asks for: bounded work per
/// sweep, and a liveness floor so a cured substrate is noticed.
///
/// bounded-work: `watch::MAX_CHANNELS_PER_SWEEP` head resolves per tick (with
/// `watch::MAX_THRESHOLD_READS_PER_SWEEP` attestation reads and
/// `watch::MAX_ARTIFACT_BYTES_PER_SWEEP` bytes staged inside it). Those are the
/// per-SWEEP budget — the ceiling on what one tick may cost. This ladder is the
/// per-CHANNEL half of the same discipline: it decides how much of that budget
/// a channel is allowed to keep asking for. Without it a single permanently
/// refusing channel would consume its slice of `MAX_CHANNELS_PER_SWEEP` every
/// 60 s forever, which is bounded per tick and unbounded in total; with it, a
/// terminal refusal costs one resolve per `TERMINAL_BACKOFF_SECS`. The two must
/// be read together — neither alone bounds the work.
pub const BACKOFF_LADDER_SECS: [u64; 5] = [0, 30, 120, 600, 1800];

/// Where a terminal refusal parks immediately: the ladder's ceiling. Only a new
/// release changes a terminal verdict, and the channel head is what tells us
/// one arrived — so we keep checking at the slowest rung, never faster.
pub const TERMINAL_BACKOFF_SECS: u64 = BACKOFF_LADDER_SECS[BACKOFF_LADDER_SECS.len() - 1];

// ---------------------------------------------------------------------------
// Followed channels
// ---------------------------------------------------------------------------

/// How this peer participates in a channel.
///
/// **`Observe` remains the default** — a bare `channelId` with no `=mode`
/// suffix parses to it, so a config that says nothing about applying never
/// applies. `Apply` and `Canary` must each be asked for by name, per channel.
///
/// **The threshold gates promotion (an EARNED head), never staging
/// adoption** (design 2026-09-01, the canary-first-adoption fix). `Apply`
/// adopts EARNED heads only — a verified STAGING head there reports
/// `Verdict::Waiting`, never a refusal, because the threshold this mode
/// enforces can only ever be met once a canary has soaked the release first.
/// `Canary` is what closes that loop: it adopts a STAGING head (no threshold
/// enforced — the soak IS the evidence being gathered) as well as an EARNED
/// one (threshold enforced, exactly like `Apply`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AdoptionMode {
    /// Watch, fetch, verify, report. Applies nothing, ever.
    Observe,
    /// Everything `Observe` does, and then routes a release that passed the
    /// whole floor to the vehicle its artifact class names
    /// (`super::apply`) — but only an EARNED head. Idempotent on
    /// `(channelId, releaseCid)`.
    Apply,
    /// Everything `Observe` does, and applies a verified release of EITHER
    /// tier: a STAGING head starts this peer's soak window (attestation
    /// threshold read and reported, never enforced), an EARNED head applies
    /// with the threshold enforced exactly as `Apply`. Idempotent on
    /// `(channelId, releaseCid)` exactly like `Apply`.
    Canary,
}

impl AdoptionMode {
    pub fn label(self) -> &'static str {
        match self {
            AdoptionMode::Observe => "observe",
            AdoptionMode::Apply => "apply",
            AdoptionMode::Canary => "canary",
        }
    }

    /// Whether this mode may hand a verified release to a vehicle.
    pub fn applies(self) -> bool {
        matches!(self, AdoptionMode::Apply | AdoptionMode::Canary)
    }

    /// Parse a mode from the runtime-config value.
    ///
    /// An unknown mode is an ERROR, not a fallback to `observe`. Falling back
    /// would make a peer that asked for `apply`/`canary` look like it was
    /// following instructions while doing something else — the config-lever
    /// failure mode this crate has already paid for once. The inverse matters
    /// just as much now that both are legal: a TYPO must never silently
    /// become one of them, which is why the match is exact and there is no
    /// prefix or fuzzy leg.
    pub fn parse(raw: &str) -> Result<Self, RefusalReason> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "observe" => Ok(AdoptionMode::Observe),
            "apply" => Ok(AdoptionMode::Apply),
            "canary" => Ok(AdoptionMode::Canary),
            _ => Err(RefusalReason::ModeNotPermitted),
        }
    }
}

/// One channel this peer follows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowedChannel {
    pub channel_id: String,
    pub mode: AdoptionMode,
}

/// A config entry that did NOT become a followed channel, and why.
///
/// Surfaced on `/admin/adoption` rather than logged and dropped: "I edited the
/// ConfigMap and nothing happened" must be answerable from the report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelConfigRefusal {
    pub entry: String,
    pub refusal: AdoptionRefusal,
}

/// The result of reading the followed-channel config.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowedChannels {
    pub channels: Vec<FollowedChannel>,
    pub refused: Vec<ChannelConfigRefusal>,
}

/// Parse the `ELOHIM_RELEASE_CHANNELS` value.
///
/// Tolerant of the shapes an operator actually types (commas, newlines, extra
/// spaces) and intolerant of the ones that would change behaviour silently (an
/// unknown mode, a duplicate channel with a conflicting mode). Duplicates of the
/// SAME channel with the same mode collapse — idempotent config is not an
/// error.
pub fn parse_followed_channels(raw: &str) -> FollowedChannels {
    let mut out = FollowedChannels::default();
    for entry in raw
        .split([',', '\n', ';'])
        .map(str::trim)
        .filter(|e| !e.is_empty())
    {
        let (channel_id, mode_raw) = match entry.split_once('=') {
            Some((c, m)) => (c.trim(), m.trim()),
            None => (entry, "observe"),
        };
        if channel_id.is_empty() {
            out.refused.push(ChannelConfigRefusal {
                entry: entry.to_string(),
                refusal: AdoptionRefusal::new(
                    RefusalReason::ModeNotPermitted,
                    "empty channel id in ELOHIM_RELEASE_CHANNELS",
                ),
            });
            continue;
        }
        let mode = match AdoptionMode::parse(mode_raw) {
            Ok(m) => m,
            Err(reason) => {
                out.refused.push(ChannelConfigRefusal {
                    entry: entry.to_string(),
                    refusal: AdoptionRefusal::new(
                        reason,
                        format!(
                            "mode '{mode_raw}' is not a mode — legal values are 'observe' \
                             (the default) and 'apply'"
                        ),
                    ),
                });
                continue;
            }
        };
        if let Some(existing) = out.channels.iter().find(|c| c.channel_id == channel_id) {
            if existing.mode != mode {
                out.refused.push(ChannelConfigRefusal {
                    entry: entry.to_string(),
                    refusal: AdoptionRefusal::new(
                        RefusalReason::ModeNotPermitted,
                        format!(
                            "channel '{channel_id}' declared twice with conflicting modes \
                             ('{}' then '{}') — the first declaration stands",
                            existing.mode.label(),
                            mode.label()
                        ),
                    ),
                });
            }
            continue;
        }
        out.channels.push(FollowedChannel {
            channel_id: channel_id.to_string(),
            mode,
        });
    }
    out
}

// ---------------------------------------------------------------------------
// Per-channel state
// ---------------------------------------------------------------------------

/// The election tier behind a resolved head.
///
/// **C4.** `None` is a real, reportable answer — "this conductor sees no
/// election for this channel" — and it is NOT "latest", NOT "nothing exists",
/// and NOT an error. A channel with no earned head leaves the controller idle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HeadTier {
    /// The winning declaration carried the EARNED provenance marker.
    Earned,
    /// A declaration stands, but staging-tier.
    Staging,
    /// No election resolved — the root-author fallback, or a pre-cure
    /// coordinator. Honest absence of authority, never a guess at one.
    None,
}

impl HeadTier {
    pub fn label(self) -> &'static str {
        match self {
            HeadTier::Earned => "earned",
            HeadTier::Staging => "staging",
            HeadTier::None => "none",
        }
    }

    /// Read the tier off a resolved head answer. Mirrors
    /// `ContentHeadWire::canonical_tier_label` exactly — the meter must not
    /// claim an election it has no evidence for, so the absence of
    /// `canonical_earned` reads as `none`, not as `staging`.
    pub fn from_canonical_earned(canonical_earned: Option<bool>) -> Self {
        match canonical_earned {
            Some(true) => HeadTier::Earned,
            Some(false) => HeadTier::Staging,
            None => HeadTier::None,
        }
    }
}

/// The head a sweep resolved, as reported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedHead {
    /// The winning version's action hash (base64) — the release CID.
    pub cid: String,
    pub tier: HeadTier,
}

/// What a sweep concluded about one channel.
///
/// `Idle` is the C4/C3 answer and is structurally distinct from both `Ok` and
/// `Refused`: a channel this conductor sees no head for has not passed and has
/// not failed — there is nothing to judge yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// The channel resolved and there is no head to judge (no election, or the
    /// channel carries no release manifest yet).
    Idle { note: String },
    /// The release passed the whole verify floor on this peer. In `observe`
    /// mode this is where a sweep ends.
    Ok { release_cid: String },
    /// **T4.** The release passed the floor AND this peer is converged on it.
    ///
    /// `already_current` distinguishes the two ways that is true: a fresh apply
    /// this sweep (`false`), or an idempotent no-op because the peer had
    /// already applied this exact `(channelId, releaseCid)` (`true`). Both are
    /// success; only one is a change, and a report that could not tell them
    /// apart would show a converged fleet as a continuously-applying one.
    Applied {
        release_cid: String,
        vehicle: String,
        already_current: bool,
    },
    /// **Design 2026-09-01 (canary-first adoption).** The release passed the
    /// whole verify floor and carries a STAGING head, but this peer's mode is
    /// `Apply` — which adopts EARNED heads only. Neither a pass-and-stop
    /// (`Ok`) nor a refusal: nothing about the release or this peer is wrong,
    /// it is simply not the mode that soaks a staging head. `Canary` mode
    /// never produces this — a canary applies a verified staging head; only
    /// `Apply` waits for the promotion ceremony instead.
    Waiting { release_cid: String, detail: String },
    /// The controller refused, with a typed reason.
    Refused { refusal: AdoptionRefusal },
}

/// Hand-written so the wire shape can carry BOTH discriminators.
///
/// The task atom's report contract spells a verdict `{ok} | {refusal: …}` —
/// keyed. A tagged `state` field is the more robust form and is the only one
/// that can name the third honest case (`idle`), which the two-way contract has
/// no room for. Emitting both means a consumer may key on either and neither is
/// a rename away from silence — and it keeps `/admin/adoption` additive for T6,
/// which is the one extension rule this surface has.
impl Serialize for Verdict {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        match self {
            Verdict::Idle { note } => {
                map.serialize_entry("state", "idle")?;
                map.serialize_entry("note", note)?;
            }
            Verdict::Ok { release_cid } => {
                map.serialize_entry("state", "ok")?;
                map.serialize_entry("ok", &true)?;
                map.serialize_entry("releaseCid", release_cid)?;
            }
            Verdict::Applied {
                release_cid,
                vehicle,
                already_current,
            } => {
                map.serialize_entry("state", "applied")?;
                // ADDITIVE: `applied` still answers `ok: true`, so a T6 reader
                // keyed on the atom's `{ok} | {refusal}` contract keeps working
                // unchanged when a channel moves from observe to apply.
                map.serialize_entry("ok", &true)?;
                map.serialize_entry("releaseCid", release_cid)?;
                map.serialize_entry("vehicle", vehicle)?;
                map.serialize_entry("alreadyCurrent", already_current)?;
            }
            Verdict::Waiting {
                release_cid,
                detail,
            } => {
                map.serialize_entry("state", "waiting")?;
                // ADDITIVE, same reasoning as `applied`: the release DID
                // verify, so a T6 reader keyed on the atom's `{ok} | {refusal}`
                // contract still sees a pass, while a tagged-`state` reader
                // gets the strictly more informative `waiting` plus why.
                map.serialize_entry("ok", &true)?;
                map.serialize_entry("releaseCid", release_cid)?;
                map.serialize_entry("reason", super::REASON_AWAITING_PROMOTION)?;
                map.serialize_entry("detail", detail)?;
            }
            Verdict::Refused { refusal } => {
                map.serialize_entry("state", "refused")?;
                map.serialize_entry("refusal", refusal)?;
            }
        }
        map.end()
    }
}

impl Verdict {
    /// The metric `reason` label for this verdict.
    pub fn reason_label(&self) -> &str {
        match self {
            Verdict::Idle { .. } => super::REASON_IDLE,
            Verdict::Ok { .. } => super::REASON_OK,
            Verdict::Applied {
                already_current: true,
                ..
            } => super::REASON_ALREADY_CURRENT,
            Verdict::Applied { .. } => super::REASON_OK,
            Verdict::Waiting { .. } => super::REASON_AWAITING_PROMOTION,
            Verdict::Refused { refusal } => &refusal.reason,
        }
    }
}

/// What this peer has actually applied on a channel — the `appliedRelease` row
/// on `GET /admin/adoption`, and the **idempotency key** the apply arm reads
/// before it spends anything.
///
/// Ephemeral (C) like everything else here: it is a record of what THIS process
/// did, not an authority claim, and it is deliberately not persisted. A
/// restarted peer re-derives convergence from its own installed reality via the
/// verify floor rather than trusting a file about its own past — which is the
/// same reason `already_current` is cheap to be wrong about in the safe
/// direction (a re-apply of the current head is a no-op at the vehicle too).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppliedRelease {
    /// The release CID (the winning version's action hash).
    pub cid: String,
    /// Unix seconds the apply completed.
    pub at: i64,
    /// Which vehicle acted.
    pub vehicle: String,
}

/// Everything the controller knows about one followed channel right now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelAdoptionState {
    pub channel_id: String,
    pub mode: AdoptionMode,
    /// `None` when the head could not be resolved at all. **Unreachable is not
    /// absence**: a null here plus a `conductor_unavailable` refusal says "we
    /// could not ask", while a `tier: "none"` says "we asked and there is no
    /// election".
    pub resolved_head: Option<ResolvedHead>,
    /// `None` until the first sweep touches this channel.
    pub verdict: Option<Verdict>,
    /// Unix seconds of the last completed check.
    pub last_checked_at: Option<i64>,
    /// Unix seconds before which the next sweep skips this channel — the
    /// backoff ladder made visible so an operator can see WHY a channel looks
    /// stale instead of inferring it.
    pub next_check_not_before: Option<i64>,
    /// How many consecutive sweeps ended in a refusal. Resets on `Ok`/`Idle`.
    pub consecutive_refusals: u32,
    /// Total sweeps that touched this channel.
    pub sweeps: u64,
    /// **T4, additive.** What this peer has applied on this channel, or `None`
    /// in `observe` mode / before the first apply. The apply arm's idempotency
    /// key: a sweep that resolves this same `cid` again is `already_current`
    /// and spends nothing beyond the resolve.
    #[serde(default)]
    pub applied_release: Option<AppliedRelease>,
    /// **T4, additive.** True when a vehicle staged something that only takes
    /// effect on the next process start — today exactly the `storage-binary`
    /// exe-slot, which is STAGED and never self-exec'd. T6's mesh receipt reads
    /// this alongside the slot path.
    #[serde(default)]
    pub pending_restart: bool,
    /// **Design 2026-09-01.** The attestation-threshold evidence this sweep
    /// read for the channel's resolved head — populated whenever the read was
    /// attempted this sweep, `None` when it was not (per-sweep read budget
    /// exhausted, or the check ended before the threshold arm). On a STAGING
    /// head this is soak PROGRESS, not a gate: `verify::verify` enforces the
    /// threshold only at `HeadTier::Earned`. Read this to see WHY a `canary`
    /// hasn't attested yet, or how close an `apply`/`observe` peer's staging
    /// head is to promotion.
    #[serde(default)]
    pub attestations: Option<QualifyingEvidence>,
}

impl ChannelAdoptionState {
    pub fn new(channel: &FollowedChannel) -> Self {
        Self {
            channel_id: channel.channel_id.clone(),
            mode: channel.mode,
            resolved_head: None,
            verdict: None,
            last_checked_at: None,
            next_check_not_before: None,
            consecutive_refusals: 0,
            sweeps: 0,
            applied_release: None,
            pending_restart: false,
            attestations: None,
        }
    }

    /// Whether a sweep at `now_unix` should skip this channel for backoff.
    pub fn is_backing_off(&self, now_unix: i64) -> bool {
        self.next_check_not_before
            .is_some_and(|not_before| now_unix < not_before)
    }

    /// Record a completed check. Owns the backoff ladder so a caller cannot
    /// forget to advance (or to reset) it.
    ///
    /// `attestations` is the threshold evidence this check actually read (or
    /// `None` when it never reached that arm this sweep) — overwritten every
    /// check exactly like `resolved_head`, so a stale count never survives a
    /// sweep that could not re-read it.
    pub fn record(
        &mut self,
        now_unix: i64,
        head: Option<ResolvedHead>,
        verdict: Verdict,
        attestations: Option<QualifyingEvidence>,
    ) {
        self.sweeps += 1;
        self.last_checked_at = Some(now_unix);
        self.resolved_head = head;
        self.attestations = attestations;
        let backoff = match &verdict {
            Verdict::Ok { .. } | Verdict::Idle { .. } | Verdict::Applied { .. }
            // A `Waiting` peer is doing nothing wrong — `apply` mode adopts
            // EARNED heads only, and this is what soaking a staging head
            // through a different mode is SUPPOSED to look like. It clears
            // the ladder exactly like a pass: a channel promoted (or
            // reverted) by the ceremony must be re-checked at full cadence,
            // not throttled as if it had refused.
            | Verdict::Waiting { .. } => {
                self.consecutive_refusals = 0;
                BACKOFF_LADDER_SECS[0]
            }
            Verdict::Refused { refusal } => {
                self.consecutive_refusals = self.consecutive_refusals.saturating_add(1);
                if refusal.reason_code().is_transient() {
                    let rung =
                        (self.consecutive_refusals as usize).min(BACKOFF_LADDER_SECS.len() - 1);
                    BACKOFF_LADDER_SECS[rung]
                } else {
                    // Terminal: only a NEW release changes this. Park at the
                    // ceiling immediately rather than climbing to it.
                    TERMINAL_BACKOFF_SECS
                }
            }
        };
        self.next_check_not_before = if backoff == 0 {
            None
        } else {
            Some(now_unix.saturating_add(backoff as i64))
        };
        self.verdict = Some(verdict);
    }

    /// Record that a vehicle applied a release on this channel.
    ///
    /// `pending_restart` is OR-ed in, never assigned: a staged artifact that
    /// only a process restart consumes stays pending until that restart ends
    /// this process. A later apply of a different class must not clear it.
    pub fn record_applied(&mut self, applied: AppliedRelease, pending_restart: bool) {
        self.applied_release = Some(applied);
        self.pending_restart = self.pending_restart || pending_restart;
    }
}

// ---------------------------------------------------------------------------
// The process-local registry (Ephemeral C)
// ---------------------------------------------------------------------------

struct Registry {
    channels: Mutex<BTreeMap<String, ChannelAdoptionState>>,
    config_refusals: Mutex<Vec<ChannelConfigRefusal>>,
    controller_running: Mutex<bool>,
    last_sweep_unix: Mutex<Option<i64>>,
    sweeps: Mutex<u64>,
}

static REGISTRY: LazyLock<Registry> = LazyLock::new(|| Registry {
    channels: Mutex::new(BTreeMap::new()),
    config_refusals: Mutex::new(Vec::new()),
    controller_running: Mutex::new(false),
    last_sweep_unix: Mutex::new(None),
    sweeps: Mutex::new(0),
});

pub fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Mark the controller as live. Reported so a flat `/admin/adoption` is
/// distinguishable from "the controller never started".
pub fn mark_controller_running(running: bool) {
    *REGISTRY.controller_running.lock().unwrap() = running;
}

/// Reconcile the registry against the current followed-channel config.
///
/// Channels that left the config are DROPPED (their state was only ever a
/// record of a sweep); channels that arrived get a fresh state; channels that
/// stayed keep their backoff and history. A mode change resets the state, which
/// is the honest thing: the previous verdict was reached under a different
/// participation contract.
pub fn reconcile_followed(followed: &FollowedChannels) {
    let mut channels = REGISTRY.channels.lock().unwrap();
    channels.retain(|id, _| followed.channels.iter().any(|c| &c.channel_id == id));
    for channel in &followed.channels {
        match channels.get_mut(&channel.channel_id) {
            Some(existing) if existing.mode == channel.mode => {}
            _ => {
                channels.insert(
                    channel.channel_id.clone(),
                    ChannelAdoptionState::new(channel),
                );
            }
        }
    }
    *REGISTRY.config_refusals.lock().unwrap() = followed.refused.clone();
}

/// Snapshot of one channel's state, for a sweep to decide whether to skip it.
pub fn channel_state(channel_id: &str) -> Option<ChannelAdoptionState> {
    REGISTRY.channels.lock().unwrap().get(channel_id).cloned()
}

/// Record a completed check for one channel.
pub fn record_check(
    channel_id: &str,
    now_unix_secs: i64,
    head: Option<ResolvedHead>,
    verdict: Verdict,
    attestations: Option<QualifyingEvidence>,
) {
    if let Some(state) = REGISTRY.channels.lock().unwrap().get_mut(channel_id) {
        state.record(now_unix_secs, head, verdict, attestations);
    }
}

/// The release this peer has applied on `channel_id`, if any.
///
/// **C6b — the idempotency read.** The apply arm calls this BEFORE it spends a
/// threshold read, a byte of staging, or a conductor call: a sweep that
/// resolves a head this peer has already applied costs exactly one head
/// resolve and nothing else.
pub fn applied_release(channel_id: &str) -> Option<AppliedRelease> {
    REGISTRY
        .channels
        .lock()
        .unwrap()
        .get(channel_id)
        .and_then(|s| s.applied_release.clone())
}

/// Record that a vehicle applied a release on `channel_id`.
///
/// Separate from [`record_check`] because it is a different KIND of fact: a
/// check is per-sweep and is overwritten every tick, while an apply is a thing
/// this peer did, and it survives every later verdict — including a refusal, so
/// "we applied X and are now refusing Y" is readable rather than erased.
pub fn record_applied(channel_id: &str, applied: AppliedRelease, pending_restart: bool) {
    if let Some(state) = REGISTRY.channels.lock().unwrap().get_mut(channel_id) {
        state.record_applied(applied, pending_restart);
    }
}

/// Record that a sweep completed.
pub fn record_sweep(now_unix_secs: i64) {
    *REGISTRY.last_sweep_unix.lock().unwrap() = Some(now_unix_secs);
    let mut sweeps = REGISTRY.sweeps.lock().unwrap();
    *sweeps = sweeps.saturating_add(1);
}

/// Every channel's state, in channel-id order.
pub fn snapshot() -> Vec<ChannelAdoptionState> {
    REGISTRY
        .channels
        .lock()
        .unwrap()
        .values()
        .cloned()
        .collect()
}

/// Body for `GET /admin/adoption`.
///
/// **T6's receipt input — extend only additively.** Every field here is either
/// an observation this peer made or a declaration it read; nothing is an
/// authority claim.
pub fn report_json() -> serde_json::Value {
    let channels = snapshot();
    let config_refusals = REGISTRY.config_refusals.lock().unwrap().clone();
    serde_json::json!({
        "controller": {
            "running": *REGISTRY.controller_running.lock().unwrap(),
            // Whether this BINARY can apply at all, stated as a fact of the
            // build rather than of the config — so `mode: observe` on every
            // channel is never mistaken for a build that could not have done
            // otherwise, and (since T4) `mode: apply` is never mistaken for a
            // build that has a vehicle for the class in question.
            "applyVehiclesCompiled": true,
            // Which artifact classes THIS process actually has a vehicle for.
            // Registered at boot; empty when no vehicles were wired (a node
            // with no conductor and no config path), which is the honest
            // reading of "compiled but not equipped".
            "applyVehicles": super::apply::registered_vehicle_labels(),
            "configKey": RELEASE_CHANNELS_KEY,
            "sweepIntervalSecs": super::watch::SWEEP_INTERVAL_SECS,
            "maxChannelsPerSweep": super::watch::MAX_CHANNELS_PER_SWEEP,
            "maxArtifactBytesPerSweep": super::watch::MAX_ARTIFACT_BYTES_PER_SWEEP,
            "backoffLadderSecs": BACKOFF_LADDER_SECS,
            "sweeps": *REGISTRY.sweeps.lock().unwrap(),
            "lastSweepUnixSecs": *REGISTRY.last_sweep_unix.lock().unwrap(),
        },
        "channels": channels,
        "configRefusals": config_refusals,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observe(id: &str) -> FollowedChannel {
        FollowedChannel {
            channel_id: id.to_string(),
            mode: AdoptionMode::Observe,
        }
    }

    #[test]
    fn a_bare_channel_id_defaults_to_observe() {
        let parsed = parse_followed_channels("runtime:coordinators:elohim:commons");
        assert_eq!(
            parsed.channels,
            vec![observe("runtime:coordinators:elohim:commons")]
        );
        assert!(parsed.refused.is_empty());
    }

    #[test]
    fn commas_newlines_and_spaces_all_separate_entries() {
        let parsed = parse_followed_channels(
            " runtime:coordinators:elohim:commons=observe ,\n runtime:config:elohim:commons ",
        );
        assert_eq!(
            parsed.channels,
            vec![
                observe("runtime:coordinators:elohim:commons"),
                observe("runtime:config:elohim:commons"),
            ]
        );
    }

    /// An unknown mode must never silently degrade to `observe`: a peer told to
    /// `apply` that quietly observes looks compliant while doing something
    /// else. It is refused, and the refusal is REPORTED — dropping it is how
    /// "I edited the ConfigMap and nothing happened" becomes unanswerable.
    #[test]
    fn an_illegal_mode_is_refused_and_reported_never_downgraded() {
        let parsed = parse_followed_channels("runtime:coordinators:elohim:commons=aply");
        assert!(parsed.channels.is_empty());
        assert_eq!(parsed.refused.len(), 1);
        assert_eq!(
            parsed.refused[0].refusal.reason_code(),
            RefusalReason::ModeNotPermitted
        );
        assert_eq!(
            parsed.refused[0].entry,
            "runtime:coordinators:elohim:commons=aply"
        );
    }

    /// **T4's mode gate, and its default.** `apply` is now a legal declaration
    /// — and it must be spelled exactly, per channel. A bare channel id, and
    /// anything that merely LOOKS like `apply`, must never become one: a peer
    /// that applies because of a typo is the failure mode the whole
    /// observe-first ladder exists to avoid.
    #[test]
    fn apply_is_legal_but_only_when_asked_for_by_exact_name() {
        let parsed = parse_followed_channels(
            "runtime:coordinators:elohim:canary-a=apply, runtime:config:elohim:commons",
        );
        assert_eq!(
            parsed.channels,
            vec![
                FollowedChannel {
                    channel_id: "runtime:coordinators:elohim:canary-a".to_string(),
                    mode: AdoptionMode::Apply,
                },
                observe("runtime:config:elohim:commons"),
            ],
            "apply must be per-channel and observe must remain the bare default"
        );
        assert!(parsed.refused.is_empty());
        assert!(AdoptionMode::Apply.applies());
        assert!(!AdoptionMode::Observe.applies());

        // Near-misses are refusals, never applies.
        for typo in ["aply", "apply!", "applying", "app ly", "APPLY-", "auto"] {
            let entry = format!("runtime:coordinators:elohim:x={typo}");
            let parsed = parse_followed_channels(&entry);
            assert!(
                parsed.channels.is_empty() && parsed.refused.len() == 1,
                "'{typo}' must refuse, not apply"
            );
        }
        // Case and surrounding whitespace are the only tolerances.
        assert_eq!(AdoptionMode::parse(" APPLY "), Ok(AdoptionMode::Apply));
    }

    /// **The canary-first-adoption fix.** `canary` is the third legal mode,
    /// spelled exactly, per channel — the same discipline `apply` got in T4.
    /// A typo must refuse, never silently become a mode that ACTS.
    #[test]
    fn canary_is_legal_but_only_when_asked_for_by_exact_name() {
        let parsed = parse_followed_channels(
            "runtime:coordinators:elohim:canary-a=canary, runtime:config:elohim:commons",
        );
        assert_eq!(
            parsed.channels,
            vec![
                FollowedChannel {
                    channel_id: "runtime:coordinators:elohim:canary-a".to_string(),
                    mode: AdoptionMode::Canary,
                },
                observe("runtime:config:elohim:commons"),
            ],
            "canary must be per-channel and observe must remain the bare default"
        );
        assert!(parsed.refused.is_empty());
        assert!(AdoptionMode::Canary.applies());

        // Near-misses are refusals, never a silent canary.
        for typo in [
            "canry",
            "canary!",
            "canarying",
            "can ary",
            "CANARY-",
            "canaries",
        ] {
            let entry = format!("runtime:coordinators:elohim:x={typo}");
            let parsed = parse_followed_channels(&entry);
            assert!(
                parsed.channels.is_empty() && parsed.refused.len() == 1,
                "'{typo}' must refuse, not become canary"
            );
        }
        // Case and surrounding whitespace are the only tolerances.
        assert_eq!(AdoptionMode::parse(" CANARY "), Ok(AdoptionMode::Canary));
    }

    /// A mode CHANGE resets the channel's state — the previous verdict was
    /// reached under a different participation contract, and an
    /// `appliedRelease` carried across an observe⇄apply flip would let a
    /// re-enabled channel claim convergence it never re-established.
    #[test]
    fn flipping_a_channel_between_observe_and_apply_resets_its_state() {
        let mut state = ChannelAdoptionState::new(&observe("c"));
        state.applied_release = Some(AppliedRelease {
            cid: "uhCkkOld".to_string(),
            at: 1_000,
            vehicle: "sync_coordinators".to_string(),
        });
        state.pending_restart = true;

        let flipped = ChannelAdoptionState::new(&FollowedChannel {
            channel_id: "c".to_string(),
            mode: AdoptionMode::Apply,
        });
        assert_eq!(flipped.applied_release, None);
        assert!(!flipped.pending_restart);
        assert_ne!(flipped.mode, state.mode);
    }

    #[test]
    fn a_duplicate_declaration_of_the_same_channel_collapses() {
        let same = parse_followed_channels("a=observe,a=observe");
        assert_eq!(same.channels.len(), 1);
        assert!(same.refused.is_empty());
    }

    /// **C6a.** The ladder is finite: a channel that refuses forever holds at
    /// the ceiling, so the controller neither hammers nor gives up.
    #[test]
    fn transient_refusals_climb_a_finite_ladder_and_hold_at_the_ceiling() {
        let mut state = ChannelAdoptionState::new(&observe("c"));
        let mut previous = 0i64;
        for sweep in 1..=10 {
            let now = sweep * 10_000;
            state.record(
                now,
                None,
                Verdict::Refused {
                    refusal: AdoptionRefusal::new(RefusalReason::ArtifactUnavailable, "no peer"),
                },
                None,
            );
            let delay = state.next_check_not_before.unwrap() - now;
            assert!(delay <= TERMINAL_BACKOFF_SECS as i64, "ladder is finite");
            assert!(delay >= previous, "ladder is monotone");
            previous = delay;
        }
        assert_eq!(previous, TERMINAL_BACKOFF_SECS as i64);
    }

    /// A terminal refusal parks at the ceiling on the FIRST sweep. Climbing to
    /// it would spend four more conductor round trips on a question only a new
    /// release can reopen.
    #[test]
    fn a_terminal_refusal_parks_at_the_ceiling_immediately() {
        let mut state = ChannelAdoptionState::new(&observe("c"));
        state.record(
            1_000,
            None,
            Verdict::Refused {
                refusal: AdoptionRefusal::new(
                    RefusalReason::DnaLineageMismatch,
                    "role lamad binds a different DNA",
                ),
            },
            None,
        );
        assert_eq!(
            state.next_check_not_before,
            Some(1_000 + TERMINAL_BACKOFF_SECS as i64)
        );
        assert!(state.is_backing_off(1_500));
        assert!(!state.is_backing_off(1_000 + TERMINAL_BACKOFF_SECS as i64));
    }

    /// **C4.** A success or an honest idle clears the ladder entirely — a cured
    /// channel is checked at full cadence on the very next sweep.
    #[test]
    fn success_and_honest_idle_both_clear_the_backoff() {
        let mut state = ChannelAdoptionState::new(&observe("c"));
        state.record(
            1_000,
            None,
            Verdict::Refused {
                refusal: AdoptionRefusal::new(RefusalReason::ArtifactUnavailable, "no peer"),
            },
            None,
        );
        assert!(state.next_check_not_before.is_some());
        state.record(
            2_000,
            None,
            Verdict::Idle {
                note: "no election".to_string(),
            },
            None,
        );
        assert_eq!(state.next_check_not_before, None);
        assert_eq!(state.consecutive_refusals, 0);
    }

    /// **C4.** The three tiers are distinguishable, and a missing
    /// `canonical_earned` reads as `none` — never as `staging`. Claiming a tier
    /// the answer carries no evidence for is the failure this mirrors
    /// `ContentHeadWire::canonical_tier_label` to avoid.
    #[test]
    fn head_tier_never_invents_an_election() {
        assert_eq!(
            HeadTier::from_canonical_earned(Some(true)),
            HeadTier::Earned
        );
        assert_eq!(
            HeadTier::from_canonical_earned(Some(false)),
            HeadTier::Staging
        );
        assert_eq!(HeadTier::from_canonical_earned(None), HeadTier::None);
        assert_eq!(HeadTier::None.label(), "none");
    }

    #[test]
    fn verdicts_project_the_metric_reason_label() {
        assert_eq!(
            Verdict::Ok {
                release_cid: "x".into()
            }
            .reason_label(),
            "ok"
        );
        assert_eq!(
            Verdict::Idle {
                note: "n".to_string()
            }
            .reason_label(),
            "idle"
        );
        assert_eq!(
            Verdict::Refused {
                refusal: AdoptionRefusal::new(RefusalReason::ThresholdUnchecked, "t")
            }
            .reason_label(),
            "threshold_unchecked"
        );
    }

    /// **T6's receipt contract.** The wire shape is pinned because it is
    /// consumed by a task that has not been written yet: a rename here would be
    /// silent until that receipt lands red on a live mesh. Both discriminators
    /// are asserted — the atom's keyed `{ok} | {refusal: …}` AND the tagged
    /// `state`, which is the only one that can name `idle`.
    #[test]
    fn the_admin_adoption_verdict_shape_is_pinned_for_t6() {
        let ok = serde_json::to_value(Verdict::Ok {
            release_cid: "uhCkkWinner".to_string(),
        })
        .unwrap();
        assert_eq!(ok["state"], "ok");
        assert_eq!(ok["ok"], true);
        assert_eq!(ok["releaseCid"], "uhCkkWinner");

        let refused = serde_json::to_value(Verdict::Refused {
            refusal: AdoptionRefusal::new(RefusalReason::DnaLineageMismatch, "role lamad"),
        })
        .unwrap();
        assert_eq!(refused["state"], "refused");
        assert_eq!(refused["refusal"]["reason"], "dna_lineage_mismatch");
        assert_eq!(refused["refusal"]["arm"], "verify");
        assert_eq!(refused["refusal"]["transient"], false);
        assert_eq!(refused["refusal"]["detail"], "role lamad");

        let idle = serde_json::to_value(Verdict::Idle {
            note: "no election".to_string(),
        })
        .unwrap();
        assert_eq!(idle["state"], "idle");
        assert!(
            idle.get("ok").is_none() && idle.get("refusal").is_none(),
            "idle is neither a pass nor a refusal — it must not answer to either key"
        );
    }

    /// **T4, additive.** The `applied` verdict extends the pinned shape without
    /// breaking it: a T6 reader keyed on the atom's `{ok} | {refusal}` contract
    /// still sees `ok: true`, while a reader keyed on the tagged `state` gets
    /// the strictly more informative `applied` plus the vehicle that acted and
    /// whether anything actually changed.
    #[test]
    fn the_applied_verdict_extends_the_pinned_shape_without_breaking_it() {
        let fresh = serde_json::to_value(Verdict::Applied {
            release_cid: "uhCkkWinner".to_string(),
            vehicle: "sync_coordinators".to_string(),
            already_current: false,
        })
        .unwrap();
        assert_eq!(fresh["state"], "applied");
        assert_eq!(fresh["ok"], true, "T6's keyed contract still answers");
        assert_eq!(fresh["releaseCid"], "uhCkkWinner");
        assert_eq!(fresh["vehicle"], "sync_coordinators");
        assert_eq!(fresh["alreadyCurrent"], false);

        let converged = Verdict::Applied {
            release_cid: "uhCkkWinner".to_string(),
            vehicle: "sync_coordinators".to_string(),
            already_current: true,
        };
        assert_eq!(
            converged.reason_label(),
            super::super::REASON_ALREADY_CURRENT,
            "a converged peer must not be metered as a continuously-applying one"
        );
        assert_eq!(
            Verdict::Applied {
                release_cid: "c".to_string(),
                vehicle: "v".to_string(),
                already_current: false,
            }
            .reason_label(),
            super::super::REASON_OK
        );
    }

    /// **The canary-first-adoption fix.** `waiting` extends the pinned shape
    /// the same additive way `applied` does: `ok: true` because the release
    /// DID verify, plus its own `reason`/`detail` so a T6 reader can tell "not
    /// applied because staging" apart from "not applied because refused".
    #[test]
    fn the_waiting_verdict_extends_the_pinned_shape_without_breaking_it() {
        let waiting = serde_json::to_value(Verdict::Waiting {
            release_cid: "uhCkkStaging".to_string(),
            detail: "verified; this peer adopts only earned releases — the canary soaks it \
                     first"
                .to_string(),
        })
        .unwrap();
        assert_eq!(waiting["state"], "waiting");
        assert_eq!(waiting["ok"], true, "T6's keyed contract still answers");
        assert_eq!(waiting["releaseCid"], "uhCkkStaging");
        assert_eq!(waiting["reason"], super::super::REASON_AWAITING_PROMOTION);
        assert!(waiting.get("refusal").is_none(), "waiting is not a refusal");
    }

    /// The `state` discriminator is a dashboard contract exactly like the
    /// refusal-reason label set (`refusal_reason_labels_are_stable` in
    /// `mod.rs`) — pinning it makes a rename a deliberate, test-visible act
    /// rather than a silent break of whatever reads `/admin/adoption`.
    #[test]
    fn the_verdict_state_tags_are_stable() {
        fn state_tag(v: &Verdict) -> String {
            serde_json::to_value(v).unwrap()["state"]
                .as_str()
                .unwrap()
                .to_string()
        }
        assert_eq!(
            state_tag(&Verdict::Idle {
                note: "n".to_string()
            }),
            "idle"
        );
        assert_eq!(
            state_tag(&Verdict::Ok {
                release_cid: "c".to_string()
            }),
            "ok"
        );
        assert_eq!(
            state_tag(&Verdict::Applied {
                release_cid: "c".to_string(),
                vehicle: "v".to_string(),
                already_current: false,
            }),
            "applied"
        );
        assert_eq!(
            state_tag(&Verdict::Waiting {
                release_cid: "c".to_string(),
                detail: "d".to_string(),
            }),
            "waiting"
        );
        assert_eq!(
            state_tag(&Verdict::Refused {
                refusal: AdoptionRefusal::new(RefusalReason::ThresholdUnchecked, "t"),
            }),
            "refused"
        );
    }

    /// A successful apply clears the backoff exactly as `Ok`/`Idle` do — a
    /// converged channel is re-checked at full cadence, which is what makes a
    /// REVERT (the ceremony declaring a prior head canonical) land in one
    /// sweep instead of up to half an hour later.
    #[test]
    fn a_successful_apply_clears_the_backoff_so_a_revert_lands_next_sweep() {
        let mut state = ChannelAdoptionState::new(&FollowedChannel {
            channel_id: "c".to_string(),
            mode: AdoptionMode::Apply,
        });
        state.record(
            1_000,
            None,
            Verdict::Refused {
                refusal: AdoptionRefusal::new(RefusalReason::DeferredBackpressure, "under load"),
            },
            None,
        );
        assert!(state.next_check_not_before.is_some());
        state.record(
            2_000,
            None,
            Verdict::Applied {
                release_cid: "uhCkkA".to_string(),
                vehicle: "sync_coordinators".to_string(),
                already_current: false,
            },
            None,
        );
        assert_eq!(state.next_check_not_before, None);
        assert_eq!(state.consecutive_refusals, 0);
        assert!(!state.is_backing_off(2_001));
    }

    /// **The canary-first fix.** `Waiting` is neither a pass-and-stop nor a
    /// refusal — a peer in `apply` mode that verified a STAGING head is doing
    /// nothing wrong, so it must be checked at full cadence next sweep, never
    /// throttled by the ladder that exists for genuine refusals.
    #[test]
    fn a_waiting_verdict_clears_the_backoff_and_never_counts_as_a_refusal() {
        let mut state = ChannelAdoptionState::new(&FollowedChannel {
            channel_id: "c".to_string(),
            mode: AdoptionMode::Apply,
        });
        state.record(
            1_000,
            None,
            Verdict::Refused {
                refusal: AdoptionRefusal::new(RefusalReason::ArtifactUnavailable, "no peer"),
            },
            None,
        );
        assert!(state.next_check_not_before.is_some());
        state.record(
            2_000,
            Some(ResolvedHead {
                cid: "uhCkkStaging".to_string(),
                tier: HeadTier::Staging,
            }),
            Verdict::Waiting {
                release_cid: "uhCkkStaging".to_string(),
                detail: "verified; this peer adopts only earned releases — the canary soaks it \
                         first"
                    .to_string(),
            },
            Some(QualifyingEvidence {
                qualifying: 0,
                threshold: 1,
                total: 0,
                ..Default::default()
            }),
        );
        assert_eq!(state.next_check_not_before, None);
        assert_eq!(state.consecutive_refusals, 0);
        assert_eq!(
            state.verdict.as_ref().unwrap().reason_label(),
            super::super::REASON_AWAITING_PROMOTION
        );
        assert!(
            state.attestations.is_some(),
            "soak progress is reported even though it was never enforced"
        );
    }

    /// `pendingRestart` is STICKY. A staged binary is on disk until a restart
    /// consumes it; a later sweep reporting `pendingRestart: false` while the
    /// slot still exists is exactly the lie T6's receipt would read as "the
    /// swap already happened".
    #[test]
    fn a_pending_restart_is_never_cleared_by_a_later_sweep() {
        let mut state = ChannelAdoptionState::new(&FollowedChannel {
            channel_id: "runtime:binary:elohim:mesh".to_string(),
            mode: AdoptionMode::Apply,
        });
        state.record_applied(
            AppliedRelease {
                cid: "uhCkkBinary".to_string(),
                at: 1_000,
                vehicle: "exe_slot_stage".to_string(),
            },
            true,
        );
        // A LATER apply of a class that stages nothing must not clear it.
        state.record_applied(
            AppliedRelease {
                cid: "uhCkkConfig".to_string(),
                at: 2_000,
                vehicle: "runtime_config_reload".to_string(),
            },
            false,
        );
        assert!(state.pending_restart, "pendingRestart must not be cleared");
        assert_eq!(state.applied_release.as_ref().map(|a| a.at), Some(2_000));

        // And an ordinary sweep verdict leaves both alone entirely.
        state.record(
            3_000,
            None,
            Verdict::Idle {
                note: "no election".to_string(),
            },
            None,
        );
        assert!(state.pending_restart);
        assert_eq!(state.applied_release.as_ref().map(|a| a.at), Some(2_000));
    }

    /// The report names the build's posture as a FACT OF THE BINARY, so a
    /// reader never has to infer from the per-channel modes whether applying
    /// was a config choice that could have gone the other way.
    #[test]
    fn the_report_states_the_builds_apply_posture() {
        let report = report_json();
        assert_eq!(report["controller"]["applyVehiclesCompiled"], true);
        assert!(
            report["controller"]["applyVehicles"].is_array(),
            "the equipped classes are a fact this node reports, not one a reader infers"
        );
        assert_eq!(report["controller"]["configKey"], RELEASE_CHANNELS_KEY);
        assert!(report["channels"].is_array());
        assert!(report["configRefusals"].is_array());
    }
}
