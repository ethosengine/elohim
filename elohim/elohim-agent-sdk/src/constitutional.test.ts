import { describe, it, expect } from 'vitest';
import { buildSystemPrompt, buildUserPrompt } from './constitutional.js';

describe('buildSystemPrompt', () => {
  it('contains all required sections', () => {
    const prompt = buildSystemPrompt();

    expect(prompt).toContain('CONSTITUTIONAL CONTEXT');
    expect(prompt).toContain('ACTIVE PRINCIPLES');
    expect(prompt).toContain('INVIOLABLE BOUNDARIES');
    expect(prompt).toContain('INTERPRETIVE GUIDANCE');
  });

  it('contains global principles', () => {
    const prompt = buildSystemPrompt();

    expect(prompt).toContain('Human Dignity');
    expect(prompt).toContain('Ecological Integrity');
    expect(prompt).toContain('Knowledge Accessibility');
    expect(prompt).toContain('Transparency of Power');
    expect(prompt).toContain('Non-Domination');
  });

  it('contains boundaries with enforcement markers', () => {
    const prompt = buildSystemPrompt();

    expect(prompt).toContain('[HARD BLOCK]');
    expect(prompt).toContain('[REQUIRES GOVERNANCE]');
    expect(prompt).toContain('[SOFT LIMIT]');
    expect(prompt).toContain('No Existential Harm');
    expect(prompt).toContain('Dignity Preservation');
  });

  it('contains interpretive guidance rules', () => {
    const prompt = buildSystemPrompt();

    expect(prompt).toContain(
      'Higher layer principles override lower layers when in conflict',
    );
    expect(prompt).toContain(
      'Dignity and flourishing take precedence when uncertain',
    );
    expect(prompt).toContain(
      'Flag ambiguous cases for human deliberation rather than deciding',
    );
    expect(prompt).toContain(
      'Log your reasoning for audit and precedent building',
    );
  });

  it('contains stack hash', () => {
    const prompt = buildSystemPrompt();

    expect(prompt).toContain('Constitutional Stack Hash:');
  });

  it('orders principles by weight (highest first)', () => {
    const prompt = buildSystemPrompt();

    const dignityIndex = prompt.indexOf('Human Dignity');
    const ecologicalIndex = prompt.indexOf('Ecological Integrity');
    const knowledgeIndex = prompt.indexOf('Knowledge Accessibility');
    const transparencyIndex = prompt.indexOf('Transparency of Power');
    const nonDominationIndex = prompt.indexOf('Non-Domination');

    expect(dignityIndex).toBeLessThan(ecologicalIndex);
    expect(ecologicalIndex).toBeLessThan(knowledgeIndex);
    expect(knowledgeIndex).toBeLessThan(transparencyIndex);
    expect(transparencyIndex).toBeLessThan(nonDominationIndex);
  });

  it('tags principles with [GLOBAL] layer', () => {
    const prompt = buildSystemPrompt();

    expect(prompt).toContain('[GLOBAL]');
  });
});

describe('buildUserPrompt', () => {
  it('includes capability name and known description', () => {
    const prompt = buildUserPrompt('path-recommendation', { query: 'test' });

    expect(prompt).toContain('path-recommendation');
    expect(prompt).toContain('Suggest learning paths based on learner context');
  });

  it('includes query param when present', () => {
    const prompt = buildUserPrompt('path-recommendation', { query: 'test' });

    expect(prompt).toContain('Query: test');
  });

  it('includes content param when present', () => {
    const prompt = buildUserPrompt('content-safety-review', {
      content: 'some content to review',
    });

    expect(prompt).toContain('Content to analyze: some content to review');
  });

  it('includes contentId param when present', () => {
    const prompt = buildUserPrompt('content-safety-review', {
      contentId: 'abc-123',
    });

    expect(prompt).toContain('Content ID: abc-123');
  });

  it('uses default description for unknown capabilities', () => {
    const prompt = buildUserPrompt('unknown-cap', {});

    expect(prompt).toContain('unknown-cap');
    expect(prompt).toContain('Process capability request');
  });

  it('includes response format instructions', () => {
    const prompt = buildUserPrompt('path-recommendation', {});

    expect(prompt).toContain('Respond with a JSON object');
    expect(prompt).toContain('constitutionalReasoning');
    expect(prompt).toContain('payload');
  });

  it('omits optional params when not provided', () => {
    const prompt = buildUserPrompt('path-recommendation', {});

    expect(prompt).not.toContain('Content to analyze:');
    expect(prompt).not.toContain('Content ID:');
    expect(prompt).not.toContain('Query:');
  });

  it('handles all known capabilities', () => {
    const capabilities = [
      { name: 'path-recommendation', desc: 'Suggest learning paths' },
      { name: 'content-safety-review', desc: 'Review content for safety' },
      { name: 'spiral-detection', desc: 'Detect patterns' },
      { name: 'attestation-recommendation', desc: 'Recommend whether to issue' },
    ];

    for (const cap of capabilities) {
      const prompt = buildUserPrompt(cap.name, {});
      expect(prompt).toContain(cap.desc);
    }
  });
});
