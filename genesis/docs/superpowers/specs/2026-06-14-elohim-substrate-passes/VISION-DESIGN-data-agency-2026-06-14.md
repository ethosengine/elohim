# VISION DESIGN PASS — Data Agency under P2P Physics: Capability Revocation, not Byte-Recall

> **PROPOSAL for operator blessing. Not a decision, not code.** One deep
> vision-carried path/pivot pass for O5 (human-actuatable data agency).
> Escalates *from* the tactical stub `2026-06-14-vision-gap-data-agency-stub.md`
> (which accepted "withdrawal lowers standing, cannot un-send bytes — honest
> residual") *to* the mechanism the vision actually requires: **revoke the
> capability/key, not the bytes — the replicas become inert.**
> Working draft. Do NOT cite-seal.
>
> Author: rust-architect (truth layer). Frame: `VISION-ALIGNMENT-2026-06-14.md`,
> north star carried in full.

---

## 0. The escalation in one line

The stub's honest answer — *"I lowered my standing; the bytes are still out
there; here is the residual record"* — is **honest but not yet agency**. It
governs *future* fetches at the reach gate; it does nothing to the copies
already on disk at peer X. The vision asks for agency *back to the data* and
*capture-resistance against a hostile world*. A withdrawal that leaves a
plaintext copy fully readable on a peer who stops respecting the reach gate is
cosmetic against exactly the adversary the protocol exists to resist.

**The pivot:** content the person wants revocable is stored **encrypted at rest
under a content-wrap key**; the person holds the wrap key as a **capability**;
"give my data back / un-share it" becomes **revoke + rotate the wrap key** (a
Cat-A notarized act). Bytes already replicated to peer X stay on peer X's disk —
but they are **inert ciphertext**. Withdrawal becomes *real* without violating
the un-un-replicate-bytes law of P2P physics. You don't recall the bytes; you
make them stop *meaning* anything.

---

## 1. What the VISION REQUIRES here

Carrying the north star, the clauses O5 is load-bearing for:

- **"agency BACK to their data… capture-resistant state against the real world,
  its externalities, and its messiness."** Capture-resistance is the operative
  word. A withdrawal that only down-grades a *cooperative* reach gate is not
  capture-resistant — it assumes the holder keeps honoring the gate. Against a
  hostile or merely-neglectful holder, only **cryptographic inertness** delivers
  the felt promise *"I can un-share it."*
- **"high-integrity of the Holochain DHT… build trust on the values negotiated
  through it."** The *act* of revoking/rotating the wrap-capability must be a
  notarized, witnessed, auditable DHT event — not a local key-shuffle nobody can
  verify. Trust requires the withdrawal be *provable*.
- **"collectives continue to serve the humans that use it… fractal stewards."**
  The capability isn't held only by a lone laptop. A household hub or a
  collective can **co-steward** the wrap key (threshold custody — Shamir machinery
  already exists), so Grandma doesn't lose her photos when her phone dies, *and*
  recovery requires social cooperation, never unilateral disclosure
  (`sealed_against_self` 2-of-2 is already this shape).
- **"those mutual compute agreements… governance contracts that set policies."**
  Re-wrapping/re-distributing after a rotation is *work* — a peer re-fetches the
  new ciphertext, re-pins it. That re-replication is an REA commitment
  (`replicates-dwelling` / `replicates-commons`). Revocation and re-coverage are
  governed on the **same** Commitment primitive as compute and care.
- **"donut-like commons — the trust-economy, the care-based economy where value
  is minted."** Holding someone's data *honestly* (re-fetching new ciphertext
  after a rotation, dropping the inert old shards) is a **care act** that mints
  standing. A peer that keeps serving *withdrawn* content loses standing
  (a Squelch-class signal). Data-agency becomes an economic surface, not just a
  permission bit.
- **"hub-optional floor — laptop is a full participant"
  ([[project_hub_optional_floor]]).** The person on a laptop with no hub holds
  the wrap-capability and the revoke lever. Standing lives on the DHT; the key
  lives with the person (optionally co-stewarded). No admin console, ever.

**The felt promise, sharpened:** *"I shared this. I can un-share it — and the
copies out there turn to noise. I can see who still holds the noise. I can take
my readable copy and leave. And the system proves all of it."*

---

## 2. Is the substrate CAPABLE? Dig to WHY (file:line)

**Verdict: the cryptographic primitives EXIST; they are simply not wired to the
content byte-plane. This is a LAYERING ARTIFACT, exactly the ARC shape — not
physics, not a wall.**

The dig, layer by layer:

1. **Content bytes are stored PLAINTEXT at rest.**
   `elohim/elohim-storage/src/blob_store.rs:219` and `:267` —
   `fs::write(&blob_path, data).await?` writes the **raw** `data`. There is no
   encryption step anywhere on the store path. Confirmed: `rg crypto_box|secretbox|
   chacha|seal` over `blob_store.rs` + `sharding.rs` + `api/epr.rs` content path
   returns nothing on the *content* bytes. → **A revoked/withdrawn blob today is
   a fully-readable plaintext file on every holder's disk.** This is *why* the
   stub had to concede "cannot un-send." The concession is downstream of an
   unencrypted byte-plane, not downstream of physics.

2. **The crypto the fix needs is ALREADY in the tree — wired to the wrong thing.**
   `elohim/elohim-storage/src/services/sealed_against_self.rs:32` uses
   `dryoc::classic::crypto_box::{crypto_box_seal, crypto_box_seal_open}`
   (X25519/XSalsa20/Poly1305 sealed-box). It is a *complete, tested* sealed-box
   facility — but `back_prop.rs:145` calls it ONLY to seal **predecessor PeerId
   records** (trust-bubble privacy), never content. The substrate *already speaks
   the cipher*; the content plane just never asked it to. (Same shape as ARC:
   `set_tgt_storage_arc_hint` already accepted any arc; the policy layer never
   used it.)

3. **The EPR envelope has NO capability/wrap field — but it HAS the lineage hook.**
   `elohim/epr/src/envelope.rs:16-52`: fields are `cid, kind, schema_ref,
   schema_key, reach, coupling, claims, supersedes, superseded_by, issued_at,
   proof`. No `wrap`/`key_ref`/`cap` field. BUT `supersedes: Option<Cid>` and
   `superseded_by: Option<Cid>` (`:41,:46`) already model **content lineage /
   rotation**. A key-rotation of a wrapped blob is *exactly* a supersession: the
   re-wrapped EPR `supersedes` the old one. The hook the rotation needs is
   already in the envelope. `Epr.payload: Vec<u8>` (`epr.rs:20`) is the raw field
   that would instead carry ciphertext.

4. **Key revocation / rotation / supersession machinery is MATURE.**
   `imagodei_integrity/src/recovery_v2.rs:135` `superseded_agent_pubkey`, `:887`
   the `new != superseded` rotation invariant, the whole `KeyRotation` +
   `RevocationAuthority` graduated-authority stack; `db/key_revocations.rs` is the
   committed projection (set_effective, threshold_reached). The protocol already
   knows how to **notarize "this key is dead, that one supersedes it" with social
   quorum.** O5 needs the *content-wrap* analog of this identity-key flow — and
   the flow already exists to copy.

5. **Threshold custody (the co-stewarding answer) EXISTS.**
   `db/custodian_shares.rs` + `p2p/shamir_transport.rs` (the `ShamirShareRequest`/
   `Response` request-response protocol) already split & reconstruct key material
   across custodians with an authorization gate. The wrap key can be
   Shamir-split to household/collective custodians **today** — so revocability
   doesn't cost recoverability. (Note `custodian_shares.rs:25-29`: shares are
   currently stored unencrypted at rest — a known deferred threat-model item; the
   wrap-key custody design must close it, see §3 rung 3.)

6. **The withdrawal *act* already has a home.** The stub's `withdraws-provide`
   Commitment action (riding `Mishpat::Commitment`, mirroring
   `validate_revokes_commitment` at `commitments.rs:342`) is the right notarized
   act — it just needs to ALSO carry the key-rotation, not only the reach-down.

**So: capable? YES at the primitive layer, NO at the wiring layer.** Every piece
exists (sealed-box, supersession lineage, key-revocation flow, threshold custody,
the withdrawal Commitment). Nobody has composed them into a **content-wrap
capability that the person revokes**. That composition is the pass.

---

## 3. The PATH / PIVOT / FORK LADDER (cheapest → deepest)

Each rung names cost, blast radius, and what it unlocks for the vision.

