---
title: DHT PII Leak Remediation
id: dht-pii-leak-remediation-plan
status: Draft
cites:
  - genesis/docs/content/elohim-protocol/architecture/social-reach-nervous-system.md
  - imagodei-surfaces | Imagodei | sha256:e0abac6f6a6a0906 | path: genesis/docs/content/elohim-protocol/architecture/imagodei-surfaces-design.md
  - imagodei-profile-page-viewer-lens-design | Imagodei Profile-vs-Page | sha256:05caf5687b42f4ba | path: genesis/docs/superpowers/specs/2026-06-22-imagodei-profile-page-viewer-lens-design.md
  - elohim-seam-map-concern-routing | The Elohim Seam Map | sha256:54b5809fb8e688d1 | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md
  - .claude/skills/p2p-design-gate/SKILL.md
domain: D-imagodei-identity   # imagodei identity surfaces + confidentiality plane (seam-map §3.13)
sprint: security-remediation  # off-vision-rung; gates the privacy promise of the identity surface (adjacent Sprint 2 imagodei/recovery)
requires_env: [household-nodes]
---

# DHT PII Leak Remediation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop the Elohim DNAs from publishing soft PII (names, bios, locations, intimacy-tiered relationship edges, precise home GPS) to the gossiped, append-only, world-readable Holochain DHT, and add a guardrail so it cannot recur.

**Architecture:** Honor the three-plane split the protocol already follows for content blobs and for the witness/attestation half: **source-chain-private** holds the owner's PII, the **DHT public notary** holds only keys / hashes / CIDs / timestamps / opaque commitments, and the **SQLite projection** (local, query-controlled, deletable) serves PII reach-gated to authorized readers. The fix routes profile PII through the *already-proven-safe* presence/projection door (the same door `seed-presences.ts` uses, which never touches the conductor) instead of the `create_human`/`create_agent` DHT door, making the public DHT identity entries PII-free anchors. A field-level CI lint ratchets the invariant shut.

**Tech Stack:** Rust (Holochain HDI/HDK integrity + coordinator zomes), Holochain sweettest, Diesel/SQLite (elohim-storage projection), Rust (doorway-service), Python (the CI lint), the `.husky/pre-push` + orchestrator CI gate.

## Global Constraints

- **The DHT is append-only and world-readable.** Already-gossiped data CANNOT be recalled. Every fix here is **forward-only**: it stops new leakage; it does not erase what alpha test humans already published.
- **Changing an integrity-zome entry struct or its `visibility` changes the DNA hash** (the hash covers integrity zomes + modifiers). Phases that touch entry structs (Phase 1) are a **DNA migration**: per root `CLAUDE.md` §Deployment Contexts, a new DNA hash does NOT reach running conductors on a normal edge redeploy (install stale-check is role-structure-only), and forcing a reinstall is gated behind `ALLOW_DNA_REINSTALL` (mints a new agent key → needs migration/lineage on prod). **The alpha genesis bootstrap pair (adam + matthew) must BOTH get the flag or they land on different DNA hashes → different DHTs → P2P partition.** Phase 1 deploy is **operator-owned**; this plan builds and tests it in sweettest only.
- **Native Rust builds in this workspace need `RUSTFLAGS=""`** (the system sets the WASM `getrandom` custom backend which breaks native link); set `CARGO_TARGET_DIR` to the pool slot. **DNA/WASM workspaces (`elohim/holochain/dna/*`) use plain cargo — do NOT redirect target/.**
- **Reach is a read-time filter, not a substrate confidentiality gate** (`social-reach-nervous-system.md`). Do NOT "make reach gate the DHT" — that is the wrong mental model and would fork the canon. PII simply must not be on the DHT at all; reach remains the projection read-filter (Phase 2 makes that filter actually enforce on the HTTP read path).
- **"Fully private" is NOT the goal — the DHT keeps a PII-FREE PUBLIC ANCHOR.** Stripping PII does not mean making identity private. The `Human` anchor (id + agent-key binding + timestamps) and the `attestation:humanness` binding (PII-free, keyed on `subject_cid`) **STAY PUBLIC on the DHT** — *because that public, community-validated proof-of-humanity is exactly what the resiliency / Weave aggregation epics roll up*: `CoverageRollup` / `recursion.rs` fold "community-validated humans per household → council → region," and the hybrid chain-mints/DHT-validates identity model (the witnessed binding must be public to aggregate). Aggregation counts witnessed *bindings*, never reads PII; the k-anonymity floor (`CollectiveFilterPattern` k≥5) protects the aggregate from re-identifying individuals (extending that floor to attestation edges is Phase 3). **The split is exact: PII off the DHT; the witnessed anchor + attestation public on it.** The projection-vs-private-source-chain choice for the PII is orthogonal — it never touches the public anchor.
- **elohim monorepo push policy:** feature branch → land on `dev` via local fast-forward (no PR); `dev → main` is the reviewed release. Do not push to `main`. DNA changes trigger the DNA pipeline; `sweettest-check` runs by default when targeting `dev`/`main`.

