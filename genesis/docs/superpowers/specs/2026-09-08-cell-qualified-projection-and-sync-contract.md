---
title: "Cell-qualified projection and sync — the contract that stops clone-authored content from being re-authored into a peer's base cell"
id: cell-qualified-projection-and-sync-contract
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
graduation-trigger: "Draft→Active when an adversarial read and a second p2p-design-gate pass are addressed in the text and the eight stations of §9 are written as scenarios bound to @concern:cell-qualified-projection under dataplane-convergence; Active→Canonical when ALL EIGHT stations pass — the atom's own done-when (stations 1-5 and 8: EMPTY identity diff on a receiving peer's base cell across all six planes after a clone write, qualifier carried on projection and sync rows, fixtures-clone.mjs as the check) plus this contract's two additional commitments (station 6 partition honesty, station 7 per-partition atomic rebuild) — with a receipt under genesis/a2o/reports/ and the DELTA on the habit atom"
actor: agent:rust-architect@fable-5.1
written: 2026-09-08
domain: D6
habits: [dataplane-convergence, happ-lineage-migration]
boundary: "Design only. This contract governs ONE qualifier (the cell a projected row or sync doc came from), THREE records that must carry it (the storage `content` projection row, the Automerge content-sync doc, the re-author path's fresh DHT action), and the refusal rule between them. It does not decide whether group clone spaces ship (D6), which spaces an operator selects, or whether storage runs one instance per cell."
cites:
  - "accountable-correction-contract | the sibling slice-0 contract this one mirrors in structure; its additive map-keyed wire recipe is the one §7 says does NOT transfer to the compact-encoded sync and shard planes | sha256:691e8b89f394214c | path: genesis/docs/superpowers/specs/2026-09-06-accountable-correction-contract.md"
  - "holons-are-spaces-how-we-use-holochain | sealed decisions D0-D10; D4 supplies this contract's named-successor-authority and stated-partition-behaviour requirement, D6 the clone-space candidacy this contract gates | sha256:ac1de36d2423be82 | path: genesis/docs/content/elohim-protocol/architecture/2026-09-06-holons-are-spaces-how-we-use-holochain.md"
  - "ai-stewarded-commons-reimplementation-plan | §0.5 supplies the partition vocabulary this contract uses (pending / conflict-visible) when currentness cannot be checked | sha256:0d5f1300b5d615dc | path: genesis/docs/content/elohim-protocol/architecture/2026-09-06-ai-stewarded-commons-reimplementation-plan.md"
  - "package-composition-at-the-holochain-seam | names this contract as what makes its D3 rule enforceable rather than declared; the packaging rules whose clone-cell assumptions this contract constrains | sha256:e417e740ce2212ce | path: genesis/docs/superpowers/specs/2026-09-08-package-composition-at-the-holochain-seam.md"
  - genesis/data/timeline/backlog/clone-content-escapes-via-shared-projection.md
---

# Cell-qualified projection and sync — the contract

**Vocabulary this contract rests on.** A Holochain hApp installs one **cell** per
declared **role** — a cell is one agent's participation in one DNA's validation context
and gossip network. An operator may **clone** a role: the conductor installs a second
cell under the same role name, with its own DNA hash and separate DHT. `elohim-storage`
reaches those cells through the local **conductor** and projects what it reads into
local SQL and an Automerge sync store. The **six planes** the fixtures-clone harness
inventories are the three source-chain views of a cell (`*-actions`, `*-ops`,
`*-content`), the DHT `targets` view, the storage `projection` rows, and the `sync`
documents. The **D-numbers** (D0, D3, D4, D6) index the ten sealed decisions in §0b of
the holons-are-spaces companion.

The 2026-09-07 household-mesh run exposed the escape the clone experiment tested for.
Content written into a `lamad` clone stayed inside that clone's DHT on the authoring
peer — `base-actions`, `base-ops`, `base-content` and `targets` diffs were all empty —
but escaped through storage: two rows landed in the peer's shared `content` projection
carrying `h_app_id: "lamad"` and no cell qualifier, two docs landed in the `elohim` sync
namespace, and eight minutes later a *receiving* peer's storage re-authored one doc into
its base `lamad` cell under a fresh DHT action. Clone isolation holds only for the
authoring peer's DHT.

