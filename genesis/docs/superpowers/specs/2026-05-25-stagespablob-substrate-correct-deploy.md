# Z.D — Substrate-correct deploy via reciprocal REA compute commitments

**Status:** design (2026-05-25)
**Author:** elohim collective
**Scope:** two-fold — (1) names the REA compute-commitment primitive (gospel-tier; instantiable across the protocol); (2) instantiates it for Z.D: the deploy-time `EprHead` republish path, replacing the Z.1 anti-pattern (`PATCH /db/content/{slug}`).
**Surfaced by:** the `2026-05-24` shakeout. App pipeline #1464 mechanically succeeded (blob uploaded, content rows PATCHed, all verifies green) yet `alpha.elohim.host` served stale content because the DHT was never told the blob changed. Documented in `2026-05-23-doorway-access-tier-patterns.md` as Anti-pattern Z.1.

---

## §0 — Canon References

This spec depends on three foundational canon documents in `genesis/docs/architecture/`. Read them first:

- [stewardship-over-sovereignty](epr:stewardship-over-sovereignty) — the philosophical anchor. Explains why "hold your own keys" is not the protocol's notion of sovereignty, why every authority surface needs a non-cryptographic recovery path, why anonymous publish has no place in this substrate. Z.D's open-question answers (§7) flow from this canon.
- [rea-compute-commitment-primitive](epr:rea-compute-commitment-primitive) — the substrate primitive. Z.D is the first concrete instance (deploy authority). The §1 of this spec lifts the primitive's shape; the canon doc holds the gospel-tier articulation and the generalization table.
- [cradle-to-grave-capability-gradient](epr:cradle-to-grave-capability-gradient) — how the primitive instantiates across life-stage capacities. Z.D's compromise-recovery story (§7 Q5) is the protocol-level instance row of that gradient.

Implementation plan (in-tree): `genesis/docs/superpowers/plans/` will host the bite-sized task decomposition for Z.D Phase 1+2 implementation after this spec lands.

---

## Why this is two specs in one

The user named the lesson plainly: *"these reciprocal REA compute agreements are foundational to the entire elohim protocol — we have to master these."*

The deploy-authority question (who signs the `EprHead` republish?) is one **instance** of a recurring shape that appears wherever **agent X commits compute, work, or standing to agent Y under bounded conditions** — deploy, hosting, household chore stewardship, qahal moderation, content authorship delegation, compute lending. Solving Z.D in isolation produces a one-off. Solving Z.D as an instance of the **REA compute-commitment primitive** produces a pattern future spec authors can copy without re-deriving.

§1 specs the primitive. §2 specs Z.D. §3 onward handles consequences and sequencing.

---

## §1 — The REA compute-commitment primitive

### Shape

A reciprocal REA compute commitment is a **Mishpat `Commitment` entry** (existing DHT entry type; new action discriminator `delegates-compute`) between a **provider agent** and a **recipient agent**, scoped to a class of **`EconomicEvent`s** the recipient is authorized to emit, bounded by enforceable conditions, with reciprocal obligations on both sides.

```
                                          Commitment
                                          ┌─────────────────────────────┐
                                          │ action: "delegates-compute" │
                                          │   scope: "<event-class>"    │
                                          │                             │
       ┌──────────────┐                   │ provider:  agent X          │   ┌──────────────────────┐
       │  Provider X  │ ────signed by──▶  │ recipient: agent Y          │ ◀─│    Recipient Y       │
       └──────────────┘                   │                             │   └──────────────────────┘
            ▲                             │ bounds: { … }               │            │
            │                             │ reciprocity: { … }          │            │
            │                             │ ttl: <rotatable>            │            │
            │                             └─────────────────────────────┘            │
            │                                                                        │
            │                                                                        ▼
            │                                                       EconomicEvent
            │                                                       ┌───────────────────────────┐
            │                                                       │ action: <within scope>    │
            │                                                       │                           │
            │                                                       │ performer: Y              │
            │                                                       │ target:    <outcome CID>  │
            │                                                       │                           │
            └──────────  back-ref  ── bounded_by: <Commitment CID> ◀│ (proves standing)         │
                                                                    └───────────────────────────┘
```

### Reciprocity is real, not decorative