---

## Evidence (the finding this plan fixes)

From the PII audit (workflow `wg3xpl4rs`, structs opened and re-verified):

- **Visibility census:** across ~119 app entry types, **exactly ONE** is declared private — `AttentionTending` (`elohim/holochain/dna/.../content_store_integrity/src/lib.rs:3694`). Public is the implicit default; the imagodei enum (`imagodei_integrity/src/lib.rs:892-947`) has zero private entries.
- **Inert labels:** `reach` / `profile_reach` / `visibility` are plain `String` fields *inside public entries* (`Human.profile_reach` `imagodei_integrity/src/lib.rs:291`; `Agent.visibility` `:332`; `Content.reach` `content_store_integrity/src/lib.rs:502`). They are read-time projection labels and do nothing to keep the payload off gossip. The comment at `imagodei_integrity/src/lib.rs:319` ("Profile data published to DHT if visibility allows") describes a gate that does not exist in code.
- **The live leak:** `create_human` (`imagodei/zomes/imagodei/src/lib.rs:396`) and `create_agent` (`:1307`) call `create_entry` **unconditionally**. The `Human` struct (`imagodei_integrity/src/lib.rs:286-295`) carries `display_name` (`:288`), `bio` (`:289`), `location` (`:292`); `Agent` (`:325-339`) adds `avatar` (`:330`). The doorway invokes `create_human` on the live conductor during hosted registration (`doorway-service/.../auth_routes.rs:994-1004` → `zome_helpers.rs:51,123`).
- **Worst offenders:** `HumanRelationship` (`imagodei_integrity/src/lib.rs:375-408`) — `party_a_id`, `party_b_id`, `relationship_type`, `intimacy_level`, `emergency_access_enabled`, free-text `context_json`; `NodeRegistration` (`node-registry/.../node_registry_integrity/src/lib.rs:70-71`) — precise `latitude`/`longitude` + `node_id`.
- **Scope:** soft PII only — grep confirms NO credential-grade PII (email/phone/legal name/DOB) is stored (the `email`/`phone` hits are enum *channel* values). Currently alpha-scale data; the code path is the bug, caught before scale.
- **The proven-safe door:** the ~161 seeded presences are NOT on the DHT — `seed-presences.ts:95` POSTs `/db/presences` → `PresenceService::create_presence` (`presence_service.rs:46-57`) → Diesel insert, **no conductor call**. The write-through allow-list (`write_through.rs:208-213`) forces only `KeyRotation | KeyRevocation | RevocationAttestation | AgentPeerBinding` (all PII-free) to the DHT. Same `display_name`/`bio` is SQLite-safe via the presence path, DHT-leaked via the registration path.
- **The witness half is already PII-free:** `attestation:humanness` keys on `subject_cid` (a hash), metadata is `additionalProperties:false` with only `humanness_method` + `confidence_score`, proof-evidence is pointers (`merkle_root`, `zkml_proof`, `issuer_signature`). PII-free precedents in-tree: private source-chain (`AttentionTending`), encrypt-before-commit (`RecoveryHint` AES-GCM, `imagodei_integrity/src/lib.rs:610-619`), k-anon aggregate (`CollectiveFilterPattern`, `content_store_integrity/src/lib.rs:3696-3700`).

---

## File Structure

