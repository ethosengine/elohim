# Imago Dei Pillar - Identity

Human identity layer aligned with "Image of God" framework.

**Architecture:** `elohim/ELOHIM_PROTOCOL_ARCHITECTURE.md`

## Philosophy

Four dimensions of human identity:
- **imagodei-core**: Stable identity center (who am I?)
- **imagodei-experience**: Learning and transformation
- **imagodei-gifts**: Developed capabilities/attestations
- **imagodei-synthesis**: Growth and meaning-making

## Sovereignty Stage Progression

Humans progress through sovereignty stages as they deepen engagement:

| Stage | Conductor | Keys | Data Location | Hosting |
|-------|-----------|------|---------------|---------|
| `visitor` | None | None | Browser localStorage | N/A |
| `hosted` | Remote edge node | Custodial (edge holds) | DHT via edge node | Costs covered by commons/steward |
| `app-user` | Local on device | Self-sovereign | DHT + local conductor | Self-hosted |
| `node-operator` | Always-on local | Self-sovereign | DHT + hosts others | Receives value flows |

### Stage Detection Logic

`SovereigntyService.determineStage()` detects stage based on:
1. Holochain connection state (connected vs disconnected)
2. Conductor location (local vs remote URL patterns)
3. Node operator status (localStorage config for MVP)

### Identity Modes

```typescript
type IdentityMode =
  | 'anonymous'      // Pure browser, no session
  | 'session'        // localStorage visitor
  | 'hosted'         // Holochain with custodial keys
  | 'self-sovereign' // Holochain with keys on device
  | 'migrating';     // In transition between stages
```

### Key Locations

```typescript
type KeyLocation =
  | 'none'       // Visitor (no keys)
  | 'browser'    // Browser IndexedDB (least secure)
  | 'custodial'  // Edge node holds keys
  | 'device'     // Local Holochain conductor
  | 'hardware';  // HSM (Ledger, YubiKey)
```

## Models

| Model | Purpose |
|-------|---------|
| `identity.model.ts` | IdentityMode, IdentityState, KeyLocation, HostingCostSummary, NodeOperatorHostingIncome, SovereigntyTransition |
| `session-human.model.ts` | SessionHuman, SessionStats, UpgradeIntent, HostingCostStatus, HOSTING_TIERS |
| `sovereignty.model.ts` | SovereigntyStage, SovereigntyState, SOVEREIGNTY_STAGES, transition helpers |
| `presence.model.ts` | ContributorPresenceView, PresenceState (unclaimed/stewarded/claimed) |
| `profile.model.ts` | HumanProfile, JourneyStats, TimelineEvent |
| `attestations.model.ts` | Agent attestations (credentials earned BY humans) |

## Services

| Service | Purpose |
|---------|---------|
| `IdentityService` | Network identity management, Holochain Human registration, mode detection |
| `SessionHumanService` | Temporary localStorage identity, hybrid state, upgrade intent tracking |
| `SessionMigrationService` | Session → Holochain migration with progress data transfer |
| `SovereigntyService` | Sovereignty stage detection, key info, data residency |
| `PresenceService` | Contributor presence creation, stewardship lifecycle |

## Guards

| Guard | Purpose |
|-------|---------|
| `identityGuard` | Requires network authentication (hosted or self-sovereign) |
| `sessionOrAuthGuard` | Allows session OR network authentication |
| `attestationGuard(type)` | Requires specific attestation |

## Components

| Component | Purpose |
|-----------|---------|
| `RegisterComponent` | Network identity registration, session migration |
| `ProfileComponent` | View/edit profile, sovereignty stage display |
| `PresenceListComponent` | List/manage contributor presences |

## Hosting Economics (Shefa Integration)

Hosted humans incur sustainable hosting costs covered by:
- **commons**: Protocol commons fund (default for new humans)
- **steward**: Another human sponsors their hosting
- **sponsor**: Organization/grant covers costs
- **self**: Human pays their own costs
- **migrated**: No longer hosted (moved to own device)

Node operators receive value flows for:
- Number of humans hosted
- Storage provided
- Uptime percentage
- Reputation score

```typescript
interface HostingCostSummary {
  coverageSource: 'commons' | 'steward' | 'sponsor' | 'self' | 'migrated';
  coveredByName?: string;
  monthlyCostDisplay: string;
  storageUsedDisplay: string;
  migrationRecommended: boolean;
}

interface NodeOperatorHostingIncome {
  hostedHumanCount: number;
  totalStorageProvidedBytes: number;
  currentMonthIncome: number;
  lifetimeIncome: number;
  incomeUnit: string;
  uptimePercentage: number;
  reputationScore: number;
}
```

