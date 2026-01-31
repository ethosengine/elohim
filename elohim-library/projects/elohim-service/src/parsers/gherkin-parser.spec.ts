/**
 * Tests for gherkin-parser
 */

import {
  parseGherkin,
  extractGherkinDescription,
  extractGherkinTags,
  GherkinTag
} from './gherkin-parser';
import { PathMetadata } from '../models/path-metadata.model';

describe('gherkin-parser', () => {
  const mockPathMeta: PathMetadata = {
    fullPath: '/test/file.feature',
    relativePath: 'test/file.feature',
    domain: 'elohim-protocol',
    epic: 'governance',
    userType: 'policy_maker',
    contentCategory: 'scenario',
    baseName: 'funding',
    extension: '.feature',
    isArchetypeDefinition: false,
    isEpicNarrative: false,
    isScenario: true,
    isResource: false,
    suggestedId: 'scenario-governance-policy-maker-funding'
  };

  describe('parseGherkin', () => {
    describe('Feature parsing', () => {
      it('should parse basic feature', () => {
        const content = `Feature: Funding Allocation

  As a policy maker
  I want to allocate funds
  So that resources are distributed fairly

Scenario: Basic allocation
  Given a budget of $1000
  When I allocate $500 to program A
  Then program A should receive $500`;

        const parsed = parseGherkin(content, mockPathMeta);

        expect(parsed.title).toBe('Funding Allocation');
        expect(parsed.scenarios).toHaveLength(1);
      });

      it('should parse feature with tags', () => {
        const content = `@governance @policy
Feature: Policy Creation

Scenario: Create policy
  Given I am a policy maker
  When I create a policy
  Then the policy is saved`;

        const parsed = parseGherkin(content, mockPathMeta);

        expect(parsed.frontmatter.tags).toContain('governance');
        expect(parsed.frontmatter.tags).toContain('policy');
      });

      it('should parse tags with values', () => {
        const content = `@epic:governance @user:policy_maker
Feature: Test Feature

Scenario: Test
  Given a condition
  Then verify`;

        const parsed = parseGherkin(content, mockPathMeta);

        expect(parsed.frontmatter.epic).toBe('governance');
        expect(parsed.frontmatter.user).toBe('policy_maker');
      });

      it('should extract feature description', () => {
        const content = `Feature: Test Feature
  This is a description
  that spans multiple lines
  and provides context

Scenario: Test
  Given something
  Then verify`;

        const parsed = parseGherkin(content, mockPathMeta);

        const description = extractGherkinDescription(parsed);
        expect(description).toBeDefined();
      });

      it('should handle feature without description', () => {
        const content = `Feature: Simple Feature

Scenario: Test
  Given condition
  Then result`;

        const parsed = parseGherkin(content, mockPathMeta);

        expect(parsed.title).toBe('Simple Feature');
        expect(parsed.scenarios).toHaveLength(1);
      });
    });

    describe('Scenario parsing', () => {
      it('should parse multiple scenarios', () => {
        const content = `Feature: Multiple Scenarios

Scenario: First scenario
  Given condition 1
  When action 1
  Then result 1

Scenario: Second scenario
  Given condition 2
  When action 2
  Then result 2`;

        const parsed = parseGherkin(content, mockPathMeta);

        expect(parsed.scenarios).toHaveLength(2);
        expect(parsed.scenarios![0].title).toBe('First scenario');
        expect(parsed.scenarios![1].title).toBe('Second scenario');
      });

      it('should parse scenario with tags', () => {
        const content = `Feature: Test

@critical @fast
Scenario: Important test
  Given a critical condition
  Then verify quickly`;

        const parsed = parseGherkin(content, mockPathMeta);

        expect(parsed.scenarios![0].tags).toContain('critical');
        expect(parsed.scenarios![0].tags).toContain('fast');
      });

      it('should parse Scenario Outline', () => {
        const content = `Feature: Data-driven test

Scenario Outline: Multiple inputs
  Given input is <input>
  When I process it
  Then output is <output>

Examples:
  | input | output |
  | a     | A      |
  | b     | B      |`;

        const parsed = parseGherkin(content, mockPathMeta);

        expect(parsed.scenarios).toHaveLength(1);
        expect(parsed.scenarios![0].type).toBe('scenario_outline');
      });

      it('should parse all step types', () => {
        const content = `Feature: Step types

Scenario: All steps
  Given a precondition
  And another precondition
  When I perform an action
  Then I see a result
  But not this result
  And also this result`;

        const parsed = parseGherkin(content, mockPathMeta);

        const scenario = parsed.scenarios![0];
        expect(scenario.steps).toHaveLength(6);
        expect(scenario.steps.map(s => s.keyword)).toContain('Given');
        expect(scenario.steps.map(s => s.keyword)).toContain('And');
        expect(scenario.steps.map(s => s.keyword)).toContain('When');
        expect(scenario.steps.map(s => s.keyword)).toContain('Then');
        expect(scenario.steps.map(s => s.keyword)).toContain('But');
      });

      it('should parse step text', () => {
        const content = `Feature: Step text

Scenario: Test
  Given I have "quoted text" in my step
  When I use numbers like 123
  Then I can use special characters: @#$`;

        const parsed = parseGherkin(content, mockPathMeta);

        const steps = parsed.scenarios![0].steps;
        expect(steps[0].text).toContain('quoted text');
        expect(steps[1].text).toContain('123');
        expect(steps[2].text).toContain('@#$');
      });
    });

    describe('Background parsing', () => {
      it('should parse background section', () => {
        const content = `Feature: With background

Background:
  Given a shared precondition
  And another shared precondition

Scenario: First
  When action 1
  Then result 1

Scenario: Second
  When action 2
  Then result 2`;

        const parsed = parseGherkin(content, mockPathMeta);

        expect(parsed.scenarios).toHaveLength(2);
        // Background is parsed but stored in feature structure
      });

      it('should handle feature without background', () => {
        const content = `Feature: No background

Scenario: Test
  Given condition
  Then result`;

        const parsed = parseGherkin(content, mockPathMeta);

        expect(parsed.scenarios).toHaveLength(1);
      });
    });

    describe('Tag parsing', () => {
      it('should parse simple tags', () => {
        const content = `@tag1 @tag2 @tag3
Feature: Tagged feature

Scenario: Test
  Given something
  Then verify`;

        const parsed = parseGherkin(content, mockPathMeta);

        expect(parsed.frontmatter.tags).toContain('tag1');
        expect(parsed.frontmatter.tags).toContain('tag2');
        expect(parsed.frontmatter.tags).toContain('tag3');
      });

      it('should parse tags with values', () => {
        const content = `@priority:high @category:security
Feature: Important feature

Scenario: Test
  Given something
  Then verify`;

        const parsed = parseGherkin(content, mockPathMeta);

        expect(parsed.frontmatter.priority).toBe('high');
        expect(parsed.frontmatter.category).toBe('security');
      });

      it('should parse mixed tags', () => {
        const content = `@simple @key:value @another
Feature: Mixed tags

Scenario: Test
  Given something
  Then verify`;

        const parsed = parseGherkin(content, mockPathMeta);

        expect(parsed.frontmatter.tags).toContain('simple');
        expect(parsed.frontmatter.tags).toContain('another');
        expect(parsed.frontmatter.key).toBe('value');
      });

      it('should handle tags with hyphens and underscores', () => {
        const content = `@multi-word-tag @under_score_tag @key:value-with-hyphen
Feature: Complex tags

Scenario: Test
  Given something
  Then verify`;

        const parsed = parseGherkin(content, mockPathMeta);

        expect(parsed.frontmatter.tags).toContain('multi-word-tag');
        expect(parsed.frontmatter.tags).toContain('under_score_tag');
        expect(parsed.frontmatter.key).toBe('value-with-hyphen');
      });

      it('should parse scenario-level tags', () => {
        const content = `Feature: Test

@scenario-tag
Scenario: Tagged scenario
  Given condition
  Then result`;

        const parsed = parseGherkin(content, mockPathMeta);

        expect(parsed.scenarios![0].tags).toContain('scenario-tag');
      });

      it('should combine tag name and value for scenario tags', () => {
        const content = `Feature: Test

@priority:high
Scenario: Important scenario
  Given condition
  Then result`;

        const parsed = parseGherkin(content, mockPathMeta);

        expect(parsed.scenarios![0].tags).toContain('priority:high');
      });
    });

    describe('Content hash generation', () => {
      it('should generate SHA256 hash', () => {
        const content = `Feature: Test

Scenario: Test
  Given something
  Then verify`;

        const parsed = parseGherkin(content, mockPathMeta);

        expect(parsed.contentHash).toBeDefined();
        expect(parsed.contentHash).toHaveLength(64);
      });

      it('should generate different hashes for different content', () => {
        const content1 = `Feature: Test 1

Scenario: Test
  Given something
  Then verify`;

        const content2 = `Feature: Test 2

Scenario: Test
  Given something else
  Then verify`;

        const parsed1 = parseGherkin(content1, mockPathMeta);
        const parsed2 = parseGherkin(content2, mockPathMeta);

        expect(parsed1.contentHash).not.toBe(parsed2.contentHash);
      });
    });

    describe('PathMeta integration', () => {
      it('should include pathMeta in result', () => {
        const content = `Feature: Test

Scenario: Test
  Given condition
  Then result`;

        const parsed = parseGherkin(content, mockPathMeta);

        expect(parsed.pathMeta).toBe(mockPathMeta);
      });

      it('should include raw content', () => {
        const content = `Feature: Test

Scenario: Test
  Given condition
  Then result`;

        const parsed = parseGherkin(content, mockPathMeta);

        expect(parsed.rawContent).toBe(content);
      });
    });
  });

  describe('extractGherkinDescription', () => {
    it('should use frontmatter description if available', () => {
      const content = `@description:Test_description
Feature: Test

Scenario: Test
  Given condition
  Then result`;

      const parsed = parseGherkin(content, mockPathMeta);
      const description = extractGherkinDescription(parsed);

      expect(description).toBe('Test_description');
    });

    it('should generate description from scenarios', () => {
      const content = `Feature: Test

Scenario: First scenario
  Given condition
  Then result

Scenario: Second scenario
  Given condition
  Then result`;

      const parsed = parseGherkin(content, mockPathMeta);
      const description = extractGherkinDescription(parsed);

      expect(description).toContain('First scenario');
      expect(description).toContain('Second scenario');
    });

    it('should limit scenario list to first 3', () => {
      const content = `Feature: Test

Scenario: Scenario 1
  Given condition
  Then result

Scenario: Scenario 2
  Given condition
  Then result

Scenario: Scenario 3
  Given condition
  Then result

Scenario: Scenario 4
  Given condition
  Then result

Scenario: Scenario 5
  Given condition
  Then result`;

      const parsed = parseGherkin(content, mockPathMeta);
      const description = extractGherkinDescription(parsed);

      expect(description).toContain('5 scenarios');
      expect(description).toContain('Scenario 1');
      expect(description).toContain('Scenario 2');
      expect(description).toContain('Scenario 3');
      expect(description).not.toContain('Scenario 4');
    });

    it('should use feature title as fallback', () => {
      const content = `Feature: Fallback Title`;

      const parsed = parseGherkin(content, mockPathMeta);
      const description = extractGherkinDescription(parsed);

      expect(description).toBe('Feature: Fallback Title');
    });
  });

  describe('extractGherkinTags', () => {
    it('should extract tags from frontmatter', () => {
      const content = `@tag1 @tag2
Feature: Test

Scenario: Test
  Given condition
  Then result`;

      const parsed = parseGherkin(content, mockPathMeta);
      const tags = extractGherkinTags(parsed);

      expect(tags).toContain('tag1');
      expect(tags).toContain('tag2');
    });

    it('should extract key-value tags', () => {
      const content = `@epic:governance @priority:high
Feature: Test

Scenario: Test
  Given condition
  Then result`;

      const parsed = parseGherkin(content, mockPathMeta);
      const tags = extractGherkinTags(parsed);

      expect(tags).toContain('epic');
      expect(tags).toContain('governance');
      expect(tags).toContain('priority');
      expect(tags).toContain('high');
    });

    it('should extract tags from path metadata', () => {
      const content = `Feature: Test

Scenario: Test
  Given condition
  Then result`;

      const pathMeta: PathMetadata = {
        ...mockPathMeta,
        epic: 'lamad',
        userType: 'learner'
      };

      const parsed = parseGherkin(content, pathMeta);
      const tags = extractGherkinTags(parsed);

      expect(tags).toContain('lamad');
      expect(tags).toContain('learner');
    });

    it('should always include scenario tag', () => {
      const content = `Feature: Test

Scenario: Test
  Given condition
  Then result`;

      const parsed = parseGherkin(content, mockPathMeta);
      const tags = extractGherkinTags(parsed);

      expect(tags).toContain('scenario');
    });

    it('should lowercase all tags', () => {
      const content = `@UPPERCASE @MixedCase
Feature: Test

Scenario: Test
  Given condition
  Then result`;

      const parsed = parseGherkin(content, mockPathMeta);
      const tags = extractGherkinTags(parsed);

      expect(tags).toContain('uppercase');
      expect(tags).toContain('mixedcase');
    });

    it('should convert underscores to hyphens in user type', () => {
      const pathMeta: PathMetadata = {
        ...mockPathMeta,
        userType: 'policy_maker'
      };

      const content = `Feature: Test

Scenario: Test
  Given condition
  Then result`;

      const parsed = parseGherkin(content, pathMeta);
      const tags = extractGherkinTags(parsed);

      expect(tags).toContain('policy-maker');
    });

    it('should not duplicate tags', () => {
      const content = `@governance
Feature: Test

Scenario: Test
  Given condition
  Then result`;

      const pathMeta: PathMetadata = {
        ...mockPathMeta,
        epic: 'governance'
      };

      const parsed = parseGherkin(content, pathMeta);
      const tags = extractGherkinTags(parsed);

      const governanceCount = tags.filter(t => t === 'governance').length;
      expect(governanceCount).toBe(1);
    });

    it('should skip "other" epic category', () => {
      const pathMeta: PathMetadata = {
        ...mockPathMeta,
        epic: 'other'
      };

      const content = `Feature: Test

Scenario: Test
  Given condition
  Then result`;

      const parsed = parseGherkin(content, pathMeta);
      const tags = extractGherkinTags(parsed);

      expect(tags).not.toContain('other');
    });
  });

  describe('Edge cases', () => {
    it('should handle empty content', () => {
      const content = '';
      const parsed = parseGherkin(content, mockPathMeta);

      expect(parsed.frontmatter).toBeDefined();
      expect(parsed.scenarios).toBeDefined();
    });

    it('should handle feature without scenarios', () => {
      const content = `Feature: Empty feature

This feature has no scenarios yet.`;

      const parsed = parseGherkin(content, mockPathMeta);

      expect(parsed.scenarios).toHaveLength(0);
    });

    it('should handle scenario without steps', () => {
      const content = `Feature: Test

Scenario: Empty scenario`;

      const parsed = parseGherkin(content, mockPathMeta);

      expect(parsed.scenarios).toHaveLength(1);
      expect(parsed.scenarios![0].steps).toHaveLength(0);
    });

    it('should handle malformed Gherkin gracefully', () => {
      const content = `This is not valid Gherkin
Just some text
Without proper structure`;

      const parsed = parseGherkin(content, mockPathMeta);

      // Should not crash
      expect(parsed).toBeDefined();
    });

    it('should handle multiple blank lines', () => {
      const content = `Feature: Test


Scenario: Test


  Given condition


  Then result`;

      const parsed = parseGherkin(content, mockPathMeta);

      expect(parsed.scenarios).toHaveLength(1);
    });

    it('should handle mixed line endings', () => {
      const content = `Feature: Test\r\n\r\nScenario: Test\r\n  Given condition\r\n  Then result`;

      const parsed = parseGherkin(content, mockPathMeta);

      expect(parsed.scenarios).toHaveLength(1);
    });

    it('should handle indentation variations', () => {
      const content = `Feature: Test

Scenario: Test
Given condition with no indent
  When action with indent
    Then result with more indent`;

      const parsed = parseGherkin(content, mockPathMeta);

      expect(parsed.scenarios![0].steps).toHaveLength(3);
    });

    it('should handle very long scenarios', () => {
      const steps = Array.from({ length: 100 }, (_, i) =>
        `  And step ${i + 1}`
      ).join('\n');

      const content = `Feature: Long scenario

Scenario: Many steps
  Given initial condition
${steps}
  Then final result`;

      const parsed = parseGherkin(content, mockPathMeta);

      expect(parsed.scenarios![0].steps.length).toBeGreaterThan(100);
    });
  });

  describe('Complex scenarios', () => {
    it('should handle real-world governance scenario', () => {
      const content = `@epic:governance @user:policy_maker
Feature: Policy Funding Allocation

  As a policy maker
  I want to allocate funds to programs
  So that resources are distributed according to priorities

Background:
  Given I am logged in as a policy maker
  And I have access to the budget allocation system

@critical @quarterly
Scenario: Allocate quarterly budget
  Given the quarterly budget is $1,000,000
  And program A has priority level 1
  And program B has priority level 2
  When I allocate $600,000 to program A
  And I allocate $400,000 to program B
  Then the allocation should be approved
  And program A should receive $600,000
  And program B should receive $400,000
  And the remaining budget should be $0

@edge-case
Scenario: Handle budget overflow
  Given the quarterly budget is $1,000,000
  When I try to allocate $1,500,000 to program A
  Then I should see an error message
  And the allocation should be rejected`;

      const parsed = parseGherkin(content, mockPathMeta);

      expect(parsed.title).toBe('Policy Funding Allocation');
      expect(parsed.frontmatter.epic).toBe('governance');
      expect(parsed.frontmatter.user).toBe('policy_maker');
      expect(parsed.scenarios).toHaveLength(2);
      expect(parsed.scenarios![0].tags).toContain('critical');
      expect(parsed.scenarios![1].tags).toContain('edge-case');
    });
  });
});
