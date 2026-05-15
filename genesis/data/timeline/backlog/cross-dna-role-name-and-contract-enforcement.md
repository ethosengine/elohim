# Cross-DNA role-name + contract-enforcement refactor

**Status:** Backlog
**Surfaced by:** Recovery M4 sprint (2026-05-15) — Task 3 `submit_intimate_witness` cross-DNA bridge
**NOT a blocker for Recovery M4** — bridges are semantically correct (target the SDK contract name); the recovery sprint can land as-is. Constant extraction folds into Layer 1's PR after.

---

## 1. Finding

The repo is in **Scenario B (with a C-flavor naming artifact)**: pillar bridges target an SDK role-name (`"elohim"`) that does not exist in the production hApp manifest. SweetTest masks the defect by installing the same DNA under role name `"elohim"` at test setup time.

### Concrete evidence

**Pillar bridges call role `"elohim"`** — 7 sites across 3 pillars:
- `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs:689,723,757,794` — `CallTargetCell::OtherRole("elohim".into())`
- `elohim/holochain/dna/mishpat/zomes/mishpat/src/lib.rs:1663,1695` — `CallTargetCell::OtherRole("elohim".into())`
- `elohim/holochain/dna/infrastructure/zomes/infrastructure/src/lib.rs:1091` — `CallTargetCell::OtherRole("elohim".into())`

**Production hApp manifest declares NO `elohim` role** — `elohim/holochain/dna/elohim/workdir/happ.yaml:24`:
```yaml
roles:
  - name: lamad
    dna:
      path: "lamad.dna"
```
Roles declared: `lamad`, `infrastructure`, `imagodei`, `mishpat`, `node_registry`. No `elohim`.

**The DNA bundle is named `lamad`** — `elohim/holochain/dna/elohim/dna.yaml:20`:
```yaml
name: lamad
```
The consolidated coordinator (`issue_attestation`, `propose_governance_action`, `get_content_by_id`) actually lives in `dna/elohim/zomes/content_store/src/lib.rs` (line 3028, 11992; `governance_action.rs:67`). So the directory was renamed `elohim/` (correct, post-consolidation) but the bundle `name:` field in dna.yaml and the role in happ.yaml were never updated — they still say `lamad` (legacy LMS-era name).

**SweetTest hand-installs under the contract-correct name** — `elohim/holochain/tests/sweettest/src/tests/recovery_m3.rs:199-205`:
```rust
let elohim_dna = load_dna("lamad", &network_seed("lamad"), Some(agent.clone())).await?;
// ...
let dnas_with_roles: Vec<(RoleName, DnaFile)> = vec![
    ("imagodei".into(), imagodei_dna),
    ("elohim".into(), elohim_dna),  // SweetTest invents the role name production lacks
];
```
The comment at line 167 explicitly notes the bridge needs role `"elohim"` to resolve.

**The elohim DNA already models the right pattern outbound** — `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs:866`:
```rust
const IMAGODEI_ROLE: &str = "imagodei";
```
…used at lines 904, 940, 976. Inbound (pillar → elohim) does not yet follow this pattern.

### Why it works in tests but is wrong for production

SweetTest provides explicit `(RoleName, DnaFile)` tuples and is free to label DNAs however the test wants. The production hApp install reads `workdir/happ.yaml` and binds roles to the names declared there. Today, a production `elohim.happ` install will create a cell under role `"lamad"`, and a runtime `OtherRole("elohim")` bridge call will fail to resolve — there is no such role in the installed app.

This means **any pillar bridge call to the consolidated coordinator is broken in production hApp installs today**. Recovery M4 Task 3's `submit_intimate_witness` is the first surfaced caller, but mishpat and infrastructure are equally affected the moment their bridges execute on a real conductor (vs SweetTest).

## 2. Risk

- **Production correctness:** all 7 cross-DNA bridge calls fail at runtime in production hApp installs. SweetTest gives false-green CI confidence.
- **Refactor amplification:** every literal `"elohim"` / `"lamad"` / `"imagodei"` string is invisible to the compiler — a rename requires grep, not type-check. Future role consolidations or renames cannot be done safely.
- **Boundary erosion:** with no enforcement, a pillar could (today) bridge to any role name string, including invented or typo'd names. The mistake is silent until a runtime failure.

## 3. Proposed refactor (three layers)

### Layer 1 — immediate, small PR (~half day)

1. **Fix the production manifest first** — update `elohim/holochain/dna/elohim/workdir/happ.yaml` role name `lamad` → `elohim`, and the bundled path `lamad.dna` → `elohim.dna`. Update `elohim/holochain/dna/elohim/dna.yaml` `name: lamad` → `name: elohim`. Update the DNA build pipeline (Jenkinsfile + any pack scripts) to emit `elohim.dna` instead of `lamad.dna`. Bump network_seed (`elohim_lamad_alpha` → `elohim_alpha`) since alpha = resettable per `dna.yaml` hygiene notes.

2. **Introduce a shared role-name constants module** in `elohim/sdk/domains/elohim/types/` (crate already exists per `ls elohim/sdk/domains/elohim/types/`). Define:
   ```rust
   pub const ELOHIM_DNA_ROLE: &str = "elohim";
   pub const IMAGODEI_DNA_ROLE: &str = "imagodei";
   pub const MISHPAT_DNA_ROLE: &str = "mishpat";
   pub const INFRASTRUCTURE_DNA_ROLE: &str = "infrastructure";
   pub const NODE_REGISTRY_DNA_ROLE: &str = "node_registry";
   ```