Every compute commitment names two-way obligation. If either side defaults, a `FeedbackSignal` accrues on-chain — the protocol witnesses the breach. This is what distinguishes a reciprocal commitment from a unilateral grant (e.g., an X-API-Key, which has no return obligation and no on-chain accountability).

| Provider obligation                                  | Recipient obligation                                              |
| ---------------------------------------------------- | ----------------------------------------------------------------- |
| Holds custody of the bounding Commitment             | Signs every in-scope `EconomicEvent` with its own key             |
| Acknowledges scope/reach escalations (soft-warn)     | References `bounded_by: <Commitment CID>` in every event          |
| Rotates the recipient's key on schedule              | Stays within bounds; substrate enforces, agent helps              |
| Revokes promptly if compromise/misbehavior detected  | Refuses to sign outside bounds; emits feedback when it must defer |

### Auditability properties

This shape gives the substrate four properties that an X-API-Key bypass cannot:

1. **Standing is checkable.** Given any `EconomicEvent`, walk back through `bounded_by` to the Commitment. Verify it's still active, signed by a provider with standing in the relevant scope, and that the event falls within bounds. Yes/no answer from the DHT.
2. **Revocation is real.** If the provider revokes the Commitment, subsequent events from the recipient referencing it fail validation. No "rotate the API key everywhere" scramble.
3. **The authority chain is itself notarized.** Every link from "operator owns this resource" to "this CI agent may republish this EPR" is a chain of DHT-witnessable Commitments. No off-chain trust.
4. **Reciprocity is observable.** The provider's acknowledgements (e.g., for soft-warn ceremonies) are themselves DHT-resident. Default = silence; default emerges as a FeedbackSignal pattern. The protocol can witness chronic non-reciprocation.

### Generalization — where this primitive recurs

The same `delegates-compute` shape, with different scopes and bounds, models nearly every act of bounded authority delegation in the protocol. Future spec authors should copy this pattern rather than re-derive it.

| Instance                                  | Provider                | Recipient                  | Event class                      | Bounds (examples)                                  |
| ----------------------------------------- | ----------------------- | -------------------------- | -------------------------------- | -------------------------------------------------- |
| **Deploy (Z.D, this spec)**               | operator steward        | deploy-svc-agent           | `republish-epr`                  | reach ceiling, rate/hr, EPR scope, key rotation TTL |
| **Hosting projection**                    | doorway operator        | doorway-svc-agent          | `serve-url-projection`           | doorway capacity, reach gates, URL-prefix scope    |
| **Household chore stewardship**           | household member        | another member             | `chore-done`                     | scope (kitchen/yard), period (week), chore type    |
| **Qahal moderation**                      | qahal collective        | moderator-agent            | `moderation-action`              | qahal scope, action types, target reach class      |
| **Content authorship delegation**         | original author         | co-steward                 | `publish-revision`               | content CID lineage, branch policy, scope          |
| **Compute lending (DePIN, future)**       | node operator           | requesting peer            | `provide-cycles`                 | watts, wall-time, task class                       |
| **Recovery delegation (graduated)**       | steward (pre-incident)  | recovery quorum            | `attest-recovery`                | reach ceiling, quorum threshold, time window       |

Z.D is the **first concrete instance**. The §2 detail below is the template the others inherit.

---

## §2 — Z.D: deploy authority as the first instance

### Roles

| Role          | Identity                                                                                                         | Why                                                                                                                                                                                                                                                          |
| ------------- | ---------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Provider**  | Operator's steward agent (the human running the doorway operation — `agent:matthew-steward`, etc.)               | The operator owns the resource being deployed. They have the substrate authority to delegate compute to CI without putting their own key in CI.                                                                                                              |
| **Recipient** | A per-operator deploy-service-agent (`agent:deploy-service-matthew-doorway`, etc.) — Ed25519 keypair held by CI | Named, durable, attestable. Rotatable on a schedule. Survives operator handover (issue a new Commitment from the next steward; revoke the old). The agent itself has no human standing — it can only act inside its bounds. |

### Bounds (the Commitment payload)

