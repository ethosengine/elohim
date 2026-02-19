# Imago Dei Pillar - Identity

Human identity layer aligned with "Image of God" framework.

**Architecture:** `elohim/ELOHIM_PROTOCOL_ARCHITECTURE.md`

## Philosophy

Four dimensions of human identity:
- **imagodei-core**: Stable identity center (who am I?)
- **imagodei-experience**: Learning and transformation
- **imagodei-gifts**: Developed capabilities/attestations
- **imagodei-synthesis**: Growth and meaning-making

## Agency Stage Progression

Humans progress through agency stages as they deepen engagement:

| Stage | Conductor | Keys | Data Location | Hosting |
|-------|-----------|------|---------------|---------|
| `visitor` | None | None | Browser localStorage | N/A |
| `hosted` | Remote edge node | Custodial (edge holds) | DHT via edge node | Costs covered by commons/steward |
| `app-steward` | Local on device | Self-sovereign | DHT + local conductor | Self-hosted |
| `node-steward` | Always-on local | Self-sovereign | DHT + hosts others | Receives value flows |

### Stage Detection Logic

`AgencyService.determineStage()` detects stage based on:
1. Holochain connection state (connected vs disconnected)
2. Conductor location (local vs remote URL patterns)
3. Node steward status (localStorage config for MVP)

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
| `identity.model.ts` | IdentityMode, IdentityState, KeyLocation, HostingCostSummary, NodeStewardHostingIncome, AgencyTransition |
| `session-human.model.ts` | SessionHuman, SessionStats, UpgradeIntent, HostingCostStatus, HOSTING_TIERS |
| `agency.model.ts` | AgencyStage, AgencyState, AGENCY_STAGES, transition helpers |
| `presence.model.ts` | ContributorPresenceView, PresenceState (unclaimed/stewarded/claimed) |
| `profile.model.ts` | HumanProfile, JourneyStats, TimelineEvent |
| `attestations.model.ts` | Agent attestations (credentials earned BY humans) |

## Services

| Service | Purpose |
|---------|---------|
| `IdentityService` | Network identity management, Holochain Human registration, mode detection |
| `SessionHumanService` | Temporary localStorage identity, hybrid state, upgrade intent tracking |
| `SessionMigrationService` | Session → Holochain migration with progress data transfer |
| `AgencyService` | Agency stage detection, key info, data residency |
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
| `ProfileComponent` | View/edit profile, agency stage display |
| `PresenceListComponent` | List/manage contributor presences |

## Hosting Economics (Shefa Integration)

Hosted humans incur sustainable hosting costs covered by:
- **commons**: Protocol commons fund (default for new humans)
- **steward**: Another human sponsors their hosting
- **sponsor**: Organization/grant covers costs
- **self**: Human pays their own costs
- **migrated**: No longer hosted (moved to own device)

Node stewards receive value flows for:
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

interface NodeStewardHostingIncome {
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

### AgencyService

```typescript
// Signals
readonly currentStage: Signal<AgencyStage>;
readonly stageInfo: Signal<AgencyStageInfo>;
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
startUpgradeIntent(targetStage: 'hosted' | 'app-steward' | 'node-steward'): void;
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