This contract defines the qualifier that closes the escape, its source, and each plane's
response to an absent qualifier or a cell missing locally. Each requirement cites the
code it binds. **Sections 1 through 7 state required target behaviour, not current
behaviour**. Citations distinguish existing behaviour from gaps; nothing is claimed as
delivered.

## 1. The qualifier and its authority

**The qualifier is `(cell_dna_hash, cell_clone_id)`**, two columns rather than one
string; neither can be parsed from the other:

- `cell_dna_hash` — the Holochain `DnaHash` of the cell that authored the record, full
  39-byte `uhC0k…` form, base64. Not truncated: the eight-byte prefix in the connect log
  line (`hc_client.rs:372`) is for logging, never identity. This is the
  **discriminating** half — a clone carries distinct DNA modifiers, so its hash differs
  from its role's provisioned cell.
- `cell_clone_id` — the clone's `clone_id` string (`role.N`) when the record came from a
  cloned cell, NULL when it came from the role's provisioned cell. This is the
  **addressing** half: it names the cell in a refusal   message and resolves a redirect
  (§3, case 2) to an `HcClient`.

**The only admissible source is the conductor's own `app_info`.** `HcClient::connect`
reads it: `list_apps` → `app_info.cell_info[role_name]` →
`cell_discovery::select_target_cell` (`hc_client.rs:340-369`,
`cell_discovery.rs:212-226`), which returns a `CellId` for a `Provisioned` cell when the
target is a bare role and for a `Cloned` cell when the target is `role.clone`
(`cell_discovery.rs:196-210` parses that form). **Only half the qualifier is retained.**
`HcClient` keeps the `CellId` and exposes its DNA hash (`hc_client.rs:655`, `:672-674`),
so `cell_dna_hash` needs no new plumbing. `select_target_cell` reads `c.clone_id` /
`c.name` to pick the `Cloned` branch (`cell_discovery.rs:221-225`) then **discards the
clone identity**. The returned `CellId` has no clone concept; a `grep` for `clone_id` in
`hc_client.rs` finds nothing. Station 1 retains the clone identity; the retained
qualifier becomes the sole source.

**A role name is not a qualifier.** `h_app_id` is an app-scope string on the projection
(`db/context.rs:51-57`; `AppContext` carries `h_app_id` and a libp2p peer id, but no
cell identity) and `AppContext::default_lamad()` hardcodes `"lamad"`
(`db/context.rs:69-71`). Every clone of the `lamad` role produces the same `h_app_id`.
This causes the measured collision: a role identifies multiple cells and cannot separate
them.

**Peer-supplied qualifiers are claims, never authority.** A qualifier that arrives over
the sync plane, the shard plane, or view-federation is a *routing hint* — the sender's
claim about the source cell. The hint supports refusal (see §3) and display, never
provenance. Only the local conductor may stamp a qualifier granting notarization
authority, just as `dht_anchor_hash` is only ever written by conductor-verified paths
(`sync/projector.rs:632-642`, the REQ-N5 guard).

## 2. Durable source and what carries the qualifier

### 2.1 The storage `content` projection row

`content` is declared
`-- Source of truth: DHT (Content entry in lamad DNA).
Classification: A.`
(`migrations/2026-01-08-000000_initial/up.sql:46-50`) with `id TEXT PRIMARY KEY` and
`h_app_id` as an ordinary column. The comment classifies the *entry*, not the *row*: the
DHT `Content` entry is class A; the table is its class-C rebuildable index. Only the
class-C classification permits §5's per-partition reset. **With `id` alone as primary
key, two cells authoring the same content id cannot coexist as rows**: the second write
silently updates the first. The qualifier therefore changes the key.

