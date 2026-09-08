---
title: "Cell-qualified projection and sync — the contract that stops clone-authored content from being re-authored into a peer's base cell"
id: cell-qualified-projection-and-sync-contract
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
graduation-trigger: "Draft→Active when the adversarial read and a second p2p-design-gate pass are addressed in the text and the ten stations of §9 are written as a2o scenarios bound to @concern:cell-qualified-projection under genesis/a2o/features/dataplane/ — adversarial read: done 2026-09-08 (§11); second gate pass: done 2026-09-08 (§8, re-answered against the revised design); stations-as-scenarios: pending. Active→Canonical when ALL TEN stations pass — the atom's own done-when (stations 1-5: EMPTY identity diff on a RECEIVING peer's base cell across all six planes after a clone write, qualifier carried on projection and sync rows) plus this contract's five additional commitments (station 6 partition honesty, 7 per-partition atomic rebuild, 8 UNKNOWN on unqualified ingest, 9 migration default, 10 clone liveness) — with a receipt under genesis/a2o/reports/ and the DELTA on the habit atom"
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

**Vocabulary.** A Holochain hApp installs one **cell** per declared **role** — one agent's
participation in one DNA's validation context and gossip network. An operator may **clone** a
role: a second cell under the same role name, own DNA hash, separate DHT. `elohim-storage`
reaches those cells through the local **conductor** and projects what it reads into local SQL
and an Automerge sync store. The **six planes** the fixtures-clone harness inventories are a
cell's three source-chain views (`*-actions`, `*-ops`, `*-content`), the DHT `targets` view,
the storage `projection` rows, and the `sync` documents. **D-numbers** index the ten sealed
decisions in §0b of the holons-are-spaces companion.

The 2026-09-07 household-mesh run exposed the escape. Content written into a `lamad` clone
stayed inside that clone's DHT on the authoring peer — all four DHT-view diffs empty — but
escaped through storage: two rows landed in the shared `content` projection with no cell
qualifier, two docs landed in the `elohim` sync namespace, and eight minutes later a
*receiving* peer's storage re-authored one doc into its base `lamad` cell under a fresh DHT
action. Clone isolation holds only for the authoring peer's DHT.

This contract defines the qualifier that closes the escape, its source on each plane that can
create a row, and each plane's response to an absent, unverifiable, or foreign qualifier.
**Sections 1–7 state required target behaviour, not current behaviour.**

## 1. The qualifier, its authority, and its verification

**The qualifier is the cell's `DnaHash`** — full 39-byte `uhC0k…` form, base64, never
truncated (the eight-byte prefix at `hc_client.rs:371-373` is for logging, never identity).
It is the **discriminating and global** half of a `CellId`: a clone carries distinct DNA
modifiers, so its hash differs from its role's provisioned cell, and because the clone's
network seed is bound into that hash, **two peers in the same clone DHT compute the same
hash**. That is what makes it usable as a key and on the wire.

**`clone_id` is addressing, never identity.** The conductor's `role.N` string is a
per-installation counter: two peers in one clone DHT normally hold different indices
(`lamad.1` vs `lamad.4`). A peer MAY keep `cell_clone_id` as a local column to resolve its own
`HcClient` and to name a cell in a refusal message. It **MUST NOT** appear in any row key, any
wire doc id, or any cross-peer comparison — local addressing in a global key manufactures
divergence between two peers of one DHT.

**Two authorities, never confused.**

- **Attribution** — which cell authored a record this peer wrote — comes from
  `Signal::App { cell_id, .. }`. `AppWebsocket::on_signal` delivers signals for *every*
  cell of the app, clones included (`hc_client.rs:1037` says so in its own docs), so
  cell-local *emission* is not cell-local *delivery*. All four subscriptions today
  destructure the cell away — `hc_client.rs:814`, `:891`, `:965`, `:1058` all match
  `Signal::App { signal, .. }` — and that discard is the attribution defect. The `cell_id`
  is already on the wire, conductor-stamped, needing no new plumbing.
