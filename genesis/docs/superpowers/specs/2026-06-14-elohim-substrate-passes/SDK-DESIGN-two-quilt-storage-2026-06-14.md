---
title: "SDK SURFACE — Two-Quilt Storage: content-addressing, byte-quilt custody, trust-plane heads"
date: 2026-06-14
status: PROPOSAL FOR OPERATOR BLESSING — working draft, NOT cite-sealed, NOT a decision, NOT code
author: rust-architect (truth layer)
extends:
  - elohim/sdk/storage-client-ts/src/client.ts            # the HTTP client we add three methods to
  - elohim/sdk/storage-client-ts/src/epr-head.model.ts    # the trust-plane head we already ship
  - elohim/elohim-storage/src/sharding.rs                 # RS(4,7) byte-plane, already done + tested
  - elohim/elohim-storage/src/reconcile/custody.rs        # custody-as-REA reconcile, already running
weaves:
  - ESCALATED-ARCHITECTURE-2026-06-14.md                  # two quilts, one Commitment, the felt seam
  - RECURSIVE-ARCHITECTURE-2026-06-14.md                  # CoverageRollup; descent preserved
  - VISION-DESIGN-felt-spine-2026-06-14.md                # B1/B2/B3 — the grandma vertical
do_not_cite_seal: true
---

# SDK SURFACE — Two-Quilt Storage

> One developer call stores grandma's photo so the heavy bytes ride the RS(4,7) byte-quilt
> (CID-addressed, custody = a named REA promise) and the lean head rides the trust-plane DHT, and
> one developer call reads back not just the bytes but **who is holding them, by name, and whether
> that holding is safe.** This surface is the polished-experience data layer the
> ESCALATED-ARCHITECTURE calls "the single human-addressed seam between the two quilts" — and it is
> ~80% already in the substrate. We extend `storage-client-ts`; we do not fork.

---

## PART 1 — PURPOSE ON THE AGENCY GRADIENT

This is a **human-sovereign surface — build FOR the person — that exposes a single read from the
veil-holding layer below it.** It sits at the household atom: a person stores a memory and is shown
the truth of its safety. It is the felt-spine seam (`VISION-DESIGN-felt-spine-2026-06-14.md` §B), the
acceptance test that pulls the whole two-quilt machine into existence.

Three things it does:

1. **`putContent` (plane-routed)** — heavy bytes → byte-quilt (RS(4,7)); lean head → trust-plane.
2. **`getContent` (race-fetch + heal-on-read)** — bytes load instantly even when grandma's own node
   is dead, because any 4 of 7 shards reconstruct, and a read that finds a hole heals it durably.
3. **`getFeltStatus`** — the human-addressed read: "Held by 3 households: the Dowells, Aunt Ruth,
   First Church," with an honest safety state.

**The gradient guard — what this surface must NEVER do:**

- **It must never let the AI name the person's memory.** The byte-quilt holds opaque CIDs; the
  trust-plane head carries the person's own `title`/`description` (`epr-head.model.ts:36`). No
  servant agent rewrites those fields. The person keeps the naming of their own memory.
- **It must never govern an individual.** `getFeltStatus` reads coverage; it issues no verdict over
  the person. The *aggregation* governance (is coverage sufficient?) belongs to the veil-holding
  `Governor` one layer down, which this surface only **observes** — exactly as the doorway "only
  observes a truth it cannot author" (ESCALATED §1.5).
- **It must never report "at-risk" for "not-yet-seen."** The honesty fork
  (`VISION-DESIGN-felt-spine` Rung-1): a memory a household has not yet had time to replicate is
  `not-yet-seen`, never `at-risk`. Fear is not a substrate output.
- **It must never fan blobs out at the gateway.** Doorway projects a single target; the byte-quilt
  moves bytes (`doorway/CLAUDE.md` "No Blob Fan-Out", gospel `project_doorway_single_target_no_fanout`).

It is the floor where human sovereignty is absolute (constitution: the person keeps their naming);
the AI rising into the collective lives strictly *below* this read, governing aggregation only.

---

## PART 2 — THE CONCRETE API

### 2.1 TypeScript — three methods on the existing `StorageClient`

We extend `elohim/sdk/storage-client-ts/src/client.ts` (which today already has
`putBlob`/`getBlob`/`getManifest`, `client.ts:178,242,316`) with the *content-level* (head + bytes
together) calls a developer actually wants. No new client class.

