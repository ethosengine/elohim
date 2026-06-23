---
id: 2026-06-23-recovery-floor-witness-reconnection-ioc-design
status: draft
created: 2026-06-23
class: imagodei
artifact_kind: spec
cites:
  - recovery-protocol-phase-2-revised-design | the Phase-2 recovery design whose IntimateQuorum wiring this spec makes real (the RecoveryAuthority it reconnects) | path: genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md
  - justice-manifesto | the recovery FLOOR this interface makes exercisable — no key ever truly lost, absolute-lockout-impossible | path: genesis/docs/architecture/justice-manifesto.md
  - stewardship-over-sovereignty | social-recovery floor over crypto self-custody; cryptography accelerates recovery, never gates it | path: genesis/docs/architecture/stewardship-over-sovereignty.md
  - cradle-to-grave-capability-gradient | the graduated recovery-authority stack whose IntimateQuorum rung this wires | path: genesis/docs/architecture/cradle-to-grave-capability-gradient.md
  - elohim-seam-map-concern-routing | where recovery sits across the seams — imagodei DNA truth + doorway projection | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md
---

# Recovery FLOOR — Witness Reconnection & IoC Interface Spec

*The contract every recovery consumer builds against. The rung after the Phase-2 design spec: it picks ONE canonical witness artifact, fixes the producer/consumer type mismatch by construction, defines a Rust Inversion-of-Control port so the coordinator never depends on witness storage, reconciles the HTTP path scheme, and gives doorway / storage / frontend / sweettest / a2o each an unambiguous target. **UX is OUT OF SCOPE** — only the zome contract, the IoC port, and the service-layer HTTP interface are defined here.*

---

## 1. Problem

### 1.1 The severance (producer ≠ consumer across the HDI/HDK wall)

The intimate-quorum recovery chain is severed at the witness artifact. There is a producer that writes one thing and a consumer that reads another, and they can never meet:

- **Consumer (validator, HDI):** `validate_intimate_quorum` resolves each witness via `must_get_valid_record(witness_hash) -> imagodei::HumanityWitness` (`elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/recovery_v2.rs:363`). It requires an **in-DNA imagodei `HumanityWitness` ActionHash**. The carrier shape is `RecoveryAuthority::IntimateQuorum { witness_hashes: Vec<ActionHash> }` (`recovery_v2.rs:77`).
- **Producer (coordinator, HDK):** `submit_intimate_witness` writes the canonical witness **cross-DNA** as an `attestation:humanness` `Content` entry on the elohim DNA via `call_elohim_issue_attestation` (`elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs:783-810`, write at ~3939-3967). It writes **NO local `HumanityWitness`** and returns a **zero-sentinel** `ActionHash::from_raw_36(vec![0u8; 36])`.

**Result:** the only ActionHashes the producer can offer are zero-sentinels that resolve to nothing. `must_get_valid_record(zero_hash)` fails deterministically → **a real `IntimateQuorum` KeyRotation can never validate and never land.** The root constraint is a hard capability wall, not a TODO: HDI validation callbacks are deterministic and **cannot `call()` cross-DNA** (`recovery_v2.rs:411-414`, `:897-900` self-document this). The producer's truth lives on the *elohim* DNA; a validator on the *imagodei* DNA can never read it. So the consumer must move, not the producer.

A **second, parallel mismatch** rides the same break: the recovery-request itself migrated cross-DNA. `create_recovery_request` returns `recovery_request_cid: String` (a `governance-action:recovery-request` Content entry on elohim), but `KeyRotation.recovery_request_hash: ActionHash` (`recovery_v2.rs:136`) and `CommitKeyRotationInput.recovery_request_hash: ActionHash` cannot be supplied — there is no imagodei ActionHash to pass.

### 1.2 The coordinator stub

`commit_key_rotation` (`lib.rs:3485`) runs only the freeze-floor gate and the revocation-floor gate, then `create_entry(KeyRotation { recovery_request_hash: input.recovery_request_hash, authority: input.authority, .. })` — passing `input.authority` and `input.recovery_request_hash` **straight through with ZERO witness resolution**. It never reads the witnesses, never recomputes the threshold, never cross-checks the witness↔request binding. The integrity validator's synthesized stub request hardcodes `required_witness_count: 2` and `human_id: None` (`recovery_v2.rs` validate path), and the coordinator does nothing to compensate. So the very gate the validator *explicitly delegates to the coordinator* (`recovery_v2.rs:196-204`, "the witness↔request cross-check is deferred to the coordinator gate") **does not exist**.

The plumbing the gate needs already exists and is unwired here:
- `compute_required_witness_count(m) = max(2, m.div_ceil(2) + 1)` (`lib.rs:2625`);
- `count_active_emergency_contacts(human_id)` (`lib.rs:2594`);
- the pure judge `check_intimate_quorum_rules(&request, &pre_resolved_witnesses)` (`recovery_v2.rs:160`), already written to take *pre-resolved* witnesses for exactly this.

### 1.3 The stubbed / phantom consumers

