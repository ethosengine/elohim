import { DocumentNode, NodeType } from './document-node.model';
import { GherkinStep, TestStatus } from './feature-node.model';

/**
 * Represents an individual Gherkin scenario
 * Extracted from feature files
 */
export interface ScenarioNode extends DocumentNode {
  type: NodeType.SCENARIO;

  /** Parent feature ID */
  featureId: string;

  /** Epic IDs this scenario relates to */
  epicIds: string[];

  /** Scenario type */
  scenarioType: 'scenario' | 'scenario_outline' | 'example';

  /** Gherkin steps */
  steps: GherkinStep[];

  /** Examples table for scenario outlines */
  examples?: ScenarioExamples[];

  /** Individual test status */
  testStatus?: Pick<TestStatus, 'status' | 'lastRun'>;

  /** Step execution details (from test reports) */
  stepResults?: StepResult[];
}

export interface ScenarioExamples {
  /** Example set name */
  name?: string;

  /** Table headers */
  headers: string[];

  /** Table rows */
  rows: string[][];
}

export interface StepResult {
  /** Step index */
  stepIndex: number;

  /** Execution status */
  status: 'passed' | 'failed' | 'pending' | 'skipped';

  /** Execution duration (ms) */
  duration?: number;

  /** Error message (if failed) */
  errorMessage?: string;

  /** Screenshot path (if applicable) */
  screenshotPath?: string;
}
