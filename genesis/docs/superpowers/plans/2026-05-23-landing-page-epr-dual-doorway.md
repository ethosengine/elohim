---
status: Draft
cites:
  - ../../plans/2026-05-19-doorway-stewardship-chain-design.md   # the design spec this plan implements
---

# Landing-Page EPR through Dual Federated Doorways — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Project a single Matthew-stewarded landing-page EPR (`elohim-host-landing` ContentNode) through two federated doorways — `alpha.elohim.host` (doorway-A) and `elohim.host` (doorway-B / "apex") — with Matthew registered as the primary `operate-doorway` REA operator at both, so visiting either surface renders the elohim-app browser bundle from a single content-addressed blob.

**Architecture:** One blob (elohim-app build) → two ContentNode rows (`lamad-spa` + `elohim-host-landing`) → two doorway deployments in the shared `elohim-alpha` namespace pointing at the same conductor + storage pool → two REA `operate-doorway` Commitments (Matthew × {alpha-elohim-host, apex-elohim-host}). The doorway service's existing root-app cache resolves `ROOT_APP_SLUG=elohim-host-landing` → blob hash via the `db/content` projection; the auth layer's `find_active_operator_binding` reads the operator commitments. No new entry types, no new tables — substrate is already in place; this plan wires it.

**Tech Stack:** Kubernetes manifests (YAML), Jenkins (Groovy DSL pipelines), TypeScript + Vitest (seeder), Node fetch + JSZip (existing), Rust (read-only — doorway service `services::federation::spawn_peer_discovery_task` discovers peers from `FEDERATION_PEERS`).

**Substrate references (no changes required):**
- `elohim/sdk/schemas/v1/objects/operator-classification.schema.json` — REA commitment shape for `operate-doorway`
- `elohim/sdk/schemas/v1/views/doorway-operator-binding-view.schema.json` — projection shape
- `elohim/elohim-storage/src/db/rea_commitments.rs:350` — `OPERATE_DOORWAY_ACTION` constant + `find_active_operator_binding`
- `doorway/doorway-service/src/auth/operator.rs` — JWT operator-capability resolver
- `doorway/doorway-service/src/routes/root_app.rs` — four-stage bootstrap shell
- `doorway/doorway-service/src/routes/health.rs:290-378` — `/health/startup` readiness logic
- `genesis/a2o/features/protocol/landing-page-dogfood.feature` — acceptance scenarios

**Acceptance criteria:**
1. `GET https://alpha.elohim.host/` → 302 → `/apps/elohim-host-landing/index.html` → 200 with HTML body.
2. `GET https://elohim.host/` → same.
3. Doorway `/health/startup` reports `rootApp.ready: true` on both surfaces.
4. `GET /api/v1/commitments?action=operate-doorway` against either surface lists Matthew's binding for that doorway's `DOORWAY_ID`.
5. Federation peer discovery: each doorway's `/admin/federation/peers` includes the other.
6. `genesis/a2o/features/protocol/landing-page-dogfood.feature` runs green.

---

## File Structure

| File | Status | Responsibility |
|---|---|---|
| `genesis/seeder/src/seed-operator-bindings.ts` | **NEW** | Build + POST `operate-doorway` REA commitments via doorway's `/api/v1/commitments`. Default set = Matthew × {alpha-elohim-host, apex-elohim-host}. Mirrors the shape of `seed-commitments.ts`. |
| `genesis/seeder/src/__tests__/seed-operator-bindings.spec.ts` | **NEW** | Vitest unit tests for the body builder — determinism, distinct ids per (operator, doorway), schema field mapping. Mirrors `seed-commitments.spec.ts`. |
| `genesis/data/lamad/content/elohim-host-landing.json` | MODIFY | Remove the `"blobHash": "PLACEHOLDER_REPLACED_BY_SEED_SCRIPT"` field; add `metadata.blobPopulatedAt` + `blobPopulatedBy` documentation. The blob is written into the DB by the App pipeline at deploy time; seed-sqlite must not overwrite it. |
| `genesis/orchestrator/manifests/doorway/alpha.yaml` | MODIFY | Flip `ROOT_APP_SLUG: "lamad" → "elohim-host-landing"`. |
| `genesis/orchestrator/manifests/doorway/alpha-b.yaml` | MODIFY | Flip `ROOT_APP_SLUG: "lamad" → "elohim-host-landing"`; symmetrize `FEDERATION_PEERS` to `https://alpha.elohim.host` (was `https://doorway-alpha.elohim.host`). |
| `Jenkinsfile` (root) | MODIFY | Extend `stageSpaBlob` helper at line 223: PUT the SPA hash to **both** `db/content/lamad-spa` and `db/content/elohim-host-landing`. One blob, two content rows. |
| `genesis/Jenkinsfile` | MODIFY | Insert a `Seed Operator Bindings` stage between the existing `Seed Custody Commitments` and `Trigger M1 Cross-Pod Fetch` stages, gated on `params.SEED_DATA`. Invokes `npx tsx src/seed-operator-bindings.ts` against `env.RESOLVED_DOORWAY_HOST`. |

---

## Pre-flight

### Task 0: Reset working tree to clean baseline

The current working tree contains an exploratory draft of these changes. Reset to a clean baseline so the plan executes from a known state and so we get clean per-commit diffs.

- [ ] **Step 0.1: Inspect current uncommitted state**

```bash
cd /projects/elohim
git status --short
```

Expected output:
```
 M Jenkinsfile
 M genesis/Jenkinsfile
 M genesis/data/lamad/content/elohim-host-landing.json
 M genesis/orchestrator/manifests/doorway/alpha-b.yaml
 M genesis/orchestrator/manifests/doorway/alpha.yaml
?? genesis/seeder/src/seed-operator-bindings.ts
```

- [ ] **Step 0.2: Confirm with operator before discarding**

Pause and ask the operator: "I'm about to `git checkout -- <files>` to discard the exploratory draft and re-execute via TDD. Proceed?" Only continue on explicit yes.

- [ ] **Step 0.3: Reset the modified files + remove the untracked seeder script**

