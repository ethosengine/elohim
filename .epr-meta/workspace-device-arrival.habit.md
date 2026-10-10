---
epr-habit-version: 1
id: workspace-device-arrival
invariant: >
  A workspace that arrives on a new machine becomes one of its human's devices through
  declared commands: the native tool is reachable, the device key exists on the persistent
  home, the human's roster knows the device (bound through the human's other device, no key
  crossing), the pinned embedding model resolves from a device-owned place, the semantic fold
  is attested within its freshness limit, and the workspace is moored on the machine's berth.
  One read (`just status device`) prints every condition, and every REFUSED line names the
  command that cures it. Nothing about arrival depends on a visitor's cache or on another
  workspace's keystore.
status: red
active: false
checks:
  - "just status device — exit 0 with six `ok` lines on the device the habit is read from (genesis/agentic/bin/device-preflight; unit: python3 -m unittest genesis.agentic.device_preflight_test)"
  - "a2o @concern:workspace-device-arrival (genesis/a2o/features/devflow/workspace-device-arrival.feature — a keyless workspace is refused by name and nothing else blocks; a second device joins through the first with no key crossing; a missing model names its provisioner; @act:host @requires:epr-cli; the six-ok scenario is @wip pending a fixture-complete fold)"
  - "epr flow report --headline — the `index:` slot reads within the fold-lag ceiling on this device (index-fold-lag-ceiling@1), and `participant:` names a standing human by the tracked roster"
refs:
  - "spec: elohim/lvi/docs/specs/2026-10-10-workspace-arrival-as-declared-device.md (the mapping of every init surface onto DevspaceSeed / Berth / device key / model pin; the three enrollment layers; the record-without-witness seam §6.3)"
  - "the devfile's arrival commands: devfile.base.yaml `setup-embed-model` + `setup-device-memory` (postStart); the provisioners genesis/agentic/bin/{embed-model-provision,device-memory-sync}"
  - "the ceremony: elohim/eprfs/epr-cli/src/actor.rs `device enroll | authorize | bind` (ruling R-P8); the key: elohim/eprfs/epr-cli/src/device_key.rs"
  - "siblings: recall-reaches-authority (the fold half); acts-attributed-to-participants (what standing is for); the 2026-08-30 stewarded-device spec (the same device on the dataplane, station 3 superseded by Mishpat device enrollment)"
retire-when: >
  when lvi's DevspaceSeed and the tevah Berth carry the device key, the roster binding, the
  model pin and the carrying capacity natively, so that arrival is the seed's own COLD→WARM
  lifecycle and `just status device` reads one berth instead of six stores.
---
DELTA 2026-10-10 (BORN red). Read on shem, the second workspace, before any cure: `just status device`
printed 3 of 6 ok (epr binary, berth moored, device key after `enroll` minted it) and 3 REFUSED —
roster bound (enrollment pending: shem's request `did:key:z6Mkm5de…bowj` awaits `authorize` on
ethosengine, the device that holds matthew's genesis row), embed model (no resolve entry reached
model.onnx + tokenizer.json: the only path was MemPalace's chroma cache under a wiped `$HOME`), fold
attested (`failed: no model directory resolves`, 2942 files behind a ceiling of 25). Cures landed the
same day: the pinned bytes on the PVC at `$XDG_CACHE_HOME/elohim/embed-models/all-MiniLM-L6-v2/onnx`
(manifest resolve entry 2, CID-neutral — the fold store `bafyreigdsx…rn6e` was reused, not rebuilt;
first 1-file fold embedded 25 chunks), `embed-model-provision` + `device-memory-sync` in the devfile's
postStart, the catch-up fold spawned (`--max-files 3000`), the device-local memory ring (15 harness
entries under the gitignored `.claude/memory/device/`), the preflight, this habit, the a2o story (3 of
4 scenarios runnable on host; the six-ok scenario @wip). STILL RED: the roster binding waits on the
operator's hand on ethosengine, and the record-without-witness seam (269 shared contributions read
as unattributed here because their acts live in the untracked `flows.jsonl`) is named in the spec
§6.3, not cured. Flip to green needs: `just status device` exit 0 on shem AND the three host
scenarios green. Blind-reader loop
on the story (one fresh reader per round): round 1 READY (0 correctness / 5 interpretability / 2
preference), round 2 REVISE (2/3/1), round 3 REVISE (1/4/2), round 4 READY (1/4/2); 13 findings
resolved across rounds (vocabulary grounded, the "stands for" contradiction, the roster-bound cure and
the bind step named from the pending state, public-material proof, title scoped to what is proven).
Deferred by the author, named: vocabulary carried in the comment block (the a2o convention this tree
uses, e.g. agent-identity-claim-and-acceptance.feature); the three conditions read on a real workspace
but not fixture-proven (epr binary, fold attested, berth moored — the @wip scenario); "arrives" proven
only there.
