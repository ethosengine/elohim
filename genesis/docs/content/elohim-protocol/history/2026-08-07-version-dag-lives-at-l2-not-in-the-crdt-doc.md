---
title: "History/ADR: The version DAG lives at L2 (the DHT), not inside the Automerge doc"
id: version-dag-lives-at-l2-not-in-crdt-doc
type: history-decision
status: Accepted
tier: history
created: 2026-08-07
topic:
  [
    crdt,
    automerge,
    holochain,
    dht-notary,
    version-dag,
    declared-head,
    content-dataplane,
    uncomposed-fork,
  ]
# A settled decision recorded from code, not from a spec: the spec that asked for the
# other answer (2026-07-01 CRDT-authoritative content state) is amended, not deleted.
cites:
  - genesis/docs/superpowers/specs/2026-07-01-crdt-authoritative-content-state-dht-notary-decouple-design.md
  - lens-version-dag-epr-policy-dependency-design | the spec that SEALED the declared-head-over-lineage-DAG primitive at L2 (Mishpat::Commitment + version_parent, root-CID chain identity, head-as-declared-dependency) — the running instance the 2026-07-01 content-dataplane spec forked from without citing | sha256:62e0f37f8f57c0ed | path: genesis/docs/superpowers/specs/2026-06-27-lens-version-dag-epr-policy-dependency-design.md
  - identity-head-key-lineage | the spec that GENERALIZED the primitive across four instances (content lens, provenance, REA collective, key rotation) and named the compose-dont-build rule this record applies to content head-election | sha256:95950b918c8803bc | path: genesis/docs/superpowers/specs/2026-07-17-identity-head-key-lineage-design.md
memory_anchors:
  - project_versioned_entity_head_is_declared_dependency
  - project_automerge_content_sync_plane_lit
  - project_mishpat_commitment_cid_is_entry_hash
  - feedback_reach_head_replication_distinct_planes
---

# The version DAG lives at L2 (the DHT), not inside the Automerge doc

> **Hot-context pointer (the one sentence to remember):**
> **The CRDT plane converges VALUES; the DHT carries VERSIONS and elects the canonical head.**
> The Automerge doc's `head` / `headActionHash` are HINTS — observability scalars a peer may
> put any bytes into — never authority. If you are about to build lineage edges or a head
> election inside a CRDT document, stop: that substrate already exists one layer up.

This record exists because the decision was, until now, inferable only from a migration comment
and zome code. A future session reading the 2026-07-01 spec alone would either re-derive this over
days or — worse — build the superseded requirement as written.

## The decision

Three layers, and the split between them is the whole point:

| Layer                        | Converges / carries                                                 | Authority?                                                 |
| ---------------------------- | ------------------------------------------------------------------- | ---------------------------------------------------------- |
| **L1 — Automerge CRDT**      | the serving **value**, plus a grow-only SET of versions that coexist | **No.** Unauthenticated peer input. Hints only.            |
| **L2 — Holochain DHT**       | the version **lineage chain** and the **canonical-head election**   | **Yes.** Notarized links, one clock every peer reads alike. |
| **L3 — SQLite (diesel)**     | read-optimized serving                                              | **No.** A projection of the L2 election.                   |

The DHT does not merely witness a head someone else picked. It runs a multi-candidate election
over a link set, and the SQL columns that record the outcome are explicitly a projection of it,
not a second ledger.

## The evidence chain (verified against the tree, 2026-08-07)

**1. The decisive artifact — the migration says it in its own words.**
`elohim/elohim-storage/migrations/2026-08-02-120000_content_add_canonical_election/up.sql`:

> Source of truth: the Holochain DHT canonical-head LINK set (anchor `canonical_head`, elected by
> `content_store::select_canonical_winner`). These two columns are a PROJECTION of that election,
> not a second ledger.

It adds `canonical_declared_at` and `canonical_earned` — the winning declaration link's *notarized
DHT timestamp* and its tier — precisely so the projection can replay the L2 election without
re-reading the DHT. Its RCA note is the sharpest statement of why the clock has to come from L2:
the previous guard keyed off `declared_head_at`, which "is three different clocks sharing one
column, so it could never order two DECLARATIONS."

**2. The election is real, and it is at L2.**
`elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` carries **two distinct anchor
namespaces**, deliberately non-intersecting:

- `"content_id"` — the per-root **version chain**, walked by `gather_content_chain` (line 2801).
- `"canonical_head"` (`CANONICAL_HEAD_ANCHOR`, line 2838) — the **cross-root canonical-head
  declaration** link set, with `staging` / `earned` provenance tiers carried in the reused
  `IdToContent` link tag (coordinator-only; no new entry or link type, no DNA-hash move).

`select_canonical_winner` (line 2983) arbitrates that set on an `ArbitrationKey` of
`(tier, clock, tiebreak)` — earned beats staging, then the notarized DHT link timestamp, then the
content-addressed `create_link_hash`. Coordinator surface: `resolve_content_head`,
`declare_content_head`, `declare_canonical_content_head`, `declare_earned_canonical_head`
(lines 3561, 3601, 4013, 4058).

**3. Storage got a POINTER, never a structure.**
`migrations/2026-07-04-120000_content_add_declared_head/up.sql` adds exactly one column —
`declared_head_action_hash TEXT` — and enumerates the only two writers (own-conductor-witnessed
commit signals; conductor-VERIFIED reconcile stamps), closing with: "NEVER written from CRDT/gossip
input — that would launder un-witnessed peer state into the notary-authority HEAD."

**4. The doc field is documented, in code, as a hint.**
`elohim/elohim-storage/src/sync/projector.rs:140-146`:

> `headActionHash` — an OBSERVABILITY/HINT scalar for peers … It is a HINT only, never an authority
> signal: any peer can put any bytes in a CRDT doc, so consumers must never treat the converged
> value as notarization.

**5. The guard is enforced, named, and tested.**
`projector.rs:463` — REQ-N5, on `reverse_project_content_doc`: "**Do not read it here. Do not add a
heal for it. Ever.**" `dht_anchor_hash` and `declared_head_action_hash` are written only by
conductor-verified paths. The reverse projector heals `blobHash` only, amber-tier, behind a
`crdt_converged_at` marker. Guard test: `converged_head_hint_is_never_stamped` (asserted at
`projector.rs:1816-1820`).

**6. Negative evidence — with a correction to the received framing.**

The claim "there is no multi-version doc structure anywhere in the CRDT layer" is **partially
refuted, and the true shape is more interesting than the claim.** What the CRDT plane actually has:

- A **grow-only `versions` map** keyed by version-cid, plus a `head` scalar
  (`projector.rs:157-158`, `217-227`, `292-300`). Two peers that authored distinct bytes for one id
  hold distinct keys, so both versions **coexist** — added, never LWW-clobbered. There is a proof
  test: `distinct_versions_coexist_head_notary_set` (`projector.rs:877`).

What it does **not** have, confirmed by grep across `elohim/elohim-storage/src/sync/` (four files:
`doc_store.rs`, `mod.rs`, `projector.rs`, `stream.rs`):

- **No lineage edges of any kind.** The only `parent` in the whole directory is
  `config.db_path.parent()` — a filesystem call. No `version_parent`, no back-pointers, no
  branch/merge structure.
- **No election.** The doc's `head` is whatever the local author last projected.
- **No `version_dag` or `content_version` table** anywhere in the storage schema. (`version_parent`
  columns *do* exist in the tree — on **lenses** (`db/lenses.rs`, `db/diesel_schema.rs:1776`) and in
  `mishpat_projection.rs` — which is the L2 primitive this record is about, not the CRDT plane.)

So the CRDT got exactly the **value-coexistence half** of a version DAG and none of the
lineage-or-election half. The code's own naming (`version_dag_current`, "the version-DAG leg")
overstates what it is: a flat keyed set with a head pointer, not a DAG. **Read that naming with
suspicion** — it is the residue of the superseded direction below.

## The superseded direction

`genesis/docs/superpowers/specs/2026-07-01-crdt-authoritative-content-state-dht-notary-decouple-design.md`
asked for the L1 answer in three places:

- **LAW-3 (§1.4):** "The CRDT converges the _version DAG_; the notary _elects HEAD_ as a declared
  dependency."
- **REQ-F4 (§3):** "A real notarized HEAD pointer AND a multi-version doc structure MUST be added
  before any HEAD-is-declared claim holds."
- **OD-1 resolution (§7.1):** "✅ RESOLVED 2026-07-01 (architect): option (b) — FULL 1c NOW …
  `blob_hash` is carried under a notarized declared-HEAD pointer the CRDT converges as a DAG of
  versions, never an LWW scalar."

