---
title: "Cell-qualified projection and sync — the contract that stops clone-authored content from being re-authored into a peer's base cell"
id: cell-qualified-projection-and-sync-contract
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
graduation-trigger: "Draft→Active when the adversarial read and a second p2p-design-gate pass are addressed in the text and the ten stations of §9 are written as a2o scenarios bound to @concern:cell-qualified-projection under genesis/a2o/features/dataplane/ — adversarial read: done 2026-09-08 (§11); second gate pass: done 2026-09-08 (§8, re-answered against the revised design); stations-as-scenarios: written 2026-09-08 (all @wip). Active→Canonical when ALL TEN stations pass — the atom's own done-when (stations 1-5: EMPTY identity diff on a RECEIVING peer's base cell across all six planes after a clone write, qualifier carried on projection and sync rows) plus this contract's five additional commitments (station 6 partition honesty, 7 per-partition atomic rebuild, 8 UNKNOWN on unqualified ingest, 9 migration default, 10 clone liveness) — with a receipt under genesis/a2o/reports/ and the DELTA on the habit atom"
actor: agent:rust-architect@fable-5.1
written: 2026-09-08
revised: 2026-09-08
domain: D6
habits: [dataplane-convergence, happ-lineage-migration]
boundary: "Design only. This contract governs ONE qualifier (the cell a projected row or sync doc came from), the FOUR records that must carry or verify it (the storage `content` projection row, the Automerge content-sync doc, the shard/replication plane's ingested record, and the re-author path's fresh DHT action), and the refusal rule between them. It does not decide whether group clone spaces ship (D6), which spaces an operator selects, or whether storage runs one instance per cell."
cites:
  - "accountable-correction-contract | the sibling slice-0 contract this one mirrors in structure; its additive map-keyed wire recipe is the one §7 says does NOT transfer to the compact-encoded sync and shard planes | sha256:691e8b89f394214c | path: genesis/docs/superpowers/specs/2026-09-06-accountable-correction-contract.md"
  - "holons-are-spaces-how-we-use-holochain | sealed decisions D0-D10; D4 supplies this contract's named-successor-authority and stated-partition-behaviour requirement, D6 the clone-space candidacy this contract gates | sha256:ac1de36d2423be82 | path: genesis/docs/content/elohim-protocol/architecture/2026-09-06-holons-are-spaces-how-we-use-holochain.md"
  - "ai-stewarded-commons-reimplementation-plan | §0.5 supplies the partition vocabulary this contract uses (pending / conflict-visible) when currentness cannot be checked | sha256:bf2f1a4c94e70670 | path: genesis/docs/content/elohim-protocol/architecture/2026-09-06-ai-stewarded-commons-reimplementation-plan.md"
  - "package-composition-at-the-holochain-seam | names this contract as what makes its D3 rule enforceable rather than declared; the packaging rules whose clone-cell assumptions this contract constrains | sha256:e417e740ce2212ce | path: genesis/docs/superpowers/specs/2026-09-08-package-composition-at-the-holochain-seam.md"
  - genesis/data/timeline/backlog/clone-content-escapes-via-shared-projection.md
---

# Cell-qualified projection and sync — the contract

**Vocabulary.** A Holochain hApp installs one **cell** per declared **role**. Each cell is one
agent's participation in one DNA's validation context and gossip network. An operator may
**clone** a role: a second cell with the same role name, its own DNA hash, and a separate DHT.
`elohim-storage` reads these cells through the local **conductor**. It projects those reads
into local SQL and an Automerge sync store. The fixtures-clone harness inventories **six
planes**: three source-chain views (`*-actions`, `*-ops`, `*-content`), the DHT `targets`
view, storage `projection` rows, and `sync` documents. **D-numbers** index the ten sealed
§0b decisions in the holons-are-spaces companion.

The 2026-09-07 household-mesh run exposed the escape. A `lamad` clone write stayed within
the authoring peer's clone DHT: all four DHT-view diffs were empty. But storage leaked it.
Two unqualified rows entered the shared `content` projection. Two docs entered the `elohim`
sync namespace. Eight minutes later, a *receiving* peer's storage re-authored one doc into
its base `lamad` cell under a fresh DHT action. Clone isolation holds only for the authoring
peer's DHT.