### Rung 0 — Ship the stub as-is (reach-down + honest residual). [BASELINE, not the answer]
- **Cost:** S/M (the stub's MVP).
- **Blast radius:** projection arm + 2 routes; no crypto, no byte-plane change.
- **Unlocks:** *future*-fetch refusal + provable standing-lower + who-holds read.
- **Vision gap:** NOT capture-resistant. A non-cooperative holder keeps a
  readable copy. This is the floor we are escalating *from*. Keep it as the
  reach-gate half — but it is not "agency back to the data."

### Rung 1 — Opt-in content-wrap (encryption-at-rest) for revocable content. [BUILDABLE NOW]
- **What:** New `BlobStore::store_wrapped(data, wrap_pk) → (cid, sealed_bytes)`
  using the **existing** `crypto_box_seal` from `sealed_against_self.rs`. The blob
  on disk becomes ciphertext; the CID addresses the ciphertext (content-address is
  preserved — peers still de-dup, still RS-shard, the quilt is unchanged). A new
  EPR envelope field `wrap: Option<WrapRef>` (`{ scheme, wrap_key_cid }`) — additive,
  `#[serde(default)] Option<_>`, no wire break. Read path: `get_payload` returns
  ciphertext to authorized holders; only a holder of the wrap-capability decrypts.
- **Cost:** M. One new BlobStore method, one additive envelope field, decrypt on
  the authorized read path, a `wrap-keys` capability store (Cat-C local, like
  `acquisition_pins.rs`).
- **Blast radius:** content byte-plane (the big one) — but **opt-in per content**:
  public commons content stays plaintext (no perf hit, still globally cacheable);
  only person-flagged "revocable/private" content is wrapped. Doorway cache and
  RS-sharding operate on ciphertext transparently (they never needed plaintext).
- **Unlocks:** the *precondition* for inertness — bytes at rest are now cipher.
  Withdrawal can now *mean* something cryptographic.

### Rung 2 — Capability revocation = key rotation as a notarized act. [THE RECOMMENDED PIVOT]
- **What:** Extend the stub's `withdraws-provide` Commitment to a
  **`rotates-wrap` / `revokes-capability`** action. Authoring it: (a) notarizes
  "wrap key K_old for content C is revoked, K_new supersedes it" on
  `Mishpat::Commitment` (mirror `validate_revokes_commitment`); (b) the person
  re-wraps the plaintext (which they hold) under K_new, producing a **new EPR that
  `supersedes` the old** (`envelope.supersedes`, already exists, `:41`);
  (c) projects the revocation through the reconcile controller into a
  `wrap_revocations` table (clone of `key_revocations.rs`). **Result: every
  replicated copy of C's *old* ciphertext is now inert — its wrap key is dead,
  notarized, witnessed. The holder has bytes that decrypt to nothing.**
- **Cost:** M (DNA action + validator: S, mirrors existing; projection arm: S;
  re-wrap + supersede EPR: M; the rotation is a known shape from `recovery_v2`).
- **Blast radius:** mishpat coordinator (additive action, DNA-hash-neutral —
  [[project_dna_hash_blind_to_coordinator_zomes]]), one projection arm, the EPR
  supersession path.
- **Unlocks:** **REAL withdrawal.** This is the line the stub could not cross.
  The honest residual report now reads *"3 peers still hold the inert ciphertext;
  none can read it."* Capture-resistance achieved against a non-cooperative holder.

### Rung 3 — Co-stewarded / threshold wrap-capability (hub-optional recoverability). [BUILDABLE NOW, on existing Shamir]
- **What:** Shamir-split K_old/K_new across the person + household-hub +
  collective custodians via the **existing** `custodian_shares` + `shamir_transport`.
  Recovery of the wrap key requires social quorum (`sealed_against_self` 2-of-2 is
  the interim seal). Closes the "lone laptop loses the key = loses the photos"
  failure and the "single key theft = total capture" failure in ONE move.
- **Cost:** M (reuse Shamir machinery; must fix the deferred at-rest share
  encryption noted at `custodian_shares.rs:25-29`).
- **Blast radius:** custody flow only; no new entry type.
- **Unlocks:** revocability **without** sacrificing recoverability or
  hub-optionality. Fractal stewardship of the *key itself* — the collective
  serves the human by co-holding their capability.

### Rung 4 — Re-coverage as an REA commitment + honest who-holds. [BUILDABLE NOW, SOFT-edged]
- **What:** After a rotation, holders must re-fetch the new-ciphertext EPR and drop
  the inert old shards. That re-replication is a `replicates-dwelling`/
  `replicates-commons` Commitment (existing). The who-holds route
  (`graph_views/shefa/distribution.rs:23-60`, today zero-filled) resolves holders
  and **labels each as `readable` (has new wrap) | `inert` (holds dead ciphertext)
  | `purged` (dropped it)** — the data-agency analog of inventory ≠ replication
  ([[project_inventory_exchange_not_byte_replication]]). A holder that keeps
  *serving* withdrawn content earns a Squelch-class standing hit.
- **Cost:** M; SOFT cross-edge on peer-inventory reads (ship dormant, light up when
  inventory reader lands).
- **Unlocks:** the **care-economy loop** — honest re-coverage mints standing;
  dishonest holding burns it. Withdrawal becomes legible and economically real.

### Rung 5 — Portability / export as Cat-C bundle. [BUILDABLE NOW]
- **What:** `GET /api/v1/me/data/export` streams plaintext (decrypted with the
  person's capability) + EPR provenance manifest + the wrap-revocation ledger.
  No new entry type; Cat-C composition over `draw` + EPR projection.
- **Cost:** M. **Unlocks:** *"take my copy and leave."* — the second half of the
  felt promise.

### Rung 6 (FORK CANDIDATE — flag, don't commit) — forward-secrecy / re-keying scheme.
- **What:** sealed-box (`crypto_box_seal`) is revoke-by-rotation, not
  forward-secret: a holder who *cached the plaintext before withdrawal* keeps it.
  No cipher can fix that (P-physics: you can't un-see what you decrypted while
  authorized). A future v2 wrap scheme (group re-keying / lazy re-encryption,
  `SealedBlob.version` already reserves v2 — `sealed_against_self.rs:73`) narrows
  the *window*, never closes it.
- **Status:** **genuine roadmap fork, NOT this pass.** Flag the honest boundary;
  do not over-promise. The honest claim is *"withdrawal makes future reads of the
  bytes-at-rest impossible; it cannot un-read what an authorized peer already
  decrypted."* That boundary must be in the UI copy and the a2o honesty scenario.

---

## 4. The recommended ESCALATION (defended) + what it COMMITS US TO

**Recommendation: build Rungs 1+2+3 as the data-agency core (the pivot), keep
Rung 0 as the reach-gate complement, ship Rungs 4+5 alongside, flag Rung 6 as a
named roadmap fork.**

The defended center is **Rung 2 — capability revocation via key rotation as a
notarized act** — sitting on **Rung 1** (encryption-at-rest) and recoverable via
**Rung 3** (threshold custody). This is the literal ARC-worked-example move applied
to data agency:

- *The artifact:* "you can't un-replicate bytes, so withdrawal must be cosmetic"
  is a **layering artifact** of a plaintext byte-plane — NOT physics.
- *One layer down:* `crypto_box_seal` already exists (`sealed_against_self.rs:32`)
  and the envelope already models supersession (`envelope.rs:41`). The substrate
  already speaks the quilt of capability.
- *What's missing:* nobody wired the cipher to the content plane or modeled the
  wrap key as a person-held, revocable capability. That POLICY/WIRING composition
  is the fork-equivalent — "write our capability layer," not "fork Holochain."

**Why this over the stub's residual-only answer:** the stub is *honest* but
concedes the adversary the vision exists to resist. Capture-resistance against a
hostile or neglectful holder is *the* differentiator versus web2 "delete"
(which is also just a cooperative flag). Cryptographic inertness is the only
mechanism that makes "give me my data back" true under P2P physics. The operator's
north star names "capture-resistant against the real world, its externalities,
and its messiness" — a non-cooperative holder *is* the messiness.

**What it COMMITS US TO (mark the irreversibles):**

1. **NEW PRIMITIVE — content-wrap capability** (`WrapRef` envelope field +
   `wrap-keys` capability store + `BlobStore::store_wrapped`). Additive, no DNA
   entry spend. *Commitment: a new cross-cutting concept (content can be sealed
   under a person-held capability) that the whole read/cache/shard path must treat
   as opaque ciphertext.* Buildable now.
2. **NEW DNA ACTION** (`rotates-wrap`/`revokes-capability`) on the existing
   `Mishpat::Commitment` — **NO new entry type** (rides the existing entry, the one
   near-irreversible spend explicitly NOT made). DNA-hash-neutral (coordinator +
   action discriminator only). Buildable now.
3. **ROADMAP FORK (flagged, not taken):** Rung 6 forward-secrecy / re-keying
   scheme + the v2 `SealedBlob` threshold scheme already reserved. *This is the one
   genuine "future crypto research" commitment — name it, scope it out of MVP.*
4. **A threat-model debt to close:** `custodian_shares.rs:25-29` at-rest share
   encryption must be fixed if wrap keys are Shamir-custodied (Rung 3 depends on it).
5. **An honesty invariant** (a2o-enforced): the UI/read-model must distinguish
   `inert` (bytes-at-rest now noise) from the residual it CANNOT fix (a peer who
   decrypted while authorized and kept plaintext). Never claim more than the cipher
   delivers.

**Operator decisions still required** (the genuine forks/values calls):
(a) Bless the content-wrap primitive (Rung 1/2) as the path vs accept residual-only.
(b) Confirm **opt-in** wrapping (commons stays plaintext) vs wrap-everything.
(c) Bless Rung 3 co-stewarded custody (and funding the share-encryption fix).
(d) Confirm Rung 6 is a named roadmap fork, not an MVP promise.
(e) Confirm withdrawal-of-standing ALWAYS succeeds (never operator-vetoable) —
the stub's open-Q1, unchanged and reaffirmed here: the person's act is sovereign.

---

## 5. COUPLING — story + value + governance as one whole

This is not a crypto feature; it is the **trust/value/governance triad** the north
star demands, instantiated on the data-agency axis:

- **STORY (the felt).** Grandma taps *"un-share these photos."* The copies on her
  neighbor's old tablet turn to noise — provably. She sees *"held by 3 peers: 1
  re-fetched the new version (readable to household), 2 hold inert copies."* She
  exports her readable album and walks. The felt experience is **dignity and
  control under adversity** — the opposite of web2's "we'll delete it (trust us)."

- **VALUE (the donut / care-economy).** Every move is REA-shaped on the **one**
  Commitment primitive ([[project_rea_compute_commitment_primitive]]):
  - The original share is a `provide` EconomicEvent
    (`content_store_integrity/src/lib.rs:1116`).
  - The withdrawal is `revokes-capability` — its **inverse**, person-emitted.
  - Re-coverage after rotation is `replicates-dwelling`/`replicates-commons` — a
    care commitment that **mints standing** for the honest holder.
  - A holder who keeps serving withdrawn content burns standing (Squelch-class).
  - Co-stewarding the wrap key is itself a care commitment (the collective serves
    the human). **arc-as-coverage-commitment ≡ compute-as-commitment ≡
    care-as-commitment ≡ capability-as-commitment** — one substrate, now a fourth
    instantiation. Value is minted by honest care, exactly the donut.

- **GOVERNANCE (capture-resistance + high-integrity DHT).** The revocation/rotation
  is a notarized `Mishpat::Commitment` — witnessed, auditable, on the DHT where no
  one can capture it. The wrap key is threshold-custodied (Shamir) so neither a
  lone key-theft NOR a hostile hub can capture the person's data — recovery *and*
  revocation both require social cooperation (the `sealed_against_self` 2-of-2
  property: never unilateral disclosure). Standing lives on the DHT, not an admin
  console — **hub-optional** ([[project_hub_optional_floor]]): the laptop holds the
  lever. This is capture-resistance made structural: against a hostile holder
  (cipher inertness), against key theft (threshold custody), against an operator
  veto (the person's withdrawal always lands), against silent failure (the honesty
  invariant). The system **stays in stasis** because the capability lever is the
  person's, the audit is the commons', and the byte-plane is honest about exactly
  what it can and cannot promise.

**The coupling in one sentence:** a person revoking their data is simultaneously a
*felt* act of dignity, an *economic* act on the care-commitment ledger, and a
*governance* act notarized on the high-integrity DHT — and the cryptographic wrap
is what makes all three TRUE rather than cosmetic against the messy, hostile real
world.

---

## Appendix — substrate citations (read, real source)

- Plaintext byte-plane (the artifact): `elohim/elohim-storage/src/blob_store.rs:219,267`.
- Cipher already in-tree, wired to the wrong thing:
  `elohim/elohim-storage/src/services/sealed_against_self.rs:32` (`crypto_box_seal`);
  `elohim/elohim-storage/src/services/back_prop.rs:145` (only use = predecessor records).
- EPR envelope (no wrap field; HAS supersession lineage):
  `elohim/epr/src/envelope.rs:16-52` (`supersedes`/`superseded_by` at :41,:46);
  raw payload `elohim/epr/src/epr.rs:20`.
- Key revocation / rotation / supersession flow to copy:
  `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/recovery_v2.rs:135,887`;
  projection `elohim/elohim-storage/src/db/key_revocations.rs`.
- Threshold custody already built: `elohim/elohim-storage/src/db/custodian_shares.rs`
  (at-rest encryption deferred, :25-29); `elohim/elohim-storage/src/p2p/shamir_transport.rs`.
- Withdrawal act home (the stub's correct primitive choice):
  `mishpat/zomes/mishpat/src/commitments.rs:342` (`validate_revokes_commitment`);
  projection `elohim/elohim-storage/src/mishpat_projection.rs:154-179`.
- Reach gate (the complementary half): `elohim/elohim-storage/src/p2p/reach_authorization.rs`.
- Who-holds skeleton (zero-filled today): `graph_views/shefa/distribution.rs:23-60`.
- DevicePin Cat-C local-store pattern (template for `wrap-keys` store):
  `elohim/elohim-storage/src/db/acquisition_pins.rs`.