```json
{
  "action": "delegates-compute",
  "scope": "republish-epr",
  "provider": "agent:matthew-steward",
  "recipient": "agent:deploy-service-matthew-doorway",
  "bounds": {
    "epr_scope": ["epr:lamad-spa", "epr:elohim-host-landing"],
    "reach_ceiling": "commons",
    "rate_per_hour": 30,
    "rotation_ttl_days": 90
  },
  "reciprocity": {
    "provider_obligations": [
      "key-custody",
      "soft-warn-acknowledgement",
      "scheduled-rotation",
      "revocation-on-compromise"
    ],
    "recipient_obligations": [
      "bounded-event-signing",
      "back-reference-commitment",
      "reach-ceiling-enforcement",
      "rotation-compliance"
    ]
  },
  "valid_from": "2026-05-25T00:00:00Z",
  "valid_until": "2026-08-23T00:00:00Z"
}
```

`reach_ceiling: "commons"` is the critical bound: the deploy-svc-agent may publish republished `EprHead`s that carry `reach=commons` or **lower-reach values that the operator has separately delegated**, but cannot escalate to `private/intimate/trusted/familiar/community/public` without a fresh provider-signed delegation. This is the substrate's hedge against a compromised CI silently elevating private content into the commons.

### The deploy event itself

Every CI republish emits one `EconomicEvent`:

```json
{
  "action": "republish-epr",
  "performer": "agent:deploy-service-matthew-doorway",
  "bounded_by": "<delegates-compute Commitment CID>",
  "target": "<new EprHead CID>",
  "supersedes": "<previous EprHead CID, or null for first publish>",
  "payload": {
    "blob_cid": "<bafkrei...>",
    "epr_kind": "Content",
    "reach": "commons",
    "bundle_path": "lamad-spa"
  },
  "signed_at": "2026-05-25T07:41:14Z"
}
```

Validation walks `bounded_by` → Commitment, checks bounds against the event payload (`reach <= reach_ceiling`, `epr_scope` includes the bundle, rate-limit check from on-chain event history), and accepts/rejects.

### CI flow (the stageSpaBlob migration)

The current `stageSpaBlob` calls `PUT /admin/seed/blob` (upload bytes) + `PATCH /db/content/{id}` (mutate the slug→hash row). Z.D replaces it with:

```
stageSpaBlob (Z.D shape):
  1. cd "$distDir"; tar bytes → CID compute (blake3 → bafkrei… match existing iroh hashing)
  2. Upload bytes:    PUT /blob/{cid}                  (content-addressed; no slug)
  3. Construct EprHead envelope:
       kind:        Content
       reach:       <from manifest; commons for SPA bundles>
       coupling:    { knowledge: <project-epr commitment CID> }
       payload.cid: <new blob CID from step 1>
       supersedes:  <previous EprHead CID — fetch from doorway>
       proof.signer: agent:deploy-service-…  (loaded from CI secret)
  4. Sign envelope with deploy-svc-agent key (Ed25519)
  5. Compute EprHead CID over the canonical-encoded envelope
  6. Emit EconomicEvent  (action=republish-epr, bounded_by=<commitment>)
  7. PUT /api/v1/epr/{cid}  with the envelope + the event
       → server validates  (path CID == envelope CID; bounded_by valid; bounds satisfied)
       → FederatedEprStore::put → diesel + KadStartProviding (P2P announce)
       → graph projection → SUPERSEDES edge written
       → emits  projection signal: epr.republished { cid, supersedes, performer }
  8. Done — no PATCH needed. The doorway picks it up via §2.5.
```

The bridge from today's CI to Z.D is mechanical glue. The substrate already supports it (the route exists at `elohim/elohim-storage/src/api/epr.rs:484`). What needs to be built:

| Component                              | Where                                                                       | Effort  |
| -------------------------------------- | --------------------------------------------------------------------------- | ------- |
| Deploy-svc-agent provisioning script   | `genesis/orchestrator/scripts/provision-deploy-agent.{ts,sh}` (new)         | small   |
| `delegates-compute` action wiring      | `elohim/holochain/dna/mishpat/zomes/coordinator/commitments/src/lib.rs`     | small   |
| Bound validator                        | `elohim/elohim-storage/src/api/epr.rs::put_epr` — extends existing validation | small   |
| `republish-epr` `EconomicEvent` schema | `elohim/sdk/schemas/v1/economic-events/republish-epr.schema.json` (new)     | small   |
| CI envelope-construction + signing     | `genesis/orchestrator/scripts/stagespablob-z-d.ts` (new); used from Jenkinsfile | medium  |
| stageSpaBlob Jenkinsfile rewrite       | replace PATCH/admin-key path with z-d.ts call                               | small   |
| Doorway-side subscriber extension      | §2.5 below                                                                  | medium  |
| `projection_events` projection refresh | doorway subscriber emits projection-store invalidation                      | small   |