This contract defines the qualifier that closes the escape. It names its source on each
row-creating plane and each plane's response to absent, unverifiable, or foreign qualifiers.
**Sections 1–7 state required target behaviour, not current behaviour.**

## 1. The qualifier, its authority, and its verification

**The qualifier is the cell's `DnaHash`**: full 39-byte `uhC0k…` form, base64, never truncated.
The eight-byte prefix at `hc_client.rs:371-373` is for logging, never identity. This is the
**discriminating and global** half of a `CellId`. A clone's distinct DNA modifiers give it
a different hash from its role's provisioned cell. The hash binds the clone's network seed.
Thus **two peers in the same clone DHT compute the same hash**, usable as a key and on the wire.

**`clone_id` is addressing, never identity.** The conductor's `role.N` is a per-installation
counter. Peers in one clone DHT normally have different indices (`lamad.1` vs `lamad.4`).
A peer MAY keep a local `cell_clone_id` column to resolve its `HcClient` and name cells in refusals.
It **MUST NOT** enter row keys, wire doc ids, or cross-peer comparisons. A global key with
local addressing creates divergence between peers of one DHT.

**Two authorities, never confused.**

- **Attribution** identifies which cell authored a record this peer wrote. It comes from
  `Signal::App { cell_id, .. }`. `AppWebsocket::on_signal` delivers signals from *every* app
  cell, including clones (documented at `hc_client.rs:1037`). Cell-local *emission* does not
  mean cell-local *delivery*. All four subscriptions discard the cell: `hc_client.rs:814`,
  `:891`, `:965`, `:1058` match `Signal::App { signal, .. }`. That discard causes the
  attribution defect. The conductor-stamped `cell_id` is already on the wire; no new plumbing.
- **Enumeration** identifies cells *this peer holds*. Its source is `app_info`: `list_apps` →
  `cell_info[role_name]` → `cell_discovery::select_target_cell` (`hc_client.rs:340-369`,
  `cell_discovery.rs:212-226`). This distinguishes `Provisioned` from `Cloned`
  (`cell_discovery.rs:196-210`, `:221-224`), then discards clone identity in the returned
  `CellId`. Enumeration runs at boot and every `app_info` change. Each locally held clone
  therefore has a clone-bound `HcClient`.

**A role name is not a qualifier.** `AppContext` carries `h_app_id` and a libp2p peer id,
without cell identity (`db/context.rs:50-57`). `default_lamad()` hardcodes `"lamad"`
(`:69-71`). Every clone therefore shares `h_app_id`: the measured collision.

**A received qualifier is a claim until verified.** **VERIFIED** requires both local
`app_info` enumeration of that DNA hash **and** resolution of the record's claimed DHT
anchor in that cell. The action's DHT space *is* its provenance. An unverifiable qualifier
is **UNVERIFIED**, treated as `foreign-cell` in every authoring decision. Its row remains a
**replica that is never re-authored**. Only the local conductor stamps VERIFIED.
`dht_anchor_hash` is written only through conductor-verified paths
(`sync/projector.rs:632-642`, REQ-N5).

## 2. Every plane that can create or move a row

### 2.1 The storage `content` projection row

`content` declares `Source of truth: DHT … Classification: A`
(`migrations/2026-01-08-000000_initial/up.sql:46-50`), with `id TEXT PRIMARY KEY` and ordinary
column `h_app_id`. The comment classifies the *entry* as A. The *table* is its class-C
rebuildable index. **With `id` alone as key, two cells' same-id content cannot coexist.**
The second write silently updates the first.

Required: `content` gains `cell_dna_hash TEXT` (NULL = unknown) and `cell_provenance TEXT`
(`verified` | `unknown`). It MAY gain local-only `cell_clone_id TEXT` for addressing.
The **logical key becomes `(h_app_id, cell_dna_hash, id)`**, without a clone segment.
`upsert_with_anchor` (`db/content_diesel.rs:1083`) currently matches only `h_app_id` and `id`
(`:1094-1099`). It must accept the qualifier and match the full key. Its `ContentCommitted`
caller (`rea_projection.rs:723-770`, call at `:762`) supplies the signal's `Signal::App`
**`cell_id`** DNA hash, never the receiving `HcClient`'s cell. It stamps `verified`.

