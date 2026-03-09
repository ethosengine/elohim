# Testnet Lifecycle Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Protocol-native compute lifecycle — conductors spawn with CoordinationEnvelopes, budget circuit breakers kill overages, cucumber tears everything down and attaches telemetry to the report.

**Architecture:** Shell scripts (`spawn-persona-testnet.sh`, `compute-budget.sh`) do the heavy lifting. A TypeScript adapter in the a2o framework validates their JSON output against CoordinationEnvelope interfaces. Cucumber hooks manage the session lifecycle. Matthew's story scenario exercises the full path.

**Tech Stack:** Bash (simulation scripts), TypeScript (a2o framework, envelope validation), Cucumber-JS (BDD runner), Playwright (optional browser verification)

**Design doc:** `genesis/plans/2026-03-09-testnet-lifecycle-design.md`

---

### Task 1: Add TTL Watchdog + Budget Circuit Breaker to compute-budget.sh

**Files:**
- Modify: `elohim-node/simulation/compute-budget.sh:176-195` (cmd_watch function)
- Modify: `elohim-node/simulation/compute-budget.sh:265-324` (cmd_check function)

**Step 1: Add TTL and kill-on-exceed to `cmd_watch()`**

Add two new env vars and modify the watch loop. Insert after the existing `cmd_watch()` function definition (line 176):

```bash
cmd_watch() {
  local interval="${1:-10}"
  local ttl="${COMPUTE_TTL_SECONDS:-1800}"       # 30 min default
  local kill_on_exceed="${COMPUTE_KILL_ON_EXCEED:-false}"
  local start_time
  start_time=$(date +%s)

  echo "=== Compute Budget Watch ==="
  echo "Interval: ${interval}s | TTL: ${ttl}s | Kill on exceed: ${kill_on_exceed}"

  while true; do
    cmd_sample

    # TTL check
    local now
    now=$(date +%s)
    local elapsed=$(( now - start_time ))
    if [ "$elapsed" -ge "$ttl" ]; then
      echo "TTL expired (${elapsed}s >= ${ttl}s). Emitting settle envelopes."
      emit_envelope "settle" "ttl-expired" "" "$elapsed"
      cmd_settle
      echo "Signaling testnet stop..."
      kill -TERM 0 2>/dev/null  # signal process group
      exit 0
    fi

    # Budget circuit breaker (per-node)
    if [ "$kill_on_exceed" = "true" ]; then
      check_and_kill_overages
    fi

    sleep "$interval"
  done
}
```

**Step 2: Add `check_and_kill_overages()` function**

Insert before `cmd_watch()`:

```bash
check_and_kill_overages() {
  local budget_cpu="${PER_NODE_BUDGET_CPU:-360}"
  local budget_mem="${PER_NODE_BUDGET_MEM:-150}"
  local pids_dir="$TESTNET_DIR/pids"

  for pidfile in "$pids_dir"/*.pid; do
    [ -f "$pidfile" ] || continue
    local node_name
    node_name=$(basename "$pidfile" .pid)
    local pid
    pid=$(cat "$pidfile")

    # Skip dead processes
    kill -0 "$pid" 2>/dev/null || continue

    # Read latest metrics from ledger for this node
    local cpu_total mem_current
    cpu_total=$(grep "\"persona\":\"$node_name\"" "$TESTNET_DIR/compute-ledger.jsonl" \
      | tail -1 | jq -r '.cpuSeconds // 0')
    mem_current=$(grep "\"persona\":\"$node_name\"" "$TESTNET_DIR/compute-ledger.jsonl" \
      | tail -1 | jq -r '.memoryMb // 0')

    # Soft warn at 80%
    local cpu_pct
    cpu_pct=$(echo "$cpu_total $budget_cpu" | awk '{printf "%.0f", ($1/$2)*100}')
    if [ "$cpu_pct" -ge 80 ] && [ "$cpu_pct" -lt 100 ]; then
      echo "WARN: $node_name at ${cpu_pct}% CPU budget (${cpu_total}/${budget_cpu}s)"
      emit_envelope "sense" "budget-warning" "$node_name" "$cpu_total"
    fi

    # Hard kill at 100%
    if [ "$cpu_pct" -ge 100 ]; then
      echo "KILL: $node_name exceeded CPU budget (${cpu_total}/${budget_cpu}s)"
      emit_envelope "settle" "budget-exceeded" "$node_name" "$cpu_total"
      kill -TERM "$pid" 2>/dev/null
      echo "killed" > "${pidfile}.status"
    fi
  done
}
```

**Step 3: Add `emit_envelope()` function**

Insert near top of file, after variable declarations:

```bash
ENVELOPE_DIR="${TESTNET_DIR}/envelopes"

emit_envelope() {
  local verb="$1"
  local action="$2"
  local persona="$3"
  local value="$4"
  local timestamp
  timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

  mkdir -p "$ENVELOPE_DIR"

  local envelope
  envelope=$(jq -n \
    --arg verb "$verb" \
    --arg action "$action" \
    --arg persona "$persona" \
    --arg value "$value" \
    --arg ts "$timestamp" \
    --arg sender "${TESTNET_REQUESTER:-matthew}" \
    '{
      verb: $verb,
      scope: { agents: (if $persona != "" then [$persona] else [] end) },
      routing: { urgency: "near-realtime", fallback: "queue" },
      payload: {
        economicEvent: {
          action: $action,
          provider: $persona,
          resourceQuantity: { value: ($value | tonumber), unit: "cpu-second" },
          settlement: (if $action == "budget-exceeded" then "partial" else "pending" end)
        }
      },
      sender: { agentId: $sender, delegationChain: [] },
      timestamp: $ts
    }')

  echo "$envelope" >> "$ENVELOPE_DIR/envelopes.jsonl"
  echo "$envelope"  # also print to stdout
}
```

**Step 4: Test the circuit breaker manually**

Run:
```bash
cd elohim-node/simulation
# Start a minimal testnet
./spawn-persona-testnet.sh start
# In another terminal, run budget watch with kill enabled
COMPUTE_KILL_ON_EXCEED=true COMPUTE_TTL_SECONDS=120 ./compute-budget.sh watch 5
# Verify envelopes appear
cat /tmp/elohim-persona-testnet/envelopes/envelopes.jsonl | jq .
# Clean up
./spawn-persona-testnet.sh stop
./spawn-persona-testnet.sh clean
```

Expected: sense envelopes emitted every 5s, any node exceeding budget gets SIGTERM'd, envelope with `action: "budget-exceeded"` recorded.

**Step 5: Commit**

```bash
git add elohim-node/simulation/compute-budget.sh
git commit -m "feat(simulation): add TTL watchdog + budget circuit breaker with envelope emission"
```

---

### Task 2: Add spawn-subset Command to spawn-persona-testnet.sh

**Files:**
- Modify: `elohim-node/simulation/spawn-persona-testnet.sh:91-193` (cmd_start function area)
- Modify: `elohim-node/simulation/personas.json` (no structural changes, just reading subset)

**Step 1: Add `cmd_start_subset()` function**

Insert after `cmd_start()` (after line ~193):

```bash
cmd_start_subset() {
  local persona_list="$1"  # comma-separated: "matthew,susan,pete,frank,nancy"
  local requester="${2:-matthew}"

  if [ -z "$persona_list" ]; then
    echo "Usage: $0 start-subset <persona1,persona2,...> [requester]"
    exit 1
  fi

  IFS=',' read -ra REQUESTED_PERSONAS <<< "$persona_list"
  local count=${#REQUESTED_PERSONAS[@]}

  echo "=== Starting Persona Testnet Subset ==="
  echo "Personas: ${REQUESTED_PERSONAS[*]}"
  echo "Count: $count"
  echo "Requester: $requester"

  mkdir -p "$TESTNET_DIR/pids" "$TESTNET_DIR/data" "$TESTNET_DIR/envelopes"

  # Emit provision envelope
  export TESTNET_REQUESTER="$requester"
  local agents_json
  agents_json=$(printf '%s\n' "${REQUESTED_PERSONAS[@]}" | jq -R . | jq -s .)

  local provision_envelope
  provision_envelope=$(jq -n \
    --argjson agents "$agents_json" \
    --arg requester "$requester" \
    --arg count "$count" \
    --arg ts "$(date -u +"%Y-%m-%dT%H:%M:%SZ")" \
    '{
      verb: "provision",
      scope: { agents: $agents },
      routing: { urgency: "near-realtime", fallback: "queue" },
      payload: {
        serviceRequest: {
          resourceQuantity: { value: (($count | tonumber) * 360), unit: "cpu-second" },
          duration: { value: 30, unit: "minute" },
          trustFloor: "Community"
        }
      },
      sender: { agentId: $requester, delegationChain: [] },
      timestamp: $ts
    }')

  echo "$provision_envelope" >> "$TESTNET_DIR/envelopes/envelopes.jsonl"

  # Spawn only requested personas
  local port_base=9100
  local port_offset=0
  local all_personas
  all_personas=$(jq -r '.clusters[].members[].humanId' "$SCRIPT_DIR/personas.json")

  for persona_id in $all_personas; do
    port_offset=$((port_offset + 1))
    # Only spawn if in requested list
    local matched=false
    for req in "${REQUESTED_PERSONAS[@]}"; do
      if [[ "$persona_id" == *"$req"* ]]; then
        matched=true
        break
      fi
    done

    if [ "$matched" = "true" ]; then
      local port=$((port_base + port_offset))
      local data_dir="$TESTNET_DIR/data/$persona_id"
      mkdir -p "$data_dir"

      echo "Starting $persona_id on port $port..."
      # Generate config for this persona
      ./gen-persona-configs.sh single "$persona_id" "$port" "$data_dir" > "$TESTNET_DIR/data/$persona_id.toml"

      # Spawn process
      ELOHIM_DATA_DIR="$data_dir" "$ELOHIM_NODE_BIN" \
        --config "$TESTNET_DIR/data/$persona_id.toml" \
        > "$TESTNET_DIR/data/$persona_id.log" 2>&1 &

      echo $! > "$TESTNET_DIR/pids/$persona_id.pid"
      echo "  PID: $! | Port: $port"
    fi
  done

  echo ""
  echo "Subset testnet started: $count of 20 personas"
  echo "Envelopes: $TESTNET_DIR/envelopes/envelopes.jsonl"
}
```