- **Enumeration** — which cells *this peer holds* — comes from `app_info`: `list_apps` →
  `cell_info[role_name]` → `cell_discovery::select_target_cell` (`hc_client.rs:340-369`,
  `cell_discovery.rs:212-226`), which distinguishes `Provisioned` from `Cloned`
  (`cell_discovery.rs:196-210`, `:221-224`) then discards the clone identity in the returned
  `CellId`. It runs at boot and on every `app_info` change, so a clone-bound `HcClient` exists
  for each clone this peer holds.

**A role name is not a qualifier.** `AppContext` carries `h_app_id` and a libp2p peer id, no
cell identity (`db/context.rs:50-57`), and `default_lamad()` hardcodes `"lamad"` (`:69-71`) —
so every clone of the role yields the same `h_app_id`, which is the measured collision.

**A received qualifier is a claim until verified.** The receiver marks it **VERIFIED** only
when both hold: this peer enumerated that DNA hash from its own `app_info`, **and** the
record's claimed DHT anchor resolves in that cell. The DHT space the action lives in *is* the
provenance. An unverifiable qualifier is **UNVERIFIED**; for every authoring decision it is
treated exactly as `foreign-cell`, and the row is kept as a **replica that is never
re-authored**. Only the local conductor stamps a VERIFIED qualifier, as `dht_anchor_hash` is
only ever written by conductor-verified paths (`sync/projector.rs:632-642`, REQ-N5).

## 2. Every plane that can create or move a row

### 2.1 The storage `content` projection row

`content` is declared `Source of truth: DHT … Classification: A`
(`migrations/2026-01-08-000000_initial/up.sql:46-50`) with `id TEXT PRIMARY KEY` and
`h_app_id` an ordinary column. That comment classifies the *entry*, not the *row*: the entry
is class A, the table its class-C rebuildable index. **With `id` alone as key, two cells
authoring the same content id cannot coexist** — the second write silently updates the first.

Required: `content` gains `cell_dna_hash TEXT` (NULL = unknown) and `cell_provenance TEXT`
(`verified` | `unknown`), and MAY gain a local-only `cell_clone_id TEXT` for addressing. The **logical key becomes
`(h_app_id, cell_dna_hash, id)`** — no clone segment. `upsert_with_anchor`
(`db/content_diesel.rs:1083`) matches, pre-contract, on `h_app_id` and `id` only
(`:1094-1099`); it takes the qualifier as an argument and matches the full key. The
`ContentCommitted` arm calling it (`rea_projection.rs:723-770`, call at `:762`) supplies the
DNA hash of the `Signal::App` **`cell_id`** that carried the signal — never the receiving
`HcClient`'s own cell — and stamps `verified`.

### 2.2 The Automerge content-sync doc

The producer writes one doc per row at `node:{id}` (`sync/projector.rs:22-25`) under the
`"elohim"` namespace (`:932`, `PROJECTION_NAMESPACE`), listed only by
`P2PNode::initiate_sync_round` (`p2p/mod.rs:9194`; the coupling is documented at
`sync/projector.rs:252-254`). The DocStore keys on `{h_app_id}:{doc_id}`
(`sync/doc_store.rs:147`).

Required: the doc id becomes `node:{cell_dna_hash}:{id}`; the namespace stays `"elohim"`, a
pinned wire contract guarded by `projection_namespace_is_wire_contract` (`:925-932`). Doc ids
are free-form and prefix-typed (`sync/doc_store.rs:298-315`), which `node:` still satisfies.
**Parsing rule:** strip `node:`, then split **once** on `:` — everything after the first
delimiter is the content id, which may itself contain `:`. `content_doc_id` is the sole
renderer; `reverse_project_content_doc` (`sync/projector.rs:643`, prefix-strip `:648-651`)
parses it, refusing on failure, and `heal_half_rows_from_docs` (`:415-432`) — the second
SQL→doc-id consumer — renders through it rather than its own format string.

### 2.3 The shard/replication plane — the second qualifier supplier

