# Sprint-07: Holochain Client Service Testing Strategy

## Objective
Develop a comprehensive testing strategy for `holochain-client.service.ts` to achieve 60%+ code coverage with meaningful, maintainable tests.

## Scope
- **Service**: `src/app/elohim/services/holochain-client.service.ts`
- **Current Coverage**: ~14.8% (lines)
- **Target Coverage**: 60%+
- **Lines of Code**: ~250 executable lines
- **Dependencies**: 10 major dependencies to mock

---

## Challenge Analysis

### Why This Service Is Hard to Test

| Challenge | Description | Impact |
|-----------|-------------|--------|
| **WebSocket Connections** | Uses `@holochain/client` AdminWebsocket and AppWebsocket | Requires complete mock of Holochain client library |
| **Strategy Pattern** | Delegates to `IConnectionStrategy` from external library | Must mock strategy AND its results |
| **Async State Machine** | 6-state connection FSM with complex transitions | Tests must verify state changes at each step |
| **Browser APIs** | localStorage, window.location for credentials/detection | Requires browser API mocks |
| **External Config** | Reads from `environment.ts` at module load | Hard to change config per-test |
| **Auto-Reconnect** | Exponential backoff with timers | Needs fake timers/jasmine.clock |
| **Signal-Based State** | Angular 17+ signals for reactive state | State assertions need signal unwrapping |

### Dependency Graph

```
HolochainClientService
├── HttpClient (Angular)
├── LoggerService (internal)
├── PerformanceMetricsService (internal)
├── CONNECTION_STRATEGY (InjectionToken)
│   └── IConnectionStrategy (from @elohim/service/connection)
│       ├── DoorwayConnectionStrategy
│       └── DirectConnectionStrategy
├── @holochain/client
│   ├── AdminWebsocket
│   └── AppWebsocket
└── Browser APIs
    ├── localStorage
    └── window.location
```

---

## Testing Strategy

### Approach: Mock Boundary Pattern

Mock at the **strategy boundary** rather than mocking Holochain internals:

```typescript
// Good: Mock the strategy interface
const mockStrategy = jasmine.createSpyObj<IConnectionStrategy>('Strategy', [
  'connect', 'disconnect', 'getSigningCredentials', 'resolveAdminUrl', 'getContentSources'
]);
mockStrategy.name = 'mock-doorway';
mockStrategy.mode = 'doorway';

// Bad: Trying to mock AdminWebsocket internals
// This couples tests to implementation details
```

### Test Categories

#### Category 1: State Management (Pure Logic)
- Initial state verification
- State transitions
- Computed signal accessors
- Configuration management

**Coverage Target**: 90%+ (no async, no mocking)

#### Category 2: Connection Flow (Strategy Integration)
- Successful connection
- Failed connection
- Disconnect behavior
- State updates from strategy results

**Coverage Target**: 70%+ (mocked strategy)

#### Category 3: Zome Calls (Happy Path + Errors)
- Connected call success
- Not connected errors
- Connection timeout handling
- Multi-DNA role resolution
- REST API calls

**Coverage Target**: 80%+ (mocked http + strategy)

#### Category 4: Auto-Reconnect Logic
- Reconnect scheduling
- Exponential backoff calculation
- Max retry limit
- Cancel reconnect

**Coverage Target**: 60%+ (fake timers)

#### Category 5: Utility Methods
- Credential storage/retrieval
- Base64 encoding
- URL resolution
- Environment detection

**Coverage Target**: 80%+ (localStorage mock)

---

## Implementation Plan

### Phase 1: Test Infrastructure Setup

Create shared testing utilities in `src/app/testing/holochain-mocks.ts`:

```typescript
/**
 * Holochain Client Test Utilities
 *
 * Provides mock factories for testing holochain-client.service.ts
 */
import { IConnectionStrategy, ConnectionConfig, ConnectionResult } from '@elohim/service/connection';
import { CellId, AgentPubKey, AppInfo } from '@holochain/client';

/**
 * Create a mock IConnectionStrategy
 */
export function createMockStrategy(overrides?: Partial<IConnectionStrategy>): jasmine.SpyObj<IConnectionStrategy> {
  const strategy = jasmine.createSpyObj<IConnectionStrategy>('MockStrategy', [
    'connect',
    'disconnect',
    'getSigningCredentials',
    'resolveAdminUrl',
    'getContentSources',
  ]);

  // Default properties
  (strategy as any).name = overrides?.name ?? 'mock-doorway';
  (strategy as any).mode = overrides?.mode ?? 'doorway';

  // Default implementations
  strategy.connect.and.returnValue(Promise.resolve({ success: false, error: 'Not configured' }));
  strategy.disconnect.and.returnValue(Promise.resolve());
  strategy.getSigningCredentials.and.returnValue(null);
  strategy.resolveAdminUrl.and.returnValue('ws://localhost:4444');
  strategy.getContentSources.and.returnValue([]);

  return strategy;
}

/**
 * Create a successful connection result
 */
export function createSuccessfulConnectionResult(): ConnectionResult {
  const mockCellId: CellId = [
    new Uint8Array(32).fill(1), // DNA hash
    new Uint8Array(32).fill(2), // Agent pub key
  ];

  return {
    success: true,
    adminWs: createMockAdminWs(),
    appWs: createMockAppWs(),
    agentPubKey: new Uint8Array(32).fill(2),
    cellIds: new Map([['lamad', mockCellId]]),
    appInfo: createMockAppInfo(),
  };
}

/**
 * Create mock AdminWebsocket
 */
export function createMockAdminWs(): any {
  return jasmine.createSpyObj('AdminWebsocket', ['listApps', 'close']);
}

/**
 * Create mock AppWebsocket
 */
export function createMockAppWs(): any {
  const appWs = jasmine.createSpyObj('AppWebsocket', ['callZome', 'close']);
  appWs.callZome.and.returnValue(Promise.resolve({ data: 'mock-result' }));
  return appWs;
}

/**
 * Create mock AppInfo
 */
export function createMockAppInfo(): AppInfo {
  return {
    installed_app_id: 'elohim',
    cell_info: {
      lamad: [{
        type: 'provisioned',
        value: {
          cell_id: [new Uint8Array(32).fill(1), new Uint8Array(32).fill(2)],
          dna_modifiers: { network_seed: 'test-seed' },
          name: 'lamad',
        }
      }]
    },
    status: { type: 'running' },
  } as unknown as AppInfo;
}

/**
 * Create mock localStorage
 */
export function createMockLocalStorage(): Storage {
  let store: Record<string, string> = {};
  return {
    getItem: (key: string) => store[key] ?? null,
    setItem: (key: string, value: string) => { store[key] = value; },
    removeItem: (key: string) => { delete store[key]; },
    clear: () => { store = {}; },
    get length() { return Object.keys(store).length; },
    key: (index: number) => Object.keys(store)[index] ?? null,
  };
}
```

### Phase 2: Core State Tests

```typescript
// holochain-client.service.spec.ts (enhanced)

describe('HolochainClientService', () => {
  let service: HolochainClientService;
  let mockStrategy: jasmine.SpyObj<IConnectionStrategy>;
  let mockLogger: jasmine.SpyObj<LoggerService>;
  let mockMetrics: jasmine.SpyObj<PerformanceMetricsService>;
  let httpMock: HttpTestingController;
  let mockLocalStorage: Storage;

  beforeEach(() => {
    mockStrategy = createMockStrategy();
    mockLogger = jasmine.createSpyObj('LoggerService', ['createChild']);
    mockLogger.createChild.and.returnValue(
      jasmine.createSpyObj('ChildLogger', ['info', 'debug', 'warn', 'error', 'startTimer'])
    );
    mockMetrics = jasmine.createSpyObj('PerformanceMetricsService', ['recordQuery']);
    mockLocalStorage = createMockLocalStorage();

    // Replace window.localStorage
    spyOnProperty(window, 'localStorage', 'get').and.returnValue(mockLocalStorage);

    TestBed.configureTestingModule({
      imports: [HttpClientTestingModule],
      providers: [
        HolochainClientService,
        { provide: CONNECTION_STRATEGY, useValue: mockStrategy },
        { provide: LoggerService, useValue: mockLogger },
        { provide: PerformanceMetricsService, useValue: mockMetrics },
      ],
    });

    service = TestBed.inject(HolochainClientService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  // Tests organized by category...
});
```