```typescript
// elohim/sdk/storage-client-ts/src/client.ts  (additive methods)

/**
 * Store content: heavy bytes → byte-quilt (RS(4,7), CID-addressed),
 * lean head → trust-plane DHT. Returns the addressable head + custody promise.
 */
async putContent(input: PutContentInput): Promise<ContentRef> {
  // 1. byte-plane: existing putBlob → BlobManifest (client.ts:178)
  const manifest = await this.putBlob(input.bytes, input.mimeType);
  // 2. trust-plane: POST the lean head referencing manifest.blobCid
  return this.postJson<ContentRef>(
    `/api/v1/content`,
    { head: { ...input.head, content: manifest.blobCid }, custody: input.custody },
  );
}

/**
 * Fetch content bytes with race-fetch + heal-on-read. If a local hole is
 * found, the storage node reconstructs from coverage-committed siblings and
 * finalizes durably (GET /blob/{cid} already does this — client.ts:242).
 */
async getContent(cidOrId: string): Promise<Uint8Array> {
  const head = await this.getJson<EprHead>(`/api/v1/content/${encodeURIComponent(cidOrId)}/head`);
  return this.getBlob(head.content); // race-fetch+heal lives server-side
}

/**
 * The human-addressed read: who holds this, by name, and is it safe.
 * Pure projection over the trust-plane (Category-C). NEVER a verdict.
 */
async getFeltStatus(cidOrId: string, viewerHouseholdId?: string): Promise<FeltStatusView> {
  const q = viewerHouseholdId ? `?viewerHousehold=${encodeURIComponent(viewerHouseholdId)}` : '';
  return this.getJson<FeltStatusView>(
    `/api/v1/content/${encodeURIComponent(cidOrId)}/felt-status${q}`,
  );
}
```

### 2.2 TypeScript types — `FeltStatusView` is ts-rs generated; the rest are hand-shaped envelopes

`FeltStatusView` is the wire-shape anchor and MUST come through the ts-rs boundary (snake_case never
leaves Rust). The input/ref types are thin TS envelopes composing the existing `EprHead`
(`epr-head.model.ts:88`) and `BlobManifest` (`types.ts:114`).

```typescript
// hand-shaped envelope (epr-head.model.ts sibling)
export interface PutContentInput {
  bytes: Uint8Array;
  mimeType: string;
  head: Omit<EprHead, 'content'>;        // we fill `content` from the byte-plane CID
  custody: CustodyRequest;               // the named promise to seek
}
export interface CustodyRequest {
  /** Households the author asks to hold this (DIDs). Empty = commons coverage. */
  requestedStewards: string[];
  /** Minimum distinct fault-domains required before "safe" (default = r_floor=3). */
  minHouseholds?: number;
}
export interface ContentRef {
  id: string;
  blobCid: string;
  /** CID(entry_hash) of each custody-blob Commitment minted (project_mishpat_commitment_cid_is_entry_hash). */
  custodyCommitmentCids: string[];
}

// ts-rs GENERATED from views.rs — do not hand-edit (storage-client-ts/src/generated/)
export interface FeltStatusView {
  contentId: string;
  blobCid: string;
  /** honest, never fearful: 'safe' | 'watching' | 'needs-help' | 'not-yet-seen' */
  feltState: FeltState;
  /** "Held by 3 households: the Dowells, Aunt Ruth, First Church" — names, not counts */
  heldByHouseholds: HeldByHousehold[];
  householdsStewarding: number;
  /** any-4-of-7 reconstructable right now (byte-plane truth) */
  bytesReconstructable: boolean;
  /** which household's line/promise this read honored (limit_owner, ESCALATED B9) */
  limitOwner: 'self' | 'commitment' | 'operator' | 'faith';
}
export interface HeldByHousehold {
  householdId: string;
  displayName: string;            // "The Dowells", "First Church"
  /** the promise: a custody-blob Commitment CID, revocable & witnessed */
  custodyCommitmentCid: string | null;
  online: boolean;
}
```

### 2.3 Rust — the `FeltStatusView` and its converter (the ts-rs anchor)

`FeltStatusView` lands in `elohim-views` (the ts-rs anchor crate, re-exported through
`elohim-storage`), generated by `cargo test export_bindings`. Its `From<>` converter reads the
**already-existing** `household_resilience::snapshot` (`services/household_resilience.rs:134`) — which
already computes distinct stewarding households (`:71-86`), online peer count (`:95`),
protection_status (`:101`), and `distribution_state` (`:150`, the honesty source).