**This is the plane on which a receiving peer's row is actually born.**
`reverse_project_content_doc` cannot create one: an absent row returns `Ok(false)` with the
comment "absent row → the shard/replication plane's job" (`sync/projector.rs:669`). The
creating path is `store_acquired_record` (`p2p/mod.rs:10588`), shared by the libp2p
`ShardResponse::Content` arm and the iroh pull task; it builds
`CreateContentInput { …, dht_anchor_hash: None }` under a hardcoded
`AppContext::default_lamad()` (`:10660-10662`) and inserts via `bulk_create_content`
(`:10664`). Every replicated row is born unqualified.

Required: `store_acquired_record` stamps `cell_dna_hash` from the **claim carried alongside
the record** (§7) and runs §1 verification before stamping `cell_provenance`. A record
arriving with **no** claim — an old peer's `ShardResponse::Content` — is stamped
`cell_dna_hash = NULL, cell_provenance = 'unknown'`. **UNKNOWN is never read as base.**
Under §3 such a row is served and replicated but never authored anywhere.

### 2.4 Every call that reaches a conductor write

Stated by target, not by count: **every call reaching `conductor_writes::call_create_content`
or `ContentService::update_via_conductor` passes the §3 gate first.** All target the cell
bound to the `HcClient` — for storage the `lamad` **role**, meaning the base cell
(`services/conductor_writes.rs:60-64`, `:360-368`). Live sites:

- The NULL-anchor bootstrap branch of `update_via_conductor`
  (`services/content_service.rs:385-386`).
- The stale-anchor heal branch (`services/content_service.rs:425-427`).
- `author_or_recover_evidence` (`api/feedback_operations.rs:430`), calling
  `call_create_content` **directly** (`:470`) to mint a `Content` entry into the base lamad
  cell from a peer-originating `CorrectionRequest` — it never passes through
  `update_via_conductor`, so it is a distinct gate site.

Two sweeps drive the first two branches and inherit the gate: the projection-reconcile
content arm's `AdoptOutcome::Author` branch (`p2p/projection_reconcile.rs:2638-2675`) and the
startup pass over every NULL-anchor row (`services/reanchor_backfill.rs:7-22`, re-author at
`:405-425`). Both refuse *before* a conductor round-trip.

## 3. Validation contract — the re-author refusal rule

**Storage never authors content into a cell other than the one whose VERIFIED qualifier the
row carries.** Every §2.4 site compares the row's `cell_dna_hash` with the target
`HcClient`'s (`hc_client.rs:655`, `:672-674`) before writing. Four outcomes:

1. **Equal and verified** — proceed. The peer's own base corpus is unchanged.
2. **Different, verified, and this peer holds that cell** — the write may be *selected* onto
   an `HcClient` bound to that cell **only** because the cell was enumerated from this peer's
   own `app_info` (§1) and the qualifier verified against an anchor this peer resolved. **A
   peer-supplied qualifier never routes a write.** If either condition fails the outcome is 4,
   not a redirect. Never redirected to base.
3. **Different, verified, this peer does not hold that cell** — refused as `foreign-cell`,
   permanently. A **positive** outcome: no retry on the next sweep, no addition to the
   reconcile arm's gap or divergence counters.
4. **UNKNOWN or UNVERIFIED** — refused as `unverified-cell`. Retained, served and replicated
   as a replica; **never authored into any cell, base included.** This is where a legacy row
   and an old peer's `ContentRecord` both land, and it is the rule that closes the escape.
   There is no wildcard: NULL never means "any cell" and never means "base".

**The refusal binds the sync consumer too.** `reverse_project_content_doc` already refuses to
launder converged doc fields into notarization provenance (`sync/projector.rs:609-642`); it
gains the sibling refusal — a doc whose qualifier differs from the row's heals nothing, and a
doc naming a cell this peer cannot verify is never projected into a row.

## 4. Partition behaviour

Using D4's vocabulary — **pending** and **conflict-visible** when currentness cannot be
checked (reimplementation plan §0.5). Six states:

- **Cell not held locally** — `pending — foreign cell`, qualifier displayed. Nothing authored
  or anchored; no reconcile arm treats it as a closable gap.
- **Qualifier UNKNOWN or unverifiable** — `pending — provenance unknown`. Served and
  replicated, never authored; resolvable by §6's sweep or a later verified claim.
- **Cell held, no `HcClient` bound** — `pending — wrong cell`, retried once a client exists.
- **Cell held, conductor cannot reach it** (cells disabled during a boot window, the condition
  `reanchor_backfill` exists for, `services/reanchor_backfill.rs:7-22`) — `pending — cell
  unavailable`, existing bounded backoff. A liveness failure, distinct from the two above.
- **Two rows share `(h_app_id, id)` under different qualifiers** — both retained, the id is
  **conflict-visible**: readers see both, each labelled with its cell. Neither is
  deterministically promoted, including by last arrival — different validation contexts have
  no authoritative ordering.
- **An arriving qualifier contradicts the row's** — if the row's is **UNKNOWN** the arrival is
  **ACCEPTED** once verified, resolving the unknown rather than contradicting it; if the row's
  is **VERIFIED** the arrival is **rejected and not retried**, a positive mismatch having
  disproved the claim, and only missing evidence warranting retry.

## 5. Rebuild path

Derived tables reset and re-derive **per cell**, atomically with their consumption state.

- The unit is one `(h_app_id, cell_dna_hash)` partition; UNKNOWN rows form their own. A
  rebuild of base never clears clone partitions, and vice versa.
- A rebuild is either a fresh partition built to completion then atomically swapped in, or an
  in-place reset that serialises live projection writers and clears the partition's `content`
  rows, sync docs, and per-partition reconcile cursors **in one transaction** — clearing rows
  but retaining cursors makes a partial rebuild look converged.
- Readers see `rebuilding` for that partition only; others stay served. Equality after rebuild
  = identical canonical logical rows (id, qualifier, provenance state, anchor, declared head,
  reach, body address), excluding SQLite layout and operational timestamps.
- The corpus back-fill (`sync/projector.rs:515-530`) becomes partition-scoped, so it cannot
  merge two cells' corpora; `heal_half_rows_from_docs` (`:415-432`) is scoped the same way
  rather than running under a hardcoded `default_lamad()`.

## 6. Migration shape for existing rows

- **Default to UNKNOWN, never to base.** Every existing row migrates to
  `cell_dna_hash = NULL, cell_provenance = 'unknown'`. Stamping the installed base cell would
  reason from absence-of-provenance to provenance — and the rows that motivated this contract
  (`fxclone-beta-20260907`, measured 2026-09-07) are clone-authored rows *inside* the
  pre-contract corpus. Stamping them base converts a diagnosable absence into a permanent
  false assertion that §4 would then defend.
- **A re-anchor sweep resolves each row.** For every UNKNOWN row it fetches the row's
  `dht_anchor_hash` in the peer's base cell: found → stamp base, `verified`; not found, or no
  anchor → stays UNKNOWN and under §3 case 4 is never re-authored. Bounded, resumable, and it
  never authors.
- **Serving is unchanged.** An UNKNOWN row carrying an anchor stays legacy-but-witnessed:
  served, a migration candidate, not a conflict — and §4's accept-on-UNKNOWN rule lets a later
  verified claim resolve it.
- **The key change is a table rebuild** — SQLite cannot widen a primary key in place: create,
  copy, swap, in one transaction. Two rows cannot collapse to the same new key.
- **Sync docs are re-derived, not renamed.** Migration clears the partition's content-doc
  space in sled; partition-scoped back-fill (§5) writes qualified ids. A surviving unqualified
  `node:{id}` doc is inert — no consumer looks it up — and is swept on the next reset.
- **Diesel migration timestamps must not collide** with siblings in the same window; two
  sharing a `YYYY-MM-DD-HHMMSS` prefix silently lose one.

## 7. Mixed-version safety of the wire change

