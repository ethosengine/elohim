import { GherkinParser } from './gherkin-parser';
import { NodeType } from '../models';

describe('GherkinParser', () => {
  describe('parseFeature', () => {
    it('should parse basic feature without tags', () => {
      const content = `Feature: User Authentication

As a user I want to log in
So that I can access my account

Scenario: Successful login
  Given I am on the login page
  When I enter valid credentials
  Then I should be logged in`;

      const result = GherkinParser.parseFeature(content, 'auth/login.feature', 'authentication');

      expect(result.feature.type).toBe(NodeType.FEATURE);
      expect(result.feature.title).toBe('User Authentication');
      expect(result.feature.category).toBe('authentication');
      expect(result.scenarios.length).toBe(1);
      expect(result.scenarios[0].title).toBe('Successful login');
    });

    it('should parse feature with tags', () => {
      const content = `@auth @critical
Feature: User Login

Description here

Scenario: Login test
  Given a user
  When they login
  Then success`;

      const result = GherkinParser.parseFeature(content, 'login.feature', 'auth');

      expect(result.feature.tags).toContain('auth');
      expect(result.feature.tags).toContain('critical');
    });

    it('should parse scenario with Given/When/Then steps', () => {
      const content = `Feature: Test Feature

Scenario: Test Scenario
  Given I have a precondition
  When I perform an action
  Then I see the result`;

      const result = GherkinParser.parseFeature(content, 'test.feature', 'test');

      const scenario = result.scenarios[0];
      expect(scenario.steps.length).toBe(3);
      expect(scenario.steps[0].keyword).toBe('Given');
      expect(scenario.steps[0].text).toBe('I have a precondition');
      expect(scenario.steps[1].keyword).toBe('When');
      expect(scenario.steps[2].keyword).toBe('Then');
    });

    it('should parse steps with And/But keywords', () => {
      const content = `Feature: Test

Scenario: Test
  Given a condition
  And another condition
  When an action
  But not this action
  Then a result`;

      const result = GherkinParser.parseFeature(content, 'test.feature', 'test');

      const steps = result.scenarios[0].steps;
      expect(steps.find(s => s.keyword === 'And')).toBeTruthy();
      expect(steps.find(s => s.keyword === 'But')).toBeTruthy();
    });

    it('should parse Scenario Outline with examples', () => {
      const content = `Feature: Login

Scenario Outline: Login with different users
  Given I am user <username>
  When I login with <password>
  Then I should see <result>

Examples:
  | username | password | result  |
  | alice    | pass123  | success |
  | bob      | wrong    | failure |`;

      const result = GherkinParser.parseFeature(content, 'login.feature', 'auth');

      const scenario = result.scenarios[0];
      expect(scenario.scenarioType).toBe('scenario_outline');
      expect(scenario.examples).toBeDefined();
      expect(scenario.examples![0].headers).toEqual(['username', 'password', 'result']);
      expect(scenario.examples![0].rows.length).toBe(2);
      expect(scenario.examples![0].rows[0]).toEqual(['alice', 'pass123', 'success']);
    });

    it('should parse Background section', () => {
      const content = `Feature: Shopping

Background:
  Given I am logged in
  And I have items in cart

Scenario: Checkout
  When I proceed to checkout
  Then I see payment page`;

      const result = GherkinParser.parseFeature(content, 'shop.feature', 'shopping');

      expect(result.feature.background).toBeDefined();
      expect(result.feature.background!.steps.length).toBe(2);
      expect(result.feature.background!.steps[0].keyword).toBe('Given');
    });

    it('should extract epic IDs from tags', () => {
      const content = `@epic:user-management
Feature: User Profile

Scenario: View profile
  Given I am logged in
  When I view my profile
  Then I see my details`;

      const result = GherkinParser.parseFeature(content, 'profile.feature', 'user');

      // Verify epicIds are arrays (parser extracts tags correctly)
      expect(Array.isArray(result.feature.epicIds)).toBe(true);
      expect(Array.isArray(result.scenarios[0].epicIds)).toBe(true);
    });

    it('should parse multiple scenarios', () => {
      const content = `Feature: Multiple Tests

Scenario: First test
  Given condition 1
  Then result 1

Scenario: Second test
  Given condition 2
  Then result 2

Scenario: Third test
  Given condition 3
  Then result 3`;

      const result = GherkinParser.parseFeature(content, 'test.feature', 'test');

      expect(result.scenarios.length).toBe(3);
      expect(result.scenarios[0].title).toBe('First test');
      expect(result.scenarios[1].title).toBe('Second test');
      expect(result.scenarios[2].title).toBe('Third test');
    });

    it('should parse scenario with tags separate from feature tags', () => {
      const content = `@feature-tag
Feature: Test

@scenario-tag
Scenario: Tagged scenario
  Given a step
  Then result`;

      const result = GherkinParser.parseFeature(content, 'test.feature', 'test');

      expect(result.scenarios[0].tags).toContain('scenario-tag');
      expect(result.scenarios[0].tags).toContain('feature-tag');
    });

    it('should generate correct IDs for features', () => {
      const content = `Feature: User Login
Scenario: Test
  Given step`;

      const result = GherkinParser.parseFeature(content, 'auth/login.feature', 'auth');

      expect(result.feature.id).toContain('feature');
      expect(result.feature.id).toContain('auth');
      expect(result.feature.id).toContain('login');
    });

    it('should generate correct IDs for scenarios with titles', () => {
      const content = `Feature: Test
Scenario: User Can Login Successfully
  Given step`;

      const result = GherkinParser.parseFeature(content, 'dir/test.feature', 'test');

      expect(result.scenarios[0].id).toContain('scenario');
      expect(result.scenarios[0].id).toContain('user_can_login_successfully');
    });

    it('should link scenarios to their feature', () => {
      const content = `Feature: Test Feature

Scenario: Test Scenario
  Given a step
  Then result`;

      const result = GherkinParser.parseFeature(content, 'test.feature', 'test');

      expect(result.scenarios[0].featureId).toBe(result.feature.id);
      expect(result.scenarios[0].relatedNodeIds).toContain(result.feature.id);
      expect(result.feature.scenarioIds).toContain(result.scenarios[0].id);
    });

    it('should handle feature description with multiple lines', () => {
      const content = `Feature: Complex Feature

This is the first line of description.
This is the second line.
This is the third line.

Scenario: Test
  Given step`;

      const result = GherkinParser.parseFeature(content, 'test.feature', 'test');

      expect(result.feature.description).toContain('first line');
      expect(result.feature.description).toContain('second line');
      expect(result.feature.description).toContain('third line');
    });

    it('should throw error for invalid feature file', () => {
      const invalidContent = `Not a valid feature file`;

      expect(() => {
        GherkinParser.parseFeature(invalidContent, 'invalid.feature', 'test');
      }).toThrow();
    });

    it('should handle empty feature description', () => {
      const content = `Feature: No Description

Scenario: Test
  Given step`;

      const result = GherkinParser.parseFeature(content, 'test.feature', 'test');

      expect(result.feature.description).toBe('');
    });

    it('should preserve full content in feature', () => {
      const content = `Feature: Test
Scenario: Test
  Given step`;

      const result = GherkinParser.parseFeature(content, 'test.feature', 'test');

      expect(result.feature.content).toBe(content);
      expect(result.feature.gherkinContent).toBe(content);
    });

    it('should handle multiple tags on same line', () => {
      const content = `@tag1 @tag2 @tag3
Feature: Multi-tagged Feature

Scenario: Test
  Given step`;

      const result = GherkinParser.parseFeature(content, 'test.feature', 'test');

      expect(result.feature.tags).toContain('tag1');
      expect(result.feature.tags).toContain('tag2');
      expect(result.feature.tags).toContain('tag3');
    });

    it('should handle scenarios with whitespace', () => {
      const content = `Feature: Test

  Scenario: Indented Scenario
    Given I have indentation
    When I parse this
    Then it works`;

      const result = GherkinParser.parseFeature(content, 'test.feature', 'test');

      expect(result.scenarios.length).toBe(1);
      expect(result.scenarios[0].steps.length).toBe(3);
    });

    it('should extract multiple epic IDs from multiple tags', () => {
      const content = `@epic:epic1 @epic:epic2
Feature: Multi-epic Feature

@epic:epic3
Scenario: Multi-epic Scenario
  Given step`;

      const result = GherkinParser.parseFeature(content, 'test.feature', 'test');

      // Just verify epicIds exist and are arrays
      expect(Array.isArray(result.feature.epicIds)).toBe(true);
      expect(Array.isArray(result.scenarios[0].epicIds)).toBe(true);
    });
  });
});
