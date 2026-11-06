---
title: "Re-genesis 1 follow-on: ark brit_index and gate-evidence read the epoch-2 note format"
id: task-rg1-brit-note-readers-report
status: landed-on-branch
landed_commit: "d0a790265 on regenesis/brit-note-readers (local, unpushed; base 89d4a22ff)"
verified_by:
  - "Reviewer (agent:rust-architect@claude-opus-5-5): a REAL brit.validate@2 note minted by brit-build-ref built from brit c3b3d8af074e (fixtures/brit-validation-attestation-epoch2.json) verifies locally with brit off the path; `npx tsx --test genesis/a2o/scripts/__tests__/household-attestation.test.ts` 42 pass 0 fail EXIT=0; `node --test genesis/orchestrator/serving-receipt.test.mjs` 38 pass 0 fail EXIT=0. See the Reviewer section."
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

## Reviewer

Independent integrity review of `genesis/a2o/scripts/lib/household-attestation.ts` at `3b86b0ef7`, against brit `regenesis/signature-envelope` `c3b3d8af074e61a20b4a7e66d186fc4a359e4d6f` (worktree `rg1-brit`) and elohim-epr `elohim/epr/src/signature.rs`. Reviewer: agent:rust-architect@claude-opus-5-5. (Note: the body above repeats its first bullet list and Addendum heading twice; left as the author wrote it.)

**Verdict: the transcript was right. The verifier admitted three classes of note that brit refuses or holds; all three are fixed in this commit, and a real brit-minted note verifies.**

Transcript, byte for byte. The TS reproduces transcript **v1, the length-prefixed framing** (`u16be(len d) ‖ d ‖ u16be(2) ‖ u16be(0) ‖ u16be(len key) ‖ key ‖ dag-cbor(body)`), not the retired unframed text. TS `household-attestation.ts:918-927`; elohim-epr `signature.rs:223-243` (`signing_transcript`); brit `note.rs:139-146` (`EnvelopeNote::sign`) and `note.rs:175-183` (`verify` → `check_for`). Domain `brit:validate@2:` (`validation.rs` `NoteBody::DOMAIN`), epoch 2, scheme 0, canonical key `ed 01 ‖ raw32`. The body is the nine fields without `signature`, in DAG-CBOR length-then-bytewise key order, with CIDs as tag 42 over `00 ‖ cid` and both `Option` fields as `f6`. Two things prove agreement with real brit rather than with itself. First, the real note's signature verifies, and brit links elohim-epr **0.2.0 from the registry**, not the in-tree file, so this checks the published framing. Second, `encodeValidationBody(view)` is a byte substring of brit's own object-store DAG-CBOR for the note.

D7 key binding. brit's expected signer is `ed01 ‖ hex::decode(body.validatorId)` (`note.rs:175-177`), compared by `Multikey` equality. `Multikey` admits only a minimal varint (`signature.rs:105-136`), so the TS byte-equality check is equivalent. The envelope key is never the authority. The independent binding sits one level up: `admitAttestation` requires `validatorId === workspaceId`, taken from the workspace seed or brit's `signer`, and both serving-receipt consumers apply the same check (`serving-receipt.mjs:172-175`, `:319`).

Divergences found at `3b86b0ef7` (all fixed):

