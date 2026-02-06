/**
 * Human Service Tests
 *
 * Tests for human network management functions including:
 * - Human creation and validation
 * - Relationship management
 * - Data persistence
 * - Transformation to ContentNodes
 */

import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import {
  createHuman,
  normalizeHumanId,
  createRelationship,
  loadHumansData,
  saveHumansData,
  addHumanToFile,
  addRelationshipToFile,
  humanToContentNode,
  humanRelationshipToContentRelationship,
  importHumansToLamad,
  listRelationshipTypes,
  listHumanCategories,
  Human,
  HumanRelationship,
  RELATIONSHIP_TYPES
} from './human.service';

describe('Human Service', () => {
  let tempDir: string;
  let testFilePath: string;

  beforeEach(() => {
    // Create temporary directory for test files
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'human-service-test-'));
    testFilePath = path.join(tempDir, 'humans.json');
  });

  afterEach(() => {
    // Clean up temporary directory
    if (fs.existsSync(tempDir)) {
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  });

  describe('createHuman', () => {
    it('should create a human with required fields', () => {
      const human = createHuman({
        id: 'alice',
        displayName: 'Alice Smith',
        bio: 'Software developer and community organizer',
        category: 'workplace'
      });

      expect(human.id).toBe('human-alice');
      expect(human.displayName).toBe('Alice Smith');
      expect(human.bio).toBe('Software developer and community organizer');
      expect(human.category).toBe('workplace');
      expect(human.profileReach).toBe('community'); // default
      expect(human.createdAt).toBeDefined();
      expect(human.updatedAt).toBeDefined();
    });

    it('should preserve human- prefix if already present', () => {
      const human = createHuman({
        id: 'human-bob',
        displayName: 'Bob Jones',
        bio: 'Teacher',
        category: 'community'
      });

      expect(human.id).toBe('human-bob');
    });

    it('should accept custom profileReach', () => {
      const human = createHuman({
        id: 'charlie',
        displayName: 'Charlie Brown',
        bio: 'Private individual',
        category: 'core-family',
        profileReach: 'hidden'
      });

      expect(human.profileReach).toBe('hidden');
    });

    it('should add optional location', () => {
      const human = createHuman({
        id: 'diana',
        displayName: 'Diana Prince',
        bio: 'Community leader',
        category: 'community',
        location: {
          layer: 'municipality',
          name: 'Springfield'
        }
      });

      expect(human.location).toEqual({
        layer: 'municipality',
        name: 'Springfield'
      });
    });

    it('should add affinities if provided', () => {
      const human = createHuman({
        id: 'eve',
        displayName: 'Eve Martinez',
        bio: 'Artist and educator',
        category: 'affinity',
        affinities: ['art', 'education', 'sustainability']
      });

      expect(human.affinities).toEqual(['art', 'education', 'sustainability']);
    });

    it('should add organizations if provided', () => {
      const human = createHuman({
        id: 'frank',
        displayName: 'Frank Miller',
        bio: 'Business owner',
        category: 'local-economy',
        organizations: [
          { orgId: 'org-1', orgName: 'Local Bakery', role: 'owner' }
        ]
      });

      expect(human.organizations).toHaveLength(1);
      expect(human.organizations![0].orgName).toBe('Local Bakery');
    });

    it('should mark minor with guardians', () => {
      const human = createHuman({
        id: 'grace',
        displayName: 'Grace Lee',
        bio: 'High school student',
        category: 'community',
        isMinor: true,
        guardianIds: ['human-parent1', 'human-parent2']
      });

      expect(human.isMinor).toBe(true);
      expect(human.guardianIds).toEqual(['human-parent1', 'human-parent2']);
    });

    it('should handle minor without explicit guardian IDs', () => {
      const human = createHuman({
        id: 'henry',
        displayName: 'Henry Kim',
        bio: 'Child',
        category: 'core-family',
        isMinor: true
      });

      expect(human.isMinor).toBe(true);
      expect(human.guardianIds).toEqual([]);
    });

    it('should mark pseudonymous users', () => {
      const human = createHuman({
        id: 'iris',
        displayName: 'Iris Anonymous',
        bio: 'Privacy-focused individual',
        category: 'edge-case',
        isPseudonymous: true
      });

      expect(human.isPseudonymous).toBe(true);
    });

    it('should add notes if provided', () => {
      const human = createHuman({
        id: 'jack',
        displayName: 'Jack Wilson',
        bio: 'Volunteer coordinator',
        category: 'community',
        notes: 'Prefers email communication'
      });

      expect(human.notes).toBe('Prefers email communication');
    });

    it('should not add optional fields when not provided', () => {
      const human = createHuman({
        id: 'karen',
        displayName: 'Karen Davis',
        bio: 'Resident',
        category: 'community'
      });

      expect(human.affinities).toBeUndefined();
      expect(human.organizations).toBeUndefined();
      expect(human.communities).toBeUndefined();
      expect(human.isMinor).toBeUndefined();
      expect(human.isPseudonymous).toBeUndefined();
      expect(human.notes).toBeUndefined();
    });
  });

  describe('normalizeHumanId', () => {
    it('should add human- prefix when missing', () => {
      expect(normalizeHumanId('alice')).toBe('human-alice');
    });

    it('should not duplicate human- prefix', () => {
      expect(normalizeHumanId('human-alice')).toBe('human-alice');
    });

    it('should handle empty string', () => {
      expect(normalizeHumanId('')).toBe('human-');
    });
  });

  describe('createRelationship', () => {
    it('should create relationship with typical intimacy', () => {
      const rel = createRelationship({
        sourceId: 'alice',
        targetId: 'bob',
        relationshipType: 'friend'
      });

      expect(rel.sourceId).toBe('human-alice');
      expect(rel.targetId).toBe('human-bob');
      expect(rel.relationshipType).toBe('friend');
      expect(rel.intimacy).toBe('trusted'); // default for friend
      expect(rel.layer).toBe('personal');
      expect(rel.createdAt).toBeDefined();
    });

    it('should allow custom intimacy override', () => {
      const rel = createRelationship({
        sourceId: 'charlie',
        targetId: 'diana',
        relationshipType: 'coworker',
        intimacy: 'recognition'
      });

      expect(rel.intimacy).toBe('recognition');
    });

    it('should add contextOrgId when provided', () => {
      const rel = createRelationship({
        sourceId: 'eve',
        targetId: 'frank',
        relationshipType: 'coworker',
        contextOrgId: 'org-tech-company'
      });

      expect(rel.contextOrgId).toBe('org-tech-company');
    });

    it('should handle family relationships', () => {
      const rel = createRelationship({
        sourceId: 'parent',
        targetId: 'child',
        relationshipType: 'parent'
      });

      expect(rel.intimacy).toBe('intimate');
      expect(rel.layer).toBe('family');
    });

    it('should handle unknown relationship types', () => {
      const rel = createRelationship({
        sourceId: 'alice',
        targetId: 'bob',
        relationshipType: 'unknown-type'
      });

      expect(rel.intimacy).toBe('recognition'); // falls back to 'other'
      expect(rel.layer).toBe('community');
    });

    it('should normalize human IDs in relationships', () => {
      const rel = createRelationship({
        sourceId: 'human-grace',
        targetId: 'henry',
        relationshipType: 'sibling'
      });

      expect(rel.sourceId).toBe('human-grace');
      expect(rel.targetId).toBe('human-henry');
    });
  });

  describe('loadHumansData', () => {
    it('should return empty data when file does not exist', () => {
      const data = loadHumansData(path.join(tempDir, 'nonexistent.json'));

      expect(data.humans).toEqual([]);
      expect(data.relationships).toEqual([]);
    });

    it('should load valid humans data', () => {
      const testData = {
        humans: [
          createHuman({
            id: 'test-user',
            displayName: 'Test User',
            bio: 'Test bio',
            category: 'community'
          })
        ],
        relationships: [],
        generatedAt: new Date().toISOString()
      };

      fs.writeFileSync(testFilePath, JSON.stringify(testData), 'utf-8');

      const loaded = loadHumansData(testFilePath);

      expect(loaded.humans).toHaveLength(1);
      expect(loaded.humans[0].displayName).toBe('Test User');
    });

    it('should return empty data on JSON parse error', () => {
      fs.writeFileSync(testFilePath, 'invalid json{', 'utf-8');

      const data = loadHumansData(testFilePath);

      expect(data.humans).toEqual([]);
      expect(data.relationships).toEqual([]);
    });
  });

  describe('saveHumansData', () => {
    it('should save humans data with timestamp', () => {
      const data = {
        humans: [
          createHuman({
            id: 'alice',
            displayName: 'Alice',
            bio: 'Test',
            category: 'community'
          })
        ],
        relationships: []
      };

      saveHumansData(testFilePath, data);

      expect(fs.existsSync(testFilePath)).toBe(true);

      const loaded = JSON.parse(fs.readFileSync(testFilePath, 'utf-8'));
      expect(loaded.generatedAt).toBeDefined();
      expect(loaded.humans).toHaveLength(1);
    });
  });

  describe('addHumanToFile', () => {
    it('should add human to new file', () => {
      const human = createHuman({
        id: 'alice',
        displayName: 'Alice',
        bio: 'First user',
        category: 'community'
      });

      addHumanToFile(testFilePath, human);

      const data = loadHumansData(testFilePath);
      expect(data.humans).toHaveLength(1);
      expect(data.humans[0].id).toBe('human-alice');
    });

    it('should add human to existing file', () => {
      const human1 = createHuman({
        id: 'alice',
        displayName: 'Alice',
        bio: 'First',
        category: 'community'
      });
      const human2 = createHuman({
        id: 'bob',
        displayName: 'Bob',
        bio: 'Second',
        category: 'workplace'
      });

      addHumanToFile(testFilePath, human1);
      addHumanToFile(testFilePath, human2);

      const data = loadHumansData(testFilePath);
      expect(data.humans).toHaveLength(2);
    });

    it('should throw error on duplicate ID', () => {
      const human1 = createHuman({
        id: 'alice',
        displayName: 'Alice',
        bio: 'First',
        category: 'community'
      });

      addHumanToFile(testFilePath, human1);

      expect(() => {
        addHumanToFile(testFilePath, human1);
      }).toThrow('Human with ID human-alice already exists');
    });
  });

  describe('addRelationshipToFile', () => {
    beforeEach(() => {
      // Setup humans for relationship tests
      const alice = createHuman({
        id: 'alice',
        displayName: 'Alice',
        bio: 'User 1',
        category: 'community'
      });
      const bob = createHuman({
        id: 'bob',
        displayName: 'Bob',
        bio: 'User 2',
        category: 'community'
      });

      saveHumansData(testFilePath, {
        humans: [alice, bob],
        relationships: []
      });
    });

    it('should add relationship between existing humans', () => {
      const rel = createRelationship({
        sourceId: 'alice',
        targetId: 'bob',
        relationshipType: 'friend'
      });

      addRelationshipToFile(testFilePath, rel);

      const data = loadHumansData(testFilePath);
      expect(data.relationships).toHaveLength(1);
      expect(data.relationships[0].sourceId).toBe('human-alice');
    });

    it('should throw error when source human not found', () => {
      const rel = createRelationship({
        sourceId: 'nonexistent',
        targetId: 'bob',
        relationshipType: 'friend'
      });

      expect(() => {
        addRelationshipToFile(testFilePath, rel);
      }).toThrow('Source human not found: human-nonexistent');
    });

    it('should throw error when target human not found', () => {
      const rel = createRelationship({
        sourceId: 'alice',
        targetId: 'nonexistent',
        relationshipType: 'friend'
      });

      expect(() => {
        addRelationshipToFile(testFilePath, rel);
      }).toThrow('Target human not found: human-nonexistent');
    });

    it('should throw error on duplicate relationship', () => {
      const rel = createRelationship({
        sourceId: 'alice',
        targetId: 'bob',
        relationshipType: 'friend'
      });

      addRelationshipToFile(testFilePath, rel);

      expect(() => {
        addRelationshipToFile(testFilePath, rel);
      }).toThrow('Relationship already exists');
    });

    it('should allow same humans with different relationship types', () => {
      const rel1 = createRelationship({
        sourceId: 'alice',
        targetId: 'bob',
        relationshipType: 'friend'
      });
      const rel2 = createRelationship({
        sourceId: 'alice',
        targetId: 'bob',
        relationshipType: 'coworker'
      });

      addRelationshipToFile(testFilePath, rel1);
      addRelationshipToFile(testFilePath, rel2);

      const data = loadHumansData(testFilePath);
      expect(data.relationships).toHaveLength(2);
    });
  });

  describe('humanToContentNode', () => {
    it('should transform human to ContentNode', () => {
      const human = createHuman({
        id: 'alice',
        displayName: 'Alice Smith',
        bio: 'Community organizer',
        category: 'community',
        affinities: ['sustainability', 'education']
      });

      const node = humanToContentNode(human);

      expect(node.id).toBe('human-alice');
      expect(node.contentType).toBe('role');
      expect(node.title).toBe('Alice Smith');
      expect(node.description).toBe('Community organizer');
      expect(node.content).toBe('Community organizer');
      expect(node.contentFormat).toBe('plaintext');
      expect(node.tags).toContain('human');
      expect(node.tags).toContain('community');
      expect(node.tags).toContain('sustainability');
      expect(node.metadata.category).toBe('community');
      expect(node.metadata.profileReach).toBe('community');
    });

    it('should include organizations in tags', () => {
      const human = createHuman({
        id: 'bob',
        displayName: 'Bob Jones',
        bio: 'Manager',
        category: 'workplace',
        organizations: [
          { orgId: 'org-tech', orgName: 'Tech Corp', role: 'manager' }
        ]
      });

      const node = humanToContentNode(human);

      expect(node.tags).toContain('org-tech');
    });

    it('should include guardians and communities in relatedNodeIds', () => {
      const human = createHuman({
        id: 'charlie',
        displayName: 'Charlie Brown',
        bio: 'Student',
        category: 'community',
        isMinor: true,
        guardianIds: ['human-parent'],
        communities: ['community-school']
      });

      const node = humanToContentNode(human);

      expect(node.relatedNodeIds).toContain('human-parent');
      expect(node.relatedNodeIds).toContain('community-school');
    });

    it('should preserve timestamps', () => {
      const human = createHuman({
        id: 'diana',
        displayName: 'Diana Prince',
        bio: 'Leader',
        category: 'community'
      });

      const node = humanToContentNode(human);

      expect(node.createdAt).toBe(human.createdAt);
      expect(node.updatedAt).toBe(human.updatedAt);
    });

    it('should store location and minor status in metadata', () => {
      const human = createHuman({
        id: 'eve',
        displayName: 'Eve Martinez',
        bio: 'Artist',
        category: 'affinity',
        location: { layer: 'municipality', name: 'Portland' },
        isMinor: false,
        isPseudonymous: true
      });

      const node = humanToContentNode(human);

      expect(node.metadata.location).toEqual({
        layer: 'municipality',
        name: 'Portland'
      });
      expect(node.metadata.isMinor).toBe(false);
      expect(node.metadata.isPseudonymous).toBe(true);
    });
  });

  describe('humanRelationshipToContentRelationship', () => {
    it('should transform relationship to ContentRelationship', () => {
      const rel = createRelationship({
        sourceId: 'alice',
        targetId: 'bob',
        relationshipType: 'friend'
      });

      const contentRel = humanRelationshipToContentRelationship(rel);

      expect(contentRel.id).toContain('rel-human-alice-human-bob-friend');
      expect(contentRel.sourceNodeId).toBe('human-alice');
      expect(contentRel.targetNodeId).toBe('human-bob');
      expect(contentRel.relationshipType).toBe('RELATES_TO');
      expect(contentRel.confidence).toBe(1.0);
      expect(contentRel.inferenceSource).toBe('explicit');
    });

    it('should generate consistent IDs', () => {
      const rel1 = createRelationship({
        sourceId: 'alice',
        targetId: 'bob',
        relationshipType: 'coworker'
      });

      const contentRel1 = humanRelationshipToContentRelationship(rel1);
      const contentRel2 = humanRelationshipToContentRelationship(rel1);

      expect(contentRel1.id).toBe(contentRel2.id);
    });
  });

  describe('importHumansToLamad', () => {
    it('should import humans and relationships', async () => {
      // Setup test data
      const alice = createHuman({
        id: 'alice',
        displayName: 'Alice',
        bio: 'User 1',
        category: 'community'
      });
      const bob = createHuman({
        id: 'bob',
        displayName: 'Bob',
        bio: 'User 2',
        category: 'workplace'
      });
      const rel = createRelationship({
        sourceId: 'alice',
        targetId: 'bob',
        relationshipType: 'friend'
      });

      saveHumansData(testFilePath, {
        humans: [alice, bob],
        relationships: [rel]
      });

      // Create output directory
      const outputDir = path.join(tempDir, 'output');

      const result = await importHumansToLamad(testFilePath, outputDir);

      expect(result.humansImported).toBe(2);
      expect(result.relationshipsImported).toBe(1);
      expect(result.errors).toEqual([]);

      // Verify content files were created
      const contentDir = path.join(outputDir, 'content');
      expect(fs.existsSync(path.join(contentDir, 'human-alice.json'))).toBe(true);
      expect(fs.existsSync(path.join(contentDir, 'human-bob.json'))).toBe(true);

      // Verify relationships file
      const relPath = path.join(outputDir, 'graph', 'relationships.json');
      expect(fs.existsSync(relPath)).toBe(true);
      const rels = JSON.parse(fs.readFileSync(relPath, 'utf-8'));
      expect(rels).toHaveLength(1);
    });

    it('should merge with existing relationships', async () => {
      // Create existing relationships
      const outputDir = path.join(tempDir, 'output');
      const graphDir = path.join(outputDir, 'graph');
      fs.mkdirSync(graphDir, { recursive: true });

      const existingRel = {
        id: 'existing-rel',
        sourceNodeId: 'human-charlie',
        targetNodeId: 'human-diana',
        relationshipType: 'RELATES_TO',
        confidence: 1.0,
        inferenceSource: 'explicit'
      };
      fs.writeFileSync(
        path.join(graphDir, 'relationships.json'),
        JSON.stringify([existingRel], null, 2),
        'utf-8'
      );

      // Import new humans
      const alice = createHuman({
        id: 'alice',
        displayName: 'Alice',
        bio: 'User',
        category: 'community'
      });
      const bob = createHuman({
        id: 'bob',
        displayName: 'Bob',
        bio: 'User',
        category: 'community'
      });
      const newRel = createRelationship({
        sourceId: 'alice',
        targetId: 'bob',
        relationshipType: 'friend'
      });

      saveHumansData(testFilePath, {
        humans: [alice, bob],
        relationships: [newRel]
      });

      const result = await importHumansToLamad(testFilePath, outputDir);

      expect(result.relationshipsImported).toBe(1);

      // Verify merged relationships
      const rels = JSON.parse(
        fs.readFileSync(path.join(graphDir, 'relationships.json'), 'utf-8')
      );
      expect(rels).toHaveLength(2);
      expect(rels.some((r: any) => r.id === 'existing-rel')).toBe(true);
    });

    it('should handle errors gracefully', async () => {
      // TODO(test-generator): [MEDIUM] Improve error handling in importHumansToLamad
      // Context: Current implementation catches errors per-human but doesn't validate input
      // Story: Robust import pipeline with detailed error reporting
      // Suggested approach:
      //   1. Add input validation before processing
      //   2. Collect detailed error context (line numbers, field names)
      //   3. Return partial success with detailed error array

      const invalidData = {
        humans: [
          // Missing required fields - should this throw or skip?
          { id: 'invalid' } as any
        ],
        relationships: []
      };

      fs.writeFileSync(testFilePath, JSON.stringify(invalidData), 'utf-8');

      const outputDir = path.join(tempDir, 'output');
      const result = await importHumansToLamad(testFilePath, outputDir);

      // Current implementation may succeed with partial data
      expect(result.errors.length).toBeGreaterThan(0);
    });
  });

  describe('listRelationshipTypes', () => {
    it('should return all relationship types', () => {
      const types = listRelationshipTypes();

      expect(types.length).toBeGreaterThan(0);
      expect(types[0]).toHaveProperty('type');
      expect(types[0]).toHaveProperty('layer');
      expect(types[0]).toHaveProperty('intimacy');
    });

    it('should include family relationships', () => {
      const types = listRelationshipTypes();
      const familyTypes = types.filter(t => t.layer === 'family');

      expect(familyTypes.length).toBeGreaterThan(0);
      expect(familyTypes.some(t => t.type === 'spouse')).toBe(true);
    });

    it('should match RELATIONSHIP_TYPES constant', () => {
      const types = listRelationshipTypes();

      expect(types.length).toBe(Object.keys(RELATIONSHIP_TYPES).length);
    });
  });

  describe('listHumanCategories', () => {
    it('should return all human categories', () => {
      const categories = listHumanCategories();

      expect(categories).toContain('core-family');
      expect(categories).toContain('workplace');
      expect(categories).toContain('community');
      expect(categories).toContain('affinity');
      expect(categories).toContain('local-economy');
      expect(categories).toContain('newcomer');
      expect(categories).toContain('visitor');
      expect(categories).toContain('red-team');
      expect(categories).toContain('edge-case');
    });

    it('should return exactly 9 categories', () => {
      const categories = listHumanCategories();

      expect(categories).toHaveLength(9);
    });
  });
});
