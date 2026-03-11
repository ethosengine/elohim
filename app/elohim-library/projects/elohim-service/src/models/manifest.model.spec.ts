/**
 * Tests for ContentManifest model and factory functions
 */

import {
  ContentManifest,
  SourceHashEntry,
  NodeHashEntry,
  SchemaMigration,
  MigrationRule,
  createEmptyManifest
} from './manifest.model';

describe('Manifest Model', () => {
  describe('createEmptyManifest', () => {
    it('should create manifest with default values', () => {
      const manifest = createEmptyManifest();

      expect(manifest.manifestVersion).toBe('1.0.0');
      expect(manifest.schemaVersion).toBe('1.0.0');
      expect(manifest.importToolVersion).toBe('0.1.0');
      expect(manifest.totalSourceFiles).toBe(0);
      expect(manifest.totalNodes).toBe(0);
      expect(manifest.totalRelationships).toBe(0);
    });

    it('should create manifest with empty collections', () => {
      const manifest = createEmptyManifest();

      expect(manifest.sourceHashes).toEqual({});
      expect(manifest.nodeHashes).toEqual({});
      expect(manifest.migrations).toEqual([]);
      expect(manifest.domainStats).toEqual({});
      expect(manifest.contentTypeStats).toEqual({});
    });

    it('should set lastUpdated to current timestamp', () => {
      const before = new Date().toISOString();
      const manifest = createEmptyManifest();
      const after = new Date().toISOString();

      expect(manifest.lastUpdated).toBeDefined();
      expect(manifest.lastUpdated).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/);
      // Should be between before and after
      expect(manifest.lastUpdated >= before).toBe(true);
      expect(manifest.lastUpdated <= after).toBe(true);
    });

    it('should create independent manifests on multiple calls', () => {
      const manifest1 = createEmptyManifest();
      const manifest2 = createEmptyManifest();

      // Modify one
      manifest1.totalNodes = 10;
      manifest1.sourceHashes['test'] = {
        hash: 'abc123',
        lastModified: '2026-01-01',
        generatedNodeIds: ['node1']
      };

      // Other should be unchanged
      expect(manifest2.totalNodes).toBe(0);
      expect(manifest2.sourceHashes).toEqual({});
    });
  });

  describe('SourceHashEntry interface', () => {
    it('should accept valid source hash entry', () => {
      const entry: SourceHashEntry = {
        hash: 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
        lastModified: '2026-02-01T12:00:00.000Z',
        generatedNodeIds: ['epic-governance', 'source-epic-governance']
      };

      expect(entry.hash).toHaveLength(64); // SHA256 hex length
      expect(entry.generatedNodeIds).toHaveLength(2);
    });

    it('should accept entry with empty generatedNodeIds', () => {
      const entry: SourceHashEntry = {
        hash: 'abc123',
        lastModified: '2026-02-01T12:00:00.000Z',
        generatedNodeIds: []
      };

      expect(entry.generatedNodeIds).toEqual([]);
    });

    it('should accept entry with multiple node IDs', () => {
      const entry: SourceHashEntry = {
        hash: 'def456',
        lastModified: '2026-02-01T12:00:00.000Z',
        generatedNodeIds: ['node1', 'node2', 'node3', 'node4', 'node5']
      };

      expect(entry.generatedNodeIds).toHaveLength(5);
    });
  });

  describe('NodeHashEntry interface', () => {
    it('should accept valid node hash entry', () => {
      const entry: NodeHashEntry = {
        hash: 'abc123def456',
        sourcePath: 'data/content/elohim-protocol/governance/epic.md',
        contentType: 'epic',
        generatedAt: '2026-02-01T12:00:00.000Z'
      };

      expect(entry.sourcePath).toContain('governance');
      expect(entry.contentType).toBe('epic');
    });

    it('should track relationship between node and source', () => {
      const sourceEntry: SourceHashEntry = {
        hash: 'source-hash',
        lastModified: '2026-02-01T12:00:00.000Z',
        generatedNodeIds: ['epic-governance']
      };

      const nodeEntry: NodeHashEntry = {
        hash: 'node-hash',
        sourcePath: 'data/content/elohim-protocol/governance/epic.md',
        contentType: 'epic',
        generatedAt: '2026-02-01T12:00:00.000Z'
      };

      expect(sourceEntry.generatedNodeIds).toContain('epic-governance');
      expect(nodeEntry.sourcePath).toBe('data/content/elohim-protocol/governance/epic.md');
    });
  });

  describe('MigrationRule interface', () => {
    it('should accept lowercase transformation', () => {
      const rule: MigrationRule = {
        field: 'contentType',
        transform: 'lowercase'
      };

      expect(rule.transform).toBe('lowercase');
    });

    it('should accept rename transformation with newField', () => {
      const rule: MigrationRule = {
        field: 'oldFieldName',
        transform: 'rename',
        newField: 'newFieldName'
      };

      expect(rule.newField).toBe('newFieldName');
    });

    it('should accept default transformation with defaultValue', () => {
      const rule: MigrationRule = {
        field: 'reach',
        transform: 'default',
        defaultValue: 'commons'
      };

      expect(rule.defaultValue).toBe('commons');
    });

    it('should accept delete transformation', () => {
      const rule: MigrationRule = {
        field: 'obsoleteField',
        transform: 'delete'
      };

      expect(rule.transform).toBe('delete');
    });

    it('should accept custom transformation with function string', () => {
      const rule: MigrationRule = {
        field: 'tags',
        transform: 'custom',
        customTransform: 'value => value.map(t => t.toLowerCase())'
      };

      expect(rule.customTransform).toContain('toLowerCase');
    });

    it('should accept all valid transform types', () => {
      const transforms: MigrationRule['transform'][] = [
        'lowercase',
        'uppercase',
        'rename',
        'delete',
        'default',
        'custom'
      ];

      expect(transforms).toHaveLength(6);
    });
  });

  describe('SchemaMigration interface', () => {
    it('should accept valid migration record', () => {
      const migration: SchemaMigration = {
        id: 'migration-001-normalize-content-types',
        fromVersion: '0.9.0',
        toVersion: '1.0.0',
        appliedAt: '2026-02-01T12:00:00.000Z',
        nodesMigrated: 150,
        rules: [
          {
            field: 'contentType',
            transform: 'lowercase'
          }
        ]
      };

      expect(migration.nodesMigrated).toBe(150);
      expect(migration.rules).toHaveLength(1);
    });

    it('should accept migration with multiple rules', () => {
      const migration: SchemaMigration = {
        id: 'migration-002-schema-overhaul',
        fromVersion: '1.0.0',
        toVersion: '2.0.0',
        appliedAt: '2026-02-15T12:00:00.000Z',
        nodesMigrated: 500,
        rules: [
          { field: 'contentType', transform: 'lowercase' },
          { field: 'oldField', transform: 'rename', newField: 'newField' },
          { field: 'reach', transform: 'default', defaultValue: 'commons' }
        ]
      };

      expect(migration.rules).toHaveLength(3);
      expect(migration.nodesMigrated).toBe(500);
    });
  });

  describe('ContentManifest interface', () => {
    it('should accept complete manifest', () => {
      const manifest: ContentManifest = {
        manifestVersion: '1.0.0',
        schemaVersion: '1.0.0',
        lastUpdated: '2026-02-01T12:00:00.000Z',
        importToolVersion: '0.1.0',
        totalSourceFiles: 10,
        totalNodes: 25,
        totalRelationships: 40,
        sourceHashes: {
          'data/content/elohim-protocol/governance/epic.md': {
            hash: 'abc123',
            lastModified: '2026-01-15T10:00:00.000Z',
            generatedNodeIds: ['epic-governance']
          }
        },
        nodeHashes: {
          'epic-governance': {
            hash: 'def456',
            sourcePath: 'data/content/elohim-protocol/governance/epic.md',
            contentType: 'epic',
            generatedAt: '2026-02-01T12:00:00.000Z'
          }
        },
        migrations: [],
        domainStats: {
          'elohim-protocol': {
            sourceFiles: 10,
            nodes: 25,
            lastImported: '2026-02-01T12:00:00.000Z'
          }
        },
        contentTypeStats: {
          'epic': {
            count: 1,
            lastUpdated: '2026-02-01T12:00:00.000Z'
          }
        }
      };

      expect(manifest.totalSourceFiles).toBe(10);
      expect(manifest.totalNodes).toBe(25);
      expect(manifest.totalRelationships).toBe(40);
    });

    it('should track domain statistics', () => {
      const manifest = createEmptyManifest();

      manifest.domainStats['elohim-protocol'] = {
        sourceFiles: 50,
        nodes: 120,
        lastImported: '2026-02-01T12:00:00.000Z'
      };

      manifest.domainStats['fct'] = {
        sourceFiles: 20,
        nodes: 45,
        lastImported: '2026-01-28T12:00:00.000Z'
      };

      expect(Object.keys(manifest.domainStats)).toHaveLength(2);
      expect(manifest.domainStats['elohim-protocol'].nodes).toBe(120);
      expect(manifest.domainStats['fct'].nodes).toBe(45);
    });

    it('should track content type statistics', () => {
      const manifest = createEmptyManifest();

      manifest.contentTypeStats['epic'] = {
        count: 5,
        lastUpdated: '2026-02-01T12:00:00.000Z'
      };

      manifest.contentTypeStats['scenario'] = {
        count: 30,
        lastUpdated: '2026-02-01T12:00:00.000Z'
      };

      expect(Object.keys(manifest.contentTypeStats)).toHaveLength(2);
      expect(manifest.contentTypeStats['epic'].count).toBe(5);
      expect(manifest.contentTypeStats['scenario'].count).toBe(30);
    });

    it('should support incremental updates', () => {
      const manifest = createEmptyManifest();

      // Initial import
      manifest.totalSourceFiles = 10;
      manifest.totalNodes = 25;

      // Add new source
      manifest.sourceHashes['new-file.md'] = {
        hash: 'new-hash',
        lastModified: '2026-02-01T13:00:00.000Z',
        generatedNodeIds: ['new-node']
      };

      manifest.totalSourceFiles = 11;
      manifest.totalNodes = 26;

      expect(manifest.totalSourceFiles).toBe(11);
      expect(manifest.totalNodes).toBe(26);
      expect(Object.keys(manifest.sourceHashes)).toHaveLength(1);
    });

    it('should track multiple migrations', () => {
      const manifest = createEmptyManifest();

      manifest.migrations.push({
        id: 'migration-001',
        fromVersion: '0.9.0',
        toVersion: '1.0.0',
        appliedAt: '2026-01-15T12:00:00.000Z',
        nodesMigrated: 100,
        rules: [{ field: 'contentType', transform: 'lowercase' }]
      });

      manifest.migrations.push({
        id: 'migration-002',
        fromVersion: '1.0.0',
        toVersion: '1.1.0',
        appliedAt: '2026-02-01T12:00:00.000Z',
        nodesMigrated: 100,
        rules: [{ field: 'reach', transform: 'default', defaultValue: 'commons' }]
      });

      expect(manifest.migrations).toHaveLength(2);
      expect(manifest.migrations[0].fromVersion).toBe('0.9.0');
      expect(manifest.migrations[1].toVersion).toBe('1.1.0');
    });
  });
});
