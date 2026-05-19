# Qahal Collective + Membership DHT Design

**Status:** Design (pre-implementation). Implementation deferred to a follow-on plan that consumes this spec. Likely co-sequenced with the deferred Wave 3 hREA / VF-GraphQL brainstorm (`genesis/docs/plans/2026-04-21-rno-lessons-cross-wave-guidance.md`).

**Date:** 2026-05-19
**Plan kinship:** L7 of `genesis/docs/plans/2026-05-19-viewer-symmetry-reciprocity-qahal-substrate.md` — substrate groundwork for Epic E of `2026-05-19-topology-resilience-qahal-synthesis.md` and the deferred Wave 3 hREA interop work.

**Source references:**
- `.claude/skills/p2p-design-gate/SKILL.md` — gate questions this spec must answer
- `/projects/research/vf-graphql/lib/schemas/agent.gql` — canonical ValueFlows `Agent` / `Organization` / `AgentRelationship` types (cloned 2026-05-19 at `0a52dbe`)
- `genesis/docs/plans/2026-05-19-topology-resilience-qahal-synthesis.md` §Epic E (slide-45 anchored Collective view)
- `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs:3633` — current elohim DNA `EntryTypes` enum (37 entries; ~63 slots of headroom)
- `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs:888` — imagodei DNA `EntryTypes` enum (11 entries; ~89 slots of headroom). Existing `HumanRelationship` lives here; `Collective`/`Membership` will live here too (qahal-shaped relational identity, sibling to `HumanRelationship`).
- `.claude/memory/project_collective_is_stewardship_unit.md` — collective = stewardship unit (load if exists; otherwise the principle is summarized in §1 below)
- `.claude/memory/project_no_sovereignty_stewardship_over_ownership.md` — no `own*`, no `sovereign*`; use steward/contributor/authored vocabulary

## 1. Problem statement

Epic E of the synthesis plan promises a `/qahal/collective/:id` page with five facets (slide 45 of *After the Feed*): members, stewards, live co-presence, upcoming activities, visible-and-persistent norms, contribution recognition. All five facets must be backed by P2P-native DHT entries — every interaction in this page is a protocol primitive, not an Angular cache.

Today the DHT has no first-class `Collective` or `Membership` entries. The topology view derives household groupings from `HumanRelationship` edges, which doesn't compose for arbitrary collectives — a congregation, a learning cohort, a civic group, a household-as-collective. The peer-topology surface treats *households* as the resilience unit (`project_household_is_resilience_unit.md`); collectives are the next abstraction up: the stewardship unit (`project_collective_is_stewardship_unit.md`).

This spec proposes the missing two entry types plus the relational link kinds, the validation invariants, the coordinator functions, and the hREA `Organization` / `AgentRelationship` mapping that makes us legible to R&O-shaped clients (the deferred Wave 3 interop deliverable).

## 2. P2P Design Gate — answers

(Per the gate skill, these four questions are answered before any HTTP route is sketched.)

### 2.1 Source-of-truth classification

| Entity | Category | Rationale |
|---|---|---|
| `Collective` | **A — notarized** | Identity-of-record. The CID must be discoverable by any peer, rejoinable after key recovery, and cited from other notarized entries (Membership, future Stewardship, Agreement). |
| `Membership` | **A — notarized** | Authority-bearing — designating a steward is a governance act. Membership existence must be auditable across peer outages. |
| `MembershipRole` (steward / contributor / observer) | **derived via link kind**, not a separate entry | The role is an attribute of the Membership entry; iteration by role is via link kinds `HasSteward` / `HasContributor` / `HasObserver` on the Collective. |
| `ActivityPresence` (live co-presence — slide 45 facet 3) | **C — operational** | Heartbeat-shaped; rebuilt from libp2p signals; not notarized. Belongs in projection (elohim-storage), not on the DHT. **Out of scope for this spec.** |
| `Norm` (visible-and-persistent norms — slide 45 facet 1) | **A — notarized**, but **not part of this spec** | Norms are a distinct governance primitive — mishpat territory. This spec defers them; Membership references them indirectly via the Collective's charter text. A follow-on `Norm` design will handle attestation chains, revision history, and quorum gates. |
| `ContributionRecord` (contribution recognition — slide 45 facet 5) | already exists | The synthesis plan §2 references `ContributionRecord` and `EconomicEvent`; reuse the existing REA primitives. Not duplicated here. |

### 2.2 Existing entry types — does a fit already exist?

`HumanRelationship` (`elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs:371`) is the closest existing entry. It's *person-to-person*, optionally with intimacy level + custody flags. It can't carry a `Collective` since (a) collectives have a charter, a steward set, a member set with roles, and an identity distinct from any one person, and (b) collapsing Collective onto HumanRelationship would force a many-to-one star-graph encoding that fights the natural graph shape.