### 2.2 The Automerge content-sync doc

The producer writes one doc per row at `node:{id}` (`sync/projector.rs:22-25`) in namespace
`"elohim"` (`:932`, `PROJECTION_NAMESPACE`). Only `P2PNode::initiate_sync_round` lists it
(`p2p/mod.rs:9194`; coupling documented at `sync/projector.rs:252-254`). DocStore keys on
`{h_app_id}:{doc_id}` (`sync/doc_store.rs:147`).

Required: doc ids become `node:{cell_dna_hash}:{id}`. Namespace `"elohim"` stays pinned by
`projection_namespace_is_wire_contract` (`:925-932`). Doc ids are free-form and prefix-typed
(`sync/doc_store.rs:298-315`); `node:` still qualifies. **Parsing rule:** strip `node:`, then
split **once** on `:`. The remainder is the content id, which may contain `:`.
`content_doc_id` is the sole renderer. `reverse_project_content_doc`
(`sync/projector.rs:643`, prefix-strip `:648-651`) parses it and refuses failures.
`heal_half_rows_from_docs` (`:415-432`), the second SQL→doc-id consumer, uses that renderer
instead of its own format string.

### 2.3 The shard/replication plane — the second qualifier supplier

**This plane creates the receiving peer's row.** `reverse_project_content_doc` cannot:
absent rows return `Ok(false)`, with comment "absent row → the shard/replication plane's job"
(`sync/projector.rs:669`). `store_acquired_record` (`p2p/mod.rs:10588`) creates rows for both
the libp2p `ShardResponse::Content` arm and iroh pull task. It builds
`CreateContentInput { …, dht_anchor_hash: None }` under hardcoded
`AppContext::default_lamad()` (`:10660-10662`) and inserts via `bulk_create_content`
(`:10664`). Every replicated row starts unqualified.

Required: `store_acquired_record` stamps `cell_dna_hash` from the **claim accompanying the
record** (§7). It runs §1 verification before stamping `cell_provenance`. With **no** claim,
an old peer's `ShardResponse::Content` receives
`cell_dna_hash = NULL, cell_provenance = 'unknown'`. **UNKNOWN never means base.**
§3 permits serving and replication, never authoring anywhere.

### 2.4 Every call that reaches a conductor write

The rule names targets, not a count: **every call reaching `conductor_writes::call_create_content`
or `ContentService::update_via_conductor` passes the §3 gate first.** All target the
`HcClient`'s bound cell. For storage, the `lamad` **role** means base
(`services/conductor_writes.rs:60-64`, `:360-368`). Live sites:

- The NULL-anchor bootstrap branch of `update_via_conductor`
  (`services/content_service.rs:385-386`).
- The stale-anchor heal branch (`services/content_service.rs:425-427`).
- `author_or_recover_evidence` (`api/feedback_operations.rs:430`) calls `call_create_content`
  **directly** (`:470`). It mints a base-lamad `Content` entry from a peer-originating
  `CorrectionRequest`. It bypasses `update_via_conductor`, so needs its own gate.

Two sweeps drive the first two branches and inherit the gate: projection-reconcile's content
`AdoptOutcome::Author` branch (`p2p/projection_reconcile.rs:2638-2675`) and the startup sweep of
every NULL-anchor row (`services/reanchor_backfill.rs:7-22`, re-author at `:405-425`).
Both refuse *before* contacting the conductor.

## 3. Validation contract — the re-author refusal rule

**Storage authors content only into the cell named by the row's VERIFIED qualifier.** Every
§2.4 site compares the row's `cell_dna_hash` with the target `HcClient`'s (`hc_client.rs:655`,
`:672-674`) before writing. Four outcomes:

1. **Equal and verified** — proceed. The peer's own base corpus is unchanged.
2. **Different, verified, and this peer holds that cell** — may select that cell's `HcClient`
   **only** after local `app_info` enumeration (§1) and qualifier verification against a
   locally resolved anchor. **A peer-supplied qualifier never routes a write.** Either
   condition failing gives outcome 4, never a redirect. Never redirect to base.