**The additive-`serde(default)` recipe does not apply.** The correction contract's
notification reference is safe because its message is map-keyed (`rmp_serde::to_vec_named`).
These planes use positional encoding: `p2p/sync_protocol.rs:358`, `:382` and
`p2p/shard_protocol.rs:200`, `:224` all encode with `rmp_serde::to_vec`, which is
**compact** — field position, not field name, is the contract.

**Shard plane (the plane that creates rows).** `ContentInventoryItem`
(`p2p/shard_protocol.rs:53-60`) and `ContentRecord` (`:65-79`) are **frozen in shape** — no
field is appended to either. The qualifier rides a **new request/response variant pair
appended last**, on the existing precedent: `ShardRequest::GetManifest` (`:48`) is documented
as "appended last so an older peer never receives it unless it asked (positional codec)". The
new response payload is a **new struct** — free-shaped because nothing decodes it yet —
carrying the existing `ContentRecord` plus `cell_dna_hash` and the record's `dht_anchor_hash`
claim, the anchor being what makes §1 verification possible at all.

**What each side does with the other's records.** A NEW peer receiving an OLD peer's
`ShardResponse::Content` stamps **UNKNOWN, never base** (§2.3), so the old→new leak direction
is closed by the receiver alone, without waiting for the sender to upgrade. An OLD peer
receiving a NEW peer's record still strips the qualifier and keeps its own defective base-cell
re-authoring: upgrades close the escape peer-by-peer, not atomically.

**Sync plane.** The request/response family already carries `h_app_id` and `doc_id`
positionally (`p2p/sync_protocol.rs:50-52`), so the qualifier travels inside the `doc_id`
string (§2.2), changing no message shape. A pre-contract peer reading a qualified doc id
stores it under that id — verified inert: it strips `node:` (`sync/projector.rs:648`), finds
no row, returns `Ok(false)` (`:669`); the sync plane creates no rows.

**Every wire change ships with a both-directions byte-compat test** — old bytes decoded by
the new build, new bytes decoded by a struct lacking the field — because on a compact codec
neither direction follows from the field's attributes alone.

## 8. Design-gate answers (P2P design gate, second pass — 2026-09-08)

| Entity | Class | Identity | Zome / hash effect | Projection |
|---|---|---|---|---|
| `content` row + qualifier columns | **C** — rebuilt from the named cell's DHT (§5) | composite `(h_app_id, cell_dna_hash, id)` — the gate's Option 3: an index over content whose own identity is the entry hash. `cell_clone_id` **excluded** — a per-installation counter would split one DHT's corpus across two peers | storage migration only; no zome; **DNA-hash-NEUTRAL** | n-a (this *is* the projection) |
| Automerge content-sync doc | **C** — derived from the row, reconstructable by back-fill | doc id `node:{cell_dna_hash}:{id}`, namespace `"elohim"`; global across peers because the network seed is bound into the DNA hash | `sync/projector.rs` producer; **DNA-hash-NEUTRAL** | DocStore key `{h_app_id}:{doc_id}` (`sync/doc_store.rs:147`); reach tier **unresolved — vocabulary in declared drift** |
| Shard-plane qualified record | **C** — a wire message, not a stored entity | none; it names an existing `ContentRecord` plus a `(cell_dna_hash, dht_anchor_hash)` claim the receiver verifies | new `ShardRequest`/`ShardResponse` variants appended last; **DNA-hash-NEUTRAL** | `store_acquired_record` → `bulk_create_content` (`p2p/mod.rs:10588`, `:10664`) |
| The re-author path's fresh DHT action | **A (Notarized)** on the existing `Content` entry type — no new type | `cid = entry_hash`; the action hash is only ever `dht_anchor_hash` | `content_store::create_content` via `call_create_content` (`:360-368`); **DNA-hash-NEUTRAL** — the refusal is a storage gate, not a validation rule | `ContentCommitted` → `upsert_with_anchor` (`rea_projection.rs:762`) |