Required: `content` gains `cell_dna_hash TEXT` and `cell_clone_id TEXT`, and its logical
key becomes `(h_app_id, cell_dna_hash, cell_clone_id, id)`. `upsert_with_anchor`
(`db/content_diesel.rs:1083`) matches, pre-contract, on `h_app_id` and `id` only
(`:1094-1099`); it takes the qualifier as an argument and matches on the full key. The
`ContentCommitted` arm that calls it (`rea_projection.rs:723-770`) supplies the
qualifier of the `HcClient` whose conductor emitted the signal. Post-commit signals are
cell-local, so the qualifier identifies the authoring cell.

### 2.2 The Automerge content-sync doc

The content-sync producer writes one doc per content row at `node:{id}`
(`sync/projector.rs:22-25`) under the `"elohim"` namespace (`sync/projector.rs:932`,
`PROJECTION_NAMESPACE`), listed only by `initiate_sync_round`
(`sync/projector.rs:250-254`, pointing at `p2p/mod.rs:6996`). The DocStore keys on
`{h_app_id}:{doc_id}` (`sync/doc_store.rs:147`). Two clones of one role therefore
collide on both doc and row keys.

Required: the doc id becomes cell-qualified —
`node:{cell_dna_hash}:{cell_clone_id|-}:{id}` — and the namespace stays `"elohim"`. The
namespace is a pinned wire contract guarded by a test
(`projection_namespace_is_wire_contract`, `sync/projector.rs:925-932`). Changing the
namespace requires every peer and the DNA to migrate in lockstep. Doc ids are free-form
and prefix-typed (`sync/doc_store.rs:298-315` infers doc type from the prefix, which
`node:` still satisfies). `content_doc_id` becomes the sole function that renders the
qualified id, and `reverse_project_content_doc` (`sync/projector.rs:643`, whose
prefix-strip is at `:648-651`) parses the id, refusing on failure.

### 2.3 The re-author path's fresh DHT action

The escape ends with a Holochain write. **Two** call sites author through the conductor.
Both target the cell bound to the `HcClient`: for storage, the `lamad` **role**, meaning
the base cell (`services/conductor_writes.rs:62-66`, `:360-368`). Both are in
`ContentService::update_via_conductor`, the sole path to conductor writes:

- The NULL-anchor branch — a row with no `dht_anchor_hash` is bootstrapped by
  `call_create_content` from the SQL row (`services/content_service.rs:385-386`).
- The stale-anchor heal — an anchored row the conductor cannot resolve is re-published
  by   `call_create_content` (`services/content_service.rs:425-427`).

Two sweeps drive those branches and inherit the §3 gate: the projection-reconcile
content arm's `AdoptOutcome::Author` branch, which sends an empty patch through
`update_via_conductor` to trigger healing (`p2p/projection_reconcile.rs:2638-2675`), and
the startup pass that walks every NULL-anchor row
(`services/reanchor_backfill.rs:1-31`). Both sweeps must also refuse *before* a
conductor round-trip. These entry-point comparisons save budget; they add no authority.

Both branches mint a **new** `Content` Create in the base cell, and the resulting
`ContentCommitted` signal stamps a base-cell `dht_anchor_hash` over a row whose bytes
came from a clone. That is the measured cross-peer write amplification.

## 3. Validation contract — the re-author refusal rule

Required change: **storage never authors content into a cell other than the one whose
qualifier the row carries.** Before writing, either call site in §2.3 compares the row's
`(cell_dna_hash, cell_clone_id)` with the target `HcClient` qualifier
(`hc_client.rs:655`, `:672-674`). Exactly three outcomes:

1. **Equal** — proceed. Writes to the peer's own base corpus remain unchanged.
2. **Different, and this peer has the named cell** — the write is redirected to an
   `HcClient` bound to that cell, or, when no such client is available, refused as
   `wrong-cell` and left `pending` (§4). It is never redirected to the base cell.
3. **Different, and this peer does not have the named cell** — refused as
   `foreign-cell`,    permanently for this peer. A `foreign-cell` refusal is a
   **positive** outcome:    no retry on the next sweep, no addition to the reconcile
   arm's gap or divergence counters.

**A row with no qualifier is legacy, and legacy means base cell only.** A NULL
`cell_dna_hash` is read as "the provisioned cell of this row's `h_app_id` role on this
peer" only for this comparison. NULL never means "any cell" and never matches as a
wildcard. Once migrated (§6) no new row is written without a qualifier; a NULL qualifier
appearing after migration is a defect to log, not a state to serve.