### §2.5 — Doorway-side response: `epr.republished` event

Today's Pattern Z bridge subscribes to `content.{created,updated,deleted}` (from the SQLite PATCH) and `projection.{registered,revoked}` (from B15). Z.D adds a third event kind:

```
event: epr.republished
data:  { "cid": "<new EprHead CID>", "supersedes": "<prev>", "performer": "<agent>" }
```

The doorway subscriber's `handle_event` gains a fourth arm:

```rust
"epr.republished" => {
    // 1. Fetch the new envelope: GET /api/v1/epr/{cid}/envelope
    // 2. Check the projection contract: is there a project-epr Commitment
    //    where coupling.knowledge points at this EPR's identity? If not, skip.
    // 3. Reach evaluation:
    //    - If new envelope.reach == projected reach → just refresh caches
    //    - If new envelope.reach != projected reach → SOFT-WARN ceremony (§3)
    // 4. Invalidate app_file_cache slug
    // 5. Invalidate projected_entries MongoDB row for this bundle path
    // 6. Next /lamad/* request resolves through the fresh chain
}
```

When this lands, the `content.{created,updated,deleted}` arm becomes redundant for any caller that's migrated to Z.D. It stays alive only while the Pattern Z bridge serves the un-migrated PATCH callers (avodah API, UI inline edits) — see §4 for the deprecation path.

---

## §3 — Reach change handling: the soft-warn ceremony

When a republished `EprHead` carries a different `reach` value than the doorway's active `project-epr` Commitment expects, the doorway **pauses serving that projection** and emits a `reach-escalation-pending` event. The operator's steward must explicitly publish an `Acknowledgement` Commitment before serving resumes.

### Why soft-warn, not auto-comply

Auto-comply enables silent reach escalation: a compromised deploy-svc-agent (or a buggy CI step) could publish a new `EprHead` with `reach=private` and the doorway would happily host private content under what visitors expect to be a public URL. Soft-warn forces a steward in the loop for any reach change.

### Why soft-warn, not hard-fail

Hard-fail (auto-revoke the projection commitment) means the operator must re-register every time they intentionally evolve a bundle's reach. This is operationally hostile for routine reach evolution (e.g., a course module moves from `private` → `community` after launch). Soft-warn preserves the projection commitment but holds serving until acknowledged.

### Ceremony

```
T0: deploy-svc-agent publishes new EprHead, reach=community
    (previous EprHead had reach=commons; project-epr commitment was registered against commons)

T1: Doorway receives epr.republished signal
    Doorway compares envelope.reach (=community) vs project-epr.reach (=commons)
    Mismatch → doorway STOPS serving this projection (returns 503 with reach-escalation marker)
    Doorway emits feedback: { kind: "reach-escalation-pending", target: <EprHead CID> }

T2: Operator's steward UI surfaces the pending escalation
    Steward reviews: is this intentional? (CI did the right thing? or compromise?)
    
T3a (intentional): Steward publishes Acknowledgement Commitment:
       action: "acknowledges-reach-change"
       target: <EprHead CID>
       new_reach: community
     Doorway receives projection signal → resumes serving with the new reach checked at request time

T3b (unintentional): Steward revokes the delegates-compute Commitment
     Substrate refuses subsequent republishes from that agent
     Doorway resumes serving the previous (un-superseded) EprHead until a corrective republish lands
```

The ceremony's cost is one stewardship-acknowledgement-per-reach-change. The cost of skipping it is silent reach escalation. The trade is correct.

### Where this lives in code

| Surface                                | File                                                                              |
| -------------------------------------- | --------------------------------------------------------------------------------- |
| Reach-comparison logic                 | `doorway/doorway-service/src/projection/reach_evaluator.rs` (new)                 |
| `acknowledges-reach-change` action     | `elohim/holochain/dna/mishpat/zomes/coordinator/commitments/src/lib.rs`           |
| Steward-facing UI surface              | `app/elohim-app/src/app/imagodei/components/reach-escalation-review/…` (Phase B)  |
| FeedbackSignal kind                    | `elohim/sdk/schemas/v1/feedback-signals/reach-escalation-pending.schema.json` (new) |