- `.claude/scripts/lint/pii_public_entry_lint.py` (NEW) — scans integrity-zome `EntryTypes` structs for PII-lexicon fields on non-`visibility=private` entries; warn-mode baseline → error-mode ratchet.
- `.claude/scripts/lint/pii_public_entry_baseline.json` (NEW) — the explicit allowlist of currently-known violations the ratchet drains.
- `.claude/scripts/lint/tests/test_pii_public_entry_lint.py` (NEW) — unit tests for the lint (fixtures: a leaking struct, a clean struct, a private struct).
- `elohim/holochain/dna/.../imagodei_integrity/src/lib.rs` (MODIFY) — slim `Human`/`Agent` structs to PII-free anchors; fix the `:319` doc drift.
- `elohim/holochain/dna/.../imagodei/zomes/imagodei/src/lib.rs` (MODIFY) — `create_human`/`create_agent` accept the slimmed input; emit a post-commit signal carrying profile fields to the projection.
- `elohim/holochain/dna/.../node_registry_integrity/src/lib.rs` (MODIFY) — coarsen `latitude`/`longitude` to region granularity on the public entry.
- `elohim/holochain/tests/sweettest/...` (NEW/MODIFY) — sweettests asserting the committed DHT entries are PII-free.
- `elohim/elohim-storage/src/services/...` + migrations (MODIFY) — projection table + handler for the profile fields arriving via signal (mirrors the presence path).
- `doorway/doorway-service/src/routes/auth_routes.rs` + `zome_helpers.rs` (MODIFY) — registration writes profile PII to the projection door, passes only the anchor to `create_human`.
- `doorway/doorway-service/src/routes/...` + `elohim/elohim-storage/src/api/...` (MODIFY) — Phase 2: add the reach guard to projection read routes (close `http-reach-enforcement-gap`).
- `.husky/pre-push` + orchestrator gate wiring (MODIFY) — run the lint.

---

## Phase 0 — Stop the bleed + tell the truth (NO DNA change; lands on `dev` now)

### Task 1: Field-level PII-in-public-entry CI lint (warn-mode + baseline)

**Files:**
- Create: `.claude/scripts/lint/pii_public_entry_lint.py`
- Create: `.claude/scripts/lint/pii_public_entry_baseline.json`
- Test: `.claude/scripts/lint/tests/test_pii_public_entry_lint.py`

**Interfaces:**
- Produces: a CLI `python3 .claude/scripts/lint/pii_public_entry_lint.py [--check] [--update-baseline]` that exits non-zero in `--check` mode when a non-baselined PII field sits on a public entry. Consumed by Phase 0 Task 4 (CI wiring) and Phase 1 Task 9 (ratchet flip).

- [ ] **Step 1: Write the failing test**

```python
# .claude/scripts/lint/tests/test_pii_public_entry_lint.py
from pii_public_entry_lint import scan_source

LEAKING = '''
pub struct Human {
    pub id: String,
    pub display_name: String,   // PII
    pub bio: String,            // PII
}
'''
PRIVATE = '''
#[entry_type(visibility = "private")]
pub struct HumanProfile {
    pub display_name: String,
    pub bio: String,
}
'''
CLEAN = '''
pub struct Human {
    pub id: String,
    pub agent_key: AgentPubKey,
    pub created_at: Timestamp,
}
'''

def test_flags_pii_on_public_struct():
    hits = scan_source(LEAKING, visibility="public")
    assert {h.field for h in hits} == {"display_name", "bio"}

def test_ignores_pii_on_private_struct():
    assert scan_source(PRIVATE, visibility="private") == []

def test_passes_clean_anchor():
    assert scan_source(CLEAN, visibility="public") == []
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd .claude/scripts/lint && python3 -m pytest tests/test_pii_public_entry_lint.py -v`
Expected: FAIL — `ModuleNotFoundError: pii_public_entry_lint` (not written yet).

- [ ] **Step 3: Write the lint**

```python
# .claude/scripts/lint/pii_public_entry_lint.py
"""Flag PII-lexicon fields on PUBLIC Holochain integrity entries.
The DHT is world-readable + append-only: PII on a public entry is an un-recallable leak.
Public is the Holochain default; only `#[entry_type(visibility = "private")]` is exempt."""
import json, re, sys, glob
from dataclasses import dataclass

