# Workspace arrival as a declared device — how a workspace inits itself, today and under lvi

**Date:** 2026-10-10
**Status:** Draft — slice 1 landed the same day (`just status device`, the a2o story, the habit); the DevspaceSeed mapping is the target lvi M0/M1 read against
**Author:** Matthew Dowell + Claude Fable 5.1
**Serves:** habit `workspace-device-arrival` (born red 2026-10-10, root `.epr-meta/`); habit `recall-reaches-authority` (the fold half)
**Reads first:** `docs/specs/2026-07-20-elohim-native-devspace-design.md` (the `DevspaceSeed`), `genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md` (`RuntimeManifest` + `Berth`), `genesis/docs/superpowers/specs/2026-08-30-workspace-stewarded-device-peer-design.md` (the workspace as matthew's device on the dataplane), `genesis/docs/superpowers/specs/2026-09-12-memory-search-scale-three-seams-design.md` (the index never travels)

## 1. The sentence this spec exists to make true

> **shem is a stewarded device, authorized by matthew's imago dei on arrival.**

A Che workspace is not a place where a human's tools happen to run. It is one of the human's
devices: it holds its own key, it is bound to the human by the human's other devices and never by
copying anything, it folds its own index from what it holds, and it declares its own carrying
capacity. "Arrival" is the moment a fresh workspace becomes that — and today arrival is **not
declared anywhere**. It is spread across a devfile, eleven SessionStart hooks, a hand-run ceremony,
a pool policy that does not know which workspace it is in, and (until today) a visitor's cache.

This spec records how a workspace inits itself today (§2), maps every piece of that onto the
artifact that owns it in the target (§3: lvi `DevspaceSeed`, tevah `Berth`, the eprfs device key
and roster, the model pin), states the arrival ceremony as it stands at three layers that are not
yet one ceremony (§4), reads it through local-first and the p2p design gate (§5), restates the
memory tiers a device holds (§6), and defines slice 1 and what retires it (§7).

## 2. How a workspace inits itself today (read from the tree, 2026-10-10)

Four surfaces, none of which reads the others.

**2.1 The devfile — one parent, two thin children** (commit b86b8c29c on dev).
`devfile.base.yaml` owns env, endpoints, one PVC mounted at `/nix`, the `setup-*` commands and
`postStart`. A child states only what differs: `devfile.yaml` is ethosengine (30Gi, cpu 10,
220Gi), `devfile_x86_v2.yaml` is shem (40Gi, cpu 8, 320Gi, storageClass `shem-zfs`, node affinity
shem). The shem child already carries the rule at its lines 35–36: *a new workspace is a new
device: it has its own conductor key; never copy another workspace's keystore.* Children fetch the
parent from the dev branch at creation, so a base change reaches a workspace on its next restart.
`$HOME` is wiped on every restart; `/nix` and `/projects` persist. `XDG_CONFIG_HOME=/nix/xdg/config`
and `XDG_CACHE_HOME=/nix/xdg/cache` are therefore the only durable per-device homes.

**2.2 Eleven SessionStart hooks** (`.claude/settings.json`), in order: durability-guard
(session state on the PVC), load-project-context (the headline, cached; the recall bootstrap),
participant-standing (names the human this device stands for; reads, never witnesses),
clean-caches, pool-preflight (the cargo slot), ram-guard (also io-guard and `berth moor`),
ci-harvest, runtime-harvest, delivery-gate, epr-evaluator-guard (restores the `epr` binary
because `$CARGO_HOME/bin` is wiped), run-projection. Each is one arrival condition in disguise.

**2.3 `genesis/agentic/pool-policy.json`** carries this workspace's carrying capacity as "one
tevah berth" (`berth`: mesh 1, cargo 2, disk-heavy 1) but has no notion of *which* workspace it
is in: `ram.fallback_max_gb 30` fits ethosengine, not shem; every other limit is global or read
live from the cgroup. Che injects `DEVWORKSPACE_ID`, `DEVWORKSPACE_POD_NAME` and
`/devworkspace-metadata/*.devworkspace.yaml`; nothing in the tree reads them.

**2.4 `hc-mesh.sh`** fixes the household (`matthew,jessica,james`, `household-dowell`) per
workspace and writes a tevah `manifest.json` + `berth.json` per mesh peer in ark mode — the only
place a `Berth` is written today, and it is written for the conductor peers, never for the
workspace that hosts them.

**2.5 What arrival looked like on shem before today.** The native semantic fold read
`attested failed: no model directory resolves` and `2942 files behind`, because the pinned
MiniLM bytes existed on ethosengine only as a side effect of MemPalace mining into `~/.cache`,
which a restart wipes. The native index had been borrowing a visitor's cache. shem had no device
key; `epr actor current --device` read `standing: null`; the participant roster's genesis row
was signed on ethosengine. The 269 shared contributions read as *unattributed* here (§6.3).

## 3. The mapping — each surface, and the artifact that owns it in the target

| Today (surface) | Target owner | Field | Status |
|---|---|---|---|
| devfile child: image, memory, cpu, PVC size, placement | tevah `Berth` (per blade) + `RuntimeManifest.envelope` | quota tier, data root, ports | ◐ written only for mesh peers (`hc-mesh.sh` ark mode) |
| devfile base: PVC at `/nix`, `XDG_*` under it | lvi `DevspaceSeed.runtime.persistent` | the DECLARED, verified-on-reap mutable set | ⚠ undeclared; today it is "whatever is under /nix" |
| devfile `env:` block | `RuntimeManifest.processes[].env` templates resolved against the berth | `CLAUDE_CONFIG_DIR`, `CARGO_*`, `MEMPALACE_*`, `HF_*` | ⚠ a flat env block, no provenance |
| `DEVWORKSPACE_ID` (set, unread) | the berth's declared identity → the device key's did | one key per OS user per device, on the PVC | ✅ key (`device_key.rs`); ⚠ nothing joins it to Che's id |
| eleven SessionStart hooks | `DevspaceSeed.runtime.health` + the readiness ladder of `RuntimeManifest.processes` | one condition each | ◐ `just status device` reads six of them in one place |
| `pool-policy.json` `berth` block | tevah `Berth.effective_quota` | per-blade, derived from the shared manifest | ⚠ one file for every workspace |
| `hc-mesh.sh` household defaults | `RuntimeManifest.processes` (the conductor/storage/doorway trio) | argv/env templates | ◐ ark mode writes it per peer |
| `setup-mempalace` (devfile, off) | §4 of the three-seams spec: the visitor | a declared per-device choice | ✅ mined on shem 2026-10-10 as a visitor |
| `setup-embed-model` (devfile, new) + `genesis/agentic/bin/embed-model-provision` | the model manifest's `resolve` list (`$XDG_CACHE_HOME/elohim/embed-models/…`, second entry since 2026-10-10) | `ModelPin` bytes on the PVC, CID-checked by the embed procedure | ✅ |
| `participant-standing.py` + `epr actor device enroll\|authorize\|bind` | the eprfs roster row (today) → Mishpat joining record (dataplane) → one-command device join (app) | §4 | ◐ three layers, not one ceremony |

The disambiguator from the seam atlas applies: the **devfile is the Berth's human-written
precursor**, not the Seed. What a workspace *is* (its toolchain closure, its entrypoint, its
declared mutable set) is the `DevspaceSeed`; what it *runs on* (quota, ports, data root, the
device key) is the `Berth`; and the per-blade facts the devfile children carry today are exactly
the facts a `Berth` holds. lvi M0/M1 should read this table before minting any field.

## 4. The arrival ceremony as it stands — three layers, not yet one

**4.1 The developer layer (eprfs, ruling R-P8) — landed today.** On the new device:
`epr actor device enroll --handle matthew` mints the device key at
`$XDG_CONFIG_HOME/elohim/device/ed25519.seed` and prints a request (the device's `did:key` and a
nonce; public material). On a device already in matthew's roster: `epr actor device authorize
'<request>'` prints an authorization signed by that device. Back on the new device: `epr actor
device bind '<authorization>'` countersigns and appends ONE row to the tracked roster
`.eprfs/status/participants/matthew.jsonl`. Commit and push carry the row; the request and the
authorization are carried by hand. **No key crosses.** On 2026-10-10 shem ran `enroll`
(device `…y5N6bowj`); the authorize step is the operator's hand on ethosengine.

**4.2 The dataplane layer (Mishpat) — shipped, run once with device = operator.**
`elohim/holochain/dna/mishpat/zomes/mishpat/src/device_enrollment.rs`: every device a person has
joined speaks for them and any of them may approve the next; the walk back to the authority is
depth- and visit-bounded; revocation cascades; **no device enters the controller set**. Driven by
`genesis/a2o/scripts/household-device-ceremony.ts`, which on the household wrote
`device-ceremony/{authority,binding}.json` with the device agent and the operator agent the SAME
key — so a true second-device run has not happened. This supersedes the 2026-08-30 spec's station
3 (`bind_identity`, W joins `controllers`): W is a **joining record**, never a controller. The
transport half stays `AgentPeerBinding` (imagodei; archetypes `node|desktop|mobile|steward`, no
`workspace`), minted by elohim-storage on boot from `<storage_dir>/identity.key`; the conductor's
own key lives in its lair. A storage-peer keystore is the device's *transport* identity; it is
bound to the human by the joining record, never by copying.

**4.3 The app layer — one command and one yes.** `genesis/a2o/features/auth/device-provisioning-paths.feature`
("a second node of his own joins with one command and one yes") and
`device-consent-grant.feature` (a device asks, its person approves in a portal) tell the story
the human sees. The developer layer is the adult, text-mode instance of the same handshake.

**The gap, said plainly:** three ceremonies with the same shape (new device mints, an existing
device of the same human approves, the record stands alone and is verified by walking back) at
three layers, with no artifact that says "this workspace's eprfs device, its conductor's agent key
and its storage peer id are one device of matthew's". That artifact is a `Berth` field in the
target; today it is this paragraph.

## 5. Local-first and the design gate, applied to arrival

**Local-first, four lines.**
- *Alone, with the key:* every arrival condition but one is met on the device by itself (key,
  model, fold, berth, binary). The device is useful before anyone else is present.
- *Alone, the key held elsewhere:* a hosted (doorway) human's device still mints its own device
  key; the doorway can read and sign as them for the *content* plane but never holds this key.
- *Alone, no key:* `just status device` says `REFUSED device key` and names `enroll`; nothing
  else refuses, and no work is blocked (the attribution floor is honor-system, ruling R-P1).
- *Working version:* the one condition that needs another party (roster bound) adds standing; its
  absence costs attribution only. An arriving device never becomes a condition for what the
  other device already did.

**p2p design gate, per entity.**
| Entity | Class | Address / store | Note |
|---|---|---|---|
| device key | B private | `ed25519.seed` on the PVC, 0600; never a DHT entry; never leaves | a wiped home is a new device, bound again honestly |
| roster row | git-carried JSONL today; a Notarized joining record at the dataplane layer | two signatures, content-addressed row | public material only |
| model files | C derived | the manifest's `resolve` list; CID-checked each run | the pin is the authority, the bytes are re-fetchable |
| fold store | C derived | `.eprfs/status/index/<measure-cid>/pinned/fold.sqlite` | never synced (three-seams invariant 1) |
| berth | C derived, per blade | `$CLAUDE_CONFIG_DIR/berth/` | rebuilt from live processes |
| device-local memory entries | SelfScope, by gitignore | `.claude/memory/device/` + `.eprfs/status/memory/device-contributions/` | §6.2 |

## 6. Memory tiers on a device (the question that started this)

**6.1 The rule.** The content is synced; the index never is. Each device folds its own index
from what it holds, under the same `ModelPin` by CID so results are comparable. A ceremony that
wants another device's memory sends it a bounded request; that peer answers from its own fold
under its own reach check and keeps its own receipt; the root that merges is the requester. The
private chain — recall receipts under `.eprfs/status/recall/`, journeys, `AttentionTending` —
never enters a fold that can leave the device. There is no shared embedding store, by design: an
embedding of a private note is still about that note (standing cannot be laundered by inference),
and a synced index is stale at the receiver with no honest freshness claim.

**6.2 The SelfScope ring, first concrete instance (2026-10-10).** Claude Code's per-workspace
auto-memory (`$CLAUDE_CONFIG_DIR/projects/<slug>/memory/`, 15 entries on shem) is copied into
`.claude/memory/device/` (gitignored) and contributed into
`.eprfs/status/memory/device-contributions/` (gitignored). The semantic fold's surfaces
(`**/*.md`) index the entries as files, so local recall sees them; the tracked `MEMORY.md`
projection and the tracked contributions never do. Device-locality is enforced by the gitignore
today; contributions carry `reach: repository` by default and no narrower ring exists on the
record yet — that is station 5/6 work, and this is the row it starts from. The importer refuses
symlinked sources (`import.rs`: "symlink source paths are refused"), so the copy is a copy.

**6.3 The seam the split workspace exposed: the record travels, the act does not.** The 269
shared contributions under `.eprfs/status/memory/contributions/` are git-tracked and present on
shem, but the index projection counts all 269 as `unattributed` here, because `ContributionActs`
reads the *acts* from `.eprfs/status/flows.jsonl`, which is per device and untracked. A re-import
would mint acts for 33 changed entries and skip 233 as already contributed (dry run 2026-10-10),
so the shared index cannot be re-projected on a second device until either the act travels with
the record or a carried record counts as its own act. The install guard (projection hook
property 4, "would drop rows") is what keeps the tracked `MEMORY.md` intact meanwhile. This is a
dataplane fact wearing a memory-tooling costume: the eprfs contribution plane has a content sync
(git) and no witness sync. Named here; not cured in slice 1.

## 7. Slice 1 (landed 2026-10-10) and what retires this spec

Slice 1 is the arrival made *readable*, not yet *declared*:

- `just status device` → `genesis/agentic/bin/device-preflight`: six lines in the
  `just mesh preflight` idiom (epr binary · device key · roster bound · embed model · fold
  attested · berth moored), each `REFUSED` naming the command that cures it, exit 1 on any.
- `genesis/agentic/bin/embed-model-provision` + the devfile command `setup-embed-model` (in
  `postStart`): the pinned bytes land on the PVC idempotently; the manifest's second `resolve`
  entry is where every workspace reads them.
- `genesis/a2o/features/devflow/workspace-device-arrival.feature` (`@act:host @requires:epr-cli`)
  and the habit `workspace-device-arrival` (root `.epr-meta/`, born red).
- The eprfs ceremony run for real across two machines (shem ↔ ethosengine), the first time.

**Retire-when:** lvi's `DevspaceSeed` and the tevah `Berth` carry the device key, the roster
binding, the model pin and the carrying capacity natively, so that arrival is the seed's own
COLD→WARM lifecycle and `just status device` reads the berth instead of six scattered stores.
Until then this spec is the declaration the devfile does not make.

## 8. Not in this spec (named so nobody re-derives them)

- Reach at replication (three-seams station 6) and search between holons (station 7).
- Enrolling shem's *conductor* through Mishpat device enrollment as a true second device; an
  `AgentPeerBinding` `workspace` archetype.
- A per-workspace overlay for `pool-policy.json` (§3 names the row; the Berth is its home).
- Making a carried contribution count as its own act (§6.3), or syncing `flows.jsonl`.
