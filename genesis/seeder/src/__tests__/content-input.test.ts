import { describe, expect, it } from 'vitest';

import { buildContentInput, buildPathInput, chaptersToSections, type PathJson } from '../content-input.js';

describe('chaptersToSections', () => {
  it('Shape 1: chapters -> modules -> sections -> conceptIds (lessons from sections)', () => {
    const path: PathJson = {
      id: 'p',
      title: 'P',
      chapters: [{
        id: 'ch1',
        title: 'Chapter 1',
        modules: [{
          id: 'mod-1',
          title: 'Module One',
          sections: [{ id: 'sec-1', title: 'Section One', conceptIds: ['manifesto', 'quiz-who-are-you'] }],
        }],
      }],
    };
    expect(chaptersToSections(path)).toEqual([{
      id: 'ch1',
      title: 'Chapter 1',
      description: undefined,
      level: 'unit',
      estimatedDuration: undefined,
      sections: [{
        id: 'sec-1',
        title: 'Section One',
        description: undefined,
        level: 'lesson',
        items: [
          { ref: 'manifesto', role: 'step', title: 'Manifesto' },
          { ref: 'quiz-who-are-you', role: 'step', title: 'Quiz Who Are You' },
        ],
      }],
    }]);
  });

  it('Shape 1b: chapters -> modules -> steps (one titled lesson per module, steps mapped like Shape 2)', () => {
    const path: PathJson = {
      id: 'fct',
      title: 'FCT',
      chapters: [{
        id: 'movement-1',
        title: 'Movement I',
        description: 'Lament & sight',
        modules: [
          {
            id: 'fct-m01',
            title: '1. The Church Dilemma',
            description: 'from assuming → to lament',
            steps: [
              {
                resourceId: 'fct-module-01-church-dilemma',
                stepTitle: 'The Church Dilemma',
                stepNarrative: 'Read the lesson.',
                learningObjectives: ['Describe the attention economy.'],
                completionCriteria: ['view'],
                optional: false,
                estimatedTime: '20 minutes',
              },
              { resourceId: 'fct-module-01-church-dilemma-discussion', stepType: 'discussion', stepTitle: 'Discuss' },
              { resourceId: 'fct-quiz-01', stepType: 'quiz' },
            ],
          },
          { id: 'fct-m02', title: '2. Systems Thinking', steps: [{ resourceId: 'fct-module-02-systems-thinking' }] },
        ],
      }],
    };
    const [movement] = chaptersToSections(path);
    expect(movement).toMatchObject({ id: 'movement-1', title: 'Movement I', description: 'Lament & sight', level: 'unit' });
    expect(movement.items).toBeUndefined();
    expect(movement.sections).toEqual([
      {
        id: 'fct-m01',
        title: '1. The Church Dilemma',
        description: 'from assuming → to lament',
        level: 'lesson',
        items: [
          {
            ref: 'fct-module-01-church-dilemma',
            role: 'step',
            title: 'The Church Dilemma',
            narrative: 'Read the lesson.',
            learningObjectives: ['Describe the attention economy.'],
            completionCriteria: { type: 'view' },
          },
          { ref: 'fct-module-01-church-dilemma-discussion', role: 'reflection', title: 'Discuss' },
          { ref: 'fct-quiz-01', role: 'checkpoint', title: 'Fct Quiz 01' },
        ],
      },
      {
        id: 'fct-m02',
        title: '2. Systems Thinking',
        description: undefined,
        level: 'lesson',
        items: [{ ref: 'fct-module-02-systems-thinking', role: 'step', title: 'Fct Module 02 Systems Thinking' }],
      },
    ]);
  });

  it('Shape 2: chapters -> steps stays flat items on the unit', () => {
    const [unit] = chaptersToSections({
      id: 'kt',
      title: 'KT',
      chapters: [{ id: 'c', title: 'C', steps: [{ resourceId: 'x', stepTitle: 'X' }] }],
    });
    expect(unit.sections).toBeUndefined();
    expect(unit.items).toEqual([{ ref: 'x', role: 'step', title: 'X' }]);
  });

  it('Shape 3: flat conceptIds become one default unit', () => {
    expect(chaptersToSections({ id: 'p', title: 'P', conceptIds: ['a-b'] })).toEqual([{
      id: 'p-default',
      title: 'P',
      description: undefined,
      level: 'unit',
      items: [{ ref: 'a-b', role: 'step', title: 'A B' }],
    }]);
  });
});