### Phase 3: Connection Flow Tests

```typescript
describe('connect', () => {
  it('should transition through connecting → authenticating → connected', async () => {
    const result = createSuccessfulConnectionResult();
    mockStrategy.connect.and.returnValue(Promise.resolve(result));

    const stateChanges: string[] = [];
    // Capture state changes (can use effect() or manual polling)

    await service.connect();

    expect(service.state()).toBe('connected');
    expect(service.isConnected()).toBeTrue();
    expect(mockStrategy.connect).toHaveBeenCalledTimes(1);
  });

  it('should set error state on connection failure', async () => {
    mockStrategy.connect.and.returnValue(Promise.resolve({
      success: false,
      error: 'Connection refused',
    }));

    await expectAsync(service.connect()).toBeRejected();

    expect(service.state()).toBe('error');
    expect(service.error()).toBe('Connection refused');
  });

  it('should store signing credentials after successful connection', async () => {
    const mockCredentials = {
      capSecret: new Uint8Array(32),
      keyPair: { publicKey: new Uint8Array(32), privateKey: new Uint8Array(64) },
      signingKey: new Uint8Array(32),
    };
    mockStrategy.connect.and.returnValue(Promise.resolve(createSuccessfulConnectionResult()));
    mockStrategy.getSigningCredentials.and.returnValue(mockCredentials);

    await service.connect();

    expect(mockLocalStorage.getItem('holochain-signing-credentials')).toBeTruthy();
  });
});
```

### Phase 4: Zome Call Tests

```typescript
describe('callZome', () => {
  beforeEach(async () => {
    // Setup connected state
    mockStrategy.connect.and.returnValue(Promise.resolve(createSuccessfulConnectionResult()));
    await service.connect();
  });

  it('should make successful zome call when connected', async () => {
    const mockAppWs = service.connection().appWs as jasmine.SpyObj<any>;
    mockAppWs.callZome.and.returnValue(Promise.resolve({ id: 'test-content' }));

    const result = await service.callZome({
      zomeName: 'content_store',
      fnName: 'get_content',
      payload: { id: 'test-1' },
    });

    expect(result.success).toBeTrue();
    expect(result.data).toEqual({ id: 'test-content' });
    expect(mockMetrics.recordQuery).toHaveBeenCalledWith(jasmine.any(Number), true);
  });

  it('should return error for non-existent role', async () => {
    const result = await service.callZome({
      zomeName: 'some_zome',
      fnName: 'some_fn',
      payload: {},
      roleName: 'non-existent-role',
    });

    expect(result.success).toBeFalse();
    expect(result.error).toContain('No cell found for role');
    expect(result.error).toContain('Available roles: lamad');
  });

  it('should wait for connection when in connecting state', async () => {
    // Reset to connecting state
    await service.disconnect();

    // Start connection but don't await
    const connectPromise = service.connect();

    // Start zome call before connection completes
    const zomePromise = service.callZome({
      zomeName: 'content_store',
      fnName: 'get_content',
      payload: {},
    });

    // Complete connection
    await connectPromise;

    const result = await zomePromise;
    expect(result.success).toBeTrue();
  });
});
```

### Phase 5: Auto-Reconnect Tests

