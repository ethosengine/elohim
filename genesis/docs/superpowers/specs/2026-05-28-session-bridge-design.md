---
status: Design
---

# Session Bridge Design — Visitor and Peer Graduation Patterns

> **Status:** Design draft. Surfaces a substrate primitive (`session-bridge`) that resolves anonymous-visitor onboarding AND peer-to-peer sampling/demo flows on the same shape.
>
> **Origin:** Discovered during the 2026-05-28 thin-client backend-migration sprint (plan: `genesis/docs/superpowers/plans/2026-05-28-thin-client-backend-migration.md`). M-AGGR-2 deferred deletion of `LocalSourceChainService` because 4 write-path consumers (consent, mastery, path-negotiation, mastery-stats) need substrate-coordinator migration. That ticket exposed a deeper gap: the protocol has no first-class vocabulary for tentative participation — anonymous visitors, demo trials, guest-shaped sampling between peer collectives. This spec proposes the primitive.

---

## §0 — Vision: anti-binary participation as protocol ethos

Most P2P protocols make participation binary — you're a member or you're not. Account creation is a one-time cliff; before it you have no first-class presence in the network.

The Elohim Protocol's social shape rejects this. The qahal pillar carries a *graduated* capability surface (see `project_qahal_graduated_capability_surface`). The recovery story uses witnesses, not single keys (see `project_socially_derived_security` + `project_recovery_grandma_standard`). Reach gates outward, standing gates inward. Account-incarnation is heavy by design — generating cryptographic identity, registering with a doorway, configuring socially-derived recovery — and the "grandma standard" pushes hard against throwing that at someone in the first ninety seconds.

If participation is graduated, the protocol needs vocabulary for what comes BEFORE full participation:

- An **anonymous browser visitor** reading reach-gated projections, exploring whether anything in this commons resonates with them
- A **web2-OAuth-identified visitor** who has been pulled across the doorway threshold but hasn't yet incarnated as a peer-native agent
- A **peer-native steward of qahal-A** test-driving qahal-B's customs as a guest before deciding to join
- A **peer-native steward of qahal-A** sampling an EPR-app from qahal-B — caching its manifest locally, running a demo, not yet "installing" it as a member

All four states share a shape: *tentative participation, staged intent, a graduation ceremony that either incarnates the participant into the canonical substrate or lets them walk away with the cache discarded*.

This spec proposes that shape as a substrate primitive — `session-bridge` — consumable by doorway (for web2 paths) and elohim-storage (for peer-local paths), and structurally analogous to (but distinct from) the existing protocol bridges in `bridges/`.

---

## §1 — The four identity states + transitions

```
                                            ┌──────────────────────┐
                                            │  peer-native-member  │
                                            │  (qahal-B)           │
                                            └──────────────────────┘
                                                      ▲
                                                      │ commit
                                                      │
   ┌─────────────────┐       OAuth          ┌───────────────────┐
   │   anonymous     │───── via doorway ───▶│  oauth-identified │
   │   browser       │                      │  via doorway      │
   │   session       │                      │                   │
   └─────────────────┘                      └───────────────────┘
            │                                         │
            │                                         │ incarnate
            │                                         ▼
            │                              ┌──────────────────────┐
            └─ direct incarnation? ───────▶│  peer-native-member  │
                (advanced flow)            │  (qahal-A)           │
                                           └──────────────────────┘
                                                      │
                                                      │ sample
                                                      ▼
                                           ┌──────────────────────┐
                                           │  peer-native-sampling│
                                           │  (visiting qahal-B)  │
                                           └──────────────────────┘
                                                      │
                                                      │ commit
                                                      ▼
                                           ┌──────────────────────┐
                                           │  peer-native-member  │
                                           │  (qahal-B)           │
                                           └──────────────────────┘
```

### State definitions

| State | Identity primitive | Where state lives | Reach surface | Standing surface |
|---|---|---|---|---|
| **anonymous** | ephemeral browser session id (cookie or fingerprint) | browser localStorage / doorway-side projection cache | reach-gated read of public commons; no write surface on substrate | zero |
| **oauth-identified** | doorway-issued identity (OAuth RP per `project_account_layer_oauth_graduation`) | doorway-side session pool | reach-gated read; doorway-mediated staged-intent writes | zero (in protocol terms — doorway-identified is web2-shape, not peer-native standing) |
| **peer-native-sampling** | full peer-native agent identity, but no standing in the foreign context being sampled | sampling cache on the sampler's elohim-storage | reach-gated read of foreign content; limited time-bounded write that does NOT propagate to the host substrate until commitment | zero in the foreign context; existing in home contexts |
| **peer-native-member** | full peer-native agent identity, incarnated in the qahal | source chain + canonical projections | full reach surface per qahal capability gradient; standing as accrued | accrued |

### Transitions

