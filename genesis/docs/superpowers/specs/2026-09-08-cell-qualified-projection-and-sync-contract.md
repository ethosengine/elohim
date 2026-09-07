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
  - "ai-stewarded-commons-reimplementation-plan | §0.5 supplies the partition vocabulary this contract uses (pending / conflict-visible) when currentness cannot be checked | sha256:bf2f1a4c94e70670 | path: genesis/docs/content/elohim-protocol/architecture/2026-09-06-ai-stewarded-commons-reimplementation-plan.md"
  - "package-composition-at-the-holochain-seam | names this contract as what makes its D3 rule enforceable rather than declared; the packaging rules whose clone-cell assumptions this contract constrains | sha256:e417e740ce2212ce | path: genesis/docs/superpowers/specs/2026-09-08-package-composition-at-the-holochain-seam.md"
  - genesis/data/timeline/backlog/clone-content-escapes-via-shared-projection.md
---

# Cell-qualified projection and sync — the contract

**Vocabulary this contract rests on.** A Holochain hApp installs one **cell** per declared
**role** — a cell being one agent's participation in one DNA's validation context and its
gossip network. An operator may additionally **clone** a role: the conductor installs a
second, independent cell under the same role name, with its own DNA hash and its own
separate DHT. `elohim-storage` reaches those cells through the local **conductor** and
projects what it reads into local SQL and an Automerge sync store. The **six planes** the
fixtures-clone harness inventories are the three source-chain views of a cell
(`*-actions`, `*-ops`, `*-content`), the DHT `targets` view, the storage `projection`
rows, and the `sync` documents. The **D-numbers** used below (D0, D3, D4, D6) index the
ten sealed decisions of the holons-are-spaces companion, §0b of that document.

The 2026-09-07 household-mesh run measured what the clone experiment was meant to
rule out. Content written into a `lamad` clone stayed inside that clone's DHT on the
authoring peer — `base-actions`, `base-ops`, `base-content` and `targets` diffs were
all empty — and then escaped anyway: two rows landed in the peer's shared `content`
projection carrying `h_app_id: "lamad"` and no cell qualifier, two docs landed in the
`elohim` sync namespace, and eight minutes later a *receiving* peer's storage had
re-authored one of them into its own base `lamad` cell under a fresh DHT action. Clone
isolation holds for the authoring peer's DHT and for nothing else.

This contract names the qualifier that closes that path, where it is sourced, and what
each plane does when it is absent or names a cell this peer does not have. Every line
names the code it binds. **Sections 1 through 7 state required target behaviour, not
current behaviour**; where the code already does what is required the citation says so,
and where it does not the gap is named. Nothing here is claimed as delivered.

## 1. The qualifier and its authority

**The qualifier is `(cell_dna_hash, cell_clone_id)`**, two columns rather than one string,
so neither can be parsed out of the other:

- `cell_dna_hash` — the Holochain `DnaHash` of the cell that authored the record, full
  39-byte `uhC0k…` form, base64. Not truncated: the eight-byte prefix in the connect log
  line (`hc_client.rs:372`) is a log affordance, never an identity. This is the
  **discriminating** half — a clone carries distinct DNA modifiers, so its hash differs
  from its role's provisioned cell.
- `cell_clone_id` — the clone's `clone_id` string (`role.N`) when the record came from a
  cloned cell, NULL when it came from the role's provisioned cell. This is the
  **addressing** half: it is what names a cell in a refusal message and what a redirect
  (§3, case 2) resolves an `HcClient` against.