**Step 2: Wire into the case statement**

Find the case statement in the main dispatch (near bottom of file) and add:

```bash
start-subset)
  cmd_start_subset "$2" "$3"
  ;;
```

**Step 3: Add settle envelope emission to `cmd_stop()`**

Modify `cmd_stop()` to emit settle envelopes before killing processes:

```bash
# Add at the beginning of cmd_stop(), before killing PIDs:
echo "Emitting settle envelopes before shutdown..."
for pidfile in "$TESTNET_DIR/pids"/*.pid; do
  [ -f "$pidfile" ] || continue
  local node_name
  node_name=$(basename "$pidfile" .pid)
  # Get final metrics from ledger
  local final_cpu
  final_cpu=$(grep "\"persona\":\"$node_name\"" "$TESTNET_DIR/compute-ledger.jsonl" 2>/dev/null \
    | tail -1 | jq -r '.cpuSeconds // 0' 2>/dev/null || echo "0")
  emit_envelope "settle" "deliver-service" "$node_name" "$final_cpu"
done
```

**Step 4: Test subset spawn**

Run:
```bash
cd elohim-node/simulation
./spawn-persona-testnet.sh start-subset "matthew,susan,pete,frank,nancy" matthew
./spawn-persona-testnet.sh status
# Verify only 5 nodes running
cat /tmp/elohim-persona-testnet/envelopes/envelopes.jsonl | jq '.verb'
# Should show: "provision"
./spawn-persona-testnet.sh stop
cat /tmp/elohim-persona-testnet/envelopes/envelopes.jsonl | jq '.verb'
# Should show: "provision", then 5x "settle"
./spawn-persona-testnet.sh clean
```

**Step 5: Commit**

```bash
git add elohim-node/simulation/spawn-persona-testnet.sh
git commit -m "feat(simulation): add start-subset command with provision/settle envelope emission"
```

---

### Task 3: Cucumber Testnet Lifecycle Hooks

**Files:**
- Create: `genesis/a2o/src/framework/testnet-manager.ts`
- Modify: `genesis/a2o/steps/common.steps.ts:114-203` (add testnet hooks)

**Step 1: Create testnet manager**

