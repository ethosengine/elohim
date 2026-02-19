/**
 * Shared types for BDD coverage gap analysis.
 *
 * The scanner compares "conceptual" scenarios (genesis feature files describing
 * human experiences) against "executable" scenarios (Cucumber features that
 * actually run against the system). The gap report drives sprint planning and
 * scenario generation.
 */

export interface ConceptualScenario {
  name: string;
  featureFile: string; // relative from repo root
  epic: string; // @epic: tag value
  userType: string; // @user_type: tag value
  governanceLayer: string; // @governance_layer: tag value
}

export interface ExecutableScenario {
  name: string;
  featureFile: string;
  tags: string[];
  framework: 'cypress' | 'cucumber-js';
}

export interface DomainGap {
  epic: string;
  conceptualCount: number;
  executableCount: number;
  coveragePercent: number;
  userTypes: string[];
  governanceLayers: string[];
  sampleScenarios: string[]; // first 5 names as examples
}

export interface GovernanceGap {
  layer: string;
  conceptualCount: number;
  executableCount: number;
  epics: string[];
}

export interface PrioritizedGap {
  rank: number;
  domain: string;
  governanceLayer: string;
  conceptualDensity: number;
  suggestedFeatureFile: string;
  rationale: string;
}

export interface CoverageGapReport {
  generatedAt: string;
  gitCommit: string;
  summary: {
    totalConceptualScenarios: number;
    totalExecutableScenarios: number;
    coveragePercent: number;
    epicsWithZeroTests: string[];
    governanceLayersWithZeroTests: string[];
  };
  domainGaps: DomainGap[];
  governanceGaps: GovernanceGap[];
  prioritizedGaps: PrioritizedGap[];
}