**The only admissible source is the conductor's own `app_info`.** `HcClient::connect`
reads it: `list_apps` → `app_info.cell_info[role_name]` →
`cell_discovery::select_target_cell` (`hc_client.rs:340-369`, `cell_discovery.rs:212-226`),
which returns a `CellId` for a `Provisioned` cell when the target is a bare role and for
a `Cloned` cell when the target is `role.clone` (`cell_discovery.rs:196-210` parses that
form). **Half of the qualifier is already retained and half is not.** `HcClient` keeps the
`CellId` and exposes its DNA hash (`hc_client.rs:655`, `:672-674`), so `cell_dna_hash`
needs no new plumbing. `select_target_cell` reads `c.clone_id` / `c.name` to pick the
`Cloned` branch (`cell_discovery.rs:221-225`) and then **discards it**, returning a bare
`CellId` that carries no clone concept — a `grep` for `clone_id` in `hc_client.rs` finds
nothing. Retaining it is station 1's work; once retained, the qualifier is read from there
and nowhere else.

**A role name is not a qualifier.** `h_app_id` is an app-scope string on the projection
(`db/context.rs:51-57`; `AppContext` carries `h_app_id` and a libp2p peer id, and no cell
identity at all) and `AppContext::default_lamad()` hardcodes `"lamad"`
(`db/context.rs:69-71`). Every clone of the `lamad` role produces the same `h_app_id`.
That is the measured collision, not an incidental one: role identity is one-to-many over
cells, so no rule built on it can separate them.

**Peer-supplied qualifiers are claims, never authority.** A qualifier that arrives over
the sync plane, the shard plane, or view-federation is a *routing hint* — it says which
cell the sender says a record came from. It is admissible for refusal (see §3) and for
display. It is never admissible as provenance: only this peer's own conductor may stamp
a qualifier that grants a row notarization authority, exactly as `dht_anchor_hash` is
only ever written by conductor-verified paths (`sync/projector.rs:632-642`, the REQ-N5
guard).

## 2. Durable source and what carries the qualifier

### 2.1 The storage `content` projection row

`content` is declared `-- Source of truth: DHT (Content entry in lamad DNA).
Classification: A.` (`migrations/2026-01-08-000000_initial/up.sql:46-50`) with
`id TEXT PRIMARY KEY` and `h_app_id` as an ordinary column. That comment classifies the
*entry*, not the *row*: the DHT `Content` entry is class A and this table is the class-C
rebuildable index over it — a distinction that matters here, because only the class-C
reading admits the per-partition reset of §5. The second fact matters more: **`id` alone
being the primary key means two cells authoring the same content id cannot coexist as
rows** — the second write silently becomes an update of the first. The qualifier is not a
decorative column; it is a key change.

Required: `content` gains `cell_dna_hash TEXT` and `cell_clone_id TEXT`, and its logical
key becomes `(h_app_id, cell_dna_hash, cell_clone_id, id)`. `upsert_with_anchor`
(`db/content_diesel.rs:1083`) matches, pre-contract, on `h_app_id` and `id` only
(`:1094-1099`); it takes the qualifier as an argument and matches on the full key. The
`ContentCommitted` arm that calls it (`rea_projection.rs:723-770`) supplies the qualifier
of the `HcClient` whose conductor emitted the signal — post-commit signals are cell-local,
so that is the authoring cell by construction.

### 2.2 The Automerge content-sync doc

The content-sync producer writes one doc per content row at `node:{id}`
(`sync/projector.rs:22-25`) under the `"elohim"` namespace
(`sync/projector.rs:932`, `PROJECTION_NAMESPACE`), which `initiate_sync_round` is the sole
lister of (`sync/projector.rs:250-254`, pointing at `p2p/mod.rs:6996`). The DocStore keys
on `{h_app_id}:{doc_id}` (`sync/doc_store.rs:147`). Two clones of one role therefore
collide on the doc key as surely as on the row key.

Required: the doc id becomes cell-qualified — `node:{cell_dna_hash}:{cell_clone_id|-}:{id}` —
and the namespace stays `"elohim"`. Qualifying the doc id rather than the namespace is
deliberate: the namespace is a pinned wire contract with a guard test
(`projection_namespace_is_wire_contract`, `sync/projector.rs:925-932`) and moving it
requires every peer and the DNA to migrate in lockstep, while doc ids are already
free-form and prefix-typed (`sync/doc_store.rs:298-315` infers doc type from the prefix,
which `node:` still satisfies). `content_doc_id` becomes the one function that renders
the qualified id, and `reverse_project_content_doc` (`sync/projector.rs:643`, whose
prefix-strip is at `:648-651`) parses it back — refusing, not guessing, when the parse
fails.