PII_LEXICON = re.compile(
    r"(name|bio|location|address|avatar|photo|lat|lng|longitude|latitude|"
    r"context|external_identifier|note|email|phone|birth|dob|gps)", re.I)
# A field is SAFE if it is plainly a hash/cid/key/pointer/commitment/timestamp.
SAFE_TYPE = re.compile(r"(Hash|Cid|CID|AgentPubKey|Timestamp|EntryHash|ActionHash|Signature|\[u8|commitment|nonce|encrypted)", re.I)
STRUCT = re.compile(r"(#\[entry_type\([^)]*\)\]\s*)?pub struct (\w+)\s*\{([^}]*)\}", re.S)
FIELD = re.compile(r"pub (\w+)\s*:\s*([^,\n]+)")

@dataclass(frozen=True)
class Hit:
    struct: str
    field: str
    ftype: str

def scan_source(src: str, visibility: str = None):
    hits = []
    for m in STRUCT.finditer(src):
        attr, name, body = m.group(1) or "", m.group(2), m.group(3)
        is_private = visibility == "private" or 'visibility = "private"' in attr
        if is_private:
            continue
        for fm in FIELD.finditer(body):
            fname, ftype = fm.group(1), fm.group(2).strip()
            if PII_LEXICON.search(fname) and not SAFE_TYPE.search(ftype):
                hits.append(Hit(name, fname, ftype))
    return hits

def scan_tree():
    hits = []
    for path in glob.glob("elohim/holochain/dna/**/*_integrity/src/**/*.rs", recursive=True):
        with open(path) as f:
            for h in scan_source(f.read()):
                hits.append({"path": path, "struct": h.struct, "field": h.field})
    return hits

def main(argv):
    baseline_path = ".claude/scripts/lint/pii_public_entry_baseline.json"
    hits = scan_tree()
    if "--update-baseline" in argv:
        json.dump(sorted(hits, key=lambda h: (h["path"], h["struct"], h["field"])),
                  open(baseline_path, "w"), indent=2)
        print(f"baseline updated: {len(hits)} known PII-on-public-entry fields")
        return 0
    try:
        baseline = json.load(open(baseline_path))
    except FileNotFoundError:
        baseline = []
    baseset = {(h["path"], h["struct"], h["field"]) for h in baseline}
    new = [h for h in hits if (h["path"], h["struct"], h["field"]) not in baseset]
    for h in hits:
        print(f"PII-ON-PUBLIC-ENTRY: {h['struct']}.{h['field']}  ({h['path']})")
    if new:
        print(f"\n*** {len(new)} NEW PII field(s) on public entries — not in baseline ***")
        for h in new:
            print(f"  NEW: {h['struct']}.{h['field']}  ({h['path']})")
    if "--check" in argv:
        return 1 if new else 0           # warn-mode: only NEW violations fail the build
    return 0

if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
```

- [ ] **Step 4: Run the unit tests to verify they pass**

Run: `cd .claude/scripts/lint && python3 -m pytest tests/test_pii_public_entry_lint.py -v`
Expected: PASS (3 passed).

- [ ] **Step 5: Generate the baseline of current known violations**

Run: `python3 .claude/scripts/lint/pii_public_entry_lint.py --update-baseline`
Expected: prints `baseline updated: N known PII-on-public-entry fields` (N ≥ ~6 — Human.display_name/bio/location, Agent.avatar, HumanRelationship.context_json, NodeRegistration latitude/longitude, ContributorPresence.external_identifiers_json). Eyeball the list against the Evidence section.

- [ ] **Step 6: Verify `--check` is green on the baseline, red on a new field**

Run: `python3 .claude/scripts/lint/pii_public_entry_lint.py --check; echo "exit=$?"`
Expected: lists known violations, `exit=0` (no NEW).
Then temporarily add `pub home_address: String` to a public struct and re-run — expect `NEW: ... home_address` and `exit=1`. Revert.

- [ ] **Step 7: Commit**

```bash
git add .claude/scripts/lint/
git commit -m "feat(lint): field-level PII-in-public-entry gate (warn-mode + baseline)"
```

### Task 2: Fix the doc-vs-behavior drift

**Files:**
- Modify: `elohim/holochain/dna/.../imagodei_integrity/src/lib.rs:319` (and grep siblings)

- [ ] **Step 1: Find every comment that claims reach/visibility gates DHT publication**

Run: `grep -rn "published to DHT if\|visibility allows\|private.*DHT\|DHT.*if.*reach" elohim/holochain/dna/`
Expected: at least the `:319` line.

- [ ] **Step 2: Correct the comment to state the truth**

Replace the `:319` comment with: `// NOTE: this entry is PUBLIC on the DHT (gossiped, world-readable, append-only). `reach`/`visibility` are read-time PROJECTION filters and do NOT keep this payload off gossip. Do NOT place PII here — see plans/2026-06-23-dht-pii-leak-remediation-plan.md.`