- **doorway** — three `501 NOT_IMPLEMENTED` handlers citing "Requires imagodei zome integration": `/auth/recover-custody` (`auth_routes.rs:2970`), `/auth/check-recovery-status` (`:2984`), `/auth/activate-recovery` (`:3030`).
- **storage** — recovery *reads* are live (`AccountView`, `pending-recovery`); the EC-vote *write* returns `503 BROWSER_WRITE_PATH_PENDING` (`api/account.rs:535-551`, gated by `verify_caller_owns_cell`). There is no storage route at all for `submit_intimate_witness` or `commit_key_rotation`.
- **frontend** — `recovery-coordinator.service.ts:130` POSTs a **phantom `/api/recovery/initiate`** (plus a whole interview/credential model) that matches no backend route; recovery components are wired into no `.routes.ts`.
- **sweettest** — `recovery_m3.rs:78-127` empty TODO bodies (the happy-path comment encodes the exact calls); `recovery_m4.rs` wiring-only. The mirrors already moved to CIDs (`recovery_m3.rs:43`, `recovery_m4.rs:1125`); the `commit_key_rotation` consumer is the unwritten gap.
- **a2o** — `genesis/a2o/features/auth/recovery/intimate-quorum-happy-path.feature` exists (`@requires:shem`); there is **no `anti-lockout/` suite**.

---

## 2. Scope

**IN scope (the contract consumers build against):**
- The canonical witness artifact decision and the recovery lifecycle (§3).
- The `WitnessResolver` IoC port + the rewritten `commit_key_rotation` (§4).
- The type changes that make producer==consumer by construction (CID-keyed witnesses + request).
- The HTTP API contract + camelCase wire views (§6) and the reconciled path scheme.
- The consumer map: each consumer's exact target + minimal change (§7).
- The ordered first PR sequence that lands one real `IntimateQuorum` rotation end-to-end at the seam (§8).
- The test/a2o contract (§9).

**OUT of scope (separate follow-ons — named so they are not silently dropped):**
- **All UX**: wiring any recovery component into `imagodei/*.routes.ts`, the lost-key entry flow, the EC voting screen, the activation screen. Only the *service-layer client interface* is defined.
- **The stubbed higher-authority layers**: `CommunityConsensus`, `GovernanceAct` remain stub-rejected. They are governance-plane follow-ons.
- **The no-contacts escape hatch (anti-lockout floor)**: `NetworkWitness` (stub-rejected, `recovery_v2.rs:87-93`) is the path that prevents *absolute lockout* for a human with zero emergency contacts. This spec defines the a2o *placeholder* scenario (§9) but does NOT implement the network-witness authority — it is a distinct floor with its own trust model.
- **The M6 browser-write trust decision**: until it lands, browser-via-doorway writes stay `503 BROWSER_WRITE_PATH_PENDING`; Tauri-direct writes (caller owns the cell) are the live path. This spec defines the route shape behind that gate; it does not lift the gate.

---

## 3. The Clean Shape

### 3.1 Canonical witness artifact (the decision)

**The witness IS the elohim-DNA `attestation:humanness` `Content` entry that `submit_intimate_witness` already writes. CID-addressed. Resolved coordinator-side (HDK), never validator-side (HDI).**

This is forced by the HDI constraint (§1.1) and is *consistent with the existing trust model*: the freeze-floor and revocation-floor gates are **already** enforced only coordinator-side (the validator is already a no-op stub for them — `recovery_v2.rs:423-431`). Quorum-counting joins them. We do **not** re-fork the witness into a local imagodei `HumanityWitness` (that would re-create the exact dual-write drift that caused the severance, and would keep quorum inside a validator that already cannot enforce the floors that matter).

**Producer==consumer by construction:** both now speak **elohim Content CIDs**. The producer writes a CID (attestation `entry_hash`, `attestation.rs:151`); the coordinator resolves that CID through the *same bridge the producer's own Gate 1 already uses*. There is no second artifact to drift against. Witness identity = the `entry_hash` CID of the attestation.

### 3.2 Recovery lifecycle (the end-to-end the seam must support)

```
1. ARM      arm emergency contacts        → emergency_access_enabled on HumanRelationship   (sets M)
2. INITIATE create_recovery_request        → recovery_request_cid (governance-action on elohim);
                                             required_witness_count = max(2, ceil(M/2)+1), human_id = Some
3. VOUCH    submit_intimate_witness (×N)    → attestation:humanness Content on elohim;
                                             returns witness_cid (= attestation entry_hash CID)
4. POLL     get_recovery_status            → live tally via ConductorWitnessResolver (projection lags)
5. COMMIT   commit_key_rotation            → resolves witness_cids via WitnessResolver, recomputes
            { IntimateQuorum { witness_cids } }   threshold, runs check_intimate_quorum_rules; on pass
                                             create_entry(KeyRotation { recovery_request_cid, .. }),
                                             move HumanToCurrentAgent link, emit KeyRotationCommitted
```

### 3.3 Type changes the decision forces (the wire-level reconnection)