### 2.3 The re-author path's fresh DHT action

The escape's last step is a real Holochain act. **Two** call sites author content through
the conductor, and both target whatever cell the `HcClient` was constructed against, which
for storage is the `lamad` **role**, i.e. the base cell
(`services/conductor_writes.rs:62-66`, `:360-368`). Both live inside one method,
`ContentService::update_via_conductor`; everything else reaches the conductor through it:

- The NULL-anchor branch — a row with no `dht_anchor_hash` is bootstrapped by
  `call_create_content` from the SQL row (`services/content_service.rs:385-386`).
- The stale-anchor heal — an anchored row the conductor cannot resolve is re-published by
  `call_create_content` (`services/content_service.rs:425-427`).

Two sweeps drive those branches and are therefore covered by the gate in §3 through them,
not separately: the projection-reconcile content arm's `AdoptOutcome::Author` branch,
which sends an empty patch through `update_via_conductor` precisely to trip the heal
(`p2p/projection_reconcile.rs:2638-2675`), and the startup pass that walks every
NULL-anchor row (`services/reanchor_backfill.rs:1-31`). Each sweep must also refuse
*before* it spends a conductor round-trip, so the comparison is duplicated at both sweep
entry points as a budget measure, never as a second authority.

Both branches mint a **new** `Content` Create in the base cell, and the resulting
`ContentCommitted` signal stamps a base-cell `dht_anchor_hash` over a row whose bytes came
from a clone. That is the measured cross-peer write amplification.

## 3. Validation contract — the re-author refusal rule

Required, and not the pre-contract behaviour: **storage never authors content into a cell
other than the one whose qualifier the row carries.** Before either call site in §2.3
issues a conductor write, it compares the row's `(cell_dna_hash, cell_clone_id)` against
the qualifier of the `HcClient` it is about to call (`hc_client.rs:655`, `:672-674`).
Three outcomes, and only three:

1. **Equal** — proceed. This is every write on the peer's own base corpus, unchanged.
2. **Different, and this peer has the named cell** — the write is redirected to an
   `HcClient` bound to that cell, or, when no such client is available, refused as
   `wrong-cell` and left `pending` (§4). It is never redirected to the base cell.
3. **Different, and this peer does not have the named cell** — refused as `foreign-cell`,
   permanently for this peer. A `foreign-cell` refusal is a **positive** outcome, not a
   transient failure: it is not retried on the next sweep, and it does not count toward
   the reconcile arm's gap or divergence counters.

**A row with no qualifier is legacy, and legacy means base cell only.** A NULL
`cell_dna_hash` is read as "the provisioned cell of this row's `h_app_id` role on this
peer" for the purpose of the comparison above, and for nothing else. It is never read as
"any cell" and never as a wildcard match. Once migrated (§6) no new row is written
without a qualifier; a NULL qualifier appearing after migration is a defect to log, not a
state to serve.

**The refusal binds the sync plane's consumer too.** `reverse_project_content_doc` already
refuses to launder converged doc fields into notarization provenance
(`sync/projector.rs:609-642`). It gains the sibling refusal: a doc whose qualifier does
not match the row's qualifier heals nothing, and a doc whose qualifier names a cell this
peer does not have is never projected into a row at all.

## 4. Partition behaviour

Required behaviour. The vocabulary is sealed decision D4's: **pending** and
**conflict-visible** when currentness cannot be checked (reimplementation plan §0.5).
Four states, three of them pending:

- **The qualifier's cell is not installed locally.** The record is retained and shown as
  `pending — foreign cell`, with the qualifier displayed. Nothing is authored, nothing is
  anchored, and no reconcile arm treats it as a gap it can close. This is the honest
  state: the peer holds bytes it can neither witness nor disprove.
