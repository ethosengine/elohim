import {
  RelationshipType,
  NodeRelationship,
  createBidirectionalRelationship
} from './node-relationship.model';

describe('NodeRelationship Model', () => {
  describe('RelationshipType enum', () => {
    it('should have all expected relationship types', () => {
      expect(RelationshipType.DESCRIBES).toBe('describes');
      expect(RelationshipType.IMPLEMENTS).toBe('implements');
      expect(RelationshipType.BELONGS_TO).toBe('belongs_to');
      expect(RelationshipType.VALIDATES).toBe('validates');
      expect(RelationshipType.RELATES_TO).toBe('relates_to');
      expect(RelationshipType.REFERENCES).toBe('references');
      expect(RelationshipType.DEPENDS_ON).toBe('depends_on');
    });
  });

  describe('createBidirectionalRelationship', () => {
    it('should create forward and reverse relationships', () => {
      const relationships = createBidirectionalRelationship(
        RelationshipType.DESCRIBES,
        'epic-1',
        'feature-1',
        'Epic describes feature'
      );

      expect(relationships.length).toBe(2);
      expect(relationships[0].sourceId).toBe('epic-1');
      expect(relationships[0].targetId).toBe('feature-1');
      expect(relationships[0].type).toBe(RelationshipType.DESCRIBES);
      expect(relationships[0].bidirectional).toBe(true);
      expect(relationships[0].description).toBe('Epic describes feature');
    });

    it('should create reverse relationship with correct type for DESCRIBES', () => {
      const relationships = createBidirectionalRelationship(
        RelationshipType.DESCRIBES,
        'epic-1',
        'feature-1'
      );

      expect(relationships[1].sourceId).toBe('feature-1');
      expect(relationships[1].targetId).toBe('epic-1');
      expect(relationships[1].type).toBe(RelationshipType.IMPLEMENTS);
      expect(relationships[1].bidirectional).toBe(true);
    });

    it('should create reverse relationship with correct type for IMPLEMENTS', () => {
      const relationships = createBidirectionalRelationship(
        RelationshipType.IMPLEMENTS,
        'feature-1',
        'epic-1'
      );

      expect(relationships[1].type).toBe(RelationshipType.DESCRIBES);
    });

    it('should create reverse relationship with correct type for REFERENCES', () => {
      const relationships = createBidirectionalRelationship(
        RelationshipType.REFERENCES,
        'epic-1',
        'epic-2'
      );

      expect(relationships[1].type).toBe(RelationshipType.REFERENCES);
    });

    it('should create reverse relationship with correct type for RELATES_TO', () => {
      const relationships = createBidirectionalRelationship(
        RelationshipType.RELATES_TO,
        'node-1',
        'node-2'
      );

      expect(relationships[1].type).toBe(RelationshipType.RELATES_TO);
    });

    it('should create reverse relationship with RELATES_TO for BELONGS_TO', () => {
      const relationships = createBidirectionalRelationship(
        RelationshipType.BELONGS_TO,
        'scenario-1',
        'feature-1'
      );

      expect(relationships[1].type).toBe(RelationshipType.RELATES_TO);
    });

    it('should create reverse relationship with RELATES_TO for VALIDATES', () => {
      const relationships = createBidirectionalRelationship(
        RelationshipType.VALIDATES,
        'scenario-1',
        'epic-1'
      );

      expect(relationships[1].type).toBe(RelationshipType.RELATES_TO);
    });

    it('should create reverse relationship with RELATES_TO for DEPENDS_ON', () => {
      const relationships = createBidirectionalRelationship(
        RelationshipType.DEPENDS_ON,
        'node-1',
        'node-2'
      );

      expect(relationships[1].type).toBe(RelationshipType.RELATES_TO);
    });

    it('should generate unique IDs for forward and reverse relationships', () => {
      const relationships = createBidirectionalRelationship(
        RelationshipType.DESCRIBES,
        'epic-1',
        'feature-1'
      );

      expect(relationships[0].id).toContain('epic-1_feature-1_describes_forward');
      expect(relationships[1].id).toContain('epic-1_feature-1_describes_reverse');
      expect(relationships[0].id).not.toBe(relationships[1].id);
    });

    it('should work without description', () => {
      const relationships = createBidirectionalRelationship(
        RelationshipType.DESCRIBES,
        'epic-1',
        'feature-1'
      );

      expect(relationships[0].description).toBeUndefined();
      expect(relationships[1].description).toBeUndefined();
    });
  });

  describe('NodeRelationship interface', () => {
    it('should accept valid relationship objects', () => {
      const relationship: NodeRelationship = {
        id: 'test-rel-1',
        type: RelationshipType.DESCRIBES,
        sourceId: 'node-1',
        targetId: 'node-2',
        bidirectional: false
      };

      expect(relationship.id).toBe('test-rel-1');
      expect(relationship.type).toBe(RelationshipType.DESCRIBES);
      expect(relationship.sourceId).toBe('node-1');
      expect(relationship.targetId).toBe('node-2');
      expect(relationship.bidirectional).toBe(false);
    });

    it('should accept optional weight', () => {
      const relationship: NodeRelationship = {
        id: 'test-rel-1',
        type: RelationshipType.DESCRIBES,
        sourceId: 'node-1',
        targetId: 'node-2',
        weight: 0.8,
        bidirectional: false
      };

      expect(relationship.weight).toBe(0.8);
    });

    it('should accept optional description', () => {
      const relationship: NodeRelationship = {
        id: 'test-rel-1',
        type: RelationshipType.DESCRIBES,
        sourceId: 'node-1',
        targetId: 'node-2',
        description: 'Test description',
        bidirectional: false
      };

      expect(relationship.description).toBe('Test description');
    });

    it('should accept optional metadata', () => {
      const relationship: NodeRelationship = {
        id: 'test-rel-1',
        type: RelationshipType.DESCRIBES,
        sourceId: 'node-1',
        targetId: 'node-2',
        metadata: { custom: 'value' },
        bidirectional: false
      };

      expect(relationship.metadata).toEqual({ custom: 'value' });
    });
  });
});
