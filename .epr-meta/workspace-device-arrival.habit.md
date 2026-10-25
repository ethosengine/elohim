---
epr-habit-version: 1
id: workspace-device-arrival
invariant: >
  A workspace that arrives on a new machine becomes one of its human's devices through
  declared commands: the native tool is reachable, the device key exists on the persistent
  home, the human's roster knows the device (bound through the human's other device, no key
  crossing), the pinned embedding model resolves from a device-owned place, the semantic fold
  is attested within its freshness limit, and the workspace is moored on the machine's berth.
  One native read (`epr device status`, also `just status device`) prints eight conditions;
  model, fold and berth enhancements may be degraded. Every REFUSED line names the
  command that cures it. Nothing about arrival depends on a visitor's cache or on another
  workspace's keystore.
status: green
active: false
checks:
  - "just status device — exit 0 with eight ok/degraded lines on the live device; native checks: elohim/eprfs/epr-cli/src/device/arrival.rs and tests/device_arrival.rs"
  - "a2o @concern:workspace-device-arrival (genesis/a2o/features/devflow/workspace-device-arrival.feature — a keyless workspace is refused by name and nothing else blocks; a second device joins through the first with no key crossing; a missing model names its provisioner; @act:host @requires:epr-cli; all five scenarios are wired, including local candidates with degraded enhancements)"
  - "epr flow report --headline — the `index:` slot reads within the fold-lag ceiling on this device (index-fold-lag-ceiling@1; lag reached its limit once on 2026-10-16), and `participant:` names a standing human by the tracked roster"
refs:
  - "spec: elohim/lvi/docs/specs/2026-10-10-workspace-arrival-as-declared-device.md (the mapping of every init surface onto DevspaceSeed / Berth / device key / model pin; the three enrollment layers; the record-without-witness seam §6.3)"
  - "the devfile's arrival commands: devfile.base.yaml `setup-embed-model` + `setup-device-memory` (postStart); the provisioners genesis/agentic/bin/{embed-model-provision,device-memory-sync}"
  - "the ceremony: elohim/eprfs/epr-cli/src/actor.rs `device enroll | authorize | bind` (ruling R-P8); the key: elohim/eprfs/epr-cli/src/device_key.rs"
  - "siblings: recall-reaches-authority (the fold half); acts-attributed-to-participants (what standing is for); the 2026-08-30 stewarded-device spec (the same device on the dataplane, station 3 superseded by Mishpat device enrollment)"
retire-when: >
  when lvi's DevspaceSeed and the tevah Berth carry the device key, the roster binding, the
  model pin and the carrying capacity natively, so that arrival is the seed's own COLD→WARM
  lifecycle and `just status device` reads one berth instead of separate stores.
---
DELTA 2026-10-25b (GREEN held, a landing with no hand step): the push of 16af7710f (fc477d91c..16af7710f: pre-push delivers a receipt-attested eprfs build; berth leases from linked worktrees; strict envelope labels) passed every gate (ALL CLEAR 464 s, eprfs on its receipt). Pre-push's own land was refused `unpublished`, correctly: the commit is not on dev until the push completes. Session arrival's `follow` then landed the eprfs gate's ingested candidate bafkreie45mgd3gy2b5klx2qldsmm2xtrho52fridbx32wpwierr6wnsfvu at 13:23:59Z, with no rebuild, and a fresh session's `just status device` reads `epr binary: bafkreie45… vs head bafkreie45… (session-pin)`. Every earlier landing was by hand. Open: a release carries `epr` only, so arrival's install-launcher reinstalls the existing launcher; `bin-launch` stays at the 12:00 build until slice M declares it in the tool's bins.