- **The qualifier's cell is installed but no `HcClient` is bound to it** — §3's
  `wrong-cell` refusal. `pending — wrong cell`, retried once a client for that cell exists.
  Distinct from the state above (no cell) and the one below (cell and client, no service).
- **The qualifier's cell is installed but the conductor cannot reach it** (cells
  disabled during a boot window — the condition `reanchor_backfill` exists for,
  `services/reanchor_backfill.rs:9-20`). `pending — cell unavailable`, retried with the
  existing bounded backoff. This is a liveness failure, distinct from the one above.
- **Two rows share `(h_app_id, id)` under different qualifiers.** Both are retained and
  the id is **conflict-visible**: readers see both, each labelled with its cell, and
  neither is deterministically promoted. There is no last-arrival winner, because the
  cells are different validation contexts and no ordering between them is authoritative.
- **The qualifier is absent and the row is anchored.** The row is legacy-but-witnessed:
  it is served, and it is a migration candidate (§6), not a conflict.
- **A qualifier arrives that contradicts a row's existing anchored qualifier.** Rejected,
  not retried. A *positive mismatch* — evidence that the claim is wrong — is a different
  thing from an absence of evidence, and only the second is worth retrying; retrying the
  first burns budget forever on an answer that will not change. This is the discipline the
  correction contract applies when an acceptance names the wrong author.

## 5. Rebuild path

Derived tables reset and re-derive **per cell**, atomically with their consumption state.

- The unit of rebuild is one `(h_app_id, cell_dna_hash, cell_clone_id)` partition. A
  rebuild of the base partition never clears clone partitions, and vice versa.
- A rebuild is either a fresh partition built to completion then atomically swapped in,
  or an in-place reset that serialises live projection writers and clears the partition's
  `content` rows, sync docs, and per-partition reconcile cursors **in one transaction**.
  Clearing rows without their cursors is what makes a half-rebuilt partition look converged.
- Readers see `rebuilding` for that partition until every retained input has been replayed;
  other partitions stay served throughout. Equality after rebuild = identical canonical
  logical rows (id, qualifier, anchor, declared head, reach, body address), excluding
  SQLite layout and operational timestamps.
- The corpus back-fill that seeds the DocStore from SQL (`sync/projector.rs:515-530`)
  becomes partition-scoped: it pages one partition and writes qualified doc ids, so a
  back-fill can no longer merge two cells' corpora into one doc space.

## 6. Migration shape for existing rows

- **Stamp, do not guess.** Every existing `content` row is stamped with the qualifier of
  the peer's **installed base cell** for that row's `h_app_id` role, read from `app_info`
  at migration time — correct for the whole pre-contract corpus by construction, since no
  pre-contract path wrote a clone-authored row *as* clone-authored. That is the defect.
- **Refuse ambiguity.** If the role resolves to no provisioned cell, or to more than one,
  the migration refuses and leaves those rows NULL rather than stamping a guess. NULL then
  means base-cell-only under §3, so an un-migrated row is safe, merely unimproved.
- **The key change is a table rebuild.** SQLite cannot widen a primary key in place; the
  migration creates the new table, copies, and swaps, inside one transaction. Two rows
  that collapse to the same new key are impossible by construction (the old key was a
  strict prefix of the new one).
- **Sync docs are re-derived, not renamed.** The migration does not rewrite doc ids in
  sled. It clears the content-doc space for the partition and lets the partition-scoped
  back-fill (§5) rewrite it under qualified ids. An unqualified `node:{id}` doc surviving
  in the store is inert — no consumer looks it up — and is swept on the next reset.
- **Diesel migration timestamps must not collide** with siblings landing in the same
  window; two migrations sharing a `YYYY-MM-DD-HHMMSS` prefix silently lose one.

## 7. Mixed-version safety of the wire change

