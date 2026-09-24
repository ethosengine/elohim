---
epr-habit-version: 1
id: guards-sense-frames
invariant: >
  Every values-guard verdict names a content-addressed frame and carries a `FrameClassification`
  with evidence spans in the original bytes, so a reader can see which frame judged the write and
  exactly what it read. Verdicts are advisory: they never ask or deny until a human ratifies a
  floor. A keyword-silent semantic catch is an abstain, never an accusation.
status: green
active: false
checks:
  - "python3 .claude/scripts/_lib/__tests__/frame_classifier_parity_test.py (the native evaluator and the Python mirror agree on decision, rule_id, frameRef, verdict, spans and reason for a fixed probe write; every `frame-ref:` in policies.yaml equals the atom CID; MEASURED 2026-09-24: absent — `test -f` exits 1, the parity test does not exist yet; MEASURED 2026-09-24 after C10: `frame classifier parity: native == python ✅`, exit 0 — both guard rows (decision, rule id, class inherited from the row per R-C7, frameRef, verdict, spans, reason with the classification token aside), the declared-frame silence, and both frame-ref pins)"
  - "python3 -c \"import yaml,sys;p=yaml.safe_load(open('.claude/epr-meta/policies.yaml'))['policies'];r=[x for x in p if x.get('id') in ('sovereignty-ontology-guard','ownership-ontology-guard') and x.get('version')==4];sys.exit(0 if len(r)==2 and all(x.get('frame') and x.get('frame-ref') for x in r) else 1)\" (both @4 guard rows declare `frame:` and `frame-ref:`; MEASURED 2026-09-24: exit 1 — no @4 row exists, and `grep -cE '^\\s+frame:'` and `grep -cE '^\\s+frame-ref:'` over policies.yaml both count 0; MEASURED 2026-09-24 after C6 (c0b6d9fc7): exit 0)"
  - "cargo test -p elohim-epr-cli --lib frames::tests::golden_fixture_classification_cid (elohim/eprfs — the golden classification CID of a fixed write under a fixed frame atom is pinned; MEASURED 2026-09-24: absent — no `frames` module in elohim/eprfs/epr-cli/src and no elohim/sdk/schemas/v1/frames/ directory; not run, since there is nothing to run; MEASURED 2026-09-24 after C4 (e13b306af): `cd elohim/eprfs && env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/lanec-target CARGO_BUILD_JOBS=1 cargo test -p elohim-epr-cli --lib frames::tests::golden_fixture_classification_cid` → `test result: ok. 1 passed; 0 failed`, EXIT=0)"
first_move: >
  Raise the registry byte cap in both hosts first (plan task C1): policies.yaml sits just under
  the 65,536-byte cap that both hosts enforce, and one more row is a total governance outage.
  Then the frame atoms under elohim/sdk/schemas/v1/frames/ (C2) and the opaque evidence channel
  through eprfs-meta (C3), then the native classifier `frames.rs` that rebinds both guards (C4)
  and its Python mirror (C5). The @4 rows keep `class: dispatch` with the deliberated sidecar.
retire-when: >
  when escalations resolve at a peer-native elohim endpoint with witnessed provenance
  (`ComputeSource::PeerNative`) under the human sortition floor — the frame is then judged by
  the protocol's own governed compute, not by a repository hook.
refs:
  - "genesis/docs/superpowers/specs/2026-07-15-sense-respond-governance-classifier-design.md — the classifier spec: frames, families, advisory posture"
  - "genesis/docs/superpowers/specs/2026-07-15-frame-witness-primitive-architecture-design.md — the frame-witness primitive this habit makes live on the guards"
  - "elohim/epr/src/witness.rs — `FrameClassification` with its content CID and `FrameEvidence` spans (the primitive exists; nothing produces it yet)"
  - "elohim/eprfs/epr-cli/src/repository_validators.rs — `sovereignty_guard` / `ownership_guard` with hand-rolled PHRASES consts, the guards this rebinds"
  - ".eprfs/status/gap-items/specs__2026-07-15-sense-respond-governance-classifier-design.json — the classifier spec's gap items, all OPEN at declaration"
---
2026-09-24: DECLARED red, `active: false` (the WIP fence holds two active habits). Declared by
task C0 of genesis/docs/superpowers/plans/2026-09-25-post-station-4-participant-classifier-search-attention-sprint.md
(Lane C, ruling R-C0). All three checks measured absent: no parity test, no @4 guard row carrying
`frame:`, no `frames` module or frame atoms. The primitive (`FrameClassification`) exists in
elohim-epr; the two live guards still judge with hand-rolled phrase lists.

DELTA 2026-09-24 (FLIPPED GREEN — checks (a), (b), (c) each green by its own command). Executed by agent:implementer@claude-opus-5-5. Lane C landed C0–C10: C0 a29e9c64f (declared), C1 de4f75b20 (registry cap 1 MiB, both hosts), C2 9f11253b1 (frame atoms; permille + reason_clause in e13b306af), C3 ef703bef5 (opaque evidence channel), C4 e13b306af (native `frames.rs`, guards rebound), C5 488f6b81c (Python mirror, phrase lists deleted), C6 c0b6d9fc7 (@4 rows with `frame:`/`frame-ref:`, parity vectors; canon-lift regenerated 3b9ceb52f), C7 b9e0943e2 (frame line on both surfaces), C8 67e7ae062 (ledger rows carry frame_ref + classification_cid), C9 25c4e47fa (native classification moved to a detached child per R-C8; Family-2 shadow probe, abstain-only, `frame-probe-abstain@1`), C10 this commit (the parity test that reads this habit). Evidence: (a) `frame classifier parity: native == python ✅` exit 0; (b) exit 0; (c) `frames::tests::golden_fixture_classification_cid ... ok`, EXIT=0. Open by name: the @3→@4 rebind of genesis/docs/content/elohim-protocol/.epr-meta is the operator's (R-C9), so live writes there are still judged under @3 (same validator and frame; the parity test asserts the class the firing row declares); the probe's 8 s deadline was measured short under load 22 (fold 0.9–3.1 s + open 4.3–5.9 s + search 2.3 s; end to end 11.3 s), and the canon doc stewardship-over-sovereignty.md scores 0.505 against the phrase query, over the 0.35 floor — calibration is owed before the probe's abstentions mean anything.
