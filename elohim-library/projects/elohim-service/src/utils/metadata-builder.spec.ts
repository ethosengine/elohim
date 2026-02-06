import { buildBaseMetadata, addProvenanceMetadata, addGovernanceScopeMetadata } from './metadata-builder';

describe('metadata-builder', () => {
  describe('buildBaseMetadata', () => {
    it('should create metadata with default version', () => {
      const result = buildBaseMetadata();
      expect(result).toEqual({
        source: 'elohim-import',
        sourceVersion: '1.0.0'
      });
    });

    it('should create metadata with custom version', () => {
      const result = buildBaseMetadata('2.0.0');
      expect(result).toEqual({
        source: 'elohim-import',
        sourceVersion: '2.0.0'
      });
    });
  });

  describe('addProvenanceMetadata', () => {
    it('should add provenance metadata when sourceNodeId is provided', () => {
      const metadata: Record<string, unknown> = {};
      addProvenanceMetadata(metadata, 'source-123');

      expect(metadata).toEqual({
        derivedFrom: 'source-123',
        extractionMethod: 'direct-import'
      });
    });

    it('should add provenance metadata with custom extraction method', () => {
      const metadata: Record<string, unknown> = {};
      addProvenanceMetadata(metadata, 'source-456', 'gherkin-parse');

      expect(metadata).toEqual({
        derivedFrom: 'source-456',
        extractionMethod: 'gherkin-parse'
      });
    });

    it('should not add metadata when sourceNodeId is undefined', () => {
      const metadata: Record<string, unknown> = { existing: 'data' };
      addProvenanceMetadata(metadata, undefined);

      expect(metadata).toEqual({ existing: 'data' });
    });

    it('should not add metadata when sourceNodeId is empty string', () => {
      const metadata: Record<string, unknown> = {};
      addProvenanceMetadata(metadata, '');

      expect(metadata).toEqual({});
    });
  });

  describe('addGovernanceScopeMetadata', () => {
    it('should add governance scope when present in frontmatter', () => {
      const metadata: Record<string, unknown> = {};
      const frontmatter = {
        governance_scope: ['local', 'regional']
      };

      addGovernanceScopeMetadata(metadata, frontmatter);

      expect(metadata.governanceScope).toEqual(['local', 'regional']);
    });

    it('should not add governance scope when not present', () => {
      const metadata: Record<string, unknown> = {};
      const frontmatter = {};

      addGovernanceScopeMetadata(metadata, frontmatter);

      expect(metadata.governanceScope).toBeUndefined();
    });

    it('should not add governance scope when empty array', () => {
      const metadata: Record<string, unknown> = {};
      const frontmatter = {
        governance_scope: []
      };

      addGovernanceScopeMetadata(metadata, frontmatter);

      expect(metadata.governanceScope).toBeUndefined();
    });

    it('should preserve existing metadata fields', () => {
      const metadata: Record<string, unknown> = {
        category: 'epic',
        epicName: 'governance'
      };
      const frontmatter = {
        governance_scope: ['global']
      };

      addGovernanceScopeMetadata(metadata, frontmatter);

      expect(metadata).toEqual({
        category: 'epic',
        epicName: 'governance',
        governanceScope: ['global']
      });
    });
  });
});