```bash
cd /projects/elohim
git checkout -- Jenkinsfile genesis/Jenkinsfile genesis/data/lamad/content/elohim-host-landing.json genesis/orchestrator/manifests/doorway/alpha-b.yaml genesis/orchestrator/manifests/doorway/alpha.yaml
rm -f genesis/seeder/src/seed-operator-bindings.ts
git status --short
```

Expected output: empty (clean tree).

- [ ] **Step 0.4: Confirm clean baseline**

```bash
cd /projects/elohim
git diff
git status
```

Expected: no diff; "working tree clean".

---

## Task 1: Write the operator-binding seeder spec (TDD red phase)

Mirror the existing `seed-commitments.spec.ts` pattern: pure body-builder tests, no network.

**Files:**
- Create: `/projects/elohim/genesis/seeder/src/__tests__/seed-operator-bindings.spec.ts`

- [ ] **Step 1.1: Write the failing spec**

Create `/projects/elohim/genesis/seeder/src/__tests__/seed-operator-bindings.spec.ts` with:

```typescript
import { describe, it, expect } from 'vitest';
import { buildOperatorCommitmentBody, type OperatorBinding } from '../seed-operator-bindings.js';

describe('buildOperatorCommitmentBody', () => {
  const binding: OperatorBinding = {
    operatorHumanId: 'human-matthew-manager',
    operatorArchetype: 'desktop',
    doorwayId: 'alpha-elohim-host',
    capabilities: ['*'],
    successionRole: 'primary',
    reachScope: 'stewards-only',
  };

  it('action is exactly "operate-doorway"', () => {
    const body = buildOperatorCommitmentBody(binding);
    expect(body.action).toBe('operate-doorway');
  });

  it('provider and receiver are deterministic 12D3KooW peer_ids derived from the operator', () => {
    const body = buildOperatorCommitmentBody(binding);
    expect(body.provider).toMatch(/^12D3KooW[a-f0-9]{38}$/);
    expect(body.receiver).toMatch(/^12D3KooW[a-f0-9]{38}$/);
    // operate-doorway has no separate counterparty — provider == receiver by convention.
    expect(body.provider).toBe(body.receiver);
  });

  it('resourceClassifiedAs is the capability array verbatim', () => {
    const body = buildOperatorCommitmentBody(binding);
    expect(body.resourceClassifiedAs).toEqual(['*']);
  });

  it('inScopeOf is exactly ["doorway:<id>"]', () => {
    const body = buildOperatorCommitmentBody(binding);
    expect(body.inScopeOf).toEqual(['doorway:alpha-elohim-host']);
  });

  it('metadata carries schemaVersion=1, successionRole, reachScope, and operator humanId', () => {
    const body = buildOperatorCommitmentBody(binding);
    expect(body.metadata).toMatchObject({
      schemaVersion: 1,
      successionRole: 'primary',
      reachScope: 'stewards-only',
      operatorHumanId: 'human-matthew-manager',
    });
  });

  it('metadata.reachScope defaults to "operator-private" when omitted', () => {
    const body = buildOperatorCommitmentBody({ ...binding, reachScope: undefined });
    expect(body.metadata).toMatchObject({ reachScope: 'operator-private' });
  });

  it('id is deterministic — same (operator, doorway, action) → same id (idempotent re-runs)', () => {
    const a = buildOperatorCommitmentBody(binding);
    const b = buildOperatorCommitmentBody(binding);
    expect(a.id).toBe(b.id);
  });

  it('id is distinct across doorways for the same operator', () => {
    const alpha = buildOperatorCommitmentBody(binding);
    const apex = buildOperatorCommitmentBody({ ...binding, doorwayId: 'apex-elohim-host' });
    expect(alpha.id).not.toBe(apex.id);
  });

  it('id has the "operate-doorway-" prefix for ergonomic log scanning', () => {
    const body = buildOperatorCommitmentBody(binding);
    expect(body.id).toMatch(/^operate-doorway-[a-f0-9]{16}$/);
  });
});
```

- [ ] **Step 1.2: Run the spec and confirm it fails for the right reason**

```bash
cd /projects/elohim/genesis/seeder
pnpm exec vitest run src/__tests__/seed-operator-bindings.spec.ts 2>&1 | tail -15
```

Expected: tests fail with `Cannot find module '../seed-operator-bindings.js'` or equivalent — the implementation file doesn't exist yet. This is the desired red state.

- [ ] **Step 1.3: Commit the red spec**

```bash
cd /projects/elohim
git add genesis/seeder/src/__tests__/seed-operator-bindings.spec.ts
git commit -m "$(cat <<'EOF'
test(seeder): add failing spec for seed-operator-bindings

Mirror of seed-commitments.spec.ts pattern — pure body-builder tests
covering the operate-doorway REA commitment shape:
- action discriminator, peer_id derivation, capability list mapping
- doorway scope encoding, metadata fields, reachScope default
- idempotent ids per (operator, doorway), distinct across doorways

Red against the not-yet-implemented seed-operator-bindings.ts.
EOF
)"
```

---

## Task 2: Implement the operator-binding seeder (TDD green phase)

**Files:**
- Create: `/projects/elohim/genesis/seeder/src/seed-operator-bindings.ts`

- [ ] **Step 2.1: Implement the body builder + script**

Create `/projects/elohim/genesis/seeder/src/seed-operator-bindings.ts` with:

```typescript
/**
 * Seed Doorway-Operator Bindings (REA operate-doorway commitments)
 *
 * Each binding is an REA Commitment with action='operate-doorway' that names
 * a human as an operator of a specific doorway. The doorway's auth layer
 * reads these via `find_active_operator_binding` and embeds the capability
 * snapshot into JWTs issued to the operator.
 *
 * Substrate references:
 *   - elohim/sdk/schemas/v1/objects/operator-classification.schema.json
 *   - elohim/sdk/schemas/v1/views/doorway-operator-binding-view.schema.json
 *   - elohim/elohim-storage/src/db/rea_commitments.rs (OPERATE_DOORWAY_ACTION)
 *   - genesis/docs/plans/2026-05-19-doorway-stewardship-chain-design.md
 *
 * Sister to seed-commitments.ts (custody-blob) — same POST endpoint, different
 * action discriminator. Same idempotency story: id is content-addressed over
 * (operator_peer_id, action, scope), re-runs collapse to 409.
 *
 * Default M5 binding set:
 *   - human-matthew-manager  →  doorway:alpha-elohim-host  (primary, *)
 *   - human-matthew-manager  →  doorway:apex-elohim-host   (primary, *)
 *
 * One human, two doorways, one capability scope. Federation pair
 * alpha.elohim.host ↔ elohim.host both list Matthew as primary operator.
 *
 * Usage:
 *   DOORWAY_URL=http://localhost:8888 npx tsx src/seed-operator-bindings.ts
 *   DOORWAY_URL=https://alpha.elohim.host DOORWAY_API_KEY=xxx \
 *     OPERATOR_BINDINGS_JSON=./bindings.json npx tsx src/seed-operator-bindings.ts
 */

import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { DoorwayClient } from './doorway-client.js';
import { deterministicPeerId, type Archetype } from './peer-id.js';

// =============================================================================
// Types
// =============================================================================

export type SuccessionRole = 'primary' | 'deputy' | 'recovery-witness';
export type ReachScope = 'operator-private' | 'stewards-only' | 'public';

export interface OperatorBinding {
  operatorHumanId: string;
  operatorArchetype: Archetype;
  doorwayId: string;
  capabilities: string[];
  successionRole: SuccessionRole;
  reachScope?: ReachScope;
}

interface CommitmentBody {
  id: string;
  action: 'operate-doorway';
  provider: string;
  receiver: string;
  resourceClassifiedAs: string[];
  inScopeOf: string[];
  note: string;
  metadata: Record<string, unknown>;
}

// =============================================================================
// Body builder (testable in isolation)
// =============================================================================

/**
 * Build a CreateReaCommitmentInputView body for one operator binding.
 *
 * `id` is content-addressed over (operator_peer_id, action, scope) so re-runs
 * are idempotent — POST returns 409 on the second invocation. Distinct
 * (operator, doorway) pairs produce distinct ids; the same operator on a
 * different doorway is a separate binding row.
 *
 * `provider` is the operator's deterministic peer_id (Stage 1 opaque hash,
 * see peer-id.ts). `receiver` is the same peer_id by convention — REA's
 * provider/receiver semantics don't apply cleanly to operate-doorway (the
 * "service" is doorway operation, not a transfer between agents). The auth
 * resolver filters by provider + scope; receiver is informational.
 */
export function buildOperatorCommitmentBody(binding: OperatorBinding): CommitmentBody {
  const peerId = deterministicPeerId(binding.operatorHumanId, binding.operatorArchetype);
  const scope = `doorway:${binding.doorwayId}`;

  const idDigest = createHash('sha256')
    .update(`${peerId}|operate-doorway|${scope}`, 'utf8')
    .digest('hex')
    .slice(0, 16);

  return {
    id: `operate-doorway-${idDigest}`,
    action: 'operate-doorway',
    provider: peerId,
    receiver: peerId,
    resourceClassifiedAs: binding.capabilities,
    inScopeOf: [scope],
    note: `${binding.operatorHumanId} as ${binding.successionRole} operator of ${binding.doorwayId}`,
    metadata: {
      schemaVersion: 1,
      successionRole: binding.successionRole,
      reachScope: binding.reachScope ?? 'operator-private',
      seedGeneration: 'genesis',
      operatorHumanId: binding.operatorHumanId,
    },
  };
}

// =============================================================================
// Default M5 binding set
//
// One human (Matthew) registered as primary operator of both alpha-cluster
// doorways. Each doorway projects the same elohim-host-landing EPR; the
// operator binding is per-doorway but the human is the same.
// =============================================================================

function defaultM5Bindings(): OperatorBinding[] {
  return [
    {
      operatorHumanId: 'human-matthew-manager',
      operatorArchetype: 'desktop',
      doorwayId: 'alpha-elohim-host',
      capabilities: ['*'],
      successionRole: 'primary',
      reachScope: 'stewards-only',
    },
    {
      operatorHumanId: 'human-matthew-manager',
      operatorArchetype: 'desktop',
      doorwayId: 'apex-elohim-host',
      capabilities: ['*'],
      successionRole: 'primary',
      reachScope: 'stewards-only',
    },
  ];
}

// =============================================================================
// Client
// =============================================================================

class OperatorBindingClient extends DoorwayClient {
  async createCommitment(body: CommitmentBody): Promise<Response> {
    return this.fetch('/api/v1/commitments', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
  }
}

// =============================================================================
// Seeding (fail-fast on non-409 errors)
// =============================================================================

export async function seedOperatorBindings(
  client: OperatorBindingClient,
  bindings: OperatorBinding[],
): Promise<void> {
  console.log(`[seed-operator-bindings] Seeding ${bindings.length} operate-doorway commitments...`);

  let created = 0;
  let alreadyExists = 0;

  for (const binding of bindings) {
    const body = buildOperatorCommitmentBody(binding);
    const label = `${binding.operatorHumanId.replace(/^human-/, '')}→${binding.doorwayId} (${binding.successionRole})`;

    const response = await client.createCommitment(body);

    if (response.ok) {
      console.log(`  [+] ${label}  caps=[${binding.capabilities.join(',')}]`);
      created += 1;
      continue;
    }

    const text = await response.text();
    if (response.status === 409 || text.includes('UNIQUE') || text.includes('already exists')) {
      console.log(`  [=] ${label} (idempotent re-run)`);
      alreadyExists += 1;
      continue;
    }

    // ANY other failure is a shape mismatch or doorway issue — fail fast.
    console.error(`  [X] ${label}: HTTP ${response.status}`);
    console.error(`      Body: ${text.slice(0, 500)}`);
    console.error(`      Sent: ${JSON.stringify(body, null, 2)}`);
    process.exit(1);
  }

  console.log(
    `[seed-operator-bindings] Done. created=${created} already-exists=${alreadyExists} total=${bindings.length}`,
  );
}

// =============================================================================
// Standalone execution
// =============================================================================

const isMain = import.meta.url === `file://${process.argv[1]}`;
if (isMain) {
  const doorwayUrl = process.env.DOORWAY_URL || 'http://localhost:8888';
  const apiKey = process.env.DOORWAY_API_KEY;

  const bindingsJsonPath = process.env.OPERATOR_BINDINGS_JSON;
  const bindings: OperatorBinding[] = bindingsJsonPath
    ? (JSON.parse(readFileSync(bindingsJsonPath, 'utf-8')) as OperatorBinding[])
    : defaultM5Bindings();

  const client = new OperatorBindingClient({ baseUrl: doorwayUrl, apiKey });

  console.log('='.repeat(60));
  console.log('Doorway-Operator Binding Seeder');
  console.log(`  Target:   ${doorwayUrl}`);
  console.log(`  Bindings: ${bindings.length}`);
  console.log('='.repeat(60));
  console.log();

  const health = await client.checkHealth();
  if (!health.healthy) {
    console.error(`ERROR: Doorway not healthy — ${health.error}`);
    process.exit(1);
  }

  await seedOperatorBindings(client, bindings);
  process.exit(0);
}
```

- [ ] **Step 2.2: Run the spec and confirm it passes**

```bash
cd /projects/elohim/genesis/seeder
pnpm exec vitest run src/__tests__/seed-operator-bindings.spec.ts 2>&1 | tail -15
```

Expected: 9 tests pass (action, peer_ids, resourceClassifiedAs, inScopeOf, metadata fields, reachScope default, deterministic id, distinct ids per doorway, id prefix).

- [ ] **Step 2.3: Run the seeder workspace typecheck on just the new file's neighborhood**

```bash
cd /projects/elohim/genesis/seeder
pnpm exec tsc --noEmit -p . 2>&1 | grep -E "(seed-operator-bindings|error)" | head -10
```

Expected: no errors mentioning `seed-operator-bindings.ts`. (Pre-existing errors in `wait-for-drain.ts` are tolerated — they are unrelated.)

- [ ] **Step 2.4: Commit the green implementation**

```bash
cd /projects/elohim
git add genesis/seeder/src/seed-operator-bindings.ts
git commit -m "$(cat <<'EOF'
feat(seeder): seed-operator-bindings — REA operate-doorway commitments

