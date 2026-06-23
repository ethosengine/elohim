# Vision-Gap Plan-STUB — O5: Human-Actuatable Data Agency

> **GREENLIGHT-TO-EXPAND.** This is a scoping stub, not an implementation plan.
> The value/governance core (a person revoking standing over their own data is a
> notarized, capture-resistant act) needs the operator's blessing before it
> expands. Self-answered + recommended below per the operator's standing rule;
> design-doc artifact + go-ahead still required. Working draft — NOT cite-sealed.
>
> Author: rust-architect (truth-layer). Companion stubs: O2 care-valueflows,
> O3 limit-respect governor, O4 home-for-AI. Frame source:
> `VISION-ALIGNMENT-2026-06-14.md`.

---

## 1. Objective + the felt promise

**O5 — agency back to their data.** Grandma can *see who holds her photos*,
*revoke a share*, *withdraw consent*, and *export/port her data* — and watch the
withdrawal take effect across the mesh. Today the protocol can revoke standing
**operator-down** (the operator-actuatable arc: `sets-authority-arc`,
`revokes-commitment`). O5 is the **mirror image, scoped to the person**: the human
holds the same notarized lever the operator does, pointed at *their own content's
reach*. A refused share-withdrawal must be the substrate honestly reporting "bytes
already escaped to peer X, here is the standing record" — never an operator's veto
of a person's wish. The felt promise: *"I shared this. I can un-share it, I can see
who still has it, and I can take my copy and leave — and the system proves it."*

This is the capture-resistance invariant ([[project_hub_optional_floor]]) applied
to the *data-agency* axis: the person on a laptop, with no hub and no doorway
nearby, must hold full revoke/portability standing. Standing lives on the DHT, not
in an operator's admin console.

---

## 2. Vision-vs-substrate GAP (promise vs code today)

The protocol *promises* the person owns the revoke lever; the code today only wires
the **operator/provider** end of it.

| Promise (O5) | What the code does today (file:line) | Gap |
|---|---|---|
| A person can revoke a share they made | `revokes-commitment` exists but supersedes a **provider's own grant** (`mishpat/zomes/mishpat/src/commitments.rs:342-357` `validate_revokes_commitment`; projection `elohim-storage/src/mishpat_projection.rs:90-95,179` `set_revoked_at`). No action scopes a **person revoking the reach/provide of their own content**. | The lever exists for compute grants, not for a person's data-provide. |
| A person can see *who holds my photos* | `graph_views/shefa/distribution.rs:23-60` builds a `DistributionSummary` (STEWARDS-edge replica count) and `resilience_snapshot.rs` builds the stewarding-collective view — but `replica_count`/holder-identity/`diversity_hint` are **zero-filled composition placeholders** (`distribution.rs:7-8,55-57`) and there is **no person-facing route** wired to a "who holds *mine*" query. | Read-model skeleton exists; not human-addressed, holders not resolved. |
| Consent is a person's actuatable act | Consent today is relationship-scoped at the DNA layer (`imagodei_integrity/src/lib.rs:388-389,974` `consent_given_by_a/b`, `RelationshipPendingConsent`); recovery v2 has rich social-quorum consent (`imagodei_integrity/src/recovery_v2.rs`). **None of it governs "withdraw consent for *this content share*."** | Consent is for relationships/recovery, not for data-shares. |
| A person can export/port their data | Blob substrate can `draw` bytes (`/blob/{hash}` GET-only); `lamad-v1/.../healing_exports.rs` does v1→v2 migration export. **No person-facing "export everything I authored + its provenance manifest" action.** | Bytes are drawable; a person-initiated portable bundle is unbuilt. |
| A native care/observed EconomicEvent | `EconomicEvent` is native in the **elohim DNA** (`content_store_integrity/src/lib.rs:1116` with `action`/`bounded_by`/`provides`); a `provide` *creates* the shareable reach. **There is no native person-emitted *un-provide*.** | The provide exists; the inverse (person-emitted withdrawal) is the missing native emitter. |

**Root cause:** every actuation path was reasoned operator→outward
(`P2P-DATAPLANE-CONTRACT-LEDGER` P-ACTUATION owns the operator arc). The person was
never given the inverse handle on the *same primitive*.

---

## 3. The MISSING BRIDGE / primitive (concrete)