3. **Different, verified, this peer does not hold that cell** — permanently refuse as
   `foreign-cell`. This **positive** outcome adds no reconcile gap or divergence counts.
   It is not retried next sweep.
4. **UNKNOWN or UNVERIFIED** — refuse as `unverified-cell`. Retain, serve, and replicate;
   **never author into any cell, including base.** Legacy rows and old peers' `ContentRecord`
   both land here. This rule closes the escape. NULL is no wildcard: neither "any cell" nor "base".

**The refusal binds the sync consumer too.** `reverse_project_content_doc` already refuses
to turn converged doc fields into notarization provenance (`sync/projector.rs:609-642`).
It must also refuse qualifier mismatches: such a doc heals nothing. A doc naming an
unverifiable cell is never projected into a row.

## 4. Partition behaviour

D4's vocabulary is **pending** and **conflict-visible** when currentness cannot be checked
(reimplementation plan §0.5). Six states:

- **Cell not held locally** — `pending — foreign cell`, qualifier displayed. Nothing authored
  or anchored. No reconcile arm counts a closable gap.
- **Qualifier UNKNOWN or unverifiable** — `pending — provenance unknown`. Served and
  replicated, never authored. §6's sweep or a later verified claim can resolve it.
- **Cell held, no `HcClient` bound** — `pending — wrong cell`. Retry once a client exists.
- **Cell held, conductor cannot reach it** — cells disabled during boot, the condition
  `reanchor_backfill` handles (`services/reanchor_backfill.rs:7-22`). Show `pending — cell
  unavailable` with existing bounded backoff. This liveness failure differs from the two above.
- **Two rows share `(h_app_id, id)` under different qualifiers** — retain both. The id is
  **conflict-visible**: readers see both, labelled by cell. Neither is deterministically
  promoted, even by last arrival. Different validation contexts have no authoritative ordering.
- **An arriving qualifier contradicts the row's** — **UNKNOWN** accepts the verified arrival
  as **ACCEPTED**: it resolves unknown provenance. For **VERIFIED**, the arrival is
  **rejected and not retried**. A positive mismatch disproves the claim; only missing evidence
  warrants retry.

## 5. Rebuild path

Derived tables reset and re-derive **per cell**, atomically with consumption state.

- Each `(h_app_id, cell_dna_hash)` partition is one unit. UNKNOWN rows have their own.
  Rebuilding base never clears clones, or vice versa.
- Either finish a fresh partition and atomically swap it in, or reset in place. An in-place
  reset serialises live projection writers. It clears partition `content` rows, sync docs,
  and per-partition reconcile cursors **in one transaction**. Keeping cursors after clearing
  rows makes a partial rebuild look converged.
- Readers see `rebuilding` only for that partition. Others remain served. Rebuild equality
  means identical canonical logical rows: id, qualifier, provenance state, anchor, declared
  head, reach, body address. SQLite layout and operational timestamps are excluded.
- Corpus back-fill (`sync/projector.rs:515-530`) becomes partition-scoped to prevent merging
  cells' corpora. `heal_half_rows_from_docs` (`:415-432`) gets the same scope instead of
  hardcoded `default_lamad()`.

## 6. Migration shape for existing rows

- **Default to UNKNOWN, never to base.** Existing rows become
  `cell_dna_hash = NULL, cell_provenance = 'unknown'`. Stamping installed base would infer
  provenance from its absence. The motivating rows (`fxclone-beta-20260907`, measured
  2026-09-07) were clone-authored *within* the pre-contract corpus. Stamping base replaces a
  diagnosable absence with a permanent false claim that §4 would defend.
- **A re-anchor sweep resolves each row.** Fetch each UNKNOWN row's `dht_anchor_hash` in
  base. Found → stamp base, `verified`. Missing or absent anchor → remain UNKNOWN, never
  re-authored (§3 case 4). The sweep is bounded, resumable, and never authors.
