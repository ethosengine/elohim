---
title: "Canonical bytes vs. schema-relative encoding — is serde the right layer? (schemaboi, Borsh, SSZ, Avro, dCBOR)"
id: serialization-canonicality-cross-pollination-2026-08-11
status: Capture
date: 2026-08-11
---

# Serialization Canonicality Cross-Pollination — August 2026

Prompted by an operator question after a Joseph Gentle talk: **is `serde` the right trait/library for eprfs, or is there a more native/performant route?** — pointing at [schemaboi](https://github.com/josephg/schemaboi), and then widening: *"schemaboi might not be the right repo, but it's a very-performant data schema approach that I'm curious if someone has picked up and mastered somewhere."*

The one-line verdict: **the premise mis-locates the layer twice over, and the drill-in found a real defect underneath it.** `serde` is not a wire format, and — verified in source — **serde derive is not on the canonical-hash path anywhere in `elohim/epr`**: every `canonical_bytes()` is a hand-built `BTreeMap<String, Ipld>`. Swapping the serialization framework would not change a single content-addressed byte. The genuinely open question is one layer down (*should schemas be content-addressed EPRs?*), and the genuinely actionable finding is one layer sideways: **eprfs tags non-CBOR bytes with the dag-cbor codec**, so the same file gets two different CIDs depending on which eprfs code path addresses it.

**Verification key:** ✅ verified in source · ◐ single-source/plausible · ⚠ web-only/unverified.

---

## 1. Separating the two things the framing conflates

`Serialize`/`Deserialize` are Rust traits; the *format* is a separate crate. What the codebase commits to is **dag-cbor** (`serde_ipld_dagcbor`), and serde is the glue reaching it. So the real question is never "serde vs schemaboi" — it is **"dag-cbor vs a schema-relative binary format,"** and that question has a much harder constraint attached than performance.

**The format is load-bearing for identity, not speed.** `elohim/epr/src/cid.rs::compute_cid` hashes canonical bytes; the CID *is* the thing's identity ✅. IPLD dag-cbor specifies a strict canonical form — sorted keys, definite lengths, no float ambiguity (RFC 8949 §4.2.1, quoted in `epr/src/cbor.rs` ✅) — precisely so two independent implementations produce byte-identical output for the same logical value. That is what makes content addressing work between peers who share no code. Change the format and **every CID in the system changes, plus every signature**, since signatures sign canonical bytes ✅ (`envelope.rs::canonical_bytes`).

---

## 2. Ground truth: where serde actually sits (verified)

The drill-in found **three distinct layers**, only one of which touches CBOR at all.

| Layer | What produces the hashed bytes | Encoder | Verdict |
|---|---|---|---|
| `elohim/epr` — the real EPR atom codec | hand-built `BTreeMap<String, Ipld>` | `cbor::encode` → `serde_ipld_dagcbor::to_vec(&Ipld)` | ✅ genuinely canonical dag-cbor |
| `eprfs-core::BlobCid` | *nothing* — it only hashes bytes handed to it | none | ✅ codec-agnostic by construction |
| `eprfs-agent::CanonicalAgent` | **ad-hoc line-oriented text** (`"slug:…\ndescription:…"`) | `String` concatenation | ⚠️ not CBOR, tagged as CBOR |

The decisive line is a comment the crate wrote about itself — `epr/src/witness.rs:24` ✅:

> "The hand-built map (**not a serde derive**) IS the [canonical form]"

Every `canonical_bytes()` in `elohim/epr` — `Envelope` (`envelope.rs:69`), `WitnessedInteraction` (`witness.rs:92`), `FrameClassification` (`witness.rs:194`) — hand-inserts into a `BTreeMap<String, Ipld>` and calls `cbor::encode`. `cbor::encode` takes **only `&Ipld`** ✅, never a generic `T: Serialize`. And `measure.rs` — the newest vocabulary (`MeasureKind`/`Interval`/`Confidence`/`Quantity`) — has **no `canonical_bytes` at all** ✅: its serde derives are JSON/wire ergonomics, not the content-hash path.

So serde appears on the hash path exactly once, as the *mechanical driver over an already-canonical dynamic `Ipld` value*. There is no derive-based struct→bytes mapping to make "more native."

**One correction to the premise worth stating plainly:** serde derive is monomorphized and generally not a bottleneck. Any real performance win here comes from the format's key repetition and canonical sorting, not from the framework. Where compactness genuinely bites — dag-cbor repeats every map key as a string in every value; a `Quantity` encodes to 116 bytes for what is semantically four numbers, two enums, and a string — the cure is schema-relative encoding, which is §4's question, not a serde question.

### 2a. The defect found underneath (actionable now)

`eprfs-core/src/address.rs` documents its own rule ✅:

> "Arbitrary file/body/blob bytes are NOT dag-cbor and **must** use `BlobCid::compute_raw` (codec `0x55`) so the codec tag tells the truth about the bytes."

Three call sites violate it:

| Site | Input | Codec used | Should be |
|---|---|---|---|
| `eprfs-agent/src/canonical.rs:139` | hand-rolled **text** envelope | `compute` → `0x71` dag-cbor | `0x55` raw |
| `eprfs-storage/src/lib.rs:83` (`put_blob`) | arbitrary rendered file bytes (markdown) | `compute` → `0x71` | `0x55` raw |
| `eprfs-local/src/verify.rs:36` | `fs::read()` file bytes | `compute` → `0x71` | `0x55` raw |

**This is not a live break inside eprfs** — `put_blob` and `verify` both use `compute`, so drift detection is internally consistent ✅. Two real consequences remain:

1. **Operator-facing divergence.** `eprfs-cli` addresses a file with `compute_raw` → `bafkrei…` ✅ (`main.rs:90`), while the projection manifest addresses the *same bytes* with `compute` → `bafyrei…`. `eprfs cid <file>` can never reproduce a manifest's blob CID.
2. **Latent interop break.** `rust-ipfs/src/block.rs:58` dispatches on `self.cid.codec()` ✅ — a `0x71`-tagged blob containing markdown routes to `DagCborCodec::decode_from_slice` and fails. The same dispatch exists in `elohim-storage/src/dag_store.rs:103`. Any eprfs blob CID crossing into an IPLD-aware consumer breaks the moment it is dereferenced.

The cite-fingerprint parity survives by luck: `short_fingerprint()` reads only the multihash digest, which is codec-independent — so the `sha256:hex16` short form matches across both codecs even though the full CIDs differ.

---

## 3. The format family — who actually mastered this

Broadening past schemaboi as directed. The field splits cleanly along an axis the operator's question implicitly assumes is one thing but isn't: **canonicality** (same value → same bytes, always) and **schema-relative compactness** (drop the key names) are *separable*, and almost nobody ships both plus content-addressed schema identity.

| Format | Canonical? | Schema-relative? | Maturity | Note |
|---|---|---|---|---|
| **dag-cbor** (ours) | ✅ by spec | ❌ keys repeat | production, multi-impl | RFC 8949 §4.2.1; the IPLD ecosystem's floor |
| **Borsh** | ✅ *designed for hashing* | ✅ non-self-describing | production (NEAR, Solana) | ⚠ "Binary Object Representation Serializer for **Hashing**"; strict spec, maps sorted lexicographically, enums as `u8` ordinal |
| **SSZ** | ✅ deterministic | ✅ schema known in advance | production (Ethereum consensus) | ⚠ adds **merkleization** → `hash_tree_root` with inclusion proofs |
| **Avro** | ◐ has *Parsing Canonical Form* | ✅ writer schema required | production since ~2009 | ⚠ schema **fingerprints** (SHA-256/CRC-64) — the mature "content-address the schema" prior art |
| **dCBOR / Gordian Envelope** | ✅ deterministic profile | ❌ | production (Blockchain Commons) | ⚠ built for exactly "identical semantic data → identical bytes" |
| **schemaboi** | ❓ unspecified | ✅ schema embedded with data | ⚠ **"EXPERIMENTAL. Do not use this for anything you care about yet!"**, JS/TS only | merges local+remote schema; foreign data preserved on round-trip |
| **Cap'n Proto / FlatBuffers / rkyv** | ❌ padding & ordering freedom | ✅ zero-copy | production | **disqualified for content addressing** — fast, but not byte-stable |

**The two that answer the operator's actual question — "someone picked this up and mastered it":**

- **SSZ is the mastered answer to peer diversity.** Ethereum's consensus layer runs many *independently written* client implementations that must agree byte-for-byte on canonical encodings and hash-tree-roots, across scheduled fork upgrades. That is exactly "maintain communications between a diversity of peers" solved at production scale. Its schema-evolution answer is **fork-versioned schemas agreed out of band** — every client knows every fork's schema in advance.
- **Avro is the mastered answer to schema evolution.** A reader must use the writer's schema; the schema travels with (or is fingerprinted alongside) the data, and *Parsing Canonical Form* + a SHA-256 fingerprint is, functionally, a content-addressed schema identifier — shipped for fifteen years.

**schemaboi's contribution is real but experimental**: it embeds the schema *with* the data and merges local+remote at runtime, so *"anything your application doesn't understand is stored separately, and re-encoded when the data is saved back to disk. Round-trips never lose data."* ⚠ That foreign-data preservation is the property version-skewed peers need. But the README's own status line disqualifies it as a dependency, it is JS/TS only, and — critically — **it does not specify canonical/deterministic encoding**, which is the one property we cannot trade away.

### 3a. Why schema-relative encoding collides with content addressing as currently built

If bytes are only interpretable relative to a schema, the CID becomes a hash of *"bytes + implied schema."* To keep addressing honest you would have to **content-address the schema itself and reference it in the envelope**. Notably, `Envelope` already has the field: `schema_ref: Cid` + `schema_key: String` ✅ (`envelope.rs:23-29`) — "CID of the Manifest EPR that declares the payload schema." The hook for this design exists and is unused for encoding.

That is arguably elegant — schema-as-an-EPR fits this substrate's grain — but it is a **substrate redesign, not a library swap**, and the two mature precedents (Avro fingerprints, SSZ fork versioning) are the proven ways to bind bytes to a schema identity.

---

## 4. The transformation half — Cambria (Ink & Switch)

Operator pointer, and the sharpest one in this survey: [cambria-project](https://github.com/inkandswitch/cambria-project) — *"Schema evolution with bi-directional lenses."* It is **not a competitor to dag-cbor and does not touch the wire format at all**, which is exactly why it matters. The two halves finally separate cleanly:

- **dag-cbor answers**: how do bytes get produced *identically* on peers that share no code? (canonicality)
- **Cambria answers**: how does a peer *interpret* data written under a schema version it has never seen? (evolution)

§3's candidates all answer the first question and mostly duck the second. Avro and SSZ give you schema **identity** (a fingerprint; a fork version). Cambria gives you schema **transformation** — a declarative, invertible path *between* two versions. Row 10 needs both, and until now the survey only had the identity half.

**Already in the corpus, un-adjudicated** ✅: brit's universal-interoperability prior-art pass cites "Cambria lenses" as one-line research grounding alongside IPLD/CAR · WIT worlds · UCAN · Unison · AT-Proto Lexicon · SHACL (`elohim/brit/docs/specs/2026-06-29-canonical-epr-meta-git-bridge-design.md:182`). This section is that adjudication.

**Lens primitives** ⚠: `rename`, `add` (with default), `remove`, `convert` (value mapping), `wrap`/`head` (scalar↔array), `in` (apply within nested), `hoist`/`plunge` (move data between nesting levels). Lenses form a **directed graph whose nodes are schema versions and whose edges are bidirectional transformations**; translating between distant versions composes lenses along the shortest path.

### 4a. The naming trap — read this before borrowing anything

**`lens` is already load-bearing, DHT-and-DB-backed vocabulary in this codebase, on a different plane** ✅. `elohim-storage/src/db/lenses.rs::find_lenses_governing_epr` — ours *govern* an EPR (plural-Mishpat interpretive/governance standing, with `version_parent` and its own version-DAG policy question, see [lens-version-dag-policy-dependency](epr:lens-version-dag-policy-dependency)). Cambria's *translate a schema*. Same word, orthogonal planes.

Anyone borrowing Cambria and calling the artifact a "lens" will collide head-on with an existing entry-type concept. **Call it a schema *migration* or *transform*; reserve "lens" for governance standing.** This is a cheap mistake to avoid now and an expensive one to unwind after it reaches a DHT entry type.

### 4b. Where it collides with content addressing (the honest tension)

Cambria's home is Automerge, where a document's identity is its op-log — *not* a hash of its bytes. Port that assumption naively and it breaks here: **a lens-translated document has different bytes, therefore a different CID, therefore a different identity, signed by nobody.**

The resolution is available and already native to this substrate: **translation must be a read-time projection, never a re-notarization.** The original CID stays the identity; the transformed value is a *derived view* with a derivation edge back to it — which is precisely `eprfs-core`'s existing `DerivationKind::Projection` and the standing "storage as projection, not truth" rule. A migration that mints a new notarized EPR is a supersession (`Envelope.supersedes`), and that is a governance act, not a decode step. Keeping those two paths distinct is the whole design.

Cambria's own distribution choice is instructive by contrast: **lenses are embedded in the document** ⚠ (cambria-automerge writes the lens source into the op-log so peers can retrieve unknown transformations) — the same family as schemaboi's embedded schema, and the opposite of Avro's fingerprint. In a content-addressed substrate the natural third option is neither: **the lens is itself an EPR, referenced by CID**, which is row 10 exactly.

### 4c. Maturity — verified, and it is a blocker

| Repo | Last push ✅ | Stars | Status |
|---|---|---|---|
| `cambria-project` | **2024-06-14** (~2 yrs dormant) | 700 | not archived, TypeScript-only, 11 open issues |
| `cambria-automerge` | **2023-01-07** (~3.5 yrs dormant) | 22 | the Automerge integration is the more dormant half |

Self-described: *"Cambria is still immature software, and isn't yet ready for production use"* (README) and *"we do not pretend to have delivered a fully formed, production-quality solution"* (essay) ⚠.

**Stated open problems that bear directly on us** ⚠: `convert` "doesn't fit the technical definition of a lens" (manual forward/backward mappings weaken the consistency guarantee); **unknown-field preservation is a gap** — notably the one thing schemaboi does well, so the two are complementary rather than rival; recursive schemas; cross-document split/merge; unformalized performance under multi-schema reads; and the candid admission that *"there are limits to interoperability"* when schemas diverge far enough.

**Verdict on Cambria: adopt the model, not the dependency.** TypeScript-only against a Rust truth layer, two-plus years dormant, and self-declared pre-production — it cannot be a dependency here. What transfers is the *design*: a schema-version graph with invertible edges, shortest-path composition, and read-time translation. That is a technique, exactly as sedimentree is in row 9 — and it converges with row 9 on the same structure. **Three items now point at one graph**: row 9's levelled strata for the L2 version-lineage DAG, the lens-market's own `version_parent` DAG policy question, and Cambria's schema-version graph. Whoever picks up row 10 should notice they are the same shape before designing a third one.

---

## 5. Verdict

**Right call for now; wrong layer to revisit it at.** Keep dag-cbor. It is doing a job — cross-implementation byte-identity for content addressing and signatures — that no candidate in §3 does better *while also* being canonical, except Borsh and SSZ, both of which would require content-addressing the schema to stay honest, i.e. the same redesign.

**Nothing currently being built locks this in.** `MeasureKind`/`Interval`/`Confidence`/`Quantity` are plain data; the serde derives are annotations over them. If the format changed, the vocabulary survives and the derives get rewritten.

**The wire-format question and the DNA-upgrade question are the same question at two layers.** A peer that cannot decode a newer peer's payload and a peer on a stale DNA hash are the same failure — *schema skew across a diverse peer population* — which is why this survey's mint-pass row cites [governance-native-dna-upgrade-path](epr:governance-native-dna-upgrade-path). Holochain's answer is the DNA hash over integrity zomes; Avro's is the schema fingerprint; SSZ's is the scheduled fork. Ours would be `Envelope.schema_ref` — *if* we ever make it load-bearing.

## Outputs (mint pass)

- **[arch-dataplane-borrows-backlog](epr:arch-dataplane-borrows-backlog) row 10** — schema-as-content-addressed-EPR, p2p-design-gated, cited to the DNA-upgrade path.
- **[eprfs-address-reuse-brit-cid-codec](epr:eprfs-address-reuse-brit-cid-codec)** — updated: original premise landed; the §2a codec-tag residue is the remaining work.

**Dies honestly here (no work item):** schemaboi as a dependency (experimental, JS-only, canonicality unspecified); **Cambria as a dependency** (TypeScript-only, ~2 yrs dormant, self-declared pre-production — the *model* survives in row 10, the crate does not); Cap'n Proto/FlatBuffers/rkyv (not byte-stable — structurally disqualified); replacing serde as a performance measure (not on the hash path; monomorphized; not the bottleneck).

## Sources

- [schemaboi](https://github.com/josephg/schemaboi) — Joseph Gentle, experimental schema-relative format
- [Borsh](https://borsh.io/) · [near/borsh](https://github.com/near/borsh) — canonical serializer built for hashing
- [SimpleSerialize (SSZ)](https://ethereum.github.io/consensus-specs/ssz/simple-serialize/) · [ethereum.org SSZ](https://ethereum.org/developers/docs/data-structures-and-encoding/ssz/) · [Upgrading Ethereum §2.9.7](https://eth2book.info/latest/part2/building_blocks/ssz/)
- [Apache Avro Specification](https://avro.apache.org/docs/1.11.1/specification/) — Parsing Canonical Form + schema fingerprints
- [CBOR vs. the Other Guys](https://cborbook.com/introduction/cbor_vs_the_other_guys.html) — dCBOR / Gordian Envelope determinism rationale
- [cambria-project](https://github.com/inkandswitch/cambria-project) · [Project Cambria essay](https://www.inkandswitch.com/cambria/) — Ink & Switch, bidirectional schema-evolution lenses (Litt, van Hardenberg, Henry)