```
// recovery_v2.rs (integrity)
RecoveryAuthority::IntimateQuorum { witness_hashes: Vec<ActionHash> }  // BEFORE — unresolvable
RecoveryAuthority::IntimateQuorum { witness_cids:  Vec<String>      }  // AFTER  — elohim CIDs

KeyRotation.recovery_request_hash: ActionHash                          // BEFORE
KeyRotation.recovery_request_cid:  String                             // AFTER  — validator stops dereferencing

// lib.rs (coordinator)
CommitKeyRotationInput.recovery_request_hash: ActionHash              // BEFORE — no imagodei AH exists
CommitKeyRotationInput.recovery_request_cid:  String                  // AFTER  — matches create_recovery_request

SubmitIntimateWitnessOutput.action_hash: ActionHash (zero-sentinel)   // BEFORE
SubmitIntimateWitnessOutput.witness_cid: String                      // AFTER  — the exact key commit consumes
```

`validate_intimate_quorum` (and its `must_get_valid_record` loop) is **deleted**. `validate_key_rotation` keeps structural checks only (new≠superseded; variant well-formed; `witness_cids.len() >= 2` shape floor; no cross-DNA deref). The pure helper `check_intimate_quorum_rules` (`recovery_v2.rs:160`) is **kept verbatim** and re-homed as the coordinator's deterministic judge.

---

## 4. The IoC Port

The coordinator depends on an abstraction, not on a DNA location. Modeled on the house `CommitmentFetcher` template (`elohim/elohim-storage/src/services/commitment_fetcher.rs`): one trait, conductor-backed + mock impls, **fail-closed on un-notarized / missing provenance**. The trait lives in the imagodei coordinator (it resolves via the cross-DNA bridge `commit_key_rotation` already uses). The port only *resolves + counts*; the pure helper *judges*. **Synchronous** (no `async_trait`) — the conductor `call()` is synchronous inside a zome.

```rust
// elohim/holochain/dna/imagodei/zomes/imagodei/src/recovery_witness.rs  (NEW, coordinator-only)

/// A witness resolved to the fields quorum-counting needs. Decoupled from where
/// the bytes live (elohim Content today; could move tomorrow without re-severing).
#[derive(Clone)]
pub struct ResolvedWitness {
    pub witness_cid: String,         // entry_hash CID — witness identity
    pub subject_human_id: String,    // the human being recovered
    pub witness_author_id: String,   // distinct-author identity (the emergency contact)
    pub recovery_request_cid: String,// the request this witness was cast for
    pub revoked: bool,               // metadata revoked_at present
    pub expired: bool,               // metadata expires_at < now
}

#[derive(Debug)]
pub enum WitnessResolveError {
    /// Infra: cross-DNA bridge unreachable. Maps to 503 upstream. NEVER silently 0-counts.
    BridgeUnreachable(String),
    /// A claimed witness CID resolves to no Content entry. Fail-closed: do not count it.
    NotFound(String),
    /// Resolved entry is not attestation:humanness, or metadata is unparseable. Fail-closed.
    Malformed(String),
}

/// IoC port. `commit_key_rotation` depends on THIS, never on call_elohim_* directly.
pub trait WitnessResolver {
    /// Resolve each claimed witness CID to its quorum-relevant fields.
    /// MUST fail-closed: a NotFound / Malformed CID is an Err, never a dropped row,
    /// so a forged or empty CID can never silently shrink the rejection surface.
    fn resolve(&self, witness_cids: &[String]) -> Result<Vec<ResolvedWitness>, WitnessResolveError>;

    /// The live required threshold for this human: max(2, ceil(M/2)+1) over ACTIVE
    /// emergency contacts. Reads the same substrate the request was sized against.
    fn required_count(&self, subject_human_id: &str) -> Result<u32, WitnessResolveError>;
}
```

### 4.1 Real implementation — conductor / cross-DNA fetch (the just-authored path)

Witnesses are read moments after they are submitted; the SQL projection lags. So the real resolver reads **live DHT via the conductor `call()` bridge**, not storage — the `ConductorCommitmentFetcher` rationale. **Load-bearing detail the port encapsulates:** `get_attestations_for_subject` returns only `{cid, attestation_kind, subject_cid, issuer_cid}` — **no metadata** (`attestation.rs:201-202`). To read `recovery_request_cid` / `revoked_at` / `expires_at`, the resolver MUST follow each CID through `get_content_by_id` for `metadata_json`.

