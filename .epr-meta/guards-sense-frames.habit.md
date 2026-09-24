---
epr-habit-version: 1
id: guards-sense-frames
invariant: >
  Every values-guard verdict names a content-addressed frame and carries a `FrameClassification`
  with evidence spans in the original bytes, so a reader can see which frame judged the write and
  exactly what it read. Verdicts are advisory: they never ask or deny until a human ratifies a
  floor. A keyword-silent semantic catch is an abstain, never an accusation.
status: red
active: false
checks:
  - "python3 .claude/scripts/_lib/__tests__/frame_classifier_parity_test.py (the native evaluator and the Python mirror agree on decision, rule_id, frameRef, verdict, spans and reason for a fixed probe write; every `frame-ref:` in policies.yaml equals the atom CID; MEASURED 2026-09-24: absent — `test -f` exits 1, the parity test does not exist yet)"
  - "python3 -c \"import yaml,sys;p=yaml.safe_load(open('.claude/epr-meta/policies.yaml'))['policies'];r=[x for x in p if x.get('id') in ('sovereignty-ontology-guard','ownership-ontology-guard') and x.get('version')==4];sys.exit(0 if len(r)==2 and all(x.get('frame') and x.get('frame-ref') for x in r) else 1)\" (both @4 guard rows declare `frame:` and `frame-ref:`; MEASURED 2026-09-24: exit 1 — no @4 row exists, and `grep -cE '^\\s+frame:'` and `grep -cE '^\\s+frame-ref:'` over policies.yaml both count 0)"
  - "cargo test -p elohim-epr-cli --lib frames::tests::golden_fixture_classification_cid (elohim/eprfs — the golden classification CID of a fixed write under a fixed frame atom is pinned; MEASURED 2026-09-24: absent — no `frames` module in elohim/eprfs/epr-cli/src and no elohim/sdk/schemas/v1/frames/ directory; not run, since there is nothing to run)"
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
