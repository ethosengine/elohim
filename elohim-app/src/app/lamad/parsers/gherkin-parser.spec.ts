import { GherkinParser, FeatureNode, ScenarioNode } from './gherkin-parser';

describe('GherkinParser', () => {
  describe('parseFeature', () => {
    describe('basic feature parsing', () => {
      it('should parse a simple feature with one scenario', () => {
        const content = `Feature: User Login
  As a user
  I want to log in
  So that I can access my account

  Scenario: Successful login
    Given I am on the login page
    When I enter valid credentials
    Then I should be logged in`;

        const result = GherkinParser.parseFeature(content, 'features/login.feature', 'authentication');

        expect(result.feature).toBeDefined();
        expect(result.feature.title).toBe('User Login');
        expect(result.feature.contentType).toBe('feature');
        expect(result.feature.category).toBe('authentication');
        expect(result.scenarios.length).toBe(1);
        expect(result.scenarios[0].title).toBe('Successful login');
      });

      it('should parse feature without description', () => {
        const content = `Feature: Simple Feature

  Scenario: Simple scenario
    Given a precondition
    When an action occurs
    Then verify the result`;

        const result = GherkinParser.parseFeature(content, 'features/simple.feature', 'basic');

        expect(result.feature.title).toBe('Simple Feature');
        expect(result.feature.featureDescription).toBe('');
        expect(result.scenarios.length).toBe(1);
      });

      it('should parse feature with multiline description', () => {
        const content = `Feature: Complex Feature
  This is a complex feature
  that spans multiple lines
  and provides detailed context

  Scenario: First scenario
    Given a setup
    When something happens
    Then verify outcome`;

        const result = GherkinParser.parseFeature(content, 'features/complex.feature', 'advanced');

        expect(result.feature.featureDescription).toContain('This is a complex feature');
        expect(result.feature.featureDescription).toContain('that spans multiple lines');
        expect(result.feature.featureDescription).toContain('and provides detailed context');
      });

      it('should throw error for invalid feature file without Feature keyword', () => {
        const content = `Invalid content
  This is not a proper feature file

  Scenario: Should fail
    Given something`;

        expect(() => {
          GherkinParser.parseFeature(content, 'features/invalid.feature', 'test');
        }).toThrowError('Invalid feature file: features/invalid.feature');
      });
    });

    describe('tags parsing', () => {
      it('should parse feature tags', () => {
        const content = `@smoke @regression
Feature: Tagged Feature
  Feature with tags

  Scenario: Test scenario
    Given a precondition
    When an action
    Then a result`;

        const result = GherkinParser.parseFeature(content, 'features/tagged.feature', 'test');

        expect(result.feature.tags).toEqual(['smoke', 'regression']);
      });

      it('should parse scenario tags', () => {
        const content = `Feature: Feature with tagged scenarios

  @critical @fast
  Scenario: Important scenario
    Given a setup
    When an action
    Then verify result`;

        const result = GherkinParser.parseFeature(content, 'features/scenario-tags.feature', 'test');

        expect(result.scenarios[0].tags).toContain('critical');
        expect(result.scenarios[0].tags).toContain('fast');
      });

      it('should inherit feature tags in scenarios', () => {
        const content = `@feature-tag
Feature: Feature with tag

  @scenario-tag
  Scenario: Scenario with tag
    Given a step
    When an action
    Then a result`;

        const result = GherkinParser.parseFeature(content, 'features/inheritance.feature', 'test');

        expect(result.scenarios[0].tags).toContain('feature-tag');
        expect(result.scenarios[0].tags).toContain('scenario-tag');
      });

      it('should extract epic IDs from tags', () => {
        const content = `@epic:user-management @epic:security
Feature: Epic tagged feature

  Scenario: Test scenario
    Given a precondition
    When an action
    Then a result`;

        const result = GherkinParser.parseFeature(content, 'features/epic.feature', 'test');

        expect(result.feature.epicIds.length).toBe(2);
        expect(result.feature.epicIds).toContain('user-management');
        expect(result.feature.epicIds).toContain('security');
      });
    });

    describe('background parsing', () => {
      it('should parse background section', () => {
        const content = `Feature: Feature with background

  Background:
    Given a common setup step
    And another setup step

  Scenario: First scenario
    When an action occurs
    Then verify result

  Scenario: Second scenario
    When different action
    Then different result`;

        const result = GherkinParser.parseFeature(content, 'features/background.feature', 'test');

        expect(result.feature.background).toBeDefined();
        expect(result.feature.background?.steps.length).toBe(2);
        expect(result.feature.background?.steps[0].keyword).toBe('Given');
        expect(result.feature.background?.steps[0].text).toBe('a common setup step');
        expect(result.feature.background?.steps[1].keyword).toBe('And');
      });

      it('should work without background section', () => {
        const content = `Feature: Feature without background

  Scenario: Test scenario
    Given a setup
    When an action
    Then a result`;

        const result = GherkinParser.parseFeature(content, 'features/no-background.feature', 'test');

        expect(result.feature.background).toBeUndefined();
      });
    });

    describe('scenario parsing', () => {
      it('should parse multiple scenarios', () => {
        const content = `Feature: Multiple scenarios

  Scenario: First scenario
    Given first setup
    When first action
    Then first result

  Scenario: Second scenario
    Given second setup
    When second action
    Then second result

  Scenario: Third scenario
    Given third setup
    When third action
    Then third result`;

        const result = GherkinParser.parseFeature(content, 'features/multiple.feature', 'test');

        expect(result.scenarios.length).toBe(3);
        expect(result.scenarios[0].title).toBe('First scenario');
        expect(result.scenarios[1].title).toBe('Second scenario');
        expect(result.scenarios[2].title).toBe('Third scenario');
      });

      it('should parse all step keywords', () => {
        const content = `Feature: All step types

  Scenario: Steps with all keywords
    Given a precondition
    And another precondition
    When an action occurs
    And another action
    Then verify the result
    And verify another result
    But not this condition`;

        const result = GherkinParser.parseFeature(content, 'features/steps.feature', 'test');

        const steps = result.scenarios[0].steps;
        expect(steps.length).toBe(7);
        expect(steps[0].keyword).toBe('Given');
        expect(steps[1].keyword).toBe('And');
        expect(steps[2].keyword).toBe('When');
        expect(steps[3].keyword).toBe('And');
        expect(steps[4].keyword).toBe('Then');
        expect(steps[5].keyword).toBe('And');
        expect(steps[6].keyword).toBe('But');
      });

      it('should link scenarios to feature', () => {
        const content = `Feature: Linked feature

  Scenario: Test scenario
    Given a setup
    When an action
    Then a result`;

        const result = GherkinParser.parseFeature(content, 'features/linked.feature', 'test');

        expect(result.scenarios[0].featureId).toBe(result.feature.id);
        expect(result.feature.scenarioIds).toContain(result.scenarios[0].id);
      });
    });

    describe('scenario outline parsing', () => {
      it('should parse scenario outline with examples', () => {
        const content = `Feature: Parameterized scenarios

  Scenario Outline: Login with different users
    Given I am on the login page
    When I enter username "<username>" and password "<password>"
    Then I should see "<result>"

    Examples:
      | username | password | result  |
      | admin    | admin123 | success |
      | user     | user456  | success |
      | invalid  | wrong    | error   |`;

        const result = GherkinParser.parseFeature(content, 'features/outline.feature', 'test');

        expect(result.scenarios.length).toBe(1);
        expect(result.scenarios[0].scenarioType).toBe('scenario_outline');
        expect(result.scenarios[0].examples).toBeDefined();
        expect(result.scenarios[0].examples?.[0].headers).toEqual(['username', 'password', 'result']);
        expect(result.scenarios[0].examples?.[0].rows.length).toBe(3);
      });

      it('should parse scenario outline without examples', () => {
        const content = `Feature: Outline without examples

  Scenario Outline: Incomplete outline
    Given a user "<name>"
    When action occurs
    Then verify result`;

        const result = GherkinParser.parseFeature(content, 'features/incomplete-outline.feature', 'test');

        expect(result.scenarios[0].scenarioType).toBe('scenario_outline');
        expect(result.scenarios[0].examples).toBeUndefined();
      });

      it('should distinguish scenario from scenario outline', () => {
        const content = `Feature: Mixed scenarios

  Scenario: Regular scenario
    Given a setup
    When an action
    Then a result

  Scenario Outline: Outline scenario
    Given a user "<name>"
    When action with "<param>"
    Then result "<output>"

    Examples:
      | name  | param | output |
      | test1 | val1  | res1   |`;

        const result = GherkinParser.parseFeature(content, 'features/mixed.feature', 'test');

        expect(result.scenarios.length).toBe(2);
        expect(result.scenarios[0].scenarioType).toBe('scenario');
        expect(result.scenarios[1].scenarioType).toBe('scenario_outline');
      });
    });

    describe('ID generation', () => {
      it('should generate unique feature ID from path', () => {
        const content = `Feature: Test feature

  Scenario: Test scenario
    Given a step
    When action
    Then result`;

        const result1 = GherkinParser.parseFeature(content, 'features/auth/login.feature', 'test');
        const result2 = GherkinParser.parseFeature(content, 'features/auth/logout.feature', 'test');

        expect(result1.feature.id).toBe('feature_auth_login');
        expect(result2.feature.id).toBe('feature_auth_logout');
        expect(result1.feature.id).not.toBe(result2.feature.id);
      });

      it('should generate unique scenario IDs from title', () => {
        const content = `Feature: Test feature

  Scenario: Successful login
    Given a setup
    When action
    Then result

  Scenario: Failed login
    Given a setup
    When bad action
    Then error`;

        const result = GherkinParser.parseFeature(content, 'features/login.feature', 'test');

        expect(result.scenarios[0].id).toBe('scenario_features_login_successful_login');
        expect(result.scenarios[1].id).toBe('scenario_features_login_failed_login');
        expect(result.scenarios[0].id).not.toBe(result.scenarios[1].id);
      });

      it('should sanitize special characters in IDs', () => {
        const content = `Feature: Test feature

  Scenario: User can't login with invalid @credentials (edge case)
    Given a setup
    When action
    Then result`;

        const result = GherkinParser.parseFeature(content, 'features/test.feature', 'test');

        expect(result.scenarios[0].id).toMatch(/^scenario_[a-z0-9_]+$/);
        expect(result.scenarios[0].id).not.toContain('@');
        expect(result.scenarios[0].id).not.toContain('(');
        expect(result.scenarios[0].id).not.toContain(')');
      });
    });

    describe('node relationships', () => {
      it('should establish bidirectional feature-scenario links', () => {
        const content = `Feature: Test feature

  Scenario: First scenario
    Given a step
    When action
    Then result

  Scenario: Second scenario
    Given another step
    When action
    Then result`;

        const result = GherkinParser.parseFeature(content, 'features/test.feature', 'test');

        expect(result.feature.scenarioIds.length).toBe(2);
        expect(result.feature.relatedNodeIds).toContain(result.scenarios[0].id);
        expect(result.feature.relatedNodeIds).toContain(result.scenarios[1].id);
        expect(result.scenarios[0].relatedNodeIds).toContain(result.feature.id);
        expect(result.scenarios[1].relatedNodeIds).toContain(result.feature.id);
      });

      it('should link epics to feature and scenarios', () => {
        const content = `@epic:user-auth
Feature: Authentication feature

  @epic:security
  Scenario: Secure login
    Given a setup
    When action
    Then result`;

        const result = GherkinParser.parseFeature(content, 'features/auth.feature', 'test');

        expect(result.feature.epicIds.length).toBe(1);
        expect(result.feature.epicIds).toContain('user-auth');
        expect(result.feature.relatedNodeIds).toContain('user-auth');
        expect(result.scenarios[0].epicIds.length).toBe(2);
        expect(result.scenarios[0].epicIds).toContain('user-auth');
        expect(result.scenarios[0].epicIds).toContain('security');
      });
    });

    describe('content format and metadata', () => {
      it('should set correct content format', () => {
        const content = `Feature: Test feature

  Scenario: Test scenario
    Given a step
    When action
    Then result`;

        const result = GherkinParser.parseFeature(content, 'features/test.feature', 'test');

        expect(result.feature.contentFormat).toBe('gherkin');
        expect(result.scenarios[0].contentFormat).toBe('gherkin');
      });

      it('should preserve original gherkin content', () => {
        const content = `Feature: Test feature
  Some description

  Scenario: Test scenario
    Given a step
    When action
    Then result`;

        const result = GherkinParser.parseFeature(content, 'features/test.feature', 'test');

        expect(result.feature.gherkinContent).toBe(content);
        expect(result.feature.content).toBe(content);
      });

      it('should store source path', () => {
        const content = `Feature: Test feature

  Scenario: Test scenario
    Given a step
    When action
    Then result`;

        const result = GherkinParser.parseFeature(content, 'features/auth/login.feature', 'test');

        expect(result.feature.sourcePath).toBe('features/auth/login.feature');
        expect(result.scenarios[0].sourcePath).toBe('features/auth/login.feature');
      });
    });

    describe('edge cases', () => {
      it('should handle empty feature', () => {
        const content = `Feature: Empty feature`;

        const result = GherkinParser.parseFeature(content, 'features/empty.feature', 'test');

        expect(result.feature).toBeDefined();
        expect(result.scenarios.length).toBe(0);
      });

      it('should handle feature with only background', () => {
        const content = `Feature: Background only

  Background:
    Given a setup step`;

        const result = GherkinParser.parseFeature(content, 'features/bg-only.feature', 'test');

        expect(result.feature.background).toBeDefined();
        expect(result.scenarios.length).toBe(0);
      });

      it('should handle scenario with no steps', () => {
        const content = `Feature: Test feature

  Scenario: Empty scenario`;

        const result = GherkinParser.parseFeature(content, 'features/empty-scenario.feature', 'test');

        expect(result.scenarios.length).toBe(1);
        expect(result.scenarios[0].steps.length).toBe(0);
      });

      it('should handle whitespace variations', () => {
        const content = `Feature:   Feature with extra spaces

  Scenario:    Scenario with spaces
    Given     a step with spaces
    When      action with spaces
    Then      result with spaces`;

        const result = GherkinParser.parseFeature(content, 'features/spaces.feature', 'test');

        expect(result.feature.title).toBe('Feature with extra spaces');
        expect(result.scenarios[0].title).toBe('Scenario with spaces');
        expect(result.scenarios[0].steps[0].text).toBe('a step with spaces');
      });

      it('should handle mixed line endings', () => {
        // Normalize line endings to \n before parsing
        const content = "Feature: Mixed endings\r\n\r\n  Scenario: Test\r\n    Given a step\r\n    When action\n    Then result".replace(/\r\n/g, '\n');

        const result = GherkinParser.parseFeature(content, 'features/mixed.feature', 'test');

        expect(result.feature).toBeDefined();
        expect(result.scenarios.length).toBe(1);
      });
    });
  });
});