## Session → Holochain Migration

Zero-friction entry with upgrade path:

```
1. Human explores as visitor (localStorage session)
2. Meaningful moments trigger upgrade prompts
3. UpgradeIntent tracks upgrade progress (can pause/resume)
4. SessionMigrationService.migrate() handles:
   - Package session data (affinity, progress, activities)
   - Register human identity in Holochain
   - Transfer progress to source chain
   - Clear session after success
5. Hybrid state allows session + Holochain coexistence during transition
```

### Hybrid State

Sessions can be linked to Holochain identities:
```typescript
// Link session to Holochain
sessionHumanService.linkToHolochainIdentity(agentPubKey, humanId);

// Check link status
sessionHumanService.isLinkedToHolochain();
sessionHumanService.getLinkedAgentPubKey();
```

## API Reference

### IdentityService

```typescript
// Registration & Profile
registerHuman(request: RegisterHumanRequest): Promise<HumanProfile>;
getCurrentHuman(): Promise<HumanProfile | null>;
updateProfile(request: UpdateProfileRequest): Promise<HumanProfile>;

// Signals
readonly mode: Signal<IdentityMode>;
readonly profile: Signal<HumanProfile | null>;
readonly isAuthenticated: Signal<boolean>;
readonly displayName: Signal<string>;
readonly attestations: Signal<string[]>;
```

### SovereigntyService

```typescript
// Signals
readonly currentStage: Signal<SovereigntyStage>;
readonly stageInfo: Signal<SovereigntyStageInfo>;
readonly connectionStatus: Signal<ConnectionStatus>;
readonly canUpgrade: Signal<boolean>;

// Methods
getDataSummary(): string;
getStageSummary(): { data: string; progress: string };
```

### SessionHumanService

```typescript
// Session Management
initializeSession(options?: { displayName?: string }): SessionHuman;
getSession(): SessionHuman | null;
hasSession(): boolean;

// Hybrid State
linkToHolochainIdentity(agentPubKey: string, humanId: string): void;
isLinkedToHolochain(): boolean;

// Upgrade Intent
startUpgradeIntent(targetStage: 'hosted' | 'app-user' | 'node-operator'): void;
updateUpgradeProgress(currentStep: string, completedStep?: string): void;
pauseUpgrade(reason?: string): void;
resumeUpgrade(): void;
cancelUpgrade(): void;

// Migration
prepareMigration(): MigrationPackage | null;
markAsMigrated(agentPubKey: string, humanId: string): void;
clearAfterMigration(): void;
```

## Terminology

- Use "human" not "user"
- Use "journey" not "consumption"
- Use "meaningful encounters" not "views"
- Use "steward" not "admin"

### Terminology Evolution

~~**TODO:**~~ **COMPLETED (2026-02-05):** The codebase has transitioned from "sovereignty" terminology to "agency" to better align with Elohim Protocol ethos.

**Refactoring completed**:
- ✅ `sovereignty.model.ts` → `agency.model.ts`
- ✅ `sovereignty.service.ts` → `agency.service.ts`
- ✅ `sovereignty-badge` → `agency-badge` component
- ✅ `SovereigntyStage` → `AgencyStage` (all types)
- ✅ `SovereigntyState` → `AgencyState`
- ✅ `SovereigntyTransition` → `AgencyTransition`
- ✅ `sovereigntyStage` field → `agencyStage` (IdentityState)
- ✅ `isMoreSovereign()` → `hasGreaterAgency()` (semantic improvement)

**Philosophy**: The concept of individual sovereignty conflicts with the communal, interdependent nature of the Elohim Protocol. "Agency" emphasizes capacity to act within relational networks rather than isolated self-determination.

**Stage names updated**:
- ✅ "App User" → "App Steward" (2026-02-05)
- ✅ "Node Operator" → "Node Steward" (2026-02-05)
- **Final progression**: Visitor → Hosted → App Steward → Node Steward
- Rationale: Both "User" and "Operator" are transactional; "Steward" emphasizes care, relationship, and responsibility throughout all stages

**Note**: This was an aggressive refactoring with no backward compatibility - clean break to avoid tech debt.
