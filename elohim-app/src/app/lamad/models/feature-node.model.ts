import { DocumentNode, NodeType } from './document-node.model';

/**
 * Represents a Gherkin feature file
 * Source: cypress/e2e/features/**\/*.feature files
 */
export interface FeatureNode extends DocumentNode {
  type: NodeType.FEATURE;

  /** Category based on directory structure */
  category: string;

  /** Epic IDs this feature implements */
  epicIds: string[];

  /** Scenario IDs within this feature */
  scenarioIds: string[];

  /** Gherkin feature description */
  featureDescription: string;

  /** Feature background (if any) */
  background?: GherkinBackground;

  /** Test status summary */
  testStatus?: TestStatus;

  /** Original Gherkin content */
  gherkinContent: string;
}

export interface GherkinBackground {
  /** Background steps */
  steps: GherkinStep[];
}

export interface GherkinStep {
  /** Step keyword (Given, When, Then, And, But) */
  keyword: string;

  /** Step text */
  text: string;

  /** Data table (if any) */
  dataTable?: string[][];

  /** Doc string (if any) */
  docString?: string;
}

export interface ScenarioExamples {
  /** Header row */
  headers: string[];

  /** Data rows */
  rows: string[][];
}

export interface TestStatus {
  /** Overall status */
  status: 'passing' | 'failing' | 'pending' | 'skipped' | 'unknown';

  /** Number of passing scenarios */
  passed: number;

  /** Number of failing scenarios */
  failed: number;

  /** Number of pending scenarios */
  pending: number;

  /** Number of skipped scenarios */
  skipped: number;

  /** Last test run timestamp */
  lastRun?: Date;

  /** Link to test report */
  reportUrl?: string;
}