- **Serving is unchanged.** An anchored UNKNOWN row stays legacy-but-witnessed: served,
  eligible for migration, not a conflict. §4's accept-on-UNKNOWN rule permits later verified
  claims to resolve it.
- **The key change is a table rebuild.** SQLite cannot widen primary keys in place.
  Create, copy, swap in one transaction. Two rows cannot collapse to one new key.
- **Sync docs are re-derived, not renamed.** Migration clears the partition's sled
  content-doc space. Partition-scoped back-fill (§5) writes qualified ids. Surviving
  unqualified `node:{id}` docs are inert: no consumer looks them up. The next reset sweeps them.
- **Diesel migration timestamps must not collide** within the same window. Sharing a
  `YYYY-MM-DD-HHMMSS` prefix silently loses one migration.

## 7. Mixed-version safety of the wire change

**The additive-`serde(default)` recipe does not apply.** The correction contract's
notification reference is safe in its map-keyed message (`rmp_serde::to_vec_named`).
These planes encode positionally: `p2p/sync_protocol.rs:358`, `:382` and
`p2p/shard_protocol.rs:200`, `:224` use `rmp_serde::to_vec`.
This is **compact**: field position, not name, defines the contract.

**Shard plane (the plane that creates rows).** `ContentInventoryItem`
(`p2p/shard_protocol.rs:53-60`) and `ContentRecord` (`:65-79`) stay **frozen in shape**.
Append no fields. Put the qualifier in a **new request/response variant pair appended last**.
Precedent: `ShardRequest::GetManifest` (`:48`) is documented as "appended last so an older
peer never receives it unless it asked (positional codec)". The response uses a **new struct**,
freely shaped because nothing decodes it yet. It carries existing `ContentRecord`,
`cell_dna_hash`, and the record's `dht_anchor_hash` claim. The anchor enables §1 verification.

**What each side does with the other's records.** A NEW receiver stamps an OLD peer's
`ShardResponse::Content` **UNKNOWN, never base** (§2.3). This alone closes old→new leakage;
no sender upgrade needed. An OLD receiver still strips NEW records' qualifiers and wrongly
re-authors into base. Upgrades close the escape peer-by-peer, not atomically.

**Sync plane.** Requests/responses already carry `h_app_id` and `doc_id` positionally
(`p2p/sync_protocol.rs:50-52`). The qualifier travels within `doc_id` (§2.2), without changing
message shape. A pre-contract peer stores qualified doc ids unchanged. Verified inert:
it strips `node:` (`sync/projector.rs:648`), finds no row, returns `Ok(false)` (`:669`).
Sync creates no rows.

**Every wire change ships with a both-directions byte-compat test.** Decode old bytes with
the new build and new bytes with a struct lacking the field. Compact encoding guarantees
neither direction from field attributes alone.

## 8. Design-gate answers (P2P design gate, second pass — 2026-09-08)

| Entity | Class | Identity | Zome / hash effect | Projection |
|---|---|---|---|---|
| `content` row + qualifier columns | **C** — rebuilt from the named cell's DHT (§5) | composite `(h_app_id, cell_dna_hash, id)` — the gate's Option 3: an index over content whose own identity is the entry hash. `cell_clone_id` **excluded** — a per-installation counter would split one DHT's corpus across two peers | storage migration only; no zome; **DNA-hash-NEUTRAL** | n-a (this *is* the projection) |
| Automerge content-sync doc | **C** — derived from the row, reconstructable by back-fill | doc id `node:{cell_dna_hash}:{id}`, namespace `"elohim"`; global across peers because the network seed is bound into the DNA hash | `sync/projector.rs` producer; **DNA-hash-NEUTRAL** | DocStore key `{h_app_id}:{doc_id}` (`sync/doc_store.rs:147`); reach tier **unresolved — vocabulary in declared drift** |
| Shard-plane qualified record | **C** — a wire message, not a stored entity | none; it names an existing `ContentRecord` plus a `(cell_dna_hash, dht_anchor_hash)` claim the receiver verifies | new `ShardRequest`/`ShardResponse` variants appended last; **DNA-hash-NEUTRAL** | `store_acquired_record` → `bulk_create_content` (`p2p/mod.rs:10588`, `:10664`) |
| The re-author path's fresh DHT action | **A (Notarized)** on the existing `Content` entry type — no new type | `cid = entry_hash`; the action hash is only ever `dht_anchor_hash` | `content_store::create_content` via `call_create_content` (`:360-368`); **DNA-hash-NEUTRAL** — the refusal is a storage gate, not a validation rule | `ContentCommitted` → `upsert_with_anchor` (`rea_projection.rs:762`) |