Sister to seed-commitments.ts (custody-blob). Builds + POSTs operator
bindings via /api/v1/commitments using the existing CreateReaCommitmentInputView
shape. Default M5 set registers Matthew as primary operator of both
alpha-cluster doorways:

  human-matthew-manager → doorway:alpha-elohim-host  (primary, capabilities=[*])
  human-matthew-manager → doorway:apex-elohim-host   (primary, capabilities=[*])

One human, two doorways, one EPR. Schema v1 (omits custody/steward refs);
v2 graduates when the bootstrap attestation chain lands. Idempotent re-runs
via content-addressed ids — POST returns 409 on the second invocation.

Substrate (no changes): operator-classification.schema.json,
rea_commitments.rs::OPERATE_DOORWAY_ACTION, auth/operator.rs.
EOF
)"
```

---

## Task 3: Remove the placeholder from the landing-page ContentNode JSON

The DB row's `blobHash` will be populated by the App pipeline's `stageSpaBlob` helper at deploy time (Task 5). seed-sqlite reads the JSON and must NOT clobber that value, so the JSON field is removed entirely.

**Files:**
- Modify: `/projects/elohim/genesis/data/lamad/content/elohim-host-landing.json`

- [ ] **Step 3.1: Apply the JSON edit**

Edit `/projects/elohim/genesis/data/lamad/content/elohim-host-landing.json` — replace:

```json
  "relatedNodeIds": [],
  "blobHash": "PLACEHOLDER_REPLACED_BY_SEED_SCRIPT",
  "metadata": {
    "category": "protocol-surface",
    "author": "Matthew Dowell",
    "embedStrategy": "iframe",
```

with:

```json
  "relatedNodeIds": [],
  "metadata": {
    "category": "protocol-surface",
    "author": "Matthew Dowell",
    "blobPopulatedAt": "deploy-time",
    "blobPopulatedBy": "Jenkinsfile:stageSpaBlob (root pipeline). The elohim-app browser bundle is zipped + uploaded as a blob; the hash is written into db/content/elohim-host-landing.blobHash and db/content/lamad-spa.blobHash. Intentionally absent from this source JSON so seed-sqlite does not clobber the deploy-time value on re-seed.",
    "embedStrategy": "iframe",
```

- [ ] **Step 3.2: Verify the JSON parses and the placeholder is gone**

```bash
cd /projects/elohim
python3 -c "
import json
data = json.load(open('genesis/data/lamad/content/elohim-host-landing.json'))
assert data['id'] == 'elohim-host-landing', f'id changed: {data[\"id\"]}'
assert 'blobHash' not in data, f'blobHash should be absent, found: {data.get(\"blobHash\")}'
assert data['metadata']['blobPopulatedAt'] == 'deploy-time'
assert data['contentFormat'] == 'html5-app'
print('OK: id=', data['id'], '  contentFormat=', data['contentFormat'], '  blobHash absent ✓')
"
```

Expected: `OK: id= elohim-host-landing   contentFormat= html5-app   blobHash absent ✓`

- [ ] **Step 3.3: Run schema validation against the project's validator**

```bash
cd /projects/elohim
pnpm run schema:validate 2>&1 | tail -15
```

Expected: validation passes for `elohim-host-landing.json` (no schema violation from the removed field). If a different unrelated file fails, that's pre-existing and not blocking.

- [ ] **Step 3.4: Commit the JSON change**

```bash
cd /projects/elohim
git add genesis/data/lamad/content/elohim-host-landing.json
git commit -m "$(cat <<'EOF'
fix(content): elohim-host-landing — drop placeholder blobHash

The PLACEHOLDER_REPLACED_BY_SEED_SCRIPT sentinel was a stub for a script
that never ran in CI. Reframe the blobHash as a deploy-time-populated
field: the App pipeline (Jenkinsfile:stageSpaBlob) uploads the
elohim-app browser bundle and writes the SHA256 into both
db/content/lamad-spa and db/content/elohim-host-landing.

Removing the field from the JSON means seed-sqlite leaves the
deploy-time DB value alone on re-seed (transformContent treats absent
blobHash as undefined; the API write omits the field; existing DB
value is preserved).

The metadata documents the seam so the next person understands why
the JSON looks "incomplete".
EOF
)"
```

---

## Task 4: Flip ROOT_APP_SLUG on both doorway manifests + symmetrize FEDERATION_PEERS

Both doorways must project the landing-page EPR as their root app. Federation peer URL on doorway-B should match the symmetric user-facing host so the pair reads cleanly.

**Files:**
- Modify: `/projects/elohim/genesis/orchestrator/manifests/doorway/alpha.yaml`
- Modify: `/projects/elohim/genesis/orchestrator/manifests/doorway/alpha-b.yaml`

- [ ] **Step 4.1: Patch alpha.yaml ROOT_APP_SLUG**

In `/projects/elohim/genesis/orchestrator/manifests/doorway/alpha.yaml`, replace:

```yaml
            - name: ROOT_APP_SLUG
              value: "lamad"
            # Elohim Agent SDK sidecar (runs in same pod)
```

with:

```yaml
            # Root app served at "/" — the landing-page EPR
            # (elohim-host-landing ContentNode, html5-app format) projected
            # through this doorway. Content-addressed truth lives in the DHT
            # blob keyed by its sha256; doorway resolves slug→blob via the
            # cache and serves the entry from app-file-cache.
            #
            # The same EPR is projected through doorway-alpha-b at elohim.host;
            # both doorways read the same content node, differing only in
            # surface URL + operator binding. See alpha-b.yaml.
            - name: ROOT_APP_SLUG
              value: "elohim-host-landing"
            # Elohim Agent SDK sidecar (runs in same pod)
```

- [ ] **Step 4.2: Patch alpha-b.yaml ROOT_APP_SLUG**

In `/projects/elohim/genesis/orchestrator/manifests/doorway/alpha-b.yaml`, replace:

```yaml
            - name: ROOT_APP_SLUG
              value: "lamad"
```

with:

```yaml
            # Root app served at "/" — the landing-page EPR shared with
            # doorway-alpha. Same elohim-host-landing ContentNode resolved
            # via the same shared storage pool; differing only in surface
            # URL (elohim.host vs alpha.elohim.host) + operator binding.
            - name: ROOT_APP_SLUG
              value: "elohim-host-landing"
```

- [ ] **Step 4.3: Patch alpha-b.yaml FEDERATION_PEERS for symmetric naming**

In `/projects/elohim/genesis/orchestrator/manifests/doorway/alpha-b.yaml`, replace:

```yaml
            # Federation: point at doorway-A. Discovery loop in
            # services::federation::spawn_peer_discovery_task will GET
            # this peer's /admin/capabilities, register both doorways in
            # the DHT, and run a heartbeat keeping the peer cache fresh.
            - name: FEDERATION_PEERS
              value: "https://doorway-alpha.elohim.host"
```

with:

```yaml
            # Federation: point at doorway-A on its user-facing surface
            # (alpha.elohim.host), symmetric with alpha.yaml's pointer at
            # elohim.host. doorway-alpha.elohim.host is still a valid alias
            # for the same backend service, but using the user-facing host
            # keeps the federation pair legible: alpha.elohim.host ↔
            # elohim.host. Discovery loop in
            # services::federation::spawn_peer_discovery_task will GET this
            # peer's /admin/capabilities, register both doorways in the DHT,
            # and run a heartbeat keeping the peer cache fresh.
            - name: FEDERATION_PEERS
              value: "https://alpha.elohim.host"
```

- [ ] **Step 4.4: Verify both YAMLs still parse to their expected document shape**

```bash
cd /projects/elohim
python3 -c "
import yaml
for path in [
    'genesis/orchestrator/manifests/doorway/alpha.yaml',
    'genesis/orchestrator/manifests/doorway/alpha-b.yaml',
]:
    with open(path) as f:
        docs = list(yaml.safe_load_all(f))
    kinds = [d.get('kind') for d in docs if d]
    assert kinds == ['Secret', 'Deployment', 'Service', 'Ingress'], f'{path}: unexpected kinds={kinds}'
    print(f'{path.split(\"/\")[-1]}: OK — 4 docs, Secret/Deployment/Service/Ingress')
"
```

Expected: both files report `OK — 4 docs, Secret/Deployment/Service/Ingress`.

- [ ] **Step 4.5: Verify ROOT_APP_SLUG is set correctly in both**

```bash
cd /projects/elohim
grep -A1 "name: ROOT_APP_SLUG" genesis/orchestrator/manifests/doorway/alpha.yaml genesis/orchestrator/manifests/doorway/alpha-b.yaml
```

Expected: both files show `value: "elohim-host-landing"`.

- [ ] **Step 4.6: Verify FEDERATION_PEERS pair is symmetric**

```bash
cd /projects/elohim
echo "alpha.yaml:"
grep -A1 "name: FEDERATION_PEERS" genesis/orchestrator/manifests/doorway/alpha.yaml
echo "alpha-b.yaml:"
grep -A1 "name: FEDERATION_PEERS" genesis/orchestrator/manifests/doorway/alpha-b.yaml
```

Expected:
```
alpha.yaml:    ...value: "https://elohim.host"
alpha-b.yaml:  ...value: "https://alpha.elohim.host"
```

- [ ] **Step 4.7: Commit the manifest changes**

```bash
cd /projects/elohim
git add genesis/orchestrator/manifests/doorway/alpha.yaml genesis/orchestrator/manifests/doorway/alpha-b.yaml
git commit -m "$(cat <<'EOF'
feat(doorway): project elohim-host-landing EPR via both alpha doorways

Both doorway-alpha (alpha.elohim.host) and doorway-alpha-b (elohim.host)
now serve the elohim-host-landing ContentNode as their ROOT_APP_SLUG.
One Matthew-stewarded landing EPR; two doorways routing the projection.

FEDERATION_PEERS on doorway-B retargeted from the internal alias
doorway-alpha.elohim.host → user-facing alpha.elohim.host, symmetric
with doorway-A's pointer at elohim.host. Backend resolves to the same
service; the change is naming legibility, not behavior.
EOF
)"
```

---

## Task 5: Extend stageSpaBlob to populate both content-node rows

The existing helper at `Jenkinsfile:223` zips the elohim-app browser bundle, uploads as a blob, and writes the hash into `db/content/lamad-spa`. Extend it to also write `db/content/elohim-host-landing` — one blob, two content rows pointing at the same hash, one bundle that serves both the lamad SPA surface and the landing-page EPR.

**Files:**
- Modify: `/projects/elohim/Jenkinsfile` (lines 223-247)

- [ ] **Step 5.1: Patch stageSpaBlob**

In `/projects/elohim/Jenkinsfile`, replace:

```groovy
def stageSpaBlob(String storageUrl, String distDir) {
    sh """
        cd '${distDir}'
        zip -r lamad-spa.zip .
        SPA_HASH=\$(sha256sum lamad-spa.zip | awk '{print \$1}')
        echo "SPA blob hash: \${SPA_HASH}"
        echo "SPA blob size: \$(du -h lamad-spa.zip | cut -f1)"

        # Upload ZIP as blob to storage
        curl -f -X PUT \
            -H 'Content-Type: application/zip' \
            --data-binary @lamad-spa.zip \
            "${storageUrl}/blob/\${SPA_HASH}" \
            || echo 'WARNING: Blob upload failed (storage may not be reachable)'

        # Update content node with new blobHash
        curl -f -X PUT \
            -H 'Content-Type: application/json' \
            -d '{"blobHash":"'\${SPA_HASH}'"}' \
            "${storageUrl}/db/content/lamad-spa" \
            || echo 'WARNING: Content node update failed'

        rm -f lamad-spa.zip
    """
}
```

with:

```groovy
def stageSpaBlob(String storageUrl, String distDir) {
    // Uploads the elohim-app browser bundle as a single blob and links it
    // to TWO content nodes:
    //
    //   db/content/lamad-spa            — the lamad SPA app surface
    //   db/content/elohim-host-landing  — the landing-page EPR projected by
    //                                     doorway-A (alpha.elohim.host) +
    //                                     doorway-B (elohim.host) as their
    //                                     ROOT_APP_SLUG
    //
    // One blob, two content rows, two projection surfaces. The JSON source
    // for both content nodes intentionally omits blobHash; the seed-sqlite
    // step does not overwrite the deploy-time value written here.
    sh """
        cd '${distDir}'
        zip -r lamad-spa.zip .
        SPA_HASH=\$(sha256sum lamad-spa.zip | awk '{print \$1}')
        echo "SPA blob hash: \${SPA_HASH}"
        echo "SPA blob size: \$(du -h lamad-spa.zip | cut -f1)"

        # Upload ZIP as blob to storage
        curl -f -X PUT \
            -H 'Content-Type: application/zip' \
            --data-binary @lamad-spa.zip \
            "${storageUrl}/blob/\${SPA_HASH}" \
            || echo 'WARNING: Blob upload failed (storage may not be reachable)'

        # Link blob to both content nodes that project this bundle.
        for slug in lamad-spa elohim-host-landing; do
            curl -f -X PUT \
                -H 'Content-Type: application/json' \
                -d '{"blobHash":"'\${SPA_HASH}'"}' \
                "${storageUrl}/db/content/\${slug}" \
                || echo "WARNING: Content node update failed for \${slug}"
        done

        rm -f lamad-spa.zip
    """
}
```

- [ ] **Step 5.2: Verify the helper still parses as part of the Jenkinsfile**

(Groovy CLI is not available in this environment. The structural sanity check is: confirm the new loop is well-bracketed and no other stage references stageSpaBlob with a changed signature.)

```bash
cd /projects/elohim
grep -n "stageSpaBlob" Jenkinsfile
```

Expected:
```
223:def stageSpaBlob(String storageUrl, String distDir) {
<line>: stageSpaBlob(storageUrl, "${env.WORKSPACE}/app/elohim-app/dist/elohim-app/browser")
```

Two references only — the def and the single existing call site. The signature is unchanged so no callers need updating.

- [ ] **Step 5.3: Verify the new content-node loop appears**

```bash
cd /projects/elohim
grep -n "for slug in lamad-spa elohim-host-landing" Jenkinsfile
```

Expected: exactly one match around line 251.

- [ ] **Step 5.4: Commit the Jenkinsfile change**

```bash
cd /projects/elohim
git add Jenkinsfile
git commit -m "$(cat <<'EOF'
feat(blob): stageSpaBlob links the SPA blob to both content rows

The elohim-app browser bundle is the bytes behind two ContentNode
projections:
  - lamad-spa            — the lamad SPA app surface
  - elohim-host-landing  — the landing-page EPR served by both
                           alpha + apex doorways as their root app

One blob, two rows pointing at the same SHA256. The JSON source for
elohim-host-landing intentionally omits blobHash so seed-sqlite leaves
the deploy-time DB value untouched.
EOF
)"
```

---

## Task 6: Add Seed Operator Bindings stage to the genesis Jenkinsfile

Wire `seed-operator-bindings.ts` into the deploy pipeline so Matthew lands as primary operator of both doorways on every alpha deploy.

**Files:**
- Modify: `/projects/elohim/genesis/Jenkinsfile`

- [ ] **Step 6.1: Insert the new stage between Seed Custody Commitments and Trigger M1 Cross-Pod Fetch**

In `/projects/elohim/genesis/Jenkinsfile`, find the closing of the `Seed Custody Commitments` stage (around line 1284) and the opening of `Trigger M1 Cross-Pod Fetch` (around line 1286), and insert the new stage:

Replace:

```groovy
                }
            }
        }

        stage('Trigger M1 Cross-Pod Fetch') {
```

with:

```groovy
                }
            }
        }

        stage('Seed Operator Bindings') {
            // Registers Matthew as primary operator of both alpha-cluster
            // doorways via REA operate-doorway commitments. Same human, two
            // doorway IDs (alpha-elohim-host + apex-elohim-host) — the
            // doorways differ in surface URL but share an operator.
            //
            // Per genesis/docs/plans/2026-05-19-doorway-stewardship-chain-design.md
            // and elohim/sdk/schemas/v1/objects/operator-classification.schema.json.
            // Schema v1 (omits custody/steward refs); v2 requires the full
            // attestation chain — graduate when bootstrap attestations land.
            when { allOf {
                expression { env.PIPELINE_SKIPPED != 'true' }
                expression { params.SEED_DATA }
            }}
            steps {
                container('builder') {
                    script {
                        def doorwayHost = env.RESOLVED_DOORWAY_HOST
                        catchError(buildResult: 'UNSTABLE', stageResult: 'UNSTABLE') {
                            dir('genesis/seeder') {
                                sh """#!/bin/bash
                                    set -euo pipefail
                                    echo "═══════════════════════════════════════════════════════════"
                                    echo "SEED OPERATOR BINDINGS (M5 matthew → alpha-elohim-host + apex-elohim-host)"
                                    echo "═══════════════════════════════════════════════════════════"
                                    echo "Doorway: ${doorwayHost}"
                                    echo ""
                                    DOORWAY_URL="${doorwayHost}" \\
                                      npx tsx src/seed-operator-bindings.ts
                                """
                            }
                        }
                    }
                }
            }
        }

        stage('Trigger M1 Cross-Pod Fetch') {
```

- [ ] **Step 6.2: Verify the stage placement**

```bash
cd /projects/elohim
grep -n "^        stage(" genesis/Jenkinsfile | tail -10
```

Expected: `stage('Seed Custody Commitments')` then `stage('Seed Operator Bindings')` then `stage('Trigger M1 Cross-Pod Fetch')` in that order.

- [ ] **Step 6.3: Verify the stage uses the existing env.RESOLVED_DOORWAY_HOST**

```bash
cd /projects/elohim
grep -B2 -A6 "Seed Operator Bindings" genesis/Jenkinsfile | head -20
```

Expected: the stage references `env.RESOLVED_DOORWAY_HOST` (set globally at line ~504) and gates on `params.SEED_DATA`.

- [ ] **Step 6.4: Commit the Jenkinsfile change**

```bash
cd /projects/elohim
git add genesis/Jenkinsfile
git commit -m "$(cat <<'EOF'
feat(ci): Seed Operator Bindings stage in genesis pipeline

New deploy-time stage that registers Matthew as primary operator of
both alpha-cluster doorways. Runs after Seed Custody Commitments
(needs the doorway live + auth healthy), before Trigger M1 Cross-Pod
Fetch. Gated on params.SEED_DATA; UNSTABLE rather than failed on
errors (catchError) so a partial seed doesn't block the rest of the
deploy.
EOF
)"
```

---

## Task 7: Integrated verification

Verify the whole chain in the local working tree before pushing.

- [ ] **Step 7.1: Re-run the full seeder spec suite (no regressions)**

```bash
cd /projects/elohim/genesis/seeder
pnpm exec vitest run 2>&1 | tail -10
```

Expected: all specs pass, including the new `seed-operator-bindings.spec.ts` (9 tests).

- [ ] **Step 7.2: Smoke-test the body builder via tsx**

```bash
cat > /tmp/check-operator-binding.mjs << 'EOF'
import { buildOperatorCommitmentBody } from '/projects/elohim/genesis/seeder/src/seed-operator-bindings.ts';