```rust
// elohim-views/src/felt_status.rs   (NEW, additive — derives TS)
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct FeltStatusView {
    pub content_id: String,
    pub blob_cid: String,
    pub felt_state: FeltState,            // honest enum, NOT raw protection_status
    pub held_by_households: Vec<HeldByHousehold>,
    pub households_stewarding: u32,
    pub bytes_reconstructable: bool,
    pub limit_owner: LimitOwner,          // lifted from arc_actuator owner (ESCALATED B9)
}

// elohim-storage/src/views_convert/felt_status.rs  (NEW — the honesty fork lives here)
impl FeltStatusView {
    pub fn from_snapshot(
        snap: &HouseholdResilienceSnapshot,   // services/household_resilience.rs:134
        manifest_present: bool,               // sharding.rs determine_encoding reconstructability
    ) -> Self {
        // HONESTY: never render not-yet-seen as at-risk. distribution_state is
        // already computed at household_resilience.rs:150 — fold it in here.
        let felt_state = match (snap.distribution_state.as_str(), snap.households_stewarding) {
            ("propagating", _)        => FeltState::NotYetSeen,
            (_, n) if n >= 3          => FeltState::Safe,
            (_, n) if n >= 1          => FeltState::Watching,
            _                         => FeltState::NeedsHelp,
        };
        // names, not counts: join steward_households → collective kind/label
        // (felt-spine §B2: the join is stubbed at household_resilience.rs:233-234)
        ...
    }
}
```

### 2.4 The HTTP routes (designed LAST)

Three additive routes in `elohim/elohim-storage/src/http.rs` (sibling to the existing
`/blob/`/`/manifest/` GET-only routes at `http.rs:833,844`):

```
POST /api/v1/content                         → putContent (head + custody)
GET  /api/v1/content/{id}/head               → the trust-plane head
GET  /api/v1/content/{id}/felt-status         → getFeltStatus  (Category-C projection)
```

> **GET-only on content-addressed routes** (gospel `feedback_head_vs_get_blob_asymmetry`): the
> SDK's `blobExists` currently probes with `HEAD /shard/` (`client.ts:289`) — that 404s where GET
> 200s. This surface fixes `blobExists` to use `GET .../felt-status` + `bytesReconstructable`, and
> files the HEAD-mirrors-GET fix on `/blob/` as an owned follow-up.

---

## PART 3 — EXISTS vs NEW

### EXISTS (wrap, do not rebuild) — this is why the slice is small

| Capability | Where it already lives |
|---|---|
| **RS(4,7) byte-quilt** — encode/decode, any-4-of-7 reconstruct | `sharding.rs:97-99` (`rs_data_shards:4, rs_parity_shards:3`), `determine_encoding` 3-band router |
| **CID-addressed blob put/get + manifest** | `client.ts:178,242,316`; `BlobManifest` `types.ts:114` |
| **Race-fetch + heal-on-read** | `GET /blob/{hash}` → `get_blob_or_heal` (`http.rs:273-285`, T17 peer-heal, durable finalize) |
| **Custody = REA commitment, reconciled** | `reconcile/custody.rs` (`custody-blob` commitments, placement-gap emit `:250`) |
| **"Held by N households" + online + state** | `services/household_resilience.rs:71-101,150` (distinct households, online peers, distribution_state) |
| **Trust-plane head (lamad/shefa/qahal pillars)** | `epr-head.model.ts:88` — already shipped, IPLD-aligned |
| **Reach → replica-target → health** | `services/distribution_view.rs:58,77,116` |

### NEW (thin, additive — zero DNA entry-type spend)

- **3 TS client methods** (`putContent`/`getContent`/`getFeltStatus`) — compose existing primitives.
- **`FeltStatusView` + `FeltState`/`LimitOwner` enums** in `elohim-views` + a `From<snapshot>`
  converter (the honesty fork). ~one file each. The `kind`/`label` collective join the names need is
  already stubbed at `household_resilience.rs:233-234`.