**One new action on the existing `Mishpat::Commitment` entry — `withdraws-provide` —
the person-scoped inverse of `sets-authority-arc`.** It is NOT a new entry type. It
is the data-agency row of the
[[project_rea_compute_commitment_primitive]] generalization table:

| Instance | Provider | Recipient | Event class | Bounds shape |
|---|---|---|---|---|
| **Data-agency withdrawal (O5)** | **the person (author/steward)** | **the substrate (self-directed)** | `withdraws-provide` | target content CID, withdrawal scope (`reach-down-to` value, or `unprovide-all`), effective-from timestamp |

Three concrete pieces:

1. **DNA action `withdraws-provide`** — a `Commitment` whose `action` string is
   `withdraws-provide`, authored *by the content's author/steward*, carrying
   `{ target_content_cid, withdraw_to_reach | unprovide, signed_at }`. Validation
   (mirroring `validate_revokes_commitment`, `commitments.rs:342`) asserts the
   author's standing over `target_content_cid` (must-get the content's authorship,
   HDI-only — no `get_links`, gate authorship checks through the coordinator per
   [[project_hdi_no_get_links_in_validators]]).

2. **Storage projection arm** — a new match arm in `parse_commitment_payload`
   (`mishpat_projection.rs:163`) → `parse_withdraws_provide` → drives the
   `reach_authorization` / provide-row down (lowers the receiver-side
   pre-authorization standing so future fetches are refused at the reach gate;
   `elohim-storage/src/p2p/reach_authorization.rs`). The withdrawal **cannot
   un-send bytes already at a peer** — it lowers standing and records the act; the
   honest read-model surfaces "still-held-by" as residual.

3. **Two person-facing read/act routes** (the human handle):
   - `GET /api/v1/me/data/holders?contentCid=…` — resolves the *who-holds-mine*
     view (consumes the dataplane `distribution` read-model; resolves the
     zero-filled holder identities once peer blob-inventory reads land — declare as
     a SOFT cross-plan edge, see §5).
   - `POST /api/v1/me/data/withdraw` — authors the `withdraws-provide` Commitment
     (person-scoped; uses `ConductorCommitmentFetcher` for the just-authored
     bounds read per [[project_mishpat_commitment_cid_is_entry_hash]]).
   - (Portability, phase 2) `GET /api/v1/me/data/export` — streams a portable
     bundle: authored content bytes + EPR provenance manifest + the person's
     withdraw/provide ledger. Cat-C composition over existing blob `draw` + EPR
     projection; no new entry type.

**Why ride the Commitment, not a new entry:** Mishpat is ~11/~100 entries — headroom
exists, but the *correct* design is that withdrawal-of-standing **is** a commitment
act (the person commits "I withdraw my provide-standing on X"), reciprocal with the
substrate, revocable, auditable. Inventing a `DataWithdrawal` entry would re-derive
the exact shape `Mishpat::Commitment` already gives for free, and split the
revoke-audit trail across two entry types.

---

## 4. p2p-design-gate ANSWERS (all four — MANDATORY)

**(1) Class:** **A — notarized.** A person revoking standing over their own data is
a Cat-A act the community must be able to witness and verify (the whole point is
*provable* withdrawal). It rides an existing DHT entry type — no new type. The
*who-holds* read route and the *export* bundle are **Cat-C operational**
(reconstructable projections, no DHT entry).

**(2) Existing DHT entry to ride:** **YES — `Mishpat::Commitment`**
(`mishpat/zomes/mishpat/src/commitments.rs`, the `sets-authority-arc` /
`revokes-commitment` family). New `action = "withdraws-provide"` discriminator
ONLY; the entry struct is unchanged. (Headroom verified path: `rg
'#\[hdk_entry_helper\]' elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/`
before authoring — but no new type is needed regardless.)

**(3) Identity:** **content-derived (CID).** The withdrawal Commitment's `cid` is
its Holochain **`entry_hash`** (per [[project_mishpat_commitment_cid_is_entry_hash]]
— `cid = entry_hash`, NOT action_hash; `dht_anchor_hash = action_hash`). It
references `target_content_cid` (the EPR/content address of the photo being
un-shared) by content address. No UUID, no slug.

**(4) Coordinator fn + projecting signal:**
- **Creates:** `withdraw_provide(WithdrawProvideInput) -> CommitmentOutput` —
  new `#[hdk_extern]` in the mishpat coordinator, mirroring the existing
  `create_commitment` path; returns `{ action_hash, entry_hash }`.