```rust
pub struct ConductorWitnessResolver;  // zero-state: uses ambient HDK call()

impl WitnessResolver for ConductorWitnessResolver {
    fn resolve(&self, witness_cids: &[String]) -> Result<Vec<ResolvedWitness>, WitnessResolveError> {
        let mut out = Vec::with_capacity(witness_cids.len());
        for cid in witness_cids {
            // get_attestations_for_subject carries NO metadata — follow the CID to Content.
            let content = call_elohim_get_content_by_id(cid)               // lib.rs:932-ish bridge
                .map_err(|e| WitnessResolveError::BridgeUnreachable(e.to_string()))?
                .ok_or_else(|| WitnessResolveError::NotFound(cid.clone()))?; // fail-closed
            if content.content.content_type != "attestation:humanness" {
                return Err(WitnessResolveError::Malformed(format!("{cid}: not a humanness attestation")));
            }
            let md: serde_json::Value = serde_json::from_str(&content.content.metadata_json)
                .map_err(|e| WitnessResolveError::Malformed(format!("{cid}: {e}")))?;
            out.push(ResolvedWitness {
                witness_cid: cid.clone(),
                subject_human_id: md["subject_human_id"].as_str().unwrap_or_default().to_string(),
                witness_author_id: md["authorizer_human_id"].as_str().unwrap_or_default().to_string(),
                recovery_request_cid: md["recovery_request_cid"].as_str().unwrap_or_default().to_string(),
                revoked: md.get("revoked_at").map(|v| !v.is_null()).unwrap_or(false),
                expired: witness_metadata_expired(&md)?,  // expires_at < sys_time()
            });
        }
        Ok(out)
    }

    fn required_count(&self, subject_human_id: &str) -> Result<u32, WitnessResolveError> {
        let m = count_active_emergency_contacts(subject_human_id)   // lib.rs:2594 — EXISTS, was unwired here
            .map_err(|e| WitnessResolveError::BridgeUnreachable(e.to_string()))?;
        Ok(compute_required_witness_count(m))                       // lib.rs:2625 — EXISTS, was unwired here
    }
}
```

### 4.2 Test double (drives `commit_key_rotation` with no DNA)

```rust
#[cfg(test)]
pub struct MockWitnessResolver {
    pub witnesses: std::collections::HashMap<String, ResolvedWitness>,
    pub required: u32,
    pub bridge_down: bool,  // exercise the 503 / fail-closed path
}
#[cfg(test)]
impl WitnessResolver for MockWitnessResolver {
    fn resolve(&self, cids: &[String]) -> Result<Vec<ResolvedWitness>, WitnessResolveError> {
        if self.bridge_down { return Err(WitnessResolveError::BridgeUnreachable("mock".into())); }
        cids.iter()
            .map(|c| self.witnesses.get(c).cloned().ok_or_else(|| WitnessResolveError::NotFound(c.clone())))
            .collect()
    }
    fn required_count(&self, _subject_human_id: &str) -> Result<u32, WitnessResolveError> { Ok(self.required) }
}
```

This makes the mismatch **un-recurrable**: the coordinator names a `WitnessResolver`, never `call_elohim_*` directly. Whatever DNA holds the witness bytes, the contract is one trait with a fail-closed `resolve`.

### 4.3 Rewritten `commit_key_rotation` (the real gate)

Freeze + revocation gates are **unchanged** (`lib.rs:3488-3556`). The new witness-quorum gate sits between them and `create_entry`.

```rust
pub fn commit_key_rotation(input: CommitKeyRotationInput) -> ExternResult<KeyRotationOutput> {
    let now = sys_time()?;
    let resolver = ConductorWitnessResolver;  // IoC: the ONLY binding to witness storage

    // [Gate A] freeze-floor      — UNCHANGED (lib.rs:3488-3504).
    // [Gate B] revocation-floor  — UNCHANGED (lib.rs:3527-3556).

    // [Gate C] NEW — witness-quorum gate (IntimateQuorum only; CryptographicQuorum
    //          sig-verifies in the validator, which CAN do that — same-DNA KeyStewardship).
    if let RecoveryAuthority::IntimateQuorum { witness_cids } = &input.authority {
        // C1: canonical human_id + live threshold, both read from the SAME cross-DNA
        //     request the producer used, so witness↔request binding cannot drift.
        let request_human_id = fetch_recovery_request_human_id(&input.recovery_request_cid)?; // lib.rs:932
        let required = resolver.required_count(&request_human_id)
            .map_err(witness_err_to_wasm)?;                 // real max(2, ceil(M/2)+1) — NOT hardcoded 2

        // C2: resolve witnesses fail-closed (forged / empty CID ⇒ Err, never a dropped row).
        let resolved = resolver.resolve(witness_cids).map_err(witness_err_to_wasm)?;

        // C3: cross-check EVERY witness was cast for THIS request and THIS human
        //     (the check recovery_v2.rs:196-204 delegates to the coordinator; the stub never did it).
        for w in &resolved {
            if w.recovery_request_cid != input.recovery_request_cid {
                return Err(guest_err("witness was cast for a different recovery request"));
            }
            if w.subject_human_id != request_human_id {
                return Err(guest_err("witness subject does not match the request subject"));
            }
        }

        // C4: build the (HumanityWitness, author) pairs the PURE helper expects and defer to it
        //     verbatim — distinct-author, threshold, revoked, human-agreement all encoded there.
        let judge_request = RecoveryRequest {
            human_id: Some(request_human_id.clone()),   // NO LONGER None
            required_witness_count: required,            // NO LONGER hardcoded 2
            ..request_skeleton(&input, now)
        };
        let pairs: Vec<(HumanityWitness, AgentPubKey)> = resolved.iter()
            .filter(|w| !w.revoked && !w.expired)        // fail-closed pre-filter
            .map(|w| Ok((synth_witness(w), pubkey_for_human(&w.witness_author_id)?)))
            .collect::<ExternResult<_>>()?;
        if let ValidateCallbackResult::Invalid(reason) =
            check_intimate_quorum_rules(&judge_request, &pairs)
        {
            return Err(guest_err(&format!("intimate-quorum gate rejected rotation: {reason}")));
        }
    }

    // create_entry(KeyRotation { recovery_request_cid, authority, .. }) — shape UNCHANGED except
    // recovery_request_hash → recovery_request_cid: String; anchors + KeyRotationCommitted signal UNCHANGED.
}
```

