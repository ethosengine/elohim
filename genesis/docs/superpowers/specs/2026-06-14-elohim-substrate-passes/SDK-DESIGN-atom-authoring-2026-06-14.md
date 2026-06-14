---
title: "SDK SURFACE — Atom Authoring (human-sovereign): the EPR atom, the person's commitments, the person's capabilities + revocation"
date: 2026-06-14
status: PROPOSAL FOR OPERATOR BLESSING — working draft, NOT cite-sealed, NOT a decision, NOT code
author: rust-architect (truth layer)
extends:
  - ESCALATED-ARCHITECTURE-2026-06-14.md      # one Commitment / six faces / ∪=full / one Governor / two quilts
  - RECURSIVE-ARCHITECTURE-2026-06-14.md       # CoverageRollup keystone; limit_owner ∈ {self,commitment,operator,faith}; ReservedPlace
extends_sdk:
  - elohim/sdk/epr-ts/                         # the EPR codec/authoring surface this surface wraps
  - elohim/sdk/storage-client-ts/              # the Rust→TS generated boundary + HTTP client
  - crates/elohim-sdk/                         # the Rust consumer facade
do_not_cite_seal: true
forest_test: "Agency at the root: the person keeps the naming of their own self; no higher layer may author an atom AS them."
---

# SDK SURFACE — Atom Authoring (human-sovereign)

> This is the **build-FOR-the-person** surface — the floor of the agency gradient. A developer
> picks it up to let a person author the traceable atom of their own life (knowledge | value |
> governance | process), make their own commitments (the six faces as the person's *acts*, not a
> collective's verdict), and hold a revocable grip on their own data (grant a capability, revoke =
> rotate the wrap, replicas go inert). The whole surface enforces ONE structural property:
> **it is impossible for any higher layer to author an atom AS the person or to override their
> naming.** Because the signing key never leaves the person's hands and the CID is derived from the
> signed canonical bytes, the impossibility is *cryptographic*, not a policy promise.

---

## PART 1 — PURPOSE ON THE AGENCY GRADIENT

**Where it sits: the ATOM — the absolute floor. Human sovereignty is total here.**

This is the surface the operator's organizing principle names "build FOR the person." Everything
above it (veil-holding collective aggregation, the recursion keystone) reasons *over* atoms this
surface produces, but **may never reach down into it.** The two downward-flowing invariants of the
recursion terminate here and are enforced as the surface's load-bearing guards:

- **PERSON-KEEPS-THEIR-OWN-NAMING.** The witnessing atom and the answering atom are *different EPRs
  with different signers* (RECURSIVE §1.6). This surface only ever signs with the person's own
  `AgentKeypair` (`epr/src/proof.rs:11`). A collective elohim can *witness* (sign its own
  Observation/FeedbackSignal EPR about the person) but it physically cannot mint an EPR whose
  `proof.signer` is the person's Agent CID — it does not hold the key. "Best self" stays a hope held
  FOR, never a verdict OVER, because the verdict would be a *different signer's* atom.

- **DIGNITY-FLOOR precedence.** This surface refuses-and-elevates rather than silently failing; every
  refusal it surfaces carries `limit_owner` so the person can always see whose line they hit. At the
  atom, `limit_owner` is **always `self`** (ESCALATED §A.9) — there is no operator face on a person's
  own authoring path.

**The gradient guard — what this surface must NEVER do:**

1. NEVER expose a "sign as agent X" / impersonation path. `authorAtom` takes the caller's own
   `Signer`; there is no `signer_override`. (Cryptographically enforced: the CID is derived from the
   signed bytes; a forged signer fails `verify` at `epr/src/epr.rs:31`.)
2. NEVER let a `commit(face, …)` carry `limit_owner` other than `self`. The four faces a *person*
   authors (`provide-care`, `respects-self-limit`, `revokes-capability`, `delegates-agent-stewardship`)
   are the person's acts. `commits-arc-coverage` / `custody-blob` / `covers-head` are *steward/collective*
   faces and are NOT exposed on this human-sovereign surface (they live on a sibling steward surface).