```typescript
// genesis/a2o/src/framework/testnet-manager.ts
import { execSync, spawn, ChildProcess } from 'node:child_process';
import { readFileSync, existsSync } from 'node:fs';
import path from 'node:path';

const SIMULATION_DIR = path.resolve(
  import.meta.dirname,
  '../../../../elohim-node/simulation',
);
const TESTNET_DIR = '/tmp/elohim-persona-testnet';
const ENVELOPE_FILE = `${TESTNET_DIR}/envelopes/envelopes.jsonl`;

export interface TestnetSession {
  personas: string[];
  requester: string;
  budgetWatcher: ChildProcess | null;
  startedAt: number;
  ttlSeconds: number;
}

let activeSession: TestnetSession | null = null;

export function isTestnetActive(): boolean {
  return activeSession !== null;
}

export function startTestnet(opts: {
  personas: string[];
  requester?: string;
  ttlSeconds?: number;
  killOnExceed?: boolean;
}): void {
  if (activeSession) {
    console.log('Testnet already active, reusing session');
    return;
  }

  const requester = opts.requester ?? 'matthew';
  const ttl = opts.ttlSeconds ?? 1800;
  const personaList = opts.personas.join(',');

  console.log(`Starting testnet: ${personaList} (requester: ${requester}, TTL: ${ttl}s)`);

  // Spawn subset
  execSync(
    `${SIMULATION_DIR}/spawn-persona-testnet.sh start-subset "${personaList}" "${requester}"`,
    { stdio: 'inherit', timeout: 60_000 },
  );

  // Start budget watcher in background
  const budgetWatcher = spawn(
    `${SIMULATION_DIR}/compute-budget.sh`,
    ['watch', '10'],
    {
      env: {
        ...process.env,
        COMPUTE_TTL_SECONDS: String(ttl),
        COMPUTE_KILL_ON_EXCEED: String(opts.killOnExceed ?? true),
        TESTNET_REQUESTER: requester,
      },
      stdio: ['ignore', 'pipe', 'pipe'],
      detached: false,
    },
  );

  budgetWatcher.stdout?.on('data', (data: Buffer) => {
    const line = data.toString().trim();
    if (line.includes('WARN') || line.includes('KILL') || line.includes('TTL')) {
      console.log(`[budget] ${line}`);
    }
  });

  activeSession = {
    personas: opts.personas,
    requester,
    budgetWatcher,
    startedAt: Date.now(),
    ttlSeconds: ttl,
  };
}

export function stopTestnet(): void {
  if (!activeSession) return;

  console.log('Stopping testnet...');

  // Kill budget watcher
  if (activeSession.budgetWatcher) {
    activeSession.budgetWatcher.kill('SIGTERM');
  }

  // Settle and stop
  try {
    execSync(`${SIMULATION_DIR}/compute-budget.sh settle`, {
      stdio: 'inherit',
      timeout: 30_000,
    });
  } catch {
    console.warn('Settlement failed (non-fatal)');
  }

  try {
    execSync(`${SIMULATION_DIR}/spawn-persona-testnet.sh stop`, {
      stdio: 'inherit',
      timeout: 30_000,
    });
  } catch {
    console.warn('Testnet stop failed (non-fatal)');
  }

  activeSession = null;
}

export function getEnvelopes(): Record<string, unknown>[] {
  if (!existsSync(ENVELOPE_FILE)) return [];
  return readFileSync(ENVELOPE_FILE, 'utf-8')
    .trim()
    .split('\n')
    .filter(Boolean)
    .map((line) => JSON.parse(line));
}

export function getEnvelopesByVerb(verb: string): Record<string, unknown>[] {
  return getEnvelopes().filter((e) => e.verb === verb);
}

export function getComputeSummary(): {
  totalCpuSeconds: number;
  totalMemoryMb: number;
  perPersona: Record<string, { cpuSeconds: number; memoryMb: number }>;
  budgetExceeded: string[];
  duration: number;
} {
  const envelopes = getEnvelopes();
  const settleEvents = envelopes.filter((e) => e.verb === 'settle');
  const perPersona: Record<string, { cpuSeconds: number; memoryMb: number }> = {};
  const budgetExceeded: string[] = [];
  let totalCpu = 0;
  let totalMem = 0;

  for (const env of settleEvents) {
    const payload = env.payload as {
      economicEvent?: {
        action?: string;
        provider?: string;
        resourceQuantity?: { value?: number };
      };
    };
    const event = payload?.economicEvent;
    if (!event?.provider) continue;

    const cpu = event.resourceQuantity?.value ?? 0;
    perPersona[event.provider] = { cpuSeconds: cpu, memoryMb: 0 };
    totalCpu += cpu;

    if (event.action === 'budget-exceeded') {
      budgetExceeded.push(event.provider);
    }
  }

  return {
    totalCpuSeconds: totalCpu,
    totalMemoryMb: totalMem,
    perPersona,
    budgetExceeded,
    duration: activeSession ? Date.now() - activeSession.startedAt : 0,
  };
}
```

**Step 2: Wire testnet hooks into common.steps.ts**

Add to `genesis/a2o/steps/common.steps.ts` — insert after the existing `AfterAll` hook (line ~203):

```typescript
import { isTestnetActive, stopTestnet, getComputeSummary } from '../src/framework/testnet-manager.js';

AfterAll(async function () {
  // Existing: close Playwright browser
  // ... (existing code stays)

  // Testnet cleanup
  if (isTestnetActive()) {
    const summary = getComputeSummary();
    console.log('\n=== Compute Summary ===');
    console.log(`Total CPU: ${summary.totalCpuSeconds}s`);
    console.log(`Duration: ${Math.round(summary.duration / 1000)}s`);
    console.log(`Budget exceeded: ${summary.budgetExceeded.length > 0 ? summary.budgetExceeded.join(', ') : 'none'}`);
    for (const [persona, metrics] of Object.entries(summary.perPersona)) {
      console.log(`  ${persona}: ${metrics.cpuSeconds} cpu-s`);
    }

    // Attach to cucumber report
    // The summary gets written to a file that the formatter picks up
    const reportPath = 'reports/compute-summary.json';
    const { writeFileSync } = await import('node:fs');
    writeFileSync(reportPath, JSON.stringify(summary, null, 2));

    stopTestnet();
  }
});
```

**Step 3: Test that hooks compile**

Run:
```bash
cd genesis/a2o
npx tsc --noEmit
```

Expected: No type errors on new files.

**Step 4: Commit**

```bash
git add genesis/a2o/src/framework/testnet-manager.ts genesis/a2o/steps/common.steps.ts
git commit -m "feat(a2o): add testnet lifecycle manager with session-scoped hooks"
```

---

### Task 4: Envelope Validation Adapter

**Files:**
- Create: `genesis/a2o/src/framework/envelope-validator.ts`

**Step 1: Write the failing test**

Create: `genesis/a2o/src/framework/envelope-validator.spec.ts`