Net effect against the audit: real `required_witness_count` (not 2); `human_id = Some(request_human_id)` (not None); distinct-author + human-agreement + revoked + expired enforced; witness↔request binding checked; no hardcoded stub. The two "exist but unwired" helpers are now wired, behind the port.

---

## 5. P2P Design Gate

```
ENTITY: recovery witness (intimate-quorum vouch)

(1) SOURCE-OF-TRUTH CLASS:  B2 — agent-scoped with a notarized attestation.
    Authored BY one agent (the emergency contact) ABOUT another (the recovering human).
    Not free-standing notarized fact (A) — a standing claim by a specific agent, carrying
    a notarized attestation ⇒ B2, not bare B. Confirmed by the producer: subject_kind=agent,
    proof_class="social-witness", reach="private" (lib.rs ~3942-3960).

(2) DHT SOURCE-OF-TRUTH?  YES. The witness IS a DHT Content entry on the elohim DNA
    (content_type="attestation:humanness", attestation.rs). Storage is a lagging projection —
    quorum counting MUST read the conductor (live DHT), not SQLite. DHT wins on disagreement.

(3) IDENTITY:  Content-derived CID = entry_hash of the attestation Content entry
    (attestation.rs:151). NOT a slug, NOT agent-composite. KeyRotation references witnesses by
    Vec<String> of these CIDs. The recovery-request is likewise CID-addressed (its own entry_hash).

(4) NEW ENTRY TYPE?  NO — and that is the point of the reconnection. attestation:humanness Content
    ALREADY EXISTS (the producer writes it today). The orphaned local imagodei HumanityWitness type
    is RETIRED from the read path (its consumer validate_intimate_quorum is deleted; the type stays
    registered only for legacy link-target compat — no new authoring). DHT entry-type budget UNCHANGED.
    Link-type budget UNCHANGED.

(5) COORDINATOR fn / SIGNAL:
    CREATE  → submit_intimate_witness (writes attestation:humanness cross-DNA) — EXISTS (returns witness_cid).
    CONSUME → commit_key_rotation resolves witnesses via WitnessResolver (HDK) — REWRITTEN (§4.3).
    PROJECT → IntimateWitnessSubmitted signal → RecoveryWitnessView in storage — EXISTS
              (field swap _hash → _cid makes the projection deterministic, §6).

(6) ROUTE LAST:  no NEW doorway main-listener route in this spec — the substrate routes proxy
    through doorway's existing /api/v1 proxy. The three /auth/* custodial paths are ALREADY in
    AUTH_OWNED_PATHS (http.rs:1794-1796), so is_service_path covers them. Any FUTURE doorway-authored
    GET on the 8080 main listener needs BOTH a match arm AND an is_service_path/AUTH_OWNED_PATHS entry
    + unit test (the /auth/portal shadow incident).
```

---

## 6. HTTP API Contract

### 6.1 Canonical path scheme — pick ONE, repoint the rest (no phantom routes left)

Three schemes exist in the wild: doorway's `/auth/recover-*` custodial 501s, the frontend's **phantom `/api/recovery/*`** (matches no backend), and storage's substrate `/api/v1/account/*`. **Canonical = the storage `/api/v1/account/recovery/*` substrate surface, proxied through doorway** (house rule: storage owns substrate routes via `build_manifest()`; doorway has no per-domain proxy files; reads + writes auto-route through doorway's `/db` / `/api/v1` proxy).

