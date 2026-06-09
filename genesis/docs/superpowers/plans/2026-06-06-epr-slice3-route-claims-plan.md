---
title: §12.6 Slice 3 — Route Claims (declare+grant), Alias Law & Link-Integrity Conformance — Implementation Plan
id: epr-slice3-route-claims-plan
status: Draft
class: protocol-canonical
domain: D8
topic: [epr, routing, routeClaims, redirect, alias, sitemap, conformance, doorway, dispatch]
cites:
  - epr-route-claims-link-conformance-design | THE spec this plan implements — Slice-3 routing contract (claims, alias law, conformance); plan tasks map to its gap items | sha256:69717bb30c4113be | status: stale — target content moved on; re-verify | path: genesis/docs/superpowers/specs/2026-06-06-epr-route-claims-link-conformance-design.md
  - pillar-epr-decomposition-design | the canonical parent whose §12.3/§12.6/§12.8 get forward-pointer amendments in Task 13 | sha256:8029079cea758380 | path: genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md
  - epr-slice2-universal-address-plan | the landed predecessor plan — Slice-2 surfaces (universal address, client claims, bridge component) this plan builds on and partially retires | sha256:78644191dd11bf3d | path: genesis/docs/superpowers/plans/2026-06-06-epr-slice2-universal-address-plan.md
---

# EPR Slice 3 — Route Claims, Alias Law & Conformance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the Slice-3 routing contract from `2026-06-06-epr-route-claims-link-conformance-design.md`: declare+grant routeClaims with doorway 302-to-pretty-mount, notarized alias law (`redirectsFrom` consumption + `redirectTemplates`), and the conformance floor (shared vectors, sitemap, lint gate, a2o).

**Architecture:** All governance compute lands at grant/refresh time (Commons Fast Path, spec §2 R1). The granted claims + alias templates ride the existing `project-epr` commitment metadata → `EprProjectionView` → boot-fetch/SSE/`replace_all` flow; doorway dispatch only reads pre-compiled in-memory indexes. The client mints from the bundle's *declared* claims via a shared template interpreter. One shared vector fixture pins both planes.

**Tech Stack:** Rust (elohim-views, elohim-storage, doorway-service/hyper), TypeScript (Angular 19, @elohim/service, vitest), Node seeder, JSON-schema contract tests, a2o (cucumber + Playwright).

**Out of scope (explicitly):** the §6 gate-face UI (gap item `#6-2` — needs angular-architect surface work; its own follow-up plan), steward-direct mode (stays 501), authed visitor-reach snapshots (classifier takes the parameter; wiring follows when gated projections exist), the substrate-visible link-audit Attestation (spec §13 captured follow-up).

**Environment (every Rust task):**
- doorway-service: `RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/doorway__doorway-service/dev cargo test --lib --bins` (run from `doorway/doorway-service/`). Use the pool slot printed at SessionStart for your worktree if different. Plain `cargo test` — no nextest in this container.
- elohim-storage: run from `elohim/elohim-storage/` with `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev` (ambient RUSTFLAGS stays).
- elohim-views ts-rs export: run from `elohim/elohim-views/` with `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/crates/dev`.
- TS: `pnpm --filter @elohim/service test`, `pnpm --filter holochain-seeder test`.

---

## File Structure

| File | Responsibility | Task |
|---|---|---|
| Create `elohim/sdk/fixtures/route-claims.vectors.json` | THE shared contract: reserved prefixes + the lamad grant + minting/dispatch vectors | 1 |
| Modify `elohim/elohim-views/src/projection.rs` | `RouteClaimTemplate`/`RouteClaimGrant`/`RedirectTemplate` types + 2 new `EprProjectionView` fields | 2 |
| Modify `elohim/sdk/schemas/v1/views/epr-projection-view.schema.json` | schema for the new fields | 2 |
| Modify `elohim/elohim-storage/tests/schema_contract.rs` | contract tests for the extended view | 2 |
| Modify `elohim/elohim-storage/src/db/rea_commitments.rs` | metadata→view mapping + validator rules 5–7 | 3, 4 |
| Modify `elohim/elohim-storage/src/services/rea_commitment_service.rs` | WIRE the validator into the create path | 4 |
| Modify `genesis/seeder/src/seed-projections.ts` (+ test) | lamad grant: routeClaims + redirectTemplates | 5 |
| Modify `app/elohim-library/projects/elohim-service/src/angular/utils/epr-ref.ts` (+ spec) | `RouteClaimTemplate` + `claimsFromDeclaration` interpreter | 6 |
| Create `app/lamad/src/app/route-claims.declaration.ts`; modify `app/lamad/src/app/app.config.ts` | single authoring home for lamad's declared claims | 7 |
| Modify `doorway/doorway-service/src/projection/epr_router.rs` | claims index + alias index + pure template fns | 8 |
| Modify `doorway/doorway-service/src/server/http.rs` | alias 302 at B13; `classify_epr_universal` + rewritten `dispatch_epr_universal`; `/sitemap.xml` arm | 9, 10 |
| Create `scripts/lint-route-literals.mjs`; modify `app/elohim-app/package.json`, `app/lamad/package.json` | minted-never-literal CI gate | 11 |
| Modify `genesis/a2o/features/lamad/deep-link-delivery.feature` + `genesis/a2o/steps/lamad/deep-link-delivery.steps.ts` | scenario-5 flip, doorway-302 bridge scenario, sitemap scenario | 12 |
| Modify `genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md` | §12.3/§12.6/§12.8 forward pointers | 13 |

---

### Task 1: The shared vector fixture (the contract, written first)

**Files:**
- Create: `elohim/sdk/fixtures/route-claims.vectors.json`

- [ ] **Step 1: Write the fixture**

```json
{
  "description": "Shared route-claims contract vectors — spec §8.4 of genesis/docs/superpowers/specs/2026-06-06-epr-route-claims-link-conformance-design.md. ONE claims vocabulary, two planes: client minting (claimsFromDeclaration + eprToRoute in @elohim/service) and doorway dispatch (EprRouter claims/alias indexes). Consumed via include_str! by doorway-service and via import by epr-ref.spec.ts + seed-projections tests. Edit here, never fork per-crate copies. reservedPrefixes mirrors doorway is_service_path (asserted by tests in BOTH doorway-service and elohim-storage's validator).",
  "reservedPrefixes": ["/epr", "/epr-head", "/db", "/api", "/blob", "/apps", "/auth/register", "/status", "/health", "/admin", "/identity", "/threshold", "/sitemap.xml"],
  "lamadGrant": {
    "schemaVersion": 1,
    "claimsManifestCid": null,
    "claims": [
      { "contentType": "path", "template": "path/{id}", "fragments": { "step": "path/{id}/step/{n}" } }
    ]
  },
  "lamadRedirectTemplates": [
    { "from": "/lamad/resource/{id}", "to": "/epr/{id}" }
  ],
  "mintVectors": [
    { "note": "claimed type, no fragment", "refId": "elohim-protocol", "contentType": "path", "expectCommands": ["/path", "elohim-protocol"], "expectHref": "/epr/elohim-protocol" },
    { "note": "claimed type, step fragment", "refId": "foundations-christian-technology", "contentType": "path", "fragmentType": "step", "fragmentValue": "2", "expectCommands": ["/path", "foundations-christian-technology", "step", "2"], "expectHref": "/epr/foundations-christian-technology#step/2" },
    { "note": "unclaimed type", "refId": "fct-module-01-church-dilemma", "contentType": "concept", "expectCommands": null, "expectHref": "/epr/fct-module-01-church-dilemma" }
  ],
  "dispatchVectors": [
    { "note": "claimed commons path id → 302 to pretty mount", "mountUrlPath": "/lamad", "template": "path/{id}", "id": "foundations-christian-technology", "expectLocation": "/lamad/path/foundations-christian-technology" },
    { "note": "id stays as-received (no re-encoding)", "mountUrlPath": "/lamad", "template": "path/{id}", "id": "a%20b", "expectLocation": "/lamad/path/a%20b" }
  ],
  "aliasVectors": [
    { "note": "legacy monolith share", "from": "/lamad/resource/{id}", "to": "/epr/{id}", "requestPath": "/lamad/resource/fct-module-01-church-dilemma", "expectLocation": "/epr/fct-module-01-church-dilemma" },
    { "note": "no match — different segment count", "from": "/lamad/resource/{id}", "to": "/epr/{id}", "requestPath": "/lamad/resource/a/b", "expectLocation": null },
    { "note": "no match — different prefix", "from": "/lamad/resource/{id}", "to": "/epr/{id}", "requestPath": "/lamad/path/x", "expectLocation": null },
    { "note": "bare redirects_from prefix swap", "bareFrom": "/learn", "mountUrlPath": "/lamad", "requestPath": "/learn/path/x", "expectLocation": "/lamad/path/x" },
    { "note": "bare redirects_from exact", "bareFrom": "/learn", "mountUrlPath": "/lamad", "requestPath": "/learn", "expectLocation": "/lamad" }
  ]
}
```

- [ ] **Step 2: Commit**