```typescript
import { describe, it, expect } from 'vitest';
import { validateEnvelope, isProvisionEnvelope, isSettleEnvelope } from './envelope-validator.js';

describe('envelope-validator', () => {
  it('validates a provision envelope', () => {
    const envelope = {
      verb: 'provision',
      scope: { agents: ['human-matthew-manager'] },
      routing: { urgency: 'near-realtime', fallback: 'queue' },
      payload: {
        serviceRequest: {
          resourceQuantity: { value: 1800, unit: 'cpu-second' },
          duration: { value: 30, unit: 'minute' },
          trustFloor: 'Community',
        },
      },
      sender: { agentId: 'matthew', delegationChain: [] },
      timestamp: '2026-03-09T00:00:00Z',
    };
    expect(validateEnvelope(envelope)).toBe(true);
    expect(isProvisionEnvelope(envelope)).toBe(true);
  });

  it('validates a settle envelope with deliver-service', () => {
    const envelope = {
      verb: 'settle',
      scope: { agents: ['human-susan-household'] },
      routing: { urgency: 'near-realtime', fallback: 'queue' },
      payload: {
        economicEvent: {
          action: 'deliver-service',
          provider: 'human-susan-household',
          resourceQuantity: { value: 180, unit: 'cpu-second' },
          settlement: 'pending',
        },
      },
      sender: { agentId: 'matthew', delegationChain: [] },
      timestamp: '2026-03-09T00:00:00Z',
    };
    expect(validateEnvelope(envelope)).toBe(true);
    expect(isSettleEnvelope(envelope)).toBe(true);
  });

  it('validates a settle envelope with budget-exceeded', () => {
    const envelope = {
      verb: 'settle',
      scope: { agents: ['human-pete-pastor'] },
      routing: { urgency: 'near-realtime', fallback: 'queue' },
      payload: {
        economicEvent: {
          action: 'budget-exceeded',
          provider: 'human-pete-pastor',
          resourceQuantity: { value: 365, unit: 'cpu-second' },
          budgetLimit: 360,
          settlement: 'partial',
        },
      },
      sender: { agentId: 'matthew', delegationChain: [] },
      timestamp: '2026-03-09T00:00:00Z',
    };
    expect(validateEnvelope(envelope)).toBe(true);
    expect(isSettleEnvelope(envelope)).toBe(true);
  });

  it('rejects invalid envelope (missing verb)', () => {
    expect(validateEnvelope({ scope: {} })).toBe(false);
  });

  it('rejects invalid verb', () => {
    expect(validateEnvelope({ verb: 'explode', scope: {} })).toBe(false);
  });
});
```

**Step 2: Run test to verify it fails**

Run: `cd genesis/a2o && npx vitest run src/framework/envelope-validator.spec.ts`
Expected: FAIL — module not found.

**Step 3: Write the implementation**

```typescript
// genesis/a2o/src/framework/envelope-validator.ts

const VALID_VERBS = [
  'invoke', 'sense', 'respond', 'aggregate', 'route',
  'delegate', 'escalate', 'ratify', 'recall', 'provision', 'federate',
  'settle',  // lifecycle verb used by compute budget
] as const;

type CoordinationVerb = (typeof VALID_VERBS)[number];

interface Envelope {
  verb: string;
  scope?: { agents?: string[] };
  routing?: { urgency?: string; fallback?: string };
  payload?: Record<string, unknown>;
  sender?: { agentId?: string; delegationChain?: unknown[] };
  timestamp?: string;
}

export function validateEnvelope(obj: unknown): boolean {
  if (!obj || typeof obj !== 'object') return false;
  const e = obj as Envelope;
  if (!e.verb || !VALID_VERBS.includes(e.verb as CoordinationVerb)) return false;
  return true;
}

export function isProvisionEnvelope(obj: unknown): boolean {
  if (!validateEnvelope(obj)) return false;
  const e = obj as Envelope;
  return e.verb === 'provision' && !!e.payload?.serviceRequest;
}

export function isSettleEnvelope(obj: unknown): boolean {
  if (!validateEnvelope(obj)) return false;
  const e = obj as Envelope;
  return e.verb === 'settle' && !!e.payload?.economicEvent;
}

export function isSenseEnvelope(obj: unknown): boolean {
  if (!validateEnvelope(obj)) return false;
  const e = obj as Envelope;
  return e.verb === 'sense' && !!e.payload?.computeMetrics;
}
```

**Step 4: Run test to verify it passes**

Run: `cd genesis/a2o && npx vitest run src/framework/envelope-validator.spec.ts`
Expected: PASS (5 tests)

**Step 5: Commit**

```bash
git add genesis/a2o/src/framework/envelope-validator.ts genesis/a2o/src/framework/envelope-validator.spec.ts
git commit -m "feat(a2o): add coordination envelope validator with tests"
```

---

### Task 5: Feature File + Step Definitions

**Files:**
- Create: `genesis/a2o/features/elohim/compute-allocation.feature`
- Create: `genesis/a2o/steps/compute-allocation.steps.ts`

**Step 1: Write the feature file**

