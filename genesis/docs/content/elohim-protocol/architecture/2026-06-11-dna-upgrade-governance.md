---
title: DNA Upgrade Governance
id: dna-upgrade-governance
tier: architecture
status: accepted — policy home + as-implemented distillation (truth:DERIVED — the stewardship philosophy lives in elohim/holochain/rna/README.md and protocol canon; this seed homes the upgrade *policy* and its enforcement, not the philosophy)
created: 2026-06-11
informed-by:
  - elohim/holochain/rna/README.md (Constitutional Evolution — the philosophy's living home; this seed only summarizes it)
  - genesis/docs/superpowers/specs/2026-04-21-bootstrap-steward-authority-frame-design.md (the Wave-1 sibling spec; progenitor/bootstrap-steward properties this policy's hygiene checks enforce)
  - genesis/docs/superpowers/specs/2026-06-09-cluster3-substrate-signal-migration-governance-signal-flow-design.md (§2.6, §8 — the live operator fork this policy governs: ALLOW_DNA_REINSTALL + pre-field lineage decision)
informs:
  - Any integrity-zome change (entry structs, link types, validation) — consult the hash table below BEFORE committing
  - dna.yaml / happ.yaml manifest edits (the "Manifest hygiene (Wave 1 / §7)" comments in all five dna.yaml files + happ.yaml point at this seed)
  - Alpha→beta network-seed promotions; any ALLOW_DNA_REINSTALL decision
derived_from:
  - elohim/holochain/dna/NETWORK_UPGRADES.md  # retired to git 2026-06-11 (holochain dna/ island recompose)
cites:
  - elohim/holochain/tests/manifest-hygiene/tests/manifest_hygiene.rs
  - elohim/holochain/tests/manifest-hygiene/README.md
  - elohim/holochain/dna/elohim/dna.yaml
  - elohim/holochain/dna/elohim/workdir/happ.yaml
  - elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
  - elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs
  - elohim/holochain/dna/elohim/Cargo.toml
  - elohim/holochain/dna/build-manifest.json
  - lamad-v1-schema-museum | the distilled v1 export-surface snapshot the migration seam (§6) exports against | sha256:5f32d29b01a7cc8a | path: genesis/docs/content/elohim-protocol/history/2026-06-11-lamad-v1-schema-museum.md
  - elohim/holochain/rna/README.md
  - elohim/holochain/rna/rust/Cargo.toml
  - cluster3-substrate-signal-migration-governance-signal-flow-design | the live operator fork this policy governs — ALLOW_DNA_REINSTALL + the pre-field lineage decision (its §2.6, §8) | sha256:b758c9c0959c0fef | path: genesis/docs/superpowers/specs/2026-06-09-cluster3-substrate-signal-migration-governance-signal-flow-design.md
  - bootstrap-steward-authority-frame-design | the Wave-1 sibling spec behind the progenitor/bootstrap-steward properties that hygiene checks 8-9 enforce | sha256:6fb209d2628d39bb | path: genesis/docs/superpowers/specs/2026-04-21-bootstrap-steward-authority-frame-design.md
---

# DNA Upgrade Governance

In Holochain, the DNA hash IS the network identity. Any change to the integrity layer — entry struct definitions, link types, validation logic, validation constants — produces a new DNA hash, which is a **completely new network**: peers on the old hash and peers on the new hash cannot see each other, and there is no network-level bridge between them. This is not a bug to route around; it is the constitutional checkpoint the protocol builds its upgrade governance on (philosophy: elohim/holochain/rna/README.md §Constitutional Evolution). This seed is the policy home: what changes the hash, what the forward-compat rules are, where they are enforced, and which parts of the migration story are implemented versus vision.

## 1. What changes the hash (HC 0.6, verified per row)

Entry structs and the `EntryTypes`/`LinkTypes` enums are defined in the *integrity* zome (elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs:3670 `#[hdk_entry_types]`, :3807 `#[hdk_link_types]`); validation callbacks live there too (`validate_create_entry`, ~:4271). Any recompile of integrity WASM with changed code is a new DNA hash. Coordinator zomes are declared in a separate manifest section (elohim/holochain/dna/elohim/dna.yaml:21-30 `integrity:` vs `coordinator:`) and are excluded from the hash by Holochain's design — they can be iterated freely.

| Change | DNA hash changes? | Migration required? |
|---|---|---|
| Add entry field (even `#[serde(default)]`) | **Yes** | Yes (under prod seed; alpha resets) |
| Remove/rename entry field | Yes | Yes |
| Add link type | **Yes** | Yes |
| Change validation logic or its constants | Yes | Yes |
| Add coordinator function | No | No |
| Change coordinator logic | No | No |
| Add to `metadata_json` contents | No | No |
| Bump `network_seed` (modifiers) | Yes — deliberate fork; this is the reset mechanism | By design |
| Change `properties` (e.g. set `progenitor_pubkey`) | Yes | By design |

Row notes, verified this session:
- "Adding the integrity field changes the DNA hash" is restated as a live blast-radius constraint in the cluster3 spec (genesis/docs/superpowers/specs/2026-06-09-cluster3-substrate-signal-migration-governance-signal-flow-design.md §2.6) — including why `metadata_json` routes are preferred when the hash cost is unaffordable (§8: "The metadata_json route makes this tradeoff vanish entirely").
- `metadata_json` is a plain `String` data field on entries (content_store_integrity/src/lib.rs:508); changing its *contents* is data, not code — the designed extension valve.
- The last two rows are modifier semantics (network_seed + properties feed the DNA hash): the entire seed-ladder mechanic (§4) and the bootstrap-steward model depend on them. `progenitor_pubkey` lives in `modifiers.properties` per role (elohim/holochain/dna/elohim/workdir/happ.yaml; enforced by hygiene check 8) — so setting the founding steward's pubkey before alpha publish itself mints a new network. The happ.yaml placeholder (`progenitor_pubkey: ~`) is explicitly flagged "override before alpha publish."

## 2. Forward-compat policy — two axes the source doc conflated

The retired NETWORK_UPGRADES.md policy listed "new `#[serde(default)]` entry fields" and "new link types" under *additive changes — no hash bump*, while its own Safe-vs-Breaking table said those changes DO change the hash. **The table is correct** (entry structs and `LinkTypes` are integrity-zome code; see §1). The policy is coherent once split into two axes:

**Hash axis — what makes a new network.** Only coordinator-only changes and `metadata_json`-content extensions are hash-stable. Everything touching integrity code forks.

**Data-compat axis — what serde discipline buys.** `#[serde(default)]` on new optional entry fields (and never repurposing an existing link type) means entries written under the old schema remain deserializable by the new code — which is what makes migration *mechanical* (export → import with defaults filled) rather than *transformational*. This is practiced, not aspirational: the live `Content` integrity struct carries `#[serde(default)]` on `schema_version`, `validation_status`, `blob_cid`, `content_size_bytes`, `content_hash` (content_store_integrity/src/lib.rs:511-528; 13 `#[serde(default)]` sites in the integrity zome).

The governance rule that survives the split:

- **Additive integrity change** (serde-default fields, new link types): new hash, but data migrates mechanically. Under an `_alpha` seed this is absorbed by reset/reseed; under `_beta`/prod it requires the migration flow but no transform authoring.
- **Breaking integrity change** (removed/renamed fields, changed validation tighter OR looser, repurposed link types, changed entry-type enum variants): new hash AND a real transform. Record lineage intent (§3), document the migration path, and the stewarded migration flow applies.
- **Coordinator/metadata changes**: free. Iterate without ceremony. (This is also why "freeze integrity early, iterate on coordinators, extend via metadata_json" remains the standing development posture — the elohim hApp practices it: the integrity schema is tracked as a versioned contract — `SCHEMA_VERSION = "v1"` exported live, see §6; the v1 surface is distilled in genesis/docs/content/elohim-protocol/history/2026-06-11-lamad-v1-schema-museum.md.)

## 3. Lineage — intended end-state vs the current regression

**Intended end-state**: every dna.yaml declares `lineage: []`; the current hash is the genesis ancestor; every breaking change prepends the previous hash (`hc dna hash workdir/<name>.dna` → `lineage: ["uhC0k<old-hash>"]`) so tooling (hc CLI, Launcher, Moss-style admin UIs, future DHT bridging) can answer "this DNA supersedes X, Y, Z."

**Current regression (since 2026-04-24)**: Holochain 0.6 gates the `lineage` manifest field behind the `unstable-migration` cargo feature; the stable `hc dna pack` rejects it as an unknown field. All five manifests omit it; the manifest-hygiene check that asserted it (`every_dna_declares_lineage_field`) was removed, with the regression documented in place (elohim/holochain/tests/manifest-hygiene/tests/manifest_hygiene.rs:165-170; elohim/holochain/tests/manifest-hygiene/README.md check 4 "Removed"). Each dna.yaml carries the same note in its hygiene comment block (e.g. elohim/holochain/dna/elohim/dna.yaml:8-12).

**Today's mechanics**: upgrade history is reconstructed from **git history + network_seed rollover**. Not recording lineage at change time means the chain cannot be retroactively reconstructed from manifests alone — which is precisely why the seed-rollover convention (§4) keeps old hashes discoverable, and why the cluster3 spec treats the alpha pair's "pre-field lineage decision" as a deliberate, operator-owned step rather than an automatic one (cluster3 spec §8).

## 4. The network-seed ladder

Network seeds are stability contracts; the suffix declares intent. Enforced: every dna.yaml's `integrity.network_seed` MUST be `elohim_<dna>_alpha` (hygiene check 3, manifest_hygiene.rs `expected_network_seed`), and happ-role seeds must match their dna.yaml counterparts (check 7). Current seeds: `elohim_lamad_alpha`, `elohim_infrastructure_alpha`, `elohim_imagodei_alpha`, `elohim_mishpat_alpha`, `elohim_node_registry_alpha` (elohim/holochain/dna/elohim/workdir/happ.yaml).

| Suffix | Meaning |
|---|---|
| `_alpha` | WILL be reset on any breaking change. Not production. |
| `_beta` | Breaking changes migrate; no silent resets. |
| (none) | Production — breaking changes require governance + lineage + migration. |

Transitions are one-way (`_alpha` → `_beta` → prod). A reset means bumping `_alpha` to `_alpha2` (or similar) — while lineage is regressed (§3), the seed bump is the discoverable history record, with the old hash recoverable from git.

Note the dir-name ≠ DNA-name trap: `dna/elohim/dna.yaml` declares `name: lamad` (dna.yaml:20) with seed `elohim_lamad_alpha` — hygiene check 2 pins names, but humans navigating by directory will misread which network they're reasoning about.

## 5. Enforcement home

This policy is not prose-only; three layers enforce it:

1. **manifest-hygiene crate** (elohim/holochain/tests/manifest-hygiene/) — a fast (0.01s, no holochain deps) schema-contract test over the 5 dna.yaml files + happ.yaml. Its README lists the 10 checks: manifest_version "0" (HC 0.6 tag), DNA name pinning, the `elohim_<dna>_alpha` seed contract, (4 removed — lineage, see §3), happ version + 5 required roles, `clone_limit: 0` default-deny, happ/dna seed coherence, bootstrap-steward DNAs declare `progenitor_pubkey`, infrastructure does NOT (federation-native), no bare "progenitor" surface-language leaks. Registered as pre-push project `manifest-hygiene` in elohim/holochain/dna/build-manifest.json (gate.projects + step globs over `dna/*/dna.yaml`, `workdir/happ.yaml`, the crate itself). The `hrea` role is intentionally absent from the required set until a real `version_pin` + pipeline bundle-fetch lands (manifest_hygiene.rs documents the re-enable path; the commented-out role block in happ.yaml explains the #1249–#1253 breakage: `hc app pack` canonicalizes every `dna.path` even for deferred roles).
2. **dna.yaml hygiene comment blocks** — all five dna.yaml files and happ.yaml open with "Manifest hygiene (Wave 1 / §7)" comments restating the contract in place (e.g. elohim/holochain/dna/elohim/dna.yaml:3-13). The wave-1 execution plan that originally homed "§7" has left the tree; **this seed is now the doc those comments resolve to.**
3. **Deployment side** — a DNA-content change (new hash, same role structure) does NOT reach running conductors on a normal edge redeploy: the conductor data dir is a persistent PVC and the install stale-check is role-structure-only. Forcing reinstall is gated behind `ALLOW_DNA_REINSTALL` (non-prod=true per-env; reinstall mints a new agent key), and a partial reinstall across peers in one namespace = different hashes = different DHTs = P2P partition — the alpha genesis pair must both get the flag (root CLAUDE.md §"DNA changes don't redeploy by default"; treated as a live operator fork with its pre-field lineage decision in the cluster3 spec §2.6 + §8). The deployment gate is the operational enforcement of "a hash change is a network event, not a deploy."

## 6. The migration seam — what is implemented

**Export side: mechanism-shipped, live in the current coordinator.** The shipping `content_store` coordinator (not just the lamad-v1 archive) exposes `export_schema_version` (lib.rs:7962, returning `SCHEMA_VERSION = "v1"`, :7958), `export_all_content`, `export_all_paths_with_steps`, `export_all_mastery`, `export_all_progress`, and the single-call `export_for_migration` returning a `MigrationExport` bundle (lib.rs:8092-8118). All are registered in the coordinator's cache-rule table under an "EXPORTS (admin/migration endpoints)" section (lib.rs:1528-1549). The v1 surface they export against is distilled in the schema museum record (genesis/docs/content/elohim-protocol/history/2026-06-11-lamad-v1-schema-museum.md).

**Import/transform side: zero-wired.** No code outside the rna toolkit's own config defaults calls `export_for_migration` (the only references are rna/rust/src/config.rs:116,133 and rna/typescript/src/config.ts:43, which name it as the default export fn). No bridge call exists anywhere in the tree — `call_bridge`/`migrate_from_v1` appeared only as illustrative examples in the retired doc. The export seam is a door with no road yet.

**The rna/ toolkit (`hc-rna`): live as a library, unwired as a migration pipeline.** Liveness verdict, verified at source:

- *LIVE — compiled into every DNA build.* `hc-rna = { path = "../../rna/rust" }` is a workspace dependency of the elohim DNA (elohim/holochain/dna/elohim/Cargo.toml:15), consumed by both zomes: the **integrity** zome uses `hc_rna::SelfHealingEntry` for structural entry validation inside `validate_create_entry` (content_store_integrity/src/lib.rs:4272), and the **coordinator** wires `hc_rna::{BridgeFirstStrategy, EntryTypeRegistry, FlexibleOrchestrator, FlexibleOrchestratorConfig}` at init (content_store/src/lib.rs:1136, `init_flexible_orchestrator` registering per-entry-type providers). The DNA pipeline rebuilds when rna/ changes (`elohim/holochain/rna/**` is a build-dna-wasm input, elohim/holochain/dna/build-manifest.json:21). The self-healing fields this machinery serves (`schema_version`, `validation_status`) are live on the `Content` integrity struct (§2).
- *LIVE — operational seeding tool.* The `hc-rna-fixtures` CLI (a bin of the same crate, rna/rust/Cargo.toml) validates JSON seed data pre-seeding and is the documented validation step in the seeding workflows; `hc-rna-schema --export-enums` generated the seeder's validation constants (genesis/seeder/src/validation-constants.ts:8).
- *UNWIRED — the cross-DNA migration purpose.* The `templates/` (migration.rs.template, migrate.ts.template, self-healing.rs.template) have zero consumers outside rna/. The TypeScript package `@holochain/rna` is not a pnpm-workspace member and has zero importers. No export→transform→import pipeline runs anywhere.

So the retired doc's STATUS note ("the rna/ module, currently on the backburner") was accurate about the migration *workflow* and misleading about the *crate* — hc-rna ships inside every DNA we build. Record both halves; bless neither beyond its evidence.

## 7. Stewarded coordination — summary only

The governance answer to "DNA hash = network identity: feature or bug?" is **stewarded coordination**: the constraint is a constitutional checkpoint, and the elohim — constitutional stewards, not administrators — are the coordination mechanism that pure P2P networks lack. They cannot change rules (the DNA enforces them; users can audit the new DNA or keep running v1); they facilitate transitions. That philosophy's **living home is elohim/holochain/rna/README.md** (§Constitutional Evolution, §The Work of the Elohim — "consensus-finders" + translation across global/constitutional/local/personal levels, §RNA's Role in Constitutional Change). This seed deliberately does not restate it; read it there.

**As-implemented today** (each mechanic cited above): the hash-change table and serde discipline (§1-2), the seed ladder + manifest hygiene enforcement (§4-5), the export seam + hc-rna library liveness (§6), and the operator-gated reinstall fork with its lineage decision (§5.3) — i.e. governance currently runs through *operator decisions constrained by enforced contracts*, with the alpha genesis pair as the proving ground (cluster3 spec §8).

**§Vision remainder — no mechanism exists for any of these:**
- The **Elohim consensus migration flow** (proposal → elohim consensus as the coordination signal → migration window with stewards running v1 AND v2 → progressive data migration → sunset). No consensus mechanism exists; no dual-DNA migration window has ever been run.
- **Bridge calls for migration** (`call_bridge` from v2 coordinator into a locally-installed v1 cell): example-only, never implemented.
- **Seeder-doubles-as-migration-tool** (export → transform → import reusing the seeding pipeline): idea-won (the seeder and the export functions both exist) but never composed.
- **Lineage-driven cross-version DHT query routing**: upstream-blocked (§3) and unbuilt.

## 8. Open questions (carried forward from the retired doc, still open 2026-06-11)

1. **Migration tooling** — the exact export/import pipeline. Evolved but open: export side shipped (§6); transform/import/orchestration unwired.
2. **Elohim consensus mechanism** — how stewards agree on upgrade readiness. Fully open (vision-only, §7).
3. **User notification** — how users learn of upcoming migrations. Open.
4. **Rollback strategy** — what if v2 has critical bugs post-migration. Open; the nearest live instance is the cluster3 spec's unanswered alpha-pair reconciliation ("backfill pre-field events or genesis a fresh chain?", §8 there).
5. **Independent/self-hosted users** — how they participate in coordination. Open.

New since the source doc:

6. **OPEN QUESTION:** when the upstream `unstable-migration` feature stabilizes (or a holonix flavor enables it), does lineage get backfilled from git across all five DNAs in one change, and does the removed hygiene check return as written? (manifest_hygiene.rs:165-170 anticipates reintroduction but doesn't specify the backfill.)
7. **OPEN QUESTION:** should *additive* integrity changes (hash-bumping but data-compatible, §2) record lineage intent once the field returns, or is lineage reserved for breaking changes only? The retired policy said breaking-only, but its additive list was partly mislabeled (the table/policy contradiction documented in §2) — the policy revision that resolves the contradiction owns this call.
