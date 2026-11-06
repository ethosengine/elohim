# Rulings R1–R4 (2026-11-05, after the Astra challenge)

Ruled by the session model (Claude Fable 5.1) after four independent gpt-6-astra challenge legs
(`astra-challenge-R{1..4}.md`), each of which returned REFUTED against `rulings-draft.md`. Every
challenger finding was weighed on its evidence; code claims the rulings rest on were re-read in
source by the ruling model. Frame: generational architecture — rework to get a foundation right is
accepted; a convenience that would have to be torn out is not.

## R1 — content address, and the BLAKE3 iroh-blobs store

**Challenge accepted (high):** my draft "admitted BLAKE3 as an interchangeable identity" without a
compatibility transition; a transport cache holding bytes *is* a second copy; "servable when held"
turned possession into authority; and two honest producers choosing different hashes would fragment
D5's (derivation, output CID) convergence. New defect confirmed in source: `parse_content_address`
accepts any CID, discards its codec and multihash, and returns the digest as presumed SHA-256
(`elohim-storage/src/blob_store.rs:228-232`). The `^bafkrei` schema-pattern claim was wrong (no
pattern exists), but the `blobCid` description does fix raw/sha2-256.

**Ruling.**
1. The byte address is **CIDv1, self-describing in codec, multihash and digest length**. Each object
   class declares, per protocol epoch, its **canonical emission profile** and the profiles it accepts.
   Today: artifact bytes and chunks are raw (0x55) + sha2-256 (0x12); structured records are dag-cbor
   (0x71) + sha2-256. **BLAKE3 is an allowed internal transport and storage hash, not an identity**
   for release artifacts, chunk manifests or `DerivationRecord.output`. Admitting a new identity hash
   is a governed migration (readers, schemas, verification, custody, retention, deterministic-output
   agreement), never a schema-regex change.
2. **Output identity is canonicalized at production**, so agreement never consults an equivalence.
   A locally verified sha2-256 ↔ BLAKE3 mapping is a rebuildable index that may locate a payload and
   speed a transfer; it never rewrites a signed record, merges principals, transfers reach, or
   manufactures agreement. A peer's alias assertion is never recorded as equivalence.
3. **Bao authenticates the transfer against its BLAKE3 root; acceptance verifies the canonical CID.**
   Both, always; never Bao alone.
4. **One authoritative payload store per device.** Transport indexes and bao outboards may reference
   its bytes; nothing keeps a second persistent payload copy. The iroh-blobs `FsStore` may *be* that
   store only after it demonstrates, with receipts: canonical-CID answer through the index; atomic
   recovery; alias-index reconstruction from payloads; daemon-independent local operation; scoped
   serving; and retention evaluated across every live reference (its tag-based `forget` must never
   collect bytes a custody commitment, hold or arrival still names). Until then pure-iroh artifact
   delivery waits or rides a demonstrated shared-payload implementation. The "Phase 11 cutover"
   comment in `p2p_iroh/blob_store.rs:7-8` is intent, not a ruling.
5. **Address resolution is not authorization.** Holding verified bytes permits lookup; every serve
   applies the object's reach and the requester's task grant (plaintext) or the quarantine and pledge
   rules (ciphertext). Hash aliases inherit no permission.
6. New surfaces refuse the legacy `sha256-<hex>` and `blake3-<hex>` spellings. Compatibility readers
   validate the algorithm explicitly: `parse_content_address` must refuse a CID whose multihash is
   not 0x12 instead of reinterpreting its digest (defect, sprint lane H4). `BlobCid::verifies` keeps
   failing closed on any profile it does not implement.
7. DNA custody entries (`sha256-…` strings) migrate to CIDv1 in the batched DNA move with their
   digests preserved; rakia's `blobCid` + `sha256` pair stays until a coordinated transition, both
   describing the same bytes.

## R2 — custody admission for wider release channels