Conclusion: `Collective` and `Membership` are **new entry types** with no fit-by-collapse. Headroom in the imagodei DNA is ~89 slots; adding two is comfortably within budget.

### 2.3 Identity scheme — CID-derived, no slugs

- `Collective.id` = content CID derived from `{founder_agent_cid, charter_text, created_at_block_height, salt}`. The `salt` is required so two distinct collectives with the same charter authored by the same founder in the same block don't collide.
- `Membership.id` = content CID over `{person_cid, collective_cid, role, joined_at_block_height, sponsor_cid_or_null}`.
- Display name (`Collective.display_name`) is mutable via `update_collective`; the id is immutable. This matches the existing pattern for `Content.title` vs `Content.id`.

No slugs are introduced. The `:id` segment in `/qahal/collective/:id` is the CID.

### 2.4 Coordinator function map

| Operation | Coordinator zome | Integrity validation gate |
|---|---|---|
| `create_collective(charter, display_name)` | `qahal_coordinator` (new — co-located with imagodei coordinator in the same DNA) | Charter must be non-empty (max 16 KiB markdown); founder agent must sign; `display_name` non-empty; salt present. |
| `update_collective(collective_cid, new_display_name)` | `qahal_coordinator` | Caller must be a current steward; charter is immutable (only `display_name` may change in v1). |
| `request_membership(collective_cid, role)` | `qahal_coordinator` | Person CID must match the call origin; role ∈ {`Contributor`, `Observer`}; `Steward` role requires `sponsor_cid` set to an existing steward who must counter-attest within a follow-up coordinator call. |
| `attest_membership(membership_cid)` | `qahal_coordinator` | Caller must be an existing steward of the named collective; this is the counter-attestation that promotes a pending Steward-role membership to active. |
| `revoke_membership(membership_cid, reason)` | `qahal_coordinator` | Caller must be a current steward; `reason` non-empty; emits `ElohimContentSignal` for projection (the storage layer surfaces revocation in the Collective's audit log). |

The `Steward` role gate (sponsorship + counter-attestation) is the minimal authority constraint; quorum-of-stewards / threshold-N-of-M is deliberately deferred (see §8 open questions). The v1 design uses a single counter-attestation because most early Collectives have one or two stewards; threshold gates land when use surfaces the need.

## 3. Entry shapes (HDK)

The Rust struct + `#[hdk_entry_helper]` sketch — mirrors the existing `HumanRelationship` pattern (`elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs:371`). Per memory `feedback_serde_json_value_breaks_zome_boundary`, no `serde_json::Value` at the SerializedBytes boundary — every payload field is a primitive or a string.

```rust
#[hdk_entry_helper]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Collective {
    /// Founder agent CID. Immutable. Becomes the implicit initial steward
    /// (no separate Membership entry created at founding — the create_collective
    /// coordinator call also creates the founder's Steward Membership atomically).
    pub founder_agent_cid: String,
    /// Charter markdown. Immutable in v1. Max 16 KiB.
    pub charter: String,
    /// Human-readable name. Mutable via update_collective.
    pub display_name: String,
    /// Block height at which create_collective was called. Anchors CID derivation.
    pub created_at_block_height: u64,
    /// Random salt to disambiguate collectives with identical founder+charter
    /// authored in the same block. 16 bytes hex-encoded.
    pub salt: String,
}

#[hdk_entry_helper]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Membership {
    pub person_cid: String,
    pub collective_cid: String,
    pub role: MembershipRole,
    /// Set when role == Steward and the membership is still awaiting
    /// counter-attestation. Cleared once attest_membership lands.
    /// None for self-joined Contributor / Observer roles.
    pub sponsor_cid: Option<String>,
    pub joined_at_block_height: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MembershipRole {
    /// Authoritative — can attest peers, update_collective, revoke memberships.
    Steward,
    /// Participates in flows + governance signals but cannot revoke peers.
    Contributor,
    /// Read-only participation; lurker tier. May propose membership upgrade.
    Observer,
}
```

## 4. Link types

Per memory `project_hdi_no_get_links_in_validators`, link traversal is HDK-only; integrity uses `must_get_*`. Link types announced in the integrity zome:

| Link kind | From | To | Purpose |
|---|---|---|---|
| `MemberOf` | Person CID | Collective | Outbound discovery: "what collectives does this person participate in?" |
| `HasMember` | Collective | Person CID | Inbound iteration: "who's in this collective?" Tag carries `MembershipRole` for cheap filtering. |
| `HasMembership` | Collective | Membership entry | Anchored iteration with full Membership metadata (role + sponsor + joined-at), ordered by `joined_at_block_height` via Action timestamps. |
| `StewardOf` | Person CID | Collective | Cheap query: "what collectives does this person steward?" Redundant with `MemberOf` filtered by role tag, but spares the role-tag scan. |
| `CharterAnchor` | Anchor(`collective:<cid>`) | Collective | Well-known anchor so any peer can resolve a collective by CID without needing the founder's agent activity. |

## 5. Validation rules

Per `project_hdi_no_get_links_in_validators`, integrity validators **must not call `get_links`** — they use `must_get_action` / `must_get_entry`. Authority checks that require link traversal (e.g. "is the caller a current steward?") run in the **coordinator**, not the integrity validator. The integrity validator's job is structural: types, sizes, signatures, deterministic invariants over already-fetched entries.

| Rule | Where enforced | Notes |
|---|---|---|
| `Collective.charter` non-empty + ≤ 16 KiB | `validate_create_collective` integrity | Pure-data check. |
| `Collective.display_name` non-empty + ≤ 256 chars | `validate_create_collective` integrity | Pure-data. |
| `Collective.salt` is 16 bytes (32 hex chars) | `validate_create_collective` integrity | Pure-data. |
| Founder signature matches `founder_agent_cid` | `validate_create_collective` integrity | `must_get_action` on the create action's author. |
| `update_collective` may only change `display_name` | `validate_update_collective` integrity | Must-get the prior entry; compare field-by-field. Charter immutability invariant. |
| `update_collective` caller must be a current steward | `qahal_coordinator::update_collective` (NOT integrity) | Requires link traversal to find current stewards — coordinator-only. |
| `Membership.role == Steward` requires `sponsor_cid` to be a current steward of the named collective | `qahal_coordinator::request_membership` | Same reason — needs link traversal. The integrity rule is only "if role == Steward, sponsor_cid must be Some()". |
| `attest_membership` caller must be a current steward of the named collective | `qahal_coordinator::attest_membership` | Coordinator-only. |
| `revoke_membership` caller must be a current steward + `reason` non-empty | `qahal_coordinator::revoke_membership` | Coordinator-only. |
| Revoke action emits `ElohimContentSignal { kind: "qahal:membership-revoked", collective_cid, membership_cid, reason }` | `qahal_coordinator::revoke_membership` post-commit | Projection drives the audit log in the storage layer. |

## 6. hREA / ValueFlows mapping

Cross-referenced against `/projects/research/vf-graphql/lib/schemas/agent.gql` (cloned at `0a52dbe`). The mapping is the bridge that lets the deferred Wave 3 VF-GraphQL endpoint expose our Collectives + Memberships as canonical VF `Organization`s and `AgentRelationship`s without lossy translation.

| qahal entity / field | VF / hREA counterpart | Mapping notes |
|---|---|---|
| `Collective` | `Organization implements Agent` (`agent.gql:Organization`) | Identity-level peer of `Person`. Both are `Agent`s in VF terms — composable into `AgentRelationship`. |
| `Collective.id` (CID) | `Organization.id: ID!` | CID is opaque to VF; serializes as the GraphQL `ID!` scalar. No translation. |
| `Collective.display_name` | `Organization.name: String!` | Direct map. |
| `Collective.charter` | `Organization.note: String` (VF agent.gql has `note`) — and **also** surface as a separate `Collective.charter` extension field on the VF-GraphQL projection, since VF's `note` is loose-purposed and the charter is contractually different from a casual note. |
| `Collective.founder_agent_cid` | derivable: `AgentRelationship { object: Organization, subject: Person, relationship: Steward, primary: true }` for the founder at creation time | No direct VF field; project the founder as an initial steward relationship instead. |
| `Membership` (any role) | `AgentRelationship { object: Organization, subject: Person, relationship: AgentRelationshipRole }` | One-to-one. `Membership.id` ↔ `AgentRelationship.id`. |
| `MembershipRole::Steward` | `AgentRelationshipRole { label: "steward", inverseLabel: "stewarded_by" }` | The VF Role concept is open-vocabulary — we declare these labels and they pass through. |
| `MembershipRole::Contributor` | `AgentRelationshipRole { label: "contributor", inverseLabel: "contributed_to_by" }` | Same — declared. |
| `MembershipRole::Observer` | `AgentRelationshipRole { label: "observer", inverseLabel: "observed_by" }` | Same. |
| `Membership.sponsor_cid` (when Steward + pending counter-attestation) | extension field `AgentRelationship.elohimSponsorAgentId: ID` | Outside canonical VF; namespace under an extension prefix. Documented as a non-portable field. |
| `Membership.joined_at_block_height` | derivable: VF doesn't track DHT block height; project as `AgentRelationship.elohimJoinedAt: DateTime` derived from the create-action timestamp | Surface as an extension field. |
| `revoke_membership` action | `deleteAgentRelationship` mutation | VF doesn't model "revocation with reason" as a first-class concept; the reason surfaces via the projection's audit-log entry, not the VF mutation. |

The mapping is **lossy in one direction only**: VF → qahal would drop `salt`, `sponsor_cid`, `joined_at_block_height`. qahal → VF is full-coverage on the canonical fields and uses extension namespace for the elohim-specific metadata.

## 7. What this spec deliberately does NOT do

- Does not propose a distinct `Group` entry separate from `Collective`. One notarized type covers congregations, learning cohorts, civic clusters, households-as-collectives. Domain distinctions live in the *charter content* and the `display_name`, not in separate entry shapes.
- Does not introduce `merge_collective` / `split_collective` coordinator functions. Those are governance-scope (mishpat territory) and depend on stewardship-quorum primitives this spec defers (§8.1).
- Does not specify live co-presence wire format. That's libp2p ephemeral state (Category C); a separate "qahal presence" projection design will own it.
- Does not commit to an Angular surface. Epic E of the synthesis plan owns that — this spec is the substrate it consumes.
- Does not author a quorum / threshold attestation primitive. v1 ships with single-sponsor counter-attestation; quorum is §8 deferred.
- Does not author Norm or charter-revision entries. Those are governance primitives — a separate spec.

## 8. Open questions for follow-on planning

### 8.1 Quorum gates for steward attestation

v1 uses single counter-attestation: sponsor proposes → one existing steward attests → membership active. Should `Steward`-role attestation require threshold-N-of-M? Pros: hardens authority capture. Cons: invents a quorum primitive before we have a use site clamoring for it. Recommend: ship v1 with single-sponsor; revisit when a real collective surfaces the need.

### 8.2 Charter mutability

v1 makes the charter immutable; only `display_name` is mutable. Is that right, or should charter revisions be allowed with a Norm-like attestation chain? Recommend: defer to Norm spec — charter revision is a Norm-level governance act, not a Collective-level CRUD.

### 8.3 Collective inheritance / parenthood

A household-as-collective may want a parent (a neighborhood-collective); a learning cohort may want a parent (a school-collective). v1 leaves Collectives as flat. If parenthood is added, it's a new link kind (`HasParent` / `HasChild`) — additive, no entry-shape change.

### 8.4 Access policy: open / invitation-only / application-only

How does a Collective declare its accession policy? Options: (a) encode as `Collective.access_policy: AccessPolicy` enum field (entry-shape change); (b) introduce a separate `AccessPolicy` entry referenced by Collective. v1 has no policy field — every collective is "request_membership; Contributor/Observer self-joins, Steward needs sponsorship." Defer policy variants until use surfaces a real distinction.

### 8.5 Cross-DNA placement

This spec puts `Collective` + `Membership` in the **imagodei DNA** (sibling to `HumanRelationship`), because: collectives are identity-shaped (`implements Agent` in VF), and the coordinator gates that authorize stewardship overlap with the imagodei attestation flow. An alternative placement is a new `qahal` DNA. Recommendation: stay in imagodei for v1 — it keeps the relational identity primitives co-located. Promote to its own DNA only if/when qahal entries multiply past ~10 types.

### 8.6 Stewardship handoff & key recovery interaction

When a steward's agent key rotates (per `imagodei` key recovery flows), how does their stewardship transfer? Likely: the existing `HumanRelationshipRenewal` pattern (imagodei `:847`) gets a sibling `MembershipRenewal` that re-attests the prior Membership to the new agent key. Defer to the implementation plan.

## 9. Implementation handoff

Implementation lands in a follow-on plan dispatched after — and probably coordinated with — the deferred Wave 3 hREA interop brainstorm (`genesis/docs/plans/2026-04-21-rno-lessons-cross-wave-guidance.md` §Wave 3). That brainstorm validates the §6 VF-GraphQL mapping above and decides whether to (a) ship the qahal coordinator zome standalone first, then wire VF-GraphQL later, or (b) co-bundle — ship the zome and the VF projection in the same sprint.

Recommended sequence when that plan is authored:

1. **Substrate first.** Land `Collective` + `Membership` entry types + link kinds + integrity validators in `imagodei_integrity`. Sweettest coverage per `project_zome-sweettest-sync`.
2. **Coordinator second.** Add `qahal_coordinator` zome with the five coordinator functions from §2.4. Coordinator-side authority checks land here.
3. **Projection third.** Diesel projection table `qahal_collectives` + `qahal_memberships` in elohim-storage, fed by `ElohimContentSignal` handlers. Surfaces a `/api/v1/collectives/{cid}` read route and a `Viewer.collectives` GraphQL resolver (which Epic E of the synthesis plan was already planning to consume).
4. **VF-GraphQL last.** Wave 3 brainstorm output drives the `Organization` + `AgentRelationship` Apollo Federation surface mapping per §6.

Each of the four phases is a separate plan + commit cluster. None is large; sequencing them in order keeps the substrate-first discipline intact.