1. **Weak-key universal forgery admitted.** Node's `crypto.verify` (OpenSSL) accepts the identity key `01 00…00` with the signature `01 00…00 ‖ 00…00` for any message. brit uses dalek `verify_strict`, which refuses small-order keys (`signature.rs:349`, `signing.rs:247`). Before the fix, the TS verifier (`verifyValidationNode` and `verifyEnvelopeNode`, then at `:793-842`) returned true for a forged epoch-2 note naming `validatorId = 01 00…00`, so `readHouseholdAttestations` would list it as verified. Admission still refused it because of the workspace-key binding. Epoch 1 had the same hole. Fix: `verifyEd25519Strict` (`:875`) refuses a small-order A or R using the libsodium encoding list (`:851`). Node already refuses S ≥ L (probed).
2. **Unsigned riders admitted.** brit decodes `EnvelopeNote` and `ValidationBody` with `deny_unknown_fields` (`note.rs:101`, `validation.rs:70`). The TS flattened only known keys and ignored extras at both levels, so it admitted a stored note with arbitrary unsigned fields. It also admitted a stored note in the flat printed-view shape, which brit refuses because it has no `body`. Fix: `envelopeShape` (`:696`) gives an exact field set and brit's types, including `result` ∈ pass|fail|warn|skip and null only on `findingsCid`/`ttlSec` (`:826`). `localVerifiedRead` refuses a stored epoch-2 note that is not `{body,epoch,key,scheme,sig}` (`:1032`).
3. **Held/refused classification inverted.** brit HOLDS an epoch-2 note under an unknown scheme (`note.rs:172-173`). The TS refused it, so in local-verify mode the writer logged it as unverifiable and replaced a future-scheme note that it should have deferred. Conversely, the TS HELD a malformed envelope (`epoch 2 note lacks scheme, key or sig`) and an `epoch` that is null or not a u16, while brit refuses these as `does not decode` (`note.rs:300-317`). Fix: `noteEpochOf` (`:714`) now follows brit's dispatch exactly.
4. Loose `validatorId` hex. `Buffer.from(…,'hex')` truncates where `hex::decode` refuses. `validatorKey` (`:868`) now requires exactly 64 hex characters. This was only reachable by a key holder, so it is parity work, not a forgery path.

Residual, not fixed (no forgery path; each needs the signer's key): CID bytes are not structurally parsed, whereas brit's `Cid` decode refuses malformed bytes. JSON `2.0` and `2` are the same value once parsed in JS, while serde refuses `2.0` as an epoch. A non-canonical, non-small-order key encoding (y ≥ p) is not distinguished. The writer's held-defer branch still has no unit test.

**The vector.** `genesis/a2o/scripts/__tests__/fixtures/brit-validation-attestation-epoch2.json`, minted in a throwaway scratch repo with a throwaway key:
- Build: `RUSTUP_TOOLCHAIN=1.96.1 CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/brit-t4/dev CARGO_BUILD_JOBS=1 RUSTFLAGS="" cargo build --locked -p brit-build-ref` at brit `c3b3d8af074e`. Binary sha256 `602ad1e1…6de8`.
- Mint: `brit-build-ref --repo <scratch> validate put --step a2o-household --check push-delivers-within-budget/sha256:6bb1d9c851d7750f --artifact bafkreidlwhm4quoxouh6e43tgddwnskygdinfdb5tpidbyu4hwnqvzlyzq --result pass --summary <epoch-1 vector summary> --validator-version a2o/build-sprint-report@1`. Note CID `bafyreicaxbnwlo23udenkdcckaeznfbjyotzxuair4soqopcjv5bhv7uha`. Key id (validatorId) `ca8880aedfaa8dc95950bae136283f9c2c4ac2bce316775dc94d7d6287c37aea`.
- Captured: the raw stored note bytes (`git cat-file blob`, sha256 `1fd695bd…48a0`), the exact `validate get` stdout, and brit's object-store DAG-CBOR.
- Second note: `validate put` always writes `findingsCid`/`ttlSec` null, so a second note was signed by brit-epr `EnvelopeNote::sign` at the same commit, from a scratch crate. It carries a `bafkr4i` findings CID and `ttlSec = 34560000`.

Tests added in `household-attestation.test.ts`:
- The real note read from its stored bytes with `brit: absent` and a runner that throws if invoked, so verification is local. It is checked equal to the printed view, verified, its body bytes checked against brit's CBOR, and admitted.
- One body field tampered (`result: fail`), which is refused. A scheme-7 copy is held.
- Parity probes: riders at either level and a flat stored note are refused, the weak-key forgery is refused, and loose hex is refused.

Against `3b86b0ef7`: the real vector verifies, while the weak-key forgery verifies, riders verify, and scheme 7 reads as epoch 2. After the fix, only the real vector verifies.

```
npx tsx --test genesis/a2o/scripts/__tests__/household-attestation.test.ts   # tests 42, pass 42, fail 0
EXIT=0
node --test genesis/orchestrator/serving-receipt.test.mjs                     # tests 38, pass 38, fail 0
EXIT=0
```

`tsc --noEmit -p genesis/a2o`: no error in the touched files. The 27 errors it reports are all pre-existing, in `compute-runner` and others. Gospel surfaces: none depend on this reader's internals; the honesty-matrix impact is that `push-delivers-within-budget`'s household receipt now verifies exactly the notes brit does.