---

## §4 — Z.E sequencing

Per Pattern Z lines 199–201: Z.B (bridge) + Z.D (substrate-correct) must both exist before Z.E (delete `PATCH /db/content/{slug}`), because callers other than `stageSpaBlob` still use PATCH. Z.E's pre-conditions:

1. **Z.D ships** (this spec): `stageSpaBlob` migrated to substrate-correct deploy.
2. **All other PATCH callers audited**: avodah API, UI inline edits, content authoring tools. Each one migrates to either `PUT /api/v1/epr/{cid}` (if it's a republish) or to a substrate-correct alternative (if it's something else — e.g., content drafts may belong as agent-scoped private chain entries until publication).
3. **The Pattern Z bridge subscriber gets simpler**: the `content.{created,updated,deleted}` arm of the doorway subscriber can be removed when no migrated caller emits those events.
4. **The PATCH route is removed**: `elohim/elohim-storage/src/http.rs::3810` registration deletes. `route_registry.rs::forward_to_storage` keeps GET/POST/PUT/DELETE/HEAD; loses PATCH.
5. **`projection_events` invariant**: every projection refresh originates from a `republish-epr` `EconomicEvent` referencing a valid `delegates-compute` Commitment. No path bypasses standing.

### Z.E scope (when it ships)

Z.E is **not part of this spec**. This spec only sequences it. The Z.E spec is created after the PATCH-caller audit completes. Until then, the Pattern Z bridge serves the un-migrated callers and we live with the bridge debt explicitly.

---

## §5 — Future extensions

These are **not part of Z.D's scope** but the spec names them so they're not lost.

### Low-trust signals on republish events

The user's framing: *"there might be a no-trust signal that goes with such artifacts so that future elohim-native developers can develop things, and model the reach, but that's maybe future to this work."*

Model: a republish carries an optional adjacent `FeedbackSignal` of kind `republish-source-trust` declared by the deploy-svc-agent itself (self-flagging) or by external observers (peer attestation). The doorway's reach evaluator consults active signals before deciding soft-warn vs. auto-comply:

| Signal aggregate                    | Reach-change handling                                                |
| ----------------------------------- | -------------------------------------------------------------------- |
| No signals OR positive-trust only   | Auto-comply for same-class moves; soft-warn for class escalation     |
| Mixed / low-trust present           | Soft-warn for **all** reach changes, regardless of direction         |
| Heavy negative-trust accumulation   | Hard-pause projection; require steward re-registration               |

This is consistent with `project_signal_kind_extensible_protocol_class` — new signal_kinds added via schema + validator + manifest, no new entry types. The Z.D path doesn't need to know about these signals; the doorway's reach evaluator does.

### AI-mature deployment

When agentic developers (elohim-native AI agents) deploy their own EPRs, they hold their own deploy-svc-agent identities, granted by their own steward (often a human collaborator or a senior elohim agent). The reciprocity model holds: the AI agent's standing is bounded by the Commitment, the human/senior in the loop holds custody and revocation. The protocol doesn't need new primitives — it needs the AI agents to participate honestly in the existing one.

This is the "AI deployment in the network is mature" framing the user named. Z.D's substrate prepares for it; the AI side prepares itself by adopting the primitive correctly.

### REA event recovery & reconciliation

If a deploy-svc-agent emits an event the substrate later rejects (e.g., bounds drifted, Commitment was revoked between event sign and receipt), what's the recovery flow? Open. Likely a `republish-rejected` FeedbackSignal back to the operator's UI, plus a re-emit-with-fresh-bounds-cycle on the CI side. Out of scope for Z.D; tracked here so the next sprint can pick it up.

---

## §6 — Acceptance signals

Z.D ships when:

1. **A deploy-svc-agent is provisioned** for at least one operator (e.g., `agent:deploy-service-matthew-doorway`). The provisioning script lives in `genesis/orchestrator/scripts/`.
2. **A `delegates-compute` Commitment exists** on the DHT signed by that operator's steward, with the deploy-svc-agent as recipient and bounds as described in §2.
3. **`stageSpaBlob` runs the Z.D shape**: signs an envelope, emits an `EconomicEvent`, calls `PUT /api/v1/epr/{cid}`. Returns success. Verified via Jenkins job artifact.
4. **The doorway subscriber handles `epr.republished`** and refreshes both `app_file_cache` and `projected_entries`. Verified via integration test: deploy a new bundle, observe next browser request returns fresh bytes without any PATCH involvement.
5. **A bounds-violation deploy is rejected**: a CI job that attempts to publish reach=`community` against a `reach_ceiling=commons` Commitment is refused by the storage validator. Verified via integration test.
6. **The Pattern Z bridge continues working** for un-migrated PATCH callers. Verified via existing Pattern Z tests (no regression).
7. **A2o scenarios exist** under `genesis/a2o/features/doorway/`:
   - `substrate-correct-deploy.feature` — happy-path Z.D
   - `bounds-violation-rejection.feature` — substrate refuses out-of-bounds republish
   - `reach-escalation-soft-warn.feature` — §3 ceremony
8. **B15's `projection.{registered,revoked}` handling is unchanged** — Z.D is additive, not destructive to existing EPR routing work.

---

## §7 — Resolved design questions

The five questions that landed in design discussion were resolved against the canon at [stewardship-over-sovereignty](epr:stewardship-over-sovereignty) and [cradle-to-grave-capability-gradient](epr:cradle-to-grave-capability-gradient). Each answer is anchored in protocol principles rather than crypto-libertarian defaults.

### §7.1 — Deploy-svc-agent key custody

**Resolution:** The substrate is indifferent to where the key lives; what matters is the **steward-stewardee relationship that grants standing**. The key is the cryptographic enactment of the operator-steward's `delegates-compute` Commitment, not a sovereignty primitive.

- **Substrate-minimum:** the key MUST live in a witnessable secret store (Vault, AWS/GCP KMS, OS keyring, Kubernetes Secret, paper backup in a safe — operator's choice). A plaintext repo file emits a `bad-custody` FeedbackSignal (declarable by any steward observer; the operator's elohim raises it on detection).
- **Recovery:** if the key is lost, the steward revokes the Commitment and issues a new one to a freshly-provisioned deploy-svc-agent. No cryptographic recovery primitive required — the social-relational primitive (steward attestation) covers it. Aligns with `project_socially_derived_security`: crypto accelerates, never gates.
- **Custodian-as-counsel:** the operator's elohim-agent watches for anomalous behavior from the deploy-svc-agent and can recommend or initiate revocation per `project_elohim_as_counsel`.

### §7.2 — CID algorithm choice

**Resolution:** Use the existing `compute_cid` helper at `elohim/epr/src/cid.rs:11-15` — **SHA2-256 multihash + dag-cbor codec (0x71)**. This is the canonical CID format the entire EPR substrate already uses (`FederatedEprStore`, `KadStartProviding` keys, graph projection). Blake3 is iroh's content-addressing layer; the EPR envelope uses dag-cbor canonical sha256 and the two layers cooperate without conflict.

### §7.3 — Rate-limit enforcement

**Resolution:** **Both, with different roles.**

- **Storage validator (cheap, no DHT round-trip):** sliding-window query against the local `economic_events` SQLite table for events with this `bounded_by` Commitment in the last `rate_per_hour` window. Rejects with HTTP 429 if over limit. This is the hot-path enforcement.
- **Mishpat zome (slow, DHT-resident):** witness emits a `rate-limit-exceeded` `FeedbackSignal` (DHT entry) per breach attempt. This is the audit truth — observable by all peers, persists across restarts, accrues against the deploy-svc-agent's reputation per `project_signal_kind_extensible_protocol_class`.

Storage rejection is the operational behavior; the FeedbackSignal is the protocol-witnessed truth.

### §7.4 — First-publish (`supersedes: null`) authority

**Resolution:** **Always require a `bounded_by` reference, even for `supersedes: null`** — anonymous publishing has no place in this substrate. Aligns with the sovereignty inversion: no standing without relationship.