- [ ] **Step 3: Verify the integrity zome still compiles**

Run: `cd elohim/holochain/dna/<imagodei-dna> && cargo check -p imagodei_integrity`
Expected: compiles (comment-only change).

- [ ] **Step 4: Commit**

```bash
git add elohim/holochain/dna/
git commit -m "docs(imagodei): correct reach/visibility-gates-DHT drift comment"
```

### Task 3: Wire the lint into pre-push + CI (warn-mode)

**Files:**
- Modify: `.husky/pre-push`
- Modify: orchestrator/genesis gate wiring (the same place other repo lints register)

- [ ] **Step 1: Add the lint invocation to pre-push (non-blocking warn for now)**

Add to `.husky/pre-push` (in the changed-project detection for DNA/integrity changes):
`python3 .claude/scripts/lint/pii_public_entry_lint.py --check || echo "::warning:: PII-on-public-entry lint flagged NEW fields (see above)"`
(Warn-mode: `|| echo` keeps it non-blocking until Phase 1 drains the baseline; Task 9 flips it to blocking.)

- [ ] **Step 2: Verify the hook runs the lint**

Run: `bash .husky/pre-push </dev/null 2>&1 | grep -i pii` (or trigger a no-op push to a scratch branch with `--no-verify` disabled).
Expected: the lint output appears; the hook does not abort.

- [ ] **Step 3: Commit**

```bash
git add .husky/pre-push
git commit -m "ci: run PII-on-public-entry lint in pre-push (warn-mode)"
```

---

## Phase 1 — Move profile PII off the DHT (DNA migration; sweettest here, deploy operator-gated)

> **Build and test in sweettest only.** Deploy is a coordinated DNA migration owned by the operator (see Global Constraints + the Migration section). Do not push a DNA-hash change to `dev` expecting it to reach alpha conductors, and never force-reinstall only one of the bootstrap pair.

### Task 4: Sweettest — assert the Human DHT entry is PII-free (failing test first)

**Files:**
- Test: `elohim/holochain/tests/sweettest/tests/imagodei_pii.rs` (NEW)

**Interfaces:**
- Consumes: the imagodei coordinator `create_human`.
- Produces: the behavioral contract Task 5 must satisfy — the committed `Human` entry deserializes with NO `display_name`/`bio`/`location`.

- [ ] **Step 1: Write the failing sweettest**