describe('buildPathInput', () => {
  it('builds an epr-composite path row and links the thumbnail it is given', () => {
    const input = buildPathInput(
      { id: 'p', title: 'P', reach: 'commons', thumbnailUrl: '/images/p.png', tags: ['t'], conceptIds: ['a'] },
      { thumbnailHash: 'sha256-abc' },
    );
    expect(input).toMatchObject({
      id: 'p',
      contentType: 'path',
      contentFormat: 'epr-composite',
      reach: 'commons',
      tags: ['t'],
      blobHash: 'sha256-abc',
      metadata: { thumbnailUrl: '/blob/sha256-abc' },
    });
    expect(JSON.parse(input.contentBody!).sections).toHaveLength(1);
  });

  it('falls back to legacy visibility for reach', () => {
    expect(buildPathInput({ id: 'p', title: 'P', visibility: 'public' }).reach).toBe('public');
  });
});

describe('buildContentInput', () => {
  it('prefers the linked blob hash over the JSON string and lets an advisory raise reach', () => {
    const input = buildContentInput(
      { id: 'app', title: 'App', contentFormat: 'html5-app', blobHash: 'abc', reach: 'community' },
      { blobHash: 'sha256-abc', advisoryReach: 'commons' },
    );
    expect(input.blobHash).toBe('sha256-abc');
    expect(input.reach).toBe('commons');
    expect(input.contentFormat).toBe('html5-app');
  });

  it('defaults an unauthored reach to private (inverted burden)', () => {
    expect(buildContentInput({ id: 'c', title: 'C', content: '# x' }).reach).toBe('private');
  });
});

describe('buildContentInput carries authored edges in signed metadata', () => {
  const lesson = {
    id: 'fct-module-01-church-dilemma',
    title: 'The Church Dilemma',
    contentType: 'lesson',
    content: '# Lament',
    reach: 'commons',
    relatedNodeIds: ['fct-module-02-systems-thinking'],
    relationships: [
      { target: 'fct-bible-psalm-13', type: 'REFERENCES', role: 'anchor' },
      { target: 'fct-module-02-systems-thinking', type: 'RELATES_TO', role: 'callback' },
      { target: 'fct-module-01-church-dilemma', type: 'RELATES_TO', role: 'self' },
    ],
  };

  it('writes one canonical, role-bearing list and skips self-edges', () => {
    const input = buildContentInput(lesson);
    expect(input.metadata?.relationships).toEqual([
      { type: 'REFERENCES', targetId: 'fct-bible-psalm-13', role: 'anchor' },
      { type: 'RELATES_TO', targetId: 'fct-module-02-systems-thinking', role: 'callback' },
    ]);
  });

  it('omits the key when the atom authors no edges', () => {
    const input = buildContentInput({ id: 'x', title: 'X', content: 'x', metadata: { relationships: [] } });
    expect(input.metadata?.relationships).toBeUndefined();
  });

  it('keeps bare relatedNodeIds out of it, so their seed hash does not move', () => {
    const input = buildContentInput({ id: 'y', title: 'Y', content: 'y', relatedNodeIds: ['manifesto'] });
    expect(input.metadata?.relationships).toBeUndefined();
    expect(input.metadata?.relatedNodeIds).toEqual(['manifesto']);
  });

  it('keeps an explicitly authored empty list, which is how an author drops every edge', () => {
    const input = buildContentInput({ id: 'z', title: 'Z', content: 'z', relationships: [] });
    expect(input.metadata?.relationships).toEqual([]);
  });
});

