/**
 * Trust Service Tests
 *
 * Tests for trust scoring and attestation-based content enrichment.
 */

import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import {
  loadAttestations,
  calculateTrustScore,
  getEffectiveReach,
  generateTrustFields,
  enrichWithTrust,
  enrichContentDirectory,
  updateContentIndexWithTrust,
  ReachLevel,
  Attestation
} from './trust.service';

describe('Trust Service', () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'trust-service-test-'));
  });

  afterEach(() => {
    if (fs.existsSync(tempDir)) {
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  });

  describe('loadAttestations', () => {
    it('should return empty map when file does not exist', () => {
      const attestations = loadAttestations(path.join(tempDir, 'nonexistent.json'));

      expect(attestations.size).toBe(0);
    });

    it('should load attestations grouped by content ID', () => {
      const testData = {
        attestations: [
          {
            id: 'att-1',
            contentId: 'content-1',
            attestationType: 'author-verified',
            status: 'active'
          },
          {
            id: 'att-2',
            contentId: 'content-1',
            attestationType: 'peer-reviewed',
            status: 'active'
          },
          {
            id: 'att-3',
            contentId: 'content-2',
            attestationType: 'steward-approved',
            status: 'active'
          }
        ]
      };

      const filePath = path.join(tempDir, 'attestations.json');
      fs.writeFileSync(filePath, JSON.stringify(testData), 'utf-8');

      const attestations = loadAttestations(filePath);

      expect(attestations.size).toBe(2);
      expect(attestations.get('content-1')).toHaveLength(2);
      expect(attestations.get('content-2')).toHaveLength(1);
    });

    it('should handle invalid JSON gracefully', () => {
      const filePath = path.join(tempDir, 'invalid.json');
      fs.writeFileSync(filePath, 'invalid json{', 'utf-8');

      const attestations = loadAttestations(filePath);

      expect(attestations.size).toBe(0);
    });

    it('should skip attestations without contentId', () => {
      const testData = {
        attestations: [
          {
            id: 'att-1',
            contentId: 'content-1',
            attestationType: 'author-verified',
            status: 'active'
          },
          {
            id: 'att-2',
            // missing contentId
            attestationType: 'peer-reviewed',
            status: 'active'
          } as any
        ]
      };

      const filePath = path.join(tempDir, 'attestations.json');
      fs.writeFileSync(filePath, JSON.stringify(testData), 'utf-8');

      const attestations = loadAttestations(filePath);

      expect(attestations.size).toBe(1);
      expect(attestations.get('content-1')).toHaveLength(1);
    });
  });

  describe('calculateTrustScore', () => {
    it('should return default score for empty attestations', () => {
      const score = calculateTrustScore([]);

      expect(score).toBe(0.8);
    });

    it('should calculate score from active attestations', () => {
      const attestations: Attestation[] = [
        {
          id: 'att-1',
          contentId: 'content-1',
          attestationType: 'author-verified',
          status: 'active'
        },
        {
          id: 'att-2',
          contentId: 'content-1',
          attestationType: 'peer-reviewed',
          status: 'active'
        }
      ];

      const score = calculateTrustScore(attestations);

      // author-verified (0.1) + peer-reviewed (0.4) = 0.5 / 1.5 = 0.333...
      expect(score).toBeCloseTo(0.333, 2);
    });

    it('should ignore revoked attestations', () => {
      const attestations: Attestation[] = [
        {
          id: 'att-1',
          contentId: 'content-1',
          attestationType: 'peer-reviewed',
          status: 'active'
        },
        {
          id: 'att-2',
          contentId: 'content-1',
          attestationType: 'governance-ratified',
          status: 'revoked'
        }
      ];

      const score = calculateTrustScore(attestations);

      // Only peer-reviewed counts: 0.4 / 1.5 = 0.266...
      expect(score).toBeCloseTo(0.267, 2);
    });

    it('should ignore expired attestations', () => {
      const attestations: Attestation[] = [
        {
          id: 'att-1',
          contentId: 'content-1',
          attestationType: 'steward-approved',
          status: 'expired'
        }
      ];

      const score = calculateTrustScore(attestations);

      expect(score).toBe(0.8); // falls back to default
    });

    it('should cap score at 1.0', () => {
      // All high-weight attestations
      const attestations: Attestation[] = [
        {
          id: 'att-1',
          contentId: 'content-1',
          attestationType: 'governance-ratified',
          status: 'active'
        },
        {
          id: 'att-2',
          contentId: 'content-1',
          attestationType: 'curriculum-canonical',
          status: 'active'
        },
        {
          id: 'att-3',
          contentId: 'content-1',
          attestationType: 'peer-reviewed',
          status: 'active'
        }
      ];

      const score = calculateTrustScore(attestations);

      expect(score).toBeLessThanOrEqual(1.0);
      expect(score).toBeGreaterThan(0.8);
    });

    it('should use default weight for unknown attestation types', () => {
      const attestations: Attestation[] = [
        {
          id: 'att-1',
          contentId: 'content-1',
          attestationType: 'unknown-type',
          status: 'active'
        }
      ];

      const score = calculateTrustScore(attestations);

      // 0.1 (default) / 1.5 = 0.066...
      expect(score).toBeCloseTo(0.067, 2);
    });
  });

  describe('getEffectiveReach', () => {
    it('should return default reach for empty attestations', () => {
      const reach = getEffectiveReach([]);

      expect(reach).toBe('commons');
    });

    it('should return highest reach from active attestations', () => {
      const attestations: Attestation[] = [
        {
          id: 'att-1',
          contentId: 'content-1',
          attestationType: 'author-verified',
          status: 'active',
          reachGranted: 'local'
        },
        {
          id: 'att-2',
          contentId: 'content-1',
          attestationType: 'peer-reviewed',
          status: 'active',
          reachGranted: 'federated'
        },
        {
          id: 'att-3',
          contentId: 'content-1',
          attestationType: 'steward-approved',
          status: 'active',
          reachGranted: 'community'
        }
      ];

      const reach = getEffectiveReach(attestations);

      expect(reach).toBe('federated'); // highest level
    });

    it('should ignore revoked attestations', () => {
      const attestations: Attestation[] = [
        {
          id: 'att-1',
          contentId: 'content-1',
          attestationType: 'author-verified',
          status: 'active',
          reachGranted: 'local'
        },
        {
          id: 'att-2',
          contentId: 'content-1',
          attestationType: 'governance-ratified',
          status: 'revoked',
          reachGranted: 'commons'
        }
      ];

      const reach = getEffectiveReach(attestations);

      expect(reach).toBe('local');
    });

    it('should return private for attestations without reachGranted', () => {
      const attestations: Attestation[] = [
        {
          id: 'att-1',
          contentId: 'content-1',
          attestationType: 'author-verified',
          status: 'active'
        }
      ];

      const reach = getEffectiveReach(attestations);

      expect(reach).toBe('private');
    });
  });

  describe('generateTrustFields', () => {
    it('should generate trust fields with attestations', () => {
      const attestations: Attestation[] = [
        {
          id: 'att-1',
          contentId: 'content-1',
          attestationType: 'peer-reviewed',
          status: 'active',
          reachGranted: 'community'
        },
        {
          id: 'att-2',
          contentId: 'content-1',
          attestationType: 'author-verified',
          status: 'active',
          reachGranted: 'local'
        }
      ];

      const fields = generateTrustFields('content-1', attestations);

      expect(fields.authorId).toBe('system');
      expect(fields.reach).toBe('community');
      expect(fields.trustScore).toBeCloseTo(0.33, 1);
      expect(fields.activeAttestationIds).toEqual(['att-1', 'att-2']);
      expect(fields.trustComputedAt).toBeDefined();
    });

    it('should use existing author ID if provided', () => {
      const fields = generateTrustFields('content-1', [], 'author-123');

      expect(fields.authorId).toBe('author-123');
    });

    it('should round trust score to 2 decimals', () => {
      const attestations: Attestation[] = [
        {
          id: 'att-1',
          contentId: 'content-1',
          attestationType: 'author-verified',
          status: 'active'
        }
      ];

      const fields = generateTrustFields('content-1', attestations);

      // 0.1 / 1.5 = 0.0666... should round to 0.07
      expect(fields.trustScore).toBe(0.07);
    });

    it('should filter out inactive attestations from IDs', () => {
      const attestations: Attestation[] = [
        {
          id: 'att-1',
          contentId: 'content-1',
          attestationType: 'peer-reviewed',
          status: 'active'
        },
        {
          id: 'att-2',
          contentId: 'content-1',
          attestationType: 'steward-approved',
          status: 'revoked'
        },
        {
          id: 'att-3',
          contentId: 'content-1',
          attestationType: 'author-verified',
          status: 'expired'
        }
      ];

      const fields = generateTrustFields('content-1', attestations);

      expect(fields.activeAttestationIds).toEqual(['att-1']);
    });
  });

  describe('enrichWithTrust', () => {
    it('should enrich content with trust fields', () => {
      const content = {
        id: 'content-1',
        title: 'Test Content',
        authorId: 'author-123'
      };

      const attestations: Attestation[] = [
        {
          id: 'att-1',
          contentId: 'content-1',
          attestationType: 'peer-reviewed',
          status: 'active',
          reachGranted: 'federated'
        }
      ];

      const attestationsByContent = new Map([['content-1', attestations]]);

      const enriched = enrichWithTrust(content, attestationsByContent);

      expect(enriched.id).toBe('content-1');
      expect(enriched.title).toBe('Test Content');
      expect(enriched.authorId).toBe('author-123'); // preserved
      expect(enriched.reach).toBe('federated');
      expect(enriched.trustScore).toBeDefined();
      expect(enriched.activeAttestationIds).toEqual(['att-1']);
      expect(enriched.trustComputedAt).toBeDefined();
    });

    it('should use default values when no attestations', () => {
      const content = {
        id: 'content-2',
        title: 'Test Content'
      };

      const attestationsByContent = new Map();

      const enriched = enrichWithTrust(content, attestationsByContent);

      expect(enriched.reach).toBe('commons');
      expect(enriched.trustScore).toBe(0.8);
      expect(enriched.activeAttestationIds).toEqual([]);
      expect(enriched.authorId).toBe('system');
    });
  });

  describe('enrichContentDirectory', () => {
    it('should enrich all JSON files in directory', async () => {
      // Create test content files
      const contentDir = path.join(tempDir, 'content');
      fs.mkdirSync(contentDir);

      fs.writeFileSync(
        path.join(contentDir, 'content-1.json'),
        JSON.stringify({ id: 'content-1', title: 'First' }),
        'utf-8'
      );
      fs.writeFileSync(
        path.join(contentDir, 'content-2.json'),
        JSON.stringify({ id: 'content-2', title: 'Second' }),
        'utf-8'
      );

      // Create attestations
      const attestationsPath = path.join(tempDir, 'attestations.json');
      fs.writeFileSync(
        attestationsPath,
        JSON.stringify({
          attestations: [
            {
              id: 'att-1',
              contentId: 'content-1',
              attestationType: 'peer-reviewed',
              status: 'active',
              reachGranted: 'community'
            }
          ]
        }),
        'utf-8'
      );

      const result = await enrichContentDirectory(contentDir, attestationsPath);

      expect(result.processed).toBe(2);
      expect(result.enriched).toBe(2);
      expect(result.withAttestations).toBe(1);
      expect(result.errors).toEqual([]);

      // Verify enriched content
      const content1 = JSON.parse(
        fs.readFileSync(path.join(contentDir, 'content-1.json'), 'utf-8')
      );
      expect(content1.reach).toBe('community');
      expect(content1.trustScore).toBeDefined();

      const content2 = JSON.parse(
        fs.readFileSync(path.join(contentDir, 'content-2.json'), 'utf-8')
      );
      expect(content2.reach).toBe('commons'); // default
    });

    it('should skip index.json', async () => {
      const contentDir = path.join(tempDir, 'content');
      fs.mkdirSync(contentDir);

      fs.writeFileSync(
        path.join(contentDir, 'index.json'),
        JSON.stringify({ nodes: [] }),
        'utf-8'
      );
      fs.writeFileSync(
        path.join(contentDir, 'content-1.json'),
        JSON.stringify({ id: 'content-1', title: 'Test' }),
        'utf-8'
      );

      const attestationsPath = path.join(tempDir, 'attestations.json');
      fs.writeFileSync(attestationsPath, JSON.stringify({ attestations: [] }), 'utf-8');

      const result = await enrichContentDirectory(contentDir, attestationsPath);

      expect(result.processed).toBe(1); // only content-1.json
    });

    it('should handle file errors gracefully', async () => {
      const contentDir = path.join(tempDir, 'content');
      fs.mkdirSync(contentDir);

      // Create invalid JSON file
      fs.writeFileSync(path.join(contentDir, 'broken.json'), 'invalid json{', 'utf-8');

      const attestationsPath = path.join(tempDir, 'attestations.json');
      fs.writeFileSync(attestationsPath, JSON.stringify({ attestations: [] }), 'utf-8');

      const result = await enrichContentDirectory(contentDir, attestationsPath);

      expect(result.errors.length).toBeGreaterThan(0);
      expect(result.errors[0]).toContain('broken.json');
    });
  });

  describe('updateContentIndexWithTrust', () => {
    it('should update index with trust summaries', () => {
      const indexPath = path.join(tempDir, 'index.json');
      const index = {
        nodes: [
          { id: 'content-1' },
          { id: 'content-2' }
        ]
      };
      fs.writeFileSync(indexPath, JSON.stringify(index), 'utf-8');

      const attestationsByContent = new Map<string, Attestation[]>([
        [
          'content-1',
          [
            {
              id: 'att-1',
              contentId: 'content-1',
              attestationType: 'peer-reviewed',
              status: 'active',
              reachGranted: 'federated'
            }
          ]
        ]
      ]);

      updateContentIndexWithTrust(indexPath, attestationsByContent);

      const updated = JSON.parse(fs.readFileSync(indexPath, 'utf-8'));

      expect(updated.nodes[0].reach).toBe('federated');
      expect(updated.nodes[0].trustScore).toBeDefined();
      expect(updated.nodes[0].hasAttestations).toBe(true);

      expect(updated.nodes[1].reach).toBe('commons'); // default
      expect(updated.nodes[1].hasAttestations).toBe(false);

      expect(updated.lastUpdated).toBeDefined();
    });

    it('should handle missing index file gracefully', () => {
      const indexPath = path.join(tempDir, 'nonexistent.json');
      const attestationsByContent = new Map();

      // Should not throw
      expect(() => {
        updateContentIndexWithTrust(indexPath, attestationsByContent);
      }).not.toThrow();
    });

    it('should handle invalid JSON gracefully', () => {
      const indexPath = path.join(tempDir, 'index.json');
      fs.writeFileSync(indexPath, 'invalid json{', 'utf-8');

      const attestationsByContent = new Map();

      // Should not throw
      expect(() => {
        updateContentIndexWithTrust(indexPath, attestationsByContent);
      }).not.toThrow();
    });
  });
});
