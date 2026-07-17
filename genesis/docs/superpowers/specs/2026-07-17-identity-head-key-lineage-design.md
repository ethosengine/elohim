---
title: "Identity Head + Agent-Key Lineage — the declared-head-over-DAG primitive, third instance"
id: identity-head-key-lineage
tier: spec
status: Draft
created: 2026-07-17
maintainers: Matthew Dowell + Claude Fable 5
class: protocol-canonical
topic: [identity, agent-key, lineage, dag, declared-head, chain-root, controllers, group-control, community-recovery, key-rotation, witnessed-binding, agent-peer-binding, did, rea-agent, collective, contributor-attribution, mishpat-commitment]
context-tier: disclosed
sovereignty-frame: descriptive
steward: rust-architect
graduation-trigger: decompose-complete OR bind-identity-coordinator-shipped
domain: D2
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-11-attestation-consolidation-design.md
  - genesis/docs/content/elohim-protocol/architecture/2026-05-23-multi-collective-collaboration-epr-design.md
refines:
  - genesis/docs/superpowers/specs/2026-06-27-lens-version-dag-epr-policy-dependency-design.md
  - genesis/docs/superpowers/specs/2026-07-17-did-bridge-identity-resolution-design.md
cites:
  - epr-meta-kinship-lineage-reconciliation | supplies the lineage-in-hashed-bytes + heads-move-by-judgment-not-last-writer-wins rules the identity DAG inherits for cross-key recognition and reconciliation | sha256:adb7385729b94c24 | path: genesis/docs/superpowers/specs/2026-07-12-epr-meta-kinship-lineage-reconciliation-design.md
  - frame-witness-primitive-architecture | the primitive this design EVALUATES and REJECTS for witnessed binding — self-reported, advisory-weight, object-scoped; §4.3 routes to the signed AgentPeerBinding instead | sha256:9acf41622029875e | path: genesis/docs/superpowers/specs/2026-07-15-frame-witness-primitive-architecture-design.md
  - qahal-collective-membership-dht-design | the WIRED Collective+Membership{Steward} DHT primitive that already IS a group-controlled identity head — §4.1 shows REA agents inherit it with zero DNA work | sha256:8d7b9704f7aa9ca0 | path: genesis/docs/superpowers/specs/2026-05-19-qahal-collective-membership-dht-design.md
  - dna-upgrade-governance | the DNA-hash lineage axis this design distinguishes from its agent-key axis — §4.4 explains why reinstall migration does not cleanly inherit and is scoped out | sha256:48b79bbffd184d89 | path: genesis/docs/content/elohim-protocol/architecture/2026-06-11-dna-upgrade-governance.md
  - stewardship-over-sovereignty | the canon grounding the ontology guard — controllers are community-backstopped by construction; the recovery quorum is a controller, never a self-sovereign apex | sha256:995eb2079924ea2e | path: genesis/docs/architecture/stewardship-over-sovereignty.md
---

# Identity Head + Agent-Key Lineage

*The DID bridge's phase-2 follow-on (§5). The central finding: an identity head is **not a new
primitive** — it is the third named instance of the **declared-head-over-lineage-DAG** shape the
substrate already runs, unified under one content-addressed root that every other subsystem can
point at instead of a rotation-fragile raw key.*

## 1. The primitive is already proven — four times

The lens-version-DAG spec sealed the shape (`refines:`): versions are an immutable DAG of
`Mishpat::Commitment` nodes with a `version_parent` back-pointer (a SET, for merge); chain identity
is the **root cid** (`version_parent=[]`), stable across the whole DAG; **which HEAD applies is a
DECLARED dependency** (pin / latest / range), the binding decides, not the infrastructure; revert is
a re-pin at declaration level. The kinship-lineage spec added: lineage lives *inside* the hashed
bytes, and heads move **by judgment over history, never last-writer-wins**.

Grounding (2026-07-17, four read-only agents, file:line verified) found the same shape already
instantiated in identity, in fragments nobody had unified:

| Instance | Substrate (WIRED today) | The shape it already is |
|---|---|---|
| Content lens | `author-lens`/`binds-policy` Commitments | version-DAG + declared head |
| Provenance | epr-meta lineage edges | lineage-in-bytes + judgment reconcile |
| **REA collective** | `Collective{founder,charter,…}` = content-CID identity + `Membership{role:Steward}` set (`imagodei_integrity/src/qahal.rs:22-45`) | **group-controlled identity head** — DID 1.1 Group Control, born with ≥1 Steward controller, recursive (`member_kind::Collective`) |
| **Human key rotation** | `KeyRotation{superseded_agent_pubkey,new_agent_pubkey,authority:RecoveryAuthority}` (`imagodei_integrity/src/recovery_v2.rs:132-139`) | **the version_parent edge** old-key→new-key + authorizing quorum |

So the design is **compose, don't build**: recognize the identity head as this primitive, add the one
missing piece (a chain-root identifier + a controller-declaration action), and re-point the few
raw-keyed references at the root.

## 2. What's actually missing (the only new surface)

1. **A chain-root cid concept.** `KeyRotation` edges exist old→new but are flat — there is no
   `version_parent` on `mishpat_integrity::Commitment` and no derivation of a stable root. The root
   cid (the `version_parent=[]` genesis node) is the identity's durable identifier, unchanged across
   every rotation/recovery. This is what other subsystems point at.
2. **`binds-identity`** — a new `Mishpat::Commitment` **action discriminator** (DNA-hash-neutral
   coordinator hot-swap, exactly as `binds-policy` was): declares *"identity chain C's current head is
   key K; controllers = {set}; controller-policy = self | Steward-set | RecoveryAuthority M-of-N."*
   This is where DID 1.1 Group Control and the community-recovery quorum live — and the human-recovery
   controller **reuses the already-wired** `RecoveryRequest`/`RecoveryAuthority` (`recovery_v2.rs`).
3. **Re-point three raw-keyed references at the chain-root** (§4).

## 3. p2p-design-gate output

1. **Entity class:** Notarized (A). No new DHT entry type — `Mishpat::Commitment` with new action
   discriminators, plus the existing imagodei `KeyRotation` / `Collective` / `Membership` /
   `AgentPeerBinding` integrity kinds. New action discriminators are **DNA-hash-neutral** (payload in
   `payload_json`, coordinator hot-swap — the `author-lens`/`binds-policy` precedent).
2. **DHT entry type exists?** YES — `Mishpat::Commitment` (~11/100 headroom) + the four wired imagodei
   kinds above. Widening `KeyRotation` to carry/derive a DAG root is payload-level.
3. **Identity:** content-derived. Chain-root cid = the genesis node (`version_parent=[]`); each version
   node is `cid == entry_hash`. Never a slug.
4. **Coordinator fn / signal:** `bind_identity` (declare controllers + head) and `rotate_identity_key`
   (append a version node, authorized by current controllers). Signal projects to the humans/collectives
   projection and the `did:elohim` head assembly (already built — this **upgrades it in place**:
   phase-1 self-only projection becomes real controllers + lineage + verified transport ids).

**Ontology guard (imago-dei, structural):** controllers are community-backstopped *by construction* —
the recovery quorum is a **controller**, not an emergency override bolted on; a human identity head's
controller-policy names the community authority in the same field that names self. Sovereignty is never
the apex; the head cannot exist without its controller-set. (`sovereignty-frame: descriptive`.)

## 4. Inheritance — the convergence, answered from evidence

### 4.1 REA agents — WIRED at the DHT layer, DARK at the reference
A `Collective` + its `Membership{Steward}` set already IS a group-controlled identity head (content-CID
identity, multi-controller, recursive). No DNA work. The gap is projection-layer only:
`rea_commitments.provider/receiver` are untyped `Text` (`diesel_schema.rs:793-794`) — populate them
with a collective's chain-root cid when the economic party is a collective (the
`Collective → Organization implements Agent` mapping the multi-collective spec §7.1 already specifies),
and reconcile the SQLite `collectives.id` slug against the DHT content-CID (a `dht_anchor_hash`-style
mirror column, the pattern `rea_commitments` already uses).