**Head-plane cost.** The class-C entities add zero heads; the A-class count is *reduced*.
Pre-contract, every clone write crossing the projection can mint one new `Content` head **per
receiving peer**, forever — unbounded in (clone writes × peers), which the gate's head-plane
budget forbids. The refusal makes it zero, and the UNKNOWN default makes it zero for the
*pre-contract* corpus too, which the first gate pass missed.

**Network stakes.** The §3 comparison and §1 verification are **floor-protected**: neither
cheapens at `Simulacra`. Fixture and genesis-stage peers run clones, so a stage-priceable
check would be disabled exactly where the defect occurs.

**Anti-patterns checked.** No new entry type; no UUID identity; no route designed first; no
k8s-plane modelling. This pass corrected three defects the first shipped: qualifying only the
plane that *cannot* create rows and not the one that does; letting a peer-supplied hint
**address** a write, elevation of privilege on unauthenticated input; and putting a peer-local
counter in a global key. The first pass's own correction stands:
qualifying the **sync namespace** rather than the doc id would have broken the pinned
`PROJECTION_NAMESPACE`.

## 9. Stations

The atom's *done when* is the EMPTY six-plane identity diff on a receiving peer's base cell
after a clone write, with projection and sync rows carrying a qualifier: stations 1–3 carry
the qualifier, 4–5 the empty diff. **Stations 6–10 are not entailed by that sentence** — they
add §4's partition honesty, §5's per-partition rebuild, §2.3's UNKNOWN ingest, §6's migration
default, and the liveness §3 could otherwise satisfy vacuously.

**The standing check is an a2o scenario**, not a script: under
`genesis/a2o/features/dataplane/`, bound to `@concern:cell-qualified-projection`, run by
`just test mesh`, receipt under `genesis/a2o/reports/`. **Stations 4 and 5 are asserted on a
RECEIVING peer**, so the scenario is inherently two-peer.
`genesis/a2o/scripts/fixtures-clone.mjs` stays the **one-peer preparation and inventory step**
it actually is (modes `prepare, install, grow, measure, authorize, inventory`; single `--db`,
single `--storage`, no diff or assert mode), invoked by the scenario to stage clones and dump
the six inventories. The comparison lives in the scenario.

1. **The qualifier is sourced from the signal** — the `ContentCommitted` handler reads
   `Signal::App { cell_id, .. }`. Check: a write into `lamad.fixtures` and one into `lamad` on
   the same peer produce different `cell_dna_hash`, the check the discarded `cell_id` fails.
2. **The row carries it** — qualifier columns, widened key, `upsert_with_anchor` matching on
   them. Check: `projection.json` rows carry a non-NULL `cell_dna_hash` after a clone write.
3. **The doc carries it.** Check: `sync.json` docs are qualified, the probes appear under the
   clone's DNA hash not the base's, and **no** doc id contains a `clone_id` segment.
4. **The re-author refuses on the receiver** — every §2.4 site compares before authoring, and
   `foreign-cell`/`unverified-cell` are terminal. Check: on the RECEIVING peer, `base-actions`
   and `base-ops` diffs EMPTY after the clone write — the plane where the escape appeared.
5. **The remaining base planes stay empty on the receiver** — `base-content` and `targets`
   diffs EMPTY, so no derived base-cell state followed the refused write.
6. **Partition states are shown, not swallowed.** Check: a second-clone write yields a
   conflict-visible id rather than an overwrite, and a row naming an unheld cell reads
   `pending — foreign cell`.
7. **Rebuild is per-partition and atomic.** Check: rebuilding one partition changes no row of
   the other and clears its own rows, docs and cursors together.
8. **An unqualified record from an old peer lands UNKNOWN.** Check: a `ContentRecord` ingested
   through `store_acquired_record` with no claim yields `cell_provenance = 'unknown'` and the
   NULL-anchor sweep leaves it un-authored — the station that would have caught the escape.
9. **Migration defaults to UNKNOWN and resolves by anchor.** Check: a corpus holding a known
   clone-authored row migrates it UNKNOWN, the sweep finds no anchor in base, the row is never
   re-authored, and a later verified clone qualifier is ACCEPTED.
