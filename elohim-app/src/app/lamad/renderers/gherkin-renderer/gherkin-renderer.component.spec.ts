import { ComponentFixture, TestBed } from '@angular/core/testing';
import { SimpleChange } from '@angular/core';
import { GherkinRendererComponent } from './gherkin-renderer.component';
import { ContentNode } from '../../models/content-node.model';

describe('GherkinRendererComponent', () => {
  let component: GherkinRendererComponent;
  let fixture: ComponentFixture<GherkinRendererComponent>;

  const createContentNode = (content: string): ContentNode => ({
    id: 'test-gherkin',
    title: 'Test Gherkin',
    description: 'Test gherkin content',
    contentType: 'feature',
    contentFormat: 'gherkin',
    content,
    tags: ['gherkin'],
    relatedNodeIds: [],
    metadata: {},
  });

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [GherkinRendererComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(GherkinRendererComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  describe('parsing basic feature', () => {
    it('should parse feature with name', () => {
      const gherkin = `Feature: User Login
  As a user I want to login`;

      component.node = createContentNode(gherkin);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      expect(component.feature).toBeTruthy();
      expect(component.feature!.name).toBe('User Login');
    });

    it('should parse feature description', () => {
      const gherkin = `Feature: Test Feature
  This is the description
  on multiple lines`;

      component.node = createContentNode(gherkin);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      expect(component.feature!.description).toContain('This is the description');
    });

    it('should parse feature tags', () => {
      const gherkin = `@wip @critical
Feature: Tagged Feature`;

      component.node = createContentNode(gherkin);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      expect(component.feature!.tags).toContain('@wip');
      expect(component.feature!.tags).toContain('@critical');
    });
  });

  describe('parsing scenarios', () => {
    it('should parse basic scenario', () => {
      const gherkin = `Feature: Test
Scenario: Basic scenario
  Given I am on the homepage
  When I click the button
  Then I see the result`;

      component.node = createContentNode(gherkin);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      expect(component.feature!.scenarios.length).toBe(1);
      expect(component.feature!.scenarios[0].name).toBe('Basic scenario');
      expect(component.feature!.scenarios[0].steps.length).toBe(3);
    });

    it('should parse scenario with tags', () => {
      const gherkin = `Feature: Test
@smoke @regression
Scenario: Tagged scenario
  Given a step`;

      component.node = createContentNode(gherkin);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      expect(component.feature!.scenarios[0].tags).toContain('@smoke');
      expect(component.feature!.scenarios[0].tags).toContain('@regression');
    });

    it('should parse multiple scenarios', () => {
      const gherkin = `Feature: Test
Scenario: First
  Given step one

Scenario: Second
  Given step two`;

      component.node = createContentNode(gherkin);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      expect(component.feature!.scenarios.length).toBe(2);
      expect(component.feature!.scenarios[0].name).toBe('First');
      expect(component.feature!.scenarios[1].name).toBe('Second');
    });
  });

  describe('parsing steps', () => {
    it('should parse Given step', () => {
      const gherkin = `Feature: Test
Scenario: Test
  Given I have something`;

      component.node = createContentNode(gherkin);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      expect(component.feature!.scenarios[0].steps[0].keyword.trim()).toBe('Given');
      expect(component.feature!.scenarios[0].steps[0].text).toBe('I have something');
    });

    it('should parse When step', () => {
      const gherkin = `Feature: Test
Scenario: Test
  When I do something`;

      component.node = createContentNode(gherkin);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      expect(component.feature!.scenarios[0].steps[0].keyword.trim()).toBe('When');
    });

    it('should parse Then step', () => {
      const gherkin = `Feature: Test
Scenario: Test
  Then something happens`;

      component.node = createContentNode(gherkin);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      expect(component.feature!.scenarios[0].steps[0].keyword.trim()).toBe('Then');
    });

    it('should parse And step', () => {
      const gherkin = `Feature: Test
Scenario: Test
  Given first
  And second`;

      component.node = createContentNode(gherkin);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      expect(component.feature!.scenarios[0].steps[1].keyword.trim()).toBe('And');
    });

    it('should parse But step', () => {
      const gherkin = `Feature: Test
Scenario: Test
  Given first
  But not this`;

      component.node = createContentNode(gherkin);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      expect(component.feature!.scenarios[0].steps[1].keyword.trim()).toBe('But');
    });

    it('should parse * step', () => {
      const gherkin = `Feature: Test
Scenario: Test
  * a generic step`;

      component.node = createContentNode(gherkin);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      expect(component.feature!.scenarios[0].steps[0].keyword.trim()).toBe('*');
    });
  });

  describe('parsing background', () => {
    it('should parse background section', () => {
      const gherkin = `Feature: Test
Background: Common setup
  Given I am logged in

Scenario: Test
  When I do something`;

      component.node = createContentNode(gherkin);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      expect(component.feature!.background).toBeTruthy();
      expect(component.feature!.background!.name).toBe('Common setup');
      expect(component.feature!.background!.steps.length).toBe(1);
    });
  });

  describe('parsing scenario outline', () => {
    it('should parse scenario outline', () => {
      const gherkin = `Feature: Test
Scenario Outline: Test with examples
  Given I have <count> items
  Then total is <total>

  Examples:
    | count | total |
    | 1     | 1     |
    | 2     | 2     |`;

      component.node = createContentNode(gherkin);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      expect(component.feature!.scenarios[0].type).toBe('scenario_outline');
      expect(component.feature!.scenarios[0].examples).toBeTruthy();
      expect(component.feature!.scenarios[0].examples!.length).toBe(1);
    });

    it('should parse scenario template (alias)', () => {
      const gherkin = `Feature: Test
Scenario Template: Test with examples
  Given I have <count> items

  Examples:
    | count |
    | 1     |`;

      component.node = createContentNode(gherkin);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      expect(component.feature!.scenarios[0].type).toBe('scenario_outline');
    });
  });

  describe('parsing data tables', () => {
    it('should parse data table in step', () => {
      const gherkin = `Feature: Test
Scenario: Test
  Given I have the following items:
    | name  | price |
    | apple | 1.00  |
    | banana| 0.50  |`;

      component.node = createContentNode(gherkin);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      const step = component.feature!.scenarios[0].steps[0];
      expect(step.dataTable).toBeTruthy();
      expect(step.dataTable!.length).toBe(3);
      expect(step.dataTable![0]).toContain('name');
    });
  });

  describe('parsing doc strings', () => {
    it('should parse doc string with triple quotes', () => {
      const gherkin = `Feature: Test
Scenario: Test
  Given I have the following JSON:
    """
    {
      "key": "value"
    }
    """`;

      component.node = createContentNode(gherkin);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      const step = component.feature!.scenarios[0].steps[0];
      expect(step.docString).toBeTruthy();
      expect(step.docString).toContain('"key"');
    });

    it('should parse doc string with triple single quotes', () => {
      const gherkin = `Feature: Test
Scenario: Test
  Given I have content:
    '''
    Some content here
    '''`;

      component.node = createContentNode(gherkin);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      const step = component.feature!.scenarios[0].steps[0];
      expect(step.docString).toBeTruthy();
    });
  });

  describe('UI methods', () => {
    it('should toggle scenario collapse state', () => {
      const gherkin = `Feature: Test
Scenario: Test
  Given a step`;

      component.node = createContentNode(gherkin);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      expect(component.feature!.scenarios[0].collapsed).toBeFalse();

      component.toggleScenario(0);
      expect(component.feature!.scenarios[0].collapsed).toBeTrue();

      component.toggleScenario(0);
      expect(component.feature!.scenarios[0].collapsed).toBeFalse();
    });

    it('should return correct scenario keyword', () => {
      expect(component.getScenarioKeyword('scenario')).toBe('Scenario');
      expect(component.getScenarioKeyword('scenario_outline')).toBe('Scenario Outline');
      expect(component.getScenarioKeyword('background')).toBe('Background');
    });

    it('should highlight placeholders in step text', () => {
      const result = component.highlightStepText('I have <count> items');
      // The method wraps placeholders first, then strings
      // Note: This creates nested spans since "placeholder" gets string-highlighted
      expect(result).toContain('&lt;count&gt;');
      expect(result).toContain('placeholder');
    });

    it('should highlight strings in step text', () => {
      const result = component.highlightStepText('I enter "username"');
      expect(result).toContain('class="string"');
      expect(result).toContain('"username"');
    });
  });

  describe('edge cases', () => {
    it('should handle empty content', () => {
      component.node = createContentNode('');
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      expect(component.feature).toBeNull();
    });

    it('should handle non-string content', () => {
      const node = createContentNode('');
      (node as any).content = { invalid: 'object' };
      component.node = node;
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      expect(component.content).toBe('');
    });

    it('should handle scenarios without feature line', () => {
      const gherkin = `Scenario: Standalone
  Given a step`;

      component.node = createContentNode(gherkin);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      expect(component.feature).toBeTruthy();
      expect(component.feature!.scenarios.length).toBe(1);
    });

    it('should calculate total steps', () => {
      const gherkin = `Feature: Test
Background:
  Given background step

Scenario: First
  Given step one
  When step two

Scenario: Second
  Then step three`;

      component.node = createContentNode(gherkin);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      expect(component.totalSteps).toBe(4); // 1 background + 3 scenario steps
    });

    it('should handle embedded mode', () => {
      component.embedded = true;
      component.node = createContentNode('Feature: Test\nScenario: Test Scenario\n  Given a step');
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });
      fixture.detectChanges();

      const container = fixture.nativeElement.querySelector('.gherkin-container');
      expect(container).toBeTruthy();
      expect(container.classList.contains('embedded')).toBeTrue();
    });

    it('should display fallback for unparseable content', () => {
      // Force feature to be null while content exists
      component.content = 'invalid content that cannot be parsed';
      component.feature = null;
      fixture.detectChanges();

      const fallback = fixture.nativeElement.querySelector('.gherkin-fallback');
      expect(fallback).toBeTruthy();
    });
  });

  describe('parsing Examples in Scenario Outline', () => {
    it('should parse Examples with name', () => {
      const gherkin = `Feature: Test
Scenario Outline: Test
  Given step

  Examples: Valid cases
    | col |
    | val |`;

      component.node = createContentNode(gherkin);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      expect(component.feature!.scenarios[0].examples![0].name).toBe('Valid cases');
    });

    it('should parse multiple Examples blocks', () => {
      const gherkin = `Feature: Test
Scenario Outline: Test
  Given step

  Examples: First
    | col |
    | 1   |

  Examples: Second
    | col |
    | 2   |`;

      component.node = createContentNode(gherkin);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      expect(component.feature!.scenarios[0].examples!.length).toBe(2);
    });
  });
});
