import { TestBed } from '@angular/core/testing';

import { CustodianCommitmentService } from './custodian-commitment.service';
import { HolochainClientService } from './holochain-client.service';

describe('CustodianCommitmentService', () => {
  let service: CustodianCommitmentService;
  let holochainMock: jasmine.SpyObj<HolochainClientService>;

  beforeEach(() => {
    const holochainSpy = jasmine.createSpyObj('HolochainClientService', ['callZome']);

    TestBed.configureTestingModule({
      providers: [
        CustodianCommitmentService,
        { provide: HolochainClientService, useValue: holochainSpy },
      ],
    });

    service = TestBed.inject(CustodianCommitmentService);
    holochainMock = TestBed.inject(HolochainClientService) as jasmine.SpyObj<HolochainClientService>;
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('getCommitmentsForContent', () => {
    it('should have getCommitmentsForContent method', () => {
      expect(service.getCommitmentsForContent).toBeDefined();
      expect(typeof service.getCommitmentsForContent).toBe('function');
    });

    it('should return array of commitments for content', async () => {
      const mockCommitments = [
        {
          id: 'commitment-1',
          custodianId: 'custodian-1',
          doorwayEndpoint: 'http://custodian1.example.com',
          contentId: 'content-123',
          domain: 'elohim-protocol',
          epic: 'governance',
          replicationStrategy: 'full_replica' as const,
          createdAt: Date.now(),
          expiresAt: Date.now() + 30 * 24 * 60 * 60 * 1000,
          isActive: true,
          storageAllocated: 1000000,
          bandwidthAllocated: 100,
          stewardTier: 1 as const,
        },
        {
          id: 'commitment-2',
          custodianId: 'custodian-2',
          doorwayEndpoint: 'http://custodian2.example.com',
          contentId: 'content-123',
          domain: 'elohim-protocol',
          epic: 'governance',
          replicationStrategy: 'full_replica' as const,
          createdAt: Date.now(),
          expiresAt: Date.now() + 30 * 24 * 60 * 60 * 1000,
          isActive: true,
          storageAllocated: 1000000,
          bandwidthAllocated: 100,
          stewardTier: 1 as const,
        },
      ];

      holochainMock.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockCommitments })
      );

      const result = await service.getCommitmentsForContent('content-123');

      expect(result).toEqual(mockCommitments);
      expect(holochainMock.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          zomeName: 'replication',
          fnName: 'get_custodian_commitments_for_content',
          payload: { content_id: 'content-123' },
        })
      );
    });

    it('should return empty array on zome failure', async () => {
      holochainMock.callZome.and.returnValue(Promise.resolve({ success: false }));

      const result = await service.getCommitmentsForContent('content-123');

      expect(result).toEqual([]);
    });

    it('should return empty array when call throws exception', async () => {
      holochainMock.callZome.and.returnValue(Promise.reject(new Error('Network error')));

      const result = await service.getCommitmentsForContent('content-123');

      expect(result).toEqual([]);
    });

    it('should return empty array when data is null', async () => {
      holochainMock.callZome.and.returnValue(Promise.resolve({ success: true, data: null }));

      const result = await service.getCommitmentsForContent('content-123');

      expect(result).toEqual([]);
    });
  });

  describe('getCommitmentsByCustomian', () => {
    it('should have getCommitmentsByCustomian method', () => {
      expect(service.getCommitmentsByCustomian).toBeDefined();
      expect(typeof service.getCommitmentsByCustomian).toBe('function');
    });

    it('should return array of commitments by custodian', async () => {
      const mockCommitments = [
        {
          id: 'commitment-1',
          custodianId: 'custodian-1',
          doorwayEndpoint: 'http://custodian1.example.com',
          contentId: 'content-1',
          domain: 'elohim-protocol',
          epic: 'governance',
          replicationStrategy: 'full_replica' as const,
          createdAt: Date.now(),
          expiresAt: Date.now() + 30 * 24 * 60 * 60 * 1000,
          isActive: true,
          storageAllocated: 1000000,
          bandwidthAllocated: 100,
          stewardTier: 1 as const,
        },
        {
          id: 'commitment-2',
          custodianId: 'custodian-1',
          doorwayEndpoint: 'http://custodian1.example.com',
          contentId: 'content-2',
          domain: 'elohim-protocol',
          epic: 'governance',
          replicationStrategy: 'full_replica' as const,
          createdAt: Date.now(),
          expiresAt: Date.now() + 30 * 24 * 60 * 60 * 1000,
          isActive: true,
          storageAllocated: 2000000,
          bandwidthAllocated: 100,
          stewardTier: 1 as const,
        },
      ];

      holochainMock.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockCommitments })
      );

      const result = await service.getCommitmentsByCustomian('custodian-1');

      expect(result).toEqual(mockCommitments);
      expect(holochainMock.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          fnName: 'get_custodian_all_commitments',
          payload: { custodian_id: 'custodian-1' },
        })
      );
    });

    it('should return empty array on zome failure', async () => {
      holochainMock.callZome.and.returnValue(Promise.resolve({ success: false }));

      const result = await service.getCommitmentsByCustomian('custodian-1');

      expect(result).toEqual([]);
    });

    it('should return empty array on exception', async () => {
      holochainMock.callZome.and.returnValue(Promise.reject(new Error('Connection failed')));

      const result = await service.getCommitmentsByCustomian('custodian-1');

      expect(result).toEqual([]);
    });
  });

  describe('createCommitment', () => {
    it('should have createCommitment method', () => {
      expect(service.createCommitment).toBeDefined();
      expect(typeof service.createCommitment).toBe('function');
    });

    it('should create commitment and return success with ID', async () => {
      holochainMock.callZome.and.returnValue(
        Promise.resolve({ success: true, data: 'commitment-123' })
      );

      const result = await service.createCommitment(
        'custodian-1',
        'content-1',
        'full_replica',
        1000000,
        100
      );

      expect(result.success).toBe(true);
      expect(result.commitmentId).toBe('commitment-123');
      expect(holochainMock.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          fnName: 'create_custodian_commitment',
        })
      );
    });

    it('should use default expiration of 30 days', async () => {
      const now = Date.now();
      holochainMock.callZome.and.returnValue(
        Promise.resolve({ success: true, data: 'commitment-123' })
      );

      await service.createCommitment('custodian-1', 'content-1', 'full_replica', 1000000, 100);

      const call = holochainMock.callZome.calls.mostRecent();
      const payload = call.args[0].payload as Record<string, any>;
      const expirationDelta = payload['expires_at'] - now;

      expect(expirationDelta).toBeGreaterThan(29 * 24 * 60 * 60 * 1000);
      expect(expirationDelta).toBeLessThan(31 * 24 * 60 * 60 * 1000);
    });

    it('should accept custom expiration days', async () => {
      const now = Date.now();
      holochainMock.callZome.and.returnValue(
        Promise.resolve({ success: true, data: 'commitment-123' })
      );

      await service.createCommitment(
        'custodian-1',
        'content-1',
        'full_replica',
        1000000,
        100,
        7
      );

      const call = holochainMock.callZome.calls.mostRecent();
      const payload = call.args[0].payload as Record<string, any>;
      const expirationDelta = payload['expires_at'] - now;

      expect(expirationDelta).toBeGreaterThan(6 * 24 * 60 * 60 * 1000);
      expect(expirationDelta).toBeLessThan(8 * 24 * 60 * 60 * 1000);
    });

    it('should return error on zome failure', async () => {
      holochainMock.callZome.and.returnValue(
        Promise.resolve({ success: false, error: 'Invalid custodian' })
      );

      const result = await service.createCommitment(
        'custodian-1',
        'content-1',
        'full_replica',
        1000000,
        100
      );

      expect(result.success).toBe(false);
      expect(result.error).toBe('Invalid custodian');
    });

    it('should catch exceptions and return error', async () => {
      holochainMock.callZome.and.returnValue(Promise.reject(new Error('Network error')));

      const result = await service.createCommitment(
        'custodian-1',
        'content-1',
        'full_replica',
        1000000,
        100
      );

      expect(result.success).toBe(false);
      expect(result.error).toBe('Error: Network error');
    });
  });

  describe('renewCommitment', () => {
    it('should have renewCommitment method', () => {
      expect(service.renewCommitment).toBeDefined();
      expect(typeof service.renewCommitment).toBe('function');
    });

    it('should renew commitment successfully', async () => {
      holochainMock.callZome.and.returnValue(Promise.resolve({ success: true }));

      const result = await service.renewCommitment('commitment-123');

      expect(result.success).toBe(true);
      expect(result.error).toBeUndefined();
      expect(holochainMock.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          fnName: 'renew_custodian_commitment',
          payload: {
            commitment_id: 'commitment-123',
            extension_days: 30,
          },
        })
      );
    });

    it('should accept custom extension days', async () => {
      holochainMock.callZome.and.returnValue(Promise.resolve({ success: true }));

      await service.renewCommitment('commitment-123', 7);

      expect(holochainMock.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          payload: {
            commitment_id: 'commitment-123',
            extension_days: 7,
          },
        })
      );
    });

    it('should return error on zome failure', async () => {
      holochainMock.callZome.and.returnValue(
        Promise.resolve({ success: false, error: 'Commitment expired' })
      );

      const result = await service.renewCommitment('commitment-123');

      expect(result.success).toBe(false);
      expect(result.error).toBe('Commitment expired');
    });

    it('should catch exceptions', async () => {
      holochainMock.callZome.and.returnValue(Promise.reject(new Error('Call failed')));

      const result = await service.renewCommitment('commitment-123');

      expect(result.success).toBe(false);
      expect(result.error).toContain('Call failed');
    });
  });

  describe('revokeCommitment', () => {
    it('should have revokeCommitment method', () => {
      expect(service.revokeCommitment).toBeDefined();
      expect(typeof service.revokeCommitment).toBe('function');
    });

    it('should revoke commitment successfully', async () => {
      holochainMock.callZome.and.returnValue(Promise.resolve({ success: true }));

      const result = await service.revokeCommitment('commitment-123');

      expect(result.success).toBe(true);
      expect(holochainMock.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          fnName: 'revoke_custodian_commitment',
          payload: { commitment_id: 'commitment-123' },
        })
      );
    });

    it('should return error on zome failure', async () => {
      holochainMock.callZome.and.returnValue(
        Promise.resolve({ success: false, error: 'Commitment not found' })
      );

      const result = await service.revokeCommitment('commitment-123');

      expect(result.success).toBe(false);
      expect(result.error).toBe('Commitment not found');
    });

    it('should catch exceptions', async () => {
      holochainMock.callZome.and.returnValue(Promise.reject(new Error('Revoke failed')));

      const result = await service.revokeCommitment('commitment-123');

      expect(result.success).toBe(false);
      expect(result.error).toContain('Revoke failed');
    });
  });

  describe('getExpiringCommitments', () => {
    it('should have getExpiringCommitments method', () => {
      expect(service.getExpiringCommitments).toBeDefined();
      expect(typeof service.getExpiringCommitments).toBe('function');
    });

    it('should return commitments expiring within threshold', async () => {
      const now = Date.now();
      const tomorrow = now + 24 * 60 * 60 * 1000;
      const nextWeek = now + 5 * 24 * 60 * 60 * 1000;

      const mockCommitments = [
        {
          id: 'c1',
          custodianId: 'cust-1',
          contentId: 'content-1',
          isActive: true,
          expiresAt: tomorrow,
        },
        {
          id: 'c2',
          custodianId: 'cust-1',
          contentId: 'content-2',
          isActive: true,
          expiresAt: nextWeek,
        },
        {
          id: 'c3',
          custodianId: 'cust-1',
          contentId: 'content-3',
          isActive: true,
          expiresAt: now + 30 * 24 * 60 * 60 * 1000,
        },
      ];

      holochainMock.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockCommitments })
      );

      const result = await service.getExpiringCommitments('cust-1', 7);

      expect(result.length).toBe(2);
      expect(result[0].id).toBe('c1');
      expect(result[1].id).toBe('c2');
    });

    it('should not include inactive commitments', async () => {
      const now = Date.now();
      const tomorrow = now + 24 * 60 * 60 * 1000;

      const mockCommitments = [
        {
          id: 'c1',
          custodianId: 'cust-1',
          doorwayEndpoint: 'http://custodian1.example.com',
          contentId: 'content-1',
          domain: 'elohim-protocol',
          epic: 'governance',
          replicationStrategy: 'full_replica' as const,
          createdAt: now,
          expiresAt: tomorrow,
          isActive: true,
          storageAllocated: 1000000,
          bandwidthAllocated: 100,
          stewardTier: 1 as const,
        },
        {
          id: 'c2',
          custodianId: 'cust-1',
          doorwayEndpoint: 'http://custodian1.example.com',
          contentId: 'content-2',
          domain: 'elohim-protocol',
          epic: 'governance',
          replicationStrategy: 'full_replica' as const,
          createdAt: now,
          expiresAt: tomorrow,
          isActive: false,
          storageAllocated: 1000000,
          bandwidthAllocated: 100,
          stewardTier: 1 as const,
        },
      ];

      holochainMock.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockCommitments })
      );

      const result = await service.getExpiringCommitments('cust-1', 7);

      expect(result.length).toBe(1);
      expect(result[0].id).toBe('c1');
    });

    it('should return empty array on exception', async () => {
      holochainMock.callZome.and.returnValue(Promise.reject(new Error('Query failed')));

      const result = await service.getExpiringCommitments('cust-1', 7);

      expect(result).toEqual([]);
    });
  });

  describe('getActiveCommitmentCount', () => {
    it('should have getActiveCommitmentCount method', () => {
      expect(service.getActiveCommitmentCount).toBeDefined();
      expect(typeof service.getActiveCommitmentCount).toBe('function');
    });

    it('should return count of active commitments', async () => {
      const mockCommitments = [
        {
          id: 'c1',
          custodianId: 'cust-1',
          doorwayEndpoint: 'http://custodian1.example.com',
          contentId: 'content-1',
          domain: 'elohim-protocol',
          epic: 'governance',
          replicationStrategy: 'full_replica' as const,
          createdAt: Date.now(),
          expiresAt: Date.now() + 30 * 24 * 60 * 60 * 1000,
          isActive: true,
          storageAllocated: 1000000,
          bandwidthAllocated: 100,
          stewardTier: 1 as const,
        },
        {
          id: 'c2',
          custodianId: 'cust-1',
          doorwayEndpoint: 'http://custodian1.example.com',
          contentId: 'content-2',
          domain: 'elohim-protocol',
          epic: 'governance',
          replicationStrategy: 'full_replica' as const,
          createdAt: Date.now(),
          expiresAt: Date.now() + 30 * 24 * 60 * 60 * 1000,
          isActive: true,
          storageAllocated: 1000000,
          bandwidthAllocated: 100,
          stewardTier: 1 as const,
        },
        {
          id: 'c3',
          custodianId: 'cust-1',
          doorwayEndpoint: 'http://custodian1.example.com',
          contentId: 'content-3',
          domain: 'elohim-protocol',
          epic: 'governance',
          replicationStrategy: 'full_replica' as const,
          createdAt: Date.now(),
          expiresAt: Date.now() + 30 * 24 * 60 * 60 * 1000,
          isActive: false,
          storageAllocated: 1000000,
          bandwidthAllocated: 100,
          stewardTier: 1 as const,
        },
      ];

      holochainMock.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockCommitments })
      );

      const result = await service.getActiveCommitmentCount('cust-1');

      expect(result).toBe(2);
    });

    it('should return 0 when no commitments', async () => {
      holochainMock.callZome.and.returnValue(
        Promise.resolve({ success: true, data: [] })
      );

      const result = await service.getActiveCommitmentCount('cust-1');

      expect(result).toBe(0);
    });

    it('should return 0 on exception', async () => {
      holochainMock.callZome.and.returnValue(Promise.reject(new Error('Query failed')));

      const result = await service.getActiveCommitmentCount('cust-1');

      expect(result).toBe(0);
    });
  });

  describe('getTotalCommittedStorage', () => {
    it('should have getTotalCommittedStorage method', () => {
      expect(service.getTotalCommittedStorage).toBeDefined();
      expect(typeof service.getTotalCommittedStorage).toBe('function');
    });

    it('should return sum of active commitment storage', async () => {
      const mockCommitments = [
        {
          id: 'c1',
          custodianId: 'cust-1',
          doorwayEndpoint: 'http://custodian1.example.com',
          contentId: 'content-1',
          domain: 'elohim-protocol',
          epic: 'governance',
          replicationStrategy: 'full_replica' as const,
          createdAt: Date.now(),
          expiresAt: Date.now() + 30 * 24 * 60 * 60 * 1000,
          isActive: true,
          storageAllocated: 1000000,
          bandwidthAllocated: 100,
          stewardTier: 1 as const,
        },
        {
          id: 'c2',
          custodianId: 'cust-1',
          doorwayEndpoint: 'http://custodian1.example.com',
          contentId: 'content-2',
          domain: 'elohim-protocol',
          epic: 'governance',
          replicationStrategy: 'full_replica' as const,
          createdAt: Date.now(),
          expiresAt: Date.now() + 30 * 24 * 60 * 60 * 1000,
          isActive: true,
          storageAllocated: 2000000,
          bandwidthAllocated: 100,
          stewardTier: 1 as const,
        },
        {
          id: 'c3',
          custodianId: 'cust-1',
          doorwayEndpoint: 'http://custodian1.example.com',
          contentId: 'content-3',
          domain: 'elohim-protocol',
          epic: 'governance',
          replicationStrategy: 'full_replica' as const,
          createdAt: Date.now(),
          expiresAt: Date.now() + 30 * 24 * 60 * 60 * 1000,
          isActive: false,
          storageAllocated: 500000,
          bandwidthAllocated: 100,
          stewardTier: 1 as const,
        },
      ];

      holochainMock.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockCommitments })
      );

      const result = await service.getTotalCommittedStorage('cust-1');

      expect(result).toBe(3000000);
    });

    it('should not count inactive commitments', async () => {
      const mockCommitments = [
        {
          id: 'c1',
          custodianId: 'cust-1',
          doorwayEndpoint: 'http://custodian1.example.com',
          contentId: 'content-1',
          domain: 'elohim-protocol',
          epic: 'governance',
          replicationStrategy: 'full_replica' as const,
          createdAt: Date.now(),
          expiresAt: Date.now() + 30 * 24 * 60 * 60 * 1000,
          isActive: true,
          storageAllocated: 1000000,
          bandwidthAllocated: 100,
          stewardTier: 1 as const,
        },
        {
          id: 'c2',
          custodianId: 'cust-1',
          doorwayEndpoint: 'http://custodian1.example.com',
          contentId: 'content-2',
          domain: 'elohim-protocol',
          epic: 'governance',
          replicationStrategy: 'full_replica' as const,
          createdAt: Date.now(),
          expiresAt: Date.now() + 30 * 24 * 60 * 60 * 1000,
          isActive: false,
          storageAllocated: 999999999,
          bandwidthAllocated: 100,
          stewardTier: 1 as const,
        },
      ];

      holochainMock.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockCommitments })
      );

      const result = await service.getTotalCommittedStorage('cust-1');

      expect(result).toBe(1000000);
    });

    it('should return 0 when no commitments', async () => {
      holochainMock.callZome.and.returnValue(
        Promise.resolve({ success: true, data: [] })
      );

      const result = await service.getTotalCommittedStorage('cust-1');

      expect(result).toBe(0);
    });

    it('should return 0 on exception', async () => {
      holochainMock.callZome.and.returnValue(Promise.reject(new Error('Query failed')));

      const result = await service.getTotalCommittedStorage('cust-1');

      expect(result).toBe(0);
    });
  });

  describe('isCommittedTo', () => {
    it('should have isCommittedTo method', () => {
      expect(service.isCommittedTo).toBeDefined();
      expect(typeof service.isCommittedTo).toBe('function');
    });

    it('should return true if custodian is committed to content', async () => {
      const mockCommitments = [
        {
          id: 'c1',
          custodianId: 'cust-1',
          doorwayEndpoint: 'http://custodian1.example.com',
          contentId: 'content-1',
          domain: 'elohim-protocol',
          epic: 'governance',
          replicationStrategy: 'full_replica' as const,
          createdAt: Date.now(),
          expiresAt: Date.now() + 30 * 24 * 60 * 60 * 1000,
          isActive: true,
          storageAllocated: 1000000,
          bandwidthAllocated: 100,
          stewardTier: 1 as const,
        },
        {
          id: 'c2',
          custodianId: 'cust-2',
          doorwayEndpoint: 'http://custodian2.example.com',
          contentId: 'content-1',
          domain: 'elohim-protocol',
          epic: 'governance',
          replicationStrategy: 'full_replica' as const,
          createdAt: Date.now(),
          expiresAt: Date.now() + 30 * 24 * 60 * 60 * 1000,
          isActive: true,
          storageAllocated: 1000000,
          bandwidthAllocated: 100,
          stewardTier: 1 as const,
        },
      ];

      holochainMock.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockCommitments })
      );

      const result = await service.isCommittedTo('cust-1', 'content-1');

      expect(result).toBe(true);
    });

    it('should return false if custodian is not committed', async () => {
      const mockCommitments = [
        {
          id: 'c1',
          custodianId: 'cust-2',
          doorwayEndpoint: 'http://custodian2.example.com',
          contentId: 'content-1',
          domain: 'elohim-protocol',
          epic: 'governance',
          replicationStrategy: 'full_replica' as const,
          createdAt: Date.now(),
          expiresAt: Date.now() + 30 * 24 * 60 * 60 * 1000,
          isActive: true,
          storageAllocated: 1000000,
          bandwidthAllocated: 100,
          stewardTier: 1 as const,
        },
      ];

      holochainMock.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockCommitments })
      );

      const result = await service.isCommittedTo('cust-1', 'content-1');

      expect(result).toBe(false);
    });

    it('should return false if commitment is inactive', async () => {
      const mockCommitments = [
        {
          id: 'c1',
          custodianId: 'cust-1',
          doorwayEndpoint: 'http://custodian1.example.com',
          contentId: 'content-1',
          domain: 'elohim-protocol',
          epic: 'governance',
          replicationStrategy: 'full_replica' as const,
          createdAt: Date.now(),
          expiresAt: Date.now() + 30 * 24 * 60 * 60 * 1000,
          isActive: false,
          storageAllocated: 1000000,
          bandwidthAllocated: 100,
          stewardTier: 1 as const,
        },
      ];

      holochainMock.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockCommitments })
      );

      const result = await service.isCommittedTo('cust-1', 'content-1');

      expect(result).toBe(false);
    });

    it('should return false on exception', async () => {
      holochainMock.callZome.and.returnValue(Promise.reject(new Error('Check failed')));

      const result = await service.isCommittedTo('cust-1', 'content-1');

      expect(result).toBe(false);
    });
  });
});