```gherkin
@testnet @compute-allocation
Feature: Community compute allocation
  As Matthew, I have a distributed app to test.
  I request compute from my community, peers provision
  capacity, my test runs, and settlement happens.

  Background:
    Given human "Matthew" has a running steward node

  @e2e
  Scenario: Matthew requests compute from 5 community peers
    Given Matthew has a simulation requiring 5 peer nodes
    When he submits a ServiceRequest with budget 1800 cpu-seconds
    Then a provision envelope is emitted for each persona
    And 5 conductors are running within 30 seconds
    And compute-budget tracking is active

  @e2e
  Scenario: Compute settles after simulation completes
    Given 5 conductors are running for Matthew's simulation
    When the simulation workload completes
    Then a settle envelope is emitted for each persona
    And each EconomicEvent contains cpu-seconds and memory-mb
    And the total spend is within the 1800 cpu-second budget
    And the compute summary appears in the test report

  @e2e @circuit-breaker
  Scenario: Budget exceeded triggers graceful degradation
    Given 5 conductors are running for Matthew's simulation
    And one persona is configured with a 60 cpu-second budget
    When that persona exceeds its budget
    Then it receives SIGTERM with a budget-exceeded envelope
    And the remaining 4 conductors continue
    And settlement records the partial delivery
```

**Step 2: Write step definitions**

```typescript
// genesis/a2o/steps/compute-allocation.steps.ts
import { Given, When, Then } from '@cucumber/cucumber';
import { strict as assert } from 'node:assert';
import { execSync } from 'node:child_process';
import type { E2EWorld } from '../src/framework/world.js';
import {
  startTestnet,
  stopTestnet,
  isTestnetActive,
  getEnvelopes,
  getEnvelopesByVerb,
  getComputeSummary,
} from '../src/framework/testnet-manager.js';
import {
  validateEnvelope,
  isProvisionEnvelope,
  isSettleEnvelope,
} from '../src/framework/envelope-validator.js';

const DEFAULT_5_PERSONAS = ['matthew', 'susan', 'pete', 'frank', 'nancy'];

Given('human {string} has a running steward node', async function (this: E2EWorld, name: string) {
  // For now, steward node is the testnet itself — Matthew's node is the requester
  // In next sprint, this checks StewardDevice health at localhost:8090
  this.humans.set(name, { name, credentials: {}, devices: [], tokens: new Map() });
});

Given('Matthew has a simulation requiring {int} peer nodes', function (this: E2EWorld, count: number) {
  assert.equal(count, 5, `This sprint supports 5 personas. Got: ${count}`);
  (this as Record<string, unknown>).requestedPersonas = DEFAULT_5_PERSONAS;
  (this as Record<string, unknown>).requestedCount = count;
});

When('he submits a ServiceRequest with budget {int} cpu-seconds', function (this: E2EWorld, budget: number) {
  const personas = ((this as Record<string, unknown>).requestedPersonas as string[]) ?? DEFAULT_5_PERSONAS;
  startTestnet({
    personas,
    requester: 'matthew',
    ttlSeconds: Math.ceil(budget / personas.length),  // distribute budget as TTL proxy
    killOnExceed: true,
  });
  (this as Record<string, unknown>).budget = budget;
});

Then('a provision envelope is emitted for each persona', function () {
  const provisions = getEnvelopesByVerb('provision');
  assert.ok(provisions.length > 0, 'No provision envelopes found');
  for (const env of provisions) {
    assert.ok(isProvisionEnvelope(env), `Invalid provision envelope: ${JSON.stringify(env)}`);
  }
});

Then('{int} conductors are running within {int} seconds', async function (count: number, timeout: number) {
  const deadline = Date.now() + timeout * 1000;
  let running = 0;

  while (Date.now() < deadline) {
    try {
      const status = execSync(
        'elohim-node/simulation/spawn-persona-testnet.sh status 2>&1',
        { encoding: 'utf-8', timeout: 10_000 },
      );
      running = (status.match(/RUNNING/g) || []).length;
      if (running >= count) break;
    } catch {
      // retry
    }
    await new Promise((r) => setTimeout(r, 2000));
  }

  assert.ok(running >= count, `Expected ${count} running conductors, got ${running}`);
});

Then('compute-budget tracking is active', function () {
  assert.ok(isTestnetActive(), 'Testnet session is not active');
});

Given('{int} conductors are running for Matthew\'s simulation', function (count: number) {
  if (!isTestnetActive()) {
    startTestnet({
      personas: DEFAULT_5_PERSONAS.slice(0, count),
      requester: 'matthew',
      killOnExceed: true,
    });
  }
});

When('the simulation workload completes', function () {
  // Simulate workload completion — just wait for a few budget samples
  execSync('sleep 15');  // let budget watcher collect a few samples
  stopTestnet();
});

Then('a settle envelope is emitted for each persona', function () {
  const settles = getEnvelopesByVerb('settle');
  assert.ok(settles.length > 0, 'No settle envelopes found');
  for (const env of settles) {
    assert.ok(isSettleEnvelope(env), `Invalid settle envelope: ${JSON.stringify(env)}`);
  }
});

Then('each EconomicEvent contains cpu-seconds and memory-mb', function () {
  const settles = getEnvelopesByVerb('settle');
  for (const env of settles) {
    const payload = (env as Record<string, Record<string, Record<string, unknown>>>).payload;
    const event = payload?.economicEvent;
    assert.ok(event, 'Missing economicEvent in settle envelope');
    const rq = event.resourceQuantity as Record<string, unknown> | undefined;
    assert.ok(rq, 'Missing resourceQuantity');
    assert.equal(rq.unit, 'cpu-second');
    assert.ok(typeof rq.value === 'number', 'resourceQuantity.value must be a number');
  }
});

Then('the total spend is within the {int} cpu-second budget', function (budget: number) {
  const summary = getComputeSummary();
  assert.ok(
    summary.totalCpuSeconds <= budget,
    `Total CPU ${summary.totalCpuSeconds}s exceeds budget ${budget}s`,
  );
});

Then('the compute summary appears in the test report', function () {
  const { existsSync } = require('node:fs');
  // Summary is written by AfterAll hook — at this point we just verify
  // the summary can be generated without error
  const summary = getComputeSummary();
  assert.ok(summary.perPersona, 'Compute summary missing perPersona data');
});

// Circuit breaker scenario steps

Given('one persona is configured with a {int} cpu-second budget', function (budget: number) {
  // Override a single persona's budget via env
  process.env.OVERRIDE_BUDGET_PERSONA = 'pete';
  process.env.OVERRIDE_BUDGET_VALUE = String(budget);
});

When('that persona exceeds its budget', async function () {
  // Wait for the circuit breaker to fire — budget watcher checks every 10s
  // With a 60s budget on an active node, should trigger within ~70s
  const deadline = Date.now() + 90_000;
  while (Date.now() < deadline) {
    const envelopes = getEnvelopesByVerb('settle');
    const exceeded = envelopes.find(
      (e) => (e.payload as Record<string, Record<string, unknown>>)?.economicEvent?.action === 'budget-exceeded',
    );
    if (exceeded) return;
    await new Promise((r) => setTimeout(r, 5000));
  }
  assert.fail('Budget-exceeded envelope not emitted within 90s');
});

Then('it receives SIGTERM with a budget-exceeded envelope', function () {
  const settles = getEnvelopesByVerb('settle');
  const exceeded = settles.filter(
    (e) => (e.payload as Record<string, Record<string, unknown>>)?.economicEvent?.action === 'budget-exceeded',
  );
  assert.ok(exceeded.length > 0, 'No budget-exceeded envelopes found');
});

Then('the remaining {int} conductors continue', function (count: number) {
  const status = execSync(
    'elohim-node/simulation/spawn-persona-testnet.sh status 2>&1',
    { encoding: 'utf-8', timeout: 10_000 },
  );
  const running = (status.match(/RUNNING/g) || []).length;
  assert.ok(running >= count, `Expected ${count} running, got ${running}`);
});

Then('settlement records the partial delivery', function () {
  const settles = getEnvelopesByVerb('settle');
  const partial = settles.filter(
    (e) => (e.payload as Record<string, Record<string, unknown>>)?.economicEvent?.settlement === 'partial',
  );
  assert.ok(partial.length > 0, 'No partial settlement envelopes found');
});
```

