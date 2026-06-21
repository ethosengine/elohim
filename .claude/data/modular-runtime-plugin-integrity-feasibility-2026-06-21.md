---
title: "Feasibility — Modular, Device-Tuned elohim Runtime with Holochain-Level Plugin Integrity"
written: 2026-06-21
author: workflow:modular-runtime-plugin-integrity-feasibility
status: assessment
---

# Feasibility — Modular, Device-Tuned elohim Runtime with Holochain-Level Plugin Integrity

**Scope.** This is the SINGLE-NODE COMPOSITION assessment: how one node's runtime is *composed* (footprint tuned to the device; extended by native mods/plugins/crates; high-leverage plugins carrying holochain-level integrity). It is ORTHOGONAL to the three sibling assessments about network ROLES (hub enablement, peer-hoster dataplane, commons aggregation). Nothing here recommends hub/peer/commons work — only footprint, the plugin loader, and integrity wiring.

Built from five adversarially-verified briefs (extension machinery; footprint tuning; the WASM-zome integrity model; capability/attestation substrate; rakia/manifest composition). Where a brief's own verification flipped its finding, this assessment uses the **corrected** verdict.

---

## 1. The Feasibility Verdict

The headline question splits into two sub-questions that land in **different buckets** — collapsing them to one answer is the main way the verdict goes wrong.

| Sub-question | Verdict |
|---|---|
| **How native can the mod/plugin/crate tooling be?** (a runtime loader/host/registry for native code) | **needs-design** — genuinely absent end-to-end. No `wasmtime`/`extism`/`wasmer`-host in any native service, no `libloading`-as-registry, no `trait Plugin`/`inventory`/`linkme`. Every native extension today is compile-time: a path-dep crate + a cargo feature + a hand-written dispatch arm. "Pluggable bridges" means *recompile with a different crate*. |
| **Can plugins realistically get holochain-level integrity?** (given a plugin to attach the property to) | **feasible-with-work** — composable from BUILT primitives, NOT net-new cryptography. The protocol already runs the exact pattern one layer down (untrusted swappable code + hash-bound validator). The authority rail (`bounds_validator` + `delegates-compute`) and a content-addressed DHT attestation surface (`issue_attestation`) both exist; only the plugin *application* of them is missing. |
| **Device/tier-tuned footprint from one core?** | **feasible-with-work** — machinery is ~60% present and demonstrably compiles; cheap first steps exist. What's missing is connective tissue, not invention. |

**Single biggest enabler already in-tree:** the **`bounds_validator` 7-check engine + `delegates-compute` Commitment** (`elohim/elohim-storage/src/services/bounds_validator.rs`; `elohim/holochain/dna/mishpat/.../commitments.rs:540-589`; projected live via `signals.rs:846-908`). It is a BUILT, bounded/revocable/audited authority primitive whose shape is *exactly* "this actor may do X within these bounds, revocably, with audit." Scoping it to `execute-plugin` is the cheap, high-leverage compose target. (The architectural *template* for the hard property is the validator-tethered coordinator hot-swap in `happ_manager.rs` — the live proof that swappable code can be as-good-as-core in execution privilege.)

**Single biggest missing piece:** the **native execution host + its load-time substrate gate** — the one place where native plugin code would be BOTH loaded AND checked against the substrate before it runs. It is absent end-to-end and is the bottleneck for *both* sub-questions: without a host there is nothing to load a plugin into, and without a load-time gate there is no point at which substrate-attested trust can be enforced.

---

## 2. Inventory Table