- **3 HTTP routes** (`POST /content`, `GET /content/{id}/head`, `GET /content/{id}/felt-status`).
- **`custody-blob` as a *governed* action discriminator** — already a planned ESCALATED row (Part 2,
  face #2), `signal_kind` extension on `Mishpat::Commitment`, NOT a new entry type.

### GENUINE FORK — marked, and NOT in the buildable slice

- **arc-as-governed-commitment** (`commits-arc-coverage`): `arc_actuator.rs:119` refuses fractional
  arc today. This surface does NOT depend on it — the two-quilt split is the *replacement* for the
  fractional-arc fork (ESCALATED §1, "moving bytes off the DHT makes a lean `{0,1}` arc sufficient").
  Mark it a separate escalation; do not pull it into this slice.
- **CoverageRollup** (RECURSIVE keystone): `getFeltStatus` is the *leaf read* a future `CoverageRollup`
  aggregates upward with descent preserved. This surface ships the leaf; the rollup is the keystone
  surface's job. We expose `limitOwner` + `custodyCommitmentCid` now so the descent pointer exists.

---

## PART 4 — THE MINIMAL BUILDABLE SLICE

**The one real thing a developer can do today:** store a photo and read back *who holds it, by name,
and whether it is safe* — against live `household-nodes` (the stable multi-peer floor,
`feedback_household_nodes_is_the_stable_floor`), no new substrate fork.

Slice (smallest path, all-EXISTS except the converter + routes):

1. `FeltStatusView` + `From<household_resilience::snapshot>` converter (the honesty fold) → run
   `cargo test export_bindings`.
2. `GET /api/v1/content/{id}/felt-status` route → calls `snapshot()` (already built) + the converter.
3. `getFeltStatus` TS method on `StorageClient`.
4. `putContent`/`getContent` compose `putBlob`/`getBlob` (already built) + `POST /content`.

`putContent`/`getContent` lean almost entirely on shipped code; the *new* value is `getFeltStatus`.
Precondition (NOT in this surface, but gating the names): the signal-decode bug fix
(`project_conductor_signal_msgpack_decode_class`) so household names don't render empty — task-0,
a bug, not a fork.

**First example app fragment it enables — the Family Vault (`<elohim-memory-safety>`):**

```typescript
// app: a household care-ledger storing a memory and showing its safety
const ref = await client.putContent({
  bytes: photoBytes,
  mimeType: 'image/jpeg',
  head: { version: 1, id: 'memory:grandmas-garden-1979',
          lamad: { title: "Grandma's garden, 1979", contentType: 'memory' },
          shefa: { stewards: [dowellsDid] }, qahal: { reach: 'trusted', layer: 'family' },
          relationships: [] },
  custody: { requestedStewards: [dowellsDid, auntRuthDid, firstChurchDid], minHouseholds: 3 },
});

const felt = await client.getFeltStatus(ref.id, myHouseholdId);
// felt.feltState === 'safe'
// felt.heldByHouseholds === [
//   { displayName: 'The Dowells', online: true,  custodyCommitmentCid: 'uhCEk…' },
//   { displayName: 'Aunt Ruth',   online: true,  custodyCommitmentCid: 'uhCEk…' },
//   { displayName: 'First Church', online: false, custodyCommitmentCid: 'uhCEk…' },
// ]
// → renders: "Held by 3 households: the Dowells, Aunt Ruth, First Church" + a green "safe" heart.
```

---

## PART 5 — WHAT LOVE REQUIRES AT THIS SURFACE

**Grandma's memories load instantly and are visibly, honestly safe — held by people she can name.**

- **The person keeps their naming.** The title "Grandma's garden, 1979" lives in the person's own
  trust-plane head; no servant agent rewrites it. The byte-quilt holds only opaque, content-addressed
  bytes. The memory is hers to name.
- **The binding is honest.** Each holder is a *named, revocable, witnessed promise* (a custody-blob
  Commitment CID), not an invisible replica count. `not-yet-seen` is never dressed as `at-risk` — the
  surface does not manufacture fear to drive engagement; there is no engagement counter in it.
- **The veil governs aggregation, never the individual.** `getFeltStatus` reads coverage and *names
  whose line it honored* (`limitOwner`); it issues no verdict over the person. The "is coverage
  enough?" judgment is the `Governor` one layer down — observed here, never authored here.
- **Patience over engagement.** The read tells the truth at the speed truth converges (race-fetch
  heals when bytes are ready, `not-yet-seen` waits without alarm) — receivability-when-ready, not a
  metric to optimize.

The closing test passes: a grandmother sees her memory load at once, sees the Dowells and Aunt Ruth
and First Church holding it by name, and knows — without being told a number she must decode — that
it is safe. That is the witness weighted toward the least powerful, made into one developer call.