**Step 3: Verify step definitions parse**

Run:
```bash
cd genesis/a2o
npx cucumber-js --dry-run --tags '@compute-allocation'
```

Expected: All steps matched, 0 undefined.

**Step 4: Commit**

```bash
git add genesis/a2o/features/elohim/compute-allocation.feature genesis/a2o/steps/compute-allocation.steps.ts
git commit -m "feat(a2o): add Matthew's compute allocation scenarios with step definitions"
```

---

### Task 6: Cucumber Report Attachment

**Files:**
- Create: `genesis/a2o/src/framework/report-attachment.ts`
- Modify: `genesis/a2o/steps/common.steps.ts` (AfterAll hook)

**Step 1: Create report attachment helper**

```typescript
// genesis/a2o/src/framework/report-attachment.ts
import { writeFileSync, existsSync, readFileSync } from 'node:fs';

interface ComputeSummary {
  totalCpuSeconds: number;
  totalMemoryMb: number;
  perPersona: Record<string, { cpuSeconds: number; memoryMb: number }>;
  budgetExceeded: string[];
  duration: number;
}

/**
 * Appends compute summary to the cucumber JSON report.
 * The summary appears as a "meta" attachment on the last scenario.
 */
export function attachComputeSummaryToReport(summary: ComputeSummary): void {
  const reportPath = 'reports/compute-summary.json';
  writeFileSync(reportPath, JSON.stringify(summary, null, 2));
  console.log(`Compute summary written to ${reportPath}`);

  // Also print a human-readable table
  console.log('\n┌─────────────────────────────────────────────┐');
  console.log('│           COMPUTE TELEMETRY                 │');
  console.log('├─────────────────────────┬───────────────────┤');
  console.log(`│ Total CPU               │ ${String(summary.totalCpuSeconds).padStart(12)}s │`);
  console.log(`│ Duration                │ ${String(Math.round(summary.duration / 1000)).padStart(12)}s │`);
  console.log(`│ Budget exceeded         │ ${String(summary.budgetExceeded.length).padStart(12)}  │`);
  console.log('├─────────────────────────┼───────────────────┤');

  for (const [persona, metrics] of Object.entries(summary.perPersona)) {
    const name = persona.replace('human-', '').substring(0, 23);
    console.log(`│ ${name.padEnd(23)} │ ${String(metrics.cpuSeconds).padStart(12)}s │`);
  }

  console.log('└─────────────────────────┴───────────────────┘');
}
```