const alpha = buildOperatorCommitmentBody({
  operatorHumanId: 'human-matthew-manager',
  operatorArchetype: 'desktop',
  doorwayId: 'alpha-elohim-host',
  capabilities: ['*'],
  successionRole: 'primary',
  reachScope: 'stewards-only',
});
const apex = buildOperatorCommitmentBody({
  operatorHumanId: 'human-matthew-manager',
  operatorArchetype: 'desktop',
  doorwayId: 'apex-elohim-host',
  capabilities: ['*'],
  successionRole: 'primary',
  reachScope: 'stewards-only',
});

console.log('alpha id:', alpha.id);
console.log('apex id: ', apex.id);
console.log('distinct ids:', alpha.id !== apex.id);
console.log('action:', alpha.action);
console.log('inScopeOf:', alpha.inScopeOf);
console.log('metadata.schemaVersion:', alpha.metadata.schemaVersion);
EOF
cd /projects/elohim/genesis/seeder
pnpm exec tsx /tmp/check-operator-binding.mjs
rm -f /tmp/check-operator-binding.mjs
```

Expected:
```
alpha id: operate-doorway-<16hex>
apex id:  operate-doorway-<16hex>  (different from alpha)
distinct ids: true
action: operate-doorway
inScopeOf: [ 'doorway:alpha-elohim-host' ]
metadata.schemaVersion: 1
```

- [ ] **Step 7.3: Cross-file slug consistency check**

```bash
cd /projects/elohim
grep -l "elohim-host-landing" \
  Jenkinsfile \
  genesis/Jenkinsfile \
  genesis/data/lamad/content/elohim-host-landing.json \
  genesis/orchestrator/manifests/doorway/alpha.yaml \
  genesis/orchestrator/manifests/doorway/alpha-b.yaml \
  genesis/seeder/src/seed-operator-bindings.ts
