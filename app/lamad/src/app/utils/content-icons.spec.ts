import { describe, expect, it } from 'vitest';

import { inferContentTypeFromId } from './content-icons';

describe('inferContentTypeFromId', () => {
  it('reads the forms a recomposed course names in its atom ids', () => {
    expect(inferContentTypeFromId('fct-module-07-regeneration')).toBe('lesson');
    expect(inferContentTypeFromId('fct-module-07-regeneration-story')).toBe('narrative');
    expect(inferContentTypeFromId('fct-module-07-regeneration-counter-story')).toBe('narrative');
    expect(inferContentTypeFromId('fct-module-07-regeneration-discussion')).toBe('discussion');
    expect(inferContentTypeFromId('fct-module-07-regeneration-application')).toBe('practice');
    expect(inferContentTypeFromId('fct-module-07-regeneration-lane-tech-workers')).toBe('practice');
    expect(inferContentTypeFromId('fct-module-07-regeneration-homework')).toBe('practice');
    expect(inferContentTypeFromId('fct-module-07-regeneration-quiz')).toBe('assessment');
    expect(inferContentTypeFromId('fct-module-07-regeneration-reflection')).toBe('reflection');
    expect(inferContentTypeFromId('fct-bible-acts-2-42-47')).toBe('bible-verse');
    expect(inferContentTypeFromId('fct-media-06-lab-parable-of-the-polygons')).toBe('simulation');
    expect(inferContentTypeFromId('evolution-of-trust')).toBe('simulation');
  });

  it('keeps the generic inferences for everything else', () => {
    expect(inferContentTypeFromId('governance-epic')).toBe('epic');
    expect(inferContentTypeFromId('some-concept')).toBe('concept');
    expect(inferContentTypeFromId('intro-video-1')).toBe('reference');
  });
});