DELTA 2026-10-25 (RED → GREEN, evidence): slice A landed on dev at 8bc6f0d0e (pre-push ALL CLEAR, 1943 s). The device store was already CIDv1 and the native `bin-launch` already installed; its head was a pre-rebase, never-published build (d5890c062), landed during development before landing required publication. `epr bin follow` correctly refused it as a non-descendant. The pre-push build of 8bc6f0d0e was ingested as bafkreigoah43… and landed with an explicit `--revert` (publication still checked), and the launcher was reinstalled from the same build. A fresh session's `just status device` then exited 0: ok epr binary (session pin = head bafkreigoah43…), device key, roster (human:matthew), embed model, repository trust and peer daemon; degraded brit (no head — `epr bin bootstrap --tool brit`), fold lag and berth (no mooring for the synthetic session). A session started before the landing keeps its older pin, which lacks `device status --root`, until it restarts. Pre-push did not land the head itself: a worktree push runs the main checkout's hook, which was 135 commits behind dev.

DELTA 2026-10-24 (RED, measured A.2): arrival is one native status/arrive surface (dup-census 3→1); 1,132 epr-cli tests, 74 device-runtime tests, 89 berth tests and 5/5 arrival scenarios pass; fmt/clippy/rust-style and 2,228 package assertions pass. Live candidate status exits 1: legacy sha256 current/epr-launch shadows the native launcher and brit has no executable candidate/head (controller review then classed a missing brit as degraded, not refused — only the device's own epr is essential, ground rule 2 — so the remaining refusal is the legacy shadow, which the A.4 pre-native arrival and the post-landing migration cure); key, roster, model and repository trust pass, fold/berth degrade. No live head was promoted from this dirty worktree. Blind reads: round 1 REVISE (1 correctness, 1 interpretability, 0 preference), both resolved; round 2 READY (0 correctness, 0 interpretability, 1 preference). Evidence: genesis/a2o/reports/recovery/device-runtime-a2/.

DELTA 2026-10-16 (GREEN re-measured, no status change): at origin/dev@42941449e, `just status device` at 19:58Z printed five ok lines and `REFUSED fold attested: degraded, 25 files behind (limit 25) — run epr flow memory index fold --max-files 1`; `epr flow memory index status --json --root /projects/elohim` showed `state: degraded, retried 18, attested_by (unclaimed)`. Cure: `epr flow memory index fold --max-files 1` folded 1 file but remained 25 behind, then `epr flow memory index fold --max-files 40` exited 0; `just status device` re-measured at 20:12Z printed all six ok lines, including `ok fold attested: complete, 0 files behind (limit 25)`, exit 0.

DELTA 2026-10-12 (GREEN re-measured, no status change): `just status device` on this workspace at 19:50Z printed six `ok` lines (epr binary, device key, roster bound to human:matthew, embed model, fold attested 0 files behind, berth moored) and exited 0.

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

DELTA 2026-10-10b (RED → GREEN, evidence): on shem `just status device` exit 0 with six `ok` lines
(epr binary · device key · roster bound "this device stands for human:matthew (roster)" · embed model
from `$XDG_CACHE_HOME/elohim/embed-models/all-MiniLM-L6-v2/onnx` · fold attested "complete, 11 files
behind (limit 25)" · berth moored). The roster binding landed in commit 248f21dd6: the R-P8 ceremony
run across two real devices for the first time — `enroll` on shem, `authorize` on ethosengine (a
first attempt on shem itself was refused: a device cannot approve itself), `bind` on shem; one row
appended to the tracked `matthew.jsonl`, no key crossed. The catch-up fold (`--max-files 3000`, debug
`epr`) embedded 2,941 files / 37,859 chunks in about 1 h 22 min (5,276 chunks dropped past the
per-file cap) and attested complete; the session headline reads `index: 11 files behind the fold
within hard 25 ✅` and `participant: human:matthew (… standing; witnessed …)`. a2o
`--profile device-arrival`: 4 scenarios green, 1 @wip (the six-ok fixture). Gate `memory-ceremony`
green end to end on 894eaf58f. Standing frontier, not a status question: the three conditions read
on the device but not fixture-proven (the @wip scenario), and the record-without-witness seam named
in the lvi arrival spec §6.3.
