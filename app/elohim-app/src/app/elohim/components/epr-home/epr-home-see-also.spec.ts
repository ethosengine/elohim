import { describe, expect, it } from 'vitest';
import { mergeSeeAlso } from './epr-home-see-also';

describe('mergeSeeAlso', () => {
  it('keeps authored rows on duplicate targets and includes discovered neighbours', () => {
    const authored = {
      id: 'authored',
      targetId: 'known',
      target: { title: 'Authored title' },
      relationshipType: 'related_to',
      inferenceSource: 'explicit',
    };
    const edge = {
      contentId: 'known',
      relationshipType: 'related_to',
      inferenceSource: 'tag',
      confidence: 1,
      depth: 1,
      children: [],
    };
    const graph = {
      rootId: 'root',
      totalNodes: 3,
      related: [
        edge,
        { ...edge, contentId: 'new-neighbour', title: 'New title' },
        { ...edge, contentId: 'untitled' },
      ],
    };
    const rows = mergeSeeAlso([authored], graph);
    expect(rows).toHaveLength(3);
    expect(rows[0]).toBe(authored);
    expect(rows[1]).toMatchObject({
      targetId: 'new-neighbour',
      target: { title: 'New title' },
      inferenceSource: 'tag',
    });
    expect(rows[2]).toMatchObject({ targetId: 'untitled', target: null });
    expect(mergeSeeAlso([authored], null)).toEqual([authored]);
  });
});