- **Projects:** the existing `MishpatSignal::CommitmentCommitted` post-commit
  signal → `subscribe_mishpat_signals` → `handle_mishpat_signal` →
  `parse_commitment_payload` (new `withdraws-provide` arm) → reach-authorization
  down-write. (The signal subscription is the one already wired for the arc arm —
  no new subscriber needed, closing the 2a-gap class cited in the CID memory.)
  Projection writes land via the reconcile path, never a direct service→Diesel
  catch-up.

---

## 5. Existing substrate to build on + what NOT to re-own

**Build on (file:line):**
- `mishpat/zomes/mishpat/src/commitments.rs:342-357` — `validate_revokes_commitment`
  is the template for `validate_withdraws_provide` (same standing-assertion shape).
- `elohim-storage/src/mishpat_projection.rs:154-179` — `parse_commitment_payload`
  match; add ONE arm. `set_revoked_at` (`:90`) is the projection-write precedent.
- `elohim-storage/src/p2p/reach_authorization.rs` — author-side earning +
  receiver-side pre-authorization is where a withdrawal lowers standing.
- `graph_views/shefa/distribution.rs:23-60` + `resilience_snapshot.rs` — the
  *who-holds* read-model skeleton (STEWARDS edges); resolve its zero-filled
  holder/diversity fields.
- `content_store_integrity/src/lib.rs:1116` `EconomicEvent` (`provide` action) —
  the thing being withdrawn; the withdrawal is its conceptual inverse.