### 4.2 Contributor attribution — mostly rotation-safe already; one surgical move
Presence↔EPR edges key on **content CIDs** (`establishing_content_ids_json`), and recognition accrual
(`recognition_score = affinity·0.6 + citation·0.4`, `contributor_presences.rs:313`) is
**presence-scoped, not agent-scoped** — a rotation does not reset it. The single raw-keyed spot is
`claimed_agent_id` / `ClaimedAgentToPresence` (`imagodei_integrity/src/lib.rs:991`). Re-anchor that on
the identity_head chain-root cid; the claim then resolves through the chain and survives rotation
completely.

### 4.3 Witnessed binding — sign the EXISTING binding, not frame-witness
Correction to the initial framing: frame-witness (`elohim/epr/src/witness.rs`) is self-reported,
advisory-weight, and object-scoped — the wrong tool. `AgentPeerBinding` is already an
integrity-notarized kind (`write_through.rs::is_integrity_kind`), merely **unsigned today**
(`STAGE1_SIGNATURE_SENTINEL`; agent and libp2p keys uncrossed). Witnessed binding = **sign it** (a real
challenge/response over the transport channel, cross-signed by the current head key), then a signed
`AgentPeerBinding` adds a verified `alsoKnownAs` transport-id to the head — replacing the phase-1
self-only, unverified `alsoKnownAs` in the `did:elohim` assembly.

### 4.4 DNA-reinstall key migration — named-hard follow-on, partly a different axis
Does NOT cleanly inherit. The manifest `lineage:` field is the **DNA-hash** chain (upstream-blocked:
HC 0.6 gates it behind `unstable-migration`), orthogonal to the **agent-key** chain this spec models.
The key axis has two structural blocks: (a) a fresh key minted at `install_fresh`
(`happ_manager.rs:294-297`) cannot self-notarize its own genesis (no chain exists yet), and (b) reinstall
often changes the DNA hash, so old-key and new-key chains live on different DNAs — a same-DNA link type
can't span them. The `KeyRotation` primitive is the right shape and is WIRED for human recovery, but
wiring it to reinstall needs an out-of-band cross-DNA record (post-install commitment by the new key,
cross-attested by the old key's chain before abandonment) + the restored manifest `lineage:` field once
upstream stabilizes. Scoped OUT of the core; captured as its own hard follow-on.

## 5. Definition of done (core; reinstall excluded)
- `binds-identity` + `rotate_identity_key` action discriminators land in mishpat (DNA-hash-neutral;
  sweettest proves controller-authorized rotation appends a version node and unauthorized is refused).
- Chain-root cid derivation over the key-lineage DAG; a human identity head resolves its root stable
  across a rotation (unit + sweettest).
- `did:elohim` assembly upgraded: real `controller` entries + lineage + **signed** `AgentPeerBinding`
  `alsoKnownAs` (the crate's phase-2 hook from the DID bridge spec §5).
- The three re-pointings (§4.1 REA reference, §4.2 `claimed_agent_id`, §4.3 signed binding) each with
  a test proving rotation-survival.
- Grandma-recovery a2o scenario (roadmap rung 2): a human loses their key, the community-recovery
  quorum authorizes a `rotate_identity_key`, and attribution + REA standing + claims all resolve
  unbroken through the chain-root. `@requires:household-nodes` for the multi-controller leg.

## 6. Composition note
This spec REFINES the lens-version-DAG primitive (same shape, identity instance) and the DID bridge
spec (its named §5 follow-on — this is the phase-2 that upgrades the phase-1 assembly in place). It
does not fork a new identity system: every mechanism it names either exists (Collective, Membership,
KeyRotation, RecoveryAuthority, AgentPeerBinding) or is a hash-neutral action discriminator on the
existing Commitment. Serves roadmap rung 2 (Grandma recovery, imagodei/lamad).
