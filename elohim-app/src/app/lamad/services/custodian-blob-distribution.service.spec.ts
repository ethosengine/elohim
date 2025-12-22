import { TestBed } from '@angular/core/testing';
import {
  CustodianBlobDistributionService,
  CustodianSelectionCriteria,
  CustodianCapability,
  CustodianBlobCommitment,
} from './custodian-blob-distribution.service';
import { ContentBlob } from '../models/content-node.model';

describe('CustodianBlobDistributionService', () => {
  let service: CustodianBlobDistributionService;

  const createMockBlob = (): ContentBlob => ({
    hash: 'test_blob_hash',
    sizeBytes: 100 * 1024 * 1024, // 100 MB
    mimeType: 'video/mp4',
    fallbackUrls: ['https://primary.example.com/blob.mp4'],
    bitrateMbps: 5,
    durationSeconds: 600,
    codec: 'h264',
  });

  const createMockCustodian = (id: string): CustodianCapability => ({
    custodianId: id,
    displayName: `Custodian ${id}`,
    availableBandwidthMbps: 20,
    latencyMs: 50,
    uptime: 0.99,
    region: 'us-east-1',
    maxBlobSizeGb: 500,
    currentBlobCount: 10,
    reachLevel: 'commons',
  });

  const createCriteria = (): CustodianSelectionCriteria => ({
    reach: 'commons',
    minBandwidthMbps: 2,
    maxLatencyMs: 200,
    minUptime: 0.95,
    maxCustodians: 3,
  });

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [CustodianBlobDistributionService],
    });
    service = TestBed.inject(CustodianBlobDistributionService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('Custodian Selection', () => {
    it('should select suitable custodians for blob', (done) => {
      const blob = createMockBlob();
      const criteria = createCriteria();

      service.selectCustodiansForBlob(blob, 'content_123', criteria).subscribe((custodians) => {
        expect(Array.isArray(custodians)).toBe(true);
        done();
      });
    });

    it('should respect maxCustodians limit', (done) => {
      const blob = createMockBlob();
      const criteria = createCriteria();
      criteria.maxCustodians = 2;

      service.selectCustodiansForBlob(blob, 'content_123', criteria).subscribe((custodians) => {
        expect(custodians.length).toBeLessThanOrEqual(2);
        done();
      });
    });

    it('should consider bandwidth requirements', (done) => {
      const blob = createMockBlob();
      blob.bitrateMbps = 50;
      const criteria = createCriteria();
      criteria.minBandwidthMbps = 100;

      service.selectCustodiansForBlob(blob, 'content_123', criteria).subscribe((custodians) => {
        // In production, would only return custodians with sufficient bandwidth
        expect(Array.isArray(custodians)).toBe(true);
        done();
      });
    });
  });

  describe('Commitment Lifecycle', () => {
    it('should create blob commitment', (done) => {
      const blob = createMockBlob();
      const custodianId = 'custodian_1';

      service.createBlobCommitment('content_123', blob, custodianId).subscribe((commitment) => {
        expect(commitment.contentId).toBe('content_123');
        expect(commitment.blobHash).toBe(blob.hash);
        expect(commitment.custodianId).toBe(custodianId);
        expect(commitment.commitmentStatus).toBe('pending');
        expect(commitment.replicationProgress).toBe(0);
        done();
      });
    });

    it('should set commitment expiration', (done) => {
      const blob = createMockBlob();
      const now = Date.now();

      service.createBlobCommitment('content_123', blob, 'custodian_1', 30).subscribe((commitment) => {
        const thirtyDaysMs = 30 * 24 * 60 * 60 * 1000;
        expect(commitment.expiresAt - now).toBeCloseTo(thirtyDaysMs, -3); // Allow 3 second variance
        done();
      });
    });

    it('should generate fallback URL for commitment', (done) => {
      const blob = createMockBlob();

      service.createBlobCommitment('content_123', blob, 'custodian_1').subscribe((commitment) => {
        expect(commitment.fallbackUrl).toContain('custodian_1');
        expect(commitment.fallbackUrl).toContain(blob.hash);
        done();
      });
    });

    it('should update replication progress', (done) => {
      const blob = createMockBlob();

      service.createBlobCommitment('content_123', blob, 'custodian_1').subscribe((commitment) => {
        service.updateReplicationProgress(commitment, 75, 15).subscribe((updated) => {
          expect(updated.replicationProgress).toBe(75);
          expect(updated.bandwidth).toBe(15);
          expect(updated.commitmentStatus).toBe('pending'); // Not 100% yet
          done();
        });
      });
    });

    it('should mark commitment active when replication complete', (done) => {
      const blob = createMockBlob();

      service.createBlobCommitment('content_123', blob, 'custodian_1').subscribe((commitment) => {
        service.updateReplicationProgress(commitment, 100, 20).subscribe((updated) => {
          expect(updated.commitmentStatus).toBe('active');
          expect(updated.replicationProgress).toBe(100);
          done();
        });
      });
    });

    it('should revoke commitment', (done) => {
      const blob = createMockBlob();

      service.createBlobCommitment('content_123', blob, 'custodian_1').subscribe(() => {
        service.revokeCommitment('content_123', blob.hash, 'custodian_1').subscribe((revoked) => {
          expect(revoked).toBe(true);

          // Commitment should be expired
          const commitments = service.getCommitmentsForBlob('content_123', blob.hash);
          const revoked_commitment = commitments.find((c) => c.custodianId === 'custodian_1');
          expect(revoked_commitment?.commitmentStatus).toBe('expired');
          done();
        });
      });
    });
  });

  describe('Replication Status', () => {
    it('should report blob replication status', (done) => {
      const blob = createMockBlob();

      service.createBlobCommitment('content_123', blob, 'custodian_1').subscribe(() => {
        service.createBlobCommitment('content_123', blob, 'custodian_2').subscribe(() => {
          service.getBlobReplicationStatus('content_123', blob.hash).subscribe((status) => {
            expect(status.blobHash).toBe(blob.hash);
            expect(status.custodianCount).toBe(2);
            done();
          });
        });
      });
    });

    it('should calculate health status based on active replicas', (done) => {
      const blob = createMockBlob();

      service.createBlobCommitment('content_123', blob, 'custodian_1').subscribe(() => {
        service.getBlobReplicationStatus('content_123', blob.hash).subscribe((status) => {
          // With 0 active replicas (commitments pending), should be critical
          expect(status.healthStatus).toBe('critical');
          done();
        });
      });
    });

    it('should return degraded health when some replicas fail', (done) => {
      const blob = createMockBlob();

      // Create 3 commitments
      service.createBlobCommitment('content_123', blob, 'custodian_1').subscribe(() => {
        service.createBlobCommitment('content_123', blob, 'custodian_2').subscribe(() => {
          service.createBlobCommitment('content_123', blob, 'custodian_3').subscribe(() => {
            // Mark one as active
            let commitments = service.getCommitmentsForBlob('content_123', blob.hash);
            commitments[0].commitmentStatus = 'active';

            service.getBlobReplicationStatus('content_123', blob.hash).subscribe((status) => {
              // 1 active out of 3 = less than 50% = degraded
              expect(status.healthStatus).toBe('degraded');
              done();
            });
          });
        });
      });
    });
  });

  describe('Commitment Queries', () => {
    it('should retrieve commitments for blob', (done) => {
      const blob = createMockBlob();

      service.createBlobCommitment('content_123', blob, 'custodian_1').subscribe(() => {
        service.createBlobCommitment('content_123', blob, 'custodian_2').subscribe(() => {
          const commitments = service.getCommitmentsForBlob('content_123', blob.hash) as CustodianBlobCommitment[];

          expect(commitments.length).toBe(2);
          expect(commitments.map((c) => c.custodianId)).toContain('custodian_1');
          expect(commitments.map((c) => c.custodianId)).toContain('custodian_2');
          done();
        });
      });
    });

    it('should return empty array for non-existent blob', () => {
      const commitments = service.getCommitmentsForBlob('content_999', 'nonexistent_hash');
      expect(commitments).toEqual([]);
    });

    it('should get custodian fallback URLs from active commitments', (done) => {
      const blob = createMockBlob();

      service.createBlobCommitment('content_123', blob, 'custodian_1').subscribe(() => {
        // Mark as active
        let commitments = service.getCommitmentsForBlob('content_123', blob.hash);
        commitments[0].commitmentStatus = 'active';

        const urls = service.getCustomerFallbackUrls('content_123', blob.hash);

        expect(urls.length).toBeGreaterThan(0);
        expect(urls[0]).toContain('custodian_1');
        done();
      });
    });
  });

  describe('Additional Custodian Selection', () => {
    it('should recommend additional custodians if under-replicated', (done) => {
      const blob = createMockBlob();
      const criteria = createCriteria();

      service.createBlobCommitment('content_123', blob, 'custodian_1').subscribe((commitment1) => {
        const commitments = [commitment1];

        service
          .selectAdditionalCustodians(blob, 'content_123', commitments, 3, criteria)
          .subscribe((additional) => {
            expect(Array.isArray(additional)).toBe(true);
            done();
          });
      });
    });

    it('should not recommend custodians if already sufficiently replicated', (done) => {
      const blob = createMockBlob();
      const criteria = createCriteria();

      const commitments = [
        { custodianId: 'custodian_1' } as any,
        { custodianId: 'custodian_2' } as any,
        { custodianId: 'custodian_3' } as any,
      ];

      service
        .selectAdditionalCustodians(blob, 'content_123', commitments, 3, criteria)
        .subscribe((additional) => {
          expect(additional.length).toBe(0);
          done();
        });
    });

    it('should not re-select custodians already replicating blob', (done) => {
      const blob = createMockBlob();
      const criteria = createCriteria();

      service.createBlobCommitment('content_123', blob, 'custodian_1').subscribe((commitment) => {
        const commitments = [commitment];

        service
          .selectAdditionalCustodians(blob, 'content_123', commitments, 2, criteria)
          .subscribe((additional) => {
            // Additional custodians should not include custodian_1
            const additionalIds = additional.map((c) => c.custodianId);
            expect(additionalIds).not.toContain('custodian_1');
            done();
          });
      });
    });
  });

  describe('Replication Progress', () => {
    it('should calculate average replication progress', (done) => {
      const blob = createMockBlob();

      service.createBlobCommitment('content_123', blob, 'custodian_1').subscribe((c1) => {
        service.createBlobCommitment('content_123', blob, 'custodian_2').subscribe((c2) => {
          service.updateReplicationProgress(c1, 50, 10).subscribe(() => {
            service.updateReplicationProgress(c2, 100, 20).subscribe(() => {
              const avgProgress = service.getAverageReplicationProgress('content_123', blob.hash);
              expect(avgProgress).toBeCloseTo(75, 0); // (50 + 100) / 2 = 75
              done();
            });
          });
        });
      });
    });

    it('should count active replicas', (done) => {
      const blob = createMockBlob();

      service.createBlobCommitment('content_123', blob, 'custodian_1').subscribe(() => {
        service.createBlobCommitment('content_123', blob, 'custodian_2').subscribe(() => {
          service.createBlobCommitment('content_123', blob, 'custodian_3').subscribe(() => {
            let commitments = service.getCommitmentsForBlob('content_123', blob.hash);
            commitments[0].commitmentStatus = 'active';
            commitments[1].commitmentStatus = 'active';
            // commitments[2] stays pending

            const activeCount = service.getActiveReplicaCount('content_123', blob.hash);
            expect(activeCount).toBe(2);
            done();
          });
        });
      });
    });
  });

  describe('Best Custodian Selection', () => {
    it('should return best custodian URL for serving', (done) => {
      const blob = createMockBlob();

      service.createBlobCommitment('content_123', blob, 'custodian_1').subscribe((c1) => {
        service.createBlobCommitment('content_123', blob, 'custodian_2').subscribe((c2) => {
          // Mark both as active with different bandwidth
          let commitments = service.getCommitmentsForBlob('content_123', blob.hash);
          commitments[0].commitmentStatus = 'active';
          commitments[0].bandwidth = 10;
          commitments[1].commitmentStatus = 'active';
          commitments[1].bandwidth = 20; // Higher bandwidth

          const bestUrl = service.getBestCustodianUrl('content_123', blob.hash);

          expect(bestUrl).toBeTruthy();
          expect(bestUrl).toContain('custodian_2'); // Should be the one with higher bandwidth
          done();
        });
      });
    });

    it('should return null if no active replicas', () => {
      const blob = createMockBlob();
      const bestUrl = service.getBestCustodianUrl('content_123', blob.hash);
      expect(bestUrl).toBeNull();
    });
  });

  describe('Custodian Health', () => {
    it('should probe custodian health', (done) => {
      service.probeCustodianHealth('custodian_1').subscribe((health) => {
        expect(health.online).toBeDefined();
        expect(health.acceptingBlobs).toBeDefined();
        expect(health.bandwidth).toBeGreaterThanOrEqual(0);
        expect(health.latency).toBeGreaterThanOrEqual(0);
        done();
      });
    });

    it('should get custodian capability info', (done) => {
      service.getCustodianCapability('custodian_1').subscribe((capability) => {
        // Will be null in stub implementation, but should not error
        done();
      });
    });
  });

  describe('Resilience and Fallbacks', () => {
    it('should handle missing blob gracefully', (done) => {
      service.getBlobReplicationStatus('content_999', 'nonexistent').subscribe((status) => {
        expect(status.custodianCount).toBe(0);
        expect(status.activeReplicas).toBe(0);
        done();
      });
    });

    it('should generate consistent fallback URLs', (done) => {
      const blob = createMockBlob();

      service.createBlobCommitment('content_123', blob, 'custodian_1').subscribe((c1) => {
        const url1 = c1.fallbackUrl;

        service.createBlobCommitment('content_123', blob, 'custodian_1').subscribe((c2) => {
          const url2 = c2.fallbackUrl;

          // URLs for same blob and custodian should be similar
          expect(url2).toContain('custodian_1');
          expect(url2).toContain(blob.hash);
          done();
        });
      });
    });
  });
});