```

Expected: all six files listed (each references the slug).

- [ ] **Step 7.4: Run the full repo typecheck (workspace-wide, scoped to seeder + app)**

```bash
cd /projects/elohim/genesis/seeder
pnpm exec tsc --noEmit -p . 2>&1 | grep -v "wait-for-drain" | grep -E "error|seed-operator-bindings" | head -10
```

Expected: no errors mentioning `seed-operator-bindings` or any new file.

- [ ] **Step 7.5: Inspect the full commit chain**

```bash
cd /projects/elohim
git log --oneline -7
git diff --stat HEAD~6..HEAD
```

Expected: 6 commits ahead of the original baseline, touching exactly the files in the File Structure table.

- [ ] **Step 7.6: Build the elohim-app locally to confirm the dist exists where stageSpaBlob expects it (optional sanity check)**

```bash
cd /projects/elohim
test -d app/elohim-app/dist/elohim-app/browser && echo "OK: dist exists at the path stageSpaBlob will zip from" || echo "INFO: dist not built locally; CI will build it. Not a blocker."
```

Expected: either "OK..." or "INFO..." — both are fine. If you want to build locally:

```bash
cd /projects/elohim
pnpm --filter elohim-app run build 2>&1 | tail -5
```

- [ ] **Step 7.7: Run the elohim-host-landing a2o scenarios locally (if feasible)**

```bash
cd /projects/elohim
ls genesis/a2o/features/protocol/landing-page-dogfood.feature
```

If the file exists, note: it cannot pass locally without a running doorway + storage stack. CI deploy will exercise it; if it's expected to pass against `hc:start` locally, see `app/elohim-app/CLAUDE.md`'s `hc:start:seed` command. This step is a sanity-check that the scenario file is present, not a green-bar requirement at this stage.

Expected: the file exists.

---

## Task 8: Push + deploy verification

Push to dev so the orchestrator dispatches downstream pipelines. The work is verified end-to-end on the alpha cluster.

- [ ] **Step 8.1: Pause and confirm with operator before pushing**

This is a real-world deploy. Pause and ask: "Six commits ready on dev. Push to remote so the orchestrator dispatches App + Genesis pipelines? This will trigger an alpha-cluster deploy." Only continue on explicit yes.

- [ ] **Step 8.2: Push to dev**

```bash
cd /projects/elohim
git push origin dev
```

Expected: push succeeds, no rejected refs.

- [ ] **Step 8.3: Watch the orchestrator dispatch**

Use the Jenkins MCP tools (or the ci-observer subagent) to track the cascade. Expected pipeline activations:
- `orchestrator` builds the build-manifest.json; computes pipeline set
- `App` pipeline → builds elohim-app → runs `stageSpaBlob` (uploads bundle, links both content rows)
- `Genesis` pipeline → runs the seed stages including the new `Seed Operator Bindings`

If `ci-observer` reports low confidence on any failure, escalate to `ci-investigator`.

- [ ] **Step 8.4: Smoke-test alpha.elohim.host on green deploy**

After both pipelines finish green:

```bash
curl -sI https://alpha.elohim.host/ | head -5
```

Expected: `HTTP/2 302` with `location: /apps/elohim-host-landing/index.html`.

```bash
curl -sI https://alpha.elohim.host/apps/elohim-host-landing/index.html | head -5
```

Expected: `HTTP/2 200` with `content-type: text/html`.

```bash
curl -s https://alpha.elohim.host/health/startup | python3 -m json.tool | head -30
```

Expected: `rootApp.ready: true` and `rootApp.slug: "elohim-host-landing"`.

- [ ] **Step 8.5: Smoke-test elohim.host (apex / doorway-B)**

```bash
curl -sI https://elohim.host/ | head -5
```

Expected: same 302 chain.

```bash
curl -s https://elohim.host/health/startup | python3 -m json.tool | head -30
```

Expected: `rootApp.ready: true` and `rootApp.slug: "elohim-host-landing"`.

- [ ] **Step 8.6: Verify Matthew's operator binding lands on both doorways**

```bash
curl -s "https://alpha.elohim.host/api/v1/commitments?action=operate-doorway" | python3 -m json.tool
```

Expected: at least one commitment with `provider` matching Matthew's deterministic peer_id and `inScopeOf: ["doorway:alpha-elohim-host"]`.

```bash
curl -s "https://elohim.host/api/v1/commitments?action=operate-doorway" | python3 -m json.tool
```

Expected: the equivalent for `doorway:apex-elohim-host`. (Both queries hit the shared storage pool so both rows will be visible regardless of which surface you query; the test is that BOTH scopes are present.)

- [ ] **Step 8.7: Verify federation peer discovery**

```bash
curl -s https://alpha.elohim.host/admin/federation/peers 2>&1 | head -20
```

Expected: shows `elohim.host` (or `apex-elohim-host` DOORWAY_ID) in the peer list with a recent heartbeat.

```bash
curl -s https://elohim.host/admin/federation/peers 2>&1 | head -20
```

Expected: shows `alpha.elohim.host` (or `alpha-elohim-host` DOORWAY_ID) in the peer list with a recent heartbeat.

If federation discovery isn't yet wired to populate the peer cache from the JWT auth — that's a known follow-up (federation peer discovery may need an additional step to graduate; track separately if so).

- [ ] **Step 8.8: Final landing-page render check**

Visit `https://alpha.elohim.host/` and `https://elohim.host/` in a browser. Both should show the elohim-app landing surface, fully loaded, no error overlay, no infinite spinner on the four-stage shell.