```typescript
describe('auto-reconnect', () => {
  beforeEach(() => {
    jasmine.clock().install();
  });

  afterEach(() => {
    jasmine.clock().uninstall();
  });

  it('should schedule reconnect with exponential backoff', async () => {
    // Setup initial connection that will fail
    mockStrategy.connect.and.returnValue(Promise.resolve({
      success: false,
      error: 'Connection lost',
    }));

    service.setAutoReconnect(true);

    // Trigger connection loss
    await expectAsync(service.connect()).toBeRejected();

    // First retry at 1000ms
    jasmine.clock().tick(999);
    expect(mockStrategy.connect).toHaveBeenCalledTimes(1);

    jasmine.clock().tick(1);
    expect(mockStrategy.connect).toHaveBeenCalledTimes(2);

    // Second retry at 2000ms (exponential)
    jasmine.clock().tick(2000);
    expect(mockStrategy.connect).toHaveBeenCalledTimes(3);
  });

  it('should stop retrying after max attempts', async () => {
    mockStrategy.connect.and.returnValue(Promise.resolve({
      success: false,
      error: 'Connection refused',
    }));

    service.setAutoReconnect(true);
    await expectAsync(service.connect()).toBeRejected();

    // Fast-forward through all retries (5 max)
    for (let i = 0; i < 6; i++) {
      jasmine.clock().tick(30000); // Max delay
    }

    // Should have stopped at 5 retries + 1 initial
    expect(mockStrategy.connect.calls.count()).toBeLessThanOrEqual(6);

    const status = service.getReconnectStatus();
    expect(status.isReconnecting).toBeFalse();
  });

  it('should cancel reconnect on explicit disconnect', async () => {
    mockStrategy.connect.and.returnValue(Promise.resolve({
      success: false,
      error: 'Connection lost',
    }));

    service.setAutoReconnect(true);
    await expectAsync(service.connect()).toBeRejected();

    // Disconnect before retry
    await service.disconnect();

    // Retry should not happen
    jasmine.clock().tick(5000);
    expect(mockStrategy.connect).toHaveBeenCalledTimes(1);
  });
});
```

### Phase 6: REST API Tests

```typescript
describe('callZomeRest', () => {
  beforeEach(async () => {
    mockStrategy.connect.and.returnValue(Promise.resolve(createSuccessfulConnectionResult()));
    await service.connect();
  });

  it('should make REST call with correct URL format', fakeAsync(() => {
    let result: any;
    service.callZomeRest({
      zomeName: 'content_store',
      fnName: 'get_content',
      payload: { id: 'test-1' },
    }).then(r => result = r);

    // Expect POST to zome REST endpoint
    const req = httpMock.expectOne(req =>
      req.url.includes('/api/v1/zome/') &&
      req.url.includes('/content_store/get_content')
    );
    expect(req.request.method).toBe('POST');
    expect(req.request.body).toEqual({ id: 'test-1' });

    req.flush({ id: 'test-1', title: 'Test Content' });
    tick();

    expect(result.success).toBeTrue();
    expect(result.data).toEqual({ id: 'test-1', title: 'Test Content' });
  }));

  it('should handle REST errors', fakeAsync(() => {
    let result: any;
    service.callZomeRest({
      zomeName: 'content_store',
      fnName: 'get_content',
      payload: { id: 'not-found' },
    }).then(r => result = r);

    const req = httpMock.expectOne(req => req.url.includes('/api/v1/zome/'));
    req.flush({ error: 'Content not found' }, { status: 404, statusText: 'Not Found' });
    tick();

    expect(result.success).toBeFalse();
    expect(result.error).toContain('not found');
  }));
});
```

---

## Test File Structure

```
src/app/
├── testing/
│   ├── index.ts                          # Barrel export
│   ├── holochain-mocks.ts               # Mock factories (NEW)
│   └── testing.module.ts                 # Shared TestBed config
├── elohim/
│   └── services/
│       ├── holochain-client.service.ts
│       └── holochain-client.service.spec.ts  # Enhanced tests
```