**The refusal binds the sync plane's consumer too.** `reverse_project_content_doc`
already refuses to launder converged doc fields into notarization provenance
(`sync/projector.rs:609-642`). It gains the sibling refusal: a doc with a qualifier
different from the row's heals nothing. A doc naming a cell missing locally is never
projected into a row.

## 4. Partition behaviour

Required behaviour uses sealed decision D4's vocabulary: **pending** and
**conflict-visible** when currentness cannot be checked (reimplementation plan §0.5).
Four states, three of them pending:

- **The qualifier's cell is not installed locally.** The record is retained and shown as
  `pending — foreign cell`, with the qualifier displayed. Nothing is authored   or
  anchored; no reconcile arm treats the record as a closable gap. The peer can neither
  witness nor disprove these bytes.
- **The qualifier's cell is installed but no `HcClient` is bound to it** — §3's
  `wrong-cell` refusal. `pending — wrong cell`, retried once a client for that cell
  exists.   This differs from a missing cell or an unreachable cell with a client.
- **The qualifier's cell is installed but the conductor cannot reach it** (cells
  disabled during a boot window — the condition `reanchor_backfill` exists for,
  `services/reanchor_backfill.rs:9-20`). `pending — cell unavailable`, retried with the
  existing bounded backoff. This is a liveness failure.
- **Two rows share `(h_app_id, id)` under different qualifiers.** Both are retained and
  the id is **conflict-visible**: readers see both, each labelled with its cell. Neither
  is deterministically promoted, including by last arrival:   different validation
  contexts have no authoritative ordering.
- **The qualifier is absent and the row is anchored.** The row is legacy-but-witnessed:
  it is served, and it is a migration candidate (§6), not a conflict.
- **A qualifier arrives that contradicts a row's existing anchored qualifier.**
  Rejected,   not retried. A *positive mismatch* disproves the claim; missing evidence
  leaves the claim   unresolved. Only missing evidence warrants retry. Retrying a
  disproved claim wastes budget   on a fixed answer, as in the correction contract when
  an acceptance names the wrong author.

## 5. Rebuild path

Derived tables reset and re-derive **per cell**, atomically with their consumption
state.

- The unit of rebuild is one `(h_app_id, cell_dna_hash, cell_clone_id)` partition. A
  rebuild of the base partition never clears clone partitions, and vice versa.
- A rebuild is either a fresh partition built to completion then atomically swapped in,
  or an in-place reset that serialises live projection writers and clears the
  partition's   `content` rows, sync docs, and per-partition reconcile cursors **in one
  transaction**.   Clearing rows but retaining cursors makes a partial rebuild look
  converged.
- Readers see `rebuilding` for that partition until every retained input has been
  replayed;   other partitions stay served throughout. Equality after rebuild =
  identical canonical   logical rows (id, qualifier, anchor, declared head, reach, body
  address), excluding   SQLite layout and operational timestamps.
- The corpus back-fill that seeds the DocStore from SQL (`sync/projector.rs:515-530`)
  becomes partition-scoped: it pages one partition and writes qualified doc ids,
  preventing back-fill from merging two cells' corpora.

## 6. Migration shape for existing rows

- **Stamp, do not guess.** Every existing `content` row is stamped with the qualifier of
  the peer's **installed base cell** for that row's `h_app_id` role, read from
  `app_info`   at migration time — correct for the entire pre-contract corpus because no
  path   recorded a clone-authored row *as* clone-authored. That omission is the defect.
- **Refuse ambiguity.** If the role resolves to no provisioned cell, or to more than
  one,   migration refuses and leaves those rows NULL. Under §3, NULL   means
  base-cell-only: un-migrated rows remain safe but unimproved.
- **The key change is a table rebuild.** SQLite cannot widen a primary key in place; the
  migration creates the new table, copies, and swaps, inside one transaction. Two rows
  cannot collapse to the same new key (the old key was a   strict prefix of the new
  one).