**Challenge accepted (high):** my draft contradicted D6 and itself at Intimate (k ≥ 1 while the
publisher's devices never count); `custodyFloor` as an ignorable extension violates D7; the numeric
ladder had no failure model and conflated whole-copy holders with RS shard holders; dwelling hub is
one declared dimension, not proven independence; 900 s repair cannot be promised under consent and
the existing kick limits (a 454-chunk refill alone is ~45 min of kick budget).

**Ruling.**
1. **Private, SelfScope and Intimate create zero DHT heads and require no replica quorum.** Local put,
   build, reuse and channel operation never wait on anyone. A second device's copy is visible
   resilience evidence shown per copy, never a prerequisite and never notarized.
2. **At Trusted reach and above**, a release channel's election is admitted only under a **custody
   admission policy** carried in `adoptionDiscipline` as a versioned constitutional policy (its own
   policy epoch per D7; an older reader that cannot interpret it cannot admit an election under it).
   The policy names: the complete manifest closure; coding parameters; the **tolerated failures** it
   promises to survive; accepted evidence ages; the retention window. Admission demonstrates every
   required object is reconstructible under those failures. Whole-copy custodians and RS shard
   holders are counted differently, and today's 64 MiB shard threshold is applied to the artifact,
   never to 1 MiB tier chunks.
3. **Evidence that counts:** an authorized `custody-blob` commitment **and** authenticated,
   replay-resistant availability evidence (R3/H1) **and** a bounded-age verified fetch attributable
   to the serving holder, evaluated inside a declared observation window. Inventory rows alone never
   count. `unknown` is kept distinct from confirmed loss. The publisher's own devices never satisfy
   an external-custody requirement.
4. **Failure domain = a declared dimension with provenance.** Household (dwelling hub) is the first;
   site, provider and administrative independence may be declared later. An unresolved or
   correlated placement is reported as unknown, never advertised as independence.
5. **Experimental profile 0** (to be measured, not promised): Trusted/Familiar survive the loss of
   one whole-copy custodian (k ≥ 2, two declared households); Community/Public survive the loss of
   two (k ≥ 3, two households) or, above the shard threshold, RS 4+3 with four distinct shard indices
   across at least two households; Commons survive the loss of four (k ≥ 5, three households).
   Inventory horizon 600 s stays; **repair within 900 s is a measured objective** with explicit start
   conditions and capacity assumptions, and a missed deadline is reported with its cause.
6. **Liveness is honest, never forced.** Missing consent, capacity or evidence yields a durable
   `pending-election` reason visible on the channel; it blocks no local work and conscripts nobody.
   After election, loss of evidence yields `unknown` or `under-custodied`; it never rewrites the
   elected head by itself.
7. Commitments bind to bounded closure or channel windows; availability observations stay
   operational and create no per-chunk DHT heads (reconcile with the footprint spec's one commitment
   per hub per channel before choosing notarization grain).

## R3 — hardening lane H1–H4 and its dependencies

**Challenge accepted (high):** E4 composes `BlobStoreTier` over `BlobStore`, so H2 *does* gate E4;
device admission is not object authorization (`gate.rs:64` ignores root and object CIDs); H1 must
cover deltas, custody announcements (`gossip_dispatch.rs:795` projects `holder_agent_cid` unbound)
and iroh end-to-end, not only libp2p snapshots; H2 must be representation-aware (chunked marker +
index + `.d` files, RS shards verified against shard hashes, staging files excluded from `list_hashes`
and tallies).

**Ruling.**
1. H1–H4 are **mandatory hardening in this sprint**, on their own elohim-storage write-set, run
   alongside independent Track E work, with explicit acceptance dependencies rather than "lane 0".
2. **E2 proceeds independently. E3 proceeds on the directory backend** once the tolerant reader is
   in, and keeps its own per-object scope gate and the unauthorized-CID refusal receipt; roster
   admission never authorizes general blob access.
3. **H2 gates the acceptance and activation of E4's `BlobStore` composition and every F2 receipt
   that rides it.** H2 delivers durable, verified publication across inline, chunked and shard
   representations (chunks and index prepared and synced before the marker; directory entries
   synced); "already exists" means "exists and verifies"; verify-on-read for inline blobs; corruption
   quarantined; staging and quarantine files excluded from inventory, tallies and serving. Existing
   identities and layouts stay readable.
4. **H1 gates every use of remotely advertised holdings as custody evidence, at every reach and
   transport.** libp2p: the payload's holder identity equals gossipsub's signed `message.source`
   (threaded through dispatch; relays preserved). iroh: end-to-end holder authentication on the
   payload. Every ingress that feeds custody (snapshots, deltas, custody announcements, salvage ads)
   enforces identity binding, freshness, replay protection and evidence provenance before touching a
   projection. Authentication proves authorship; possession and commitment are R2's separate
   evidence.
5. **Stage 2 is not a deferral of H1.** The authenticated envelope lands with H1 under a declared
   level and D7 epoch; universal *rejection* of legacy advertisements follows measured adoption, and
   until then legacy rows are fetch hints that can never satisfy a custody floor.
6. **H4 (new):** `parse_content_address` refuses non-sha2-256 multihashes instead of reinterpreting
   digests (R1.6).
7. H1 receives a Claude-family adversarial review before landing. Local puts and builds keep working
   with no peers, no quorum and no DHT heads throughout.

## R4 — release lifecycle: supersede, withdraw, standing, security and harm

**Challenge accepted (high):** "yank keeps serving pinned peers" defeats a security withdrawal; "head
moves to the prior winner" bypasses present qualification and contradicts the election design (a
ceremony declares a prior head canonical, then the same verification and adoption loop runs);
"revoke retains for audit" conflicts with the swarm spec's terminal **refuse** (mandated local
destruction) and the inalienable spec's consent withdrawal (must stop replication and serving);
restrictive cells as ignorable manifest defaults violate D7; three labels omitted expiry, deprecation
and commitment termination; "retain: yes" must mean "while an obligation remains".