| Machinery | Status | Evidence (file:line / doc) | Gap |
|---|---|---|---|
| **Bridges / extension system** | LIVE-WIRED (the one route); ABSENT as a *system* | `bridges/` = `CLAUDE.md` + `valueflows/` only; consumed as plain path-dep `elohim/elohim-storage/Cargo.toml:51-52` (not even feature-gated); dispatch is hardcoded `else if sub_path=="vf-graphql"` `api/mod.rs:423-437` | No `trait Bridge`/registry/discovery. Adding a bridge = edit Cargo.toml + add a match arm + recompile. atproto = SPEC-ONLY (Draft); activitypub = ABSENT. Doorway consumes zero bridges. |
| **Cargo-feature footprint (the levers)** | LIVE-WIRED (mechanism); BUILT-UNCONSUMED (for the shipped binary) | storage `[features]` `Cargo.toml:267-273` (`graph-native`/`p2p`/`p2p-iroh`/`ssr`/`compression`); cfg-gated with a real thin-build stub `lib.rs:69-86` (zero-size `GraphEngine(Infallible)`); V8/ssr stripped on every storage build `Dockerfile:99-110` | The **edge ships the fat default** — `elohim-node` container = the `elohim-storage:STORAGE_TAG_PLACEHOLDER` image (`genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml:219-223`). The thin storage flavor (`default-features=false, features=["p2p"]`, `steward/node/Cargo.toml:36`) is built by **no pipeline** → BUILT-UNCONSUMED, not LIVE-WIRED. doorway has no `[features]` block at all. No `[profile.release]` size tuning anywhere. |
| **Workspace crate graph** | LIVE-WIRED (boundary modularity) | No top-level workspace (`/projects/elohim/Cargo.toml` absent); `elohim/Cargo.toml` excludes the big crates; `elohim-storage` pulls 15 path-deps; `elohim-facings` extracted DB-free; cache-core `native`-vs-WASM split is the one true device-target compile divergence | Modularity is compile-time crate/feature selection, not a runtime "core + slot." No tier→crate-set mapping. |
| **Zome integrity model** | LIVE-WIRED (but partial; two senses of "core") | DNA hash excludes coordinators by design `holochain_zome_types/.../dna_def.rs:75-78,45-47`; content-addressed wasm `dna_file.rs:85` + wasmer 6.0.1 `holochain/Cargo.toml:95`; coordinator hot-swap `happ_manager.rs:418-494` gated `:126-128` | Coordinators are **execution-privilege**-equal to core, NOT **network-consensus**-equal (DHT never votes on coordinator bytes; swap is operator-gated). Even the "real" integrity layer is partial: `AgentPeerBinding` self-asserted (`STAGE1_SIGNATURE_SENTINEL`). `ALLOW_COORDINATOR_UPDATE` is **code-default-inherited** from `ALLOW_DNA_REINSTALL` — not independently pipeline-wired (template comment only). |
| **Native-plugin integrity mechanism** | ABSENT | No artifact hash on `NodeRegistration` (`node_registry_integrity/src/lib.rs:56-96` = free-text `zomes_hosted` + self-signature, no `binary_hash`/`image_digest`); no `wasmtime`/`extism` (storage `Cargo.toml:82` "we're native, not wasm"); no `cargo-vet`/`cosign`/`sigstore` (only `deny.toml` = license/advisory gating); `libloading` is tx5 transport FFI only | No content-addressed, notarized, sandbox-isolated native unit anywhere. brit's `BuildAttestationContentNode` (`elohim/brit/brit-epr/src/elohim/attestation/build.rs:9-46`) is real ed25519 but brit-submodule-LOCAL — NOT on the DHT, NOT for plugins; the promised `build-attestation.schema.json` does not exist. Integrity-by-analogy, not a wired path. |
| **Capability / attestation authorization** | BUILT-UNCONSUMED (authority primitive); LIVE-for-other-kinds (attestation surface) | `delegates-compute` Commitment + 7-check `bounds_validator.rs`, projected `signals.rs:846-908`; generic `issue_attestation(kind, subject_cid)` `content_store/src/attestation.rs:16,44,47`, live for ~23 kinds, allowlist-gated `generated_attestation_kinds.rs:7` | **TRAP:** the `@capability*` tags on elohim-elements are UI **render profiles** (lens/theme/contrast), NOT authorization (`elohim-elements/CLAUDE.md`; `capability/mixin.ts:26`) — do not conflate. Enforcement live on only one conditional path (`api/epr.rs:622-680` republish-epr) + commons-provide author emit + diagnostics route; no live CI writer. No `attestation:plugin-trust` kind; no `execute-plugin` scope; no artifact-hash bound; no loader-side gate. |
| **rakia composition** | BUILT-UNCONSUMED (engine); LIVE-WIRED (manifest format, via JS) | `build_constellation`/`plan_from_changes` `rakia-core/src/constellation.rs:81-248`; live consumer is `genesis/orchestrator/graph-walker.mjs` (zero rakia refs); rakia-core has no `fn main`/`[[bin]]`, `rakia-executor/-peer/-cli` commented out `Cargo.toml:6-10`; brit Stage 1a (`brit-helper.sh` WARN+exit 0) | rakia is a **build-time CI change-detection** substrate — orthogonal axis (rebuild-which-steps, not assemble-which-runtime). No `tier`/`profile`/`flavor`/`features`/`device-class` field in any of 11 live manifests. `composition` executor `kind` is SPEC-ONLY and means build-step, not device-runtime. Plausible *future* extension point only. |