**Head-plane cost.** Class-C entities add zero heads. The A-class count *falls*.
Before this contract, each clone write crossing the projection can mint one `Content` head
**per receiving peer**, forever. This is unbounded in (clone writes × peers), forbidden by
the gate's head-plane budget. Refusal reduces it to zero. UNKNOWN defaults extend that zero
to the *pre-contract* corpus, missed by the first gate pass.

**Network stakes.** §3 comparison and §1 verification are **floor-protected**. Neither
cheapens at `Simulacra`. Fixture and genesis-stage peers run clones. A stage-priceable check
would be disabled where the defect occurs.

**Anti-patterns checked.** No new entry type, UUID identity, route-first design, or k8s-plane
modelling. This pass fixes three first-pass defects: qualifying only the plane that *cannot*
create rows; letting peer-supplied hints **address** writes, elevating unauthenticated input's
privilege; and using peer-local counters in global keys. The first pass's correction stands:
qualifying the **sync namespace** instead of doc ids would break pinned `PROJECTION_NAMESPACE`.

## 9. Stations

The atom's *done when*: EMPTY six-plane identity diff on a receiving peer's base cell after
a clone write, with qualified projection and sync rows. Stations 1–3 carry the qualifier;
4–5 prove empty diffs. **Stations 6–10 are additional commitments**: §4 partition honesty,
§5 per-partition rebuild, §2.3 UNKNOWN ingest, §6 migration default, and liveness.
Without liveness, §3 could pass vacuously.

**The standing check is an a2o scenario**, not a script. It lives under
`genesis/a2o/features/dataplane/`, bound to `@concern:cell-qualified-projection`.
`just test mesh` runs it; receipts go under `genesis/a2o/reports/`.
**Stations 4 and 5 assert on a RECEIVING peer**, requiring two peers.
`genesis/a2o/scripts/fixtures-clone.mjs` remains **one-peer preparation and inventory**:
modes `prepare, install, grow, measure, authorize, inventory`, single `--db`, single
`--storage`, no diff/assert mode. The scenario invokes it to stage clones and dump all six
inventories. The scenario compares them.

1. **The qualifier is sourced from the signal.** The `ContentCommitted` handler reads
   `Signal::App { cell_id, .. }`. Check: same-peer writes into `lamad.fixtures` and `lamad`
   yield different `cell_dna_hash`. Discarding `cell_id` fails this check.
2. **The row carries it.** Qualifier columns, widened key, `upsert_with_anchor` matching the full key.
   Check: `projection.json` rows have non-NULL `cell_dna_hash` after clone writes.
3. **The doc carries it.** Check: qualified `sync.json` docs place probes under the clone's
   DNA hash, not base's. **No** doc id contains a `clone_id` segment.
4. **The re-author refuses on the receiver.** Every §2.4 site compares before authoring.
   `foreign-cell`/`unverified-cell` are terminal. Check: RECEIVING peer `base-actions` and
   `base-ops` diffs EMPTY after clone writes, where the escape appeared.
5. **The remaining base planes stay empty on the receiver.** `base-content` and `targets`
   diffs EMPTY: no derived base-cell state follows refusal.
6. **Partition states are shown, not swallowed.** Check: second-clone writes make ids
   conflict-visible without overwriting. Rows naming unheld cells show `pending — foreign cell`.
7. **Rebuild is per-partition and atomic.** Check: rebuilding leaves the other partition's
   rows unchanged. Its own rows, docs, and cursors clear together.
8. **An unqualified record from an old peer lands UNKNOWN.** Check: `ContentRecord` through
   `store_acquired_record` without a claim yields `cell_provenance = 'unknown'`.
   The NULL-anchor sweep leaves it un-authored. This station would have caught the escape.