- `imagodei_integrity/src/recovery_v2.rs` + consent fields (`:388`) — the
  consent-vocabulary precedent the withdraw surface should *speak the same language
  as* (don't fork a parallel consent model).

**Do NOT re-own (cite ledgers):**
- **`doorway/.../routes/self_healing.rs`** — dataplane **P-DIAGNOSTIC** SOLE owner
  (`P2P-DATAPLANE-CONTRACT-LEDGER` RESOLUTION-G). The withdraw surface is its own
  new route; do not extend the self-healing view.
- **`elohim-storage/src/mishpat_projection.rs:163` `sets-authority-arc` arm** —
  dataplane **P-ACTUATION** SOLE owner (S13). O5 adds a *sibling* arm
  (`withdraws-provide`); it does NOT touch the arc arm. Declare as a co-located
  additive edit, sequenced behind P-ACTUATION's structural landing if concurrent.
- **`doorway/.../routes/federation.rs`, `coherence.rs`, `bootstrap/*`** — owned by
  the **FEDERATION-WEB2-LEDGER** (F-EDGE / F-COHERENCE / F-BOOTSTRAP). O5 consumes
  none of these.
- **`P2PStatusInfo.anchor` / `self_cid_present`** — dataplane P-DIAGNOSTIC (S9).
  The *who-holds* route MAY read it (SOFT consume) to show per-holder
  content-presence; it must NOT redefine it.

**Cross-plan edges (declared):**
- **SOFT → P-DIAGNOSTIC** — the *who-holds* route resolves real holder identities
  once peer blob-inventory reads land (today zero-filled). Ship the route returning
  the honest skeleton now (correct-but-dormant); enrich when the inventory reader
  exists. NEVER wire it to "inventory gossip count ⇒ holder"
  ([[project_inventory_exchange_not_byte_replication]]).
- **SOFT → P-ACTUATION** — consume the `Actuation`/`ActuationRefusal` contract shape
  (`elohim_compute::actuation`) so a refused withdrawal speaks the same refusal
  vocabulary as the operator arc — but the refusal *semantics* are inverted: a
  refusal here means "bytes already escaped" (an honest residual report), not
  "operator declined." This inversion is the **O3 boundary**: who owns the refusal?
  (see open questions).

---

## 6. The FIRST a2o SCENARIO (story-first — the spec)

Home: `genesis/a2o/features/auth/` (data-agency is imagodei-adjacent) or a new
`genesis/a2o/features/data-agency/`. Couples O5 ⨯ O1 (grandma sees + controls).

```gherkin
@vision:O5 @data-agency @requires:household-nodes
Feature: A person withdraws a photo share and sees it take effect

  Background:
    Given Grandma authored the photo "beach-day.jpg" with reach "neighborhood"
    And the photo is held by 3 peers in her household mesh

  Scenario: Grandma sees who holds her photo
    When Grandma opens "who holds my photos" for "beach-day.jpg"
    Then she sees it is held by 3 named peers
    And she sees its current reach is "neighborhood"

  Scenario: Grandma withdraws the share and it is provably recorded
    Given Grandma is viewing "beach-day.jpg"
    When she chooses "withdraw — only my household may see this"
    Then a notarized "withdraws-provide" commitment is authored under her agent
    And the photo's reach standing is lowered to "household"
    And a new neighborhood peer requesting the photo is refused at the reach gate
    And Grandma sees the withdrawal recorded with a verifiable timestamp

  Scenario: The withdrawal is honest about bytes already held
    Given a neighborhood peer already holds a copy of "beach-day.jpg"
    When Grandma withdraws to "household"
    Then the who-holds view still shows that peer as a residual holder
    And the view labels it "already held — withdrawal lowers standing, cannot un-send"
```

The **honesty scenario is the spec's spine**: it forbids the regression of
pretending a withdrawal un-sends bytes (the data-agency analog of inventory ≠
replication). "Provably recorded" + "refused at the reach gate" are the two
substrate assertions that prove the lever is real.

---

## 7. Effort + risk + why this serves the objective

**Effort: M** (DNA action + validator: S; projection arm + reach-down write: S;
two person-facing routes: M; portability/export bundle: deferred phase-2, M-L).
The withdraw lever (DNA + projection + withdraw route) is the **S/M MVP**; the
who-holds resolution and export bundle layer on after.

**Risk:**
- **Low (substrate):** rides an existing entry; mirrors a proven validator +
  projection pattern; no new DNA entry type, no DNA-hash move (coordinator + a
  new action discriminator are DNA-hash-neutral when the entry struct is unchanged
  — verify against [[project_dna_hash_blind_to_coordinator_zomes]]).
- **Medium (semantic):** the refusal-ownership inversion (§5) is a genuine design
  fork that touches O3. Getting "a refused withdrawal is honesty, not veto" wrong
  would re-import the operator-veto smell into the person's lever.
- **Medium (cross-plan):** holder-identity resolution depends on peer-inventory
  reads (SOFT edge); ship dormant to avoid blocking on dataplane sequencing.

**Why it serves O5:** it puts the *exact same notarized, revocable, auditable lever
the operator holds* into the person's hand, pointed at their own data — the
literal inversion of the operator-actuatable arc, on the same primitive. It couples
O1 (grandma sees + controls), O2 (the provide/withdraw is REA-shaped), O3 (limit
self-respect — a withdrawal is a person setting their own boundary), and O7
(capture-resistance: standing on the DHT, not an admin console).

---

## 8. OPEN QUESTIONS for the operator (decisions only you can make)

1. **Refusal ownership (the O3 boundary — most important).** When a withdrawal
   *cannot fully take effect* (bytes already at a peer), is that a `withdraws-provide`
   that **succeeds with a residual report** (recommended — the substrate is honest,
   the person's standing-lower always lands), or a **refusal**? My recommendation:
   the withdrawal of *standing* ALWAYS succeeds (it is the person's act, never
   refused); the "still-held-by" is a separate honest read-model, NOT a refusal.
   A refusal would mean an operator-shaped veto leaked into the person's lever —
   the exact anti-pattern O5 exists to kill. **Confirm this framing.**

2. **Is `withdraws-provide` enough, or does O5 need a distinct `port-data` /
   `export` notarized act too?** I recommend export is **Cat-C operational**
   (a projection bundle, no notarized entry) for the MVP — portability is reading
   your own bytes + provenance, not a community-witnessed act. Confirm, or decide
   export must be notarized (e.g. for legal-grade "I took my data" proof).

3. **Consent vocabulary unification.** Should the withdraw surface speak the
   *same* consent vocabulary as recovery-v2 / relationship consent
   (`imagodei`), or is data-share consent a distinct vocabulary? I recommend
   reusing the imagodei consent vocabulary to avoid a fork — but this is a
   value-layer call.

4. **Does the who-holds view ship dormant now** (honest zero/skeleton until
   peer-inventory reads land, per correct-but-dormant discipline), **or wait** for
   the dataplane holder-resolution? I recommend ship-dormant (unblocks the UI, the
   moment data arrives it lights up). Confirm.

5. **DNA entry budget sign-off.** No new entry type is proposed (rides
   `Mishpat::Commitment`). Confirm you concur with riding the existing entry rather
   than minting a `DataWithdrawal` type — this is the one near-irreversible spend I
   am explicitly NOT making without your blessing.