**Ruling.** Release lifecycle, current eligibility, serving authorization and custody retention are
**separate decisions composed under the constitutional floor**, each act binding its subject, scope,
issuer, authority evidence, effective interval, reason and policy epoch.
- **Supersede** records that an authorized election chose another release; the former loses
  current-head status and keeps its history; it becomes eligible again only by current qualification
  and a new election.
- **Withdraw (yank)** is a channel-scoped act under a verified delegation or the channel's standing
  rule. It prevents new adoption in its scope. It elects no replacement and grants no serving
  authority; pinned recovery continues only where current reach, grant and safety policy allow.
- **Standing revocation** records whose authority ended, when, and for which acts. Signatures and
  publication-time standing stay assessable from retained evidence; prospective loss of standing
  does not invalidate prior releases; a compromise finding names the affected releases or interval.
- **Security hold and harm** override the defaults, reusing the existing pair: **quarantine**
  (expiring; blocks adoption, serving and redistribution while bytes stay held under the authorized
  hold) and **refuse** (terminal; actuates the mandated local destruction). **Subject consent
  withdrawal** removes further replication and serving authority. No channel policy, pin or custody
  commitment overrides these floors. Global recall is never promised, and the limit is stated plainly.
- **Heads move only through the election mechanism** among presently qualifying candidates (R2's
  evidence included); each adopter verifies against its installed reality; no qualifying candidate
  yields explicit `no-eligible-head`; an unavailable authority yields `unreachable`, never absence.
- **Custody follows bounded obligations.** Status alone neither conscripts retention nor orders
  purge; termination reconciles commitments, advertisements, providers, stores and caches; audit
  evidence is retained separately from payload availability. **Deprecation** is advisory; **expiry**
  and review transitions are explicit.
- The matrix cells live in `adoptionDiscipline` under their own policy epoch (D7), proven by a
  mixed-version refusal test. General enforcement stays gated on the standing design (spec §13); the
  existing `PathEvidence` lineage-roster checks are reusable evidence, not blanket authorization.
  The standing body today has one member: a fact about the network, not an authorization check.

## What the challenge changed in the handoff (§7.3)

- E4 is gated by H2; E2 and directory-backed E3 are not.
- H1's scope: snapshots, deltas, custody announcements and salvage ads; iroh end-to-end; freshness
  and replay before projection.
- H4 added (`parse_content_address` algorithm confusion).
- Stories (`@wip`): pending-election, quarantine vs refuse, consent withdrawal, no-eligible-head.
- `adoptionDiscipline` gains a policy epoch before any restrictive field (custody admission,
  lifecycle cells) is read.