```rust
// elohim/holochain/tests/sweettest/tests/imagodei_pii.rs
// Contract: a Human entry gossiped to the DHT must carry NO PII.
#[tokio::test(flavor = "multi_thread")]
async fn human_entry_carries_no_pii() {
    let (conductor, _agent, cell) = setup_imagodei().await;   // existing sweettest harness
    let input = CreateHumanInput { id: "h1".into() /* slimmed: no display_name/bio/location */ };
    let hash: ActionHash = conductor.call(&cell.zome("imagodei"), "create_human", input).await;

    let record: Record = conductor.call(&cell.zome("imagodei"), "get_human", hash).await;
    let raw = record.entry().as_option().unwrap().as_app_entry().unwrap();
    let json = serde_json::to_string(raw).unwrap().to_lowercase();
    for pii in ["display_name", "\"bio\"", "location", "avatar"] {
        assert!(!json.contains(pii), "Human DHT entry leaked PII field: {pii}");
    }
}

// Contract: the PII-free Human anchor STAYS PUBLIC and aggregatable — "fully private" would
// break the resiliency/Weave roll-up of community-validated proof-of-humanity.
#[tokio::test(flavor = "multi_thread")]
async fn human_anchor_stays_public_and_attestable() {
    let (conductor, _agent, cell) = setup_imagodei().await;
    let input = CreateHumanInput { id: "h1".into() };
    let hash: ActionHash = conductor.call(&cell.zome("imagodei"), "create_human", input).await;

    // The anchor is gossiped + resolvable (public), and carries the agent-key binding.
    let record: Record = conductor.call(&cell.zome("imagodei"), "get_human", hash.clone()).await;
    let human: Human = record.entry().to_app_option().unwrap().unwrap();
    assert_eq!(human.id, "h1");
    // A community humanness attestation can still bind to the public anchor's subject_cid,
    // and a count query (the unit the resiliency rollup folds) still sees it.
    let subject_cid = human_subject_cid(&hash);
    issue_humanness_attestation(&conductor, &cell, subject_cid.clone()).await;
    let count: u32 = conductor.call(&cell.zome("imagodei"), "count_humanness_attestations", subject_cid).await;
    assert_eq!(count, 1, "public humanness attestation must remain aggregatable");
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd elohim/holochain/tests/sweettest && RUSTFLAGS="" cargo test --test imagodei_pii human_entry_carries_no_pii -- --nocapture`
Expected: FAIL — either `CreateHumanInput` still requires the PII fields, or the entry JSON still contains `bio`/`location`.

### Task 5: Slim `Human`/`Agent` to PII-free anchors

**Files:**
- Modify: `imagodei_integrity/src/lib.rs:286-295` (`Human`), `:325-339` (`Agent`)
- Modify: `imagodei/zomes/imagodei/src/lib.rs:396` (`create_human`), `:1307` (`create_agent`)

**Interfaces:**
- Produces: `CreateHumanInput { id }` and `Human { id, agent_key, created_at }` (anchor only); a post-commit `Signal::HumanProfile { agent_key, display_name, bio, location }` consumed by Task 7. Same shape for `Agent`/`avatar`.

- [ ] **Step 1: Remove PII fields from the `Human` and `Agent` structs**

In `imagodei_integrity/src/lib.rs`, delete `display_name` (`:288`), `bio` (`:289`), `location` (`:292`) from `Human`, and `avatar` (`:330`) from `Agent`. Keep `id`, the agent-key binding, and timestamps. Leave `profile_reach` only if still used as a projection hint; otherwise remove it too (its meaning moves to the projection).

- [ ] **Step 2: Update `create_human`/`create_agent` to the slimmed input + emit the profile signal**

In `imagodei/zomes/imagodei/src/lib.rs`, change `create_human` to accept `CreateHumanInput { id }`, `create_entry` the slimmed `Human`, and in `post_commit` (or via a returned struct the doorway forwards) emit `Signal::HumanProfile { agent_key, display_name, bio, location }` so the PII flows to the projection, never the DHT entry.

- [ ] **Step 3: Run the Task 4 sweettest to verify it passes**

Run: `cd elohim/holochain/tests/sweettest && RUSTFLAGS="" cargo test --test imagodei_pii human_entry_carries_no_pii -- --nocapture`
Expected: PASS.

- [ ] **Step 4: Run the imagodei sweettest suite + fmt/clippy**

Run: `cd elohim/holochain/tests/sweettest && RUSTFLAGS="" cargo test imagodei -- --run-ignored all` then `cargo fmt --check && cargo clippy -- -D warnings` in the imagodei DNA workspace.
Expected: PASS / clean. (Update any sweettest that constructs the old `Human`/`Agent` shape — zome-sweettest-sync.)

- [ ] **Step 5: Commit**

```bash
git add elohim/holochain/dna/ elohim/holochain/tests/sweettest/
git commit -m "feat(imagodei)!: slim Human/Agent to PII-free DHT anchors (DNA migration)"
```

### Task 6: Coarsen `NodeRegistration` location on the public entry

**Files:**
- Modify: `node-registry/.../node_registry_integrity/src/lib.rs:70-71`
- Test: `elohim/holochain/tests/sweettest/tests/node_registry_pii.rs` (NEW)

- [ ] **Step 1: Write the failing test**