---

## 3. The Integrity Path — making a native plugin "might as well be core"

**The reframe that carries the whole section: trust the notary, not the plugin.** The WASM/zome model does NOT achieve trust by making the swappable code immutable. It makes the swappable code **untrusted** and routes every effect on notarized state through a **hash-bound validator** (the integrity zome, whose hash is part of the DNA hash). A hot-swapped coordinator runs with byte-identical execution privilege to a bundled one, yet cannot forge notarized data because the immutable validator gates every DHT write regardless of authorship. Trust is concentrated in the validator; the plugin stays cheap and replaceable.

So the menu of candidate mechanisms the task named buys *different properties*, and only one of them is the integrity that matters:

| Candidate | What it actually buys | Verdict for "as-good-as-core" |
|---|---|---|
| Content-hash notarization of the plugin artifact | **Provenance / audit** (which bytes ran, attestably) — NOT integrity-of-effect | Necessary for audit, insufficient alone |
| `wasmtime`/`wasmer` sandbox for native plugins | **Isolation** (a different, orthogonal property) | Nice-to-have, not the integrity property; ABSENT today |
| Signed-crate + supply-chain attestation | **Build-time provenance** | Adjacent (brit has the ed25519 piece), not runtime integrity |
| Capability-grant-to-execute (`delegates-compute`) | **Bounded, revocable, audited authorization** (who may run, within what bounds) | The authorization half — BUILT primitive |
| **Validator-routing of effects** (the existing zome-call + integrity path) | **Integrity-of-effect** — the actual "as-good-as-core" property | The keystone; already exists, free to borrow |

**The concrete minimal mechanism — make the native plugin a notary CLIENT, not a notary:**

1. **Run the plugin untrusted in the host** (`storage`/`doorway`), exactly as a coordinator runs untrusted in the conductor. It gets *no* privileged write surface to notarized state. (The `conductor-first` rule in `elohim/holochain/dna/CLAUDE.md` already mandates this for SQLite.)
2. **Route every notarized write through the existing conductor zome-call + integrity-validation path** — which already rejects invalid writes regardless of who authored them. Integrity-of-effect comes **for free**, borrowed from the validator. This is the line that makes the plugin "might as well be core."
3. **Supply provenance + authorization from already-built primitives:** a `delegates-compute`-shaped Commitment with `scope = execute-plugin` and `bounds = {allowed-artifact-CIDs, reach_ceiling, rate, ttl}`, enforced by `bounds_validator` on the plugin's `bounded_by` execution event; plus `issue_attestation("attestation:plugin-trust", subject_cid=<plugin CID>)` to put "this artifact is trusted for capability X" on the DHT, revocably and queryably.

**Assembled from existing primitives (compose, don't invent):**
- Content-addressed plugin identity — the protocol's native CID *is* the artifact identity.
- Bounded/revocable/audited right-to-execute — `delegates-compute` + `bounds_validator` (BUILT).
- Revocable, queryable DHT trust binding — `issue_attestation(kind, subject_cid)` (BUILT, live for other kinds).
- Integrity-of-effect — the conductor's hash-bound validation path (LIVE-WIRED).

**Must be invented / net-new wiring (none cryptographically novel):**
- The `attestation:plugin-trust` kind (a manifest + codegen edit — mechanically trivial).
- The `execute-plugin` scope + an `executes-plugin` event class with a `bounded_by`.
- An **artifact-hash → capability binding** in commitment bounds (today bounds carry `epr_scope` content-ids, not artifact CIDs).
- The **load-time enforcement gate** at the host's load site (today even the one real loader — Sophia's `script.src` from unpinned CDN, no SRI — has no hook to ask the substrate "is this CID attested before I run it?").
- A **live writer** for the commitment/attestation (enforcement is dormant even for existing scopes — the deploy pipeline still uses the Z.1 anti-pattern).

