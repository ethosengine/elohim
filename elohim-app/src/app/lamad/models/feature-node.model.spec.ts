import {
  FeatureNode,
  GherkinBackground,
  GherkinStep,
  TestStatus
} from './feature-node.model';
import { NodeType } from './document-node.model';

describe('FeatureNode Model', () => {
  describe('FeatureNode interface', () => {
    it('should create valid feature node', () => {
      const feature: FeatureNode = {
        id: 'feature-1',
        type: NodeType.FEATURE,
        title: 'User Login',
        description: 'Feature for user authentication',
        tags: ['authentication'],
        sourcePath: '/features/login.feature',
        content: 'Feature: User Login',
        relatedNodeIds: [],
        category: 'authentication',
        epicIds: ['epic-1'],
        scenarioIds: ['scenario-1', 'scenario-2'],
        featureDescription: 'As a user, I want to log in',
        gherkinContent: 'Feature: User Login\n\nScenario: Successful login',
        metadata: {}
      };

      expect(feature.type).toBe(NodeType.FEATURE);
      expect(feature.category).toBe('authentication');
      expect(feature.epicIds).toContain('epic-1');
      expect(feature.scenarioIds).toHaveLength(2);
    });

    it('should support optional background', () => {
      const background: GherkinBackground = {
        steps: [
          {
            keyword: 'Given',
            text: 'the application is running'
          }
        ]
      };

      const feature: FeatureNode = {
        id: 'feature-1',
        type: NodeType.FEATURE,
        title: 'Feature with Background',
        description: 'Description',
        tags: [],
        sourcePath: '/features/test.feature',
        content: 'Content',
        relatedNodeIds: [],
        category: 'test',
        epicIds: [],
        scenarioIds: [],
        featureDescription: 'Feature description',
        background,
        gherkinContent: 'Gherkin content',
        metadata: {}
      };

      expect(feature.background).toBeDefined();
      expect(feature.background?.steps).toHaveLength(1);
      expect(feature.background?.steps[0].keyword).toBe('Given');
    });

    it('should support optional test status', () => {
      const testStatus: TestStatus = {
        status: 'passing',
        passed: 5,
        failed: 0,
        pending: 0,
        skipped: 0,
        lastRun: new Date('2025-01-01')
      };

      const feature: FeatureNode = {
        id: 'feature-1',
        type: NodeType.FEATURE,
        title: 'Tested Feature',
        description: 'Feature with test status',
        tags: [],
        sourcePath: '/features/test.feature',
        content: 'Content',
        relatedNodeIds: [],
        category: 'test',
        epicIds: [],
        scenarioIds: [],
        featureDescription: 'Description',
        testStatus,
        gherkinContent: 'Gherkin',
        metadata: {}
      };

      expect(feature.testStatus?.status).toBe('passing');
      expect(feature.testStatus?.passed).toBe(5);
      expect(feature.testStatus?.failed).toBe(0);
    });
  });

  describe('GherkinBackground interface', () => {
    it('should create background with steps', () => {
      const background: GherkinBackground = {
        steps: [
          { keyword: 'Given', text: 'I am on the homepage' },
          { keyword: 'And', text: 'I am logged in' }
        ]
      };

      expect(background.steps).toHaveLength(2);
      expect(background.steps[0].keyword).toBe('Given');
      expect(background.steps[1].keyword).toBe('And');
    });
  });

  describe('GherkinStep interface', () => {
    it('should create basic step', () => {
      const step: GherkinStep = {
        keyword: 'Given',
        text: 'I am on the login page'
      };

      expect(step.keyword).toBe('Given');
      expect(step.text).toBe('I am on the login page');
    });

    it('should support data table', () => {
      const step: GherkinStep = {
        keyword: 'Given',
        text: 'the following users exist',
        dataTable: [
          ['username', 'password'],
          ['user1', 'pass1'],
          ['user2', 'pass2']
        ]
      };

      expect(step.dataTable).toHaveLength(3);
      expect(step.dataTable?.[0]).toEqual(['username', 'password']);
      expect(step.dataTable?.[1]).toEqual(['user1', 'pass1']);
    });

    it('should support doc string', () => {
      const step: GherkinStep = {
        keyword: 'Given',
        text: 'the following JSON payload',
        docString: '{\n  "key": "value"\n}'
      };

      expect(step.docString).toContain('key');
      expect(step.docString).toContain('value');
    });

    it('should support all Gherkin keywords', () => {
      const keywords = ['Given', 'When', 'Then', 'And', 'But'];

      keywords.forEach(keyword => {
        const step: GherkinStep = {
          keyword,
          text: `step with ${keyword}`
        };

        expect(step.keyword).toBe(keyword);
      });
    });
  });

  describe('TestStatus interface', () => {
    it('should track passing status', () => {
      const status: TestStatus = {
        status: 'passing',
        passed: 10,
        failed: 0,
        pending: 0,
        skipped: 0
      };

      expect(status.status).toBe('passing');
      expect(status.passed).toBe(10);
      expect(status.failed).toBe(0);
    });

    it('should track failing status', () => {
      const status: TestStatus = {
        status: 'failing',
        passed: 5,
        failed: 3,
        pending: 1,
        skipped: 1
      };

      expect(status.status).toBe('failing');
      expect(status.failed).toBe(3);
    });

    it('should support all status types', () => {
      const statuses: Array<TestStatus['status']> = [
        'passing',
        'failing',
        'pending',
        'skipped',
        'unknown'
      ];

      statuses.forEach(statusType => {
        const status: TestStatus = {
          status: statusType,
          passed: 0,
          failed: 0,
          pending: 0,
          skipped: 0
        };

        expect(status.status).toBe(statusType);
      });
    });

    it('should include optional last run timestamp', () => {
      const lastRun = new Date('2025-01-15T10:30:00Z');
      const status: TestStatus = {
        status: 'passing',
        passed: 5,
        failed: 0,
        pending: 0,
        skipped: 0,
        lastRun
      };

      expect(status.lastRun).toEqual(lastRun);
    });

    it('should include optional report URL', () => {
      const status: TestStatus = {
        status: 'passing',
        passed: 5,
        failed: 0,
        pending: 0,
        skipped: 0,
        reportUrl: 'https://example.com/reports/123'
      };

      expect(status.reportUrl).toBe('https://example.com/reports/123');
    });
  });
});