3. **Replace every hardcoded role literal** in the 8 call sites (7 inbound + 1 outbound `IMAGODEI_ROLE` in elohim itself) with the constant. Also replace the SweetTest helper's `("elohim".into(), ...)` tuple literals with the same constant — tests should consume the contract, not invent names.

4. **Migrate the harness** — update `recovery_m3.rs:199` (`load_dna("lamad", ...)`) and the eight other sweettest sites with `const DNA: &str = "lamad"` to read from a shared constant tied to the dna.yaml name.

### Layer 2 — contract-enforcement (~1–3 days, small sprint)

1. **Manifest validator harness** — extend the existing schema-driven manifest validator at `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json` + the codegen pipeline at `elohim/sdk/schemas/scripts/codegen-ts.mjs`. New checks:
   - **Inbound boundary:** a pillar zome creating any entry type the contract reserves for elohim DNA (Content, Attestation, Agreement, Commitment, etc.) is a hard validation error. (Today this is clean per `grep EntryTypes::Content` in pillar dirs — preserve it via enforcement.)
   - **Manifest discriminator allowlist:** a pillar manifest declaring `attestation:*` / `governance-action:*` discriminators whose prefix isn't registered in elohim DNA's coordinator is a hard validation error. (Aligns with `feedback_schema_first_ioc.md` — schemas as contract.)
   - **Bridge-call allowlist:** any literal `OtherRole("...")` string in any zome source must match a known SDK role from the constants module — flagged via the AST walker added in step 3, or a clippy-allow grep harness in pre-push.

2. **Pillar manifest already declares what it implements** — extend each pillar's `manifest.json` (`elohim/sdk/domains/{imagodei,lamad,mishpat,infrastructure}/manifest.json`) with an explicit `implements_contract: { dna_role: "elohim", contract_version: "v1" }` block. Validator confirms every bridge call's target role appears in some pillar's `implements_contract.dna_role` field — boundary leaks fail at validate time, not runtime.

3. **Extend the p2p-design-gate hook** at `/projects/elohim/.claude/hooks/p2p-plan-audit.py` (or add a sibling `cross-dna-boundary-audit.py`) to scan zome source for:
   - Hardcoded role string literals in `OtherRole(...)` — fail with pointer to constants module.
   - Direct entry creation of contract-reserved entry types from pillar zomes.

### Layer 3 — large, optional (~multi-day, defer until friction warrants)

Split `dna/elohim/` into two DNAs / build directories: one for the consolidated SDK protocol-core (Content, Attestation, GovernanceAction, EPR primitives), one for LMS-specific lamad zomes (learning-path traversal, mastery sessions, etc. — to the extent any still live in the elohim DNA). The directory rename half-happened during the attestation-consolidation sprint (see `.claude/memory/project_attestation_consolidation_sprint_state.md`); Layer 3 finishes it. Only worth doing if Layer 1 + 2 surfaces ongoing friction from the dual-purpose DNA.

## 4. Scope estimate per layer

| Layer | Effort | Trigger |
|-------|--------|---------|
| 1 — constant + manifest rename | ~half day, single PR | Schedule immediately after Recovery M4 lands; bundles cleanly with the M4 cleanup tail |
| 2 — contract-enforcement validators | ~1–3 days, small sprint | After Layer 1; pairs with any future cross-DNA bridge work |
| 3 — split DNAs | multi-day, multi-PR | Defer; revisit if pillar-vs-protocol coupling on lamad DNA becomes painful |

## 5. Linked memories / sprints

- `.claude/memory/project_elohim_dna_as_sdk_boundary.md` — the architectural framing: elohim DNA = SDK contract, pillars = implementations
- `.claude/memory/project_attestation_consolidation_sprint_state.md` — why the naming is mid-consolidation (Stage A landed; B–G pending)
- `genesis/docs/plans/2026-05-15-recovery-m4-completion-shamir-optional-kickoff-prompt.md` — sprint that surfaced the cross-DNA bridge
- `.claude/memory/feedback_schema_first_ioc.md` — schemas as contract; manifest validator is the natural enforcement seat
- `.claude/memory/project_doorway_manifest_driven_routes.md` — sister pattern: manifests declare HTTP routes, validator enforces

## 6. Boundary-leak shapes noticed during investigation (good Layer 2 enforcement-rule candidates)

- **Inbound entry-type boundary is currently clean** — `grep EntryTypes::Content` in pillar zome dirs returns empty. Lock this in via the Layer 2 inbound check before regression.
- **Outbound bridge convention is asymmetric** — elohim DNA already uses `IMAGODEI_ROLE` constant (line 866 of `content_store/src/lib.rs`), pillars use string literals. Asymmetry suggests the constant pattern emerged in one direction first; Layer 1 resolves the asymmetry.
- **SweetTest harness has 9 sites of `const DNA: &str = "lamad"`** — every one is a co-conspirator in the misnaming. These should all reference the shared constant after Layer 1, so a future rename moves through `cargo build`.
- **`network_seed` strings (`elohim_lamad_alpha`, `elohim_imagodei_alpha`)** are another hardcoded-literal class — same enforcement shape as roles (constants module + manifest validator).
- **Discriminator strings** (`attestation:*`, `governance-action:*`) are the next boundary class — they're already centralized in elohim coordinator code but pillar zomes pass raw strings when proposing. Layer 2's manifest-discriminator-allowlist check is the natural enforcement seat.
