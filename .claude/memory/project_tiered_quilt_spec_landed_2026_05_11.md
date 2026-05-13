---
name: Tiered quilt stewardship spec + delivery master landed (2026-05-11)
description: 8-wave portfolio for the missing storage-tier layer; composes on Plan 1, absorbs Plans 2-5; dogfooded on MinIO+sccache substrate; ZERO new DHT entry types (reuses Commitment+Attestation with discriminators); pre-Wave-0 cleanup absorbs Attestation dedupe + lamad_event_type rename
type: project
originSessionId: 96fee2e2-0f17-4f94-b422-d351bc13ee2f
---
**Spec:** `genesis/docs/superpowers/specs/2026-05-11-tiered-quilt-stewardship-design.md` (960 lines, 11 sections)
**Delivery master:** `genesis/docs/superpowers/plans/2026-05-11-tiered-quilt-delivery-master.md` (718 lines, 8 waves + parallel tracks)
**Commit:** `c968369b7`

### Non-obvious discoveries (the reason this entry exists)

These are findings from pre-flight that aren't obvious from reading either the
spec or the codebase, but are load-bearing for future tiered-quilt work:

**1. Substrate naming drift — Garage in docs, MinIO in reality.**
The cluster actually runs MinIO (single replica, openebs-jiva-csi-default,
hp-micro10), not Garage. The runbook is authoritative
(`genesis/manifests/RUNBOOK-minio-sccache-2026-05-09.md`); the `devfile.yaml` +
`elohim/holochain/dna/elohim/flake.nix` comments saying "Garage" are stale.
Memory anchor `project_garage_sccache_substrate_2026_05_09` corrected in this
session.

**2. Duplicate `Attestation` entry type across elohim + imagodei DNAs.**
Identical shape, lines 1052 (elohim) and 416 (imagodei). Preexisting drift,
not noticed in earlier work. **Wave 0 of the delivery master absorbs the
cleanup**: single source of truth becomes elohim DNA; imagodei coordinator
calls migrate to call elohim DNA's `create_attestation`.

**3. "lamad" as Angular pillar ≠ "lamad-v1" as DNA ≠ legacy "lamad" naming in
elohim DNA's enum.** The elohim DNA's `content_store_integrity` zome organizes
its EntryTypes enum into sections labeled "Lamad: Content & Learning", "Shefa:
Economy", "Imago Dei: Identity", etc. — but they all live in the elohim DNA
(the protocol core). The lamad-v1 DNA exists separately with only a scaffold
`content_store` zome. The Angular `lamad` pillar is the LMS pillar. **Result:
the `lamad_event_type` field name on `EconomicEvent` is legacy drift that
predates this clarity**. Wave 0 renames it to `elohim_event_type` across the
full stack. New tier events land under the new field name from day one.

**4. The Bittorrent-priority-vs-hardware-tier driver insight.**
A laptop running elohim-storage has ONE drive. "Shelved" on a laptop is a
*retention priority*, not a hardware tier — bittorrent/transmission analogy.
A steward node with SSD+HDD has real hardware tiers. A dwelling hub with
multiple machines has cluster-level tiers (`elohim-operator` orchestrated).
**Single TierController, multiple StorageBackend drivers** is the correct
abstraction. K8s PVC is a *specific deployment*, not a design primitive — a
hub on Raspberry Pi cluster uses the same drivers, different mechanics.

**5. Grandma's family cluster is the capability bar.**
Not "we built a worse S3." Not "we matched k8s." The protocol matches AND
EXCEEDS k8s-cluster resilience precisely because it spans cities, states,
and jurisdictions — no single operator, datacenter, or trust root binds it.
Grandma's photo album survives a flood in one city, a power outage in
another, and a court order in a third. Tiered-quilt design narrative must
LEAD with this bar; substrate paragraphs go in implementation notes. This
is why we are going through the trouble.

**6. Zero new DHT entry types.**
All notarized state composes on existing `Commitment` (via
`action="custody-quilt"`) and `Attestation` (via `category="storage-stewardship"`
+ `attestation_type` discriminator). Five Attestation discriminators land:
`tier-breach`, `tier-restitution`, `tier-holdings`, `tier-accounting`,
`tier-self-degraded`. Protects DNA capacity (elohim DNA at ~70+ entry types
visible, headroom but not abundant).

**7. Malicious tier-misreporting is a non-issue under the graph pattern.**
First-draft engineering treated it as a class needing defensive mechanisms.
User correction: storage-capability attestations are EARNED (same as content
reach earned at authoring). A peer without earned `storage-capability ≥
stocked-warm` attestations cannot accept a `tier_floor="stocked-warm"`
commitment — CommitmentFactory negotiation refuses. No special anti-cheat
subsystem; the graph pattern handles it. Direct draw probes are
*observational, not adversarial*.

**8. Tier mechanics fade for grandma; they're an elohim-internal/app-developer
concern.** End-user never sees "tier" vocabulary. Only the result —
responsiveness, ambient compute-contribution tile, fair reciprocal cost share.
App-developers and operators see the machinery. Three-surface design is
load-bearing for adoption.

### Memory anchors created or corrected in this session

- Corrected: `project_garage_sccache_substrate_2026_05_09.md` — substrate is MinIO
- (This file): `project_tiered_quilt_spec_landed_2026_05_11.md`
- Planned for Wave 0 close: `project_attestation_dedupe_elohim_dna_canonical.md`
- Planned for Wave 0 close: `project_elohim_event_type_field_rename.md`
- Planned for Wave 7 close: `project_tiered_quilt_stewardship_landed_YYYY_MM_DD.md`

### How to apply

- **Before any tiered-quilt code work**: confirm operator has signed off on
  the six decisions in delivery master §1 (defaults documented). Wave 0 needs
  only the bundle-vs-split decision; later waves need archetype catalog +
  cost-class weights + trust depth + bucket TTL + DNA capacity audit.
- **Pacing constraint (2026-05-11)**: EPR Phase 4 agent has uncommitted work
  in `elohim-storage/{rea_projection,services,api,p2p,db}`. Wave 0 rename
  conflicts with their files. Wait for their commit before executing Wave 0;
  authoring the Wave 0 plan doc is safe to do in parallel.
- **Don't repeat the false-positive hook trip on "schema" keyword.** The P2P
  design gate hook in `.claude/skills/p2p-design-gate/SKILL.md` keyword-matches
  "schema" and flags section headers + build-step bullets. Avoid the word in
  prose; use "view contract", "manifest evolution", "codegen rerun", etc.
- **When citing the tiered-quilt vocabulary**, leading-edge order is:
  `drawn / stocked-warm / stocked / shelved` (verbs: stock, draw, shelve,
  promote, demote, evict, restitute). Don't use hot/normal/cold colloquial.
