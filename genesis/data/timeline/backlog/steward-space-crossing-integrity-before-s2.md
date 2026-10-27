---
id: "backlog-steward-space-crossing-integrity-before-s2"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Steward-space provenance is trusted, not proven: three gaps that must close before a second principal joins"
slug: "steward-space-crossing-integrity-before-s2"
written: "2026-10-27"
author: "claude-opus-5-5"
status: "open"
priority: "high"
area: "eprfs-peer / epr-rea flow store / brit-build-ref"
domain: "D5"
relatedNodeIds:
  - "habit:devices-share-one-berth"
  - "habit:sprint-lands-each-closure-once"
tags: [steward-space, provenance, integrity, s2-entry, security]
---

# Steward-space provenance is trusted, not proven

Decision D5 of `genesis/docs/superpowers/specs/2026-10-27-steward-space-landing-plane-design.md`
lets a sibling device's gate verdict cross the steward space, with provenance taken from the admitted
device whose signed head carried it. The A8 decision read (2026-10-27) found three gaps. Inside
one steward's grant none of them escalates anything: each needs either an admitted device of the
same steward or a local writer acting against its own device, and either can already bypass a gate.
Once a second principal is admitted, each gap lets that principal forge provenance. **S2 does not
start until all three are closed.**

1. **A forged `via:` slot reads as folded.** `via_device` only parses a string slot
   (`elohim/eprfs/eprfs-peer/src/allow.rs:132`), and the sidecar store accepts any record whose CID
   matches its bytes (`elohim/epr-rea/src/store.rs:258-276`). A local writer can append
   `via:did:key:B` to its own `flows.jsonl`, and the lease view (`lease_view.rs:135`) and any
   verdict projection would then read the record as folded from B.
   - Cure: a sealed crossing receipt per folded event (original CID, folded CID, source DID, signed
     head, inclusion proof), resolved through a `CrossingProvenanceResolver`. A naked `via:` is
     display only.
2. **Leaves are not proven to sit under the signed root.** Each leaf is checked against its own CID,
   but its membership under the root the head commits is not (`tier.rs:444`; `Diff` answers at
   `:486` get no integrity check). An admitted device can sign root R and serve a different,
   self-consistent tree.
   - Cure: proof-bearing list, read and diff responses, or a verified spine fetch. Do this together
     with A12 HeadV2.
3. **`brit-build-ref build find` does not fail closed.** At the pinned brit `76ba95d8`,
   `find_build_by_inputs` parses note JSON and matches on `inputsHash` alone, and `build find`
   returns the rows without verifying the signature against `agentId`.
   - Cure: deserialize the typed `BuildAttestationContentNode`, check the canonical signature and
     the stored CID relationship, and test forged `agentId`, altered `success` and bad signatures.

Probe for done: an a2o station under `genesis/a2o/features/devflow/` where a device outside the
admitted roster, and a local forged `via:`, each try to cause a lease effect or a sibling reuse, and
both are refused.