10. **Clone content stays live.** Check: after a clone write the authoring peer can re-anchor
    and heal its *own* clone corpus, the clone-bound `HcClient` existing from §1's enumeration,
    so the row lands in §3 case 1. A contract that refuses everything passes 4–8, fails this.

## 10. What this contract does not decide

Whether group clone spaces ship remains D6's operator-selected candidacy, gated by D0, D3 and
a measured phone/hub budget; which spaces an operator selects, or how a person joins one; and
whether storage runs one instance per cell rather than one across a peer's cells (shared
instances are made correct here, not optimal).

**Reinstall and DHT eras.** A forced reinstall resets the clone index, and with a deterministic
network seed the recreated clone carries the same `DnaHash` — so the qualifier repeats across a
re-key boundary and §5 treats the two eras as one partition. Deliberate: the partition is
class-C, rebuildable from whichever DHT the peer is in *after* the reinstall, and §1
verification re-checks every anchor against the live cell, so a stale-era row resolves to
UNKNOWN, never to a false `verified`.

**The `AgentPubKey` half.** A `CellId` is `(DnaHash, AgentPubKey)`; this qualifier keeps only
the DNA hash, which alone answers "which DHT?" — the agent half would be *wrong* to include,
since a clone DHT holds many agents and a row's qualifier must be equal across all of them.
The omission was risky only while the first draft used the qualifier as an address; with §3
case 2 resolving the client from this peer's own enumeration, it is never dereferenced.

The clone-enumeration gap in `sync_coordinators`, cross-space evidence, D4's
action-identity-to-content-identity rule, and reach semantics across cells remain open; none
blocks closing the escape.

## 11. Adversarial read (2026-09-08)

- **S1** shard plane creates the row, contract qualified only sync — **adopted**: §2.3, §3
  case 4, §7's shard-plane answer, station 8.
- **S2** `on_signal` delivers every cell and `Signal::App { signal, .. }` discards `cell_id` —
  **adopted**: §1 splits attribution (signal `cell_id`) from enumeration (`app_info`); station 1.
- **S3** §3 case 2 used an unauthenticated hint as an address — **adopted**: case 2 now selects
  only among cells this peer enumerated *and* verified; anything else falls to case 4.
- **S4** `clone_id` is per-installation but sat in the key and doc id — **adopted**: §1 demotes
  it to local addressing, key and doc id are DNA-hash-only, §2.2 adds the split-once rule.
- **S5** station 8 named a diff/assert mode the script lacks, and it is one-peer — **adopted**:
  §9 makes the standing check a two-peer a2o scenario, keeping the script as preparation.
- **S6** `api/feedback_operations.rs:470` is a third conductor-write site — **adopted**: §2.4
  states the rule by call target and lists it; station 4 binds every §2.4 site.
- **S7** all clone content could go dark and still pass 8/8 — **adopted**: §10's deferral
  removed, §1 requires boot-and-on-change enumeration, station 10 measures liveness.
- **S8** migration's base default launders the escape — **adopted**: §6 defaults to UNKNOWN
  with an anchor-resolving sweep, §4 accepts on UNKNOWN, station 9.
- **S9** reinstall resets the index; a qualifier can repeat across a DHT era — **adopted,
  scoped**: demoting `clone_id` removes the index from the key; §10 states why
  `(h_app_id, cell_dna_hash)` suffices and where era-naming is deferred.
- **S10** the qualifier is not a full `CellId` — **adopted as stated, rebutted as a defect**: it
  mattered only because case 2 dereferenced it as an address. With S3 fixed the DNA hash alone
  answers "which DHT?", and the agent half would be actively wrong for a multi-agent clone DHT.
- **S11** §7 covered the plane that cannot leak — **adopted**: §7 leads with the shard plane and
  the UNKNOWN receiver rule, keeping the verified sync-plane inertness claim as secondary.
- **Minor** unscoped `heal_half_rows_from_docs`; §4 said "four states" then listed six — both
  **adopted**: §2.2 and §5 scope the heal path; §4 says six.