**The notarized-HEAD-pointer half was built. The converge-the-DAG-in-L1 half was not, and should
not be.** LAW-3's second clause (the notary elects HEAD as a declared dependency) is exactly what
`select_canonical_winner` does. LAW-3's first clause (the CRDT converges the version DAG) is the
part the codebase declined. OD-1 also called `resolve_head(id)→declared_head_action_hash` "net-new
substrate"; `resolve_content_head` exists at L2 in the `content_store` zome.

The 07-01 spec is amended in place with dated SUPERSEDED annotations pointing here. Its reasoning
is intact — the arc has value, and its adversarial verdicts still hold.

## The process failure is the transferable lesson: an uncomposed fork

The 07-01 spec **cited the principle and re-derived the mechanism.** It references the memory policy
seed `project_versioned_entity_head_is_declared_dependency` twice (in LAW-3 and in §7.2). But grep
across the whole spec finds **zero** occurrences of `version_parent`, `Mishpat::Commitment`, or the
2026-06-27 lens-version-DAG spec. (Its "2026-06-27" companion reference is the *resilience-facings
§11–12* decision — a different document.)

That is the failure mode, and it is repeatable: **it knew the rule and did not look for the running
instances.** Four days earlier, the same shape had been sealed as a concrete L2 substrate. The 07-01
spec re-derived its own answer for the content dataplane in isolation, and named as "net-new
substrate" a selector that already existed one layer up. This is an **uncomposed fork of a recurring
primitive**, not a deliberate divergence with rationale — no weighing of the existing instance
appears anywhere in the document.

The correction is procedural: when a spec cites a policy seed, the next move is to find every place
that seed is already *instantiated* and state explicitly whether you compose with it or fork from
it. A fork with reasons is a decision; a fork without them is drift wearing a spec's clothes.

## The primitive it should have composed with

**declared-head-over-lineage-DAG** — sealed in
`genesis/docs/superpowers/specs/2026-06-27-lens-version-dag-epr-policy-dependency-design.md`, and
generalized in `2026-07-17-identity-head-key-lineage-design.md`, whose central finding is that "an
identity head is **not a new primitive** — it is the third named instance of the
**declared-head-over-lineage-DAG** shape the substrate already runs." That spec's §1 tabulates the
instances, **all at L2 as `Mishpat::Commitment` DAGs**:

| Instance             | Substrate                                                                      |
| -------------------- | ------------------------------------------------------------------------------ |
| Content lens/policy  | `author-lens` / `binds-policy` Commitments                                     |
| Provenance           | epr-meta lineage edges                                                          |
| REA collective       | `Collective{founder,charter}` + `Membership{role:Steward}` (`imagodei_integrity/src/qahal.rs`) |
| Human key rotation   | `KeyRotation{superseded_agent_pubkey,new_agent_pubkey,authority:RecoveryAuthority}` (`imagodei_integrity/src/recovery_v2.rs`) |

The shared shape:

- an immutable DAG of nodes with a `version_parent` back-pointer (**a SET**, so merge is
  expressible);
- chain identity is the **root CID** (`version_parent=[]`), stable across the whole DAG;
- **which HEAD applies is a DECLARED dependency** — pin / latest / range. The binding decides; the
  infrastructure never auto-resolves "newest";
- **revert is a re-pin at declaration level**, never a mutation of the DAG.

Content head-election is the same shape. It belongs beside those instances, not inside a document.

## The unifying insight — one inversion at two layers

REQ-N5 says: **the doc's head hint is never consumed as authority.**
The primitive says: **the binding decides the head, not the infrastructure.**

These are the same inversion, stated at two altitudes. In both, the thing that *has* the value is
denied the right to *declare* what the value means. The CRDT holds a head scalar and is refused
authority over it; the query layer can see every version and is refused the right to pick one. In
both cases the selection is an explicit, notarized, declared act — never an emergent property of
whoever wrote last or whoever sorted newest.

Any layer that can hold a candidate will, left alone, start electing. Both rules exist to stop that.

## What this means for the next design

1. **Do not build lineage edges or head election inside a CRDT document.** Values converge there;
   versions do not live there.
2. **A head hint on a converged doc is diagnostic only.** If you are about to plumb a doc field into
   a SQL write, route it through a conductor-verified path instead (REQ-N5's own instruction).
3. **When you need a version DAG, you are reaching for `Mishpat::Commitment` + `version_parent` at
   L2.** It is a recurring primitive with several running instances — compose, do not build.
4. **Distrust the name `version_dag_current` in `sync/projector.rs`.** It guards a grow-only set
   with a head pointer. The DAG is elsewhere.
5. **Reach ≠ head ≠ replication** stays true across all of this: this record is entirely about the
   head plane.
