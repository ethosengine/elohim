---
title: "Table 1: what each attestation record carries today"
id: gate-attestation-evidence-attestation-properties
status: draft
class: process-meta
date: 2026-11-24
context-tier: disclosed
steward: human:matthew
author: "Claude (session d6cfb7e1, Opus 5.5, ethosengine device), read-only evidence for integrator shem; Table 1 cells gathered by a read-only search agent and spot-checked by hand"
serves: [push-gate-pays-once]
tags: [attestation, gate-verdict, artifact, household, jenkins, evidence]
graduation-trigger: >-
  An attestation design records readable toolchain, platform, independent-execution count and an enrichment path, and this table is re-read against it.
cites: []
---

# Table 1: what each attestation record carries today

Read-only, dev d74bc6707 (worktree roll-2b-v3). Evidence gathered by a read-only search agent;
cells marked (checked) were re-read by hand before sending. Abbreviations: GE gate-evidence.mjs,
BC brit-build-ref build_cmd.rs, AR epr-cli artifact.rs, DS artifact/describe.rs, CU artifact/custody.rs,
PR epr-cli peer.rs, HA a2o household-attestation.ts, SR serving-receipt.mjs, TR/OB jenkins-bridge
translate.rs/observe.rs, BA scripts/brit-attest.mjs.

| | closure/inputs | toolchain | platform/features | signer + standing | independent count | method/judge version | enrich later | where |
|---|---|---|---|---|---|---|---|---|
| 1 Gate verdict | hash only: material GE:367-388, note keeps `inputs_hash` (GE:812, checked) | hashed rustc -vV (GE:274-283); not readable from the note | rustflags via `run` (GE:372), host triple only inside the rustc hash; `hardware_profile` empty, no `--hardware` passed (GE:806-816, checked); no features | brit workspace key; local reuse needs agentId == self (GE:601), no roster check; sibling verdict roster-checked at read (PR:1790) but rides an unsigned flow note (GE:879-884) | absent: newest row or first sibling (GE:598,618) | manifestCid signed (GE:810), epoch 2; recipe/runner/v:3 only inside the hash | none: a re-put replaces the note (GE:801-804); gate-verdict@1 points back by CID | notes/brit/build/gate:<p>; forge devices/<did>/build; crosses as gate-verdict@1 |
| 2 Artifact (epr artifact put) | key = method + input CID (AR:977-1025), output BlobCid (AR:1469) | rustc -vV, cargo, toolchain (DS:68-91), enforced at put | target, features, profile, rustflags, CFLAGS (DS:20-99), isa-c (AR:1577) | brit agentId + custodian DID (AR:1471-1480); siblings roster-checked at read (PR:1967-1976), local not | advisory: `holders` counts devices per output, --min 2 (CU:349-370) | method CID, family (AR:1560-1564); no MethodDescriptor schema version | separate records by key CID (divergence Cite CU:436-458, other Produce); no enrichment verb | payload store + flow sidecar; notes/brit/build/mesh-build:<p>; crosses as artifact-produce@1 only with --cross |
| 3 Household ValidationAttestation | sutParts per component (sut.ts:82-97), artifactCid = their hash (HA:167-240); no commit field | absent | absent | workspace key only; reader admits only its own key (HA:1241-1245); no roster | absent (minStations counts scenarios, SR:105) | validatorVersion and schema recorded; reader checks schema, never validatorVersion (checked) | none: findingsCid never set; a changed summary replaces the note | notes/brit/validate/a2o-household/<concern>/<sut>, local only, never published |
| 4 Jenkins (A bridge ExternalClaim, B brit-attest ci note) | A: commit slot only with the orchestrator graph (TR:370-415); B: inputs-hash = commit tree (BA:254) | absent | absent for the executor; B records the harvesting host (BA:316) | A unsigned, `service:jenkins`, offer approved by a distinct Steward (OB:173-251); B device brit key | absent (one Process per build) | A: recipe id@version (TR:460); B: manifest CID | A: unsigned, a new record breaks nothing; B: changed verdict replaces note | A: flows.jsonl under JENKINS_BRIDGE_ROOT, no crossing; B: notes/brit/build/ci:<pipeline> |

## Gaps that matter for "how much re-execution does a lander owe"

1. Gate verdicts carry toolchain and platform only inside a one-way hash; the signed note has no readable rustc, triple or rustflags, and hardware_profile is empty.
2. Nothing counts independent executions where it decides anything; only `epr artifact holders` counts, as advice.
3. A sibling gate verdict reaches reuse through an unsigned flow note; its trust is the gossip roster admission, not the brit signature.
4. The household reader never checks validatorVersion, records no toolchain or platform, and admits only its own key (no sibling household receipt can count).
5. Jenkins observations are unsigned and say nothing of the executor's toolchain or platform.
6. No record kind has an enrichment primitive: a changed verdict overwrites; side records refer back by CID by convention only.
7. (checked) device-notes-publish pushes every local refs/notes/brit/build/* (refspec, line 65), so the harvested ci:* notes go to the forge although brit-attest.mjs says they stay on the device. A decision for the steward: publish them deliberately or filter them.
8. With no rustflags in a manifest, the gate inherits ambient RUSTFLAGS and that value is not in its key.
