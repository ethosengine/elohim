/**
 * Mock service factories for unit testing
 */
import { type Mock, vi } from 'vitest';

// ============================================================================
// Content Service Mock
// ============================================================================

export interface MockContentService {
  getContent: Mock;
  getContentByPath: Mock;
  listContent: Mock;
  createContent: Mock;
  updateContent: Mock;
  deleteContent: Mock;
}

export function createMockContentService(): MockContentService {
  return {
    getContent: vi.fn(),
    getContentByPath: vi.fn(),
    listContent: vi.fn(),
    createContent: vi.fn(),
    updateContent: vi.fn(),
    deleteContent: vi.fn(),
  };
}

// ============================================================================
// Auth Service Mock
// ============================================================================

export interface MockAuthService {
  login: Mock;
  logout: Mock;
  isAuthenticated: Mock;
  getToken: Mock;
  getCurrentHuman: Mock;
  refreshToken: Mock;
}

export function createMockAuthService(): MockAuthService {
  return {
    login: vi.fn(),
    logout: vi.fn(),
    isAuthenticated: vi.fn().mockReturnValue(false),
    getToken: vi.fn().mockReturnValue(null),
    getCurrentHuman: vi.fn().mockReturnValue(null),
    refreshToken: vi.fn(),
  };
}

// ============================================================================
// Storage Client Mock
// ============================================================================

export interface MockStorageClientService {
  get: Mock;
  put: Mock;
  delete: Mock;
  list: Mock;
  getBlob: Mock;
  putBlob: Mock;
  getBaseUrl: Mock;
}

export function createMockStorageClient(): MockStorageClientService {
  return {
    get: vi.fn(),
    put: vi.fn(),
    delete: vi.fn(),
    list: vi.fn(),
    getBlob: vi.fn(),
    putBlob: vi.fn(),
    getBaseUrl: vi.fn().mockReturnValue('http://localhost:8888'),
  };
}

// ============================================================================
// Holochain Client Mock
// ============================================================================

export interface MockHolochainClientService {
  connect: Mock;
  disconnect: Mock;
  callZome: Mock;
  isConnected: Mock;
  onSignal: Mock;
}

export function createMockHolochainClient(): MockHolochainClientService {
  return {
    connect: vi.fn().mockReturnValue(Promise.resolve()),
    disconnect: vi.fn(),
    callZome: vi.fn().mockReturnValue(Promise.resolve(null)),
    isConnected: vi.fn().mockReturnValue(false),
    onSignal: vi.fn(),
  };
}

// ============================================================================
// Doorway Client Mock
// ============================================================================

export interface MockDoorwayClientService {
  getStatus: Mock;
  callEndpoint: Mock;
  isAvailable: Mock;
}

export function createMockDoorwayClient(): MockDoorwayClientService {
  return {
    getStatus: vi.fn().mockReturnValue(Promise.resolve({ healthy: true })),
    callEndpoint: vi.fn(),
    isAvailable: vi.fn().mockReturnValue(true),
  };
}

// ============================================================================
// Path Service Mock
// ============================================================================

export interface MockPathService {
  getPath: Mock;
  getPathSteps: Mock;
  getCurrentStep: Mock;
  advanceStep: Mock;
  completeStep: Mock;
}

export function createMockPathService(): MockPathService {
  return {
    getPath: vi.fn(),
    getPathSteps: vi.fn(),
    getCurrentStep: vi.fn(),
    advanceStep: vi.fn(),
    completeStep: vi.fn(),
  };
}

// ============================================================================
// Mastery Service Mock
// ============================================================================

export interface MockMasteryService {
  getMastery: Mock;
  updateMastery: Mock;
  getPathMastery: Mock;
}

export function createMockMasteryService(): MockMasteryService {
  return {
    getMastery: vi.fn().mockReturnValue(Promise.resolve({ level: 0, score: 0 })),
    updateMastery: vi.fn(),
    getPathMastery: vi.fn(),
  };
}

// ============================================================================
// Presence Service Mock
// ============================================================================

export interface MockPresenceService {
  setPresence: Mock;
  getPresence: Mock;
  subscribeToPresence: Mock;
  unsubscribe: Mock;
}

export function createMockPresenceService(): MockPresenceService {
  return {
    setPresence: vi.fn(),
    getPresence: vi.fn().mockReturnValue(Promise.resolve({ status: 'offline' })),
    subscribeToPresence: vi.fn(),
    unsubscribe: vi.fn(),
  };
}

// ============================================================================
// Storage API Service Mock (Shefa)
// ============================================================================

export interface MockStorageApiService {
  createEconomicEvent: Mock;
  getEconomicEvents: Mock;
  updateEconomicEvent: Mock;
  deleteEconomicEvent: Mock;
}

export function createMockStorageApiService(): MockStorageApiService {
  return {
    createEconomicEvent: vi.fn(),
    // eslint-disable-next-line @typescript-eslint/no-empty-function
    getEconomicEvents: vi.fn().mockReturnValue({ pipe: () => ({ subscribe: () => {} }) }),
    updateEconomicEvent: vi.fn(),
    deleteEconomicEvent: vi.fn(),
  };
}
