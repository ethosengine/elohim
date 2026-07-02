---
id: "backlog-hub-enablement-dial-readiness-2026-06-21"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Hub Enablement Readiness — the hubbiness dial, hardware tiering + graduation, and the non-technical-steward provisioning path"
slug: "hub-enablement-dial-readiness-2026-06-21"
written: "2026-06-21"
author: "workflow:hub-enablement-dial-readiness"
status: "backlog"
priority: "medium"
tags: [hub, enablement, hubbiness-dial, hardware-tiering, graduation, steward-provisioning, readiness-assessment, seam-3-12]
# OPEN concern: point-in-time readiness assessment (verdict NO — a non-technical steward cannot yet
# toggle a recycled laptop into a household hub). The gap list is the backlog. Routed out of
# .claude/data 2026-07-02 (machine-ledger law); referenced by the seam map (seam 3.12).
---

# Hub Enablement Readiness Assessment

**Scope:** the hub as a human-owned ROLE on recycled consumer hardware — the *hubbiness dial* (human runtime setting vs operator deploy-time), *hardware tiering + graduation with identity continuity*, and the *non-technical-steward provisioning path* ("plug in an old laptop, toggle it into a household hub").

**Companion to two sibling assessments:** peer-hoster async-sync dataplane; commons recursive aggregation. This one covers the *enablement* axis — getting a recycled device into the hub role at all — not the dataplane it would carry once enabled, nor the upward aggregation of what it stewards.

Synthesized from three verified briefs (hubbiness dial; tier graduation + identity continuity; steward provisioning UX), each adversarially re-checked against source.

---

## 1. THE DECISIVE VERDICT

**NO.** A non-technical household steward cannot plug in an old laptop and toggle it into being a household hub / peer-hoster today.

The single biggest blocker is **there is nothing for the steward to run, and no toggle to flip.** The always-on node binary that contains the "become a hub" setup wizard (`steward/node` / `elohim-node`) is **built by no pipeline and deployed by no manifest** — the container *named* `elohim-node` in production actually runs the `elohim-storage` binary (`_edgenode-consolidated.template.yaml:223` → `elohim/elohim-storage/Dockerfile:260 ENTRYPOINT ["elohim-storage"]`). The wizard, the auto-join orchestrator, and the hubbiness dial are all SPEC-ONLY, BUILT-UNCONSUMED, or ABSENT. The only working path to an always-on hub is **operator-grade**: hand-edit `genesis/orchestrator/data/deployments.json`, let Jenkins `sed`-render a k8s StatefulSet, deploy onto a cluster the steward cannot touch. The "dial" that does exist (`target_arc_factor`) is a deploy-baked operator JSON string whose runtime actuator is **explicitly documented as neutered** (`_edgenode-consolidated.template.yaml:99-105`: "The runtime arc_actuator can NOT change this at runtime").

There is one bright spot, on a *different axis*: agent-pub-key continuity from hosted→own-desktop is genuinely BUILT both ends (encrypted key bundle). But that is App-Steward graduation (a person onto their own laptop), not laptop→hub graduation, and it does not carry source-chain history.

---

## 2. READINESS TABLE

