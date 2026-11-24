---
title: "Accepting a release closure from a sibling or the forge without a source rebuild"
id: gate-attestation-evidence-release-acceptance
status: draft
class: process-meta
date: 2026-11-24
context-tier: disclosed
steward: human:matthew
author: "Claude (session d6cfb7e1, Opus 5.5, ethosengine device); an unreviewed proposal for integrator shem, read from the code at dev 9f272d343"
serves: [push-gate-pays-once]
tags: [release, artifact, attestation, acceptance, evidence, proposal]
graduation-trigger: >-
  The steward rules on N, the forge signer and the commit-keyed draw, and the ruling's design
  replaces this proposal or cites it as superseded.
cites: []
---

# Accepting a release closure from a sibling or the forge without a source rebuild

Design note, no code. Base: dev 9f272d343. For integrator shem, alongside Table 1
(`genesis/docs/superpowers/specs/evidence/2026-11-24-gate-attestation/attestation-properties-table.md`).

## The question

A device needs the release closure (the `epr` and `bin-launch` binaries) for a commit. Today it
either builds it from source (the eprfs gate, then `eprfs-produce.sh`'s `epr artifact put`), or it
draws a release a sibling built. What would let it accept bytes built elsewhere, from a sibling
or from the forge, with the same confidence as its own build, and what would an attestation have
to carry?

## What the store checks today

There are two acceptance paths, and they trust different things.

**1. Mesh binaries: keyed fetch (`epr artifact fetch`).** This is the stronger of the two.
- **The key is recomputed here.** `clean_key` derives the method and input CIDs from this
  checkout's clean source (epr-cli `artifact.rs` fetch, ~:1774-1800). A sibling's claim is only
  ever looked up by that key; the key is never taken from the sibling.
- **Holders are admitted siblings' custody records.** `resolve_holders` (eprfs-peer
  `fetch.rs:1479`) picks an output for the key: the expected output if pinned, else this
  device's own, else a strict plurality of distinct remote devices. A tie is a refused
  mismatch. This device's own custody cannot supply a fetch.
- **The bytes are verified.** The manifest must be the one the holder's crossed record names,
  and each chunk is verified by CID and length before it is stored (fetch.rs header :7-24).
- **Divergence is recorded.** Two outputs for one key leave a divergence Cite
  (`custody.rs:436-458`).
- **The count is advice only.** `epr artifact holders --min` counts distinct devices
  (`custody.rs:349-370`), but nothing requires more than one.

**2. Release closures: sibling draw (`epr bin`, `bin.rs` `admissible` ~:966 and `take`
~:1051).**
- **Offers are keyed by commit, not by a recomputed key.** An offer is (sibling device, release
  CID, per-binary outputs, requirement statements), found from admitted siblings' Produce rows
  whose commit equals the commit asked for (`bin.rs` ~:671-690). The receiver does not recompute
  a derivation key for the release: "built from this commit" is the sibling's word.
- **Requirements are judged before any byte moves.** Each binary's requirement statement
  (`env:requires` CID plus the document inline) is verified against its CID and judged against
  this host. A statement with an unmeasured axis (`env:gaps`) or no ISA-C marker is refused.
- **The bytes must make the release named.** The drawn bytes must hash to the release CID the
  offer named (`sibling-release-mismatch`). Requirements are re-extracted from the staged bytes
  and judged again.
- **The binary is exercised.** The staged `epr` runs its own exercise (bundled C on this CPU)
  before ingest.
- **The release is marked as drawn.** Ingest tags it `gate_run: sibling:<device>`. Landing or
  pinning such a release re-runs the exercise (`judge_held`, ~:389-419).
- **What is not checked:** that the release came from that commit's closure, that the
  producer's gate was green for that closure, or that any second device produced the same bytes.

**3. The forge.** I found no path by which the store accepts a forge-published release with any
attestation. Jenkins verdicts reach the device only as an unsigned ExternalClaim, with no
toolchain or executor facts (Table 1 row 4).