- **Sync docs are re-derived, not renamed.** Migration clears the partition's
  content-doc space in sled; partition-scoped back-fill (§5) writes qualified ids. An
  unqualified `node:{id}` doc surviving   in the store is inert — no consumer looks it
  up — and is swept on the next reset.
- **Diesel migration timestamps must not collide** with siblings landing in the same
  window; two migrations sharing a `YYYY-MM-DD-HHMMSS` prefix silently lose one.

## 7. Mixed-version safety of the wire change

**The additive-`serde(default)` recipe does not apply here.** The correction contract's
notification reference is safe because its message is map-keyed
(`rmp_serde::to_vec_named`). The affected planes use positional encoding:
`sync_protocol.rs:358,382` and `shard_protocol.rs:200,224` both encode with
`rmp_serde::to_vec`, which is **compact** — structs and enum payloads become positional
arrays, so field position, not field name, is the contract.

Wire rules:

- **A new field is never appended to an existing compact-encoded struct** —
  `ContentInventoryItem` (`shard_protocol.rs:51-60`) and `ContentRecord`
  (`shard_protocol.rs:63-79`) are frozen in shape.
- **The qualifier rides a new message variant appended last**, following the existing
  precedent: `ShardRequest::GetManifest` is documented as "appended last so an   older
  peer never receives it unless it asked (positional codec)"
  (`shard_protocol.rs:44-48`). A pre-contract peer never sends the new variant or
  receives the new reply. Its enumeration still works on old variants.
- **The sync-plane request/response family already carries `h_app_id` and `doc_id`**
  positionally (`sync_protocol.rs:50-52` and siblings). The qualifier travels inside the
  `doc_id` string (§2.2), without changing sync message shapes. A pre-contract peer
  reading a qualified doc id sees an opaque id it does not have, requests it, and stores
  it under that id — inert on its side, and harmlessly re-derived when it upgrades.
- **What a pre-contract peer does with a qualified row**: the   old variant strips the
  qualifier, preserving defective behaviour, including base-cell re-authoring. Upgrades
  close the escape peer-by-peer, not atomically.   Upgraded peers lose the defect; a
  mixed mesh remains unprotected.
- **Every wire change ships with a both-directions byte-compat test** — old bytes
  decoded   by the new build and new bytes decoded by a struct lacking the field —
  because on a   compact codec neither direction follows from the field's attributes
  alone.

## 8. Design-gate answers (P2P design gate)

| Entity | Class | Identity | Creator / zome / hash effect | Projection |
|---|---|---|---|---|
| `content` projection row + qualifier columns | **C (Ephemeral)** — rebuilt from the named cell's DHT (§5); the qualifier changes the key, not the class | composite `(h_app_id, cell_dna_hash, cell_clone_id, id)` — the gate's Option 3 (slug/composite rather than content-derived CID or agent-scoped tuple), justified: the row is not itself content-addressed, it is an index over content whose own identity is the entry hash | storage migration only; no zome touched; **DNA-hash-NEUTRAL** | n-a (this *is* the projection) |
| Automerge content-sync doc | **C** — derived from the row, reconstructable by the partition-scoped back-fill | doc id `node:{dna}:{clone}:{id}` under namespace `"elohim"` — Option 3, same justification | `sync/projector.rs` producer; no zome; **DNA-hash-NEUTRAL** | DocStore key `{h_app_id}:{doc_id}` (`doc_store.rs:147`); reach tier **unresolved — reach vocabulary in declared drift** |
| The re-author path's fresh DHT action | **A (Notarized)** on the existing `Content` entry type in `content_store_integrity` — no new type | for the entry, `cid = entry_hash`; the action hash is only ever `dht_anchor_hash` | `content_store::create_content` via `conductor_writes::call_create_content` (`:360-368`); **DNA-hash-NEUTRAL** — the refusal is a storage-side gate, not a validation rule | `ContentCommitted` → `upsert_with_anchor` (`rea_projection.rs:723-770`) |

