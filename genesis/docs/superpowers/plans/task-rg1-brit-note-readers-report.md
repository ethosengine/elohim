---
title: "Re-genesis 1 follow-on: ark brit_index and gate-evidence read the epoch-2 note format"
id: task-rg1-brit-note-readers-report
status: claimed-not-verified
landed_commit: "d0a790265 on regenesis/brit-note-readers (local, unpushed; base 89d4a22ff)"
verified_by:
  - "genesis/local-dev/rg1-note-readers/gate-ark.log: every ark step green (fmt, clippy, tests, rust-style PASS, doc-tests) but EXIT=1 from `[preflight] REFUSED submodules: not checked out: elohim/brit elohim/rakia sophia` (worktree has no submodules; --reference clone failed: parent is shallow)"
  - "genesis/local-dev/rg1-note-readers/gate-orchestrator.log: EXIT=1; 20 failures, all submodule-absence (pin/registry/selection-oracle/commit-hook-trailer tests); none in gate-evidence. `node --test genesis/orchestrator/gate-evidence.test.mjs` = 68 pass, 0 fail, EXIT=0"
actor: agent:rust-architect@claude-sonnet-5-5
cites: []
concerns:
  - "Both gate EXIT lines are 1, for the submodule preflight only. Re-run `just gate elohim-ark` and `just gate orchestrator` from a checkout with submodules to get EXIT=0; the touched code's own tests are green."
  - "Witness::Signature { sig: String } (eprfs-core) has one string field, so an epoch-2 row's witness is `brit@2/<scheme>/<hex key>/<hex sig>`; epoch 1 is the stored hex unchanged. Neither consumer verifies signatures (brit verifies; stdout is the verified read), so no transcript framing is implemented. A typed witness would be an eprfs-core change."
  - "Held semantics: a held note (brit's `held: <commit> <reason>` stderr line, a row whose epoch is not 1/2, or an epoch-2 row lacking scheme/key/sig) is reported once, dropped from rows, and is NOT counted unverifiable (it does not veto reuse; it is simply not evidence). brit-attest.mjs harvest defers rather than overwrite a held note; the gate's recordEvidence warns and writes anyway (its last-run-wins rule, as for unreadable)."
  - "Seam: `build get` prints the same flat view as `build find` rows, so readBuildNote's inputsHash/success reads work for @2 unchanged. An explicit epoch 1 is read as v1 here (brit refuses it earlier as Invalid). a2o TypeScript readers (genesis/a2o/scripts/lib/household-attestation.ts etc.) and the nested elohim/rakia/elohim/brit copy were not audited or changed."
  - "Brit fallbacks: `brit submodule` has only `list` (used git submodule update --init, which failed); `brit commit` refused 'refusing to ignore an untrusted core.hooksPath' (used git commit). prepare-commit-msg printed UNMEASURED for session model; trailers written by hand."
---

# Note readers for the epoch-2 envelope

- `elohim/ark/cli/src/brit_index.rs`: `Attestation` gains `epoch`/`scheme`/`key`/`sig`; `NoteEpoch {V1,V2,Held}` decided on `epoch` alone; `Found.held`; `parse_held`; `say_unverifiable` also voices held notes; `record_of` uses `witness_sig()`. Tests: `epoch_one_row_reads_unchanged`, `epoch_two_row_is_admitted_with_sig_key_and_scheme_carried`, `unknown_epoch_is_held_never_a_row`; pre-existing tests unchanged and green.
- `genesis/orchestrator/scripts/brit-attest.mjs`: `heldNotes`, `noteEpoch`, `noteSignature`, `readBuildNote` state `held`.
- `genesis/orchestrator/gate-evidence.mjs`: `findEvidence` returns `{rows, unverifiable, held}`.
- Tests in `gate-evidence.test.mjs`: epoch-1 unchanged, epoch-2 admitted, unknown epoch held.

## Addendum: a2o TypeScript readers of validation attestations

Files: `genesis/a2o/scripts/lib/household-attestation.ts` is the one reader; `genesis/orchestrator/scripts/serving-receipt.mjs` imports `readHouseholdAttestations` and `admitAttestation` from it and has no note parsing of its own.

- Unlike the ark and gate-evidence readers, this one DOES verify signatures (`verifyValidationNode`, used by the local read and by `admitAttestation`). Epoch 2 now verifies ed25519 over elohim-epr `signing_transcript` v1: `u16be(len d) | d | u16be(2) | u16be(scheme) | u16be(len key) | key | dag-cbor(body)` with `d = brit:validate@2:`, scheme 0 only, `key` must equal `ed 01 || validatorId`, body = the node's nine fields without `signature` (`encodeValidationBody`). Framing taken from `elohim/epr/src/signature.rs` and brit `validation.rs`. The TS test signs with the same framing, so it is a self-consistency check, not a cross-implementation vector: no real brit-0.1.13-minted note exists yet. Mint one from brit-cli 0.1.13 and add it beside `fixtures/brit-validation-attestation.json`.
- `noteEpochOf` and `flattenNote`: epoch absent or 1 is the frozen path unchanged; 2 flattens the stored `{body,epoch,key,scheme,sig}` to the view brit prints (body fields plus epoch/scheme/key/sig); any other epoch, or a broken envelope, is held. `britValidateGet` also treats a `held:` stderr line as held. Held is warned as `household attestation held: <ref> <reason>`, never a row, and `admitAttestation` refuses it as `held: ...`. `ValidationNodeJson.signature` is now optional.
- Writer (`publishHouseholdEvidence` -> `putAttestation` -> `brit-build-ref validate put`) calls the brit CLI. brit-cli up to 0.1.12 mints epoch-1 notes; 0.1.13 mints `@2` envelopes (T4: new notes are envelope-signed). The writer's read-before-write now defers, rather than overwrites, when the prior note is held.
- Tests: `genesis/a2o/scripts/__tests__/household-attestation.test.ts` (39 pass: epoch 1 unchanged, epoch 2 read/verified/tamper-refused, unknown epoch held, plus a verified-reads brit epoch-2 view with a `held:` line). `genesis/orchestrator/serving-receipt.test.mjs` (38 pass = prior 35 plus epoch 1, epoch 2, unknown epoch held). Both EXIT=0. The epoch-2 receipt test runs with brit off PATH, because the brit binary at hand predates `@2` and refuses the envelope (`unknown field body`): the brit pin and these readers must move together.
- Not done: no unit test for the writer's new held-defer branch.