9. **Migration defaults to UNKNOWN and resolves by anchor.** Check: a known clone-authored
   corpus row migrates UNKNOWN. The sweep finds no base anchor. It is never re-authored.
   A later verified clone qualifier is ACCEPTED.
10. **Clone content stays live.** Check: after writing, the authoring peer can re-anchor and
    heal its *own* clone corpus. §1 enumeration supplies the clone-bound `HcClient`, placing
    the row in §3 case 1. Refusing everything passes 4–8 but fails this.

## 10. What this contract does not decide

Group clone spaces remain D6's operator-selected candidates, gated by D0, D3, and a measured
phone/hub budget. Operator space selection and how people join remain undecided. So does
one storage instance per cell versus one across a peer's cells. This contract makes shared
instances correct, not optimal.

**Reinstall and DHT eras.** Forced reinstall resets the clone index. With a deterministic
network seed, the recreated clone has the same `DnaHash`. The qualifier repeats across
re-keying; §5 deliberately treats both eras as one partition. The class-C partition rebuilds
from the peer's post-reinstall DHT. §1 re-checks every anchor against the live cell.
Stale-era rows become UNKNOWN, never falsely `verified`.

**The `AgentPubKey` half.** A `CellId` is `(DnaHash, AgentPubKey)`. This qualifier keeps only
the DNA hash, which answers "which DHT?". Including the agent half would be *wrong*:
a clone DHT has many agents whose row qualifiers must match. Omission was risky only when
the first draft used qualifiers as addresses. §3 case 2 now resolves clients through local
enumeration; the qualifier is never dereferenced.

The clone-enumeration gap in `sync_coordinators`, cross-space evidence, D4's
action-identity-to-content-identity rule, and cross-cell reach semantics remain open.
None blocks closing the escape.

## 11. Adversarial read (2026-09-08)

- **S1** shard creates rows; contract qualified only sync. **Adopted**: §2.3, §3 case 4,
  §7 shard answer, station 8.
- **S2** `on_signal` delivers every cell; `Signal::App { signal, .. }` discards `cell_id`.
  **Adopted**: §1 separates attribution (signal `cell_id`) from enumeration (`app_info`); station 1.
- **S3** §3 case 2 addressed writes using unauthenticated hints. **Adopted**: select only
  locally enumerated *and* verified cells. Otherwise case 4.
- **S4** per-installation `clone_id` appeared in keys and doc ids. **Adopted**: §1 confines it
  to local addressing. Keys and doc ids use DNA hash only. §2.2 adds split-once parsing.
- **S5** station 8 named nonexistent diff/assert modes in a one-peer script. **Adopted**:
  §9 uses two-peer a2o scenarios; the script remains preparation.
- **S6** `api/feedback_operations.rs:470` is a third conductor-write site. **Adopted**: §2.4
  names call targets and lists this site. Station 4 binds all §2.4 sites.
- **S7** clone content could go dark while passing 8/8. **Adopted**: §10 deferral removed.
  §1 requires boot-and-on-change enumeration. Station 10 measures liveness.
- **S8** migration's base default launders the escape. **Adopted**: §6 defaults UNKNOWN with
  an anchor-resolving sweep. §4 accepts on UNKNOWN. Station 9.
- **S9** reinstall resets indices; qualifiers can repeat across DHT eras. **Adopted,
  scoped**: demoting `clone_id` removes indices from keys. §10 explains why
  `(h_app_id, cell_dna_hash)` suffices and where era-naming is deferred.
- **S10** qualifier is not a full `CellId`. **Adopted as stated, rebutted as a defect**:
  mattered only when case 2 dereferenced it. S3 fixed, DNA hash alone answers "which DHT?".
  The agent half would be wrong for multi-agent clone DHTs.
- **S11** §7 covered the plane that cannot leak. **Adopted**: §7 leads with shard and
  UNKNOWN receiver rules. Verified sync-plane inertness remains secondary.
- **Minor** unscoped `heal_half_rows_from_docs`; §4 said "four states" but listed six.
  Both **adopted**: §2.2 and §5 scope healing; §4 says six.