**Head-plane cost.** The two class-C entities add zero heads. The third reduces head
cost: pre-contract, every clone write crossing the projection can mint one new A-class
`Content` head **per receiving peer**, forever. The measured run produced one head on
one peer within eight minutes from two probe writes; the short window limited the
result. Cost is unbounded in (clone writes × peers). The design gate's head-plane cost
budget (Step 1.5: declare item count at seed and at one year, and explain unbounded
counts' effect on quiesce) forbids accepting this growth unchecked. The refusal makes
the count zero. Reducing heads needs no bundling justification.

**Network stakes.** The qualifier comparison in §3 is **floor-protected**: it does not
cheapen at `Simulacra`. Fixture and genesis-stage peers run clones. A stage-priceable
check would therefore be disabled where the defect occurs.

**Anti-patterns checked.** No new entry type; no UUID identity (the qualifier is a
Holochain `DnaHash` carried whole, never string-compared across namespaces); no route
designed first (this contract adds none); no k8s-plane modelling. The gate corrected one
anti-pattern: the first sketch qualified the **sync namespace** rather than the doc id,
which would have broken the pinned `PROJECTION_NAMESPACE` wire contract and silently
un-synced every peer.

## 9. Stations

The atom's *done when* is: "the identity diff on a receiving peer's base cell is EMPTY
across all six planes after a clone write, with the projection/sync rows carrying a cell
qualifier; the harness (`genesis/a2o/scripts/fixtures-clone.mjs`) is the check." The
three requirements map to stations: row qualifiers to stations 1–3, the empty base-cell
diff to stations 4–5, and the harness to station 8. **Stations 6 and 7 are not entailed
by that sentence** — they add §4's partition honesty and §5's per-partition rebuild.
Naming these commitments as stations makes them measurable. The frontmatter's
Active→Canonical trigger requires all eight.

1. **The qualifier is sourced.** `HcClient` exposes `(cell_dna_hash, cell_clone_id)`
   from    `app_info`, distinguishing `Provisioned` from `Cloned`. Check: two
   `HcClient`s built    against `lamad` and `lamad.fixtures` report different
   qualifiers.
2. **The row carries it.** `content` has both columns, the widened key, and
   `upsert_with_anchor` matches on it. Check: the harness's `projection.json` rows carry
   a    non-NULL `cell_dna_hash` after a clone write.
3. **The doc carries it.** `content_doc_id` renders the qualified id and the producer
   writes it. Check: the harness's `sync.json` documents are qualified, and the two
   probes appear under the clone's qualifier, not the base's.
4. **The re-author refuses.** Both call sites in §2.3 compare before authoring.    A
   `foreign-cell` refusal is terminal. Check: on a receiving peer, `base-actions` and
   `base-ops` diffs are EMPTY after the clone write — the plane where the measured
   escape appeared.
5. **The remaining base planes stay empty.** `base-content` and `targets` diffs EMPTY on
   the receiving peer, confirming no derived base-cell state followed the refused write.
6. **Partition states are shown, not swallowed.** A row whose qualifier names an
   uninstalled cell reads `pending — foreign cell`; two qualifiers on one id read
   conflict-visible. Check: a deliberate second-clone write produces a conflict-visible
   id    rather than an overwrite.
7. **Rebuild is per-partition and atomic.** Resetting the base partition leaves clone
   partitions served; resetting a clone partition clears its rows, docs and cursors
   together. Check: a rebuild of one partition changes no row of the other.
8. **The harness is the standing check.** `fixtures-clone.mjs` asserts stations 4 and 5
   as EMPTY across all six inventories (`*-actions`, `*-ops`, `*-content`, `targets`,
   `projection`, `sync`) and fails on any non-empty base-cell delta, with a receipt
   under    `genesis/a2o/reports/fixtures-clone/`.

## 10. What this contract does not decide

Whether group clone spaces ship — household, collective, or fixtures — remains D6's
operator-selected candidacy, gated by D0, D3 and a measured phone/hub budget. Which
spaces an operator selects, or how a person joins one. Whether storage runs one instance
per cell rather than one instance across a peer's cells; this contract makes shared
instances correct without claiming optimality. The clone-enumeration gap in
`sync_coordinators`. Cross-space evidence, the action-identity-to-content-identity rule
D4 also names, and reach semantics across cells. These remain open; none blocks closing
the escape.