**The additive-`serde(default)` recipe does not transfer here, and assuming it would is
the trap.** The correction contract's notification reference is safe because its message
is map-keyed (`rmp_serde::to_vec_named`). The two planes this contract touches are not:
`sync_protocol.rs:358,382` and `shard_protocol.rs:200,224` both encode with
`rmp_serde::to_vec`, which is **compact** — structs and enum payloads become positional
arrays, so field position, not field name, is the contract.

The rule that follows:

- **A new field is never appended to an existing compact-encoded struct** —
  `ContentInventoryItem` (`shard_protocol.rs:51-60`) and `ContentRecord`
  (`shard_protocol.rs:63-79`) are frozen in shape.
- **The qualifier rides a new message variant appended last**, which is the precedent this
  wire already set: `ShardRequest::GetManifest` is documented as "appended last so an
  older peer never receives it unless it asked (positional codec)"
  (`shard_protocol.rs:44-48`). A pre-contract peer never sends the new variant, so it never
  receives the new reply, and its enumeration continues to work on the old variants.
- **The sync-plane request/response family already carries `h_app_id` and `doc_id`**
  positionally (`sync_protocol.rs:50-52` and siblings). The qualifier travels inside the
  `doc_id` string (§2.2), so no sync message shape changes at all. A pre-contract peer
  reading a qualified doc id sees an opaque id it does not have, requests it, and stores
  it under that id — inert on its side, and harmlessly re-derived when it upgrades.
- **What a pre-contract peer does with a qualified row**: it receives the row through the
  old variant with the qualifier stripped, so it keeps the defective behaviour — including
  re-authoring into its base cell. The escape closes peer-by-peer as peers upgrade, not
  atomically; this contract removes the defect from upgraded peers and does not protect a
  mixed mesh.
- **Every wire change ships with a both-directions byte-compat test** — old bytes decoded
  by the new build and new bytes decoded by a struct lacking the field — because on a
  compact codec neither direction follows from the field's attributes alone.

## 8. Design-gate answers (P2P design gate)

| Entity | Class | Identity | Creator / zome / hash effect | Projection |
|---|---|---|---|---|
| `content` projection row + qualifier columns | **C (Ephemeral)** — rebuilt from the named cell's DHT (§5); the qualifier changes the key, not the class | composite `(h_app_id, cell_dna_hash, cell_clone_id, id)` — the gate's Option 3 (slug/composite rather than content-derived CID or agent-scoped tuple), justified: the row is not itself content-addressed, it is an index over content whose own identity is the entry hash | storage migration only; no zome touched; **DNA-hash-NEUTRAL** | n-a (this *is* the projection) |
| Automerge content-sync doc | **C** — derived from the row, reconstructable by the partition-scoped back-fill | doc id `node:{dna}:{clone}:{id}` under namespace `"elohim"` — Option 3, same justification | `sync/projector.rs` producer; no zome; **DNA-hash-NEUTRAL** | DocStore key `{h_app_id}:{doc_id}` (`doc_store.rs:147`); reach tier **unresolved — reach vocabulary in declared drift** |
| The re-author path's fresh DHT action | **A (Notarized)** on the existing `Content` entry type in `content_store_integrity` — no new type | for the entry, `cid = entry_hash`; the action hash is only ever `dht_anchor_hash` | `content_store::create_content` via `conductor_writes::call_create_content` (`:360-368`); **DNA-hash-NEUTRAL** — the refusal is a storage-side gate, not a validation rule | `ContentCommitted` → `upsert_with_anchor` (`rea_projection.rs:723-770`) |

**Head-plane cost.** The two class-C entities add zero heads. The third is where the
budget actually moves, and it moves *downward*: pre-contract, every clone write crossing the
projection can mint one new A-class `Content` head **per receiving peer**, forever — the
measured run produced one such head on one peer within eight minutes from two probe
writes, partial only because the window was short. Unbounded in (clone writes × peers),
which is exactly the shape the design gate's head-plane cost budget (its Step 1.5: declare
item count at seed and at one year, and say what an unbounded count does to quiesce) says
must not be waved through. The refusal makes the count zero. No bundling justification is
needed for a change whose head-plane effect is subtraction.