---

## Coverage Targets by Method

| Method | Lines | Current | Target | Priority |
|--------|-------|---------|--------|----------|
| `connect()` | 60 | 0% | 80% | HIGH |
| `callZome()` | 50 | 20% | 90% | HIGH |
| `callZomeRest()` | 35 | 0% | 80% | MEDIUM |
| `disconnect()` | 15 | 50% | 90% | LOW |
| `scheduleReconnect()` | 30 | 0% | 70% | MEDIUM |
| `waitForConnection()` | 15 | 0% | 80% | MEDIUM |
| `getDisplayInfo()` | 20 | 0% | 90% | LOW |
| Private helpers | 30 | 10% | 50% | LOW |

---

## Test Cases Checklist

### State Management
- [x] Initial state is disconnected
- [x] isConnected() returns false initially
- [x] error() returns undefined initially
- [ ] connection() exposes full state object
- [ ] state transitions are reflected in signals

### Connection Flow
- [ ] connect() calls strategy.connect()
- [ ] connect() updates state to 'connecting' then 'connected'
- [ ] connect() stores signing credentials on success
- [ ] connect() sets error state on failure
- [ ] connect() throws on failure (for caller handling)
- [ ] connect() with custom config overrides defaults
- [ ] testAdminConnection() works without full connect

### Zome Calls
- [x] callZome() returns error when disconnected
- [ ] callZome() succeeds when connected
- [ ] callZome() waits for connection if connecting
- [ ] callZome() times out if connection takes too long
- [ ] callZome() uses correct cellId for roleName
- [ ] callZome() returns error for unknown roleName
- [ ] callZome() handles zome call errors
- [ ] callZome() records metrics for all calls
- [ ] callZomeRest() makes correct HTTP request
- [ ] callZomeRest() handles HTTP errors

### Auto-Reconnect
- [ ] setAutoReconnect(true) enables reconnection
- [ ] scheduleReconnect() uses exponential backoff
- [ ] reconnect stops after maxRetries
- [ ] disconnect() cancels pending reconnect
- [ ] getReconnectStatus() returns current state

### Utilities
- [ ] uint8ArrayToBase64() encodes correctly
- [ ] hasStoredCredentials() checks localStorage
- [ ] storeSigningCredentials() serializes correctly
- [ ] getConfig() returns current config
- [ ] getDisplayInfo() formats data for UI
- [ ] isCheEnvironment() detects Che URLs

---

## Verification Commands

```bash
# Run only holochain-client tests
npm run test -- --include='**/holochain-client.service.spec.ts' --code-coverage

# View coverage for specific file
open coverage/elohim-app/app/elohim/services/holochain-client.service.ts.html

# Check coverage threshold
npm run test -- --include='**/elohim/**/*.spec.ts' --code-coverage | grep holochain-client
```

---

## Acceptance Criteria

- [ ] `holochain-client.service.ts` has ≥ 60% line coverage
- [ ] `holochain-client.service.ts` has ≥ 50% branch coverage
- [ ] All new tests pass: `npm run test`
- [ ] Test utilities are reusable (`src/app/testing/holochain-mocks.ts`)
- [ ] No flaky tests (async timing issues resolved)
- [ ] Tests run in < 5 seconds

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Holochain client library internals change | Mock at strategy boundary, not library level |
| Async timing causes flaky tests | Use jasmine.clock() for timers, fakeAsync for promises |
| localStorage not available in test | Provide mock via spy |
| Signal state hard to assert | Use `effect()` or direct signal call |
| Environment config imported at load time | Use dependency injection for config |

---

## Notes

- The strategy pattern makes this service more testable than directly using @holochain/client
- Focus on testing the service's behavior, not the strategy implementation
- The `@elohim/service/connection` library should have its own tests
- Consider adding integration tests that run against a real conductor (separate suite)
