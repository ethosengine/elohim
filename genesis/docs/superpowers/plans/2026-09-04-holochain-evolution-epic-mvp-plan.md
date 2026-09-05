---
title: "Holochain Evolution Epic — MVP implementation plan (Stations 1–5, 9, 10 on the household mesh, then 6–8)"
id: holochain-evolution-epic-mvp-plan
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
domain: D2
habits: [happ-lineage-migration]
graduation-trigger: every `- [ ]` task below is checked with its named evidence line in the epic's §11 ledger, and `@concern:happ-lineage-migration` Stations 1–10 pass on the household mesh (the habit's runnable check)
topic: [dna-lineage, happ-migration, notarization-witness, release-channel, mishpat, adoption-controller, dual-cell, a2o, node-registry]
informed-by:
  - genesis/docs/superpowers/specs/2026-09-03-holochain-evolution-epic-design.md (THE spec; §11 is the ledger every task reports into)
  - genesis/a2o/features/delivery/happ-lineage-migration.feature (the finish lines — Stations 1–10)
  - genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md (rung 5: the channel, verify, vehicles and receipt chain every task rides)
cites:
  - "holochain-evolution-epic | Holochain Evolution Epic | sha256:3855d752cdd009cd | path: genesis/docs/superpowers/specs/2026-09-03-holochain-evolution-epic-design.md"
  - genesis/a2o/features/delivery/happ-lineage-migration.feature
  - elohim/holochain/.epr-meta/happ-lineage-migration.habit.md
  - elohim/holochain/tests/sweettest/src/tests/happ_lineage_migration.rs
  - elohim/elohim-storage/src/services/release_adoption/verify.rs
  - elohim/elohim-storage/src/services/release_adoption/apply.rs
  - elohim/elohim-storage/src/happ_manager.rs
  - elohim/elohim-storage/src/hc_client_registry.rs
  - elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs
  - elohim/rakia/schemas/v1/release-manifest.schema.json
  - genesis/a2o/steps/delivery/runtime-upgrade-propagation.steps.ts
---

# Holochain Evolution Epic — MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Each task carries ONE `- [ ]` deliverable checkbox (the valueflow mints one commitment per checkbox — keep it that way); the numbered steps inside are the work.

**Goal:** the household mesh crosses `node_registry` from v1 to v2 (v1 + `NotarizationWitness`) carried by the network itself — Stations 1–5, 9, 10 of `happ-lineage-migration.feature` green — then Stations 6–8.

**Architecture:** rung 5's release channel carries a `happ-lineage` manifest; the adoption controller refuses it unless it names its parent and the elohim's `migrates-lineage` commitment is notarized; the apply vehicle installs v2 BESIDE v1 under the same agent key and drives a bounded cross-cell carry; v2's integrity zome re-verifies every carried v1 notarization (kernel proven: probes A/B). Storage changes are additive (Codex D2): `HcClient` untouched.

**Tech Stack:** holochain 0.7.0 / hdk 0.7 / hdi 0.8 (sweettest pins), Rust (elohim-storage, zomes), JSON Schema (rakia release manifest), TypeScript a2o (cucumber, `@holochain/client`), the household mesh (`just mesh …`).

**Spec:** `genesis/docs/superpowers/specs/2026-09-03-holochain-evolution-epic-design.md` — read §2 (record), §3 (gate + constraints), §4/§4.1 (authority), §6 (seams), §11 (ledger) before any task.

## Global Constraints

