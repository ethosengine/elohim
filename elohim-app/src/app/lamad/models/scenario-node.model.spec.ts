import {
  ScenarioNode,
  ScenarioExamples,
  StepResult
} from './scenario-node.model';
import { NodeType } from './document-node.model';
import { GherkinStep } from './feature-node.model';

describe('ScenarioNode Model', () => {
  describe('ScenarioNode interface', () => {
    it('should create valid scenario node', () => {
      const steps: GherkinStep[] = [
        { keyword: 'Given', text: 'I am on the login page' },
        { keyword: 'When', text: 'I enter valid credentials' },
        { keyword: 'Then', text: 'I should be logged in' }
      ];

      const scenario: ScenarioNode = {
        id: 'scenario-1',
        type: NodeType.SCENARIO,
        title: 'Successful Login',
        description: 'User logs in successfully',
        tags: ['login'],
        sourcePath: '/features/login.feature',
        content: 'Scenario: Successful Login',
        relatedNodeIds: [],
        featureId: 'feature-1',
        epicIds: ['epic-1'],
        scenarioType: 'scenario',
        steps,
        metadata: {}
      };

      expect(scenario.type).toBe(NodeType.SCENARIO);
      expect(scenario.featureId).toBe('feature-1');
      expect(scenario.scenarioType).toBe('scenario');
      expect(scenario.steps.length).toBe(3);
    });

    it('should support scenario outline type', () => {
      const scenario: ScenarioNode = {
        id: 'scenario-1',
        type: NodeType.SCENARIO,
        title: 'Login with different credentials',
        description: 'Parameterized login test',
        tags: [],
        sourcePath: '/features/login.feature',
        content: 'Scenario Outline: Login',
        relatedNodeIds: [],
        featureId: 'feature-1',
        epicIds: [],
        scenarioType: 'scenario_outline',
        steps: [],
        metadata: {}
      };

      expect(scenario.scenarioType).toBe('scenario_outline');
    });

    it('should support example type', () => {
      const scenario: ScenarioNode = {
        id: 'scenario-1',
        type: NodeType.SCENARIO,
        title: 'Example scenario',
        description: 'Example',
        tags: [],
        sourcePath: '/features/test.feature',
        content: 'Example: Test',
        relatedNodeIds: [],
        featureId: 'feature-1',
        epicIds: [],
        scenarioType: 'example',
        steps: [],
        metadata: {}
      };

      expect(scenario.scenarioType).toBe('example');
    });

    it('should support examples for scenario outlines', () => {
      const examples: ScenarioExamples = {
        headers: ['username', 'password'],
        rows: [
          ['user1', 'pass1'],
          ['user2', 'pass2']
        ]
      };

      const scenario: ScenarioNode = {
        id: 'scenario-1',
        type: NodeType.SCENARIO,
        title: 'Parameterized Login',
        description: 'Login with parameters',
        tags: [],
        sourcePath: '/features/login.feature',
        content: 'Scenario Outline',
        relatedNodeIds: [],
        featureId: 'feature-1',
        epicIds: [],
        scenarioType: 'scenario_outline',
        steps: [],
        examples: [examples],
        metadata: {}
      };

      expect(scenario.examples?.length).toBe(1);
      expect(scenario.examples?.[0].headers).toContain('username');
      expect(scenario.examples?.[0].rows.length).toBe(2);
    });

    it('should support test status', () => {
      const scenario: ScenarioNode = {
        id: 'scenario-1',
        type: NodeType.SCENARIO,
        title: 'Tested Scenario',
        description: 'Scenario with test status',
        tags: [],
        sourcePath: '/features/test.feature',
        content: 'Scenario',
        relatedNodeIds: [],
        featureId: 'feature-1',
        epicIds: [],
        scenarioType: 'scenario',
        steps: [],
        testStatus: {
          status: 'passing',
          lastRun: new Date('2025-01-01')
        },
        metadata: {}
      };

      expect(scenario.testStatus?.status).toBe('passing');
      expect(scenario.testStatus?.lastRun).toBeDefined();
    });

    it('should support step results', () => {
      const stepResults: StepResult[] = [
        {
          stepIndex: 0,
          status: 'passed',
          duration: 100
        },
        {
          stepIndex: 1,
          status: 'passed',
          duration: 200
        }
      ];

      const scenario: ScenarioNode = {
        id: 'scenario-1',
        type: NodeType.SCENARIO,
        title: 'Scenario with Results',
        description: 'Executed scenario',
        tags: [],
        sourcePath: '/features/test.feature',
        content: 'Scenario',
        relatedNodeIds: [],
        featureId: 'feature-1',
        epicIds: [],
        scenarioType: 'scenario',
        steps: [],
        stepResults,
        metadata: {}
      };

      expect(scenario.stepResults?.length).toBe(2);
      expect(scenario.stepResults?.[0].status).toBe('passed');
    });
  });

  describe('ScenarioExamples interface', () => {
    it('should create examples with headers and rows', () => {
      const examples: ScenarioExamples = {
        headers: ['name', 'age'],
        rows: [
          ['Alice', '30'],
          ['Bob', '25']
        ]
      };

      expect(examples.headers.length).toBe(2);
      expect(examples.rows.length).toBe(2);
      expect(examples.rows[0]).toEqual(['Alice', '30']);
    });

    it('should support optional name', () => {
      const examples: ScenarioExamples = {
        name: 'Valid Users',
        headers: ['username'],
        rows: [['user1']]
      };

      expect(examples.name).toBe('Valid Users');
    });

    it('should handle empty rows', () => {
      const examples: ScenarioExamples = {
        headers: ['col1', 'col2'],
        rows: []
      };

      expect(examples.rows.length).toBe(0);
    });
  });

  describe('StepResult interface', () => {
    it('should create passed step result', () => {
      const result: StepResult = {
        stepIndex: 0,
        status: 'passed',
        duration: 150
      };

      expect(result.status).toBe('passed');
      expect(result.duration).toBe(150);
    });

    it('should create failed step result with error', () => {
      const result: StepResult = {
        stepIndex: 1,
        status: 'failed',
        duration: 100,
        errorMessage: 'Element not found',
        screenshotPath: '/screenshots/error.png'
      };

      expect(result.status).toBe('failed');
      expect(result.errorMessage).toBe('Element not found');
      expect(result.screenshotPath).toBe('/screenshots/error.png');
    });

    it('should support all step statuses', () => {
      const statuses: Array<StepResult['status']> = [
        'passed',
        'failed',
        'pending',
        'skipped'
      ];

      statuses.forEach(status => {
        const result: StepResult = {
          stepIndex: 0,
          status
        };

        expect(result.status).toBe(status);
      });
    });

    it('should track step index', () => {
      const results: StepResult[] = [
        { stepIndex: 0, status: 'passed' },
        { stepIndex: 1, status: 'passed' },
        { stepIndex: 2, status: 'failed' }
      ];

      expect(results[0].stepIndex).toBe(0);
      expect(results[1].stepIndex).toBe(1);
      expect(results[2].stepIndex).toBe(2);
    });

    it('should include screenshot for visual tests', () => {
      const result: StepResult = {
        stepIndex: 0,
        status: 'failed',
        screenshotPath: '/cypress/screenshots/login-failed.png'
      };

      expect(result.screenshotPath).toContain('login-failed.png');
    });
  });
});