- For an operator's first deploy of a new EPR, the operator's existing `delegates-compute` Commitment to the deploy-svc-agent must have `epr_scope: ["*"]` or include the new EPR's identity.
- For a freshly-provisioned operator (first-ever deploy), bootstrap is done by the operator-steward themselves publishing the first `delegates-compute` Commitment — they hold standing as a human steward of the doorway resource. No protocol-level "anonymous publish" gate exists.
- Reach-bound: bootstrap `delegates-compute` Commitments at `reach_ceiling: "commons"` are the safe default; any higher reach requires explicit operator-steward signature with elevated capacity.

### §7.5 — Compromise recovery for deploy-svc-agent

**Resolution:** Cradle-to-grave shape. The deploy-svc-agent is a **stewarded compute resource** under the operator-steward — analogous to a ward's agentic compute under a guardian. Recovery follows the 4-layer graduated authority pattern from [cradle-to-grave-capability-gradient](epr:cradle-to-grave-capability-gradient) §3:

| Compromise scenario | Recovery primitive |
| --- | --- |
| Operator-steward retains capacity, detects compromise | **Operator self-revokes** the `delegates-compute` Commitment via Mishpat; provisions new deploy-svc-agent; new Commitment supersedes old. Storage validator rejects in-flight events from the revoked agent immediately. |
| Operator-steward unreachable but alive | **Intimate-circle quorum** (operator's recovery quorum, per `KeyStewardship`) issues `RevokeCommitment` against the deploy-svc-agent's commitment via threshold signature. |
| Operator-steward incapacitated/deceased | **Successor-steward** (named in the operator's pre-arranged succession Commitment) assumes authority and re-issues a fresh `delegates-compute` to a new deploy-svc-agent. Existing already-republished EPRs stay live (audit trail intact, innocent-until-proven). |
| Operator-steward compromised AND deploy-svc-agent compromised | **Network witness / qahal governance act** — slowest path, but absolute lockout is impossible. Qahal stewards can authorize revocation of the operator's standing entirely, transferring doorway custody to a successor. |

**Prior accepted events:** stay accepted. The audit trail (`bounded_by` chain) preserves the historical truth that "at time T, this agent had standing." Past good republishes are not retroactively invalidated.

### §7 implementation reference

Each resolution informs concrete work in the implementation plan at `/projects/.claude-config/plans/abundant-wandering-lemon.md` (Phase 1 tasks 1.5–1.18 + Phase 2). The plan's "Resolved Open Questions" section restates these answers task-by-task for engineers; this section is the canonical record in the spec itself.

---

## §8 — References

- `genesis/docs/superpowers/specs/2026-05-23-doorway-access-tier-patterns.md` — Pattern Z spec; this Z.D is the migration that closes Z.1.
- `genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md` — pillar EPR decomposition; the project-epr Commitment substrate Z.D builds on.
- `elohim/elohim-storage/src/api/epr.rs:484` — `PUT /api/v1/epr/{cid}` route (already implemented; Z.D extends its validation).
- `elohim/elohim-storage/src/p2p/mod.rs` — `FederatedEprStore` + `KadStartProviding` substrate.
- `elohim/holochain/dna/mishpat/zomes/coordinator/commitments/` — where `delegates-compute` action lands.
- `elohim/sdk/domains/elohim/manifest.json` — where the new `republish-epr` event kind + `reach-escalation-pending` signal kind register.

### Memory anchors

- `project_compute_commitments_bounded` — compute commitments are bounded REA primitives; breach never contaminates attribution.
- `project_depin_contracts_are_policy` — stewardship contracts on DHT (REA); libp2p operates within bounds.
- `project_rea_prefix_redundant` — REA is the pattern; resolve asymmetry by dropping prefix, never adding it.
- `project_no_sovereignty_stewardship_over_ownership` — steward/contributor/authored; deploy-svc-agent is a stewarded compute resource, not "owned."
- `project_signal_kind_extensible_protocol_class` — extension path for future trust signals (§5).
- `project_socially_derived_security` — recovery model the deploy-svc-agent rotation pattern mirrors.

---

## Closing note: master the primitive

The diagram in §1 is the protocol-shape that pays compounding returns. Six rows in the generalization table use it. The seventh row, an eighth row, the rows we haven't named yet — they all inherit the same shape. Z.D is the proving ground. Get the bounds, the back-reference, the reciprocity, and the auditability right here and the rest of the protocol gets easier, not harder.