| From | To | Ceremony | Where it lives | Carries staged intent? |
|---|---|---|---|---|
| anonymous | oauth-identified | OAuth flow at doorway | doorway | yes — anonymous staged intent migrates to oauth-shaped staged intent in doorway's session pool |
| oauth-identified | peer-native-member | account incarnation (key generation + qahal join + recovery setup) | doorway + imagodei DNA + qahal DNA | yes — oauth-shaped staged intent replays as real coordinator calls into the target DNAs |
| peer-native-member (A) | peer-native-sampling (in B) | sampling-handshake (acquire B's manifest, RS-decode necessary projections, open sampling cache) | sampler's elohim-storage + B's doorway | n/a — sampling starts empty; intent accumulates during sampling |
| peer-native-sampling (in B) | peer-native-member (B) | join-via-graduation (replay sampling-cache intent through B's coordinators, request membership, accept reach binding) | sampler's elohim-storage + B's coordinators | yes — sampling-cache intent graduates into canonical B-substrate writes |
| any | abandon | session timeout, explicit discard, browser close | wherever the session lived | no — intent discarded; the bridge is fail-closed |

---

## §2 — The `session-bridge` crate — public surface

Lives at `crates/session-bridge/` (not `bridges/`, because `bridges/` is by convention for protocol-to-protocol translators between adjacent canonical substrates; this primitive translates within the protocol between *tentative* and *canonical* of the same substrate).

### Core types

```rust
/// The current state of a participant's relationship to a target context.
/// Target context = the qahal / commons / EPR-app whose substrate this session
/// is tentatively interacting with.
pub enum SessionLifecycle {
    Anonymous {
        session_id: BrowserSessionId,
        opened_at: Timestamp,
        expires_at: Timestamp,
    },
    OauthIdentified {
        session_id: BrowserSessionId,
        oauth_subject: OauthSubject,
        doorway: DoorwayHandle,
        opened_at: Timestamp,
        expires_at: Timestamp,
    },
    PeerNativeSampling {
        sampler_agent_id: AgentPubKey,  // the sampler's home identity
        target_context: ContextHandle,  // the qahal being sampled
        sampling_cache_root: CacheRoot,
        opened_at: Timestamp,
        expires_at: Timestamp,
    },
    PeerNativeMember {
        agent_id: AgentPubKey,
        context: ContextHandle,
        member_since: Timestamp,
    },
}

/// A piece of intent the participant has expressed during their session
/// that should graduate (replay as canonical substrate writes) when they
/// commit to the target context.
///
/// Generic over Pillar so each pillar can define its own staged-intent
/// shape; the bridge handles lifecycle uniformly.
pub trait StagedIntent: Serialize + DeserializeOwned + Send + Sync {
    type Pillar: PillarTag;
    type GraduatedEntry;  // the canonical substrate entry this graduates to

    /// What target context can this intent be graduated into?
    fn target_context(&self) -> &ContextHandle;

    /// Is this intent still actionable given the current lifecycle state?
    /// (e.g. consent expires; a mastery claim becomes stale; a sampling note
    /// loses meaning after sampling-cache discard.)
    fn is_actionable(&self, lifecycle: &SessionLifecycle) -> bool;
}

/// The graduation contract — implemented per intent type. Each impl knows
/// how to replay a piece of staged intent as a real coordinator call into
/// the canonical substrate, and how to handle the failure modes.
pub trait GraduationCeremony {
    type Intent: StagedIntent;
    type Coordinator: CoordinatorHandle;

    /// Execute the graduation. May produce one or more canonical entries
    /// (an intent might decompose into multiple substrate writes), or
    /// reject the intent if the graduating identity can't carry it under
    /// the destination's reach/standing rules.
    async fn graduate(
        &self,
        intent: Self::Intent,
        graduating_identity: AgentPubKey,
        coordinator: &Self::Coordinator,
    ) -> Result<Vec<EntryHash>, GraduationFailure>;
}

/// What the participant can see of the target context.
/// For anonymous + oauth states: a doorway-cached projection slice.
/// For sampling state: a peer-local cache, possibly RS-decoded from a quilt.
/// For member state: not used — the member has direct projection access.
pub struct SamplingCache {
    pub manifest: AppManifest,
    pub projection_slices: HashMap<ViewName, ProjectionSlice>,
    pub cache_root: CacheRoot,
    pub expires_at: Timestamp,
}
```

### Lifecycle operations

```rust
pub trait SessionBridge {
    /// Open a new anonymous session. Used by doorway when an unknown browser
    /// arrives.
    async fn open_anonymous(&self, context: ContextHandle) -> Result<SessionLifecycle, BridgeError>;

    /// Transition anonymous → oauth-identified after a successful OAuth
    /// flow at doorway. The anonymous session's staged intent migrates.
    async fn promote_to_oauth(
        &self,
        session: SessionLifecycle,
        oauth: OauthSubject,
    ) -> Result<SessionLifecycle, BridgeError>;

    /// Open a peer-native sampling session — the sampler is already a peer
    /// in their home context, but visiting a target context as a guest.
    async fn open_sampling(
        &self,
        sampler: AgentPubKey,
        target: ContextHandle,
    ) -> Result<(SessionLifecycle, SamplingCache), BridgeError>;

    /// The graduation ceremony — replay all actionable staged intent for
    /// this session as canonical substrate writes. Returns a manifest of
    /// what graduated, what was rejected (with reasons), and what was
    /// discarded as stale.
    async fn graduate(
        &self,
        session: SessionLifecycle,
        graduating_identity: AgentPubKey,
    ) -> Result<GraduationManifest, GraduationFailure>;

    /// Discard a session. Sampling cache cleared, staged intent dropped.
    /// Idempotent and fail-closed.
    async fn discard(&self, session: SessionLifecycle) -> Result<(), BridgeError>;

    /// Stage an intent — accumulates in the session's intent pool. The
    /// intent will graduate or be discarded depending on whether the
    /// session graduates or abandons.
    async fn stage<I: StagedIntent>(
        &self,
        session: &SessionLifecycle,
        intent: I,
    ) -> Result<StageReceipt, BridgeError>;
}
```

### What the crate does NOT include

- The storage backend for staged intent — implementations provide their own (doorway uses its projection-cache SQLite; elohim-storage uses native diesel)
- The specific intent shapes per pillar — those live in the pillar's crate (lamad defines `StagedMasteryIntent`, imagodei defines `StagedConsentIntent`, etc.)
- The OAuth flow itself — that's doorway's existing OAuth surface
- The peer-native sampling handshake — that's a libp2p-protocol-level concern, lives near the swarm setup

---

## §3 — Staged intent shapes per pillar

Each pillar defines its own staged-intent shape that mirrors the canonical entry it eventually graduates into. The shapes are deliberately *simpler* than the canonical entries (fewer fields, no provenance signatures, no reach annotations) — they represent intent, not yet attestation.

### Lamad — mastery and path intents

```rust
pub struct StagedMasteryIntent {
    pub content_id: ContentId,
    pub mastery_level: MasteryLevel,  // self-assessed during sampling
    pub felt_at: Timestamp,
    pub context: ContextHandle,
}
// Graduates to: lamad::ContentMastery entry on the agent's source chain
// (Category B: agent-scoped private until witness-attested)

pub struct StagedPathExploredIntent {
    pub path_id: PathId,
    pub step_indexes_visited: Vec<usize>,
    pub committed_step_index: Option<usize>,
    pub context: ContextHandle,
}
// Graduates to: lamad::HumanProgress entry update on agent's source chain
```

### Imagodei — consent intents

```rust
pub struct StagedConsentIntent {
    pub subject: ConsentSubject,  // what is being consented to
    pub decision: ConsentDecision,  // grant / deny / not-yet-decided
    pub decided_at: Timestamp,
    pub context: ContextHandle,
}
// Graduates to: imagodei::Consent entry on the agent's source chain
//
// IMPORTANT — consent shape constraints. Anonymous consent is meaningless
// (no identifiable consenter). StagedConsentIntent's is_actionable() must
// return false when lifecycle is Anonymous. It graduates only from
// OauthIdentified onwards.
```

### Qahal — membership-application intents

```rust
pub struct StagedMembershipApplicationIntent {
    pub applying_to: QahalContext,
    pub sponsor_witnesses: Vec<AgentPubKey>,  // accrued during sampling
    pub stated_intent: String,
    pub applied_at: Timestamp,
}
// Graduates to: qahal::MembershipApplication on the qahal's DHT
// (Category A: notarized membership claim)
```

### Shefa — economic-event intents

```rust
pub struct StagedEconomicEventIntent {
    pub intent_kind: LamadEventType,
    pub content_id: Option<ContentId>,
    pub elapsed_ms: Option<u64>,
    pub context: ContextHandle,
}
// Graduates to: shefa::EconomicEvent via the existing M-REA-1
// LamadEventIntent surface (the same intent shape, just held in the
// session bridge until commitment).
//
// Note this is exactly the M-REA-1 surface — the bridge holds the same
// intent shape pre-commitment, then emits it when the agent incarnates.
```

### Pattern

Each pillar's staged-intent shape:
1. Names the eventual canonical entry type
2. Carries enough context to construct the canonical entry at graduation time
3. Knows its own `is_actionable()` predicate (some intents require oauth-or-better; some require sampling state to be the right context; etc.)
4. Is serializable for storage in the session-bridge's staged-intent pool

---

## §4 — The graduation ceremony

The ceremony is the heart of the primitive. It's where tentative becomes canonical. It is **NOT** a mechanical replay — it's an appraisal interaction, and that framing reshapes the whole design.

### The Half-Price Books metaphor

The right intuition is bringing a stack of used books to the Half-Price Books counter. You hand over the batch. They appraise it. They make you an offer. You accept and walk out with a receipt, or you refuse and walk out with your stack intact. The transaction is one batched negotiation, not a series of independent item-level decisions.

The graduation ceremony has the same shape: the participant brings their batched session of staged intent, the substrate (with elohim inference where needed) appraises it, and presents an offered resolution. The participant accepts and graduates, or refuses and walks away with the session intact (until they decide to discard, or until expiry).

The aspiration is that the protocol's economic shape — REA + mutual credit + commons stewardship + elohim wisdom as appraiser — yields resolutions more satisfying than pennies-on-the-dollar. Guest contributions during sampling deserve fair appraisal of their actual value to the commons, not capitalist-resale-shape devaluation.

### Two resolution paths per intent

Each staged intent resolves through one of two paths:

**Deterministic resolution** — clear 1:1 mapping from staged intent to canonical substrate entry. `StagedMasteryIntent { content_id: X, mastery_level: gold }` graduates to `ContentMastery { content_id: X, level: gold, agent: <new-agent>, attested_at: <now> }`. No negotiation needed; the substrate just accepts and writes.

**Negotiated resolution** — the staged intent expresses value whose canonical form requires appraisal. Example: a visitor who sampled three different paths and contributed reflection notes on each. The notes are valuable but their canonical home isn't predetermined — are they private mastery records? Public attestation? Steward-witnessed reflections that feed into the path's lamad signal? The graduation ceremony invokes elohim inference to read the intent's content + the target context's standing-curve + the participant's profile (where consented) and produce a proposed resolution that the participant can review and accept.

### Inputs

- The session lifecycle (its current state determines what staged intent is even eligible to graduate)
- The graduating identity (agent pubkey; for visitor-graduation, this is the freshly-incarnated agent; for sampling-graduation, this is the sampler's existing identity)
- The session's pool of staged intent (every `StagedIntent` accumulated during the session)
- For negotiated resolutions: access to an elohim inference surface (which elohim agent / with what context window / with what authority — see §8)

### Output: `GraduationOffer` → participant accepts/refuses → `GraduationManifest`

```rust
/// First the substrate produces an offer the participant can inspect.
/// This is the "Half-Price Books counter shows you the offered price"
/// step — explicit, reviewable, refusable.
pub struct GraduationOffer {
    pub deterministic_resolutions: Vec<DeterministicResolution>,
    pub negotiated_resolutions: Vec<NegotiatedResolution>,
    pub rejected_intents: Vec<RejectedIntent>,         // can't graduate at all — reach denied, intent stale, etc., with reasons
    pub discarded_intents: Vec<DiscardedIntent>,       // not actionable from this lifecycle state — e.g. anonymous consent
    pub elohim_appraisal_notes: Option<AppraisalNotes>, // narrative from inference about what the batch represents and how the offer was constructed
}

/// A negotiated resolution names the proposed canonical form + the reasoning,
/// so the participant can refuse or counter-offer (where counter-offer is
/// supported by the pillar).
pub struct NegotiatedResolution {
    pub intent: StagedIntentEnvelope,
    pub proposed_canonical_entries: Vec<ProposedEntry>,
    pub appraised_value: ValueSummary,               // pillar-specific: standing increment, mutual-credit issuance, attestation strength, etc.
    pub reasoning: Option<String>,                   // narrative from elohim inference
    pub participant_alternatives: Vec<ProposedAlternative>, // where pillar supports it: alternative resolutions the participant could pick instead
}

/// Once the participant accepts (whole batch, or selected subset where
/// supported), the actual substrate writes happen and produce the manifest.
pub struct GraduationManifest {
    pub graduated: Vec<GraduatedIntent>,      // succeeded — list of canonical EntryHashes per pillar
    pub failed_mid_graduation: Vec<FailedIntent>,  // coordinator rejected at write time despite the offer
    pub session_terminated: bool,             // true if the session itself dissolved into membership; false for partial-graduation states
    pub appraisal_record: Option<EntryHash>,  // optional: notarize the appraisal itself for the participant's records
}
```

### Per-intent algorithm (offer phase)

For each staged intent in the pool:

1. **Predicate check** — does `intent.is_actionable(lifecycle)` return true? If no, mark as discarded with a reason ("anonymous consent is meaningless under protocol rules").

2. **Reach check** — does the graduating identity have the reach surface to author this entry in the target context? Sampling state has zero standing in the target; some intents need standing > 0 to graduate. If insufficient, mark rejected with the reach gap quoted.

3. **Classification** — deterministic or negotiated? Pillar-specific predicates (e.g. mastery is always deterministic for self-attestation; consent is always deterministic when actionable; reflection notes are always negotiated; sampling-derived signals are usually negotiated).

4. **Deterministic resolutions** — directly produce a `DeterministicResolution` with the canonical entry shape predicted.

5. **Negotiated resolutions** — invoke elohim inference with the intent + relevant target-context signals + (where consented) participant context. Produce a `NegotiatedResolution` with proposed entries, appraised value, reasoning, and any alternatives the participant can choose between.

6. **Compose offer** — assemble `GraduationOffer` for the participant to review.

### Participant decision phase

The participant inspects the offer. Options vary by pillar but generally include:

- **Accept whole batch** — graduate everything in the offer
- **Accept subset** — graduate selected resolutions; rest stay staged for later
- **Counter** (where supported) — pick one of the `participant_alternatives` for specific negotiated resolutions
- **Refuse** — walk away. Staged intent remains in the session for a later attempt, or session discard releases it

### Write phase

Once accepted, the substrate executes the writes. Failures here go into `failed_mid_graduation` (distinct from `rejected_intents` in the offer — those never made the offer; these were offered, accepted, then failed at write). Idempotency on coordinator side allows retry of the failed subset.

### Failure modes

- **Partial graduation** — some accepted resolutions succeed, some fail at write. The manifest carries both; the participant can retry the failed subset.
- **Offer staleness** — between offer construction and participant acceptance, the target-context state may have shifted (reach changed, standing recomputed, the pillar's rules updated). Offers carry an expiry; expired offers must be re-computed.
- **Identity mismatch** — graduating identity doesn't match the session's expected post-graduation identity. Hard fail; session not consumed.
- **Inference unavailable** — if elohim inference is unreachable for negotiated resolutions, the offer can still ship the deterministic part; negotiated intents are held back with a "needs appraisal" status. The participant can graduate deterministic now, return for negotiated later.

### Atomicity

The ceremony is NOT atomic. It's intentional. The Half-Price Books counter doesn't promise that every book transfers ownership simultaneously — they promise the receipt is true. The manifest is the receipt. This matches the protocol's eventual-consistency substrate.

---

## §5 — Doorway consumer wrapping (web2 visitor path)

Doorway consumes `session-bridge` for the visitor-graduation path. The wrapping pattern:

### Where state lives

Doorway's session pool — a SQLite table managed by doorway-service, NOT by elohim-storage's diesel. Doorway is the web2-facing surface and holds web2-shaped state.

### HTTP surfaces doorway exposes

```
POST /api/v1/visitor/session/open
  → { sessionId, expiresAt }

POST /api/v1/visitor/session/{sessionId}/oauth-promote
  body: OAuth credential
  → { sessionId, identity: oauth-shape, expiresAt }

POST /api/v1/visitor/session/{sessionId}/stage/{pillar}
  body: pillar-specific StagedIntent JSON
  → { stageReceipt }

GET  /api/v1/visitor/session/{sessionId}/staged-intent
  → { intents: [...] }

POST /api/v1/visitor/session/{sessionId}/graduate
  body: { newAgentPubKey, qahalContextHandle }
  → { graduationManifest }

DELETE /api/v1/visitor/session/{sessionId}
  → 204
```

### Caching of reach-gated content

Doorway already proxies reach-gated content reads via its projection cache. The session-bridge wrapping adds session-id'd authorization to those reads so that the visitor's expected content-reach is bounded by their lifecycle state.

### Browser-side wrapper

The browser doesn't need to know about the session-bridge primitive directly — it interacts with doorway HTTP routes. A thin Angular service `VisitorSessionService` wraps the HTTP calls and exposes a reactive stream of staged intent for UI binding.

---

## §6 — elohim-storage consumer wrapping (peer-native sampling path)

elohim-storage natively consumes `session-bridge` for the peer-native sampling path. Different shape:

### Where state lives — sampler-hosts-own-cache, confirmed

In the sampler's own elohim-storage diesel — a `sampling_session` table + a `sampling_staged_intent` table. The sampler hosts their own sampling state; the target context's substrate is not consulted until graduation. This is analogous to a browser holding its own cookies — the participant controls their own session state and can discard it at any time with no protocol-side coordination required.

**Important — this is NOT cookies-as-surveillance.** The browser-cookie shape is the right *functional* analogy (locally controlled, discardable, scoped to a session) but the *surveillance* shape that web2 grew around cookies — persistent cross-context tracking, opaque profile building, third-party access — is exactly what the session-bridge must not reproduce. Specific guardrails:

- **Ephemeral by default.** A sampling session that goes inactive for its expiry window discards itself. Persistence past the session requires explicit graduation; there is no "long-lived sampling profile" that accumulates across visits without the participant's deliberate consent + the substrate's canonical entry-creation.
- **No cross-context profiling.** The session-bridge never aggregates staged intent across different target contexts to build a behavioral profile. Each session is scoped to one target context; intent in session-A is not visible to session-B even on the same machine. Cross-context correlation requires the participant to graduate into a peer-native identity, at which point their canonical entries are subject to the existing reach/standing rules.
- **No third-party access.** Doorway holds web2-side session pools; other doorways do not have access to them. Sampler's elohim-storage holds peer-native sampling cache; other peers do not have access. The substrate primitives that share state across peers (DHT, libp2p protocols) are for canonical entries only, not for staged intent.
- **Always visible to the participant.** The bridge exposes a read interface ("show me everything you currently hold about me in this session") that returns the complete staged intent pool plus any cache slices. No opacity.
- **Discard is fail-closed.** Explicit discard clears the staged intent pool, the sampling cache, and any session metadata. The substrate retains nothing. (Per `project_forgetting_as_design` — deliberate forgetting is a first-class protocol move.)
- **Consent before profile.** Even within a single session, if a pillar wants to use accumulated intent to inform a negotiated graduation appraisal (the elohim-inference step in §4), the appraisal scope is bounded by the session itself, not by historical sessions. Cross-session intelligence requires the participant to have graduated.

The cookie metaphor names *who controls the storage and how it gets discarded*. It does not name *what the storage gets used for*. The session-bridge is on the participant's side, not the platform's side.

### Sampling cache

When a sampler opens a sampling session for `qahal-B`, elohim-storage:

1. Acquires `qahal-B`'s app manifest via the existing manifest substrate
2. RS-decodes the necessary projection slices the sampler will need to read (per the quilt vocabulary — `project_quilt_pantry_vocabulary`)
3. Holds them in a local pantry slot keyed to the sampling session
4. Expires the cache per the session's expiry policy

This costs the sampler local storage; it does NOT cost the host qahal anything. The host doesn't know they're being sampled until the sampler graduates (becomes a member) — sampling is invisible from the host's perspective by design.

### Coordinator surfaces

elohim-storage exposes Rust-native `SessionBridge` trait method calls; no HTTP surface needed (Tauri context calls them directly through the existing storage-client SDK).

### Graduation in this path

When a sampler decides to join `qahal-B`:

1. Their elohim-storage calls `bridge.graduate(samplingSession, samplingIdentity)`
2. The graduation ceremony replays each staged intent through `qahal-B`'s coordinators — which the sampler doesn't have membership in yet, so the coordinators are called via libp2p as "membership-application + initial-state-bundle" requests
3. `qahal-B`'s coordinators validate the application, accept the staged intent as the new member's starting state, and respond with a membership-acceptance manifest
4. The sampler is now a member; the sampling cache discards (or transitions to a member-cache) and the session's intent pool is consumed

This pattern uses the existing protocol surfaces (libp2p coordinator calls, qahal membership flow) — the bridge is the orchestrator, not a new substrate primitive.

---

## §7 — Relationship to existing primitives

### `bridges/` (atproto, activitypub, valueflows)

Conventional bridges translate between two adjacent canonical substrates — peer-native EPR-REA ↔ peer-native AT Protocol, etc. The session-bridge is structurally distinct: it translates between *tentative* and *canonical* of the **same** substrate. That's why it lives at `crates/session-bridge/` rather than `bridges/session/`.

### `qahal` graduated capability surface (`project_qahal_graduated_capability_surface`)

The qahal pillar already has a graduated capability model — reach gates outward, standing gates inward. The session-bridge slots in BELOW the lowest qahal-membership tier as a kind of "pre-tier" — sampling participants have zero standing in the target context but still have a defined participation surface. After graduation, they enter the qahal's standard graduated-capability gradient.

### Reach gating

Sampling-state and oauth-identified-state participants are read-included for reach-gated content per the sampling cache, but their writes don't carry into the host substrate until graduation. This is consistent with the existing reach-gating model — they read what they're permitted to read; their writes are staged-but-not-canonical.

### AttentionTending and other agent-authored substrate moves

Things like `AttentionTending`, `FeedbackSignal`, `Commitment`, `GovernanceState` — these are agent-authored canonical entries that require signing keys and accrued standing. They are NOT staged-intent shapes. A visitor or sampler cannot AttentionTend; that's a network-citizen authoring move (see operator framing 2026-05-28: "this is intrusive clickbait, allowing users or stewards to express a value against something with intrusive virality"). The session-bridge holds intentions that *might one day* author entries; AttentionTending is what an authorized agent *does* author. Different layer.

### `bridges/atproto` two-graduation pattern (`project_account_layer_oauth_graduation`)

The existing memory note about doorway-as-OAuth-RP names "OAuth graduation" — the moment a web2-OAuth identity becomes peer-native. That memory note frames it as a one-step graduation. This spec refines it as a two-step graduation: anonymous → oauth-identified → peer-native, with the session-bridge holding staged intent across both transitions.

### M-REA-1's `LamadEventIntent` surface

The session-bridge's `StagedEconomicEventIntent` is intentionally identical to M-REA-1's `LamadEventIntent`. The bridge holds intents pre-incarnation; M-REA-1's coordinator composes the canonical EconomicEvent post-incarnation. Same intent shape, different lifecycle stage. The bridge doesn't replicate the substrate composition logic — it defers to M-REA-1's coordinator at graduation time.

---

## §8 — Open design questions

### Closed by operator framing 2026-05-28

The following questions had explicit answers in design discussion. Recorded here so the closure is visible alongside the new questions the answers surfaced.

**[CLOSED] Storage location for the sampling cache.** Sampler hosts their own cache, browser-cookie shape (locally controlled, discardable, scoped to a session) but explicitly NOT browser-cookie surveillance-shape. Detailed guardrails in §6.

**[CLOSED] Staged intent surviving identity-graduation.** The graduation ceremony handles this — the batched intent is appraised holistically at graduation time, with deterministic intents resolving 1:1 and negotiated intents going through elohim inference. The Half-Price Books metaphor is canonical: you bring the batch, the substrate appraises and offers, you accept or walk away. Surviving intent doesn't survive *passively* across graduation; it survives by going through the ceremony and being explicitly accepted into canonical form.

### Still open

1. **Sampling cache expiry policy.** Time-based default? Storage-pressure-based with LRU? Per-app-manifest-defined? Probably a small default (e.g. 7 days, but worth tighter for anonymous browser state to reinforce the not-surveillance posture) with explicit "stop sampling" + storage-pressure eviction.

2. **What writes are allowed during sampling.** Sampling state has zero standing, so by default no canonical substrate writes. The session-bridge stages intent locally, but is there a category of zero-standing-permissible local writes that don't even need staging (e.g. ephemeral UI preferences for the sampling app)? Worth a per-pillar matrix that maps "during sampling, this kind of write goes to: never / session-bridge intent pool / sampling-local cache / requires-graduation-first".

3. **Graduation-ceremony rollback semantics.** Non-atomic by design. But what if a graduation-in-progress is interrupted between offer-accept and substrate-write (network failure)? The `failed_mid_graduation` list lets the participant retry the subset, but the participant's mental model of "I accepted that offer" doesn't match "some of it didn't actually land." UX implication: the participant needs visibility into write-state after acceptance. Spec should mandate a "graduation in progress" status surface the participant can poll.

4. **OAuth identity persistence vs anonymity preservation.** Some participants will want to OAuth-identify (to access reach-gated content that requires identification) without persisting that identity post-session. Doorway should support an ephemeral-oauth mode where the OAuth subject is held only in-memory and discarded with the session, not persisted to doorway's session pool. This reinforces the not-surveillance posture for participants who want to dip in without committing.

5. **Sampling between collectives at different scales.** A sampler from a household-scale qahal visiting a regional-scale commons may have to sample more aggressively (more cache, more state) than a sampler at the same scale. Worth thinking about whether sampling-cache size scales with target-context size or stays bounded.

6. **Sampling as a federation primitive.** Could the same shape be used between federated commons (not just sampling-then-joining, but ongoing inter-commons participation that's never expected to graduate)? Probably yes — long-running sampling sessions become a federation pattern. Worth noting but deferring detailed design until use case clarifies.

### New — opened by the appraisal/negotiation framing in §4

7. **Which elohim is the appraiser?** Negotiated-resolution intents invoke "elohim inference" — but which elohim? The participant's home-elohim (familiar with their context, has their consented profile)? The target context's commons-elohim (per `project_commons_elohim_co_steward` — speaks for the collective interest)? A neutral third elohim acting as appraisal counsel (per `project_elohim_as_counsel`)? Possibly all three, with different roles: home-elohim represents the participant's interests, commons-elohim represents the host's interests, neutral elohim mediates and produces the offer. This is the substrate-correct shape but it's expensive — needs design about when the three-elohim ceremony is necessary vs when a simpler single-elohim appraisal suffices.

8. **Appraisal authority and reproducibility.** If elohim inference produces a negotiated resolution, can the participant audit the reasoning? Re-run the appraisal with a different elohim and compare? Notarize the original appraisal as part of their graduation record? `GraduationOffer::elohim_appraisal_notes` and `GraduationManifest::appraisal_record` gesture at this but the substrate semantics need design. (Related: `project_elohim_councils_capture_apex` — wisdom holds the structural top of authority; appraisal is one of the moves wisdom makes.)

9. **What does refusal cost?** A participant who refuses the graduation offer at the Half-Price Books counter walks out with their stack. In the protocol, this means: staged intent stays in the session, the session continues until expiry, the participant can later re-attempt with a new offer (presumably with shifted target-context state that might yield a different appraisal). But there's a risk of "shopping the offer" — repeatedly re-graduating to extract more favorable appraisals. The substrate should probably encode some friction or memory of prior offers to prevent appraisal-shopping while preserving the participant's right to refuse-and-return.

10. **What's the negotiation primitive on the participant side?** The offer presents `participant_alternatives` for negotiated resolutions. What's the substrate semantics for the participant proposing a *novel* alternative (not on the menu)? Is that "request a re-appraisal with this counter-proposal," or "this offer is rejected; here's a new staged intent to add to the pool"? Probably the latter — keep the protocol simple by treating counter-offers as new staged intents that go through the same offer-construction cycle.

11. **Avoiding the cookie-shape surveillance trap concretely.** The §6 guardrails name the principles but need substrate-level enforcement, not just convention. How does the protocol verify a doorway is honoring "no cross-context profiling"? Probably needs auditable session-pool boundaries + observable behavior the participant can verify (e.g. "show me everything you hold about me" must be a complete answer, not a filtered one). Worth a section in the doorway spec.

---

## §9 — Migration path: task #14 and the 4 deferred consumers

`LocalSourceChainService` has 4 deferred write-path consumers from M-AGGR-2:
- `path-negotiation.service.ts` — committing to a path version at a step
- `content-mastery.service.ts` — recording mastery attainment
- `mastery-stats.service.ts` — derived mastery aggregations
- `human-consent.service.ts` — consent decisions

Each of these is a candidate for becoming a session-bridge consumer rather than a direct substrate writer. Under this design:

- `human-consent.service.ts` calls `bridge.stage(StagedConsentIntent { ... })` during anonymous/oauth phase; the consent graduates at incarnation time when the agent's source chain becomes addressable
- `content-mastery.service.ts` calls `bridge.stage(StagedMasteryIntent { ... })` during all states except member-of-target-context (where it writes directly to the agent's chain)
- `path-negotiation.service.ts` similar — stage during pre-member states, write directly when member
- `mastery-stats.service.ts` reads from the session-bridge intent pool when no member-context exists; reads from canonical projections when in member-context

The four consumers become "bridge-aware" — they check the session-bridge lifecycle state and route writes accordingly. The session-bridge handles the eventual replay into the canonical substrate at graduation.

This is the implementation form of task #14. It requires the session-bridge crate to land first, then the four consumers migrate to bridge-aware writes.

### Suggested ticket sequencing for the post-sprint work

1. **Land the `crates/session-bridge/` skeleton** — types, traits, no consumers yet. Schema for the lifecycle states + staged intent envelopes.
2. **Doorway consumer wrapping** — HTTP surfaces for the visitor path. Browser-side `VisitorSessionService`.
3. **Per-pillar staged-intent shapes** — define `StagedMasteryIntent`, `StagedConsentIntent`, etc. in their pillars. Implement `GraduationCeremony` for each.
4. **Migrate the 4 deferred LocalSourceChainService consumers** to bridge-aware writes.
5. **Delete `LocalSourceChainService`.**
6. **elohim-storage native wrapping** — sampling-session SQL + Rust trait implementations.
7. **Peer-native sampling handshake** — libp2p protocol for sampling-cache acquisition + graduation request.

(1)–(5) is the visitor-graduation path and unblocks the long-promised deletion. (6)–(7) is the peer-sampling path and is genuinely new substrate work.

---

## §10 — References

- Plan that surfaced this: `genesis/docs/superpowers/plans/2026-05-28-thin-client-backend-migration.md` — particularly §3 Ticket M-AGGR-2 and the deferred-deletion note in the M-AGGR-2 close-out
- `project_account_layer_oauth_graduation` (memory) — doorway as OAuth RP; post-graduation loses identity agency
- `project_peer_native_account_canonical_surface` (memory) — peer-native steward via account management
- `project_recovery_grandma_standard` (memory) — heavy account ceremony, ambient onboarding
- `project_qahal_graduated_capability_surface` (memory) — graduated capability shape this primitive slots beneath
- `project_doorway_full_facilitator_sprint` (memory) — doorway as the web2-facing surface
- `project_doorway_is_federation_surface_atproto` (memory) — bridge pattern at doorway
- `project_imagodei_three_surfaces` (memory) — identity surface decomposition (social profile / self-knowledge / account management)
- `project_socially_derived_security` (memory) — recovery model that makes account incarnation heavy by design
- M-REA-1 commits (intent surface that this primitive holds pre-commitment) — `aece1093c` → `8cc0b759f`
- M-AGGR-2 commits (read-side cutover, deferred write-side) — `6e184ef96` → `6c1fde7bc`
- Operator framing for AttentionTending vs visitor concern, 2026-05-28 conversation

---

## §11 — What this spec is NOT

- A code ticket. This is the design that informs ticket #14 and its successors.
- A finalized API. The trait shapes in §2 are draft-quality; real implementation will surface edge cases that reshape them.
- A storage decision. §6 proposes the sampler's elohim-storage holds sampling cache, but §8 leaves that for explicit confirmation.
- A federation spec. §8 question 8 notes federation could emerge from this primitive, but that's a separate design pass.
- A web2 onboarding-flow UX spec. This addresses the substrate layer; the UX layer for "what does a visitor experience" needs its own design document, informed by this primitive's contract.