- **Line:** 0.7 only. CLI: `/projects/.claude-config/tools/hc-0.7/{holochain,hc}` first on PATH for every pack (`/opt/holochain/bin/hc` is 0.6 — never). Relay: `/projects/.claude-config/tools/iroh-relay-1.0.3/bin/iroh-relay`.
- **Default DNA pack stays byte-identical** to the pristine hash `uhC0kyvKwO2J5u3mf52tjASWe0ryhdpNYalrSeMGJODF3OpUxyeoH` (local; CI's differs by the local-only RUSTFLAGS and is `dna-hashes.baseline`). Every integrity-zome edit is gated by `#[cfg(feature = "lineage-witness")]`, appended AFTER the file's last line (any line shift moves the wasm), and re-hashed before commit. `just build` → `node-registry.dna` + `node-registry-v1.dna`; `just build-witness` → `node-registry-v2.dna`.
- **Cargo:** `berth claim cargo` first, `berth release cargo` after; `CARGO_BUILD_JOBS=4`; never two cargo builds at once; plain `cargo test` (no nextest); echo `EXIT=$?` on its own line after every cargo command; never judge from piped output. Sweettest: `RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__holochain__tests__sweettest/dev`. elohim-storage: `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev RUSTFLAGS='--cfg getrandom_backend="custom"'`, features `p2p p2p-iroh` for the mesh binary. DNA workspaces: plain in-tree cargo, no `CARGO_TARGET_DIR`.
- **Mesh:** `HOLOCHAIN_BIN=/projects/.claude-config/tools/hc-0.7 MESH_RELAY_BIN=/projects/.claude-config/tools/iroh-relay-1.0.3/bin/iroh-relay MESH_CONDUCTOR_LAUNCH=ark MESH_TRANSPORT_BACKEND=dual just mesh start`; storage rebuilt into the dev slot then `MESH_RESTART_APPLY_PROFILE=1 STORAGE_BIN=<slot>/debug/elohim-storage just mesh storage-restart matthew jessica james`. Never start mesh processes from a background tool task.
- **Vocabulary (spec §4):** the crossing is the ELOHIM's notarized path — never "consent", never `ConsentMissing`; refusals are `PathNotNotarized`, `PathRevoked`, `QuorumUnmet`, `RootMismatch`, `DnaLineageMismatch`. Adoptable ⇔ commitment `state == "active" && revoked_at IS NULL`.
- **Gate before commit (added 2026-09-05 after Task 3 turned the a2o lint gate red — `sonarjs/max-switch-cases` — a defect the task review could not see):** every task runs the gate for the tree it touched before committing — `just gate genesis/a2o` for a2o/TypeScript work, `just gate elohim-storage` for storage, the DNA workspace's `just clippy`/`just build` for zomes — and the report quotes the gate's exit line. A task review reads the report's gate evidence; a missing gate line is an Important finding.
- **Evidence:** every task ends with a one-line dated entry in the epic's §11.4 ledger and, when a Station flips, a DELTA in `elohim/holochain/.epr-meta/happ-lineage-migration.habit.md` + `python3 .claude/scripts/habits-project.py`. Commit-only; the integrator pushes. Path-limited `git add`.
- **Coordinator-only changes** ship to the running mesh by rung 5 (`epr-release-package.ts` + `release-ceremony.ts`, artifact class `coordinator-bundle`), never by mesh restart — each such delivery is a cycle-time row in the arc doc.

---

## File map (who owns what)

| File | Responsibility |
|---|---|
| `elohim/holochain/dna/node-registry/zomes/node_registry_coordinator/src/lib.rs` | v1: `export_records` (bounded); v2 (feature): `carry_from`, `commit_witness`, `get_witnesses_for` |
| `elohim/holochain/dna/node-registry/zomes/node_registry_integrity/src/lib.rs` | v2 (feature, appended): `NotarizationWitness` + validation (exists, `e233bb4f7`); `after-close` rule (Task 14) |
| `elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs` | `migrates-lineage`, `sunsets-lineage` arms; quorum on `revokes-commitment` |
| `elohim/rakia/schemas/v1/release-manifest.schema.json` | `happ-lineage` class, `migrateFrom`, `lineage`, `adoptionDiscipline.path` |
| `genesis/a2o/scripts/epr-release-package.ts` | packager flags for the new fields |
| `elohim/elohim-storage/src/services/release_adoption/{mod,verify,apply}.rs` | `ArtifactClass::HappLineage`, `PathEvidence`, `verify_path`, positive lineage branch, `HappLineageVehicle` |
| `elohim/elohim-storage/src/happ_manager.rs` | `install_lineage` (non-destructive, existing key) |
| `elohim/elohim-storage/src/lineage_roles.rs` (new) | `LineageRoles`: `role -> {app_id, authoring}` resolver |
| `elohim/elohim-storage/src/hc_client_registry.rs`, `src/node_registry_api.rs` | consult the resolver; registry-backed node-registry client |
| `elohim/elohim-storage/src/runtime_passport.rs` | dual-cell view per role |
| `elohim/holochain/dna/Jenkinsfile` | build + archive `node-registry-v2.dna` (feature variant) |
| `genesis/a2o/steps/delivery/happ-lineage-migration.steps.ts` (new) + `genesis/a2o/steps/delivery/lineage-candidate.ts` (new) | Stations 1–10 |
| `elohim/holochain/tests/sweettest/src/tests/happ_lineage_migration.rs` | kernel probes (A, B, B2) + carry-loop test |

---

### Task 1: v1 bounded export (`export_records`) — hash-neutral, delivered by rung 5

**Files:**
- Modify: `elohim/holochain/dna/node-registry/zomes/node_registry_coordinator/src/lib.rs` (unconditional section, beside `get_signed_action`)
- Test: `elohim/holochain/tests/sweettest/src/tests/happ_lineage_migration.rs` (new test `export_records_is_bounded_and_resumable`)

**Interfaces:**
- Produces: `#[hdk_extern] fn export_records(input: ExportInput) -> ExternResult<ExportPage>` with
  `ExportInput { cursor: Option<u32>, limit: u32 }` and
  `ExportPage { records: Vec<SignedActionHashed>, entries: Vec<Option<Entry>>, next_cursor: Option<u32>, digest: String }` — `records` are the agent's own chain actions of app entry types (`query(ChainQueryFilter::new().include_entries(true))`), ordered by `action_seq`, at most `limit` (cap 64); `digest` = hex sha256 over the concatenated action hashes of the WHOLE chain page-independent (compute once per call over all app records; Task 7 compares it to the carry receipt).

- [ ] **Task 1 deliverable: `export_records` returns bounded, cursor-resumable pages of the agent's own signed actions with a chain digest, tested in sweettest, and is live on the mesh via a rung-5 coordinator-bundle release (cycle-time row recorded).**

1. Write the failing sweettest (append to `happ_lineage_migration.rs`; it needs only v1):

```rust
#[tokio::test(flavor = "multi_thread")]
async fn export_records_is_bounded_and_resumable() -> Result<()> {
    let seed = format!("export-{}", uuid::Uuid::new_v4());
    let (mut conductor, alice) = single_agent_conductor().await?;
    let v1 = load_dna_from_path(&v1_path(), &seed, None).await?;
    let app = conductor.setup_app_for_agent("v1", alice.clone(), &[v1]).await?;
    let z = app.into_cells().remove(0).zome("node_registry_coordinator");
    for i in 0..5u32 {
        let _: ActionHash = conductor.call(&z, "register_node", node_registration(&format!("n{i}"))).await;
    }
    let p1: ExportPage = conductor.call(&z, "export_records", ExportInput { cursor: None, limit: 2 }).await;
    assert_eq!(p1.records.len(), 2);
    let p2: ExportPage = conductor.call(&z, "export_records", ExportInput { cursor: p1.next_cursor, limit: 64 }).await;
    assert!(p1.next_cursor.is_some());
    assert_eq!(p1.records.len() + p2.records.len(), 5 + /* genesis-era app records the fixture creates */ 0);
    assert_eq!(p1.digest, p2.digest, "digest is page-independent");
    assert!(p2.next_cursor.is_none());
    Ok(())
}
```
   (`node_registration(name)` is whatever the existing `register_node` test in `src/tests/node_registry.rs` builds — copy that constructor; define `ExportInput`/`ExportPage` mirror structs at the top of the test file with `#[derive(Serialize, Deserialize, Debug)]`.)
2. Run it: `cargo test --test happ_lineage_migration export_records -- --nocapture` → expect FAIL (`export_records` unknown zome fn). Echo `EXIT=$?`.
3. Implement in the coordinator (unconditional block, hash-neutral):

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExportInput { pub cursor: Option<u32>, pub limit: u32 }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExportPage {
    pub records: Vec<SignedActionHashed>,
    pub entries: Vec<Option<Entry>>,
    pub next_cursor: Option<u32>,
    pub digest: String,
}

const EXPORT_CAP: u32 = 64;

#[hdk_extern]
pub fn export_records(input: ExportInput) -> ExternResult<ExportPage> {
    let limit = input.limit.clamp(1, EXPORT_CAP) as usize;
    let all = query(ChainQueryFilter::new().include_entries(true))?;
    // only app entries (Create/Update with EntryType::App) — genesis, cap grants and agent-validation are not facts to carry
    let mut app: Vec<Record> = all.into_iter()
        .filter(|r| matches!(r.action().entry_type(), Some(EntryType::App(_))))
        .collect();
    app.sort_by_key(|r| r.action().action_seq());
    let mut hasher = Sha256::new();
    for r in &app { hasher.update(r.action_address().get_raw_39()); }
    let digest = hex::encode(hasher.finalize());
    let start = input.cursor.unwrap_or(0) as usize;
    let page: Vec<Record> = app.into_iter().skip(start).take(limit).collect();
    let next_cursor = if page.len() == limit { Some((start + limit) as u32) } else { None };
    Ok(ExportPage {
        entries: page.iter().map(|r| r.entry().as_option().cloned()).collect(),
        records: page.into_iter().map(|r| r.signed_action).collect(),
        next_cursor,
        digest,
    })
}
```
   Add `sha2` and `hex` to the coordinator crate's `Cargo.toml` if absent (check the mishpat coordinator, which already hashes payloads, for the exact versions used in this workspace).
4. `cd elohim/holochain/dna/node-registry && just build && just build-witness`; `hc dna hash node-registry.dna` MUST print `uhC0kyvKwO2J5u3mf52tjASWe0ryhdpNYalrSeMGJODF3OpUxyeoH`.
5. Run the test → PASS. Run `cargo test --test node_registry` → 3 passed (regression).
6. Deliver to the mesh by rung 5: from `genesis/a2o`, `pnpm exec tsx scripts/epr-release-package.ts --artifact ../../elohim/holochain/dna/node-registry/node-registry.dna --artifact-class coordinator-bundle --applies-to-from http://localhost:8090/version --channel-id runtime:coordinators:node_registry:commons` then `pnpm exec tsx scripts/release-ceremony.ts publish <manifest.json>` / `promote …` exactly as `runtime-upgrade-propagation.steps.ts` `ensureCanaryApplied` does; confirm `GET localhost:8090/version` shows the new node_registry coordinator wasm hash on all three peers. Record the wall-clock as a cycle-time row in `genesis/data/timeline/backlog/upgrade-propagation-p2p-design-arc.md`.
7. Commit: `git add elohim/holochain/dna/node-registry/zomes/node_registry_coordinator elohim/holochain/tests/sweettest/src/tests/happ_lineage_migration.rs && git commit -m "feat(node-registry): export_records — bounded, cursor-resumable, digest (hash-neutral; epic Task 1)"`.

---

### Task 2: mishpat `migrates-lineage` / `sunsets-lineage` arms + quorum on revocation

**Files:**
- Modify: `elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs` (`validate_commitment_payload` match at ~185-206; add validators after `validate_delegates_compute` ~722-767)
- Test: `elohim/holochain/tests/sweettest/src/tests/mishpat.rs` (append)

**Interfaces:**
- Consumes: `CreateCommitmentInput { action, payload_json, signed_at }`, `create_commitment`.
- Produces: accepted payload shapes (Codex E, adopted):

```json
{"action":"migrates-lineage","role":"node_registry","from_dna_hash":"uhC0k…","to_dna_hash":"uhC0k…",
 "release_cid":"uhCEk…","constitution_root":"bafy…","roster_cid":"bafy…","signing_payload_cid":"bafy…",
 "signatures":[{"agent":"uhCAk…","signature":"<base64 64B>"}],
 "evidence":{"soak":["bafy…"],"forecast":"bafy…","deliberation":"bafy…"},
 "window":{"opens_at":"RFC3339","revert_until":"RFC3339"}}
```
   `sunsets-lineage`: same identity fields + `migration_commitment_cid`, `evidence:{convergence:[…],soak:[…],deliberation}`, `window:{sunsets_at}`. `revokes-commitment` gains optional `signatures` checked with the same rule when the target is a lineage commitment (the payload carries `target_action`).
   Quorum rule (MVP): `signatures` non-empty, unique agents, and every signature verifies over the UTF-8 bytes of `signing_payload_cid` via `verify_signature(agent, sig, signing_payload_cid.as_bytes().to_vec())`; k = `required_signatures` (default 1) — roster-chain verification against `roster_cid` is Task 2b in the epic ledger (needs the elohim-DNA bridge; MVP declares the 1-of-1 progenitor roster).

- [ ] **Task 2 deliverable: `create_commitment` accepts a well-formed `migrates-lineage` and `sunsets-lineage` payload with ≥1 verified signature and refuses malformed ones with a named field; sweettest green; shipped to the mesh's mishpat cell by rung-5 coordinator hot-swap.**

1. Failing sweettest (append to `mishpat.rs`; mirror the file's existing `create_commitment` test setup):

```rust
#[tokio::test(flavor = "multi_thread")]
async fn migrates_lineage_commitment_requires_signature() -> Result<()> {
    let (conductor, cell, alice) = mishpat_cell().await?;            // reuse the file's fixture
    let cid = "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi";
    let sig: Signature = conductor.keystore().sign(alice.clone(), cid.as_bytes().to_vec()).await?;
    let good = serde_json::json!({
        "action":"migrates-lineage","role":"node_registry",
        "from_dna_hash":"uhC0kyvKwO2J5u3mf52tjASWe0ryhdpNYalrSeMGJODF3OpUxyeoH",
        "to_dna_hash":"uhC0kEKiIscIk5BDdethLGMFGLnvSvP2gRP5o74v0vAvoRnEzbiJ1",
        "release_cid":"uhCEkAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        "constitution_root":"bafyroot","roster_cid":"bafyroster","signing_payload_cid":cid,
        "signatures":[{"agent":alice.to_string(),"signature":base64::engine::general_purpose::STANDARD.encode(sig.0)}],
        "evidence":{"soak":["bafysoak"],"forecast":"bafyf","deliberation":"bafyd"},
        "window":{"opens_at":"2026-09-04T00:00:00Z","revert_until":"2026-09-11T00:00:00Z"}
    });
    let out: CommitmentOutput = conductor.call(&cell.zome("mishpat"), "create_commitment",
        CreateCommitmentInput { action: "migrates-lineage".into(), payload_json: good.to_string(), signed_at: "2026-09-04T00:00:00Z".into() }).await;
    assert!(!out.cid.is_empty());
    let mut bad = good.clone(); bad["signatures"] = serde_json::json!([]);
    let err = conductor.call_fallible::<_, CommitmentOutput>(&cell.zome("mishpat"), "create_commitment",
        CreateCommitmentInput { action: "migrates-lineage".into(), payload_json: bad.to_string(), signed_at: "2026-09-04T00:00:00Z".into() }).await.unwrap_err().to_string();
    assert!(err.contains("signatures"), "{err}");
    Ok(())
}
```
2. Run → FAIL with `unhandled action: migrates-lineage`. Echo `EXIT=$?`.
3. Implement, following `validate_delegates_compute`'s style exactly:

```rust
"migrates-lineage" => validate_migrates_lineage(&payload),
"sunsets-lineage" => validate_sunsets_lineage(&payload),
```
```rust
fn validate_lineage_signatures(payload: &serde_json::Value) -> Result<(), String> {
    let cid = payload.get("signing_payload_cid").and_then(|v| v.as_str())
        .filter(|s| !s.is_empty()).ok_or("signing_payload_cid must be a non-empty string")?;
    let sigs = payload.get("signatures").and_then(|v| v.as_array())
        .filter(|a| !a.is_empty()).ok_or("signatures must be a non-empty array")?;
    let required = payload.get("required_signatures").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
    let mut seen = std::collections::BTreeSet::new();
    for s in sigs {
        let agent = s.get("agent").and_then(|v| v.as_str()).ok_or("signature.agent missing")?;
        if !seen.insert(agent.to_string()) { return Err(format!("duplicate signer {agent}")); }
        let key = AgentPubKey::try_from(agent).map_err(|e| format!("signature.agent invalid: {e:?}"))?;
        let raw = base64::engine::general_purpose::STANDARD.decode(
            s.get("signature").and_then(|v| v.as_str()).ok_or("signature.signature missing")?)
            .map_err(|e| format!("signature not base64: {e}"))?;
        let bytes: [u8; 64] = raw.try_into().map_err(|_| "signature must be 64 bytes".to_string())?;
        let ok = verify_signature(key, Signature(bytes), cid.as_bytes().to_vec())
            .map_err(|e| format!("verify_signature: {e:?}"))?;
        if !ok { return Err(format!("signature by {agent} does not verify over signing_payload_cid")); }
    }
    if seen.len() < required { return Err(format!("quorum unmet: {} of {required} signatures", seen.len())); }
    Ok(())
}

fn validate_migrates_lineage(payload: &serde_json::Value) -> Result<(), String> {
    for field in ["action","role","from_dna_hash","to_dna_hash","release_cid","constitution_root","roster_cid","signing_payload_cid","signatures","evidence","window"] {
        if payload.get(field).is_none() { return Err(format!("migrates-lineage missing required field: {field}")); }
    }
    if payload["action"] != "migrates-lineage" { return Err("action field must equal 'migrates-lineage'".into()); }
    for f in ["from_dna_hash","to_dna_hash"] {
        let h = payload[f].as_str().unwrap_or("");
        if !h.starts_with("uhC0k") { return Err(format!("{f} must be a DNA hash (uhC0k…)")); }
    }
    if payload["from_dna_hash"] == payload["to_dna_hash"] { return Err("from_dna_hash and to_dna_hash must differ".into()); }
    let w = payload["window"].as_object().ok_or("window must be object")?;
    let (a, b) = (w.get("opens_at").and_then(|v| v.as_str()).ok_or("window.opens_at missing")?,
                  w.get("revert_until").and_then(|v| v.as_str()).ok_or("window.revert_until missing")?);
    if a >= b { return Err("window.opens_at must precede window.revert_until".into()); }
    validate_lineage_signatures(payload)
}

fn validate_sunsets_lineage(payload: &serde_json::Value) -> Result<(), String> {
    for field in ["action","role","from_dna_hash","to_dna_hash","migration_commitment_cid","signing_payload_cid","signatures","evidence","window"] {
        if payload.get(field).is_none() { return Err(format!("sunsets-lineage missing required field: {field}")); }
    }
    if payload["action"] != "sunsets-lineage" { return Err("action field must equal 'sunsets-lineage'".into()); }
    if payload["window"].get("sunsets_at").and_then(|v| v.as_str()).is_none() { return Err("window.sunsets_at missing".into()); }
    validate_lineage_signatures(payload)
}
```
   Add `base64 = "0.22"` (or the workspace's pinned version — grep other zomes) to the mishpat coordinator `Cargo.toml`. (RFC3339 strings compare lexicographically when both are `Z`-suffixed UTC — assert that with a `.ends_with('Z')` check on both.)
4. `cd elohim/holochain/dna/mishpat && just build`; sweettest → PASS; `cargo test --test mishpat` → all green.
5. Deliver to the mesh by rung 5 (coordinator-bundle release on `runtime:coordinators:mishpat:commons`), verify `/version` shows the new mishpat coordinator hash on all peers.
6. Commit: `git add elohim/holochain/dna/mishpat/zomes/mishpat elohim/holochain/tests/sweettest/src/tests/mishpat.rs && git commit -m "feat(mishpat): migrates-lineage / sunsets-lineage commitment arms with signature quorum (coordinator-only; epic Task 2)"`.

---

### Task 3: release manifest schema + packager flags

**Files:**
- Modify: `elohim/rakia/schemas/v1/release-manifest.schema.json` (`artifactClass` enum 34-41; `roleBinding` 166-195; `adoptionDiscipline` 295-321)
- Modify: `genesis/a2o/scripts/epr-release-package.ts` (options + manifest literal ~805-816)
- Test: `genesis/a2o/scripts/__tests__/epr-release-package.spec.ts` (append) — run with `pnpm test:unit`

**Interfaces:**
- Produces schema fields: `artifactClass: "happ-lineage"`; `roleBinding.migrateFrom` (`$ref dnaHash`), `roleBinding.lineage` (array of dnaHash, minItems 1, uniqueItems); `adoptionDiscipline.path: { commitmentCid: string (pattern `^uhCEk[A-Za-z0-9_-]{48}$`) }`; root `if artifactClass == "happ-lineage" then require roleBinding.migrateFrom + .lineage + adoptionDiscipline.path`.
- Packager flags: `--migrate-from <dnaHash>` (repeatable `role=hash`), `--lineage <dnaHash,...>`, `--path-commitment <cid>`.

- [ ] **Task 3 deliverable: a `happ-lineage` manifest validates against the schema only when it carries `migrateFrom`, `lineage` and `path.commitmentCid`; the packager emits it; unit test green.**

1. Failing unit test (append; the spec file already imports the packager's validate helper — reuse its pattern):

```ts
it('happ-lineage requires migrateFrom, lineage and path', () => {
  const m = baseManifest({ artifactClass: 'happ-lineage' });
  expect(validateManifest(m).ok).toBe(false);
  m.appliesTo.roles.node_registry.migrateFrom = 'uhC0kyvKwO2J5u3mf52tjASWe0ryhdpNYalrSeMGJODF3OpUxyeoH';
  m.appliesTo.roles.node_registry.lineage = [m.appliesTo.roles.node_registry.migrateFrom];
  m.adoptionDiscipline.path = { commitmentCid: 'uhCEk' + 'A'.repeat(48) };
  expect(validateManifest(m).ok).toBe(true);
});
```
2. Run `cd genesis/a2o && pnpm test:unit -- --test-name-pattern happ-lineage` → FAIL (enum rejects `happ-lineage`).
3. Schema edits — enum gains `"happ-lineage"` with the description suffix `; happ-lineage → the lineage crossing (install-beside + carry), spec 2026-09-03-holochain-evolution-epic-design §4`; `roleBinding.properties` gains:

```json
"migrateFrom": { "$ref": "#/$defs/dnaHash", "description": "The DNA hash this release migrates FROM — must equal the adopting peer's installed hash for the role (verify.rs positive branch). Required when artifactClass is happ-lineage." },
"lineage": { "type": "array", "minItems": 1, "uniqueItems": true, "items": { "$ref": "#/$defs/dnaHash" }, "description": "The ancestry the new DNA's properties declare; migrateFrom MUST be a member (enforced in Rust)." }
```
   `adoptionDiscipline.properties` gains `"path": { "type": "object", "required": ["commitmentCid"], "properties": { "commitmentCid": { "type": "string", "pattern": "^uhCEk[A-Za-z0-9_-]{48}$", "description": "The elohim's migrates-lineage commitment (entry hash) that notarizes this path." } } }`. Root-level: `"if": { "properties": { "artifactClass": { "const": "happ-lineage" } } }, "then": { "properties": { "adoptionDiscipline": { "required": ["soakSecs","attestationThreshold","canaryOrder","path"] } } }` (role-level requiredness is enforced in Rust in Task 4 because `roleBinding` is a `$defs` entry reused by every class).
4. Packager: add the three options to the usage block and `parseArgs`; in the manifest literal set `appliesTo.roles[role].migrateFrom / .lineage` when given and `adoptionDiscipline.path = { commitmentCid }` when `--path-commitment` is present.
5. Test → PASS. Run the schema codegen if the manifest type is generated (`grep -rn "release-manifest" elohim/sdk/schemas/scripts/codegen-ts.mjs`; if listed, `pnpm run schema:codegen:ts` and commit the generated file — note the Prettier oscillation memory: only the union wrap lines may flip).
6. Commit: `git add elohim/rakia/schemas/v1/release-manifest.schema.json genesis/a2o/scripts/epr-release-package.ts genesis/a2o/scripts/__tests__/epr-release-package.spec.ts && git commit -m "feat(release-manifest): happ-lineage class — migrateFrom, lineage, adoptionDiscipline.path (epic Task 3)"`.

---

### Task 4: verify — the positive lineage branch and `verify_path`

**Files:**
- Modify: `elohim/elohim-storage/src/services/release_adoption/mod.rs` (`ArtifactClass` 150-162; `RefusalReason` ~441; new `PathEvidence`)
- Modify: `elohim/elohim-storage/src/services/release_adoption/verify.rs` (`verify_envelope` 420-489; `verify` 709+; new `verify_path`)
- Test: same file's `#[cfg(test)] mod tests` (follow the existing `verify_envelope` tests' fixtures)

**Interfaces:**
- Produces:

```rust
// mod.rs
pub enum ArtifactClass { CoordinatorBundle, ConfigEpr, StorageBinary, HappBundle, HappLineage /* "happ-lineage" */ }
pub enum RefusalReason { …, PathNotNotarized, PathRevoked, QuorumUnmet, RootMismatch }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathEvidence {
    pub commitment_cid: String,
    pub state: String,              // "active" | "proposed" | …
    pub revoked_at: Option<String>,
    pub from_dna_hash: String,
    pub to_dna_hash: String,
    pub constitution_root: String,
    pub signatures: usize,
    pub required_signatures: usize,
}
// verify.rs
pub fn verify_path(manifest: &ReleaseManifest, installed: &InstalledReality, path: &Answer<PathEvidence>) -> Result<(), AdoptionRefusal>;
// VerifyInput gains: pub path: Answer<PathEvidence>   (caller-fetched; this module does no I/O)
```
- Consumes: `ReleaseManifest.applies_to.roles[role].migrate_from: Option<String>`, `.lineage: Option<Vec<String>>`, `adoption_discipline.path: Option<PathRef { commitment_cid }>` (add these fields to the Rust manifest struct where `ReleaseManifest` is defined — grep `struct ReleaseManifest`; serde `rename_all = "camelCase"` already).

- [ ] **Task 4 deliverable: `verify` accepts a `happ-lineage` release whose `migrateFrom` equals the installed hash and whose path evidence is `active` + unrevoked + quorum-met + same root, and refuses each failure with its named reason; `DnaLineageMismatch` unchanged for every other class; unit tests green.**

1. Failing tests (verify.rs `mod tests`; build fixtures with the module's existing `manifest_for(...)` / `installed_with(...)` helpers — read them first, do not invent new fixture builders):

```rust
#[test]
fn happ_lineage_positive_branch_accepts_migrate_from_equal_to_installed() {
    let mut m = manifest_for(ArtifactClass::HappLineage);
    m.applies_to.roles.get_mut("node_registry").unwrap().migrate_from = Some(INSTALLED_NR.into());
    m.applies_to.roles.get_mut("node_registry").unwrap().dna_hash = V2_NR.into();
    m.applies_to.roles.get_mut("node_registry").unwrap().lineage = Some(vec![INSTALLED_NR.into()]);
    let installed = installed_with("node_registry", INSTALLED_NR);
    assert!(verify_envelope(&m, &Answer::Present(installed)).is_ok());
}
#[test]
fn happ_lineage_refuses_when_migrate_from_is_not_installed() {
    let mut m = manifest_for(ArtifactClass::HappLineage);
    m.applies_to.roles.get_mut("node_registry").unwrap().migrate_from = Some("uhC0kOTHER".into());
    let r = verify_envelope(&m, &Answer::Present(installed_with("node_registry", INSTALLED_NR))).unwrap_err();
    assert_eq!(r.reason, RefusalReason::DnaLineageMismatch);
}
#[test]
fn coordinator_bundle_still_refuses_dna_line() { /* the existing test, unchanged, must still pass */ }
#[test]
fn verify_path_absent_is_path_not_notarized() {
    let m = lineage_manifest();
    let r = verify_path(&m, &installed_with("node_registry", INSTALLED_NR), &Answer::Absent).unwrap_err();
    assert_eq!(r.reason, RefusalReason::PathNotNotarized);
}
#[test]
fn verify_path_revoked_quorum_root() {
    let m = lineage_manifest();
    let inst = installed_with("node_registry", INSTALLED_NR);
    let mut ev = path_evidence_ok();
    ev.revoked_at = Some("2026-09-04T00:00:00Z".into());
    assert_eq!(verify_path(&m, &inst, &Answer::Present(ev.clone())).unwrap_err().reason, RefusalReason::PathRevoked);
    ev.revoked_at = None; ev.signatures = 0;
    assert_eq!(verify_path(&m, &inst, &Answer::Present(ev.clone())).unwrap_err().reason, RefusalReason::QuorumUnmet);
    ev.signatures = 1; ev.constitution_root = "bafyOTHER".into();
    assert_eq!(verify_path(&m, &inst, &Answer::Present(ev)).unwrap_err().reason, RefusalReason::RootMismatch);
}
```
2. `cargo test --lib services::release_adoption::verify` → FAIL (variants missing). Echo `EXIT=$?`.
3. Implement. In `verify_envelope` replace the mismatch block (478-489) with:

```rust
        if binding.dna_hash != installed_role.dna_hash {
            let crossing_ok = manifest.artifact_class == ArtifactClass::HappLineage
                && binding.migrate_from.as_deref() == Some(installed_role.dna_hash.as_str())
                && binding.lineage.as_ref().map_or(false, |l| l.iter().any(|h| h == &installed_role.dna_hash));
            if !crossing_ok {
                return Err(refuse(
                    RefusalReason::DnaLineageMismatch,
                    format!(
                        "role '{role}': release binds DNA {} but this peer runs {} — crossing the DNA line \
                         needs a happ-lineage release whose migrateFrom names the installed hash and whose \
                         lineage contains it (spec 2026-09-03-holochain-evolution-epic-design §4)",
                        binding.dna_hash, installed_role.dna_hash
                    ),
                ));
            }
        }
```
   `verify_path`:

```rust
pub fn verify_path(manifest: &ReleaseManifest, installed: &InstalledReality, path: &Answer<PathEvidence>) -> Result<(), AdoptionRefusal> {
    if manifest.artifact_class != ArtifactClass::HappLineage { return Ok(()); }
    let wanted = manifest.adoption_discipline.path.as_ref()
        .ok_or_else(|| refuse(RefusalReason::ManifestSchemaInvalid, "happ-lineage without adoptionDiscipline.path".into()))?;
    let ev = match path {
        Answer::Present(ev) => ev,
        Answer::Absent => return Err(refuse(RefusalReason::PathNotNotarized,
            format!("no migrates-lineage commitment {} is notarized on this peer's conductor", wanted.commitment_cid))),
        Answer::Unreachable => return Err(refuse(RefusalReason::ConductorUnavailable,
            "path evidence unreadable — establishes nothing in either direction (C4)".into())),
    };
    if ev.commitment_cid != wanted.commitment_cid {
        return Err(refuse(RefusalReason::PathNotNotarized, format!("commitment {} is not the manifest's path {}", ev.commitment_cid, wanted.commitment_cid)));
    }
    if ev.revoked_at.is_some() { return Err(refuse(RefusalReason::PathRevoked, format!("path {} revoked at {}", ev.commitment_cid, ev.revoked_at.clone().unwrap()))); }
    if ev.state != "active" { return Err(refuse(RefusalReason::PathNotNotarized, format!("path {} is {}, not active", ev.commitment_cid, ev.state))); }
    if ev.signatures < ev.required_signatures { return Err(refuse(RefusalReason::QuorumUnmet, format!("{} of {} signatures", ev.signatures, ev.required_signatures))); }
    for (role, binding) in &manifest.applies_to.roles {
        if let Some(inst) = installed.roles.get(role) {
            if ev.from_dna_hash != inst.dna_hash || ev.to_dna_hash != binding.dna_hash {
                return Err(refuse(RefusalReason::PathNotNotarized, format!("path {} names {}→{}, release is {}→{}", ev.commitment_cid, ev.from_dna_hash, ev.to_dna_hash, inst.dna_hash, binding.dna_hash)));
            }
            if let Some(root) = inst.constitution_root.as_deref() {
                if root != ev.constitution_root { return Err(refuse(RefusalReason::RootMismatch, format!("path root {} ≠ installed root {root}", ev.constitution_root))); }
            }
        }
    }
    Ok(())
}
```
   (`InstalledRole` gains `constitution_root: Option<String>`, read from the role's DNA properties by whatever populates `InstalledReality` — grep `InstalledRole {` constructors; `None` when the role declares no root, which is every role today.) Call `verify_path(&input.manifest, installed, &input.path)?` in `verify` right after `verify_lineage` and before `verify_threshold`.
4. Tests → PASS; `cargo test --lib services::release_adoption` all green. `cargo clippy -p elohim-storage -- -D warnings` clean.
5. Commit: `git add elohim/elohim-storage/src/services/release_adoption && git commit -m "feat(adoption): happ-lineage verify — positive DNA-line branch + verify_path (PathNotNotarized/PathRevoked/QuorumUnmet/RootMismatch) (epic Task 4)"`.

---

### Task 5: `install_lineage` — install a second app under the existing key, never uninstall

**Files:**
- Modify: `elohim/elohim-storage/src/happ_manager.rs` (beside `install_fresh` 768-790)
- Test: `elohim/elohim-storage/tests/happ_manager_install_lineage.rs` (new; needs a running conductor → mark `#[ignore]` with a `MESH_ADMIN_URL` env gate, run manually against the mesh; assert with `list_apps`)

**Interfaces:**
- Produces: `pub async fn install_lineage(admin_ws: &AdminWebsocket, happ_path: &Path, lineage_app_id: &str, agent_key: AgentPubKey, lineage: &[DnaHash], role: &str) -> anyhow::Result<()>` — installs `happ_path` as `lineage_app_id` for `agent_key`, with `roles_settings` carrying `DnaModifiersOpt { properties: {"lineage": [...]} }` for `role`, enables it; idempotent (already-installed → Ok). `pub fn lineage_app_id(base: &str, dna_hash: &str) -> String` → `format!("{base}@{}", &dna_hash[5..17])`.

- [ ] **Task 5 deliverable: `install_lineage` installs `elohim@<hash12>` beside `elohim` under the same agent key with the lineage property set, idempotently; verified against the household mesh with `list_apps` showing both apps for one key.**

1. Test (ignored unless `MESH_ADMIN_URL` is set):

```rust
#[tokio::test]
#[ignore = "needs a conductor: MESH_ADMIN_URL=ws://localhost:4445"]
async fn install_lineage_installs_beside_under_same_key() {
    let admin = AdminWebsocket::connect(std::env::var("MESH_ADMIN_URL").unwrap()).await.unwrap();
    let apps = admin.list_apps(None).await.unwrap();
    let elohim = apps.iter().find(|a| a.installed_app_id == "elohim").expect("elohim installed");
    let key = elohim.agent_pub_key.clone();
    let v2 = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../holochain/dna/node-registry/node-registry-v2.happ");
    let v1_hash = DnaHash::try_from("uhC0kyvKwO2J5u3mf52tjASWe0ryhdpNYalrSeMGJODF3OpUxyeoH").unwrap();
    elohim_storage::happ_manager::install_lineage(&admin, &v2, "elohim@test-lineage", key.clone(), &[v1_hash], "node_registry").await.unwrap();
    elohim_storage::happ_manager::install_lineage(&admin, &v2, "elohim@test-lineage", key.clone(), &[/*same*/], "node_registry").await.unwrap(); // idempotent
    let apps = admin.list_apps(None).await.unwrap();
    let side = apps.iter().find(|a| a.installed_app_id == "elohim@test-lineage").expect("side app");
    assert_eq!(side.agent_pub_key, key);
    admin.disable_app("elohim@test-lineage".into()).await.unwrap();
    admin.uninstall_app("elohim@test-lineage".into(), false).await.unwrap(); // test cleanup only — production never uninstalls
}
```
   (`node-registry-v2.happ` is produced by Task 9's `just build-witness-happ`; until then point `v2` at any one-role bundle.)
2. Implement:

```rust
pub fn lineage_app_id(base: &str, dna_hash: &str) -> String {
    let short: String = dna_hash.chars().skip(5).take(12).collect();
    format!("{base}@{short}")
}

pub async fn install_lineage(
    admin_ws: &AdminWebsocket, happ_path: &Path, lineage_app_id: &str,
    agent_key: AgentPubKey, lineage: &[DnaHash], role: &str,
) -> anyhow::Result<()> {
    let apps = admin_ws.list_apps(None).await.map_err(|e| anyhow::anyhow!("list_apps: {e}"))?;
    if apps.iter().any(|a| a.installed_app_id == lineage_app_id) {
        info!(app_id = lineage_app_id, "lineage app already installed — idempotent");
        return Ok(());
    }
    let props = serde_yaml::Value::Mapping({
        let mut m = serde_yaml::Mapping::new();
        m.insert("lineage".into(), serde_yaml::Value::Sequence(lineage.iter().map(|h| serde_yaml::Value::String(h.to_string())).collect()));
        m
    });
    let mut roles_settings = HashMap::new();
    roles_settings.insert(role.to_string(), RoleSettings::Provisioned {
        membrane_proof: None,
        modifiers: Some(DnaModifiersOpt { network_seed: None, properties: Some(YamlProperties::new(props)) }),
        init_properties: None,
    });
    let payload = InstallAppPayload {
        source: AppBundleSource::Path(happ_path.to_path_buf()),
        agent_key: Some(agent_key),
        installed_app_id: Some(lineage_app_id.to_string()),
        roles_settings: Some(roles_settings),
        network_seed: None,
        ignore_genesis_failure: false,
        restore_from_dht: false,
    };
    admin_ws.install_app(payload).await.map_err(|e| anyhow::anyhow!("install_app({lineage_app_id}) failed: {e}"))?;
    admin_ws.enable_app(lineage_app_id.to_string()).await.map_err(|e| anyhow::anyhow!("enable_app failed: {e}"))?;
    info!(app_id = lineage_app_id, "lineage app installed beside the base app under the existing key");
    Ok(())
}
```
   Never call `uninstall_for_reinstall` from this path. The network seed must equal the base role's seed so both cells are on the same network family — pass `network_seed: Some(<seed from the base app's cell modifiers>)` if `install_app`'s default does not inherit; read it from `app_info.cell_info[role]` Provisioned cell's `dna_modifiers.network_seed` in a helper `base_role_seed(admin, "elohim", role)`.
3. `cargo test -p elohim-storage --test happ_manager_install_lineage -- --ignored` against the mesh (matthew, `ws://localhost:4445`) → PASS; `list_apps` shows both.
4. Commit: `git add elohim/elohim-storage/src/happ_manager.rs elohim/elohim-storage/tests/happ_manager_install_lineage.rs && git commit -m "feat(happ_manager): install_lineage — a second app beside the base under the existing agent key, lineage property set, idempotent (epic Task 5)"`.

---

### Task 6: `LineageRoles` resolver + registry-backed `NodeRegistryApi`

**Files:**
- Create: `elohim/elohim-storage/src/lineage_roles.rs`
- Modify: `elohim/elohim-storage/src/hc_client_registry.rs` (`HcRegistryInputs` + `connect_role`/`connect_role_forever` 186-206, 253-280; `client()` 131)
- Modify: `elohim/elohim-storage/src/node_registry_api.rs` (39-98)
- Modify: `elohim/elohim-storage/src/main.rs` (where `NodeRegistryApi::connect` is called ~4112-4129; registry construction)
- Test: `elohim/elohim-storage/src/lineage_roles.rs` unit tests

**Interfaces:**
- Produces:

```rust
pub struct LineageRoles { inner: RwLock<BTreeMap<String, RoleLineage>> }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleLineage { pub reading_app_id: String, pub authoring_app_id: String }
impl LineageRoles {
    pub fn new(base_app_id: &str, roles: &[&str]) -> Self;            // every role: reading = authoring = base
    pub fn app_id_for(&self, role: &str) -> String;                   // authoring app id (base when unknown)
    pub fn open_window(&self, role: &str, lineage_app_id: &str);      // reading = base, authoring = lineage
    pub fn revert(&self, role: &str);                                 // authoring = base; reading = lineage (disabled cell)
    pub fn snapshot(&self) -> BTreeMap<String, RoleLineage>;          // for the passport
}
```
   `HcRegistryInputs` gains `pub lineage: Arc<LineageRoles>`; `connect_role`/`connect_role_forever` build `HcClientConfig { app_id: inputs.lineage.app_id_for(role), .. }`. `NodeRegistryApi` becomes `pub struct NodeRegistryApi { registry: Arc<HcClientRegistry> }` with `create_shard_assignment` doing `let client = self.registry.client("node_registry").ok_or(StorageError::Unavailable("node_registry client".into()))?;` — add `"node_registry"` to the registry's role set (`HcClientRegistry` gains `pub node_registry: RwLock<Option<Arc<HcClient>>>` and `SUPERVISED_ROLES` becomes 4).

- [ ] **Task 6 deliverable: every `call_zome` for a role routes to the role's AUTHORING app id via `LineageRoles`; node-registry is registry-backed; with no window open the mesh behaves byte-for-byte as before (`just gate elohim-storage` green, Act I mesh subset green).**

1. Unit tests in `lineage_roles.rs`:

```rust
#[test] fn defaults_to_base() { let l = LineageRoles::new("elohim", &["node_registry"]); assert_eq!(l.app_id_for("node_registry"), "elohim"); assert_eq!(l.app_id_for("unknown"), "elohim"); }
#[test] fn window_then_revert() { let l = LineageRoles::new("elohim", &["node_registry"]); l.open_window("node_registry", "elohim@EKiIscIk5BDd"); assert_eq!(l.app_id_for("node_registry"), "elohim@EKiIscIk5BDd"); l.revert("node_registry"); assert_eq!(l.app_id_for("node_registry"), "elohim"); assert_eq!(l.snapshot()["node_registry"].reading_app_id, "elohim@EKiIscIk5BDd"); }
```
2. `cargo test -p elohim-storage --lib lineage_roles` → FAIL (module missing) → implement (a `parking_lot::RwLock` or `std::sync::RwLock` map; ~50 lines) → PASS.
3. Wire the registry: add the field, the two `HcClientConfig` sites, the `node_registry` slot; make `NodeRegistryApi::new(registry: Arc<HcClientRegistry>)` and delete its private `hc_client`; update `main.rs` call sites (grep `NodeRegistryApi::connect`).
4. `cargo build` (mesh features) → EXIT=0; `just gate elohim-storage` → green; rebuild into the dev slot and `just mesh storage-restart matthew jessica james`; `just test mesh @concern:runtime-upgrade-propagation` (rung 5 stations must still pass — the regression that proves "byte-for-byte as before").
5. Commit: `git add elohim/elohim-storage/src/lineage_roles.rs elohim/elohim-storage/src/hc_client_registry.rs elohim/elohim-storage/src/node_registry_api.rs elohim/elohim-storage/src/main.rs elohim/elohim-storage/src/lib.rs && git commit -m "feat(storage): LineageRoles resolver — role→authoring app id; node-registry registry-backed (additive, HcClient untouched; epic Task 6)"`.

---

### Task 7: `HappLineageVehicle` — install beside, carry, attest

**Files:**
- Modify: `elohim/elohim-storage/src/services/release_adoption/apply.rs` (new vehicle beside `HappBundleVehicle` 423-499; registration in `main.rs` 5647-5652)
- Modify: `elohim/elohim-storage/src/services/release_adoption/mod.rs` (`AppliedReceipt` gains `carry: Option<CarryReceipt>`)
- Test: `elohim/elohim-storage/src/services/release_adoption/apply.rs` unit tests with a fake admin (the file's existing vehicle tests show the pattern) + the mesh (Station 3/4 in Task 11)

**Interfaces:**
- Consumes: Task 5 `install_lineage`, Task 6 `LineageRoles`, Task 9 zome `carry_from`.
- Produces:

```rust
pub struct HappLineageVehicle { admin: AdminWebsocket, base_app_id: String, lineage: Arc<LineageRoles>, registry: Arc<HcClientRegistry> }
impl ApplyVehicle for HappLineageVehicle {
    fn handles(&self) -> &'static [ArtifactClass] { &[ArtifactClass::HappLineage] }
    fn name(&self) -> &'static str { "happ-lineage" }
    async fn apply(&self, v: &VerifiedRelease) -> Result<AppliedReceipt, AdoptionRefusal>;
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CarryReceipt { pub role: String, pub carried: u32, pub v1_count: u32, pub digest: String, pub v1_digest: String, pub witness_hashes: Vec<String> }
```
   `apply`: (1) `sole_artifact(v, "happ-lineage")` → the v2 `.happ`; (2) for each role binding with `migrate_from`: `agent_key` = base app's key (from `app_info`), `lineage_app_id = happ_manager::lineage_app_id(&self.base_app_id, &binding.dna_hash)`; `install_lineage(...)`; (3) authorize + connect a client for the lineage app (`HcClient::connect(HcClientConfig { app_id: lineage_app_id, role: Some(role), .. })`) and read the v1 cell id from the base app's `app_info`; (4) loop `carry_from(CarryInput { v1_cell, cursor, limit: 32 })` on v2 until `next_cursor` is `None`, summing `carried`; (5) `self.lineage.open_window(role, &lineage_app_id)`; (6) receipt with `CarryReceipt`; refuse `ApplyFailed` with the zome error text on any failure, leaving the side app installed (never uninstall) and the window closed.

- [ ] **Task 7 deliverable: applying a verified `happ-lineage` release on a mesh peer installs v2 beside v1 under the same key, carries every v1 record with a witness, flips the role to authoring-on-v2, and returns a `CarryReceipt` whose `carried == v1_count` and `digest == v1_digest`.**

1. Unit test with the fake admin (mirror the `HappBundleVehicle` test): a manifest with `artifact_class: HappLineage` and no `migrate_from` on any role → `apply` returns `ApplyFailed` naming "no role declares migrateFrom"; and `handles()` contains only `HappLineage`.
2. Implement the vehicle; register in `main.rs` right after `CoordinatorBundleVehicle`:

```rust
                .with(std::sync::Arc::new(apply::HappLineageVehicle::new(
                    admin.clone(), args.app_id.clone(), lineage_roles.clone(), hc_registry.clone(),
                )))
```
3. `cargo test -p elohim-storage --lib services::release_adoption::apply` → PASS; `cargo build` → EXIT=0.
4. Rebuild into the dev slot; `storage-restart` all three peers. The live proof is Task 11's Station 3/4.
5. Commit: `git add elohim/elohim-storage/src/services/release_adoption elohim/elohim-storage/src/main.rs && git commit -m "feat(adoption): HappLineageVehicle — install beside, bounded carry, attest (epic Task 7)"`.

---

### Task 8: passport dual-cell view + `/admin/adoption` path refusals

**Files:**
- Modify: `elohim/elohim-storage/src/runtime_passport.rs` (`HappRolePassport` 74-80; `assemble_storage_passport` 119)
- Modify: `elohim/elohim-views/src/*.rs` where `RuntimeVersionResponse`/role views are ts-rs-anchored (grep `HappRolePassport`) + `elohim/sdk/schemas/v1/views/runtime-version.schema.json` if it exists
- Test: `elohim/elohim-storage/tests/schema_contract.rs` (existing harness catches drift)

**Interfaces:**
- Produces on each `HappRolePassport`: `pub lineage: Option<RoleLineageView { reading_app_id: String, authoring_app_id: String, reading_dna_hash: String, authoring_dna_hash: String, closed: bool }>` (present only while a window is open or after a sunset); `HappPassport` lists the base app AND every `elohim@…` app.

- [ ] **Task 8 deliverable: `GET /version` on a dual-celled peer shows the role with both cells, which one authors, under one key; `GET /admin/adoption` names `path not notarized` / `quorum unmet` / `root mismatch` / `lineage mismatch` verbatim as the refusal reason strings the story quotes.**

1. Add the schema field first (`runtime-version.schema.json`), then the Rust struct, then run `cargo test --test schema_contract` → PASS; `pnpm run schema:codegen:ts` (the Prettier oscillation memory applies).
2. Map `RefusalReason::{PathNotNotarized, PathRevoked, QuorumUnmet, RootMismatch, DnaLineageMismatch}` to the snake/kebab strings the adoption report already emits for other reasons (grep how `coordinator_lineage_mismatch` is rendered) → `path_not_notarized`, `path_revoked`, `quorum_unmet`, `root_mismatch`, `dna_lineage_mismatch`; the a2o steps normalise underscores to spaces.
3. Commit: `git add elohim/elohim-storage/src/runtime_passport.rs elohim/elohim-views elohim/sdk/schemas/v1/views elohim/sdk/storage-client-ts/src/generated && git commit -m "feat(passport): dual-cell role view + lineage refusal names (epic Task 8)"`.

---

### Task 9: v2 `carry_from` (cross-cell, bounded) + one-role bundle + CI feature variant

**Files:**
- Modify: `elohim/holochain/dna/node-registry/zomes/node_registry_coordinator/src/lib.rs` (feature-gated section)
- Modify: `elohim/holochain/dna/node-registry/justfile` (`build-witness-happ`)
- Create: `elohim/holochain/dna/node-registry/workdir-v2/happ.yaml` (one role `node_registry`, `path: ../node-registry-v2.dna`, `properties: { lineage: [] }` placeholder overridden at install)
- Modify: `elohim/holochain/dna/Jenkinsfile` (641-649: after the default pack, `cargo build --release --target wasm32-unknown-unknown --features lineage-witness && hc dna pack . -o node-registry-v2.dna`; archive it; print `DNA-HASH node_registry_v2 …` but do NOT append it to `$DNA_HASH_FILE` — the guard's baseline is the default build only)
- Test: `happ_lineage_migration.rs` — extend probe A with `carry_from` driving the loop and asserting `carried == 5`, `digest == export digest`

**Interfaces:**
- Produces (v2 coordinator, `#[cfg(feature = "lineage-witness")]`):

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CarryInput { pub v1_cell: CellId, pub cursor: Option<u32>, pub limit: u32 }
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CarryReceipt { pub carried: u32, pub next_cursor: Option<u32>, pub v1_digest: String, pub witness_hash: Option<ActionHash> }
#[hdk_extern] pub fn carry_from(input: CarryInput) -> ExternResult<CarryReceipt>
```
   Body: `call(CallTargetCell::OtherCell(input.v1_cell), "node_registry_coordinator", "export_records".into(), None, ExportInput { cursor: input.cursor, limit: input.limit.min(16) })` → for each record: if `record.action().author() == agent_info()?.agent_initial_pubkey` and the entry is present → `create_entry` natively with the SAME bytes (match on the entry type by `EntryType::App(def)` index → deserialize into the concrete struct → `create_entry(EntryTypes::X(x))`) and push `CarriedProof { action, signature, entry: None }`; else push `CarriedProof { action, signature, entry: Some(entry) }` (held-carry); then ONE `commit_witness(NotarizationWitness { lineage_dna_hash: <from properties.lineage[0]>, proofs })` for the page. The v1 cell must grant the call: v1's coordinator `init` creates an `Unrestricted` cap grant for `export_records` (add to v1's `init` in the unconditional section — hash-neutral) OR the vehicle issues a `grant_zome_call_capability` for the v2 cell's agent (same agent — an `Author`-assigned grant is implicit for same-agent calls in 0.7: verify in the probe; if not, add the grant in init).

- [ ] **Task 9 deliverable: `carry_from` on v2 pulls one bounded page from v1 across cells, re-creates own records natively (same entry hash), held-carries others, commits one witness per page, and returns cursor + digest; sweettest drives it to the end and matches the export digest; CI archives `node-registry-v2.dna`.**

1. Extend probe A: after the v1 record exists, call `carry_from` on `z2` with `v1_cell = old_cell.cell_id()`, `limit 16`; assert `carried == 1`, `next_cursor == None`, and `get_witnesses_for(v1_entry_hash)` returns one link; assert `v1_digest` equals `export_records(...).digest` from v1. Run → FAIL (fn missing).
2. Implement `carry_from` (feature block, appended). `just build && just build-witness`; default hash unchanged (assert in the shell: `[ "$(hc dna hash node-registry.dna)" = uhC0kyvKwO2J5u3mf52tjASWe0ryhdpNYalrSeMGJODF3OpUxyeoH ]`).
3. `build-witness-happ` recipe: `hc app pack workdir-v2 -o node-registry-v2.happ`.
4. Probe → PASS. Jenkinsfile edit; verify with `[build:dna]` on the next integrator push that the archive lists `node-registry-v2.dna` and the guard still passes (baseline unchanged).
5. Commit: `git add elohim/holochain/dna/node-registry elohim/holochain/dna/Jenkinsfile elohim/holochain/tests/sweettest/src/tests/happ_lineage_migration.rs && git commit -m "feat(node-registry v2): carry_from — bounded cross-cell carry with witnesses; one-role v2 bundle; CI builds the feature variant (epic Task 9)"`.

---

### Task 10: mesh fixture — v1 baseline, v2 candidate, the elohim's key, harness commitments

**Files:**
- Create: `genesis/a2o/steps/delivery/lineage-candidate.ts` (mirror `coordinator-candidate.ts`: resolves `node-registry-v2.happ`, computes both hashes with `hc dna hash`, writes the `happ-lineage` manifest via `epr-release-package.ts --artifact-class happ-lineage --migrate-from node_registry=<v1> --lineage <v1> --path-commitment <cid>`)
- Create: `genesis/a2o/steps/delivery/lineage-commitments.ts` (`notarizeMigration({from,to,releaseCid,revertUntil})`, `revoke(cid)`, `notarizeSunset(migrationCid)` — call mishpat `create_commitment` on matthew's conductor with the bootstrap steward's key via `@holochain/client` `AppWebsocket`, signing `signing_payload_cid` with `admin.sign`… (the harness holds the steward key: read how `release-ceremony.ts` signs delegations and reuse it))
- Modify: `app/elohim-app/scripts/hc-mesh.sh` `prologue`/reset: converge every peer to baseline before and after a run — disable+uninstall any `elohim@*` side app, reset `LineageRoles` via `POST /admin/lineage/reset` (add that admin route in Task 6's registry: it only calls `LineageRoles::revert` for every role and disables side apps)

- [ ] **Task 10 deliverable: `just test mesh @concern:happ-lineage-migration --name '^Station 1'` runs against a mesh where v1 is the baseline, the v2 candidate is minted from `node-registry-v2.happ`, and the harness can notarize/revoke commitments under the steward's key; the fixture converges the fleet to baseline before and after every run (rung 5's lesson, 8181d60a8).**

1. Write `lineage-candidate.ts` and `lineage-commitments.ts` with unit tests under `genesis/a2o/scripts/__tests__/` for the manifest shape (no mesh needed) — run `pnpm test:unit`.
2. Add the `/admin/lineage/reset` route (storage, Task 6's file) and the fixture reset in `Before`/`AfterAll` of the steps file (Task 11).
3. Commit: `git add genesis/a2o/steps/delivery/lineage-candidate.ts genesis/a2o/steps/delivery/lineage-commitments.ts genesis/a2o/scripts/__tests__ app/elohim-app/scripts/hc-mesh.sh elohim/elohim-storage/src/http.rs && git commit -m "test(a2o): lineage fixture — v2 candidate, steward-key commitments, baseline convergence (epic Task 10)"`.

---

### Task 11: Stations 1–5, 9, 10 as cucumber steps

**Files:**
- Create: `genesis/a2o/steps/delivery/happ-lineage-migration.steps.ts` (compose `getRaw`/`postRaw`, `release-ceremony.ts` via `spawnSync` exactly as `runtime-upgrade-propagation.steps.ts` does, plus Task 10's helpers)
- Modify: `elohim/holochain/.epr-meta/happ-lineage-migration.habit.md` (DELTA per station flip) + `python3 .claude/scripts/habits-project.py`

**Interfaces:**
- Consumes: `/admin/adoption` report shape (`AdoptionReport` in the rung-5 steps), `/version` passport with Task 8's `lineage` view, Task 10's helpers.
- Station → assertion map (the story is the spec; quote its reason strings):
  - S1: publish the lineage manifest → every peer's `/admin/adoption` shows the release `admissible` (verify passed envelope); publish a v2 manifest WITHOUT `migrateFrom` → every peer names `dna_lineage_mismatch` ("lineage mismatch").
  - S2: earned but no commitment → `path_not_notarized`; notarize → `adoptable`; assert no peer's adoption log contains a prompt string.
  - S3: canary james → `/version` lineage view `reading=elohim`, `authoring=elohim@…`, same key; conductor PID unchanged (`GET /db/p2p/conductor-diagnostics` pid field as rung 5 reads it).
  - S4: `CarryReceipt` from `/admin/adoption` (`carried == v1_count`, `digest == v1_digest`); for each carried record `GET /db/node-registry/...` shows `notarized_*` from the proof (Task 8b if the projection is not there yet: assert via the witness link count through the storage zome-call proxy).
  - S5: jessica's record readable on james's v2 via `get_witnesses_for` with `courier == james`; jessica's chain length unchanged.
  - S9/S10: harness commits forged witness / under-quorum commitment → the named refusals.

- [ ] **Task 11 deliverable: Stations 1, 2, 3, 4, 5, 9, 10 pass on the household mesh in one run (`cucumber-stations-mvp-r<N>.{log,json}` under `genesis/a2o/reports/release-ceremony/<date>/`), the habit atom carries the receipt id, and the epic's §11.2 board flips those rows green.**

1. Skeleton: `pnpm generate:skeletons features/delivery/happ-lineage-migration.feature` → fill each step; run with the one-feature cucumber config trick from the rung-5 delta (`--config <empty> --name '^Station 1'`), one station at a time, red before green.
2. After each green station: DELTA line in the habit atom + re-project + ledger line in the epic.
3. Commit per station: `git add genesis/a2o/steps/delivery/happ-lineage-migration.steps.ts elohim/holochain/.epr-meta/happ-lineage-migration.habit.md genesis/manifests/habits.yaml genesis/docs/superpowers/specs/2026-09-03-holochain-evolution-epic-design.md && git commit -m "story(epic): Station N green on the mesh (receipt …)"`.

---

### Task 12: Station 6 — the bridge sweep (trailing)

**Files:**
- Create: `elohim/elohim-storage/src/services/lineage_bridge.rs` (a ticker on dual-celled peers: every `LINEAGE_SWEEP_SECS` (30) call v1 `export_records` from the reading cell with the last cursor, held-carry new records into v2 via `carry_from`, record `backward_carry: "unavailable"` when v1 lacks the witness type — detected by `v1 app_info` DNA hash ∉ any lineage with the witness)
- Modify: passport (`lineage.backward_carry: "unavailable" | "available"`), main.rs (spawn the sweep when a window opens)

- [ ] **Task 12 deliverable: a record jessica authors on v1 during the window is readable on james's v2 within one sweep interval, held with jessica's signature; the passport reports backward carry unavailable; Station 6 green.**

1. Unit test the cursor bookkeeping (pure fn `next_sweep(state, page) -> state`) then wire the ticker; measure on the mesh with Station 6.
2. Commit as the Task 11 pattern.

---

### Task 13: Station 7 — revert before sunset (trailing)

**Files:**
- Modify: `HappLineageVehicle` — on a revert manifest (rung 5's re-election of the prior head) for a role with an open window: `self.lineage.revert(role)`; `admin.disable_app(lineage_app_id)`; never uninstall; re-author pending v2 records on v1 natively (`export_records` from the v2 cell → `create_entry` on v1 for own records; count `pending` for the rest) and report `pending` in the passport.

- [ ] **Task 13 deliverable: revoking the migration commitment inside its horizon returns every peer to v1 authoring, leaves v2 cells disabled and intact, re-authors james's window-time v2 record on v1 with the same entry hash, reports the rest as pending; Station 7 green.**

---

### Task 14: Station 8 — sunset (trailing; after Probe B2's verdict is in the epic)

**Files:**
- Integrity (feature block, appended): the `after close` rule — a `CarriedProof` whose `action.header.author` has a carried `CloseChain` in the same witness batch or an earlier witness for that lineage, and whose `action_seq` > that close's `action_seq`, is `Invalid("after close")`. Implementation: the witness carrying the close is committed FIRST with `proofs: [CloseChain proof]`; the validator on later witnesses looks up the close via a link `AuthorToClose` (entry-hash anchor of the author key → the close witness) with `must_get_valid_record`; absence of a close = no rule (pre-sunset carries are unaffected).
- Coordinator: `seal_close(v1_cell) -> (close_hash, open_hash)`: `close_chain_for(v2)` on v1 via cross-cell call, `open_chain_from((v1_hash, close_hash))` on v2, then `commit_witness` with the close proof and the `AuthorToClose` link.
- Vehicle: on a `sunsets-lineage` commitment (Task 2) → `seal_close` per role, disable v1 cell, `LineageRoles::sunset(role)` (closed = true).

- [ ] **Task 14 deliverable: after fleet convergence and a notarized sunset, every peer seals close→open in order, v1 stays readable, the harness's post-close write on v1 is accepted by the conductor but its carried proof is refused by v2 as "after close" on every peer, and a post-sunset revocation changes nothing; Station 8 green.**

---

### Task 15 (Lane B spike, claimable, after Task 9): fork host fn `must_get_record_from_lineage`

**Files:** `elohim/holochain-conductor` branch `elohim-0.7-lineage-spike` (fork): a validation host fn `must_get_record_from_lineage(dna_hash, action_hash)` that dereferences a locally-installed lineage cell's store; measured question: can a foreign lineage be made visible to validation at all? Report in the epic §9 as a dated delta; no merge to the fork's main until the epic decides B2 (spec §9 decision rule).

- [ ] **Task 15 deliverable: a measured yes/no with the diff size, recorded in the epic §9 and §11.**

---

### Tasks 16–20: minted from the live measurement (2026-09-05, r15/r16 — see epic §11.4)

The mesh run exposed five gaps none of the fifteen tasks named. Each is one gap item; each closes a named station or a named trust boundary.

### Task 16: roster check in `verify_path` (Station 10, arm 1)

**Files:**
- Modify: `elohim/elohim-storage/src/services/release_adoption/path_evidence.rs` (fetch the roster commitment named by `payload.roster_cid` through the peer's OWN conductor — `get_commitment` — and carry `roster_members: Vec<String>` + the signers' agent strings on `PathEvidence`)
- Modify: `elohim/elohim-storage/src/services/release_adoption/verify.rs` (`verify_path`: after the count check, refuse `QuorumUnmet` with detail `signer <agent> is not on roster <cid>` when any counted signer ∉ roster members; an UNREADABLE roster is `Unreachable` → `conductor_unavailable`, never a pass)
- Test: unit tests with an off-roster signer; a2o Station 10 arm 1 goes green on the mesh.

- [ ] **Task 16 deliverable: a commitment signed by an agent that is not on the earned roster is refused `quorum_unmet` on every peer; Station 10 arm 1 green.**

### Task 17: `constitution_root` reaches the passport (Station 10, arm 2)

**Files:**
- Modify: `elohim/elohim-storage/src/runtime_passport.rs` (per role: read the installed cell's modifiers `properties` via admin `app_info` → decode `LineageProperties.constitution_root` → `HappRolePassport.constitution_root: Option<String>`)
- Modify: `elohim/elohim-storage/src/services/release_adoption/verify.rs:136-138` (`from_happ_passport` carries it, so `RootMismatch` can fire)
- Test: unit test the properties decode; Station 10 arm 2 green.

- [ ] **Task 17 deliverable: a path whose `constitution_root` differs from the installed role's is refused `root_mismatch`; Station 10 green.**

### Task 18: `export_held_records` — the held view a sweep can carry (Station 5; feeds Task 12)

**Files:**
- Modify: `elohim/holochain/dna/node-registry/zomes/node_registry_coordinator/src/lib.rs` (UNCONDITIONAL, coordinator-only, hash-neutral: `export_held_records(ExportHeldInput { agent, cursor, limit }) -> ExportPage` over `get_agent_activity(agent, ChainQueryFilter::new(), ActivityRequest::Full)` + DHT `get` per record, same cursor/digest/`total` discipline as `export_records`, cap 64; and `known_agents() -> Vec<AgentPubKey>` = authors of `NodeRegistration` entries the cell can read)
- Modify: `carry_from` (gated) accepts `source: Own | Held(agent)` and calls the matching v1 extern.
- Test: sweettest — bob's v1 record held-carried into alice's v2 with `entry: Some`, one witness, `self_carried == 0`.

- [ ] **Task 18 deliverable: a neighbour's v1 record is carried into v2 as a held-carry with the courier's witness; default DNA hash unchanged.**

### Task 19: path lifecycle from DHT truth (Station 7's revocation; trust boundary G4)

**Files:**
- Modify: `elohim/elohim-storage/src/services/release_adoption/path_evidence.rs` (`state`/`revoked_at` from `mishpat::get_commitment_state_links(commitment_cid)` through the peer's own conductor — the CommitmentByState links — with the local projection row only as a cache; no links and no row → `proposed`; unreadable → `Unreachable`)
- Modify: `elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs` (coordinator-only: `create_lineage_commitment` also authors the `active` state link when the validator accepted the quorum, so activation is DHT-visible; `revoke` authors `revoked`)
- Test: Station 7 — a revocation authored on matthew is read as `path_revoked` on james within one sweep.

- [ ] **Task 19 deliverable: a revoked path is refused `path_revoked` on every peer, not only the author's; Station 7's revocation half green.**

### Task 20: carry idempotency (G5)

**Files:**
- Modify: `elohim/holochain/dna/node-registry/zomes/node_registry_coordinator/src/lib.rs` (gated `carry_from`: before `create_entry`, `query` this chain for the entry hash; if present, count it as already carried and author no second witness for it)
- Test: sweettest — a second `carry_from` at cursor 0 reports the same `carried`, `self_carried == 0`, and `get_witnesses_for` still returns exactly one link.

- [ ] **Task 20 deliverable: a retried carry commits no duplicate entries or witnesses; the "one witness per carried entry" property holds under retry.**

---

### Tasks 21–23: the sunset-hardening crossing (integrity-side; ONE hash-moving DNA-lineage event, minted 2026-09-05 from the reviews)

The rehearsal measures with these open and says so in every receipt. They are integrity changes, so they ride one deliberate crossing (mishpat + node-registry v2), never a hot-swap.

### Task 21: G1 — the roster bound to the elohim (mishpat integrity)

**Files:**
- Modify: `elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs` (`commitment_action_requirements` gains the lineage arms: `migrates-lineage` / `sunsets-lineage` / lineage-targeting `revokes-commitment` require every signature in `signatures` to verify over `signing_payload_cid`, every signer ∈ the roster named by `roster_cid` — `must_get_entry` of that commitment, members read from its body — and `roster.constitution_root == payload.constitution_root`; a `declares-roster` arm whose author must be the progenitor named by the DNA's `LineageProperties` or a member of the previous roster (chain to the root))
- Test: sweettest — an off-roster signer's lineage commitment is REJECTED by validation on a receiving peer, not only refused by the author's coordinator.

- [ ] **Task 21 deliverable: a receiving peer's integrity validation rejects a lineage commitment whose signers are not on a roster that chains to the declared root; storage's `verify_path` roster check becomes a coherence check over a validated fact.**

### Task 22: G7 — state links authored only by their rightful author (mishpat integrity)

**Files:**
- Modify: `elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs` (`validate_commitment_by_state_tag` → full `CommitmentByState` link validation: `active|t` only when the link author == the anchor commitment's author; `revoked|t` only when the link author == the author of a validated `revokes-commitment` naming that anchor — `must_get_entry` + tag carries the revocation's entry hash)
- Modify: coordinator `create_commitment_state_link` (tag carries the revocation entry hash for `revoked`) — coordinator-only, rides the same crossing.
- Test: sweettest — a stranger's `revoked|t` link on a lineage anchor is rejected; the honest revocation's link is accepted.

- [ ] **Task 22 deliverable: no agent can revoke or activate a path it did not author or lawfully revoke; storage's lifecycle read becomes a verified fact.**

### Task 23: G6 — the after-close fence reaches every courier (node-registry v2 integrity + coordinator)

**Files:**
- Modify: coordinator `carry_from` / `seal_close`: after a seal, every witness for that lineage carries the close proof (`CarriedProof` of the `CloseChain`) as its first proof, so a courier that never sealed still carries the close; `export_held_records` surfaces the neighbour's `CloseChain` when present.
- Modify: integrity `refuse_carried_after_close`: a `CarriedProof` whose author has a `CloseChain` toward this DNA in the SAME batch (now guaranteed present after the seal) and whose `action_seq` is past it is refused — no chain walk needed for the cross-courier case.
- Test: sweettest — jessica's post-close v1 write, carried by james (who never sealed), is refused by v2 as after-close.

- [ ] **Task 23 deliverable: Station 8's "every peer" claim holds — a post-close v1 fact is refused by v2 regardless of which courier carries it.**

---

### Tasks 24–25: growth shapes found by reading the landed code (2026-09-05; G8 coordinator-side now, G9 rides the crossing)

### Task 24: G8 — the export walk is linear, the digest computed once per walk

**Files:**
- Modify: `elohim/holochain/dna/node-registry/zomes/node_registry_coordinator/src/lib.rs` (unconditional): `export_records` / `export_held_records` take an opaque `cursor` that carries `(position, chain_head_hash)`; the first page computes the digest and total ONCE and returns them with the cursor; later pages verify the head has not moved (else refuse `chain moved — restart at 0`) and walk only their window (`query` with a `sequence_range`); `entry_already_witnessed` batches the witness lookups per page. Storage's `fold_carry` and `next_sweep` already restart on a digest change — unchanged.
- Test: sweettest — a 200-record chain exports in pages with one digest computation; a mid-walk write makes the next page refuse with the named reason.

- [ ] **Task 24 deliverable: carrying N records costs O(N) record loads, not O(N²/64); default `node-registry.dna` hash unchanged.**

### Task 25: G9 — bounded after-close validation (rides the Tasks 21–23 crossing)

**Files:**
- Modify: integrity `refuse_carried_after_close` (gated block): each witness carries `close_seq: Option<u32>` for its lineage; validation compares a carried proof's `action_seq` against the close named in the SAME witness (present after the seal per Task 23) and walks the carrier chain only when the witness predates the seal, bounded by `ChainFilter::until_hash` of the last witness.
- Test: sweettest — validation cost per witness is constant after the seal (assert the walk length via a counter exposed in test builds).

- [ ] **Task 25 deliverable: after-close validation is O(1) per witness once the close travels in the witness; hash-moving, lands with Tasks 21–23.**

---

### Task 26: G10 — entry types matched by NAME across lineage ends, never by index

**Files:**
- Modify: `elohim/holochain/dna/node-registry/zomes/node_registry_coordinator/src/lib.rs` (unconditional: `ExportPage` gains `type_names: Vec<String>` positional with `records` (`#[serde(default)]`); `carry_from` and `readopt_from` resolve the concrete `EntryTypes` variant by the carried NAME through this DNA's `zome_types.entries`, refusing a page whose name is unknown here with a named Guest error — the index is never trusted across DNAs).
- Test: sweettest — a page whose `type_names` is deliberately permuted is refused; the honest page still carries; default hash unchanged.

- [ ] **Task 26 deliverable: a carry or re-adoption between lineage ends can never re-create a record as the wrong type; the type travels by name.**

---

### Task 27: G11 — a migration's successors are discoverable by every peer (mishpat coordinator + storage read)

**Files:**
- Modify: `elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs` (coordinator-only, hash-neutral): when `create_lineage_commitment` / `create_commitment` accepts a `sunsets-lineage` (or a `revokes-commitment` naming a lineage action), author a `MigrationToSuccessor` link from the MIGRATION commitment's anchor (its entry hash) to the new commitment, tag `sunset|<cid>` / `revoked|<cid>` — reuse an existing link type the integrity zome already declares (read `mishpat_integrity` for one with a free tag shape; if none fits, this half is hash-moving and joins Tasks 21–23 — say which); add `get_lineage_successors(migration_cid) -> Vec<Link>`.
- Modify: `elohim/elohim-storage/src/services/release_adoption/path_evidence.rs` (`fetch_sunset_evidence_for` reads the successors through the peer's own conductor instead of the author-scoped query; unreadable → `Unreachable`).
- Test: mishpat sweettest — bob's peer finds alice's sunset for alice's migration; storage unit test for the read.

- [ ] **Task 27 deliverable: a non-authoring peer discovers the sunset that names its migration; the sunset arm is live on the canary; Station 8 can seal there.**

---

### Task 28: the resume pin travels through the bridge and the vehicle (G8, storage half)

**Files:**
- Modify: `elohim/elohim-storage/src/services/release_adoption/carry.rs` (`CarryInput.resume: Option<ExportResume>` and `CarryReceipt.resume` mirrors of the zome's additive fields — read the zome's `ExportResume { head, digest, total, observed_head, cursor_seq }` and mirror byte-for-byte; `fold_carry` threads the page's `resume` into the next `CarryInput`; the named refusal `chain moved — restart at 0` restarts the walk at cursor 0 with no resume, exactly as a digest change does), `services/lineage_bridge.rs` (`AgentSweep` keeps the last `resume` per neighbour; `next_sweep` clears it on restart; the passport sweep view shows `scanned` so R1's metric is visible per neighbour).
- Test: unit tests — the fold threads `resume`; a `chain moved` refusal restarts at 0; the sweep state round-trips `resume`; the `scanned` number reaches the view.

- [ ] **Task 28 deliverable: on the mesh a multi-page carry's second and later pages report `scanned` ≪ the chain length (R1's metric), because the pin actually reaches the zome.**

---

## Self-review (done at authoring, 2026-09-04)

- **Spec coverage:** §2 record → Tasks 9, 14 (the kernel exists: `e233bb4f7`); §3 gate entities → Tasks 2, 3, 4; §4 path steps 1–5 → Tasks 3, 4, 7, 13, 14; §4.1 root binding → Task 4 (`RootMismatch`, `constitution_root` on `InstalledRole`), quorum → Task 2 (MVP 1-of-1; roster chain = ledger item 2b); §5 bridge → Task 12; §5.1 many versions → out of MVP (Stations 11–13 later); §6 seam table → one task per row; §9 → Task 15; §11 → every task's ledger line.
- **Placeholder scan:** none of the forbidden phrases; every code step shows code; the two "grep first" instructions name what to grep and why (fixture builders, generated-type registration) rather than deferring the work.
- **Type consistency:** `ArtifactClass::HappLineage` ↔ `"happ-lineage"` (kebab); `PathEvidence` fields used identically in Task 4 tests and body; `CarryReceipt` (storage, Task 7) carries `role/carried/v1_count/digest/v1_digest/witness_hashes`, the zome's `CarryReceipt` (Task 9) is per-page `{carried,next_cursor,v1_digest,witness_hash}` — the vehicle folds pages into the storage receipt; `LineageRoles` API names match across Tasks 6, 7, 13, 14; `lineage_app_id(base, hash)` is the one app-id minting fn.