| # | Enablement capability | Status | Evidence (file:line / doc) | Gap |
|---|---|---|---|---|
| **A. The hubbiness dial** | | | | |
| A1 | Static archetype → capability-flags path (deploy-time hub-ness) | **LIVE-WIRED** | `deployments.json.deviceArchetype` → `elohim/holochain/Jenkinsfile:559,642` → `boot_registration.rs:147,114` → `genesis/data/devices/devices.json` → `stewarded_nodes` + `NodeRegistration` DHT entry (`node-registry/.../shape.rs:48`) | This is the *thing the vision contrasts against* — fixed at boot, operator-owned, redeploy to change |
| A2 | `target_arc_factor` intensity lever ("6pm laptop vs overnight") | **LIVE-WIRED but deploy-time / operator-owned** | `deployments.json edgenodeArcFactor:"0"` (jessica :73, james :104) → `Jenkinsfile:638 TARGET_ARC_FACTOR_PLACEHOLDER` → `arc_policy.rs`; runtime actuator disabled `template.yaml:99-105` | Only the `{0,1}` lever is deployed (fractional upstream-blocked in kitsune2); operator-edited, not a human gesture |
| A3 | Human-owned runtime hubbiness TOGGLE (the dial itself) | **SPEC-ONLY** | `2026-05-08-iroh-libp2p-complementarity.md:223` ("dial is owned by the human, declared via standing manifests, signed/witnessable/reversible"); §"out of scope... stub epics". Crate-wide grep for `hub_role`/`is_hub`/`becomeHub`/`hostMode` = zero code | No toggle, no per-increment human action |
| A4 | `record_capability` runtime capability mutation | **BUILT-UNCONSUMED** | `p2p_iroh/peer_map.rs:309`; sole caller is `#[cfg(test)]` at :663. Live inserts hardcode `capability_level:5` (:249,:298) | The one fn that could mutate capability at runtime has no production caller |
| A5 | "Standing manifest" entity (the dial's declared home) | **ABSENT** | grep `standing.?manifest`/`StandingManifest` across `.rs/.ts/.json` = zero (re-verified) | No entity to declare/sign/reverse against |
| A6 | Signed / witnessable / reversible at increments | **partial / ABSENT** | `sign_shape` (`boot_registration.rs:134`) = `Sha256::digest(...)`, a hash not an agent signature (STAGE1_SIGNATURE_SENTINEL); reversal = edit JSON + redeploy | No witness, no reverse mechanism |
| **B. Hardware tiering + graduation** | | | | |
| B1 | Real hardware probe (CPU/RAM/disk via `sysinfo`) | **BUILT-UNCONSUMED** | `steward/node/src/dashboard/metrics.rs:373-425` (dashboard-only); gossip announce un-wired `pod/capacity.rs:25-35` ("TODO: Wire into gossipsub"); registration reports `memory_bytes:0` (`registration.rs:269-271`) | Probe exists, nothing maps it to a tier/placement |
| B2 | `ResourceSnapshot` carrying disk/RAM/load (the spec's named signal source) | **ABSENT** | `elohim/elohim-compute/src/resources.rs:8-16` holds only request counters + managed-storage bytes; no `sysinfo` dep in `Cargo.toml` | The signal the tiered-quilt spec reads physically lacks capacity fields |
| B3 | Hardware-tier DETECTION (machine self-classifies) | **ABSENT** | no probe→tier path; `boot_registration.rs` reads env var + static fixture | Tiering is *declaration*, never *detection* |
| B4 | Declared node-shape registration (env archetype → fixture → DHT) | **LIVE-WIRED** (declaration, not detection) | `boot_registration.rs:108-124` (called `main.rs:566`); deployed value is `DEVICE_ARCHETYPE_PLACEHOLDER` | Capacity values come from hand-authored JSON, not the box |
| B5 | `TierController` / `HeuristicClassifier` / tier drivers | **SPEC-ONLY** | no `elohim-storage/src/tier/` dir; in-code refs are comments ("TierController deferred", `inventory_broadcaster.rs:23,101`); `2026-05-11-tiered-quilt-stewardship-design.md` §3 (Waves 0-7 Draft) | Entire controller hierarchy unbuilt |
| B6 | Device-archetype L0-5 gradient at runtime | **SPEC-ONLY** (fixtures are TEST data) | `genesis/plans/2026-04-13-device-archetypes-design.md`; `genesis/data/devices/*.md` ("Fixture Data for Peer Diversity Testing") | No runtime path assigns a level to a machine |
| B7 | `capability_level → Track3Bridge` hub routing | **BUILT-UNCONSUMED** (computed, dead at call site) | constructed `peer_map.rs:486`; consumer `http_blob_router.rs:154-155` matches only `Ok(TransportChoice::Iroh)` → Track3 collapses to libp2p-only; pattern-matched only in a `#[test]` (`peer_map.rs:875`) | The "consumer-grade peer routed through a hub" branch never fires in prod |
| B8 | Hosted→device agent PUB KEY continuity (key bundle) | **BUILT** both ends; Tauri reachability unproven | doorway `KeyExportFormat` → `steward/device/src-tauri/src/identity.rs:34` (Argon2id+ChaCha20) → `lib.rs:687-694` installs hApp with the SAME agent key | Different axis (own-desktop), and end-to-end live exercise unconfirmed |
| B9 | Source-chain / history transfer on graduation | **ABSENT** | `2026-06-11-dna-upgrade-governance.md` §6 ("export seam is a door with no road yet"); `export_for_migration` exists `lib.rs:8092` but no bridge call anywhere | Install with provided key genesis-seeds a fresh chain |
| B10 | Doorway graduation endpoints | **LIVE-WIRED but flag-accounting only** | `/admin/graduation/{pending,completed,force}` `http.rs:2745-2754`; `handle_force_graduation` flips MongoDB `is_steward` + deprovisions cell (`admin_conductors.rs:729-771`); doorway-service/CLAUDE.md "source-chain migration not yet built" | No key export, no chain migration — accounting only |
| B11 | Laptop → NUC/DwellingHub hub-role graduation preserving identity (the literal ask) | **SPEC-ONLY** | `2026-05-08-...md:173-200` ("hubs are a role, not a hardware tier"); `hardware-spec.md:53-64` | No code moves a hub role across hardware carrying identity |
| B12 | DNA-layer lineage / network-migration preserving key | **SPEC-ONLY / open problem** | `2026-06-11-dna-upgrade-governance.md` §3 (lineage field regressed since 2026-04-24, gated behind `unstable-migration`), §7 ("no mechanism exists for any of these") | Forcing reinstall mints a NEW key needing a migration that doesn't exist |
| **C. Steward provisioning UX** | | | | |
| C1 | Deployed always-on node provisioning (operator path) | **LIVE-WIRED** (operator-grade) | `deployments.json` $comment; `_edgenode-consolidated.template.yaml:222-223`; `Dockerfile:260`; driver `elohim/holochain/Jenkinsfile` | Irreducibly operator-grade; cluster ops are operator-owned (no kubectl from dev) |
| C2 | "Become a hub" setup wizard (`/api/setup/{join,doorway}`, pairing) | **BUILT-UNCONSUMED** (theatrical) | router mounted `steward/node/src/main.rs:272`; `setup.rs` internals all `// TODO`/hardcoded (`decode_join_key` returns `("operator-key","cluster-key","my-family")`); returns fake `node_id:"node-1"` | Validates join key by length only; binary deployed by nothing |
| C3 | Auto-join via doorway orchestrator (mDNS → register_node → NATS) | **BUILT-UNCONSUMED** (simulated + flag-disabled) | `node_bootstrap.rs:335-351` ("DNA registration simulated"); `orchestrator/mod.rs:136-150` in-memory `HashMap` only; gated by `orchestrator_enabled` (`main.rs:286`) set by no manifest | DNA write commented out; off in prod |
| C4 | `steward/node` `elohim-node` binary in production | **BUILT-UNCONSUMED** | CI gate-tests it (`steward/device/build-manifest.json:15-17,38`) + `Dockerfile` exists, but no pipeline publishes the image, no manifest deploys it; prod runs `elohim-storage` | Needs an image-publish stage + a manifest that pulls + runs the dashboard |
| C5 | Cluster join / pairing / leader election / mDNS scan | **BUILT-UNCONSUMED / partly ABSENT** | `cluster/{discovery,membership,leader}.rs` 3-line TODO stubs (`steward/node/CLAUDE.md:68`); `api_scan_network` TODO (`routes.rs:101`) | Discovery primitives stubbed |
| C6 | Identity handoff to a *separate always-on hub* | **ABSENT** | handoff exists only for own-desktop (`tauri-desktop` §Handoff); no node-provisioning-with-steward-identity mechanism | The two buckets are different; hub-join is the dead orchestrator path |
| C7 | Household-fabric onboarding | **ABSENT** | only hit is a doc-comment on `node_role`/`NODE_ROLE` (`elohim/elohim-storage/src/config.rs:128`) | No onboarding code |
| C8 | `elohim-operator` (per-hub specialist that "graduates with the hub") | **SPEC-ONLY** | `2026-05-08-...md` §"Every hub runs an elohim-operator"; node has only a `pod/` ops loop | Spec-only |

---

## 3. THE DELTA — from today's mechanism to the vision

**Today's mechanism (developer/operator flow):**
A static, deploy-time, k8s-shaped pipeline. A human's hub-ness is a fixed property of the `deviceArchetype` string an *operator hand-edits* in `genesis/orchestrator/data/deployments.json`. Jenkins `sed`-bakes it (plus `nodeRole`, `target_arc_factor`, resource budgets) into a StatefulSet template, deploying the `elohim-storage` container onto the operator's cluster. At boot, `boot_registration.rs` copies the archetype's `canSteward`/`canDoorway`/`canInfer`/`capability_level` out of a hand-authored fixture (`devices.json`) into a one-shot, hash-stamped (not agent-signed) `NodeRegistration` DHT entry. Changing any of it is a redeploy. The steward never touches any of this.

**The vision (human-owned runtime dial on recycled hardware):**
A person plugs in an old laptop, runs an installer that becomes an always-on daemon, and dials hubbiness up/down at the device at any increment ("just my laptop" ↔ "also a household hub"). Each increment is a signed, household-witnessable, reversible **standing manifest** entry. The device *measures its own hardware* and self-classifies its tier. The elohim-operator then picks up/drops Track-3 spoke and Track-2 federation commitments accordingly. Graduating to bigger hardware carries the identity (and ideally the history) with it.

**Concrete missing pieces between the two:**

1. **A steward-runnable artifact.** The `elohim-node` always-on binary builds via no pipeline and ships in no image. *Delta:* image-publish stage + a deploy/installer surface a non-developer can run (the recycled laptop currently has nothing to download that becomes a hub).
2. **A real toggle wired to a real entity.** No `StandingManifest` entity exists; `record_capability` (the one runtime capability mutator) has no production caller; `capability_level` is hardcoded to `5`. *Delta:* define the standing-manifest DHT entry; give `record_capability` a real caller driven by it; have transport routing read a human-set value, not a constant.
3. **Hardware DETECTION feeding tiering.** The `sysinfo` probe exists but is dashboard-only; `ResourceSnapshot` lacks disk/RAM/CPU/load fields entirely; no `TierController` consumes any of it. *Delta:* add capacity fields to `ResourceSnapshot`, gossip the probe, build the classifier→driver path the tiered-quilt spec specifies.
4. **Working discovery + auto-join.** mDNS scan is TODO; the doorway orchestrator's `register_node` is in-memory + DNA-simulated + flag-disabled. *Delta:* implement the real zome call, persist registration, enable the flag in a manifest.
5. **The Track3Bridge consumer.** The hub-bridge verdict is computed at `peer_map.rs:486` but `http_blob_router.rs:154` collapses it to libp2p-only. *Delta:* one call-site change to actually route consumer-grade peers through a capable hub.
6. **Signed/witnessable/reversible semantics.** `sign_shape` is a hash sentinel; there is no witness or reverse mechanism. *Delta:* real agent signatures + a household-witness + a reversal path.
7. **The dial's downstream effect.** The `elohim-operator` that "graduates with the hub" and picks up commitments is spec-only. *Delta:* the per-hub operator agent + a `delegates-compute` Mishpat authoring flow (the notarized primitive already validates but has no live create path — `rea_commitments.rs:92-95` "other actions still take the legacy diesel-direct path").

---

## 4. PRIORITIZED GAP WORK-LIST (leverage-ordered)

Built-unconsumed items are flagged as **cheap high-leverage** — the machinery exists; only wiring is missing.

1. **[code-now] ⚡ CHEAP-HIGH — Consume the Track3Bridge arm.** `http_blob_router.rs:154` matches only `Ok(TransportChoice::Iroh)`; the hub-bridge verdict is already computed at `peer_map.rs:486`. A single call-site change activates "consumer-grade peer routed through a capable hub." Highest leverage-per-byte: makes the *first* hub behavior real. (Caveat: the manifests it reads are unsigned/placeholder, so pair with #3.)

2. **[code-now] ⚡ CHEAP-HIGH — Give `record_capability` a production caller.** The runtime capability mutator (`peer_map.rs:309`) is tested and ready; every live insert hardcodes `capability_level:5`. Wiring even a config/CLI caller breaks the "consuming a constant" deadlock and is the seam a future toggle plugs into.

3. **[code-now] ⚡ CHEAP-HIGH — Add capacity fields to `ResourceSnapshot` + gossip the `sysinfo` probe.** The probe (`metrics.rs:373-425`) already reads CPU/RAM/disk; `ResourceSnapshot` (`resources.rs:8-16`) just lacks the fields and the broadcast (`pod/capacity.rs:25-35` TODO). This unblocks detection-based tiering (the spec's named signal source).

4. **[code-now] — Enable + complete the auto-join orchestrator.** `node_bootstrap.rs:335-351` DNA write is commented-out and `orchestrator_enabled` is set by no manifest. Implement the real `register_node` zome call, persist it, flip the flag. Medium-cheap; gates discovery.

5. **[needs-substrate] — Define the `StandingManifest` DHT entry + ride it on `delegates-compute`.** The dial's declared home is ABSENT; the notarized Mishpat primitive (`delegates-compute`) validates but has no live authoring path (`rea_commitments.rs:92-95`). This is the substrate home for a signed/reversible/witnessable hub declaration — the protocol-correct base the dial should sit on.

6. **[needs-substrate] — Replace `sign_shape` hash sentinel with real agent signatures + a household-witness/reverse path.** STAGE1_SIGNATURE_SENTINEL (`boot_registration.rs:134`) blocks the "signed/witnessable/reversible at every increment" requirement. Needs DHT-layer witness + revocation, not a hash.

7. **[needs-substrate] — Solve DNA-key lineage (the graduation prerequisite).** Lineage field regressed (`2026-06-11-dna-upgrade-governance.md` §3), migration import zero-wired (§6), no consensus migration flow (§7). Blocks hardware graduation that changes the DNA/network. Heaviest, but gates identity continuity across hardware. See §5.

8. **[design-first] — The `steward/node` deploy/installer surface.** Decide whether the always-on hub ships as a publishable container image (image-publish stage + manifest) or a non-developer installer for recycled hardware. Until decided, the wizard (`setup.rs`) and dashboard are unreachable regardless of how much wiring lands. This is the gate on a *steward-grade* answer to the verdict.

9. **[design-first] — The `elohim-operator` per-hub agent + the hubbiness-dial UX.** The agent that "graduates with the hub" and the steward-facing toggle UI are both spec-only. Design the role-manifest, witness-UX, and renegotiation flow the spec explicitly seeded as stub epics.

---

## 5. THE IDENTITY-CONTINUITY QUESTION

**Is graduating the hub across hardware solved? — Split answer: one axis is BUILT, the asked axis is an OPEN LINEAGE PROBLEM.**

- **Agent-pub-key continuity (hosted → the person's OWN desktop) is BUILT** both ends: doorway exports an encrypted `KeyExportFormat` custodial bundle; the Tauri desktop decrypts it (Argon2id+ChaCha20, `steward/device/src-tauri/src/identity.rs:34`) and installs the hApp with the *same* agent key (`lib.rs:687-694`). DHT authorship identity is genuinely preserved across Stage 2→3. (Caveat: end-to-end live exercise of the Tauri shell is unproven from the repo.) **But this is App-Steward graduation — a person onto their own laptop — not laptop→NUC→DwellingHub hub-role graduation.** Different axis.

- **The asked axis — moving a hub role across hardware while preserving identity — is SPEC-ONLY**, and its prerequisite is an **explicitly unsolved lineage problem**:
  - **Source-chain history transfer is ABSENT.** Installing with a provided key genesis-seeds a *fresh* chain; the export seam (`export_for_migration`, `lib.rs:8092`) exists but "is a door with no road yet" — no bridge call anywhere (`2026-06-11-dna-upgrade-governance.md` §6).
  - **The `lineage` DNA field is regressed** since 2026-04-24 (Holochain 0.6 gates it behind `unstable-migration`; all five `dna.yaml` omit it; the hygiene check was deleted — §3).
  - **No migration/consensus mechanism exists** (§7 "no mechanism exists for any of these"). Forcing a conductor reinstall mints a *new* agent key that needs a migration that doesn't exist (root CLAUDE.md).
  - **Household identity is config-declared, not migrated** — `HOUSEHOLD_ID` is an env var re-declared per deploy (`boot_registration.rs:152-157`), not carried with the agent.

**What is needed:** (1) restore the `lineage` field (track the upstream `unstable-migration` gate); (2) build the export→transform→import bridge call (the road for the existing door); (3) a lineage-aware migration/consensus flow so a graduated hub on new hardware presents as the *same* steward identity carrying its history; (4) carry `HOUSEHOLD_ID` with the agent rather than re-declaring it. Until (1)–(3) land, hub graduation across hardware that changes the DNA/network *cannot* preserve the key — it is, by the governance doc's own framing, an open problem, not an implementation gap.

---

## 6. BRIEF DISAGREEMENTS / UNCERTAINTIES

The three briefs were **mutually reinforcing, not contradictory** — each independently confirmed the deploy-time-static / human-runtime-dial split, and the adversarial passes only corrected file-path labels (no status label changed). Residual uncertainties to flag:

1. **`steward/node` build status — corrected, not contradicted.** The provisioning brief first said the crate "builds via no Jenkinsfile / built by nothing"; its own verification corrected this: CI *does* gate-test it (`steward/device/build-manifest.json:15-17,38` watches `steward/node/src/**`, declares a `steward-node` gate) and a `Dockerfile` exists. The accurate statement is: **compiled and gate-tested, but no pipeline publishes an `elohim-node` image and no manifest deploys one.** BUILT-UNCONSUMED survives; the "built by nothing" phrasing was overstated.

2. **Tauri key-handoff reachability is UNPROVEN.** The tiering brief flags that the hosted→desktop key-bundle path is built both ends but could not be confirmed to run in any tested/deployed path (no evidence the Tauri binary executes in a live flow). Treat B8 as "BUILT, reachability unverified," not "working in production."

3. **`arcFactor` granularity.** One brief described it as a fractional intensity dial; verification sharpened it to a **`{0,1}` binary deployed lever** (fractional is upstream-blocked in kitsune2, per the `$arcFactorComment` in `deployments.json`). Doesn't change the verdict (still operator/deploy-time), only the "intensity dial" framing.

4. **File-path labels (cosmetic, resolved).** `peer_map.rs` → `elohim/elohim-storage/src/p2p_iroh/peer_map.rs`; `http_blob_router.rs` is at `elohim/elohim-storage/src/http_blob_router.rs` (NOT under `src/p2p_iroh/`). All cited line numbers are correct at the true paths.

No brief left the *verdict* uncertain. The convergent finding across all three: enablement is a developer/operator flow today; every steward-facing surface of the human-owned hub role is stubbed, simulated-and-disabled, or absent.