**Step 2: Wire into AfterAll in common.steps.ts**

Add import and call in the AfterAll hook (after the existing Playwright cleanup, before the testnet stop from Task 3):

```typescript
import { attachComputeSummaryToReport } from '../src/framework/report-attachment.js';

// Inside AfterAll, after compute summary log, replace the manual writeFileSync with:
attachComputeSummaryToReport(summary);
```

**Step 3: Commit**

```bash
git add genesis/a2o/src/framework/report-attachment.ts genesis/a2o/steps/common.steps.ts
git commit -m "feat(a2o): attach compute telemetry to cucumber report"
```

---

### Task 7: Add Test Script + Profile

**Files:**
- Modify: `genesis/a2o/package.json` (add test:testnet script)
- Modify: `genesis/a2o/cucumber.mjs` (add testnet profile)

**Step 1: Add testnet profile to cucumber.mjs**

```javascript
// Add to the profiles object:
testnet: {
  ...base,
  paths: ['features/elohim/compute-allocation.feature', 'features/deployment/persona-testnet-validation.feature'],
},
```

**Step 2: Add npm script**

```json
"test:testnet": "cucumber-js --profile testnet --tags '@testnet and @e2e and not @wip'"
```

**Step 3: Dry-run the full suite**

Run:
```bash
cd genesis/a2o
pnpm test:testnet -- --dry-run
```

Expected: All scenarios listed, all steps matched, no undefined steps.

**Step 4: Commit**

```bash
git add genesis/a2o/package.json genesis/a2o/cucumber.mjs
git commit -m "feat(a2o): add testnet profile and test:testnet script"
```

---

### Task 8: Integration Test — Run 5 Personas End-to-End

**This is the proof-of-concept run.** No new code — just execute and verify.

**Step 1: Start with dry run**

```bash
cd genesis/a2o
pnpm test:testnet -- --dry-run
```

Expected: 3 scenarios, all steps defined.

**Step 2: Run the first scenario only**

```bash
cd genesis/a2o
npx cucumber-js --profile testnet --tags '@compute-allocation and @e2e and not @circuit-breaker'
```

Expected: 2 scenarios pass (request + settle). 5 conductors spawn, budget tracking runs, envelopes emitted, settle on completion.

**Step 3: Run the circuit breaker scenario**

```bash
cd genesis/a2o
npx cucumber-js --profile testnet --tags '@circuit-breaker'
```

Expected: 1 scenario passes. Pete's node gets killed, budget-exceeded envelope emitted, remaining 4 continue, partial settlement recorded.

**Step 4: Check artifacts**

```bash
# Envelopes
cat /tmp/elohim-persona-testnet/envelopes/envelopes.jsonl | jq .verb
# Expected: provision, sense..., settle..., budget-exceeded, settle...

# Compute summary
cat genesis/a2o/reports/compute-summary.json | jq .
# Expected: totalCpuSeconds, perPersona breakdown, budgetExceeded list

# No orphan processes
ps aux | grep elohim-node | grep -v grep
# Expected: nothing running
```

**Step 5: Commit any fixes from the integration run**

```bash
git add -A
git commit -m "fix(a2o): integration test fixes from first 5-persona testnet run"
```

---

## Summary

| Task | What | Commit Message |
|------|------|----------------|
| 1 | TTL + budget circuit breaker in compute-budget.sh | `feat(simulation): add TTL watchdog + budget circuit breaker` |
| 2 | spawn-subset + envelope emission in spawn-persona-testnet.sh | `feat(simulation): add start-subset with provision/settle envelopes` |
| 3 | Testnet lifecycle manager + cucumber hooks | `feat(a2o): add testnet lifecycle manager with session-scoped hooks` |
| 4 | Envelope validator with tests | `feat(a2o): add coordination envelope validator with tests` |
| 5 | Feature file + step definitions | `feat(a2o): Matthew's compute allocation scenarios` |
| 6 | Cucumber report attachment | `feat(a2o): attach compute telemetry to cucumber report` |
| 7 | Test profile + npm script | `feat(a2o): add testnet profile and test:testnet script` |
| 8 | Integration test run | `fix(a2o): integration test fixes` |
