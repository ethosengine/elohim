import { HexNode } from '../hexagon-grid.component';

// Deterministic affinity cycle: unseen → low → medium → high
const CYCLE: { affinityLevel: HexNode['affinityLevel']; affinity: number }[] = [
  { affinityLevel: 'unseen', affinity: 0.0 },
  { affinityLevel: 'low',    affinity: 0.3 },
  { affinityLevel: 'medium', affinity: 0.65 },
  { affinityLevel: 'high',   affinity: 0.92 },
];

function makeNodes(count: number, label = 'Node'): HexNode[] {
  return Array.from({ length: count }, (_, i) => ({
    id: `${label.toLowerCase().replace(/\s+/g, '-')}-${i}`,
    title: `${label} ${i + 1}`,
    ...CYCLE[i % CYCLE.length],
  }));
}

/** 48 mixed-affinity nodes — default showcase */
export const nodes48 = makeNodes(48);

/** 12 nodes — sparse layout */
export const nodes12 = makeNodes(12, 'Concept');

/** 120 nodes — dense layout */
export const nodes120 = makeNodes(120, 'Item');

/** All high affinity — mastered curriculum */
export const nodesAllHigh: HexNode[] = Array.from({ length: 48 }, (_, i) => ({
  id: `mastered-${i}`,
  title: `Mastered ${i + 1}`,
  affinity: 0.9 + (i % 10) * 0.01,
  affinityLevel: 'high',
}));

/** All unseen — new learner */
export const nodesAllUnseen: HexNode[] = Array.from({ length: 48 }, (_, i) => ({
  id: `new-${i}`,
  title: `Concept ${i + 1}`,
  affinity: 0,
  affinityLevel: 'unseen',
}));

/** Progressive mastery — gradient from unseen to high */
export const nodesLearnerProgress: HexNode[] = Array.from({ length: 60 }, (_, i) => {
  const progress = i / 59;
  let affinityLevel: HexNode['affinityLevel'];
  if (progress < 0.25)      affinityLevel = 'unseen';
  else if (progress < 0.5)  affinityLevel = 'low';
  else if (progress < 0.75) affinityLevel = 'medium';
  else                      affinityLevel = 'high';
  return {
    id: `progress-${i}`,
    title: `Concept ${i + 1}`,
    affinity: progress,
    affinityLevel,
  };
});