```rust
#[tokio::test(flavor = "multi_thread")]
async fn node_registration_location_is_coarse() {
    // a NodeRegistration with precise coords in must commit only region-grade precision
    let reg = register_node(lat: 29.4241, lng: -98.4936).await; // San Antonio, precise
    assert!(reg.region.is_some());
    // no full-precision float on the public entry
    let json = entry_json(&reg).await;
    assert!(!json.contains("29.4241") && !json.contains("98.4936"));
}
```

- [ ] **Step 2: Run to verify it fails** — `RUSTFLAGS="" cargo test --test node_registry_pii -- --nocapture` → FAIL.

- [ ] **Step 3: Replace precise `latitude`/`longitude` with a coarse `region` field** (e.g. round to ~0.1° / city-grade, or a region slug). Keep precise coords, if needed at all, in the local projection only — never the public entry.

- [ ] **Step 4: Run to verify it passes** → PASS. Run node-registry sweettests + fmt/clippy.

- [ ] **Step 5: Commit**

```bash
git add elohim/holochain/dna/
git commit -m "feat(node-registry)!: coarsen public NodeRegistration location to region grade"
```

### Task 7: Projection home + handler for profile PII (the safe door)

**Files:**
- Modify: `elohim/elohim-storage/src/services/...` (a `profile`/`human_profiles` projection mirroring `contributor_presences`)
- Create: `elohim/elohim-storage/src/migrations/<ts>_human_profiles/` (Diesel) — header comment `-- Source of truth: local projection (PII; reach-gated read)`
- Modify: `doorway-service/.../auth_routes.rs:994-1004`, `zome_helpers.rs:51,123`

**Interfaces:**
- Consumes: `Signal::HumanProfile { agent_key, display_name, bio, location }` from Task 5.
- Produces: a reach-gated `GET` projection row (Phase 2 enforces the gate).

- [ ] **Step 1: Write the failing storage test** — a test that, on receiving `Signal::HumanProfile`, inserts a `human_profiles` row and that `create_human` registration writes display_name to the projection, not the conductor entry. Run → FAIL.

- [ ] **Step 2: Add the Diesel migration + handler** mirroring `presence_service.rs:46-57` / `contributor_presences.rs` (the proven-safe path; no `create_entry`). Set the migration's source-of-truth comment.

- [ ] **Step 3: Update doorway registration** so `auth_routes.rs` passes only `{ id }` to `create_human` (via `zome_helpers`) and writes `display_name`/`bio`/`location` to the projection door.