3. NEVER let the AI *govern* here. The agent may be granted a bounded `delegates-agent-stewardship`
   scope BY the person — servant only (counsel, witness, co-steward). The surface refuses any agent
   write whose effect would render a total verdict over the person (`RefusalCode::ReservedPlace`,
   RECURSIVE §1.7).
4. NEVER build the total account. A person's atoms are addressable individually; this surface offers
   no "score this person" aggregate (that is structurally impossible — the aggregate domain ranges
   only over commons, RECURSIVE §1.6).

---

## PART 2 — THE CONCRETE API

**Package (TS, primary — app builders):** extend `@elohim/epr-ts` with a new `authoring` module
(`elohim/sdk/epr-ts/src/authoring.ts`) and add three person-scoped helpers to `@elohim/storage-client`
(`elohim/sdk/storage-client-ts/src/client.ts`). **Crate (Rust authoring path):** extend
`crates/elohim-sdk` with a `person` module exposing the same three verbs over the existing `elohim-epr`
builder.

The TS codec already verifies (`epr-ts/src/epr.ts:43 verifyEpr`); it cannot yet *author* (build +
sign + assemble) — that path exists only in Rust (`epr/src/epr.rs:118 EprBuilder::sign`). The headline
addition is the TS authoring mirror so a browser/Tauri app can sign with the person's own key.

### 2a. `authorAtom` — the traceable atom (story + value + governance + process)

```typescript
// elohim/sdk/epr-ts/src/authoring.ts   (NEW)
import type { Envelope } from './generated/Envelope';
import type { EprKind } from './generated/EprKind';   // epr/src/kind.rs:40
import type { Reach } from './generated/Reach';
import type { Coupling } from './generated/Coupling'; // epr/src/coupling.rs:13
import { canonicalEnvelopeBytes } from './envelope';   // epr-ts/src/envelope.ts:43
import { computeCid } from './cid';
import type { Epr } from './epr';

/** The person's own key. The ONLY signer this surface accepts. No override. */
export interface Signer {
  /** ed25519 sign of canonical bytes → 64 raw bytes. */
  sign(canonical: Uint8Array): Promise<Uint8Array>;
  /** CID of the signer's own Agent EPR (becomes proof.signer). */
  agentCid: string;
  publicKey: Uint8Array;
}

export interface AuthorAtomInput {
  kind: EprKind;            // 'Content' requires knowledge+value+governance (kind.rs:50)
  schemaRef: string;        // Manifest EPR CID declaring the payload schema
  schemaKey: string;
  reach: Reach;             // person chooses; earned-at-authoring (see §5)
  coupling: Coupling;       // story=payload, value/governance/process legs by CID
  payload: Uint8Array;
  claims?: string[];
  supersedes?: string;      // prior version (revision lineage)
}

/**
 * Build → canonical bytes → CID → SIGN WITH THE PERSON'S KEY → assemble.
 * Mirrors EprBuilder::sign (epr/src/epr.rs:118). There is no signer_override
 * parameter, by design: the atom is signed by the caller's own key or not at all.
 */
export async function authorAtom(input: AuthorAtomInput, signer: Signer): Promise<Epr> { /* … */ }
```