- The frontend **retires `/api/recovery/*` entirely** and repoints to `/api/v1/account/recovery/*`.
- The doorway `/auth/recover-custody` flow is retained **only** as the steward→JWT custodial-account convenience that wraps the same coordinator fns — not a parallel recovery API.
- **Naming-drift reconciliation (no phantom left):** the existing `POST /api/v1/account/recovery/{id}/vote` forwards to `submit_revocation_vote` — that is the **revocation** path, distinct from recovery-witness vouching. To remove ambiguity, recovery-witness submission lands at `POST /api/v1/account/recovery/{cid}/witness` (new), and the revocation vote keeps `/recovery/{id}/vote`. Both are documented here so neither reads as a phantom. (Sibling field-name fix: storage's `SubmitRevocationVoteZomeInput.revocation_id` should align to `revocation_cid` to match the m4 mirror — flagged, not blocking.)

### 6.2 Contract table (camelCase wire; route designed last + thinnest)

| Step | Method + Path | R/W | Request (camelCase) | Response (camelCase) | Coordinator fn |
|------|---------------|-----|---------------------|----------------------|----------------|
| initiate | `POST /api/v1/account/recovery` | **W** | `{ humanAgentPubkey, newAgentPubkey, hostingDoorwayPubkey, proposedAuthorityKind, requestNonce }` | `RecoveryRequestView` (carries `recoveryRequestCid`) | `create_recovery_request` |
| submit witness | `POST /api/v1/account/recovery/{cid}/witness` | **W** | `{ note? }` | `RecoveryWitnessView` (carries `witnessCid`) | `submit_intimate_witness` |
| poll status | `GET /api/v1/account/recovery/{cid}` | **R** | — | `RecoveryStatusView` | (read) `get_recovery_status` + `ConductorWitnessResolver` for just-authored witnesses |
| commit rotation | `POST /api/v1/account/recovery/{cid}/rotation` | **W** | `{ newAgentPubkey, supersededAgentPubkey, authority: { kind:"intimateQuorum", witnessCids:[…] } }` | `KeyRotationView` | `commit_key_rotation` |
| arm contacts | `POST /api/v1/account/recovery/emergency-contacts` | **W** | `{ contactHumanId, emergencyAccessEnabled }` | `EmergencyContactView` | (set `emergency_access_enabled` on `HumanRelationship`) |
| list contacts | `GET /api/v1/account/recovery/emergency-contacts` | **R** | — | `EmergencyContactView[]` | (read) projection |

Existing live reads stay as-is: `GET /api/v1/account` (`AccountView`), `GET /api/v1/account/pending-recovery` (`RecoveryRequestView[]` where caller is EC). The pre-existing `POST /api/v1/account/recovery/{id}/vote` → `submit_revocation_vote` is the **revocation** path (kept; not conflated).

**Reads vs writes / bridge needs.** Reads project from SQLite, **but witness tallies are just-authored → read the live count via `ConductorWitnessResolver`** (projection lags). Writes go `storage → forward_to_imagodei → conductor`, gated by `verify_caller_owns_cell` (returns `503 BROWSER_WRITE_PATH_PENDING` for browser-via-doorway until M6; Tauri-direct works today). **admission_exempt:** recovery writes are NOT exempt (real writes under load); the **poll-status read** SHOULD be `admission_exempt` so a recovering human can check status while the node sheds — recommend marking `GET /api/v1/account/recovery/{cid}` admission_exempt.

### 6.3 Wire views (ts-rs camelCase; 6-step add-a-view recipe)

The recovery views already exist end-to-end (`elohim/elohim-views/src/imagodei.rs`, schemas in `sdk/schemas/v1/views/`, codegen registered). Steps 1-4 are DONE; this spec **edits fields** to the CID contract, then re-runs steps 5-6 (`pnpm run schema:codegen:ts` + `cargo test export_bindings`). Source-of-truth declared in each schema ("Source of truth: DHT.").

**`RecoveryRequestView`** (imagodei.rs:567) — no shape change; the two load-bearing fields become *populated*, not `None`/hardcoded:
```jsonc
{ "dhtAnchorHash": "...", "humanAgentPubkey": "...", "newAgentPubkey": "...",
  "hostingDoorwayPubkey": "...", "proposedAuthorityKind": "intimateQuorum",
  "proposedAuthorityJson": "", "requestNonce": [/* u8 */],
  "humanId": "human-…",            // now ALWAYS Some for IntimateQuorum
  "requiredWitnessCount": 3,       // now coordinator-computed, not hardcoded 2
  "createdAt": "..." }
```

**`KeyRotationView`** (imagodei.rs:594) — `recoveryRequestHash` → **`recoveryRequestCid`** (already a `String` field, so this is a `_hash`→`_cid` content rename, NOT a type change); `authorityJson` carries `witnessCids` for the IntimateQuorum variant:
```jsonc
{ "dhtAnchorHash": "...", "humanAgentPubkey": "...", "newAgentPubkey": "...",
  "supersededAgentPubkey": "...", "recoveryRequestCid": "bafyrei…",
  "authorityKind": "intimateQuorum",
  "authorityJson": "{\"witnessCids\":[\"bafyrei…\",\"bafyrei…\"]}",
  "rotatedAt": "..." }
```

**`RecoveryWitnessView`** (imagodei.rs:613) — `recoveryRequestHash` → **`recoveryRequestCid`**; ADD `witnessCid` (deterministic resolution from the projection):
```jsonc
{ "dhtAnchorHash": "...", "witnessCid": "bafyrei…", "recoveryRequestCid": "bafyrei…",
  "witnessAgentId": "human-…", "humanId": "human-…", "note": "…|null", "submittedAt": "..." }
```

**`RecoveryStatusView`** (NEW — the poll surface; full recipe steps 1-6):
```jsonc
{ "recoveryRequestCid": "bafyrei…", "humanId": "human-…",
  "status": "pending|threshold_met|rotated|expired",
  "requiredWitnessCount": 3, "currentWitnessCount": 2,
  "witnesses": [ /* RecoveryWitnessView */ ],
  "rotationCid": "bafyrei…|null", "expiresAt": "...", "createdAt": "..." }
```

**`EmergencyContactView`** (NEW — armed contacts; full recipe). *Cheaper option:* reuse the existing `HumanRelationshipView` filtered to `emergencyAccessEnabled == true` and skip the new view — architect's call.
```jsonc
{ "dhtAnchorHash": "...", "humanId": "human-…", "contactHumanId": "human-…",
  "contactDisplay": "…", "emergencyAccessEnabled": true, "armedAt": "..." }
```

---

## 7. Consumer Map

| Consumer | File:line | Builds against (target) | Minimal change |
|----------|-----------|-------------------------|----------------|
| **doorway** | `doorway/doorway-service/src/routes/auth_routes.rs:2970 / :2984 / :3030` (3 × 501) | `ZomeCaller::call("imagodei", …)` → `create_recovery_request` / `submit_intimate_witness` / `commit_key_rotation`; reuse the existing camelCase `RecoverCustody*` / `CheckRecoveryStatus*` / `ActivateRecovery*` structs as the HTTP shapes | replace each 501 body with a `ZomeCaller` call mapping request→coordinator input and output→response; custodial path only; is_service_path already covered |
| **storage** | `elohim/elohim-storage/src/api/account.rs` (`:624` forward_to_imagodei, `:522/535` 503s, `:770` EC-vote) | the 6 routes (§6.2) in `build_manifest()` (`http.rs:~11920`) via `forward_to_imagodei` + `verify_caller_owns_cell` + `map_zome_err_to_http`; reads return `RecoveryStatusView` / `RecoveryRequestView` / `EmergencyContactView` | add handlers mirroring `handle_self_revocation` shape; align `SubmitRevocationVoteZomeInput` field drift (`revocation_id` → `revocation_cid`) as a sibling fix; EC-vote 503 stays until M6 |
| **frontend** | `app/elohim-app/src/app/imagodei/services/recovery-coordinator.service.ts:130` (phantom `/api/recovery/initiate`) | the storage `/api/v1/account/recovery/*` contract (§6) via `doorwayRegistry.selectedUrl()`; types = ts-rs-generated `RecoveryRequestView` / `RecoveryStatusView` / `RecoveryWitnessView` / `EmergencyContactView` from `@elohim/storage-client` | repoint all `/api/recovery/*` calls to `/api/v1/account/recovery/*`; replace the bespoke interview/credential model with the generated views; **UX/route wiring OUT OF SCOPE** — only the service-layer client interface |
| **sweettest** | `elohim/holochain/tests/sweettest/src/tests/recovery_m3.rs:78-127` (empty TODO) | `create_recovery_request` → `recoveryRequestCid` → N× `submit_intimate_witness` → `witnessCid` → `commit_key_rotation { authority: IntimateQuorum { witness_cids } }` (CID-keyed end-to-end) | fill the TODO bodies against the CID contract; the e2e becomes *writable* because producer (`witnessCid`) == consumer (`witness_cids`); `MockWitnessResolver` for unit-layer threshold tests |
| **a2o** | `genesis/a2o/features/auth/recovery/intimate-quorum-happy-path.feature` (`@requires:shem`); no `anti-lockout/` | the executable contract above is the acceptance target | make `intimate-quorum-happy-path.feature` pass against the reconnected seam; ADD a placeholder `anti-lockout/` scenario for the `NetworkWitness` floor (currently stub-rejected); happy path provable on household-nodes, only cross-node discovery needs shem |

---

## 8. Ordered First Iteration (the minimal PR sequence)

The goal of iteration one: **reconnect the chain and land ONE real `IntimateQuorum` rotation end-to-end at the seam**, leaving UX. Each PR is independently green.

1. **PR-1 — Type reconnection (integrity + coordinator I/O).** `RecoveryAuthority::IntimateQuorum { witness_cids: Vec<String> }`; `KeyRotation.recovery_request_cid: String`; `CommitKeyRotationInput.recovery_request_cid: String`; `SubmitIntimateWitnessOutput.witness_cid: String` (return the real attestation CID, drop the zero-sentinel). Delete `validate_intimate_quorum`; demote `validate_key_rotation` to structural-only; keep `check_intimate_quorum_rules`. *Green:* integrity + coordinator compile; `compute_required_witness_count` unit tests unchanged.
2. **PR-2 — The IoC port.** Add `recovery_witness.rs`: `WitnessResolver` trait, `ResolvedWitness`, `WitnessResolveError`, `ConductorWitnessResolver`, `MockWitnessResolver`. *Green:* unit tests drive `MockWitnessResolver` (fail-closed on NotFound, bridge-down → 503-class, threshold = `required`).
3. **PR-3 — Rewrite `commit_key_rotation`** to run Gate C via the port (§4.3). *Green:* a coordinator unit test with `MockWitnessResolver` proves: 2 witnesses with required=3 → rejected; 3 distinct → accepted; one revoked/expired → not counted; witness for a different request → rejected.
4. **PR-4 — Wire-view field swap + codegen.** Edit `KeyRotationView`/`RecoveryWitnessView` (`_hash`→`_cid`, add `witnessCid`); add `RecoveryStatusView` (+ `EmergencyContactView` or the filtered-`HumanRelationshipView` decision); update schemas; `pnpm run schema:codegen:ts` + `cargo test export_bindings`. *Green:* `schema_contract.rs` passes; generated TS byte-stable.
5. **PR-5 — Storage routes.** Add the 6 §6.2 routes to `build_manifest()` via `forward_to_imagodei`; mark poll-status `admission_exempt`. *Green:* route tests; writes return the M6 503 (honest), reads return the views.
6. **PR-6 — Sweettest e2e.** Fill `recovery_m3.rs:78-127` happy-path against the CID contract (arm → initiate → vouch ×N → commit). *Green:* `m3_happy_path_intimate_quorum` lands a real `IntimateQuorum` rotation and verifies the `HumanToCurrentAgent` link moved — **the proof the chain is reconnected end-to-end.**

Deferred to follow-on PRs (post-seam): doorway 501→ZomeCaller bodies (PR-7), frontend repoint + model replacement (PR-8, service layer only), a2o anti-lockout placeholder (PR-9). UX wiring is a separate epic.

---

## 9. Test / a2o Contract

- **Coordinator unit (PR-3):** `MockWitnessResolver`-driven matrix — below-threshold reject, at-threshold accept, revoked-excluded, expired-excluded, cross-request reject, cross-human reject, bridge-down → 503-class error (never a silent 0-count).
- **Schema contract (PR-4):** each edited/new view round-trips `validate_against_schema` + `assert_source_of_truth_declared` in `elohim/elohim-storage/tests/schema_contract.rs`.
- **Sweettest e2e (PR-6):** `recovery_m3.rs::m3_happy_path_intimate_quorum` — the executable form of the lifecycle (§3.2); `m3_non_contact_witness_rejected`; `m3_freeze_floor_blocks_intimate_allows_cryptographic` (freeze gate unchanged, must still bite).
- **a2o acceptance:** `genesis/a2o/features/auth/recovery/intimate-quorum-happy-path.feature` becomes runnable against the reconnected seam (provable on household-nodes; only cross-node discovery needs `@requires:shem`). ADD `genesis/a2o/features/auth/recovery/anti-lockout/` with a **placeholder** scenario asserting that a human with zero emergency contacts has a defined non-lockout path (the `NetworkWitness` floor) — `@wip`, since the authority itself is out of scope here.

---

## 10. What This Spec Does NOT Do

- It does **not** build any UX or wire a recovery component into `imagodei/*.routes.ts`. Only the service-layer client interface is defined.
- It does **not** implement `CommunityConsensus`, `GovernanceAct`, or `NetworkWitness` authorities (they remain stub-rejected). The anti-lockout/no-contacts escape is named and given a placeholder a2o scenario only.
- It does **not** lift the M6 browser-write trust gate; browser-via-doorway writes stay `503 BROWSER_WRITE_PATH_PENDING`. Tauri-direct is the live write path.
- It does **not** introduce any new DHT entry type or link type — the witness is the already-existing elohim `attestation:humanness` Content entry; the orphaned local `HumanityWitness` is retired from the read path only.
- It does **not** change the freeze-floor or revocation-floor gates (already correct and coordinator-enforced).

---

## References

- `genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md` — the design spec this builds on.
- IoC template: `elohim/elohim-storage/src/services/commitment_fetcher.rs` (trait + conductor/projection/mock impls + fail-closed `NotarizedRequired`).
- Severance consumer: `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/recovery_v2.rs` (`:77` enum, `:136` KeyRotation, `:160` pure judge, `:350-380` validate_intimate_quorum to delete, `:411-414`/`:897-900` HDI-cannot-cross-DNA).
- Producer + coordinator: `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` (`:783-810` issue_attestation bridge, `:932` fetch_recovery_request_human_id, `:2594` count_active_emergency_contacts, `:2625` compute_required_witness_count, `:3485-3602` commit_key_rotation stub, `:3869-4002` submit_intimate_witness).
- Witness substrate: `elohim/holochain/dna/elohim/zomes/content_store/src/attestation.rs` (`:151` CID = entry_hash; `:201-202` thin output — why the resolver must follow the CID for metadata).
- Wire views: `elohim/elohim-views/src/imagodei.rs` (`:567` RecoveryRequestView, `:594` KeyRotationView, `:613` RecoveryWitnessView, `:718` AccountView).
- Schemas + codegen: `elohim/sdk/schemas/v1/views/{recovery-request,recovery-witness,key-rotation}.schema.json`; `elohim/sdk/schemas/scripts/codegen-ts.mjs`; harness `elohim/elohim-storage/tests/schema_contract.rs`.
- Storage seam + routes: `elohim/elohim-storage/src/api/account.rs` (`:522/535` 503s, `:624` forward_to_imagodei, `:770` EC-vote); `elohim/elohim-storage/src/http.rs:~11920` build_manifest.
- Doorway: `doorway/doorway-service/src/routes/auth_routes.rs` (3 × 501); `doorway/doorway-service/src/services/zome_caller.rs`; `doorway/doorway-service/src/server/http.rs:1778-1804` AUTH_OWNED_PATHS / is_auth_owned_path.
- Frontend: `app/elohim-app/src/app/imagodei/services/recovery-coordinator.service.ts:130`.
- Sweettest: `elohim/holochain/tests/sweettest/src/tests/recovery_m3.rs`, `…/recovery_m4.rs`.
- a2o: `genesis/a2o/features/auth/recovery/` (add `anti-lockout/`).

---