If both render: the chain is delivered. If either fails, capture the response body + browser console and triage via the doorway logs:

```bash
kubectl -n elohim-alpha logs deployment/elohim-doorway-alpha --tail=200
kubectl -n elohim-alpha logs deployment/elohim-doorway-alpha-b --tail=200
```

---

## What's intentionally NOT in this plan

- **Schema v2 graduation** for operator commitments (custody + steward attestation hashes). The script emits v1; v2 lands when the bootstrap attestation chain is in place. Tracked under the existing `2026-05-19-doorway-stewardship-chain-design.md` plan.
- **Retiring the `doorway-alpha.elohim.host` ingress alias.** Kept for now; harmless redundancy; remove once nothing internal references it.
- **A separate landing-page bundle distinct from the elohim-app build.** Current plan dogfoods the same bytes; if a slimmer landing-page-only bundle is desired later, it becomes a new EPR with its own slug, not a change to this chain.
- **Schema v2 of the operator-binding-view exposing succession plans + deputies.** Out of scope.

---

## Rollback

If any deploy step fails irrecoverably:

```bash
cd /projects/elohim
git log --oneline -7              # find the pre-plan HEAD
git revert --no-commit HEAD~5..HEAD
git commit -m "revert: landing-page EPR dual-doorway (rollback)"
git push origin dev
```

The orchestrator re-dispatches with the reverts; the doorways re-deploy with `ROOT_APP_SLUG=lamad`; the placeholder returns. The landing page goes back to its previous (broken) state but nothing else regresses.