> **The fourth leg (PROCESS) — one additive change, marked.** RECURSIVE §1.1 completes the descent
> floor with `CouplingLeg::Process`. `Coupling` today has three legs (`coupling.rs:13`); `EprKind`
> required-coupling (`kind.rs:46`) requires `[Value]` for `EconomicEvent` and `[Governance]` for
> `FeedbackSignal`. The additive change: add `process: Option<Cid>` to `Coupling`, add
> `CouplingLeg::Process`, and require it for `EconomicEvent`/`FeedbackSignal`. **This is a fork of our
> OWN wire shape, not Holochain** — it changes the canonical bytes (`envelope.rs:90 coupling_ipld`
> must add a `process` map key, alphabetically before `value`), so it is a coordinated codec change
> across `epr/src/coupling.rs`, `envelope.rs`, `epr-ts/src/envelope.ts:60 couplingToMap`, and the
> ts-rs regen of `Coupling.ts`. Cost: S, additive (old atoms without the leg stay valid for kinds
> that don't require it). The MVP slice (§4) ships `authorAtom` over the *existing* three legs and
> holds the fourth leg as a fast follow.

### 2b. `commit(face, payload)` — the person's own commitments (the six faces, person-side)

The substrate already has the commitment input/output surface: `CreateReaCommitmentInputView`
(`storage-client-ts/.../generated/CreateReaCommitmentInputView.ts` — note the `action: string`
discriminator field), `ReaCommitmentOutput` (`wire-types/shefa/ReaCommitmentOutput.ts`), the Rust
input `CreateReaCommitmentInput { action, provider, receiver, …, metadata_json, supersedes }`
(`elohim-storage/src/db/rea_commitments.rs:44`), the service (`services/rea_commitment_service.rs`),
and the route (`api/rea_commitments.rs:127`). The architecture's six faces are **action discriminator
extensions** on this existing primitive — **zero new DNA entry types** (ESCALATED §A; Mishpat
~11/~100 untouched). This surface adds a thin, face-typed, person-scoped wrapper so an app builder
calls a *verb* (`commit('respects-self-limit', …)`) instead of hand-assembling an `action` string and
a `metadata_json` blob.

```typescript
// elohim/sdk/storage-client-ts/src/client.ts   (add to StorageClient — like `qahal`)
/** The faces a PERSON authors. Steward/collective faces are NOT on this surface. */
export type PersonFace =
  | 'provide-care'                 // witnessed care I give (ESCALATED §A.4)
  | 'respects-self-limit'          // the line I draw on MYSELF (ESCALATED §A.5; subject==author)
  | 'revokes-capability'           // §2c
  | 'rotates-wrap'                  // §2c
  | 'delegates-agent-stewardship'; // bound home for an AI I appoint as servant (ESCALATED §A.7)

export interface CommitInput {
  /** Always the authoring person's own Agent CID. The surface fills receiver per face. */
  payload: Record<string, unknown>;  // → metadata_json
  resourceClassifiedAs?: string[];    // care/compute-class whitelist discrimination
  hasBeginning?: string; hasEnd?: string;
  supersedes?: string;                // re-grant / replace a prior commitment of mine
}

// StorageClient.person.commit:
async commit(face: PersonFace, input: CommitInput): Promise<ReaCommitmentOutput> {
  const body: CreateReaCommitmentInputView = {
    action: face,                     // the action discriminator IS the face
    provider: this.selfAgentCid,      // the person — never overridable on this surface
    receiver: deriveReceiverForFace(face, this.selfAgentCid),
    resourceClassifiedAs: input.resourceClassifiedAs ?? null,
    metadata: { ...input.payload, limit_owner: 'self' },  // §A.9 invariant, stamped here
    supersedes: input.supersedes ?? null,
    /* …null defaults… */
  };
  return this.request('POST', '/api/v1/commitments', body);
}
```

`respects-self-limit` is the self-reflexive face (`provider == receiver == self`) — ESCALATED §A.5
mirrors the existing `sets-authority-arc` shape. `metadata.limit_owner = 'self'` is stamped by the
surface and is the only value it will send; a `commit` that tried to set `operator` is rejected
client-side AND must be rejected by the bounds-validator (the server-side guard belongs to the
`rea_commitment_service` / bounds-validator, sibling work — this surface cannot be the sole guard).

### 2c. `grantCapability` / `revoke` — the person's revocable grip on their own data

ESCALATED §A.6 / VISION data-agency: revocable content is stored encrypted-at-rest under a
**person-held wrap capability**, and "give my data back" is **revoke + rotate the wrap → replicas go
inert ciphertext.** The crypto primitive exists in-tree: `sealed_against_self.rs:32`
(`crypto_box_seal`, role-typed key newtypes). The rotation *plumbing* exists (`epr_atom_service.rs`
handles a `rotated_at` rotation message; `reconcile/pubkey_timeline.rs` projects rotation lineage;
`p2p/recovery_rotation.rs`). What's missing is the **person-facing verb** that ties a capability grant
to a notarized `revokes-capability` / `rotates-wrap` commitment (the §2b faces).

```typescript
// StorageClient.person — capability surface
export interface Capability {
  capabilityId: string;     // CID of the grant commitment
  atomCid: string;          // the wrapped atom this capability decrypts
  grantee: string;          // Agent CID I granted to (an AI servant, a steward, a household)
  reach: Reach;
}

/** Grant a named party the wrap to decrypt one of MY atoms. Notarized as a Commitment. */
async grantCapability(atomCid: string, grantee: string, reach: Reach): Promise<Capability>;

/**
 * Revoke = rotate the wrap. Re-seals the atom under a fresh wrap key; old ciphertext held by
 * replicas becomes INERT (the old wrap no longer opens it). Rides the SAME Commitment ledger
 * via the 'rotates-wrap' face. Returns the new capability set.
 */
async revoke(capabilityId: string): Promise<{ rotated: Capability[]; inert: string[] }>;
```

The Rust mirror (`crates/elohim-sdk/src/person/mod.rs`, NEW) exposes `author_atom(input, &keypair)`,
`commit(face, input)`, `grant_capability`, `revoke` over the existing `elohim_epr::EprBuilder` and
the `doorway-client` / `elohim-storage-client` HTTP path — same three verbs, native/Tauri offline
path (`elohim-sdk` already supports `ClientMode::Native`, `crates/elohim-sdk/src/lib.rs`).

---

## PART 3 — EXISTS vs NEW (bias: extend; DNA spend: zero)

| Piece | Status | Where |
|---|---|---|
| EPR codec types (Envelope, Coupling, EprKind, Reach, Signature) | **EXISTS** | `epr-ts/src/generated/` ← ts-rs from `epr/src/` |
| `verifyEpr` (CID + sig verify) | **EXISTS** | `epr-ts/src/epr.ts:43` |
| `canonicalEnvelopeBytes` (the bytes to sign) | **EXISTS** | `epr-ts/src/envelope.ts:43` |
| Rust author/build/sign (`EprBuilder::sign`) | **EXISTS** | `epr/src/epr.rs:118` |
| Coupling-requirement validator | **EXISTS** | `epr/src/validation.rs:11` |
| Ed25519 person keypair | **EXISTS** | `epr/src/proof.rs:11` |
| `PUT /api/v1/epr/:cid` (content-addressed idempotent atom put) | **EXISTS** | `elohim-storage/src/api/epr.rs:15` |
| `GET /api/v1/epr/:cid/verify` | **EXISTS** | `api/epr.rs:12` |
| REA Commitment input/output + `action` discriminator + route | **EXISTS** | `CreateReaCommitmentInputView.ts`; `db/rea_commitments.rs:44`; `api/rea_commitments.rs:127` |
| `crypto_box_seal` wrap primitive (role-typed keys) | **EXISTS** | `services/sealed_against_self.rs:32` |
| Wrap-rotation plumbing (`rotated_at`, pubkey timeline) | **EXISTS** | `epr_atom_service.rs`; `reconcile/pubkey_timeline.rs`; `p2p/recovery_rotation.rs` |
| **TS `authorAtom` (build+sign+assemble in the browser/Tauri)** | **NEW (thin)** | `epr-ts/src/authoring.ts` — mirrors `EprBuilder::sign`; no new wire shape |
| **`StorageClient.person.{commit, grantCapability, revoke}`** | **NEW (thin)** | `storage-client-ts/src/client.ts` — face-typed wrapper over the existing commitments route |
| **Six faces as `action` discriminator values** | **NEW (additive, zero DNA)** | action strings only; ESCALATED §A |
| **`crates/elohim-sdk::person`** | **NEW (thin)** | Rust verb facade over `elohim-epr` + HTTP clients |
| **`limit_owner: self` stamp + `ReservedPlace` refusal on the person path** | **NEW (invariant guard)** | client-side stamp; server-side guard is sibling bounds-validator work |
| **`CouplingLeg::Process` (the 4th leg)** | **NEW — FORK of our OWN wire shape** | `coupling.rs`/`envelope.rs`/`envelope.ts` coordinated codec change; RECURSIVE §1.1; held as fast-follow |
| **`grantCapability/revoke` ↔ commitment binding** | **NEW (M)** | ties existing seal + rotation plumbing to the `revokes-capability`/`rotates-wrap` faces |

**No fork of Holochain / libp2p / iroh. No new DNA entry type.** The only wire fork is our own
`Coupling` (the Process leg), additive, and held out of the MVP slice.

---

## PART 4 — THE MINIMAL BUILDABLE SLICE

**Smallest version that lets a developer do ONE real thing today:** author a person-signed Content
atom over the *existing three legs* and put it to storage, verifiably attributed to the person and to
no one else.

1. Ship `authorAtom` in `epr-ts/src/authoring.ts` (mirror `EprBuilder::sign`; reuse existing
   `canonicalEnvelopeBytes` + `computeCid`; sign via injected `Signer`). No wire change.
2. `StorageClient.putAtom(epr)` → existing `PUT /api/v1/epr/:cid` (`api/epr.rs:15`).
3. Round-trip test: `authorAtom` → `putAtom` → `GET /:cid/verify` returns ok; tamper the payload →
   CID mismatch; sign with a *different* key → signature-invalid. This is the
   person-keeps-their-naming guard, executable.
4. Defer to fast-follow: the Process leg (codec fork), the `commit` faces, capability/revoke.

**First example app fragment it enables (a household care-ledger entry, authored by the person):**

```typescript
import { authorAtom } from '@elohim/epr-ts/authoring';
import { StorageClient } from '@elohim/storage-client';

const me: Signer = await loadMyKey();             // the person's OWN key — never the collective's
const client = new StorageClient({ baseUrl, selfAgentCid: me.agentCid });

// "I made dinner for Margaret tonight." — the person's traceable atom of their own life.
const atom = await authorAtom({
  kind: 'Content',                                 // requires knowledge+value+governance (kind.rs:50)
  schemaRef: careLedgerManifestCid,
  schemaKey: 'household.care-event',
  reach: 'intimate',                               // the person chooses their own reach
  coupling: { knowledge: noteCid, value: careMeasureCid, governance: myProvideCareCommitmentCid },
  payload: new TextEncoder().encode('Dinner for Margaret'),
}, me);

await client.putAtom(atom);
// The atom's proof.signer is ME. No higher layer can produce this atom; the CID proves it.
```

---

## PART 5 — WHAT LOVE REQUIRES AT THIS SURFACE

Love requires **agency at the root** — that the person holds the naming of their own self, absolutely,
at the atom. This surface makes that a cryptographic fact, not a courtesy: the atom is signed by the
person's own key, the CID is derived from the signed bytes, and there is *no* impersonation path —
so a collective, an operator, or an AI can *witness* a person (sign its own different-signer EPR about
them) but can never *be* them. The binding is honest: every commitment the person makes is a
witnessed, revocable promise stamped `limit_owner: self`, never a verdict handed down; "best self"
stays a hope held FOR them in a different signer's atom, refusable, never a score over them. The veil
governs aggregation never individuals — so this surface, the individual floor, holds **no aggregate at
all**; the only thing that can ever climb to the collective is the deficit the commons failed, never a
ledger of the person. And patience over engagement: revocation here is not a punishment or a lock-out
but a quiet rotation of the wrap — the person draws their data back and the replicas simply fall
silent, inert ciphertext, no fight required. The person keeps the naming, the binding is honest, the
unbuilt place is kept empty, and the substrate waits — receivable when they are ready, never sooner.