A wasmtime/extism sandbox would add *isolation* and is a clean upgrade, but it is a separate concern and does NOT by itself confer the network-consensus integrity the validator-routing path provides.

---

## 4. The Footprint Path — device/tier-tuned binaries from one core

The machinery is ~60% present and demonstrably compiles. The thin storage flavor *builds and embeds cleanly*; the storage image *already* excludes V8; the cache-core native-vs-WASM split is real and live. What's missing is connective tissue: nothing maps a device/tier/role to a feature set (not in code, not even in a spec), the edge ships the fat default, and there is zero release-profile size tuning.

**Cheapest first steps, in order:**

1. **Select the thin flavor for the standalone/edge storage image** — pass `--no-default-features --features "p2p compression"` (or whatever the headless edge actually needs) in `elohim/elohim-storage/Dockerfile`. The cfg-stubs already make this compile (`lib.rs:69-86`); `steward/node` already proves the thin storage embeds. Biggest single dependency-mass cut (drops cozo + async-graphql + the GraphQL surface). [code-now]
2. **Add `[profile.release]` size tuning** to root `.cargo/config.toml` — `opt-level="z"` (or `"s"`), `lto=true`, `codegen-units=1`, `strip=true`, `panic="abort"` (where panic-unwind isn't required). Currently ABSENT; shipped native binaries get default release codegen. Pure config, applies across all native services. [code-now]
3. **Replace the fragile `sed`-based V8/ssr strip with feature-selection.** The one footprint cut that actually ships is a line-anchored `sed` that silently breaks on any Cargo.toml reformat. Make it `--no-default-features` selection instead. [code-now]
4. **Give doorway a `[features]` block** so a no-UI / no-V8 doorway flavor becomes expressible (today: single fat binary including `elohim-render` unconditionally). [code-now / design-first for the feature taxonomy]
5. **Introduce a tier→flavor mapping** — the headline gap. The device-archetype model (`deployments.json` `deviceArchetype`; `genesis/plans/2026-04-13-device-archetypes-design.md`) maps archetype → RAM/CPU limits + fixture params today; extend it (or the build-manifest data-model, via a new `composition`/assembler executor kind) to carry a per-archetype feature/crate set. This is the connective tissue that turns "the levers exist" into "the right binary ships per device." [design-first]

Keep the WASM/DNA artifacts out of this: they are healthy by default (single-digit-MB DNAs via `hc dna pack`) AND must stay byte-identical across peers (a per-device WASM tweak would change the DNA hash → partition). Footprint tuning is a *native-binary* concern only.

---

## 5. Prioritized Work-List (leverage-ordered)

Cheap high-leverage items are flagged **[BUILT-UNCONSUMED → cheap win]** — they consume a primitive that already exists rather than inventing one.

1. **Add `attestation:plugin-trust` (and an `attestation:artifact-trust`) kind** — manifest + `pnpm run schema:codegen:rs`. Unlocks putting "this plugin CID is trusted for capability X" on the DHT using the BUILT, live-for-other-kinds attestation surface. **[code-now]** **[BUILT-UNCONSUMED → cheap win]**
2. **Select the thin storage flavor in the edge image** + add `[profile.release]` size tuning. Biggest immediate footprint cut from machinery that already compiles. **[code-now]** **[BUILT-UNCONSUMED → cheap win]**
3. **Define `execute-plugin` scope + `executes-plugin` event class, and wire `bounds_validator` onto a real path.** The 7-check engine + `delegates-compute` exist; this scopes them to plugins and lights the dormant enforcement on a path that actually fires. **[code-now]** for the scope/event; **[needs-substrate]** for a live commitment writer.
4. **Add artifact-CID to commitment bounds** (today `epr_scope` content-ids only) so a grant can say "these artifact hashes may execute." **[needs-substrate]**
5. **Replace the `sed` V8/ssr strip with `--no-default-features`; give doorway a `[features]` block.** Removes a fragile shipped cut and makes a no-UI doorway expressible. **[code-now]**
6. **Build the load-time substrate gate at the plugin load site** — the host asks the substrate "is this CID attested + within bounds?" before executing. Even the existing Sophia loader (`script.src`, no SRI) has no such hook. This is the integrity keystone for native plugins. **[needs-substrate]**
7. **Introduce a tier/archetype → feature-set mapping** (extend `deviceArchetype` or add a `composition` executor kind to the build-manifest). The headline footprint gap; converts "levers exist" into "right binary per device." **[design-first]**
8. **Design the native plugin execution host** — a `wasmtime`/`extism` host (preferred: it gives isolation *and* a natural CID-addressed unit) or a vetted native-ABI/trait-object registry. The single biggest missing piece; the bottleneck for native-plugin tooling end-to-end. **[design-first]**
9. **Promote a real `delegates-compute` live writer** in CI/deploy (retire the Z.1 `PATCH /db/content` anti-pattern) so enforcement is exercised end-to-end before plugins depend on it. **[needs-substrate]**

---

## 6. Where the briefs disagreed or left uncertainty

- **"elohim-node" is an overloaded name — the footprint brief's headline claim was wrong and its own verification flipped it.** The *deployed* "elohim-node" container is the **fat `elohim-storage` image**, not the `steward/node` thin-storage binary. The thin flavor is BUILT-UNCONSUMED (no pipeline builds it), not LIVE-WIRED. (Resolved in favor of the corrected verdict; confirmed by direct Dockerfile/manifest check.)
- **Two senses of "as-good-as-core" for coordinators.** A hot-swapped coordinator is execution-privilege-equal to bundled code, but NOT network-consensus-equal — the DHT never votes on coordinator bytes; the swap is operator-gated. CLAUDE.md's "plugin that is cryptographically as-good-as-core" framing is true only in the execution-privilege sense. This nuance strengthens the pessimistic read.
- **The "real" integrity layer is itself partial.** Even AgentPeerBinding is self-asserted/unsigned today (`STAGE1_SIGNATURE_SENTINEL`); the transport-identity resolver is blocked. So the foundation the native-plugin path would borrow from is not a finished cross-signed substrate.
- **`ALLOW_COORDINATOR_UPDATE` is not independently pipeline-wired** — it inherits via code default from `ALLOW_DNA_REINSTALL`. One brief over-stated deploy integration (cited `Jenkinsfile:614`, which is the reinstall flag). Practical consequence: you cannot arm coordinator hot-swap on prod without also enabling the re-key-bearing reinstall flag; default-OFF on prod.
- **The `@capability*` trap.** One brief had to actively disambiguate: elohim-elements `@capability*` tags are UI render profiles, not security capabilities. Any reasoning of the form "elohim-elements has capability contracts, so plugins can be capability-gated" is a category error. The authorization substrate is Mishpat commitments, a different layer entirely.
- **brit attestation: real crypto, wrong home.** brit's `BuildAttestationContentNode` is genuine ed25519 (refuting "pure vapor"), but it lives in the brit submodule, writes to git notes-refs (NOT the Holochain DHT), is Stage-1a-disabled in CI, governs deploy-promotion (not runtime authorization), and has no plugin notion. Adjacent vocabulary, not a wired path — the integrity-by-analogy in the rakia spec should not be read as a half-built capability.
- **Enforcement surface is slightly larger than the narrowest claim** — `bounds_validator` is invoked by the republish-epr conditional branch AND the commons-provide author emit (`economic_event_emit_service.rs:195` via `conductor_commitment_author.rs:311`) AND the diagnostics route. But all are internal/reconciler-driven or conditional; the core verdict (no generic, user-facing, or plugin path drives enforcement; no live CI writer) holds.
- **Uncertain / unmeasured:** native release binary sizes were not measurable in-tree (no release artifacts; the cited 925 MB is a *dev* binary, ~79% debuginfo). The footprint-cut magnitudes in step 1-2 are directional, not measured. The `elohim.happ` 10.6 MB / per-DNA sizes are from `local-dev/deployed-bundles` and may not byte-match the current edge build.