```bash
git add elohim/sdk/fixtures/route-claims.vectors.json
git commit -m "feat(sdk): route-claims shared contract vectors (spec §8.4 two-layer drift guard)"
```

---

### Task 2: View types — `EprProjectionView` extension (dual-path: schema + ts-rs)

**Files:**
- Modify: `elohim/elohim-views/src/projection.rs`
- Modify: `elohim/sdk/schemas/v1/views/epr-projection-view.schema.json`
- Test: `elohim/elohim-storage/tests/schema_contract.rs`

NOTE: `EprProjectionView` is ts-rs-anchored (`cargo test export_bindings` from `elohim/elohim-views`); do NOT add it to `INTERFACE_FILES` in codegen-ts.mjs.

- [ ] **Step 1: Write the failing schema-contract test** — append to `elohim/elohim-storage/tests/schema_contract.rs` next to `epr_projection_view_cached_mode_matches_schema` (~line 3397):

```rust
#[test]
fn epr_projection_view_with_route_claims_matches_schema() {
    use elohim_views::projection::{
        EprProjectionView, ProjectionMode, RedirectTemplate, RouteClaimGrant, RouteClaimTemplate,
    };

    let view = EprProjectionView {
        commitment_id: "sha256-claims0123".into(),
        epr_id: "lamad-spa".into(),
        doorway_id: "doorway:alpha-elohim-host".into(),
        url_path: "/lamad".into(),
        mode: ProjectionMode::Cached,
        reach: "commons".into(),
        base_href: "/lamad/".into(),
        entry_file: "index.html".into(),
        spa_fallback: true,
        redirects_from: vec![],
        redirect_templates: vec![RedirectTemplate {
            from: "/lamad/resource/{id}".into(),
            to: "/epr/{id}".into(),
        }],
        route_claims: Some(RouteClaimGrant {
            schema_version: 1,
            claims_manifest_cid: None,
            claims: vec![RouteClaimTemplate {
                content_type: "path".into(),
                template: "path/{id}".into(),
                fragments: std::collections::BTreeMap::from([(
                    "step".to_string(),
                    "path/{id}/step/{n}".to_string(),
                )]),
            }],
        }),
        preview_epr_ref: None,
        gate_hints: vec![],
        dead_end: false,
        steward_direct_endpoint: None,
        seeded_at: "2026-06-06T00:00:00Z".into(),
        seeded_by: "12D3KooWTest".into(),
    };

    let json = serde_json::to_value(&view).unwrap();
    assert_eq!(json["routeClaims"]["claims"][0]["contentType"], serde_json::json!("path"));
    assert_eq!(json["redirectTemplates"][0]["from"], serde_json::json!("/lamad/resource/{id}"));
    validate_against_schema("views/epr-projection-view.schema.json", &json);
}
```

Also update the two existing `EprProjectionView` literals in this file (`epr_projection_view_cached_mode_matches_schema`, the stewardDirect twin) with `redirect_templates: vec![], route_claims: None,`.

- [ ] **Step 2: Run to verify it fails** — `cd elohim/elohim-storage && CARGO_TARGET_DIR=... cargo test --test schema_contract epr_projection_view`. Expected: COMPILE FAIL (`RouteClaimGrant` not found).

- [ ] **Step 3: Add the Rust types + fields** — in `elohim/elohim-views/src/projection.rs`, after `StewardDirectEndpoint`:

```rust
/// A serializable route-claim template (spec §3, 2026-06-06 route-claims design).
/// `template` and `fragments` values use `{id}` / `{n}` placeholders, substituted
/// segment-safe (a placeholder binds exactly one path segment).
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RouteClaimTemplate {
    /// The EPR contentType this claim binds (e.g. "path").
    pub content_type: String,
    /// Mount-relative route template (e.g. "path/{id}").
    pub template: String,
    /// Fragment-type → deeper route template (e.g. step → "path/{id}/step/{n}").
    #[serde(default)]
    pub fragments: std::collections::BTreeMap<String, String>,
}

/// The GRANTED claims block on a project-epr commitment (spec §3.2): the
/// steward-authored operative routing law. `claims_manifest_cid` fingerprints
/// the bundle-manifest declaration acknowledged at grant time (claims-stale
/// drift detection, §3.4).
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RouteClaimGrant {
    pub schema_version: u32,
    pub claims_manifest_cid: Option<String>,
    pub claims: Vec<RouteClaimTemplate>,
}

/// A route-level alias promise on the commitment (spec §4): requests matching
/// `from` 302 to `to`. One hop; `to` must be a canonical address.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RedirectTemplate {
    pub from: String,
    pub to: String,
}
```

And in `EprProjectionView`, directly after `pub redirects_from: Vec<String>,`:

```rust
    /// Route-level alias promises (spec §4 — the notarized bridge story).
    #[serde(default)]
    pub redirect_templates: Vec<RedirectTemplate>,
    /// GRANTED route claims (spec §3.2). None = no claims granted.
    #[serde(default)]
    pub route_claims: Option<RouteClaimGrant>,
```

- [ ] **Step 4: Extend the schema** — in `epr-projection-view.schema.json`: add to `required`: `"redirectTemplates"` (after `"redirectsFrom"`). Add to `properties`:

```json
    "redirectTemplates": {
      "type": "array",
      "items": { "$ref": "#/$defs/RedirectTemplate" },
      "description": "Route-level alias promises: requests matching 'from' 302 to 'to'. One hop, canonical targets only."
    },
    "routeClaims": {
      "oneOf": [{ "type": "null" }, { "$ref": "#/$defs/RouteClaimGrant" }],
      "description": "GRANTED route claims (steward-authored at grant time). Null when no claims granted."
    },
```

And to `$defs`:

```json
    "RouteClaimTemplate": {
      "type": "object",
      "additionalProperties": false,
      "required": ["contentType", "template", "fragments"],
      "properties": {
        "contentType": { "type": "string", "description": "EPR contentType this claim binds." },
        "template": { "type": "string", "description": "Mount-relative route template with {id} placeholder." },
        "fragments": {
          "type": "object",
          "additionalProperties": { "type": "string" },
          "description": "Fragment-type to deeper route template ({id}/{n} placeholders)."
        }
      }
    },
    "RouteClaimGrant": {
      "type": "object",
      "additionalProperties": false,
      "required": ["schemaVersion", "claims"],
      "properties": {
        "schemaVersion": { "type": "integer" },
        "claimsManifestCid": { "type": ["string", "null"], "description": "Bundle-manifest CID acknowledged at grant time (claims-stale fingerprint)." },
        "claims": { "type": "array", "items": { "$ref": "#/$defs/RouteClaimTemplate" } }
      }
    },
    "RedirectTemplate": {
      "type": "object",
      "additionalProperties": false,
      "required": ["from", "to"],
      "properties": {
        "from": { "type": "string", "pattern": "^/", "description": "Request-path template to match ({id} binds one segment)." },
        "to": { "type": "string", "pattern": "^/", "description": "Canonical target template (one hop)." }
      }
    }
```

- [ ] **Step 5: Fix the compile ripples** — `EprProjectionView` literals exist in: `epr_router.rs` test helper `make_projection` (doorway), `commitment_to_projection_view` (storage — Task 3 handles properly; for now add the two fields with `Default`-ish values to compile: `redirect_templates: vec![], route_claims: None,`), and the two schema_contract literals (done in Step 1). Grep to be exhaustive:

```bash
grep -rn "EprProjectionView {" --include="*.rs" elohim/ doorway/
```

- [ ] **Step 6: Run tests** — storage: `cargo test --test schema_contract epr_projection_view` → PASS (3 tests). elohim-views: `cargo test` → PASS (camelCase serialization test).

- [ ] **Step 7: Regenerate ts-rs bindings** — from `elohim/elohim-views/`: `cargo test export_bindings`. Verify `elohim/sdk/storage-client-ts/src/generated/RouteClaimGrant.ts`, `RouteClaimTemplate.ts`, `RedirectTemplate.ts` exist and `EprProjectionView.ts` gained `routeClaims`/`redirectTemplates`.

- [ ] **Step 8: Commit**

```bash
git add elohim/elohim-views/src/projection.rs elohim/sdk/schemas/v1/views/epr-projection-view.schema.json elohim/elohim-storage/tests/schema_contract.rs elohim/sdk/storage-client-ts/src/generated/ doorway/doorway-service/src/projection/epr_router.rs elohim/elohim-storage/src/db/rea_commitments.rs
git commit -m "feat(views): routeClaims grant + redirectTemplates on EprProjectionView (spec §3.2, §4 — schema+ts-rs dual path)"
```

---

### Task 3: Storage metadata→view mapping

**Files:**
- Modify: `elohim/elohim-storage/src/db/rea_commitments.rs` (`commitment_to_projection_view`, ~line 736)

- [ ] **Step 1: Write the failing test** — append to the `#[cfg(test)]` module in `rea_commitments.rs`:

```rust
    #[test]
    fn projection_view_maps_route_claims_and_redirect_templates_from_metadata() {
        let metadata = serde_json::json!({
            "urlPath": "/lamad",
            "routeClaims": {
                "schemaVersion": 1,
                "claimsManifestCid": null,
                "claims": [{ "contentType": "path", "template": "path/{id}",
                             "fragments": { "step": "path/{id}/step/{n}" } }]
            },
            "redirectTemplates": [{ "from": "/lamad/resource/{id}", "to": "/epr/{id}" }]
        });
        let row = ReaCommitment {
            id: "test-claims".into(),
            action: "project-epr".into(),
            provider: "p".into(),
            receiver: "p".into(),
            in_scope_of: Some("doorway:alpha-elohim-host|epr:lamad-spa".into()),
            note: None,
            metadata: Some(metadata.to_string()),
            created_at: chrono::NaiveDateTime::default(),
            // ...fill remaining ReaCommitment fields exactly as the existing
            // mapping tests in this module do (copy a neighbouring literal).
            ..make_test_commitment_row()
        };
        let view = commitment_to_projection_view(row).unwrap();
        let grant = view.route_claims.expect("granted claims must map");
        assert_eq!(grant.claims[0].content_type, "path");
        assert_eq!(grant.claims[0].fragments.get("step").unwrap(), "path/{id}/step/{n}");
        assert_eq!(view.redirect_templates[0].from, "/lamad/resource/{id}");
        assert_eq!(view.redirect_templates[0].to, "/epr/{id}");
    }
```

