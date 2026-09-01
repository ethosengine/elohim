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
topic: [upgrade-propagation, canonical-head, release-channel, adoption-controller, reach, vsm-ecology, rollback, dataplane]
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
happ-bundle channel for joined peers; the adoption controller; release-manifest
schema; context-bearing attestations (riding an existing kind, §5); the a2o
receipt (§10). Three things must be in the bones day one so the ecology is
never precluded: channel ids carry reach scope from birth; manifests carry the
envelope declaration; attestations carry context fields.

**Out (deliberately):**
- **Native binaries self-updating over p2p** — the mesh exe-slot mechanism
  (`hc-mesh.sh restart_storage`) is the local prior art, but a fleet binary
  replacing itself is a bigger safety bite; binaries stay on the now-cheap
  staggered, conductor-preserving k8s roll until the coordinator-class loop
  has fleet soak. Revisit with steward/node as the update agent (§11.5).
- **Rung 6** — integrity/DNA-hash moves and breaking canonical-bytes
  migrations; fenced to `governance-native-dna-upgrade-path.md`.
- **The full consensus/psephos ceremony** — MVP declare authority is the
  bootstrap-steward + `HeadDelegation` rail already in the zome; the
  deliberative ceremony grows into it (companion §5) without changing the
  substrate shape.

## 10. Definition of done (the receipt)

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
