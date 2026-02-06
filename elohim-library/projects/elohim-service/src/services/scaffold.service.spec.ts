/**
 * Scaffold Service Tests
 *
 * Tests for template generation for user types and epics.
 */

import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import {
  generateReadme,
  generateTodo,
  scaffoldUserType,
  scaffoldEpic,
  scaffoldAll,
  listEpicsAndUsers,
  getTotalUserCount,
  EPICS
} from './scaffold.service';

describe('Scaffold Service', () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'scaffold-service-test-'));
  });

  afterEach(() => {
    if (fs.existsSync(tempDir)) {
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  });

  describe('generateReadme', () => {
    it('should generate README with frontmatter', () => {
      const readme = generateReadme('value_scanner', 'adult', 'Care Economy');

      expect(readme).toContain('---');
      expect(readme).toContain('epic: value_scanner');
      expect(readme).toContain('user_type: adult');
      expect(readme).toContain('archetype_name: "Adult"');
      expect(readme).toContain('epic_domain: "Care Economy"');
    });

    it('should format user type as title case', () => {
      const readme = generateReadme('value_scanner', 'young_adult', 'Care Economy');

      expect(readme).toContain('archetype_name: "Young Adult"');
      expect(readme).toContain('# Young Adult');
    });

    it('should format epic as title case', () => {
      const readme = generateReadme('public_observer', 'citizen', 'Civic Democracy');

      expect(readme).toContain('# Citizen - Public Observer');
    });

    it('should include standard sections', () => {
      const readme = generateReadme('value_scanner', 'adult', 'Care Economy');

      expect(readme).toContain('## Archetype');
      expect(readme).toContain('## Core Needs');
      expect(readme).toContain('## Key Relationships');
      expect(readme).toContain('## Relevant Governance Layers');
      expect(readme).toContain('## Implementation Notes');
    });

    it('should include TODO markers', () => {
      const readme = generateReadme('value_scanner', 'adult', 'Care Economy');

      expect(readme).toContain('[**TODO**:');
    });
  });

  describe('generateTodo', () => {
    it('should generate TODO with checklists', () => {
      const todo = generateTodo('value_scanner', 'adult');

      expect(todo).toContain('# Scenarios TODO');
      expect(todo).toContain('## Required Scenario Files');
      expect(todo).toContain('### Geographic/Political Layers');
      expect(todo).toContain('### Functional Layers');
    });

    it('should include all geographic layers', () => {
      const todo = generateTodo('value_scanner', 'adult');

      expect(todo).toContain('individual');
      expect(todo).toContain('family');
      expect(todo).toContain('neighborhood');
      expect(todo).toContain('municipality');
      expect(todo).toContain('global');
    });

    it('should include all functional layers', () => {
      const todo = generateTodo('value_scanner', 'adult');

      expect(todo).toContain('workplace_organizational');
      expect(todo).toContain('educational');
      expect(todo).toContain('ecological_bioregional');
    });

    it('should include scenario file format template', () => {
      const todo = generateTodo('value_scanner', 'adult');

      expect(todo).toContain('## Scenario File Format');
      expect(todo).toContain('```yaml');
    });

    it('should format user type in headings', () => {
      const todo = generateTodo('value_scanner', 'young_adult');

      expect(todo).toContain('Young Adult');
    });
  });

  describe('scaffoldUserType', () => {
    it('should create README and TODO files', () => {
      const result = scaffoldUserType(tempDir, 'value_scanner', 'adult');

      expect(result.created).toHaveLength(2);
      expect(result.skipped).toHaveLength(0);
      expect(result.errors).toHaveLength(0);

      const userPath = path.join(tempDir, 'value_scanner', 'adult');
      expect(fs.existsSync(path.join(userPath, 'README.md'))).toBe(true);
      expect(fs.existsSync(path.join(userPath, 'TODO.md'))).toBe(true);
    });

    it('should create directory structure', () => {
      scaffoldUserType(tempDir, 'value_scanner', 'adult');

      const userPath = path.join(tempDir, 'value_scanner', 'adult');
      expect(fs.existsSync(userPath)).toBe(true);
    });

    it('should skip existing files', () => {
      // First scaffold
      scaffoldUserType(tempDir, 'value_scanner', 'adult');

      // Second scaffold should skip
      const result = scaffoldUserType(tempDir, 'value_scanner', 'adult');

      expect(result.created).toHaveLength(0);
      expect(result.skipped).toHaveLength(2);
    });

    it('should return error for unknown epic', () => {
      const result = scaffoldUserType(tempDir, 'unknown_epic', 'user');

      expect(result.errors).toHaveLength(1);
      expect(result.errors[0]).toContain('Unknown epic');
    });

    it('should write valid README content', () => {
      scaffoldUserType(tempDir, 'value_scanner', 'caregiver');

      const readmePath = path.join(tempDir, 'value_scanner', 'caregiver', 'README.md');
      const content = fs.readFileSync(readmePath, 'utf-8');

      expect(content).toContain('epic: value_scanner');
      expect(content).toContain('user_type: caregiver');
      expect(content).toContain('Caregiver');
    });
  });

  describe('scaffoldEpic', () => {
    it('should scaffold all users for an epic', () => {
      const result = scaffoldEpic(tempDir, 'public_observer');

      const epicConfig = EPICS['public_observer'];
      const expectedFiles = epicConfig.users.length * 2; // README + TODO per user

      expect(result.created).toHaveLength(expectedFiles);
      expect(result.errors).toHaveLength(0);
    });

    it('should create directories for all users', () => {
      scaffoldEpic(tempDir, 'public_observer');

      const epicConfig = EPICS['public_observer'];
      epicConfig.users.forEach(user => {
        const userPath = path.join(tempDir, 'public_observer', user);
        expect(fs.existsSync(userPath)).toBe(true);
      });
    });

    it('should return error for unknown epic', () => {
      const result = scaffoldEpic(tempDir, 'unknown_epic');

      expect(result.errors).toHaveLength(1);
    });
  });

  describe('scaffoldAll', () => {
    it('should scaffold all epics and users', () => {
      const result = scaffoldAll(tempDir);

      expect(result.created.length).toBeGreaterThan(0);
      expect(result.errors).toHaveLength(0);

      // Verify some known epics exist
      expect(fs.existsSync(path.join(tempDir, 'value_scanner'))).toBe(true);
      expect(fs.existsSync(path.join(tempDir, 'public_observer'))).toBe(true);
      expect(fs.existsSync(path.join(tempDir, 'autonomous_entity'))).toBe(true);
    });

    it('should create all user directories', () => {
      scaffoldAll(tempDir);

      // Check a sampling of users across epics
      expect(fs.existsSync(path.join(tempDir, 'value_scanner', 'adult'))).toBe(true);
      expect(fs.existsSync(path.join(tempDir, 'public_observer', 'citizen'))).toBe(true);
      expect(fs.existsSync(path.join(tempDir, 'autonomous_entity', 'worker'))).toBe(true);
    });

    it('should handle partial failures gracefully', () => {
      // TODO(test-generator): [MEDIUM] Add error handling tests for scaffoldAll
      // Context: scaffoldAll doesn't validate epic/user combinations upfront
      // Story: Robust scaffolding with validation
      // Suggested approach:
      //   1. Add epic configuration validation
      //   2. Pre-validate all paths before creating
      //   3. Return detailed error context (which epic/user failed and why)

      // Current implementation continues on error and collects them
      const result = scaffoldAll(tempDir);

      // Should complete even if individual scaffolds fail
      expect(result).toHaveProperty('created');
      expect(result).toHaveProperty('errors');
    });
  });

  describe('listEpicsAndUsers', () => {
    it('should return all epics with their users', () => {
      const list = listEpicsAndUsers();

      expect(list.length).toBeGreaterThan(0);
      expect(list[0]).toHaveProperty('epic');
      expect(list[0]).toHaveProperty('description');
      expect(list[0]).toHaveProperty('users');
    });

    it('should include value_scanner epic', () => {
      const list = listEpicsAndUsers();
      const valueScanner = list.find(e => e.epic === 'value_scanner');

      expect(valueScanner).toBeDefined();
      expect(valueScanner?.description).toContain('Care Economy');
      expect(valueScanner?.users).toContain('adult');
      expect(valueScanner?.users).toContain('caregiver');
    });

    it('should include public_observer epic', () => {
      const list = listEpicsAndUsers();
      const publicObserver = list.find(e => e.epic === 'public_observer');

      expect(publicObserver).toBeDefined();
      expect(publicObserver?.description).toContain('Civic Democracy');
      expect(publicObserver?.users).toContain('citizen');
    });

    it('should include autonomous_entity epic', () => {
      const list = listEpicsAndUsers();
      const autonomousEntity = list.find(e => e.epic === 'autonomous_entity');

      expect(autonomousEntity).toBeDefined();
      expect(autonomousEntity?.description).toContain('Workplace');
      expect(autonomousEntity?.users).toContain('worker');
    });

    it('should include governance epic', () => {
      const list = listEpicsAndUsers();
      const governance = list.find(e => e.epic === 'governance');

      expect(governance).toBeDefined();
      expect(governance?.description).toContain('AI Governance');
    });

    it('should include social_medium epic', () => {
      const list = listEpicsAndUsers();
      const socialMedium = list.find(e => e.epic === 'social_medium');

      expect(socialMedium).toBeDefined();
      expect(socialMedium?.description).toContain('Digital Communication');
    });
  });

  describe('getTotalUserCount', () => {
    it('should return total count of all users across epics', () => {
      const total = getTotalUserCount();

      expect(total).toBeGreaterThan(0);

      // Verify by manually counting
      const manualCount = Object.values(EPICS).reduce(
        (sum, config) => sum + config.users.length,
        0
      );

      expect(total).toBe(manualCount);
    });
  });

  describe('EPICS constant', () => {
    it('should have all required epics', () => {
      expect(EPICS).toHaveProperty('value_scanner');
      expect(EPICS).toHaveProperty('public_observer');
      expect(EPICS).toHaveProperty('autonomous_entity');
      expect(EPICS).toHaveProperty('governance');
      expect(EPICS).toHaveProperty('social_medium');
    });

    it('should have valid structure for each epic', () => {
      Object.entries(EPICS).forEach(([key, config]) => {
        expect(config).toHaveProperty('id');
        expect(config).toHaveProperty('description');
        expect(config).toHaveProperty('users');
        expect(Array.isArray(config.users)).toBe(true);
        expect(config.users.length).toBeGreaterThan(0);
      });
    });
  });
});