(If the module has no `make_test_commitment_row` helper, construct the full `ReaCommitment` literal by copying an existing one from this file's tests — do not invent field names.)

- [ ] **Step 2: Run to verify it fails** — `cargo test -p elohim-storage --lib projection_view_maps_route_claims`. Expected: FAIL — the temporary `route_claims: None` stub from Task 2 Step 5 maps nothing.

- [ ] **Step 3: Implement the mapping** — in `commitment_to_projection_view`, replace the Task-2 stubs (placed after the `redirects_from` mapping) with the established metadata idioms:

```rust
        redirect_templates: metadata
            .get("redirectTemplates")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default(),
        route_claims: metadata.get("routeClaims").and_then(|v| {
            if v.is_null() {
                None
            } else {
                serde_json::from_value(v.clone()).ok()
            }
        }),
```

- [ ] **Step 4: Run tests** — same filter → PASS; then the file's full module: `cargo test -p elohim-storage --lib rea_commitments` → PASS.

- [ ] **Step 5: Commit** — `git commit -m "feat(storage): map routeClaims + redirectTemplates through commitment_to_projection_view"`

---

### Task 4: Validator rules 5–7 + WIRE validation into the create path

**Files:**
- Modify: `elohim/elohim-storage/src/db/rea_commitments.rs` (validator + input struct)
- Modify: `elohim/elohim-storage/src/services/rea_commitment_service.rs` (`create`, line ~38)

REALITY CHECK (harvest): `validate_project_epr_commitment` currently has **zero production call sites** — its doc-comment claims request-time invocation but none exists. This task wires it AND extends it.

- [ ] **Step 1: Write the failing validator tests** — append to the test module:

```rust
    fn reserved_prefixes_from_fixture() -> Vec<String> {
        #[derive(serde::Deserialize)]
        struct F { #[serde(rename = "reservedPrefixes")] reserved_prefixes: Vec<String> }
        let raw = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../sdk/fixtures/route-claims.vectors.json"));
        serde_json::from_str::<F>(raw).expect("route-claims fixture must parse").reserved_prefixes
    }

    #[test]
    fn validator_rejects_alias_on_reserved_prefix() {
        let mut input = make_project_epr_input_for_test("commons", None, vec![], false, None);
        input.redirects_from = vec!["/epr".into()];
        let err = validate_project_epr_commitment(&input).expect_err("reserved alias must reject");
        assert!(err.to_string().contains("reserved"), "unexpected error: {err}");
    }

    #[test]
    fn validator_rejects_redirect_template_chain_target() {
        let mut input = make_project_epr_input_for_test("commons", None, vec![], false, None);
        input.redirect_templates = vec![RedirectTemplate {
            from: "/old/{id}".into(),
            to: "/older/{id}".into(), // not /epr/... and not the mount → not canonical
        }];
        let err = validate_project_epr_commitment(&input).expect_err("non-canonical target must reject");
        assert!(err.to_string().contains("canonical"), "unexpected error: {err}");
    }

    #[test]
    fn validator_accepts_lamad_legacy_template() {
        let mut input = make_project_epr_input_for_test("commons", None, vec![], false, None);
        input.url_path = "/lamad".into();
        input.redirect_templates = vec![RedirectTemplate {
            from: "/lamad/resource/{id}".into(),
            to: "/epr/{id}".into(),
        }];
        assert!(validate_project_epr_commitment(&input).is_ok());
    }

    #[test]
    fn validator_fixture_reserved_list_is_nonempty() {
        // Two-layer guard: storage's validator and doorway's is_service_path both
        // assert against the SAME fixture list (doorway side in Task 8).
        assert!(reserved_prefixes_from_fixture().iter().any(|p| p == "/epr"));
    }
```

Update `make_project_epr_input_for_test` to set the new fields (`redirects_from: vec![], redirect_templates: vec![], route_claims: None`) and add those three fields to `ProjectEprValidationInput`.

- [ ] **Step 2: Run to verify failure** — COMPILE FAIL (fields missing), then assertion failures.

- [ ] **Step 3: Implement** — extend `ProjectEprValidationInput`:

```rust
    /// Mount-level bare aliases (legacy urlPaths that 302 to url_path).
    pub redirects_from: Vec<String>,
    /// Route-level alias templates (spec §4).
    pub redirect_templates: Vec<RedirectTemplate>,
    /// Granted claims (spec §3.2) — present for claim-vocabulary validation.
    pub route_claims: Option<RouteClaimGrant>,
```

Append rules 5–6 to `validate_project_epr_commitment` (after rule 4):

```rust
    // Rule 5 (spec §4): aliases may never collide with a reserved service
    // prefix. The reserved list is pinned by the shared route-claims fixture
    // (two-layer guard with doorway's is_service_path).
    const RESERVED_URL_PREFIXES: &[&str] = &[
        "/epr", "/epr-head", "/db", "/api", "/blob", "/apps", "/status",
        "/health", "/admin", "/identity", "/threshold", "/sitemap.xml",
    ];
    let alias_paths = input
        .redirects_from
        .iter()
        .cloned()
        .chain(input.redirect_templates.iter().map(|t| t.from.clone()));
    for alias in alias_paths {
        if !alias.starts_with('/') {
            return Err(StorageError::Validation(format!(
                "alias must start with '/', got: {alias}"
            )));
        }
        if RESERVED_URL_PREFIXES
            .iter()
            .any(|p| alias == *p || alias.starts_with(&format!("{p}/")))
        {
            return Err(StorageError::Validation(format!(
                "alias collides with a reserved service prefix: {alias}"
            )));
        }
    }

    // Rule 6 (spec §4): one hop — a redirect template's target must be a
    // canonical address: the universal /epr floor or this projection's mount.
    for t in &input.redirect_templates {
        let canonical = t.to.starts_with("/epr/")
            || t.to == input.url_path
            || t.to.starts_with(&format!("{}/", input.url_path));
        if !canonical {
            return Err(StorageError::Validation(format!(
                "redirect template target must be canonical (/epr/… or the mount), got: {}",
                t.to
            )));
        }
    }
```

- [ ] **Step 4: Wire validation into the create path** — in `ReaCommitmentService::create`, before the `PROJECT_EPR_ACTION` dispatch:

```rust
        if input.action == PROJECT_EPR_ACTION {
            // Spec §2.4 + §4: validate BEFORE the commitment row is built.
            // (Previously this validator existed but was never invoked — the
            // doc-comment's request-time claim was aspirational.)
            let meta: serde_json::Value = input
                .metadata_json
                .as_deref()
                .map(serde_json::from_str)
                .transpose()
                .map_err(|e| StorageError::Validation(format!("metadataJson parse: {e}")))?
                .unwrap_or(serde_json::Value::Null);
            let v_input = crate::db::rea_commitments::ProjectEprValidationInput {
                url_path: meta.get("urlPath").and_then(|v| v.as_str()).unwrap_or("/").to_string(),
                mode: meta.get("mode").cloned().and_then(|v| serde_json::from_value(v).ok()).unwrap_or(ProjectionMode::Cached),
                reach: meta.get("reach").and_then(|v| v.as_str()).unwrap_or("commons").to_string(),
                preview_epr_ref: meta.get("previewEprRef").and_then(|v| v.as_str()).map(String::from),
                gate_hints: meta.get("gateHints").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default(),
                dead_end: meta.get("deadEnd").and_then(|v| v.as_bool()).unwrap_or(false),
                steward_direct_endpoint: meta.get("stewardDirectEndpoint").and_then(|v| if v.is_null() { None } else { serde_json::from_value(v.clone()).ok() }),
                redirects_from: meta.get("redirectsFrom").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default(),
                redirect_templates: meta.get("redirectTemplates").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default(),
                route_claims: meta.get("routeClaims").and_then(|v| if v.is_null() { None } else { serde_json::from_value(v.clone()).ok() }),
            };
            crate::db::rea_commitments::validate_project_epr_commitment(&v_input)?;
            return Self::create_via_conductor(conn, ctx, input, events, hc_lamad).await;
        }
```

(Adjust the `input.metadata_json` field name to the actual `CreateReaCommitmentInput` field — grep `metadata_json\|metadataJson` in `rea_commitments.rs` for the struct definition; use what exists.)

NOTE — claim-uniqueness (spec §3.3, rule 7): cross-commitment `(doorway, contentType)` uniqueness needs a DB query of existing project-epr rows for the doorway. Implement as a second check here IF a `list projections for doorway` query fn already exists in `rea_commitments.rs` (grep `action=project-epr` query fns ~line 680); compare each existing `route_claims` grant's contentTypes against the incoming grant and reject on overlap with a `Validation("contentType '… ' already granted to …")`. If no query fn fits cleanly, log a `tracing::warn!` on conflict at the DOORWAY index-build instead (Task 8 makes conflicts deterministic + visible) and record the deferral in the commit message — do not invent a new query layer for MVP.

- [ ] **Step 5: Run tests** — `cargo test -p elohim-storage --lib rea_commitments` AND `cargo test -p elohim-storage --lib rea_commitment_service` → PASS. Existing validator tests must still pass unchanged (their error substrings are stable).

- [ ] **Step 6: Commit** — `git commit -m "feat(storage): wire project-epr validation into create path + alias rules 5-6 (spec §4)"`

---

### Task 5: Seeder — the lamad grant

**Files:**
- Modify: `genesis/seeder/src/seed-projections.ts`
- Test: `genesis/seeder/src/__tests__/seed-projections.test.ts`

- [ ] **Step 1: Write the failing test**

```typescript
  it('grants lamad routeClaims + the legacy resource redirect template', () => {
    const lamad = defaultProjectionSeeds().find(
      s => s.eprId === 'lamad-spa' && s.doorwayId === 'alpha-elohim-host',
    )!;
    const meta = JSON.parse(buildProjectionCommitmentBody(lamad).metadataJson);
    expect(meta.routeClaims.schemaVersion).toBe(1);
    expect(meta.routeClaims.claims).toEqual([
      { contentType: 'path', template: 'path/{id}', fragments: { step: 'path/{id}/step/{n}' } },
    ]);
    expect(meta.redirectTemplates).toEqual([{ from: '/lamad/resource/{id}', to: '/epr/{id}' }]);
  });
```

- [ ] **Step 2: Run** — `pnpm --filter holochain-seeder test` → FAIL (undefined fields).

- [ ] **Step 3: Implement** — extend `ProjectionSpec`:

```typescript
export interface RouteClaimTemplate {
  contentType: string;
  template: string;
  fragments?: Record<string, string>;
}
export interface RouteClaimGrant {
  schemaVersion: number;
  claimsManifestCid: string | null;
  claims: RouteClaimTemplate[];
}
export interface RedirectTemplate {
  from: string;
  to: string;
}
```

Add to `ProjectionSpec`: `routeClaims: RouteClaimGrant | null;` and `redirectTemplates: RedirectTemplate[];`. Add to `metadataObject` in `buildProjectionCommitmentBody`: `routeClaims: spec.routeClaims,` and `redirectTemplates: spec.redirectTemplates,`. In `defaultProjectionSeeds()` add to `base`: `routeClaims: null as RouteClaimGrant | null, redirectTemplates: [] as RedirectTemplate[],` and to `lamadAt`:

```typescript
    routeClaims: {
      schemaVersion: 1,
      claimsManifestCid: null,
      claims: [
        { contentType: 'path', template: 'path/{id}', fragments: { step: 'path/{id}/step/{n}' } },
      ],
    },
    redirectTemplates: [{ from: '/lamad/resource/{id}', to: '/epr/{id}' }],
```

- [ ] **Step 4: Run** — `pnpm --filter holochain-seeder test` → PASS (all existing tests too).

- [ ] **Step 5: Commit** — `git commit -m "feat(seeder): lamad grant — routeClaims + legacy resource redirectTemplate (spec §3.2, §4)"`

---

### Task 6: Client interpreter — `claimsFromDeclaration`

**Files:**
- Modify: `app/elohim-library/projects/elohim-service/src/angular/utils/epr-ref.ts`
- Test: `app/elohim-library/projects/elohim-service/src/angular/utils/epr-ref.spec.ts`

- [ ] **Step 1: Write the failing vector-driven spec** — append a describe block (vitest ambient globals; import the fixture relatively):

```typescript
import vectors from '../../../../../../../elohim/sdk/fixtures/route-claims.vectors.json';
// ^ verify depth with: ls from the spec file's dir; adjust ../ count so the
//   path resolves to elohim/sdk/fixtures/route-claims.vectors.json.

describe('claimsFromDeclaration (route-claims contract vectors)', () => {
  const ctx: BundleRouteContext = {
    claims: claimsFromDeclaration(vectors.lamadGrant.claims),
  };

  for (const v of vectors.mintVectors) {
    it(v.note, () => {
      const ref: EprRef = {
        id: v.refId,
        tier: 'head',
        ...(v.fragmentType
          ? { fragment: { type: v.fragmentType as 'step', value: v.fragmentValue! } }
          : {}),
      };
      const res = eprToRoute(ref, ctx, v.contentType);
      expect(res?.commands ?? null).toEqual(v.expectCommands);
      expect(res?.href).toBe(v.expectHref);
    });
  }
});
```

Add `claimsFromDeclaration` and `type EprRef` to the spec's existing import line from `'./epr-ref'`.

- [ ] **Step 2: Run** — `pnpm --filter @elohim/service test` (or `pnpm exec vitest run` in the library project) → FAIL (`claimsFromDeclaration` not exported).

- [ ] **Step 3: Implement** — in `epr-ref.ts`, after the `RouteClaim` interface:

```typescript
/** Serializable claim shape (spec §3.1) — what bundle manifests DECLARE and
 *  commitment grants carry. {id}/{n} placeholders, segment-safe. */
export interface RouteClaimTemplate {
  contentType: string;
  template: string;
  fragments?: Record<string, string>;
}

/**
 * Interpret serializable claim templates into executable RouteClaims (§8.3).
 * The single authoring home: bundles declare templates; this turns them into
 * the commands() the router consumes. A fragment whose type has a template
 * uses it ({n} = fragment value); otherwise the base template applies.
 */
export function claimsFromDeclaration(decl: readonly RouteClaimTemplate[]): RouteClaim[] {
  return decl.map(d => ({
    contentType: d.contentType,
    commands: (ref: EprRef): string[] => {
      const fragTpl = ref.fragment ? d.fragments?.[ref.fragment.type] : undefined;
      const tpl = fragTpl ?? d.template;
      const substituted = tpl
        .replace('{id}', ref.id)
        .replace('{n}', ref.fragment?.value ?? '');
      const segments = substituted.split('/').filter(s => s.length > 0);
      return ['/' + segments[0], ...segments.slice(1)];
    },
  }));
}
```

- [ ] **Step 4: Run** — spec PASSES (all mint vectors + every pre-existing eprToRoute test).

- [ ] **Step 5: Commit** — `git commit -m "feat(elohim-service): claimsFromDeclaration — serializable claim templates → executable RouteClaims (spec §8.3)"`

---

### Task 7: lamad derives its context from a declaration

**Files:**
- Create: `app/lamad/src/app/route-claims.declaration.ts`
- Modify: `app/lamad/src/app/app.config.ts` (lines ~172–186)

- [ ] **Step 1: Create the declaration** (single authoring home — mirrors the seeder grant; the route-claims fixture pins equivalence):

```typescript
import type { RouteClaimTemplate } from '@elohim/service';

/**
 * lamad's DECLARED route claims (spec §3.1 — the bundle's request; the
 * steward's project-epr grant activates them doorway-side). Keep in sync with
 * the granted shape in genesis/seeder/src/seed-projections.ts (lamadAt) —
 * drift between declaration and grant is the spec §3.4 claims-stale condition.
 */
export const LAMAD_ROUTE_CLAIMS: readonly RouteClaimTemplate[] = [
  { contentType: 'path', template: 'path/{id}', fragments: { step: 'path/{id}/step/{n}' } },
];
```

- [ ] **Step 2: Rewire app.config** — replace the hand-written claims provider (the `// §12.3: lamad claims contentType 'path'` block) with:

```typescript
    // §12.3 / Slice 3 (§8.3): lamad's claims derive from its DECLARATION —
    // one authoring home; the executable commands come from the shared
    // interpreter. Everything not declared stays cross-bundle.
    {
      provide: BUNDLE_ROUTE_CONTEXT,
      useValue: { claims: claimsFromDeclaration(LAMAD_ROUTE_CLAIMS) } satisfies BundleRouteContext,
    },
```

Update the import line to add `claimsFromDeclaration` (from `@elohim/service`) and `LAMAD_ROUTE_CLAIMS` (from `./route-claims.declaration`); drop the now-unused `type EprRef` import if nothing else uses it.

- [ ] **Step 3: Run the lamad tests + lint** — `cd app/lamad && pnpm test && pnpm run lint`. Expected: PASS — behavior is identical by the Task-6 vectors (same commands arrays).

- [ ] **Step 4: Commit** — `git commit -m "feat(lamad): derive BUNDLE_ROUTE_CONTEXT from the route-claims declaration (spec §8.3 single authoring home)"`

---

### Task 8: Doorway — claims + alias indexes and pure template fns

**Files:**
- Modify: `doorway/doorway-service/src/projection/epr_router.rs`

- [ ] **Step 1: Write failing vector-driven tests** — append to the existing test module (extend `make_projection` first with `redirect_templates: vec![], route_claims: None,`):

```rust
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct AliasVector {
        note: String,
        #[serde(default)] from: Option<String>,
        #[serde(default)] to: Option<String>,
        #[serde(default)] bare_from: Option<String>,
        #[serde(default)] mount_url_path: Option<String>,
        request_path: String,
        expect_location: Option<String>,
    }
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct DispatchVector { note: String, mount_url_path: String, template: String, id: String, expect_location: String }
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ClaimsFixture {
        reserved_prefixes: Vec<String>,
        alias_vectors: Vec<AliasVector>,
        dispatch_vectors: Vec<DispatchVector>,
    }
    fn claims_fixture() -> ClaimsFixture {
        let raw = include_str!(concat!(env!("CARGO_MANIFEST_DIR"),
            "/../../elohim/sdk/fixtures/route-claims.vectors.json"));
        serde_json::from_str(raw).expect("route-claims fixture must parse")
    }

    #[test]
    fn reserved_prefixes_fixture_agrees_with_is_service_path() {
        // Two-layer guard for the reserved list itself (spec §4 validation).
        for p in claims_fixture().reserved_prefixes {
            assert!(crate::server::http::is_reserved_url_path(&p),
                "fixture reserved prefix {p:?} must be reserved per is_reserved_url_path");
        }
    }

    #[test]
    fn template_alias_vectors_resolve() {
        for v in claims_fixture().alias_vectors {
            let got = if let (Some(from), Some(to)) = (v.from.as_deref(), v.to.as_deref()) {
                EprRouter::match_alias_template(&v.request_path, from, to)
            } else {
                EprRouter::match_bare_alias(
                    &v.request_path,
                    v.bare_from.as_deref().unwrap(),
                    v.mount_url_path.as_deref().unwrap(),
                )
            };
            assert_eq!(got, v.expect_location, "alias vector failed: {}", v.note);
        }
    }

    #[test]
    fn mount_location_vectors_mint() {
        for v in claims_fixture().dispatch_vectors {
            assert_eq!(
                EprRouter::mint_mount_location(&v.mount_url_path, &v.template, &v.id),
                v.expect_location,
                "dispatch vector failed: {}", v.note
            );
        }
    }

    #[test]
    fn resolve_alias_consults_projection_tables() {
        let router = EprRouter::new();
        let mut lamad = make_projection("lamad", "/lamad");
        lamad.redirect_templates = vec![elohim_views::projection::RedirectTemplate {
            from: "/lamad/resource/{id}".into(),
            to: "/epr/{id}".into(),
        }];
        lamad.redirects_from = vec!["/learn".into()];
        router.replace_all(vec![lamad]);
        assert_eq!(
            router.resolve_alias("/lamad/resource/fct-module-01-church-dilemma"),
            Some("/epr/fct-module-01-church-dilemma".into())
        );
        assert_eq!(router.resolve_alias("/learn/path/x"), Some("/lamad/path/x".into()));
        assert_eq!(router.resolve_alias("/lamad/path/x"), None, "live mounts are not aliases");
    }

    #[test]
    fn claim_binding_resolves_content_type_to_mount() {
        let router = EprRouter::new();
        let mut lamad = make_projection("lamad", "/lamad");
        lamad.route_claims = Some(elohim_views::projection::RouteClaimGrant {
            schema_version: 1,
            claims_manifest_cid: None,
            claims: vec![elohim_views::projection::RouteClaimTemplate {
                content_type: "path".into(),
                template: "path/{id}".into(),
                fragments: Default::default(),
            }],
        });
        router.replace_all(vec![lamad]);
        assert_eq!(
            router.claimed_mount_location("path", "foundations-christian-technology"),
            Some("/lamad/path/foundations-christian-technology".into())
        );
        assert_eq!(router.claimed_mount_location("concept", "x"), None);
    }
```

- [ ] **Step 2: Run to verify failure** — `RUSTFLAGS="" cargo test --lib epr_router` → COMPILE FAIL (methods missing).

- [ ] **Step 3: Implement** — extend `EprRouter`:

```rust
/// A granted claim binding compiled at table-load (spec §8.5): contentType →
/// (mount, template). Pre-resolved so dispatch stays a pure lookup (R1).
#[derive(Debug, Clone)]
struct ClaimBinding {
    mount_url_path: String,
    template: String,
}

#[derive(Debug, Default)]
pub struct EprRouter {
    /// urlPath → projection.
    table: RwLock<HashMap<String, EprProjectionView>>,
    /// contentType → claimed mount binding (compiled from grants in replace_all).
    claims: RwLock<HashMap<String, ClaimBinding>>,
}
```

In `replace_all`, after the table insert loop (same write-critical section ordering — take the claims lock after releasing the table lock to keep lock order trivial):

```rust
        // Compile the granted-claims index (spec §3.3: conflicts were rejected
        // at grant time; the router stays defensive — deterministic first-wins
        // by ascending url_path, with a warning, if a conflict slips through).
        let mut claim_index: HashMap<String, ClaimBinding> = HashMap::new();
        let mut sorted: Vec<&EprProjectionView> = {
            // re-read what we just installed
            Vec::new()
        };
        let table = self.table.read().expect("router lock poisoned");
        sorted.extend(table.values());
        sorted.sort_by(|a, b| a.url_path.cmp(&b.url_path));
        for p in sorted {
            if let Some(grant) = &p.route_claims {
                for c in &grant.claims {
                    if let Some(existing) = claim_index.get(&c.content_type) {
                        tracing::warn!(content_type = %c.content_type,
                            kept = %existing.mount_url_path, dropped = %p.url_path,
                            "claim conflict at router build — grant-time validation should have prevented this");
                        continue;
                    }
                    claim_index.insert(c.content_type.clone(), ClaimBinding {
                        mount_url_path: p.url_path.clone(),
                        template: c.template.clone(),
                    });
                }
            }
        }
        drop(table);
        *self.claims.write().expect("router lock poisoned") = claim_index;
```

(Restructure `replace_all` so the legal projections are inserted, the table lock dropped, then the claims index built from a read lock — keep the existing reserved-partition logic untouched.)

Add the pure fns + lookups:

```rust
    /// Mint the pretty-mount Location for a claimed contentType (spec §5.1).
    /// The id is substituted as-received from the request path — already
    /// percent-encoded, never re-encoded.
    pub(crate) fn mint_mount_location(mount_url_path: &str, template: &str, id: &str) -> String {
        let sub = template.replace("{id}", id);
        if mount_url_path == "/" {
            format!("/{sub}")
        } else {
            format!("{mount_url_path}/{sub}")
        }
    }

    /// Segment-wise template match: `{xxx}` binds exactly ONE non-empty
    /// segment. Returns the substituted `to` on match.
    pub(crate) fn match_alias_template(request_path: &str, from: &str, to: &str) -> Option<String> {
        let req: Vec<&str> = request_path.split('/').filter(|s| !s.is_empty()).collect();
        let pat: Vec<&str> = from.split('/').filter(|s| !s.is_empty()).collect();
        if req.len() != pat.len() {
            return None;
        }
        let mut bindings: Vec<(&str, &str)> = Vec::new();
        for (r, p) in req.iter().zip(pat.iter()) {
            if p.starts_with('{') && p.ends_with('}') {
                bindings.push((p, r));
            } else if r != p {
                return None;
            }
        }
        let mut out = to.to_string();
        for (placeholder, value) in bindings {
            out = out.replace(placeholder, value);
        }
        Some(out)
    }

    /// Bare alias (redirects_from): prefix-swap the alias for the mount.
    pub(crate) fn match_bare_alias(request_path: &str, bare_from: &str, mount: &str) -> Option<String> {
        if request_path == bare_from {
            return Some(mount.to_string());
        }
        request_path
            .strip_prefix(&format!("{bare_from}/"))
            .map(|rest| if mount == "/" { format!("/{rest}") } else { format!("{mount}/{rest}") })
    }

    /// Resolve an alias 302 for a request path (spec §4): template aliases
    /// first (they may live UNDER a live mount, e.g. /lamad/resource/{id}),
    /// then bare redirects_from prefix swaps. None = not an alias.
    pub fn resolve_alias(&self, request_path: &str) -> Option<String> {
        let table = self.table.read().expect("router lock poisoned");
        for p in table.values() {
            for t in &p.redirect_templates {
                if let Some(loc) = Self::match_alias_template(request_path, &t.from, &t.to) {
                    return Some(loc);
                }
            }
        }
        for p in table.values() {
            for bare in &p.redirects_from {
                if let Some(loc) = Self::match_bare_alias(request_path, bare, &p.url_path) {
                    return Some(loc);
                }
            }
        }
        None
    }

    /// The claimed pretty-mount Location for a contentType, if granted.
    pub fn claimed_mount_location(&self, content_type: &str, id: &str) -> Option<String> {
        let claims = self.claims.read().expect("router lock poisoned");
        claims
            .get(content_type)
            .map(|b| Self::mint_mount_location(&b.mount_url_path, &b.template, id))
    }
```

- [ ] **Step 4: Run** — `RUSTFLAGS="" cargo test --lib epr_router` → PASS (new + all FROZEN-adjacent existing tests; do NOT edit the frozen shakeout/handoff modules).

- [ ] **Step 5: Commit** — `git commit -m "feat(doorway): EprRouter claims + alias indexes, pure template fns, vector-pinned (spec §8.5, §4)"`

---

### Task 9: Doorway dispatch — alias 302 at B13 + the visitor-tiered universal resolver

**Files:**
- Modify: `doorway/doorway-service/src/server/http.rs`

- [ ] **Step 1: Write the failing classifier tests** — add a new test module next to `mod epr_universal_tests` (~line 3503):

```rust
#[cfg(test)]
mod epr_claims_dispatch_tests {
    use super::*;

    #[test]
    fn classify_redirects_claimed_commons() {
        let d = classify_epr_universal(
            Some(HeadFacts { content_type: Some("path".into()), reach: Some("commons".into()) }),
            Some("/lamad/path/abc".to_string()), // pre-resolved claimed location
        );
        assert_eq!(d, EprUniversalDisposition::RedirectToMount { location: "/lamad/path/abc".into() });
    }

    #[test]
    fn classify_serves_shell_for_unclaimed_commons() {
        let d = classify_epr_universal(
            Some(HeadFacts { content_type: Some("concept".into()), reach: Some("commons".into()) }),
            None,
        );
        assert_eq!(d, EprUniversalDisposition::ServeShell);
    }

    #[test]
    fn classify_never_redirects_gated_targets() {
        // Spec §5.1: 302 only when reach passes — anon never passes non-commons.
        let d = classify_epr_universal(
            Some(HeadFacts { content_type: Some("path".into()), reach: Some("household:x".into()) }),
            Some("/lamad/path/abc".to_string()),
        );
        assert_eq!(d, EprUniversalDisposition::ServeShell);
    }

    #[test]
    fn classify_serves_shell_when_head_unknown() {
        // Fail open toward the floor (spec §9): no head facts → shell.
        assert_eq!(classify_epr_universal(None, None), EprUniversalDisposition::ServeShell);
    }
}
```

- [ ] **Step 2: Run** — COMPILE FAIL. 

- [ ] **Step 3: Implement the pure classifier** — near `dispatch_epr_universal`:

```rust
/// Head facts for the /epr/{id} resolver — fetched from storage's LOCAL
/// projection only (never a DHT walk on the dispatch path, spec §5.1 R1).
/// Tolerant deserialization: a shape mismatch yields None fields → ServeShell
/// (fail open toward the floor).
#[derive(Debug, Clone, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HeadFacts {
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub reach: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum EprUniversalDisposition {
    /// Claimed + reach passes → 302 to the pretty mount (browser carries the
    /// fragment per RFC 7231 — the doorway never sees or needs it, spec §5.2).
    RedirectToMount { location: String },
    /// Everything else: the shell renders (universal viewer or gate face).
    ServeShell,
}

/// Spec §5.1 — pure, table-data-only (no prefix guards). MVP visitor tier:
/// anonymous (commons-only); the authed snapshot wires in when gated
/// projections exist (spec: out of scope here, signature stays ready).
pub(crate) fn classify_epr_universal(
    head: Option<HeadFacts>,
    claimed_location: Option<String>,
) -> EprUniversalDisposition {
    let reach_passes = head
        .as_ref()
        .and_then(|h| h.reach.as_deref())
        .map(|r| r == "commons")
        .unwrap_or(false);
    match (reach_passes, claimed_location) {
        (true, Some(location)) => EprUniversalDisposition::RedirectToMount { location },
        _ => EprUniversalDisposition::ServeShell,
    }
}
```

- [ ] **Step 4: Verify the head-facts source shape against the live local stack** (one-time, before wiring the fetch):

```bash
# start the dev trio if not running (hc-dev-orchestrator skill), then:
curl -s localhost:8090/db/content/foundations-christian-technology | head -c 400
```

Expected: a JSON object containing `"contentType"` and `"reach"` top-level (ContentView). If the body wraps the view (e.g. `{"item": {...}}`), note the envelope and unwrap accordingly in Step 5's `fetch_head_facts` (`v.get("item").unwrap_or(&v)`).

- [ ] **Step 5: Rewrite `dispatch_epr_universal`**:

```rust
/// §12.1 universal EPR address + Slice 3 claims (spec §5.1): a claimed,
/// commons-reach target 302s to its pretty mount; everything else serves the
/// ROOT projection's bundle (the shell renders the viewer or gate face).
async fn dispatch_epr_universal(state: &AppState, original_path: &str) -> Response<Full<Bytes>> {
    // Extract the id segment: /epr/{id}[/...] — id stays percent-encoded.
    let id = original_path
        .strip_prefix("/epr/")
        .unwrap_or("")
        .split('/')
        .next()
        .unwrap_or("");

    // Head facts: one LOCAL storage lookup (storage caches /db/content 300s).
    let head = if id.is_empty() {
        None
    } else {
        fetch_head_facts(state, id).await
    };
    let claimed = head
        .as_ref()
        .and_then(|h| h.content_type.as_deref())
        .and_then(|ct| state.epr_router.claimed_mount_location(ct, id));

    match classify_epr_universal(head, claimed) {
        EprUniversalDisposition::RedirectToMount { location } => {
            tracing::debug!(path = %original_path, location = %location,
                "universal /epr address — 302 to claimed pretty mount (Slice 3)");
            Response::builder()
                .status(StatusCode::FOUND)
                .header("Location", location)
                // Commons-only by construction here; cacheable (R1 for crawlers).
                .header("cache-control", "public, max-age=300")
                .body(Full::new(Bytes::new()))
                .unwrap()
        }
        EprUniversalDisposition::ServeShell => match state.epr_router.dispatch("/") {
            Some(root) => dispatch_to_projected_epr(state, "/", root).await,
            None => Response::builder()
                .status(StatusCode::FOUND)
                .header("Location", "/threshold")
                .body(Full::new(Bytes::new()))
                .unwrap(),
        },
    }
}

/// Tolerant local head lookup. Any failure → None (fail open to the shell).
async fn fetch_head_facts(state: &AppState, id: &str) -> Option<HeadFacts> {
    let storage_url = state.args.storage_url.as_deref()?.trim_end_matches('/').to_string();
    let url = format!("{storage_url}/db/content/{id}");
    let resp = state
        .ssr_http_client
        .get(&url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let v: serde_json::Value = resp.json().await.ok()?;
    // Unwrap a possible envelope; tolerate either shape.
    let body = v.get("item").unwrap_or(&v);
    serde_json::from_value(body.clone()).ok()
}
```

- [ ] **Step 6: Add the alias 302 to B13** — in `handle_request`, inside the B13 block (before the `dispatch` call):

```rust
        // Slice 3 alias law (spec §4): a notarized alias promise 302s BEFORE
        // mount dispatch — template aliases may live under a live mount
        // (e.g. /lamad/resource/{id}).
        if let Some(location) = state.epr_router.resolve_alias(&path) {
            tracing::debug!(path = %path, location = %location,
                "alias promise matched — 302 (redirects_from / redirectTemplates)");
            return Ok(to_boxed(
                Response::builder()
                    .status(StatusCode::FOUND)
                    .header("Location", location)
                    .header("cache-control", "public, max-age=300")
                    .body(Full::new(Bytes::new()))
                    .unwrap(),
            ));
        }
```

- [ ] **Step 7: Run the full doorway test suite** — `RUSTFLAGS="" cargo test --lib --bins` → PASS, including the FROZEN shakeout + handoff modules untouched, and `cargo clippy -- -D warnings && cargo fmt --check`.

- [ ] **Step 8: Commit** — `git commit -m "feat(doorway): Slice-3 dispatch — alias 302 at B13 + claims-aware /epr/{id} resolver (spec §4, §5.1)"`

---

### Task 10: Sitemap — event-invalidated materialized projection

**Files:**
- Modify: `doorway/doorway-service/src/server/http.rs` (service path list + new arm + materializer)
- Modify: `doorway/doorway-service/src/projection/epr_router.rs` (generation counter)

- [ ] **Step 1: Failing tests** — in the epr_router test module:

```rust
    #[test]
    fn replace_all_bumps_generation() {
        let router = EprRouter::new();
        let g0 = router.generation();
        router.replace_all(vec![make_projection("a", "/a")]);
        assert!(router.generation() > g0);
    }
```

And in http.rs (`epr_claims_dispatch_tests`):

```rust
    #[test]
    fn sitemap_xml_renders_mounts_and_claimed_entries() {
        let router = crate::projection::epr_router::EprRouter::new();
        let mut lamad = /* make a /lamad projection literal as in epr_router tests */;
        // give it the path claim as in Task 8's claim_binding test
        router.replace_all(vec![lamad]);
        let xml = render_sitemap(&router, "https://alpha.elohim.host",
            &[("path".to_string(), vec!["foundations-christian-technology".to_string()])]);
        assert!(xml.contains("<loc>https://alpha.elohim.host/lamad</loc>"));
        assert!(xml.contains("<loc>https://alpha.elohim.host/lamad/path/foundations-christian-technology</loc>"));
        assert!(xml.starts_with("<?xml"));
    }
```

- [ ] **Step 2: Implement** —
- `EprRouter`: add `generation: AtomicU64` bumped at the end of `replace_all`; `pub fn generation(&self) -> u64`.
- `http.rs`: pure `render_sitemap(router, base_url, commons_ids_by_type) -> String` building standard `<urlset>` XML: one `<url><loc>` per mount (`table` values' url_path) + one per claimed commons id (`mint_mount_location`). 
- Materializer: an `AppState` field `sitemap_cache: tokio::sync::RwLock<Option<(u64 /*generation*/, String /*xml*/)>>`. The `/sitemap.xml` arm: if cached generation == `router.generation()` → serve cached; else fetch commons ids per claimed type (`GET {storage}/db/content?contentType={ct}&reach=commons&limit=500`, tolerant-parse `items[].id`), render, cache, serve. Headers: `content-type: application/xml`, `cache-control: public, max-age=300`, `ETag: "g{generation}"`. Storage unavailable → serve mounts-only sitemap (fail open, log warn).
- Add `"/sitemap.xml"` to the `is_service_path` exact-match list (`matches!(path, "/admin" | "/status.json" | "/epr" | "/db" | "/sitemap.xml")`) and an explicit `(Method::GET, "/sitemap.xml")` arm before the wildcard.
- Base URL: reuse however the doorway already derives its public origin (grep `public_url\|external_url\|base_url` in args; if none exists, derive from the request `Host` header: `format!("https://{host}")`, falling back to `http://{host}` when TLS is not terminated here — match how SEO/canonical code in the repo does it, or use the Host header directly).

- [ ] **Step 3: Run** — doorway suite green + clippy + fmt.

- [ ] **Step 4: Commit** — `git commit -m "feat(doorway): /sitemap.xml — generation-invalidated materialized projection of the routing table (spec §7.5)"`

---

### Task 11: Static lint gate — links minted, never literal

**Files:**
- Create: `scripts/lint-route-literals.mjs`
- Modify: `app/elohim-app/package.json`, `app/lamad/package.json` (lint script chain)

- [ ] **Step 1: Write the script** (generalizes runbook §4.4; keepers via pragma):

```javascript
#!/usr/bin/env node
// Link-integrity static gate (spec §7.1 of
// genesis/docs/superpowers/specs/2026-06-06-epr-route-claims-link-conformance-design.md):
// links are MINTED (eprToRoute / eprToUniversalHref / claims), never literal.
// Generalizes the pillar-bundle-split runbook §4.4 router-literal canary.
// Keepers: a line containing `route-literal-ok: <reason>` is exempt; <base href>,
// SEO canonical generators, and doc comments should carry the pragma.
import { execSync } from 'node:child_process';

const TARGETS = process.argv.slice(2);
if (TARGETS.length === 0) {
  console.error('usage: lint-route-literals.mjs <srcDir> [...more]');
  process.exit(2);
}
// Forbidden literal route minting patterns (single- or double-quoted).
const PATTERNS = ["'/lamad", '"/lamad', "'/resource/", '"/resource/', "'/epr/", '"/epr/'];
let failures = 0;
for (const dir of TARGETS) {
  for (const pat of PATTERNS) {
    let out = '';
    try {
      out = execSync(
        `grep -rn ${JSON.stringify(pat)} ${dir} --include='*.ts' --include='*.html'`,
        { encoding: 'utf8' },
      );
    } catch {
      continue; // grep exit 1 = no matches
    }
    for (const line of out.split('\n').filter(Boolean)) {
      if (line.includes('route-literal-ok:')) continue;
      if (line.includes('.spec.ts:')) continue; // tests assert minted output
      console.error(`route literal: ${line}`);
      failures += 1;
    }
  }
}
if (failures > 0) {
  console.error(`\n${failures} raw route literal(s). Mint via eprToRoute/eprToUniversalHref/claims,`);
  console.error(`or annotate a documented keeper with: // route-literal-ok: <reason>`);
  process.exit(1);
}
console.log('lint-route-literals: clean');
```

- [ ] **Step 2: Run it on the tree and triage** —

```bash
node scripts/lint-route-literals.mjs app/lamad/src app/elohim-app/src
```

Expected: a handful of hits. For each: legitimate keepers (`<base href>`, SEO canonical builders, doc comments, `legacy-resource-redirect` route path declarations, the `/epr/` minting inside `eprToUniversalHref` itself is in elohim-library — not scanned) get `// route-literal-ok: <reason>`; genuine literals get rewritten through the minting utils. Re-run until clean.

- [ ] **Step 3: Wire into both app lint chains** — in `app/elohim-app/package.json` and `app/lamad/package.json`, append to the existing `"lint"` script value: `" && node ../../scripts/lint-route-literals.mjs src"` (elohim-app) / the lamad-relative equivalent (check each package's existing `lint` entry and relative depth to `scripts/`; lamad sits at `app/lamad` → `../../scripts/...`).

- [ ] **Step 4: Verify** — `cd app/elohim-app && pnpm run lint` → passes including the gate; same for lamad.

- [ ] **Step 5: Commit** — `git commit -m "feat(ci): lint-route-literals gate — links minted, never literal (spec §7.1; generalizes runbook §4.4)"`

---

### Task 12: a2o — scenario flips + new conformance scenarios

**Files:**
- Modify: `genesis/a2o/features/lamad/deep-link-delivery.feature`
- Modify: `genesis/a2o/steps/lamad/deep-link-delivery.steps.ts`

Slice-3 dispatch CHANGES two scenario expectations (this is the point — the substrate took over):

- [ ] **Step 1: Flip scenario 5** (`Universal EPR address resolves to a rendered surface`) — `/epr/foundations-christian-technology` is a *claimed* (`path`) commons id: it now 302s to `/lamad/path/...`. Replace with:

```gherkin
  @browser-only
  Scenario: Universal EPR address 302s a claimed type to its pretty mount
    # Spec §5.1 (Slice 3): claimed commons contentType → 302 to the mount.
    # The browser follows the redirect; the learner lands on the path overview.
    Given a learner opens the deep link "/epr/foundations-christian-technology"
    Then the lamad path overview renders
    And the rendered surface is not a raw error response

  @browser-only
  Scenario: Universal EPR address renders unclaimed types in the shell viewer
    # Spec §5.1: unclaimed contentType stays at the universal address —
    # the shell's cross-pillar resource viewer (safe-by-default floor).
    Given a learner opens the deep link "/epr/fct-module-01-church-dilemma"
    Then the cross-pillar resource viewer renders
    And the rendered surface is not a raw error response
```

- [ ] **Step 2: Upgrade the bridge `@regression` anchor** — keep the rendered outcome, add the substrate mechanism as an HTTP-transport scenario (no browser):

```gherkin
  @regression
  Scenario: The monolith-era share is honored by a notarized alias promise
    # Spec §4 (Slice 3): the redirectTemplates grant on the lamad projection
    # 302s the legacy address at the DOORWAY — before any bundle boots. The
    # client bridge component remains only as the SPA-plane twin.
    When the path "/lamad/resource/fct-module-01-church-dilemma" is requested without following redirects from doorway "alpha"
    Then the response status is 302
    And the response Location header is "/epr/fct-module-01-church-dilemma"
```

Keep the existing `@browser-only @regression` rendered scenario as-is (the end-to-end render outcome is unchanged — browser follows the 302 chain to the shell viewer).

- [ ] **Step 3: Add the sitemap scenario**:

```gherkin
  Scenario: The sitemap enumerates the claimed static plane
    # Spec §7.5: the sitemap is a derived projection of the routing table —
    # mounts plus claimed commons entries; it cannot drift from dispatch.
    When the path "/sitemap.xml" is requested without following redirects from doorway "alpha"
    Then the response status is 200
    And the response body contains "/lamad/path/foundations-christian-technology"
```

- [ ] **Step 4: Add the missing steps** — in `deep-link-delivery.steps.ts` (reuse `fetchApp`/`responseStore` from `../delivery.steps.js`; check whether `fetchApp` follows redirects — it must NOT for these steps; if it does, add a `redirect: 'manual'` variant here rather than changing the shared helper):

```typescript
When(
  'the path {string} is requested without following redirects from doorway {string}',
  async function (this: E2EWorld, path: string, doorwayId: string) {
    const doorway = this.getDoorway(doorwayId);
    const resp = await fetchApp(doorway.url, path, { redirect: 'manual' });
    responseStore.set(this, resp);
  }
);

Then('the response Location header is {string}', function (this: E2EWorld, expected: string) {
  const resp = responseStore.get(this);
  assert.ok(resp, 'No response captured — run the request step first');
  assert.equal(resp.headers['location'], expected);
});

Then('the response body contains {string}', function (this: E2EWorld, needle: string) {
  const resp = responseStore.get(this);
  assert.ok(resp, 'No response captured — run the request step first');
  assert.ok(
    resp.body.toString('utf8').includes(needle),
    `Expected body to contain "${needle}"`
  );
});
```

(Adapt `fetchApp`'s options parameter to its actual signature — read `genesis/a2o/steps/delivery.steps.ts` first; if it takes no options, add a local `fetchAppNoRedirect` helper in this file using the same transport with `redirect: 'manual'`. Before adding any step, grep the steps tree for an existing `response status`/`Location`/`body contains` step to avoid ambiguous redefinition.)

- [ ] **Step 5: Gherkin-parse check** — `cd genesis/a2o && pnpm exec cucumber-js --dry-run features/lamad/deep-link-delivery.feature` (or the repo's a2o dry-run equivalent). Expected: parses, steps bind, no ambiguity.

- [ ] **Step 6: Commit** — `git commit -m "test(a2o): Slice-3 routing scenarios — claimed 302, alias promise at the doorway, sitemap enumeration (spec §10)"`

---

### Task 13: Parent-spec forward pointers + closing audits

**Files:**
- Modify: `genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md` (§12.3, §12.6, §12.8)
- Modify: `genesis/data/timeline/backlog/epr-routing-complementary-captures.md`

- [ ] **Step 1: Amend the parent** (small, forward-pointing — this is a managed surface; the PreToolUse hook will inject cite discipline):
  - §12.3 routeClaims paragraph: append one sentence — `Slice-3 elevation: the full claims contract (declare+grant, alias law, conformance) is canonical in 2026-06-06-epr-route-claims-link-conformance-design.md.`
  - §12.6 slice table Slice 3 row: append `— designed by epr-route-claims-link-conformance-design (2026-06-06)`.
  - §12.8: append — `The 2026-06-06 design upholds Class C for claims (CID-addressed manifest content; grant rides the Category-A commitment) and adds redirectTemplates as commitment metadata (A); see its §12 gate record.`
- [ ] **Step 2: Re-point the backlog** — in `epr-routing-complementary-captures.md`, update the `LamadNotFoundComponent → designed gate experience` line: it is now gated on the gate-face follow-up plan (gap `#6-2` of the Slice-3 spec), not "once §12 Slice 3 lands."
- [ ] **Step 3: Cite hygiene + audits**:

```bash
python3 .claude/scripts/memory-kit/cite-gen.py --seal genesis/docs/superpowers/plans/2026-06-06-epr-slice3-route-claims-plan.md
python3 .claude/scripts/memory-kit/cite-gen.py --verify genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md
python3 .claude/scripts/memory-kit/spec-coherence-index.py
python3 .claude/scripts/memory-kit/placement-audit.py --ledger | head -20
```

- [ ] **Step 4: Commit** — `git commit -m "docs(spec): parent §12.3/§12.6/§12.8 forward pointers to the Slice-3 design + backlog re-point"`

---

### Task 14: Full-gate verification sweep

- [ ] **Step 1: Rust gates** — doorway: `RUSTFLAGS="" cargo build --release && cargo test --lib --bins && cargo clippy -- -D warnings && cargo fmt --check`. storage: `cargo test --lib && cargo test --test schema_contract`. views: `cargo test && cargo test export_bindings` (then `git status` — generated TS must be committed, byte-stable on re-run).
- [ ] **Step 2: TS gates** — `pnpm --filter @elohim/service test`, `cd app/lamad && pnpm test && pnpm run lint`, `cd app/elohim-app && pnpm run lint && pnpm test`, `pnpm --filter holochain-seeder test`.
- [ ] **Step 3: Schema gates** — `pnpm run schema:test && pnpm run schema:validate && pnpm run schema:codegen:ts -- --verify` (codegen freshness; note the known Prettier oscillation on Reach/ContentFormat is cosmetic and pre-existing).
- [ ] **Step 4: Local-stack smoke** (hc-dev-orchestrator + seeding):
  - `curl -si localhost:8888/lamad/resource/abc | head -3` → `302` + `Location: /epr/abc`
  - `curl -si localhost:8888/epr/foundations-christian-technology | head -3` → `302` + `Location: /lamad/path/foundations-christian-technology` (after re-seeding projections so the lamad grant is present: re-run the projection seeder — the new metadata produces the SAME deterministic commitment id, so a 409 means the OLD grant-less row persists; for local dev, reset the local content.db projections or bump via the documented local re-seed flow before asserting)
  - `curl -s localhost:8888/sitemap.xml | head -5` → XML with mounts
- [ ] **Step 5: Update gap-items states** — in `.claude/memory-kit/gap-items/specs__2026-06-06-epr-route-claims-link-conformance-design.json`, flip implemented items `OPEN → CLAIMED` (verification to CI/alpha flips them onward; a checked box is a claim, never trusted as done). Items `#6-2` (gate face) stays OPEN (deferred plan).
- [ ] **Step 6: Final commit** — story-harvest check per CLAUDE.md (finishing-a-development-branch invokes story-harvest), then integrator owns the push.

---

## Plan Self-Review (done at authoring)

1. **Spec coverage**: §3 claims contract → Tasks 2–7; §3.3 uniqueness → Task 4 (with an honest MVP deferral path to router-warn); §4 alias law → Tasks 4, 8, 9; §5.1 resolver → Task 9; §5.2 fragments → free (RFC 7231, asserted by a2o fragment scenario — NOTE: add the `#step/2` cold-load case to Task 12 if alpha verification shows fragment loss); §5.3 epr-summary-hint → existing `/epr-head/` surface (no new work; boundary-face consumption is the deferred gate-face plan); §6 gate face → explicitly deferred (gap `#6-2`); §7.1 lint → Task 11; §7.5 sitemap/sweep → Task 10 (sweep statuses `claims-stale`/`DEAD-ALIAS` ride the conformance crawler follow-up; recorded in spec §13); §8 homes → Tasks 1, 2, 6, 7, 8; §10 a2o → Task 12; §11 parent amendments → Task 13.
2. **Placeholders**: none — every adaptive instruction names the exact file to read and the decision rule.
3. **Type consistency**: `RouteClaimTemplate`/`RouteClaimGrant`/`RedirectTemplate` names identical across Rust (Task 2), seeder TS (Task 5), and @elohim/service TS (Task 6); `claimed_mount_location`/`resolve_alias`/`mint_mount_location` used in Task 9 match Task 8 definitions; `HeadFacts`/`classify_epr_universal`/`EprUniversalDisposition` defined and used only in Task 9.

---

## Execution record (2026-06-06, same-day)

Executed via orchestrated subagents (5 parallel + serial chains), all commits on `dev`:

| Task(s) | Commit | Evidence |
|---|---|---|
| 1 fixture | `74499bcd6` | — |
| 2 view types | `dc72b333f` | schema_contract 3 ✓ · views 367 ✓ · export_bindings regenerated |
| 3+4 storage | `8b8ca9dd8` | full lib 1355 ✓ · validator WIRED (had zero call sites) · rule-7 → router-warn |
| 5 seeder | `3777bd185` | vitest 257 ✓ |
| 6+7 client | `01fb7c851` | epr-ref 38 ✓ (3 vector-driven) · elohim-service 780 ✓ · lamad 2742 ✓ |
| 8–10 doorway | `385a7485a` | full suite 551 ✓ · clippy -D warnings ✓ · fmt ✓ · frozen oracles untouched |
| 11 lint gate | `c31f1d577` | 32 hits triaged · gate exit 0 · 290 component tests ✓ |
| 12 a2o | `5dfc3ddc1` | 10 scenarios · dry-run 42/42 bound, 0 ambiguous |
| 13 parent | `704dcfc88` | cite-verify ✓ |
| 14 sweep | `aec717b98` | schema:test 17 ✓ · schema:validate 3426 ✓ · codegen fresh ✓ · schema_contract 209 ✓ · cite graph clean |

**Verification debt (honest):** live-stack curl smoke + the seven `@browser-only` a2o runs await
alpha post-push (dev container cannot browser-render; local stack carries the known DHT-anchor
provenance gap). NOTE for re-seeding: the grant rides commitment *metadata*, but the commitment
id is content-addressed over (steward|action|scope) — a re-run 409s and the OLD grant-less row
persists; refreshing existing alpha projections needs a metadata update path or a one-time
projection re-seed, surfaced during Task 14 — carry this into the deploy watch. Gap items: 11
CLAIMED / 3 OPEN (5-3 hint-consumption legs, 6-2 gate face, 7-5 crawler+sweep — all deferred by
design).