**Network stakes.** The qualifier comparison in §3 is **floor-protected**: it does not
cheapen at `Simulacra`. A fixture or genesis-stage peer is precisely the peer running
clones, so a stage-priceable qualifier check would be disabled exactly where the defect
lives.

**Anti-patterns checked.** No new entry type; no UUID identity (the qualifier is a
Holochain `DnaHash` carried whole, never string-compared across namespaces); no route
designed first (this contract adds none); no k8s-plane modelling. One was caught and
corrected during the gate: the first sketch qualified the **sync namespace** rather than
the doc id, which would have broken the pinned `PROJECTION_NAMESPACE` wire contract and
silently un-synced every peer.

## 9. Stations

The atom's *done when* is: "the identity diff on a receiving peer's base cell is EMPTY
across all six planes after a clone write, with the projection/sync rows carrying a cell
qualifier; the harness (`genesis/a2o/scripts/fixtures-clone.mjs`) is the check." It names
three things, and each maps onto stations here: the qualifier on the rows is stations 1–3,
the empty base-cell diff is stations 4–5, and the harness is station 8. **Stations 6 and 7
are not entailed by that sentence** — they are this contract's own additional commitments
(§4's partition honesty and §5's per-partition rebuild), and they are named as stations so
they are measured rather than assumed. The frontmatter's Active→Canonical trigger requires
all eight.

1. **The qualifier is sourced.** `HcClient` exposes `(cell_dna_hash, cell_clone_id)` from
   `app_info`, distinguishing `Provisioned` from `Cloned`. Check: two `HcClient`s built
   against `lamad` and `lamad.fixtures` report different qualifiers.
2. **The row carries it.** `content` has both columns, the widened key, and
   `upsert_with_anchor` matches on it. Check: the harness's `projection.json` rows carry a
   non-NULL `cell_dna_hash` after a clone write.
3. **The doc carries it.** `content_doc_id` renders the qualified id and the producer
   writes it. Check: the harness's `sync.json` documents are qualified, and the two
   probes appear under the clone's qualifier, not the base's.
4. **The re-author refuses.** Both call sites in §2.3 compare before authoring, and a
   `foreign-cell` refusal is terminal. Check: on a receiving peer, `base-actions` and
   `base-ops` diffs are EMPTY after the clone write — the plane where the measured
   escape appeared.
5. **The remaining base planes stay empty.** `base-content` and `targets` diffs EMPTY on
   the receiving peer, confirming no derived base-cell state followed the refused write.
6. **Partition states are shown, not swallowed.** A row whose qualifier names an
   uninstalled cell reads `pending — foreign cell`; two qualifiers on one id read
   conflict-visible. Check: a deliberate second-clone write produces a conflict-visible id
   rather than an overwrite.
7. **Rebuild is per-partition and atomic.** Resetting the base partition leaves clone
   partitions served; resetting a clone partition clears its rows, docs and cursors
   together. Check: a rebuild of one partition changes no row of the other.
8. **The harness is the standing check.** `fixtures-clone.mjs` asserts stations 4 and 5
   as EMPTY across all six inventories (`*-actions`, `*-ops`, `*-content`, `targets`,
   `projection`, `sync`) and fails on any non-empty base-cell delta, with a receipt under
   `genesis/a2o/reports/fixtures-clone/`.

## 10. What this contract does not decide

Whether group clone spaces ship at all — household, collective, or fixtures — which is
D6's operator-selected candidacy, gated behind D0, D3 and a measured phone/hub budget.
Which spaces an operator selects, or how a person joins one. Whether storage runs one
instance per cell rather than one instance across a peer's cells; this contract makes the
shared-instance shape correct and does not argue it is optimal. The clone-enumeration gap
in `sync_coordinators`. Cross-space evidence, the action-identity-to-content-identity rule
D4 also names, and reach semantics across cells. Those remain open, and none of them is a
precondition for closing the escape.