- [ ] **Step 4: Run storage tests + the doorway route tests** (`RUSTFLAGS="" cargo test` with `CARGO_TARGET_DIR` set per pool) → PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/ doorway/doorway-service/
git commit -m "feat(storage,doorway): route profile PII through the projection door, not the DHT"
```

### Task 8: Regenerate TS bindings + reconcile consumers

- [ ] **Step 1:** Run `cargo test export_bindings` (storage) and `pnpm run schema:codegen:ts` as applicable; rebuild affected TS types for the slimmed `Human`/`Agent`/`NodeRegistration`.
- [ ] **Step 2:** Fix Angular adapters/consumers that read `human.displayName` from the DHT view so they read the projection profile view instead (`app/elohim-app/.../imagodei`). Run `pnpm exec vitest run` on touched specs → PASS.
- [ ] **Step 3:** Commit: `git commit -m "chore(sdk,app): reconcile TS to PII-free identity views"`.

### Task 9: Flip the lint to error-mode (ratchet) for the drained entries

- [ ] **Step 1:** Re-run `python3 .claude/scripts/lint/pii_public_entry_lint.py --update-baseline` — the baseline should now drop Human/Agent/NodeRegistration entries (they're clean), leaving only the Phase 3 items (HumanRelationship, ContributorPresence).
- [ ] **Step 2:** In `.husky/pre-push`, change the warn `|| echo` to a hard failure for DNA/integrity changes: `python3 .claude/scripts/lint/pii_public_entry_lint.py --check` (no `|| echo`).
- [ ] **Step 3:** Commit: `git commit -m "ci: ratchet PII-on-public-entry lint to blocking (Phase 1 drained)"`.

---

## Phase 2 — Close the read-side gap (NO DNA change; parallelizable with Phase 1)

### Task 10: Enforce reach on storage projection read routes (`http-reach-enforcement-gap`)

**Files:**
- Modify: `elohim/elohim-storage/src/api/...` and/or `doorway-service/.../routes/...` (every projection read route)
- Test: route tests asserting an unauthorized caller cannot read a non-public-reach profile row.

- [ ] **Step 1: Write the failing test** — a `GET` for a `reach: intimate` profile row by a non-authorized caller must return 403/empty, not 200-with-PII. Run → FAIL (today returns the row).
- [ ] **Step 2: Add the reach guard** at the read path so the projection's `reach` filter actually enforces (the filter that Phase 1 made the *sole* legitimate gate). Audit every read route — the gap is "reach enforced in exactly one place, missing on some routes."
- [ ] **Step 3: Run route tests** → PASS.
- [ ] **Step 4: Commit:** `git commit -m "fix(storage,doorway): enforce reach on projection read routes (close http-reach-enforcement-gap)"`.

---

## Phase 3 — Intimate graph + social-graph metaleak (FOLLOW-ON; separate plan)

> This is design-heavy (multi-party relationship truth, sealed edges, unlinkable vouching) and the writing-plans Scope Check says to split it. Capture it as the next plan, do NOT bloat this one.

- **HumanRelationship → private/sealed:** the intimate edge (`party_a/b`, `intimacy_level`, `context_json`) moves to a `visibility="private"` source-chain entry and/or an encrypted (`RecoveryHint`-style AES-GCM, or `SealedBlob` `sealed_against_self.rs:64-79`) form; the DHT holds at most an opaque commitment that a relationship exists between two anchors.
- **ContributorPresence:** `external_identifiers_json`/`display_name` already live in the projection for seeded presences; ensure no code path publishes them to a DHT entry, and bring them under the same reach-gated read.
- **Seal the residual metaleak:** even PII-free payloads leak who-attested-whom + timing via action headers and attestation links. Apply `SealedBlob`/`KeyEnvelope` to attestation edges, or add a k-anonymity/nullifier-commitment floor for unlinkable vouching (the `CollectiveFilterPattern` k≥5 precedent).
- **Action item now:** add a one-line backlog entry to `genesis/data/timeline/backlog/` linking this Phase 3 to domain D-imagodei-identity so it queues as the next sprint.

---

## Migration & Deploy (operator-owned — NOT executed by this plan)

Phase 1 changes the DNA hash. To land it on alpha without partition:
1. Land Phases 0 + 2 on `dev` first (no DNA change; immediate guardrail + read-gate).
2. Build + green all Phase 1 sweettests locally (`household-nodes`); confirm `cargo fmt`/`clippy` clean and `sweettest-check` passes when targeting `dev`.
3. Operator coordinates the DNA migration: set `ALLOW_DNA_REINSTALL` on **both** alpha bootstrap peers (adam + matthew) per `elohim/holochain/Jenkinsfile` env wiring; plan agent-key lineage/migration (reinstall mints a new key — do not blind-wipe prod). Verify both peers land on the **same** new DNA hash (`uhC0k…`) before declaring done.
4. The already-leaked alpha data is permanent and out of scope for deletion — note it in the migration record.

---

## Self-Review

- **Spec coverage:** (1) visibility-split identity entries → Tasks 4–8; (2) reach reframed (not "gate the DHT") → Global Constraints + Task 2 + Phase 2; (3) http-reach-enforcement-gap → Task 10; (4) social-graph metaleak → Phase 3; (5) field-level CI lint guardrail → Tasks 1, 3, 9. Append-only/forward-only and the operator-gated DNA migration are captured in Global Constraints + the Migration section. ✅
- **Placeholder scan:** lint code, sweettest code, and test sketches are concrete; DNA struct edits reference exact file:line; commands carry expected output. Where the exact harness call differs, the implementer adapts to the existing `setup_imagodei()` sweettest helper (named, not invented). ✅
- **Type consistency:** `CreateHumanInput { id }`, `Human { id, agent_key, created_at }`, and `Signal::HumanProfile { agent_key, display_name, bio, location }` are used consistently across Tasks 4, 5, 7. ✅
