---
title: "Runtime Artifacts as Elected Content — rung-5 upgrade propagation over the p2p dataplane"
id: runtime-artifacts-elected-content
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
graduation-trigger: the mesh a2o receipt (publish → elect → adopt → attest → promote → converge → revert-by-re-election) passes on 3 peers AND the operator records acceptance of §4's constitutional-posture language (a signed-off edit or an epr flow note on this spec)
created: 2026-09-01
domain: D2
topic: [upgrade-propagation, canonical-head, release-channel, adoption-controller, reach, vsm-ecology, rollback, dataplane, brit, change-class-routing, ci-publisher, build-attestation]
informed-by:
  - genesis/data/timeline/backlog/upgrade-propagation-p2p-design-arc.md (the velocity ladder + all three 2026-08-31/2026-09-01 operator course-sets this spec designs from)
  - genesis/docs/content/elohim-protocol/architecture/2026-06-11-dna-upgrade-governance.md (the hash mechanics + migration-seam build-state this spec composes with, never restates)
  - genesis/docs/content/elohim-protocol/architecture/2026-07-14-upgrade-revert-and-constitutional-consensus.md ("the companion" throughout; its §1 two-conductor covenant stands verbatim at the DNA seam — §4 below EXTENDS the consent doctrine to the above-the-DNA-line classes §1's mechanic never governed)
  - genesis/docs/superpowers/specs/2026-07-17-identity-head-key-lineage-design.md (the compose-don't-build rule for declared-head instances)
  - genesis/docs/superpowers/specs/2026-08-10-fresh-head-nomination-design.md (the candidacy/anti-self-election contract template §7 fills)
  - genesis/docs/superpowers/specs/2026-07-22-reach-ontology-vocabulary-split-spec.md (narrow-never-widen; the reach axis release channels are born on)
  - genesis/plans/2026-03-20-p2p-native-build-system-roadmap.md (the four-stage arc; this spec is the buildable slice of Stages 1-2)
cites:
  - genesis/data/timeline/backlog/upgrade-propagation-p2p-design-arc.md
  - "dna-upgrade-governance | DNA Upgrade Governance | sha256:48b79bbffd184d89 | path: genesis/docs/content/elohim-protocol/architecture/2026-06-11-dna-upgrade-governance.md"
  - "upgrade-revert-and-constitutional-consensus | Upgrade, Revert, and Constitutional Consensus | sha256:4673f9958d96b617 | path: genesis/docs/content/elohim-protocol/architecture/2026-07-14-upgrade-revert-and-constitutional-consensus.md"
  - "substrate-trust-contract-runbook | The Substrate Trust Contract | sha256:e47d962ca7259c79 | path: genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md"
  - "identity-head-key-lineage | Identity Head + Agent-Key Lineage | sha256:95950b918c8803bc | path: genesis/docs/superpowers/specs/2026-07-17-identity-head-key-lineage-design.md"
  - "fresh-head-nomination-after-ghost-declaration-decay | Fresh-head nomination after ghost-declaration decay | sha256:0c365178d261e30e | path: genesis/docs/superpowers/specs/2026-08-10-fresh-head-nomination-design.md"
  - genesis/data/timeline/backlog/governance-native-dna-upgrade-path.md
  - elohim/elohim-storage/src/happ_manager.rs
  - elohim/elohim-storage/src/services/head_adoption.rs
  - elohim/elohim-storage/src/runtime_passport.rs
  - elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
  - elohim/rakia/README.md
  - elohim/rakia/docs/plans/stage-2-canopy.md
  - elohim/brit/docs/specs/2026-04-12-brit-design.md
  - elohim/brit/docs/specs/2026-06-29-canonical-epr-meta-git-bridge-design.md
  - elohim/brit/docs/specs/2026-07-12-shared-crate-consolidation-design.md
  - elohim/brit/docs/specs/2026-04-27-build-contract-before-push-design.md
  - elohim/rakia/docs/specs/2026-04-27-rakia-as-brit-attestation-executor-design.md
  - "rung5-workspace-orchestration-plan | Rung 5 from the workspace — the every-class-over-p2p plan §12.2 routes change classes onto | sha256:169a4ce413aaa5aa | path: genesis/docs/superpowers/plans/2026-09-05-rung5-workspace-orchestration-plan.md"
  - "holochain-evolution-epic | Holochain Evolution Epic — the happ-lineage class and the accepted constitutional crossing §12.1's promotion row defers to | sha256:d821c5f45fd5d2e5 | path: genesis/docs/superpowers/specs/2026-09-03-holochain-evolution-epic-design.md"
  - genesis/data/timeline/backlog/edge-quiesce-gate-timeout-aborts.md
---

# Runtime Artifacts as Elected Content

**One sentence:** a release is a content node, the canonical release is an elected
head, the adoption controller is a per-peer reconciliation loop that keeps the
running node converged on the head its channel declares, and revert is the
election moving back — so delivery moves THROUGH the p2p network, and the CI
roll stops being the delivery path for everything above the DNA line.

**The velocity ladder** (arc doc, operator course-set 2026-08-31 — a debt
snowball, smallest atomic-discipline debts paid first):

| Rung | Substance | State |
|---|---|---|
| 1 | Coordinator hot-swap vehicle (admin `update_coordinators`, fleet driver) | LANDED 2026-08-31 |
| 2 | Conductor split into its own workload (`--admin-url` external attach) | LANDED 2026-09-01 |
| 3 | Staggered conductor rolls (bounded per-peer windows, genesis anchor last) | LANDED 2026-09-01 |
| 4 | Config as runtime surface (watched file, seconds, same PID) | LANDED 2026-09-01 |
| 5 | **This spec** — artifacts as elected content + upgrade/revert over p2p | Draft |
| 6 | DNA lineage migration (integrity-hash moves; constitutional) | Fenced (see below) |

Rung 6 — integrity/DNA-hash moves AND breaking canonical-bytes/wire-format
migrations (a format migration IS a lineage migration, per the wire-format
constraint in `governance-native-dna-upgrade-path.md`) — is deliberately OUT of
scope: the `CloseChain`/`OpenChain` + `InitProperties` migration window, whose
DoD lives in that backlog item. This spec governs every change class ABOVE the
DNA line: coordinator wasm bundles, hApp bundles for fresh joiners,
config-as-EPR, and (deferred, §9) native binaries.

Throughout, **"the companion"** names
`genesis/docs/content/elohim-protocol/architecture/2026-07-14-upgrade-revert-and-constitutional-consensus.md`
— the constitutional/agentic layer this spec implements a slice of. **"The
elohim"** in the governance sense is that document's term: the protocol's AI
agents, running on participants' own hardware, whose ceiling of authority is
*earned* (safety + wisdom benchmarks) and held only as bounded, revocable,
attested commitments — never a standing key (companion §3). **Rakia** is the
distributed build substrate (`elohim/rakia/` — declarative build manifests →
dependency graph → content-addressed, attested artifacts).

## 1. Compose, don't build — the fifth instance

The identity-head spec's rule binds here: a runtime head is **not a new
primitive**. It is the fifth instance of the declared-head-over-lineage-DAG
shape — the four that spec's §1 table proves (content lens, provenance, REA
collective, key rotation; that spec unified them as its own "third named
instance") — and it reuses the mesh-proven content election chain END TO END:

| Leg | Wired surface (verified 2026-09-01 grounding) |
|---|---|
| Declare | `content_store::declare_canonical_content_head` / `declare_earned_canonical_head` (earned-authority gate, three arms: root author; a device carrying the root author's signed `HeadDelegation`; or the bootstrap steward/progenitor — the §9 MVP authority) |
| Elect | `select_canonical_winner` → `seam_contracts::election::select_arbitrated_winner` — ANY earned beats ALL staging, newest-within-tier, link-hash tiebreak; partition-safe convergence |
| Carry + verify | declare-carries-Record, `validate_carried_head_record`, tamper-refused; proven cross-peer (`genesis/a2o/scripts/carried-election-mesh-proof.ts`) |
| Anti-self-election | `services/head_adoption.rs` four arms (LOCAL-DHT / PEER-HINT / AUTHOR-THEN-ADOPT / CONTEST-THEN-OBEY) |
| Project | `StampMode::{Declare,GapFill,HealCanonical}` + `canonical_move_verdict` monotonicity; the substrate trust contract's invariants — **I1** verification terminates in the receiving peer's own conductor (announcements are doorbells; no head is ever adopted from a gossip/HTTP payload), **I2** canonical channels alone move a DECLARED head (heal/boot paths fill, never move), **I3** a conductor resolve names its own authority (non-canonical answers never displace declared rows) |
| Bytes | the blob plane — content-agnostic PUT, shard bands to 64 MiB with RS parity above, and the native iroh-blobs ALPN leg (BLAKE3-verified streaming, no application cap) for large artifacts |
| Apply | rung-1 coordinator hot-swap (`happ_manager::sync_coordinators`, per-role DNA-lineage guard, embedded AND external conductors, `POST /admin/coordinators/sync`); rung-4 runtime-config reload |
| Observe | runtime passport `GET /version` (per-role DNA hashes + coordinator wasm hashes + boot flags); `/p2p/status` `irohPeers` user-agent overlay; `version-matrix --observed` |

**What is genuinely new is ONE component** — the adoption controller (§6) — plus
a release-manifest schema (§5) and the a2o receipt (§10). The MVP requires
**zero zome changes on the release/channel plane** (§5's `metadata_json` valve;
attestations likewise ride an existing generated kind — the first-class
build/soak kinds are a hash-moving codegen change batched into §11.1).
Everything new is storage-side, DNA-hash-NEUTRAL.

## 2. Upstream position (researched 2026-09-01)

Holochain 0.6.x/0.7 supplies three rails and no solution: `update_coordinators`
(mature — rung 1 rides it), clone cells (peer-local epochs), and
`InitProperties` + `MigrationTarget` on `CloseChain`/`OpenChain` (0.6.2/0.7 —
rung 6's rails). `lineage:`/`GetCompatibleCells` remain gated behind
`unstable-migration` (viable for us: we ship a forked conductor). DPKI was
removed in 0.6 — key continuity is protocol-owned: the identity-head spec owns
the rotation axis, and its §4.4 names the cross-DNA reinstall leg a hard
follow-on that rung 6 must land with. **No ecosystem project (Moss, Kangaroo,
Holo) has mixed-version fleets working; the ecosystem norm is "everyone updates
together."** Mixed-version operation — the additive-`serde(default)` wire
discipline + the sync-state epoch contract + the election window — is ours,
and it is the differentiating piece.

## 3. Channels, releases, and the reach axis

A **release channel** is a content identity (a content id under the existing
`Content` entry type) whose versions are release manifests and whose canonical
head is the channel's current release. Channel id convention:

```
runtime:<artifact-class>:<network>:<channel-name>
  runtime:coordinators:elohim:commons        (the constitutional head)
  runtime:coordinators:elohim:canary-a       (low-reach experiment)
  runtime:config:elohim:commons
  runtime:happ:elohim:commons                (install source for joined peers; §6 bootstrap caveat)
```

Channels are **reach-scoped from birth**: the channel's declared reach names its
audience (commons channel at commons reach; experiment channels at
self/trusted reach), and the reach spec's **narrow-never-widen law applies to
runtime heads verbatim** — an experiment can never widen its own audience;
promotion upward is a ceremony act carrying evidence. Within a channel the
existing two-tier election (staging < earned) is the promotion ladder — this is
rakia's discovery 1 ("reach IS deployment promotion": self → trusted →
community → public) completing its own thought.

**A/B and low-reach experiments are sibling channels**, not competing
declarations: two variants soak as two low-reach channels adopted by disjoint
canary sets; the election window is this layer's analog of the controlled trial
the companion §9 designed on the dual-conductor window (v-old as control arm).
Concurrent staging declarations on ONE channel deterministically resolve to one
winner by the arbiter — that is convergence, not experimentation; experiments
get their own channel.

## 4. Constitutional posture — stewarded auto-adoption

Items 1-3 and 5 record the arc doc's 2026-09-01 operator course-set essence;
item 4 and the scheduling-residual clause in item 2 are **this spec's proposed
enforcement shape**, accepted or rejected via the graduation-trigger.

1. **Upgrade stewardship is a domain of the NETWORK itself** *(recorded
   essence)*. Protocols are by nature monopoly power; that apex power is
   deliberately vested one degree removed from humans — in the elohim, at the
   constitutional level. The Chrome-OTA class of convenience+security power is
   wielded THERE. The authority that declares an earned head is itself bounded,
   revocable, attested (the compute-commitment/`HeadDelegation` rails),
   auditable and escalatable; the eternity clause (companion §10) is what makes
   vesting it safe — no upgrade may remove the ability to observe and revert
   (the method half of the clause; the dignity floor is the other).
2. **The default posture is automatic adoption** *(recorded essence; the
   residual clause is proposed)*. The runtime tracks its channel's earned head
   as a constitutional duty, the way it carries validation rules. Consent is
   exercised through governance — meaningfully heard, decisions explained,
   intimate context weighed with the whole context in good faith — NOT as a
   per-node veto on staying current. *Proposed:* the per-peer residual is
   intimate-context scheduling (maintenance windows, bandwidth), mediated and
   explainable, never a veto. Scope note: the companion §1's two-conductor
   covenant **stands verbatim at the DNA seam** (rung 6), where per-peer
   consent is topologically enforced; this section EXTENDS the consent
   doctrine to the above-the-line classes §1's mechanic never governed — the
   ceremony consents on behalf of the network; peers consent by belonging,
   diverge by declaration, and are heard through governance. This forecloses
   generalizing "propagation is consent" into a per-node veto for
   above-the-line classes.
3. **Divergence taxonomy** *(recorded essence)* — only one branch is refusal,
   and it is legible:
   - *lag within the window*: normal; the mixed-version wire discipline exists
     for it;
   - *declared fork*: legitimate — a lineage record, a seed/clone split, and a
     reconciliation map back (the bridge, companion §7);
   - *silent staleness*: a **harm class the network heals** (like a missing
     blob), never a freedom. The never-updating household member is exposed,
     not sovereign.
4. **Enforcement — proposed shape** *(this spec's design, open at §11.3)*: no
   one can push bytes onto a peer's hardware, and no one needs to. The DHT
   already enforces the hard version (DNA hash = network identity). Rung 5
   gives the soft layers the same shape: **protocol currency is attached to
   participation standing** — a peer materially behind the earned head loses
   serving authority for commons traffic, notary standing, doorway roles. Not
   punishment; coherence. You can always run what you want; you can't call it
   the network.
5. **The unity is not arbitrary** *(recorded essence — a values disclosure,
   the operator's stated conviction; the functional argument for a floor of
   agreement is §8's)*: because we are all created in the imago dei we all
   share something, so we all have to agree SOMEWHERE. The compatibility
   envelope (§8) — how robustly it can support, extend, and afford diversity,
   and bring reconciliation back, as exercised by its own evolution for good
   or ill — is where this system closes. This decision was deliberately made
   late, on top of the proven election machinery, and it is the right and only
   approach we can possibly support.

The proof case is the stewarded household node: it stays current because the
elohim steward it — no update prompts; protections are structural (canary soak
before it moves, revert that needs nothing from it, an explanation channel, and
escalation reach if intimate context was mishandled). The voice is real; the
veto over staying current was never the protection — it was the exposure.

## 5. P2P design gate (run 2026-09-01; back-fill answers forward)

**Entity: ReleaseManifest** (one per version; the channel is its content id)
- Classification: **Notarized (A), reusing the existing `Content` entry type —
  no new entry type, DNA-hash-NEUTRAL.** MVP rides an existing broad
  contentType with a `metadata_json` discriminator (`kind:
  "release-manifest"`) — the designed extension valve (dna-upgrade-governance
  §1 row notes: metadata_json contents are data, not code). Whether a
  first-class `runtime-artifact` contentType is worth batching into the next
  constitutional DNA change is an open question (§11.1).
- Body (canonical JSON, schema home proposed at
  `elohim/rakia/schemas/v1/release-manifest.schema.json`, rakia being the
  build-substrate seam): artifact blob CIDs + sizes; artifact class; per-role
  DNA hashes + coordinator wasm hashes it applies to; **compatibility
  envelope** (§8) — wire epoch, additive-wire floor, lineage parent (the
  previous release CID — a hint verified against the channel's L2 version
  chain, §8); build provenance (builder agent, toolchain,
  **the binary's own build-info**, closing the base-vs-fork axis the
  conductor-pin-ships-base-binary incident opened — never trust the pin tag);
  channel id + declared reach.
- Head-plane cost: channels are the heads — a handful per network; releases
  are versions under an existing per-channel head. Dozens of releases/year per
  channel; trivially under the ~500 bundling threshold (composite root shape).
- Address: Content-Derived CID (CIDv1 over the content bytes — distinct from
  the Holochain EntryHash the conductor mints for the entry; the projection's
  `dht_anchor_hash` carries the action hash). **Which release applies is a
  DECLARED head, never recency** — a consumer pins the head it depends on the
  way a lockfile pins a dependency.
- Transport affinity: iroh for artifact bytes (iroh-blobs ALPN leg; coordinator
  bundles run 1-64 MiB); the manifest itself is ordinary content + blob.
- Stakes: all four stages; **artifact verification is floor-protected
  (Constitutional) — it never cheapens, including at Simulacra.**
- Coordinator: existing `content_store` authoring + `declare_canonical_content_head`
  → EntryHash (cid) with action hash as `dht_anchor_hash` only. No new route;
  admin surfaces are node-local (§6), excluded from `build_manifest()` exactly
  as `POST /admin/coordinators/sync` is.

**Entity: BuildAttestation / SoakAttestation**
- Classification: **Notarized (A) riding an EXISTING generated attestation
  kind** on the elohim DNA with a `metadata_json` kind discriminator — the
  same valve as ReleaseManifest. The generated `ATTESTATION_KINDS` list is
  compiled INTO the integrity zome (`attestation_validator.rs` floor 1 refuses
  unknown kinds), so minting first-class `attestation:build-provenance` /
  `attestation:soak` kinds IS a hash-moving manifest+codegen change — batched
  into §11.1's constitutional DNA change, not the MVP. The exact existing kind
  is chosen at implementation from `generated_attestation_kinds.rs`.
- Identity: agent-scoped composite (agent × artifact CID × kind).
- Body carries **context**: hardware profile / device archetype, region/hub,
  probe results. Rakia's Stage-2 canopy plan
  (`elohim/rakia/docs/plans/stage-2-canopy.md`) makes the principle binding:
  attestation doesn't require bit-identical reproducibility — two peers
  producing different output CIDs for the same inputs is information, not
  failure. Context-bearing attestations are what let a regional channel elect
  the head that FITS while the commons head holds the envelope.
- These are the evidence that moves staging → earned. A builder's own
  attestation never suffices to earn its release (C1).

**Entity: AdoptionDiscipline** (soak windows, canary ordering, attestation
thresholds, rollout waves — per channel)
- Classification: **constitutional artifact of the ceremony** — notarized
  alongside the channel (fields of the channel's root content / release
  manifests), NOT per-peer preference. Reclassified from Private(B) by the
  2026-09-01 course-set.

**Entity: AdoptionState** (what this peer's controller is doing now)
- Classification: **Ephemeral (C)** — SQLite/in-memory, reconstructable;
  surfaced through the runtime passport (`/version`), which stays Category-C,
  node-local, never notarized, never gossiped as authority. Fleet visibility
  stays observational (`version-matrix --observed`).

**P2P design gate back-fill check** (the gate's three reverse-proof questions):
(1) the coordinator returns the manifest **EntryHash**; node-local admin
surfaces address channels by channel id and resolve heads through the
conductor — no route accepts a hash the coordinator didn't return. (2)
Integrity zome: `content_store_integrity` (the elohim DNA, packed from
`dna/elohim/`) — untouched on the MVP plane; the first-class contentType +
attestation kinds are declared hash-moving and batched (§11.1). (3) 1-year
item count: ≤ ~10 channels × ≤ ~50 releases + bounded attestations — no
measurable quiesce delta; channels add single-digit heads to a sweep that
prices thousands.

## 6. The adoption controller (the one new component)

A storage-side reconciliation loop (P1: k8s-controller-shaped — the DHT is the
manifest, the controller eagerly reconciles), proposed home
`elohim/elohim-storage/src/services/release_adoption.rs`:

1. **Watch** — resolve the canonical head for each channel this peer follows
   *through its own conductor* (I1: a `ContentHeadDeclared` signal or peer
   hint only triggers a verified local resolve — never adoption from a
   payload).
2. **Fetch** — artifact bytes by CID over the blob plane
   (`peer_blob_inventory` evidence-ordered fetch; REA serve events already
   flow).
3. **Verify locally (floor-protected)** — manifest schema; blob CID match;
   **envelope check** (§8) against this node's installed reality (runtime
   passport: per-role DNA hashes, coordinator wasm hashes) — the same per-role
   lineage refusal `happ_manager` already enforces, moved to verify time;
   attestation threshold per the channel's AdoptionDiscipline. The
   lineage-window check verifies the manifest's declared parent against the
   channel's **L2 version chain** (the content_id anchor) — the body field is
   a hint that must match; a mismatch is a typed refusal, never an accepted
   envelope.
4. **Apply** via the existing vehicle per artifact class — coordinator bundle →
   the `sync_coordinators` apply path (~2 min, mesh-proven ×3, embedded and
   external conductors); config EPR → runtime-config reload (seconds); happ
   bundle → the install path for **already-joined or re-installing peers**.
   *Bootstrap caveat (I1 boundary):* a FRESH joiner has no cell on the
   network's DNA yet and structurally cannot perform the verified local
   resolve for the very channel that supplies the DNA — its first bundle is
   seeded out-of-band (a pinned, content-addressed bundle + channel-id trust
   anchor, as `join-peer` requires today); only AFTER joining does the
   controller converge coordinator/config layers via I1-compliant resolves.
5. **Attest** the outcome — soak probes green → SoakAttestation; failure →
   typed refusal + the evidence that feeds contest/revert. Every arm carries a
   typed reason and a per-decision metric (C8), the
   `elohim_content_election_*` pattern extended to adoption.

**Revert is free by construction**: the ceremony declares a prior head
canonical; every controller converges backward through the identical loop. No
separate mechanism, no operator flag — this is what retires the
`ALLOW_DNA_REINSTALL`-class operator fork for everything above the DNA line,
and (with rung 6) the last out-of-band reset.

## 7. Concern-canon disposition

The concern canon is the repo's register of sixteen recurring failure classes
(C0-C14, C6 split into C6a/C6b) that every new decision surface must answer at
birth (`.claude/epr-meta/policies.yaml` + `concerns.yaml`; states:
answered / partial / unbound / n-a). Condensed disposition:

- **C0 plane** — answered by construction: election/authority at L2 (DHT);
  bytes on the data plane; AdoptionState is projection (the
  version-DAG-at-L2 law).
- **C1 anti-self-election** — answered: adopt-before-author +
  contest-then-obey wired; a release's builder cannot earn it with its own
  attestation; earned declarations gate on the three-arm authority (§1).
- **C2 monotonic authority** — answered: `canonical_move_verdict` replays the
  arbiter; staging never displaces earned.
- **C3 liveness** — answered: a channel with no earned head leaves the
  controller idle, never guessing.
- **C4 honest absence** — answered: `tier: none` is reported honestly; no
  head ≠ latest.
- **C5 evidence-not-authority** — answered: attestations and peer hints are
  evidence; authority terminates in each peer's conductor resolve.
- **C6a bounded work** — partial: design answer stated (bounded fetch/apply
  per sweep, finite backoff); proof lands with the controller's contract
  tests.
- **C6b idempotent effect** — partial: idempotent on (channel, release CID);
  contract-tested at implementation.
- **C7 advertise/serve symmetry** — answered by reuse: the existing blob
  inventory discipline.
- **C8 observability-per-decision** — partial: typed reasons on every
  adopt/refuse/revert arm + metrics; registered in `seam-registry.yaml` at
  birth.
- **C9 identity/lineage continuity** — answered for this rung: per-role
  DNA-hash guard at verify time; release lineage verified against the L2
  chain; key continuity's cross-DNA leg is rung 6's (identity-head spec
  §4.4).
- **C10 contract evolution** — answered: the envelope IS the answer (additive
  wire discipline + epoch declared in the manifest).
- **C11 externally-imposed backpressure** — partial: adoption scheduling
  defers to ram-guard/PVC/quiesce state so a peer under pressure
  lags-within-window rather than churns; proof at implementation.
- **C12 consent/authorization** — answered by design: §4 — constitutional
  consent, delegated declare authority, per-peer scheduling residual.
- **C13 graduated authority** — answered: staging → earned; reach-scoped
  channels; elohim ceiling authority bounded/revocable.
- **C14 witnessed residual** — partial: refused/diverged states are visible
  (observed matrix + attested refusals) — until the fleet matrix receipt
  lands.

## 8. The compatibility envelope — where unity is enforced

Variety lives ABOVE the envelope; unity AT it. The envelope is
machine-checkable at verify time from the manifest:

1. **Wire epoch** — the sync-state contract (epoch before position); a release
   declares the epochs it speaks.
2. **Additive-wire floor** — `serde(default)` discipline; a release may add,
   never remove/repurpose, within a lineage window. The window is bounded by
   the channel's **L2 version chain**, not by the manifest's self-declared
   parent (§6.3). A removal/repurposing beyond the additive floor is either a
   declared fork or rung 6's migration ceremony — never an accepted envelope.
3. **DNA line** — per-role integrity hashes the release binds to; crossing it
   is rung 6's ceremony, structurally refused here (the `happ_manager`
   lineage-guard rule at verify time).
4. **Floor-protected verification** — never stage-priced.

5. **Declared requirements** — a release may require another channel's head
   (`envelope.requires`); adoption waits until the node's own passports satisfy it. See §12.3.

A branch inside the envelope is ecology; a branch that breaks it is a declared
fork with a bridge map, a rung-6 migration, or it is not the network. What
works flows UP through hubs (soak evidence over the recursive rollup seam) and
back DOWN as context-fitted channel heads — the viable system model (VSM,
after Stafford Beer — the recursion lens the weave epic already applies to the
protocol) exercising itself: S1 peers running variants · S2 the wire
discipline (anti-oscillation) · S3 adoption controllers (operations) · S4 the
experiment window + soak evidence (the network learning about itself) · S5 the
constitutional election. This composes with the weave epic's VSM-recursion
subsystem rather than inventing a parallel structure.

## 9. MVP cut and non-goals

**In (MVP, mesh-first):** coordinator-bundle channel + config channel +
happ-bundle channel for joined peers; **the storage-binary channel for the
LOCAL and MESH rungs only** — the binary packaged as an EPR object on the same
dataplane (inheriting resiliency, replication, reach — all substrate
primitives come along) and applied via the proven exe-slot swap
(`hc-mesh.sh restart_storage`), under the developer/test-fixture trust context
(Simulacra stakes, declared); the adoption controller; release-manifest
schema; context-bearing attestations (riding an existing kind, §5); the a2o
receipt (§10). Three things must be in the bones day one so the ecology is
never precluded: channel ids carry reach scope from birth; manifests carry the
envelope declaration; attestations carry context fields.

**Out (deliberately):**
- **FLEET binaries self-updating over p2p** — a fleet binary replacing itself
  is a bigger safety bite; fleet binaries stay on the now-cheap staggered,
  conductor-preserving k8s roll until the coordinator-class loop and the
  local/mesh binary rung have soak. Revisit with steward/node as the update
  agent (§11.5). The graduated ladder (operator DoD, 2026-09-01): **local →
  hybrid (the T3 workspace conductor rung) → mesh → cluster** — each rung's
  receipt is the next rung's admission.
- **Rung 6** — integrity/DNA-hash moves and breaking canonical-bytes
  migrations; fenced to `governance-native-dna-upgrade-path.md`.
- **The full consensus/psephos ceremony** — MVP declare authority is the
  bootstrap-steward + `HeadDelegation` rail already in the zome; the
  deliberative ceremony grows into it (companion §5) without changing the
  substrate shape.

## 10. Definition of done (the receipt)

The operator's DoD framing (2026-09-01): **in the same way we — as developers
authorized as matthew's runtime/device — drive the ceremony that converges
peers on an elected head for epr-content** (synced, picked up, and served in
projection by doorways), we package our binaries as EPR objects on the same
dataplane, each component of the stack discovers them, and — leveraging the
trust the developer/test-fixture context provides — updates its own runtime
from those DHT-signed, attested packages. Driven **locally first, then hybrid,
then mesh, then eventually the whole cluster.** Long term, the Jenkins
pipeline becomes an **external observer** of the CI/CD process (the
build-system roadmap's Stage-3 end-state), and the k8s cluster ceases to be
needed as the operations plane.

The a2o story, `@concern:runtime-upgrade-propagation`, on the 3-peer mesh
(fleet confirms, never discovers): publish a coordinator release to a low-reach
channel → staging election converges on all peers → canary adopts + attests
(context-bearing) → promotion ceremony declares earned on the commons channel →
fleet controllers converge (conductor PIDs unchanged; ~2 min class) → **revert
by re-election** converges back → the observed version matrix shows every
transition. Cycle-time delta recorded in the arc doc's table (the arc's own
measure). One peer rides an experiment channel throughout — compatible,
divergent, and heard — proving both halves at once: the protocol stewarding
itself, and the diversity that teaches it.

**Implementation decomposition** — six discrete, disjoint, connected backlog
atoms (cluster `arch-dataplane-refactor-backlog`, each claimable by an
implementation agent; dependency edges declared in each):
`task-release-manifest-schema-packager` · `task-release-channel-ceremony-driver`
· `task-release-adoption-controller-observe` · `task-release-apply-vehicles`
· `task-release-soak-attestation-rail` · `task-runtime-upgrade-a2o-receipt`.

## 11. Open questions

1. The constitutional DNA-change batch: first-class `runtime-artifact`
   contentType AND first-class `attestation:build-provenance` /
   `attestation:soak` kinds (both hash-moving codegen changes) — decide when
   rung 6's first governed DNA change is batched.
2. Attestation threshold semantics for earned promotion (count vs diversity —
   device-archetype/region spread) — the AdoptionDiscipline schema owns this;
   start count-based, design the field for diversity.
3. Participation-standing enforcement (§4.4) — which roles gate on currency
   first (notary? commons serving?), and the grace-window shape.
4. The fresh-joiner bootstrap trust anchor (§6.4) — the exact shape of the
   pinned bundle + channel-id anchor (content-addressed seed file? doorway
   handoff?), and whether the mesh harness or the controller owns the
   post-join convergence handshake.
5. When binaries come in scope: is steward/node the update agent (it owns the
   process), with elohim-storage attesting?

## 12. Brit composition — one evolution primitive for every EPR; CI publishes, it does not deliver

*Added 2026-09-23, Draft. Composed from brit's designed model and from the CI wall-clock
investigation of 2026-09-22 (orchestrator #1884-#1891: DNA → edge → app ran in series, and
app delivered in 0 of 8 dispatches). §1-§11 stand unchanged. This section adds the frame
they compose into.*

### 12.1 The recognition: brit's deferred layer already runs in the network

brit (`elohim/brit`, the covenantal VCS) designs version control as EPRs:
- a branch is `{stable id, head, steward, reach}`;
- a ref update is a chained, authority-gated record ("a ref update without qahal authority
  is a protocol violation");
- a merge is a proposal whose consent rules are read from the parent EPR;
- a fork is "a legitimate new covenant, not a defection";
- build, deploy and validation are attestations of one shape.

The layer that makes that model live is DEFERRED in brit: the commit-like head-able node
with `parent`, signed heads, and head election. What is BUILT there is content addressing
(`BritCid` = CIDv1 dag-cbor sha2-256, byte-identical to `elohim-epr`), directory seals
(`EprMeta`), and signed Build/Deploy/Validation attestation nodes indexed by notes refs.

That deferred layer already exists in the network, and §1 names it: the
declared-head-over-lineage DAG on `content_store` (`declare_canonical_content_head` /
`declare_earned_canonical_head`, the lineage-parent admission rule, the arbitrated election).
brit's own design says its `HeadDeclaration` is "the same shape as the live substrate rule
that canonical channels alone move declared heads".

| brit concept (designed) | Network primitive (running) |
|---|---|
| Branch `{id, head, steward, reach}` | Release channel / content id with a canonical head and a declared reach (§3) |
| `RefUpdate` chain, authority-gated | Head declarations chained by `lineageParentCid`, admitted by the zome; canonical channels alone move a declared head (I2) |
| Merge proposal, consent from the parent EPR | Promotion = earned declaration under the parent's authority: constitutional for runtime (§4, epic §4.1), steward/collective for content |
| Fork = legitimate covenant | Sibling channel with a declared divergence (§8's "declared fork with a bridge map") |
| Build / Deploy / Validation attestation | Provenance + publish / adoption event / soak attestation (§5, §6) |
| Reach lifecycle self → trusted → community → public | Local draft → staging tier at channel reach → earned tier → widened reach (narrow-never-widen, §3) |

**The decision:**
- brit does NOT build a second version DAG in git notes.
- Below the publish line (reach `self`/`intimate`: drafts, local branches), brit keeps local
  heads and seals. That costs nothing on the DHT.
- Publishing makes brit's local head CID a `Content` version plus a staging declaration on
  the EPR's channel. From then on the network's election is the version DAG. brit is its
  canonical-first client.
- Git stays the bridge for source files: an `EprMeta` seal CID names a source tree, and builds
  and releases cite that seal as their input.

So "rung 5" is not a runtime special case. It is the fifth instance (§1) of the primitive by
which **any EPR** evolves: a learning path, a governance document, an app bundle, a schema,
or the network's own runtime. Versions are content, heads are declared per branch, promotion
is earned by attestation, consumers pin heads as dependencies, forks are sibling channels,
and revert moves the head back. The SDK exposes one primitive, and the network's own upgrades
dogfood it.

### 12.2 Change classes are channels; routing is by class, not by pipeline order

Each artifact class is a channel of that primitive. The adoption controller's vehicle for the
class (§6, `release_adoption/apply.rs`) is the only class-specific code.

**Current state per class (2026-09-23 survey):**

| Class | Current state |
|---|---|
| coordinator | VERIFIED on mesh. On fleet, verified to canary apply + attest (2026-09-06). Six alpha peers follow at `observe`, so promotion moves nobody yet. |
| config | Watcher VERIFIED on mesh. `config-epr` vehicle BUILT; see the fleet mount caveat under 12.7. |
| happ-lineage | Mesh stations 1-5 and 7-10 green. Station 6 red. |
| storage-binary | BUILT; Simulacra-gated. |
| SPA / app bundle | No class (see below). |
| doorway binary | No class (see below). |
| conductor line | No vehicle (Nachalah slice 3). |

**The SPA already delivers as a content head.** CI PUTs the bytes, PATCHes the pillar EPR's
`blobHash` once, and fans out staging `canonical-head` declarations. But CI authors that head
with an admin key; there is no manifest, envelope, canary or earned tier. Making the app
bundle a class means doorways adopt it from the app EPR's own channel. The app pipeline then
publishes a candidate instead of steering every doorway. The app is an ordinary EPR evolving
by the §12.1 primitive, which is the "any EPR" claim proven on the protocol's own front door.

The doorway binary gets a class beside `storage-binary`, with the same stage-slot vehicle and
the same ladder gate (local → T3 → mesh → cluster; the §9 cut is a gate on that ladder, never
an exclusion — operator ruling 2026-09-06).

### 12.3 Cross-class ordering is a declared requirement, not a pipeline sequence

The app pipeline was ordered behind edge (commit `8ebce05a3`, 09-14) to express one fact: this
app build needs a storage version that speaks the new head contract. That is a compatibility
requirement, and encoding it as pipeline order is what starved app delivery. §8 gains a fifth
envelope item:

5. **Declared requirements** — `envelope.requires: [{ channel, atLeast: <manifestCid> }]`.
   - **Satisfied** iff the adopting node's own runtime passport, plus the passports of the
     co-located components it declares (a doorway declares its backing storage), shows an
     applied release on `channel` whose L2 version chain contains `atLeast`.
   - **Unsatisfied** → a typed refusal `requirement_unmet { channel, atLeast, installed }`.
     The controller re-checks on its next sweep: a bounded wait, never a push.
   - A requirement never causes the other class to be adopted. It only defers this one. That
     keeps the anti-self-election rule (C1) and never lets a release force a peer's hand.
   - Additive schema, carried in `metadata_json`: DNA-hash-NEUTRAL.

With requirements carried by artifacts, the CI dependency edge app → edge is deleted: the
build DAG orders builds only by build inputs (rakia's `hash_inputs`), never by delivery. A
push's wall clock becomes its longest single build, not the sum of a delivery chain.

### 12.4 CI's role: builder, publisher, witness — never the delivery path

- **Build:** unchanged. rakia/Jenkins compile.
- **Publish:** CI runs `release-ceremony publish` into the class's channel at **staging** tier,
  at a reach no wider than the channel's, carrying the build attestation (12.5). This replaces
  rung 1's `fleet-coordswap-dispatch.sh` push, the SPA fan-out PATCH, and the edge fleet roll
  as delivery paths.
- **Never promote:** the earned tier requires soak attestations and excludes the builder
  (the `attestationThreshold` rule). CI cannot promote its own output. This is the brit
  principle "CI doesn't own governance".
- **Witness:** Dataplane Validation stops being a stage inside every edge build (measurement by
  deploy). It becomes a continuous observer that authors soak (Validation) attestations the
  promotion rule already reads. Delivery reads the latest attestation; it never produces one.

The k8s roll remains the vehicle only for what no vehicle carries yet: fleet binaries before
their ladder receipt, and the conductor line. For those, the 2026-09-22 content-keyed restart
decision (edge `pod-inputs-fingerprint.sh`, `storage-workload-image.sh`,
`conductor-happ-stamp.sh`) is the interim. The live-object annotations it records
(`elohim.host/storage-inputs`, `happ-roll-key`) are **scaffold**: they hold the facts a
release manifest's `artifacts` + `appliesTo` carry, and they retire when the class moves to
election. Do not extend them (the "k8s is not the architecture" rule).

### 12.5 One attestation schema, two homes

brit's `BuildAttestationContentNode`
`{manifestCid, stepName, inputsHash, outputCid, agentId, hardwareProfile, buildDurationMs,
builtAt, success, signature}` is adopted as the payload of a `release-build` discriminator,
riding the existing attestation kind exactly as soak evidence does (`attestation:device-health`
+ `release-soak`, per §5). Deploy maps to the adoption event. Validation maps to
`release-soak`.

Because `BritCid` and the network CID are byte-identical:
- a build attestation sealed locally (brit notes ref, reach `self`) and the same attestation
  published on the DHT are **one object at one address**;
- publishing is a reach change, not a copy.

Only attestations of published candidates enter the DHT; every other build's attestation
stays local.

### 12.6 Content-keyed memoization replaces commit-keyed baselines

The orchestrator's selection diffs from a held global baseline (`__global__`), so every
failed run re-selects everything since the last good commit. That is why DNA and edge
re-ran on every push from #1884 to #1891, even where edge had already shipped the range.
rakia-brit's baselines as git refs are still commit-keyed, and brit's own build-contract
spec names commit-keyed baselines as the root cause of over-building.

**The primitive:**
- A step's input CID is the `EprMeta` seal of its declared inputs (rakia `hash_inputs`).
- A step is stale iff no successful build attestation exists for that input CID.
- The index (input CID → attestation CID) is **Ephemeral (C)**, rebuilt from local brit
  notes refs and published `release-build` attestations.
- Baselines — a record of the last commit that built — are no longer needed at all: the
  question becomes "has this exact input been built?", answered by content.

### 12.7 P2P design gate (2026-09-23; answers forward)

**Release requirement (`envelope.requires`)**
- **Classification:** part of the release manifest's content (Notarized A via `Content`). Not
  a new entity.
- **Head plane:** +0 heads.
- **Address:** inside the manifest CID.
- **DNA:** `content_store_integrity` untouched; DNA-hash-NEUTRAL.
- **Stakes:** all four stages; verification is floor-protected.

**App-bundle and doorway-binary classes**
- **Classification:** Notarized A. Versions are `Content` under one channel content id, so a
  composite root: one head per channel.
- **Head plane:** about 0.5-1k versions a year per channel, all under that single head.
- **Bytes:** blob-plane `bafkrei…`. Transport `auto`, with iroh-blobs for large artifacts.
- **DNA:** hash-NEUTRAL.

**`release-build` attestation**
- **Classification:** Linked A2 on the release. It rides the existing attestation kind with a
  discriminator; no new type.
- **Head plane:** 1-3 per published release. Unpublished builds stay local (B).

**Published brit head**
- **Classification:** Notarized A. It *is* a `Content` version plus a declaration.
- **Head plane:** one head per published branch. Local branches cost nothing.

**Memoization index**
- **Classification:** Ephemeral C.
- **Reconstruction:** from notes refs and published attestations.

**Concern canon for the `requirement_unmet` predicate**
- C1 answered (defers only, never elects).
- C3 answered (bounded re-check each sweep).
- C4 answered (typed refusal names what is installed).
- C8 partial (needs a `/db/p2p/adoption` field).
- C10 answered (additive field; old controllers ignore it and adopt as today). The consequence
  is that the requirement is honored only by upgraded controllers, so the first `requires`
  release ships after the controller that reads it.
- The remaining classes are `n-a`: the predicate reads only local passports.

**SDO/RWA test**
- `hardwareProfile` in a published attestation can fingerprint participants (boundary 6).
- The published form carries a coarse device archetype only; the full profile stays in the
  local seal.
- A doorway projection aggregating every network's attestations would be a dragnet of who
  builds what and when. Attestations stay within their channel's reach, and projections carry
  a retention floor (boundary 2).

**Fleet caveat found in the 2026-09-23 survey (unverified, captured as backlog)**
- The runtime-config file is mounted `subPath` + `readOnly` in
  `_edgenode-consolidated.template.yaml`.
- k8s does not propagate ConfigMap edits into subPath mounts, and a read-only mount defeats
  `ConfigEprVehicle` and `/admin/runtime-config/follow` on fleet pods.
- So config-class election may be mesh-only until the mount changes.

### 12.8 Slices — each receipt admits the next

1. **Requirements.** `envelope.requires` in the schema + packager + the adoption controller's
   verify step; a mesh a2o station (a release requiring a storage version waits, then adopts
   when storage converges).
2. **App bundle as a class.** The app EPR's channel; doorways adopt; the app pipeline publishes
   at staging instead of the fan-out PATCH; then delete the app → edge `dependsOn`.
   Receipt: an app delivery with no edge in the push.
3. **CI publishes coordinators.** Replace the rung-1 push with a staging publish. Needs the
   operator's observe → apply flip for fleet peers (a `deployments.json` edit, reversible).
4. **Continuous witness.** Dataplane Validation runs as a scheduled observer authoring soak
   attestations; the per-edge-build stage is removed.
5. **Content-keyed memoization** (12.6); retire commit-keyed baselines and the 12.4
   annotations as each class moves.
6. **Binaries climb the ladder.** storage- and doorway-binary on local → T3 → mesh → cluster;
   the conductor line via Nachalah slice 3.
7. **The any-EPR surface.** Expose publish, promote, revert, fork and pin to app developers as
   brit verbs over any EPR; content-fork arbitration composes from
   `content-head-election-vs-reach-fork-arbitration.md`. Design only until 1-2 prove.

**Blockers to name, not design around**
- brit and rakia crates resolve only from the auth-gated `elohim` Nexus (HTTP 401 in the
  devspace); brit-side code cannot be built here until read access returns.
- Channel publish is "god-mode OPEN" (`authorize_canonical_head_declarer`, a labeled dev
  scaffold). CI publishing at staging is safe only because staging can never beat earned.
- §4's constitutional posture still awaits operator acceptance. 12.4's "never promote" holds
  regardless of how §4 settles.