## The gap, in one line

Path 2 accepts "an admitted sibling says these bytes are commit C's release, and they run here".
It does not establish "these bytes are what C's closure builds". Path 1 establishes the key, but
trusts one admitted holder's word about the output.

## What acceptance without a rebuild would require

The proposal is to accept bytes B as the release of commit C when all of the following hold.

1. **The key is recomputed locally.** This device derives the release's derivation key K from
   C's clean closure (method plus input CIDs, as `epr artifact key` does for mesh binaries), and
   the offer is looked up by K, not by C. This moves path 2 onto path 1's footing.
2. **Producer records are signed and bound to K.** Each claim is a Produce record signed by the
   producer's device key and naming K, the output CID O and the method descriptor in readable
   form: toolchain, target, features, flags, requires CID, isa-c. Table 1 shows the artifact
   record already carries all of this, except a schema version on the method descriptor.
3. **Standing is checked at read time.** Each producer is an admitted device of this steward
   under the current roster (`admits_device`), and Jenkins is admitted the same way if it is to
   count (see below).
4. **The count is computed by the reader, never read from the record.** Accept O when at least N
   distinct admitted producers hold O for K and no admitted producer holds a different output for
   K. A divergence means refuse and record, never take a plurality. The steward sets N. N = 1
   means "trust any one of my devices"; N = 2 means "reproduced".
5. **The bytes are verified.** BlobCid(B) = O, chunk by chunk, as fetch already does.
6. **The local requirement and exercise judges still pass.** They are unchanged from path 2:
   they test this host, which no remote attestation can.
7. **The acceptance is recorded.** This device writes its own record: K, O, the producers
   counted, N, and the rule. It is a separate record that refers to the producers' records by
   CID. A later producer adds to the count without re-signing anything (the enrichment Table 1
   found missing). A device that later builds K itself and gets O becomes one more producer.

**The forge.** A forge-built release could count only if its producer is admitted like a device.
One option is a CI signing key enrolled as a device of the steward with a narrow grant. Another
is the harvesting device witnessing "the forge's bytes hash to O" — but that adds no independent
build: it is the same execution seen twice. Until one of these exists, forge bytes can be fetched
and verified against O, but only a device that holds K's output itself (N counted on devices) can
vouch for them.

## What an attestation would have to carry

| Field | Today (Table 1, artifact row) | Needed |
|---|---|---|
| Derivation key K (method + input) | present (`env:key`, `env:method`, `env:input`) | the receiver recomputes it and never trusts it |
| Output CID O and size | present (`env:output`, `env:bytes`) | same |
| Readable toolchain, target, features, flags | present in the method descriptor | add a schema version to `MethodDescriptor` |
| Requires CID and document, isa-c, gaps | present | same |
| Producer device and signature | present (brit agentId, custodian DID) | same; standing checked at read time |
| Gate verdict of the producer's run for K's closure | absent: the Produce does not name the gate attestation | the CID of the producer's green gate attestation for the same commit, so "it was tested" travels with "it was built" |
| Independent-producer count | absent as a fact (advice only) | derived by the reader per item 4, never carried |
| Acceptance record | absent | per item 7 |

## Decisions for the steward

1. **N for release closures.** 1 (trust a device) or 2 (reproduced). The second implies
   reproducible builds, which the divergence record would show quickly.
2. **Whether a forge build can count,** and under which signer.
3. **Whether path 2 should be retired for path 1's keyed lookup,** so that "built from commit C"
   is always a recomputed key, never a claim.

What this would cost: a keyed release lookup in `bin.rs` reusing `resolve_holders`, a
schema-version field on the method descriptor, an acceptance record kind, and one test per
refusal (key mismatch, divergence, unadmitted producer, too few producers). The host judges in
`admissible` and `take` stay as they are.
