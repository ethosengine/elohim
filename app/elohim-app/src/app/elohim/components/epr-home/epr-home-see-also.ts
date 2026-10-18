import type {
  ContentGraphView,
  RelationshipWithContentView,
} from '@elohim/storage-client/generated';

export type SeeAlsoRow = Pick<
  RelationshipWithContentView,
  'id' | 'targetId' | 'relationshipType' | 'inferenceSource'
> & {
  target: { title: string } | null;
};

/** Keep authored names and provenance when discovery finds the same neighbour. */
export function mergeSeeAlso(
  authored: readonly SeeAlsoRow[],
  graph: ContentGraphView | null
): SeeAlsoRow[] {
  const rows = new Map(authored.map(row => [row.targetId, row]));
  for (const edge of graph?.related ?? []) {
    if (rows.has(edge.contentId)) continue;
    const titled = edge as typeof edge & { title?: string };
    rows.set(edge.contentId, {
      id: edge.contentId,
      targetId: edge.contentId,
      target: titled.title ? { title: titled.title } : null,
      relationshipType: edge.relationshipType,
      inferenceSource: edge.inferenceSource,
    });
  }
  return [...rows.values()];
}
