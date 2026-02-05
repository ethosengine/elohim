/**
 * AgencyService Tests
 *
 * Tests agency stage detection and state computation.
 */

import { TestBed } from '@angular/core/testing';
import { signal } from '@angular/core';

import { AgencyService } from './agency.service';
import { HolochainClientService } from '../../elohim/services/holochain-client.service';
import type {
  EdgeNodeDisplayInfo,
  HolochainConnectionState,
  HolochainConnection,
} from '../../elohim/models/holochain-connection.model';

describe('AgencyService', () => {
  let service: AgencyService;
  let mockHolochainClient: jasmine.SpyObj<HolochainClientService>;
  let connectionSignal: ReturnType<typeof signal<HolochainConnection>>;

  const createMockDisplayInfo = (
    state: HolochainConnectionState = 'connected',
    appUrl = 'ws://localhost:8888',
    hasStoredCredentials = false
  ): EdgeNodeDisplayInfo => ({
    state,
    mode: 'doorway',
    adminUrl: 'ws://localhost:8888/admin',
    appUrl,
    agentPubKey: 'agent-pub-key-123',
    cellId: { dnaHash: 'dna-hash', agentPubKey: 'agent-pub-key-123' },
    appId: 'elohim',
    dnaHash: 'dna-hash',
    connectedAt: new Date(),
    hasStoredCredentials,
    networkSeed: 'test-network',
    error: null,
  });

  beforeEach(() => {
    // Create connection signal
    connectionSignal = signal<HolochainConnection>({
      state: 'disconnected',
      adminWs: null,
      appWs: null,
      cellId: null,
      cellIds: new Map(),
      agentPubKey: null,
      appInfo: null,
    });

    // Create mock Holochain client
    mockHolochainClient = jasmine.createSpyObj(
      'HolochainClientService',
      ['getDisplayInfo'],
      {
        connection: connectionSignal.asReadonly(),
      }
    );

    mockHolochainClient.getDisplayInfo.and.returnValue(createMockDisplayInfo('disconnected'));

    TestBed.configureTestingModule({
      providers: [
        AgencyService,
        { provide: HolochainClientService, useValue: mockHolochainClient },
      ],
    });

    service = TestBed.inject(AgencyService);
  });

  // ==========================================================================
  // Stage Detection
  // ==========================================================================

  describe('stage detection', () => {
    beforeEach(() => {
      localStorage.clear();
    });

    it('should detect visitor stage when disconnected', () => {
      connectionSignal.set({
        state: 'disconnected',
        adminWs: null,
        appWs: null,
        cellId: null,
        cellIds: new Map(),
        agentPubKey: null,
        appInfo: null,
      });
      mockHolochainClient.getDisplayInfo.and.returnValue(createMockDisplayInfo('disconnected'));

      expect(service.currentStage()).toBe('visitor');
    });

    it('should detect hosted stage when connected to remote conductor', () => {
      connectionSignal.set({
        state: 'connected',
        adminWs: null,
        appWs: null,
        cellId: { dnaHash: 'dna', agentPubKey: 'agent' } as any,
        cellIds: new Map(),
        agentPubKey: 'agent' as any,
        appInfo: null,
      });
      mockHolochainClient.getDisplayInfo.and.returnValue(
        createMockDisplayInfo('connected', 'wss://edge.elohim.network')
      );

      expect(service.currentStage()).toBe('hosted');
    });

    it('should detect app-user stage when connected to localhost', () => {
      connectionSignal.set({
        state: 'connected',
        adminWs: null,
        appWs: null,
        cellId: { dnaHash: 'dna', agentPubKey: 'agent' } as any,
        cellIds: new Map(),
        agentPubKey: 'agent' as any,
        appInfo: null,
      });
      mockHolochainClient.getDisplayInfo.and.returnValue(
        createMockDisplayInfo('connected', 'ws://localhost:8888')
      );

      expect(service.currentStage()).toBe('app-user');
    });

    it('should detect app-user stage when connected to 127.0.0.1', () => {
      connectionSignal.set({
        state: 'connected',
        adminWs: null,
        appWs: null,
        cellId: { dnaHash: 'dna', agentPubKey: 'agent' } as any,
        cellIds: new Map(),
        agentPubKey: 'agent' as any,
        appInfo: null,
      });
      mockHolochainClient.getDisplayInfo.and.returnValue(
        createMockDisplayInfo('connected', 'ws://127.0.0.1:8888')
      );

      expect(service.currentStage()).toBe('app-user');
    });

    it('should detect visitor during connecting state without credentials', () => {
      connectionSignal.set({
        state: 'connecting',
        adminWs: null,
        appWs: null,
        cellId: null,
        cellIds: new Map(),
        agentPubKey: null,
        appInfo: null
      });
      mockHolochainClient.getDisplayInfo.and.returnValue(
        createMockDisplayInfo('connecting', 'ws://localhost:8888', false)
      );

      expect(service.currentStage()).toBe('visitor');
    });

    it('should detect hosted during connecting state with stored credentials', () => {
      connectionSignal.set({
        state: 'connecting',
        adminWs: null,
        appWs: null,
        cellId: null,
        cellIds: new Map(),
        agentPubKey: null,
        appInfo: null
      });
      mockHolochainClient.getDisplayInfo.and.returnValue(
        createMockDisplayInfo('connecting', 'wss://edge.elohim.network', true)
      );

      expect(service.currentStage()).toBe('hosted');
    });
  });

  describe('node operator detection', () => {
    beforeEach(() => {
      localStorage.clear();
    });

    it('should detect node-operator stage when configured', () => {
      // Set up node operator config
      localStorage.setItem(
        'elohim_node_operator_config',
        JSON.stringify({ isNodeOperator: true, hostedHumanCount: 5 })
      );

      connectionSignal.set({
        state: 'connected',
        adminWs: null,
        appWs: null,
        cellId: { dnaHash: 'dna', agentPubKey: 'agent' } as any,
        cellIds: new Map(),
        agentPubKey: 'agent' as any,
        appInfo: null,
      });
      mockHolochainClient.getDisplayInfo.and.returnValue(
        createMockDisplayInfo('connected', 'ws://localhost:8888')
      );

      expect(service.currentStage()).toBe('node-operator');
    });

    it('should not detect node-operator when hostedHumanCount is 0', () => {
      localStorage.setItem(
        'elohim_node_operator_config',
        JSON.stringify({ isNodeOperator: true, hostedHumanCount: 0 })
      );

      connectionSignal.set({
        state: 'connected',
        adminWs: null,
        appWs: null,
        cellId: { dnaHash: 'dna', agentPubKey: 'agent' } as any,
        cellIds: new Map(),
        agentPubKey: 'agent' as any,
        appInfo: null,
      });
      mockHolochainClient.getDisplayInfo.and.returnValue(
        createMockDisplayInfo('connected', 'ws://localhost:8888')
      );

      expect(service.currentStage()).toBe('app-user');
    });

    it('should handle malformed node operator config', () => {
      localStorage.setItem('elohim_node_operator_config', 'invalid-json');

      connectionSignal.set({
        state: 'connected',
        adminWs: null,
        appWs: null,
        cellId: { dnaHash: 'dna', agentPubKey: 'agent' } as any,
        cellIds: new Map(),
        agentPubKey: 'agent' as any,
        appInfo: null,
      });
      mockHolochainClient.getDisplayInfo.and.returnValue(
        createMockDisplayInfo('connected', 'ws://localhost:8888')
      );

      expect(service.currentStage()).toBe('app-user');
    });
  });

  // ==========================================================================
  // Connection Status
  // ==========================================================================

  describe('connection status', () => {
    it('should return connected status when connected', () => {
      connectionSignal.set({
        state: 'connected',
        adminWs: null,
        appWs: null,
        cellId: { dnaHash: 'dna', agentPubKey: 'agent' } as any,
        cellIds: new Map(),
        agentPubKey: 'agent' as any,
        appInfo: null,
      });
      mockHolochainClient.getDisplayInfo.and.returnValue(
        createMockDisplayInfo('connected', 'ws://localhost:8888')
      );

      const status = service.connectionStatus();
      expect(status.state).toBe('connected');
      expect(status.label).toBe('Connected to Network');
    });

    it('should return connecting status', () => {
      connectionSignal.set({
        state: 'connecting',
        adminWs: null,
        appWs: null,
        cellId: null,
        cellIds: new Map(),
        agentPubKey: null,
        appInfo: null
      });
      mockHolochainClient.getDisplayInfo.and.returnValue(
        createMockDisplayInfo('connecting', 'ws://localhost:8888')
      );

      const status = service.connectionStatus();
      expect(status.state).toBe('connecting');
      expect(status.label).toBe('Connecting...');
    });

    it('should return error status with message', () => {
      connectionSignal.set({
        state: 'error',
        adminWs: null,
        appWs: null,
        appInfo: null,
        cellId: null,
        cellIds: new Map(),
        agentPubKey: null,
        error: 'Network timeout'
      });
      mockHolochainClient.getDisplayInfo.and.returnValue({
        ...createMockDisplayInfo('error'),
        error: 'Network timeout',
      });

      const status = service.connectionStatus();
      expect(status.state).toBe('error');
      expect(status.label).toBe('Connection Error');
      expect(status.description).toContain('Network timeout');
    });

    it('should return offline status when disconnected', () => {
      connectionSignal.set({
        state: 'disconnected',
        adminWs: null,
        appWs: null,
        cellId: null,
        cellIds: new Map(),
        agentPubKey: null,
        appInfo: null
      });
      mockHolochainClient.getDisplayInfo.and.returnValue(
        createMockDisplayInfo('disconnected', 'ws://localhost:8888')
      );

      const status = service.connectionStatus();
      expect(status.state).toBe('offline');
      expect(status.label).toBe('Offline');
    });
  });

  // ==========================================================================
  // Data Residency
  // ==========================================================================

  describe('data residency', () => {
    it('should return visitor data residency for visitor stage', () => {
      connectionSignal.set({
        state: 'disconnected',
        adminWs: null,
        appWs: null,
        cellId: null,
        cellIds: new Map(),
        agentPubKey: null,
        appInfo: null
      });
      mockHolochainClient.getDisplayInfo.and.returnValue(createMockDisplayInfo('disconnected'));

      const state = service.agencyState();
      expect(state.dataResidency.length).toBeGreaterThan(0);
      expect(state.dataResidency.some((d) => d.locationLabel.includes('Browser'))).toBe(true);
    });

    it('should return hosted data residency for hosted stage', () => {
      connectionSignal.set({
        state: 'connected',
        adminWs: null,
        appWs: null,
        cellId: { dnaHash: 'dna', agentPubKey: 'agent' } as any,
        cellIds: new Map(),
        agentPubKey: 'agent' as any,
        appInfo: null,
      });
      mockHolochainClient.getDisplayInfo.and.returnValue(
        createMockDisplayInfo('connected', 'wss://edge.elohim.network')
      );

      const state = service.agencyState();
      expect(state.dataResidency.length).toBeGreaterThan(0);
      expect(state.dataResidency.some((d) => d.locationLabel.includes('Elohim Server') || d.locationLabel.includes('DHT Network'))).toBe(true);
    });

    it('should return app-user data residency for app-user stage', () => {
      connectionSignal.set({
        state: 'connected',
        adminWs: null,
        appWs: null,
        cellId: { dnaHash: 'dna', agentPubKey: 'agent' } as any,
        cellIds: new Map(),
        agentPubKey: 'agent' as any,
        appInfo: null,
      });
      mockHolochainClient.getDisplayInfo.and.returnValue(
        createMockDisplayInfo('connected', 'ws://localhost:8888')
      );

      const state = service.agencyState();
      expect(state.dataResidency.length).toBeGreaterThan(0);
      expect(state.dataResidency.some((d) => d.locationLabel.includes('Device'))).toBe(true);
    });
  });

  // ==========================================================================
  // Key Information
  // ==========================================================================

  describe('key information', () => {
    it('should return empty keys for visitor stage', () => {
      connectionSignal.set({
        state: 'disconnected',
        adminWs: null,
        appWs: null,
        cellId: null,
        cellIds: new Map(),
        agentPubKey: null,
        appInfo: null
      });
      mockHolochainClient.getDisplayInfo.and.returnValue(createMockDisplayInfo('disconnected'));

      const state = service.agencyState();
      expect(state.keys).toEqual([]);
    });

    it('should return agent pub key for connected state', () => {
      connectionSignal.set({
        state: 'connected',
        adminWs: null,
        appWs: null,
        cellId: { dnaHash: 'dna', agentPubKey: 'agent' } as any,
        cellIds: new Map(),
        agentPubKey: 'agent-pub-key-123456789' as any,
        appInfo: null,
      });
      mockHolochainClient.getDisplayInfo.and.returnValue({
        ...createMockDisplayInfo('connected', 'ws://localhost:8888'),
        agentPubKey: 'agent-pub-key-123456789',
      });

      const state = service.agencyState();
      expect(state.keys.length).toBeGreaterThan(0);
      const agentKey = state.keys.find((k) => k.type === 'agent-pubkey');
      expect(agentKey).toBeDefined();
      expect(agentKey?.value).toBe('agent-pub-key-123456789');
    });

    it('should truncate long keys for display', () => {
      connectionSignal.set({
        state: 'connected',
        adminWs: null,
        appWs: null,
        cellId: { dnaHash: 'dna', agentPubKey: 'agent' } as any,
        cellIds: new Map(),
        agentPubKey: 'agent-pub-key-very-long-string-here' as any,
        appInfo: null,
      });
      mockHolochainClient.getDisplayInfo.and.returnValue({
        ...createMockDisplayInfo('connected', 'ws://localhost:8888'),
        agentPubKey: 'agent-pub-key-very-long-string-here',
      });

      const state = service.agencyState();
      const agentKey = state.keys.find((k) => k.type === 'agent-pubkey');
      expect(agentKey?.truncated).toContain('...');
    });

    it('should mark custodial keys as non-exportable', () => {
      connectionSignal.set({
        state: 'connected',
        adminWs: null,
        appWs: null,
        cellId: { dnaHash: 'dna', agentPubKey: 'agent' } as any,
        cellIds: new Map(),
        agentPubKey: 'agent-pub-key-123' as any,
        appInfo: null,
      });
      mockHolochainClient.getDisplayInfo.and.returnValue({
        ...createMockDisplayInfo('connected', 'wss://edge.elohim.network'),
        agentPubKey: 'agent-pub-key-123',
      });

      const state = service.agencyState();
      const signingKey = state.keys.find((k) => k.type === 'signing-key');
      expect(signingKey?.canExport).toBe(false);
    });

    it('should mark device keys as exportable', () => {
      connectionSignal.set({
        state: 'connected',
        adminWs: null,
        appWs: null,
        cellId: { dnaHash: 'dna', agentPubKey: 'agent' } as any,
        cellIds: new Map(),
        agentPubKey: 'agent-pub-key-123' as any,
        appInfo: null,
      });
      mockHolochainClient.getDisplayInfo.and.returnValue({
        ...createMockDisplayInfo('connected', 'ws://localhost:8888'),
        agentPubKey: 'agent-pub-key-123',
      });

      const state = service.agencyState();
      const signingKey = state.keys.find((k) => k.type === 'signing-key');
      expect(signingKey?.canExport).toBe(true);
    });
  });

  // ==========================================================================
  // Migration Availability
  // ==========================================================================

  describe('migration availability', () => {
    it('should indicate migration available from visitor to hosted', () => {
      connectionSignal.set({
        state: 'disconnected',
        adminWs: null,
        appWs: null,
        cellId: null,
        cellIds: new Map(),
        agentPubKey: null,
        appInfo: null
      });
      mockHolochainClient.getDisplayInfo.and.returnValue(createMockDisplayInfo('disconnected'));

      const state = service.agencyState();
      expect(state.migrationAvailable).toBe(true);
      expect(state.migrationTarget).toBe('hosted');
    });

    it('should indicate migration available from hosted to app-user', () => {
      connectionSignal.set({
        state: 'connected',
        adminWs: null,
        appWs: null,
        cellId: { dnaHash: 'dna', agentPubKey: 'agent' } as any,
        cellIds: new Map(),
        agentPubKey: 'agent' as any,
        appInfo: null,
      });
      mockHolochainClient.getDisplayInfo.and.returnValue(
        createMockDisplayInfo('connected', 'wss://edge.elohim.network')
      );

      const state = service.agencyState();
      expect(state.migrationAvailable).toBe(true);
      expect(state.migrationTarget).toBe('app-user');
    });

    it('should indicate no migration from node-operator (final stage)', () => {
      localStorage.setItem(
        'elohim_node_operator_config',
        JSON.stringify({ isNodeOperator: true, hostedHumanCount: 5 })
      );

      connectionSignal.set({
        state: 'connected',
        adminWs: null,
        appWs: null,
        cellId: { dnaHash: 'dna', agentPubKey: 'agent' } as any,
        cellIds: new Map(),
        agentPubKey: 'agent' as any,
        appInfo: null,
      });
      mockHolochainClient.getDisplayInfo.and.returnValue(
        createMockDisplayInfo('connected', 'ws://localhost:8888')
      );

      const state = service.agencyState();
      expect(state.migrationAvailable).toBe(false);
      expect(state.migrationTarget).toBeUndefined();
    });
  });

  // ==========================================================================
  // Computed Properties
  // ==========================================================================

  describe('computed properties', () => {
    it('should compute canUpgrade based on migration availability', () => {
      connectionSignal.set({
        state: 'disconnected',
        adminWs: null,
        appWs: null,
        cellId: null,
        cellIds: new Map(),
        agentPubKey: null,
        appInfo: null
      });
      mockHolochainClient.getDisplayInfo.and.returnValue(createMockDisplayInfo('disconnected'));

      expect(service.canUpgrade()).toBe(true);
    });

    it('should provide stage info for current stage', () => {
      connectionSignal.set({
        state: 'disconnected',
        adminWs: null,
        appWs: null,
        cellId: null,
        cellIds: new Map(),
        agentPubKey: null,
        appInfo: null
      });
      mockHolochainClient.getDisplayInfo.and.returnValue(createMockDisplayInfo('disconnected'));

      const info = service.stageInfo();
      expect(info.stage).toBe('visitor');
      expect(info.description).toContain('browser');
    });
  });

  // ==========================================================================
  // Helper Methods
  // ==========================================================================

  describe('helper methods', () => {
    it('should generate data summary', () => {
      connectionSignal.set({
        state: 'disconnected',
        adminWs: null,
        appWs: null,
        cellId: null,
        cellIds: new Map(),
        agentPubKey: null,
        appInfo: null
      });
      mockHolochainClient.getDisplayInfo.and.returnValue(createMockDisplayInfo('disconnected'));

      const summary = service.getDataSummary();
      expect(summary).toContain('categories');
    });

    it('should generate stage summary for visitor', () => {
      connectionSignal.set({
        state: 'disconnected',
        adminWs: null,
        appWs: null,
        cellId: null,
        cellIds: new Map(),
        agentPubKey: null,
        appInfo: null
      });
      mockHolochainClient.getDisplayInfo.and.returnValue(createMockDisplayInfo('disconnected'));

      const summary = service.getStageSummary();
      expect(summary.data).toBe('Browser only');
      expect(summary.progress).toBe('Temporary');
    });

    it('should generate stage summary for hosted', () => {
      connectionSignal.set({
        state: 'connected',
        adminWs: null,
        appWs: null,
        cellId: { dnaHash: 'dna', agentPubKey: 'agent' } as any,
        cellIds: new Map(),
        agentPubKey: 'agent' as any,
        appInfo: null,
      });
      mockHolochainClient.getDisplayInfo.and.returnValue(
        createMockDisplayInfo('connected', 'wss://edge.elohim.network')
      );

      const summary = service.getStageSummary();
      expect(summary.data).toBe('DHT Network');
      expect(summary.progress).toBe('Saved');
    });

    it('should generate stage summary for app-user', () => {
      // Clear localStorage to ensure we're not in node-operator mode
      localStorage.clear();

      connectionSignal.set({
        state: 'connected',
        adminWs: null,
        appWs: null,
        cellId: { dnaHash: 'dna', agentPubKey: 'agent' } as any,
        cellIds: new Map(),
        agentPubKey: 'agent' as any,
        appInfo: null,
      });
      mockHolochainClient.getDisplayInfo.and.returnValue(
        createMockDisplayInfo('connected', 'ws://localhost:8888')
      );

      const summary = service.getStageSummary();
      expect(summary.data).toBe('Your Device');
      expect(summary.progress).toBe('Saved');
    });
  });

  // ==========================================================================
  // Network Stats
  // ==========================================================================

  describe('network stats', () => {
    it('should include network stats when connected', () => {
      const connectedAt = new Date();
      connectionSignal.set({
        state: 'connected',
        adminWs: null,
        appWs: null,
        cellId: { dnaHash: 'dna', agentPubKey: 'agent' } as any,
        cellIds: new Map(),
        agentPubKey: 'agent' as any,
        appInfo: null,
      });
      mockHolochainClient.getDisplayInfo.and.returnValue({
        ...createMockDisplayInfo('connected', 'ws://localhost:8888'),
        connectedAt,
      });

      const state = service.agencyState();
      expect(state.networkStats).toBeDefined();
      expect(state.networkStats?.connectedSince).toBe(connectedAt);
      // TODO(test-generator): [MEDIUM] Network stats are hardcoded to 0
      // Context: totalPeers, dataShared, dataReceived always return 0
      // Story: Real-time network metrics for sovereignty dashboard
      // Suggested approach: Integrate with Holochain conductor stats API
      expect(state.networkStats?.totalPeers).toBe(0);
    });

    it('should not include network stats when disconnected', () => {
      connectionSignal.set({
        state: 'disconnected',
        adminWs: null,
        appWs: null,
        cellId: null,
        cellIds: new Map(),
        agentPubKey: null,
        appInfo: null
      });
      mockHolochainClient.getDisplayInfo.and.returnValue(createMockDisplayInfo('disconnected'));

      const state = service.agencyState();
      expect(state.networkStats).toBeUndefined();
    });
  });
});
