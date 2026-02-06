/**
 * Mock service factories for unit testing
 */

// ============================================================================
// Content Service Mock
// ============================================================================

export interface MockContentService {
  getContent: jasmine.Spy;
  getContentByPath: jasmine.Spy;
  listContent: jasmine.Spy;
  createContent: jasmine.Spy;
  updateContent: jasmine.Spy;
  deleteContent: jasmine.Spy;
}

export function createMockContentService(): MockContentService {
  return jasmine.createSpyObj<MockContentService>('ContentService', [
    'getContent',
    'getContentByPath',
    'listContent',
    'createContent',
    'updateContent',
    'deleteContent',
  ]);
}

// ============================================================================
// Auth Service Mock
// ============================================================================

export interface MockAuthService {
  login: jasmine.Spy;
  logout: jasmine.Spy;
  isAuthenticated: jasmine.Spy;
  getToken: jasmine.Spy;
  getCurrentHuman: jasmine.Spy;
  refreshToken: jasmine.Spy;
}

export function createMockAuthService(): MockAuthService {
  const mock = jasmine.createSpyObj<MockAuthService>('AuthService', [
    'login',
    'logout',
    'isAuthenticated',
    'getToken',
    'getCurrentHuman',
    'refreshToken',
  ]);

  // Default behaviors
  mock.isAuthenticated.and.returnValue(false);
  mock.getToken.and.returnValue(null);
  mock.getCurrentHuman.and.returnValue(null);

  return mock;
}

// ============================================================================
// Storage Client Mock
// ============================================================================

export interface MockStorageClientService {
  get: jasmine.Spy;
  put: jasmine.Spy;
  delete: jasmine.Spy;
  list: jasmine.Spy;
  getBlob: jasmine.Spy;
  putBlob: jasmine.Spy;
  getBaseUrl: jasmine.Spy;
}

export function createMockStorageClient(): MockStorageClientService {
  const mock = jasmine.createSpyObj<MockStorageClientService>('StorageClientService', [
    'get',
    'put',
    'delete',
    'list',
    'getBlob',
    'putBlob',
    'getBaseUrl',
  ]);

  mock.getBaseUrl.and.returnValue('http://localhost:8888');

  return mock;
}

// ============================================================================
// Holochain Client Mock
// ============================================================================

export interface MockHolochainClientService {
  connect: jasmine.Spy;
  disconnect: jasmine.Spy;
  callZome: jasmine.Spy;
  isConnected: jasmine.Spy;
  onSignal: jasmine.Spy;
}

export function createMockHolochainClient(): MockHolochainClientService {
  const mock = jasmine.createSpyObj<MockHolochainClientService>('HolochainClientService', [
    'connect',
    'disconnect',
    'callZome',
    'isConnected',
    'onSignal',
  ]);

  mock.isConnected.and.returnValue(false);
  mock.callZome.and.returnValue(Promise.resolve(null));
  mock.connect.and.returnValue(Promise.resolve());

  return mock;
}

// ============================================================================
// Doorway Client Mock
// ============================================================================

export interface MockDoorwayClientService {
  getStatus: jasmine.Spy;
  callEndpoint: jasmine.Spy;
  isAvailable: jasmine.Spy;
}

export function createMockDoorwayClient(): MockDoorwayClientService {
  const mock = jasmine.createSpyObj<MockDoorwayClientService>('DoorwayClientService', [
    'getStatus',
    'callEndpoint',
    'isAvailable',
  ]);

  mock.isAvailable.and.returnValue(true);
  mock.getStatus.and.returnValue(Promise.resolve({ healthy: true }));

  return mock;
}

// ============================================================================
// Path Service Mock
// ============================================================================

export interface MockPathService {
  getPath: jasmine.Spy;
  getPathSteps: jasmine.Spy;
  getCurrentStep: jasmine.Spy;
  advanceStep: jasmine.Spy;
  completeStep: jasmine.Spy;
}

export function createMockPathService(): MockPathService {
  return jasmine.createSpyObj<MockPathService>('PathService', [
    'getPath',
    'getPathSteps',
    'getCurrentStep',
    'advanceStep',
    'completeStep',
  ]);
}

// ============================================================================
// Mastery Service Mock
// ============================================================================

export interface MockMasteryService {
  getMastery: jasmine.Spy;
  updateMastery: jasmine.Spy;
  getPathMastery: jasmine.Spy;
}

export function createMockMasteryService(): MockMasteryService {
  const mock = jasmine.createSpyObj<MockMasteryService>('MasteryService', [
    'getMastery',
    'updateMastery',
    'getPathMastery',
  ]);

  mock.getMastery.and.returnValue(Promise.resolve({ level: 0, score: 0 }));

  return mock;
}

// ============================================================================
// Presence Service Mock
// ============================================================================

export interface MockPresenceService {
  setPresence: jasmine.Spy;
  getPresence: jasmine.Spy;
  subscribeToPresence: jasmine.Spy;
  unsubscribe: jasmine.Spy;
}

export function createMockPresenceService(): MockPresenceService {
  const mock = jasmine.createSpyObj<MockPresenceService>('PresenceService', [
    'setPresence',
    'getPresence',
    'subscribeToPresence',
    'unsubscribe',
  ]);

  mock.getPresence.and.returnValue(Promise.resolve({ status: 'offline' }));

  return mock;
}

// ============================================================================
// Storage API Service Mock (Shefa)
// ============================================================================

export interface MockStorageApiService {
  createEconomicEvent: jasmine.Spy;
  getEconomicEvents: jasmine.Spy;
  updateEconomicEvent: jasmine.Spy;
  deleteEconomicEvent: jasmine.Spy;
}

export function createMockStorageApiService(): MockStorageApiService {
  const mock = jasmine.createSpyObj<MockStorageApiService>('StorageApiService', [
    'createEconomicEvent',
    'getEconomicEvents',
    'updateEconomicEvent',
    'deleteEconomicEvent',
  ]);

  // Default: return empty observable for queries
  // eslint-disable-next-line @typescript-eslint/no-empty-function
  mock.getEconomicEvents.and.returnValue({ pipe: () => ({ subscribe: () => {} }) });

  return mock;
}
