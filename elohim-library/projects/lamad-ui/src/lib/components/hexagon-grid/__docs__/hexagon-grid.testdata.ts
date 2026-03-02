import { HexNode, HexEdge, HexMasteryLevel, HexGraphData } from '../hexagon-grid.model';

// =============================================================================
// Legacy data (existing, unchanged)
// =============================================================================

const CYCLE: { affinityLevel: HexNode['affinityLevel']; affinity: number }[] = [
  { affinityLevel: 'unseen', affinity: 0 },
  { affinityLevel: 'low', affinity: 0.3 },
  { affinityLevel: 'medium', affinity: 0.65 },
  { affinityLevel: 'high', affinity: 0.92 },
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
  if (progress < 0.25) affinityLevel = 'unseen';
  else if (progress < 0.5) affinityLevel = 'low';
  else if (progress < 0.75) affinityLevel = 'medium';
  else affinityLevel = 'high';
  return {
    id: `progress-${i}`,
    title: `Concept ${i + 1}`,
    affinity: progress,
    affinityLevel,
  };
});

// =============================================================================
// Helpers
// =============================================================================

function pick<T>(arr: T[]): T {
  return arr[Math.floor(seededRandom() * arr.length)];
}

// Deterministic pseudo-random for reproducible stories
let seed = 42;
function seededRandom(): number {
  seed = (seed * 16807) % 2147483647;
  return (seed - 1) / 2147483646;
}
function resetSeed(s = 42): void {
  seed = s;
}

function affinityLevelFromScore(a: number): HexNode['affinityLevel'] {
  if (a >= 0.67) return 'high';
  if (a >= 0.34) return 'medium';
  if (a > 0) return 'low';
  return 'unseen';
}

// =============================================================================
// Scenario 1: Arithmetic Course — Domain Heat Map (Khan Academy style)
// =============================================================================

const ARITHMETIC_CATEGORIES: { name: string; concepts: string[] }[] = [
  {
    name: 'Counting & Place Value',
    concepts: [
      'Counting to 100', 'Skip counting', 'Place value ones', 'Place value tens',
      'Place value hundreds', 'Comparing numbers', 'Ordering numbers', 'Number line',
      'Rounding to nearest 10', 'Rounding to nearest 100', 'Even and odd numbers',
      'Number patterns',
    ],
  },
  {
    name: 'Addition & Subtraction',
    concepts: [
      'Adding single digits', 'Adding with carrying', 'Subtracting single digits',
      'Subtracting with borrowing', 'Mental addition', 'Mental subtraction',
      'Word problems: addition', 'Word problems: subtraction', 'Missing addend',
      'Properties of addition', 'Estimation: sums', 'Multi-digit addition',
      'Multi-digit subtraction',
    ],
  },
  {
    name: 'Multiplication & Division',
    concepts: [
      'Multiplication as groups', 'Times tables 2-5', 'Times tables 6-9',
      'Times tables 10-12', 'Multiplication properties', 'Multi-digit multiplication',
      'Division as sharing', 'Division with remainders', 'Long division',
      'Factors and multiples', 'Prime numbers', 'Divisibility rules',
    ],
  },
  {
    name: 'Fractions',
    concepts: [
      'What is a fraction?', 'Fractions on number line', 'Comparing fractions',
      'Equivalent fractions', 'Simplifying fractions', 'Adding fractions',
      'Subtracting fractions', 'Mixed numbers', 'Improper fractions',
      'Fractions of a whole',
    ],
  },
  {
    name: 'Decimals & Measurement',
    concepts: [
      'Decimal place value', 'Comparing decimals', 'Adding decimals',
      'Subtracting decimals', 'Fractions to decimals', 'Units of length',
      'Units of weight', 'Units of time', 'Perimeter', 'Area basics',
    ],
  },
];

const BLOOM_LEVELS: HexMasteryLevel[] = [
  'not_started', 'seen', 'remember', 'understand', 'apply', 'analyze', 'evaluate', 'create',
];

export function makeArithmeticTerritory(): HexGraphData {
  resetSeed(101);
  const nodes: HexNode[] = [];
  const edges: HexEdge[] = [];
  let idx = 0;

  for (const cat of ARITHMETIC_CATEGORIES) {
    for (const concept of cat.concepts) {
      // Distribute mastery: earlier categories tend more mastered, later less
      const catIndex = ARITHMETIC_CATEGORIES.indexOf(cat);
      const masteryBias = 1 - catIndex / ARITHMETIC_CATEGORIES.length;
      const roll = seededRandom();
      const masteryIdx = Math.min(7, Math.floor(roll * masteryBias * 8));
      const level = BLOOM_LEVELS[masteryIdx];
      const affinity = masteryIdx / 7;

      nodes.push({
        id: `arith-${idx}`,
        title: concept,
        affinity,
        affinityLevel: affinityLevelFromScore(affinity),
        masteryLevel: level,
        category: cat.name,
      });
      idx++;
    }
  }

  // Prerequisite edges within categories (sequential)
  let prevId: string | null = null;
  for (const node of nodes) {
    if (prevId && nodes.find(n => n.id === prevId)?.category === node.category) {
      edges.push({
        id: `edge-${prevId}-${node.id}`,
        sourceId: prevId,
        targetId: node.id,
        edgeType: 'follows',
        confidence: 0.8,
        directed: true,
      });
    }
    prevId = node.id;
  }

  // Cross-category prerequisites
  const crossPrereqs: [string, string][] = [
    ['Adding single digits', 'Multiplication as groups'],
    ['Times tables 2-5', 'Division as sharing'],
    ['Comparing numbers', 'Comparing fractions'],
    ['Place value tens', 'Decimal place value'],
    ['Adding fractions', 'Adding decimals'],
  ];
  for (const [from, to] of crossPrereqs) {
    const src = nodes.find(n => n.title === from);
    const tgt = nodes.find(n => n.title === to);
    if (src && tgt) {
      edges.push({
        id: `xedge-${src.id}-${tgt.id}`,
        sourceId: src.id,
        targetId: tgt.id,
        edgeType: 'depends_on',
        confidence: 0.9,
        directed: true,
      });
    }
  }

  return { nodes, edges };
}

// =============================================================================
// Scenario 2: Adam & Eve Love Map
// =============================================================================

interface LoveMapPersona {
  name: string;
  owner: 'self' | 'other';
  domains: { category: string; concepts: string[]; affinityRange: [number, number] }[];
}

const ADAM: LoveMapPersona = {
  name: 'Adam',
  owner: 'self',
  domains: [
    {
      category: 'Stewardship',
      concepts: ['Incorruptible Stewardship', 'Regenerative Agriculture', 'Intergenerational Thinking', 'Ecological Accounting'],
      affinityRange: [0.7, 1.0],
    },
    {
      category: 'Value Recognition',
      concepts: ["The Value Scanner's Eye", 'Care Economy Visibility', 'Resource Mapping', 'Generative Economics'],
      affinityRange: [0.6, 0.95],
    },
    {
      category: 'Community',
      concepts: ['Community Garden Club', 'Local Food Systems', 'Soil Health Networks'],
      affinityRange: [0.5, 0.85],
    },
  ],
};

const EVE: LoveMapPersona = {
  name: 'Eve',
  owner: 'other',
  domains: [
    {
      category: 'Truth-Seeking',
      concepts: ["The Public Observer's Lens", 'Transparency as Immune System', 'Accountability Mechanisms', 'Investigative Patterns'],
      affinityRange: [0.7, 1.0],
    },
    {
      category: 'Digital Relationships',
      concepts: ['Social Medium', 'Genuine Connection', 'Digital Presence', 'Relationship-Centered Communication'],
      affinityRange: [0.6, 0.95],
    },
    {
      category: 'Community',
      concepts: ['Community Garden Club', 'Neighborhood Networks', 'Mutual Aid'],
      affinityRange: [0.45, 0.8],
    },
  ],
};

// Concepts both Adam and Eve share
const SHARED_CONCEPTS = [
  { title: 'Care Economy Visibility', category: 'Shared Ground', overlapScore: 0.85 },
  { title: 'Community Garden Club', category: 'Shared Ground', overlapScore: 0.9 },
  { title: 'Technology Serves Love', category: 'Shared Ground', overlapScore: 0.72 },
  { title: 'Love as Technology', category: 'Shared Ground', overlapScore: 0.68 },
  { title: 'Value-Generative Economics', category: 'Synthesis', overlapScore: 0.6 },
  { title: 'Communities Self-Govern', category: 'Synthesis', overlapScore: 0.55 },
  { title: 'Different Values Coexist', category: 'Synthesis', overlapScore: 0.75 },
  { title: 'Our Path Forward', category: 'Synthesis', overlapScore: 0.5 },
];

export function makeAdamEveLoveMap(): HexGraphData {
  resetSeed(777);
  const nodes: HexNode[] = [];
  const edges: HexEdge[] = [];
  let idx = 0;

  // Adam's knowledge (red)
  for (const domain of ADAM.domains) {
    for (const concept of domain.concepts) {
      if (SHARED_CONCEPTS.some(s => s.title === concept)) continue; // skip, will add as shared
      const affinity = domain.affinityRange[0] + seededRandom() * (domain.affinityRange[1] - domain.affinityRange[0]);
      nodes.push({
        id: `adam-${idx}`,
        title: concept,
        affinity,
        affinityLevel: affinityLevelFromScore(affinity),
        owner: 'self',
        category: domain.category,
        sourceLabel: 'Adam',
      });
      idx++;
    }
  }

  // Eve's knowledge (blue)
  for (const domain of EVE.domains) {
    for (const concept of domain.concepts) {
      if (SHARED_CONCEPTS.some(s => s.title === concept)) continue;
      const affinity = domain.affinityRange[0] + seededRandom() * (domain.affinityRange[1] - domain.affinityRange[0]);
      nodes.push({
        id: `eve-${idx}`,
        title: concept,
        affinity,
        affinityLevel: affinityLevelFromScore(affinity),
        owner: 'other',
        category: domain.category,
        sourceLabel: 'Eve',
      });
      idx++;
    }
  }

  // Shared knowledge (purple)
  for (const shared of SHARED_CONCEPTS) {
    const affinity = 0.5 + seededRandom() * 0.5;
    nodes.push({
      id: `shared-${idx}`,
      title: shared.title,
      affinity,
      affinityLevel: affinityLevelFromScore(affinity),
      owner: 'shared',
      overlapScore: shared.overlapScore,
      category: shared.category,
      sourceLabel: 'Adam & Eve',
    });
    idx++;
  }

  // Teaching edges: Adam teaches Eve (chapter 2)
  const adamTeaches = ['Incorruptible Stewardship', 'Regenerative Agriculture', "The Value Scanner's Eye", 'Intergenerational Thinking'];
  for (const title of adamTeaches) {
    const src = nodes.find(n => n.title === title && n.owner === 'self');
    const tgt = nodes.find(n => n.owner === 'shared' && n.category === 'Shared Ground');
    if (src && tgt) {
      edges.push({
        id: `teaches-adam-${src.id}`,
        sourceId: src.id,
        targetId: tgt.id,
        edgeType: 'teaches',
        confidence: 0.7,
        directed: true,
        label: 'Adam teaches',
      });
    }
  }

  // Teaching edges: Eve teaches Adam (chapter 3)
  const eveTeaches = ["The Public Observer's Lens", 'Social Medium', 'Genuine Connection', 'Transparency as Immune System'];
  for (const title of eveTeaches) {
    const src = nodes.find(n => n.title === title && n.owner === 'other');
    const tgt = nodes.find(n => n.owner === 'shared' && n.category === 'Synthesis');
    if (src && tgt) {
      edges.push({
        id: `teaches-eve-${src.id}`,
        sourceId: src.id,
        targetId: tgt.id,
        edgeType: 'teaches',
        confidence: 0.7,
        directed: true,
        label: 'Eve teaches',
      });
    }
  }

  return { nodes, edges };
}

// =============================================================================
// Scenario 3: Ethics & Compliance Training (Freshness Decay)
// =============================================================================

const COMPLIANCE_MODULES: { category: string; modules: string[] }[] = [
  {
    category: 'Data Privacy',
    modules: [
      'GDPR Fundamentals', 'Data Classification', 'Consent Management',
      'Breach Notification', 'Right to Erasure', 'Cross-Border Transfer',
    ],
  },
  {
    category: 'Anti-Bribery',
    modules: [
      'FCPA Overview', 'Gift Policy', 'Third-Party Due Diligence',
      'Red Flag Recognition', 'Reporting Obligations',
    ],
  },
  {
    category: 'Workplace Safety',
    modules: [
      'Hazard Identification', 'Emergency Procedures', 'PPE Requirements',
      'Incident Reporting', 'Ergonomics', 'Fire Safety',
    ],
  },
  {
    category: 'Information Security',
    modules: [
      'Password Hygiene', 'Phishing Recognition', 'Clean Desk Policy',
      'Secure File Sharing', 'Social Engineering', 'Incident Response',
    ],
  },
  {
    category: 'Ethics & Conduct',
    modules: [
      'Code of Conduct', 'Conflict of Interest', 'Whistleblower Protection',
      'Anti-Harassment', 'Diversity & Inclusion', 'Fair Competition',
    ],
  },
];

export function makeComplianceTraining(monthsSinceTraining = 8): HexGraphData {
  resetSeed(333);
  const nodes: HexNode[] = [];
  let idx = 0;

  // Decay rates by how deeply the content was learned
  // "seen" fades fast (annual slideshows), "apply" fades slowly (hands-on drills)
  const DECAY_RATES: Record<HexMasteryLevel, number> = {
    not_started: 0,
    seen: 0.05,
    remember: 0.03,
    understand: 0.02,
    apply: 0.015,
    analyze: 0.01,
    evaluate: 0.008,
    create: 0.005,
  };

  for (const cat of COMPLIANCE_MODULES) {
    for (const mod of cat.modules) {
      // At training time, everyone was at "understand" or "apply"
      const trainingLevel: HexMasteryLevel = seededRandom() > 0.4 ? 'apply' : 'understand';
      const trainingAffinity = trainingLevel === 'apply' ? 0.75 : 0.55;

      // Calculate freshness decay
      const lambda = DECAY_RATES[trainingLevel];
      const freshness = Math.max(0.05, Math.exp(-lambda * monthsSinceTraining));

      nodes.push({
        id: `compliance-${idx}`,
        title: mod,
        affinity: trainingAffinity,
        affinityLevel: affinityLevelFromScore(trainingAffinity),
        masteryLevel: trainingLevel,
        freshness,
        category: cat.category,
        subtitle: `Trained ${monthsSinceTraining}mo ago`,
      });
      idx++;
    }
  }

  return { nodes, edges: [] };
}

// =============================================================================
// Scenario 4: Political Compass Love Map
// =============================================================================

const POLITICAL_QUADRANTS: {
  name: string;
  compassX: number;
  compassY: number;
  positions: string[];
}[] = [
  {
    name: 'Libertarian Left',
    compassX: -0.7, compassY: -0.7,
    positions: [
      'Universal Basic Income', 'Worker Cooperatives', 'Drug Decriminalization',
      'Open Borders', 'Community Land Trusts', 'Mutual Aid Networks',
      'Direct Democracy', 'Restorative Justice',
    ],
  },
  {
    name: 'Authoritarian Left',
    compassX: -0.7, compassY: 0.7,
    positions: [
      'Universal Healthcare', 'Public Education', 'Progressive Taxation',
      'Labor Protections', 'Housing as Right', 'Climate Regulation',
      'Anti-Trust Enforcement', 'Public Media',
    ],
  },
  {
    name: 'Libertarian Right',
    compassX: 0.7, compassY: -0.7,
    positions: [
      'Free Markets', 'Property Rights', 'Gun Rights', 'Crypto/Sound Money',
      'Minimal Regulation', 'Voluntary Association', 'Private Education',
      'Self-Defense',
    ],
  },
  {
    name: 'Authoritarian Right',
    compassX: 0.7, compassY: 0.7,
    positions: [
      'National Security', 'Traditional Family', 'Immigration Control',
      'Law and Order', 'Religious Freedom', 'Military Strength',
      'Fiscal Conservatism', 'Border Sovereignty',
    ],
  },
];

// Cross-quadrant shared positions (things people agree on despite ideological distance)
const POLITICAL_SHARED: { title: string; quadrants: string[] }[] = [
  { title: 'Free Speech', quadrants: ['Libertarian Left', 'Libertarian Right'] },
  { title: 'Anti-Corruption', quadrants: ['Libertarian Left', 'Libertarian Right'] },
  { title: 'Education Access', quadrants: ['Authoritarian Left', 'Libertarian Left'] },
  { title: 'Community Safety', quadrants: ['Authoritarian Right', 'Authoritarian Left'] },
  { title: 'Economic Opportunity', quadrants: ['Libertarian Right', 'Authoritarian Right'] },
  { title: 'Environmental Stewardship', quadrants: ['Libertarian Left', 'Authoritarian Left'] },
];

export function makePoliticalCompass(
  personAQuadrant = 'Libertarian Left',
  personBQuadrant = 'Libertarian Right',
): HexGraphData {
  resetSeed(555);
  const nodes: HexNode[] = [];
  const edges: HexEdge[] = [];
  let idx = 0;

  for (const quad of POLITICAL_QUADRANTS) {
    const isA = quad.name === personAQuadrant;
    const isB = quad.name === personBQuadrant;

    for (const position of quad.positions) {
      let owner: HexNode['owner'];
      let affinity: number;

      if (isA && isB) {
        owner = 'shared';
        affinity = 0.5 + seededRandom() * 0.5;
      } else if (isA) {
        owner = 'self';
        affinity = 0.5 + seededRandom() * 0.5;
      } else if (isB) {
        owner = 'other';
        affinity = 0.5 + seededRandom() * 0.5;
      } else {
        // Neither person's primary quadrant — neutral
        owner = seededRandom() > 0.5 ? 'self' : 'other';
        affinity = seededRandom() * 0.35; // low affinity — not their territory
      }

      nodes.push({
        id: `pol-${idx}`,
        title: position,
        affinity,
        affinityLevel: affinityLevelFromScore(affinity),
        owner,
        category: quad.name,
        group: quad.name,
        compassX: quad.compassX + (seededRandom() - 0.5) * 0.4,
        compassY: quad.compassY + (seededRandom() - 0.5) * 0.4,
      });
      idx++;
    }
  }

  // Shared positions bridge quadrants
  for (const shared of POLITICAL_SHARED) {
    const overlap = 0.5 + seededRandom() * 0.5;
    nodes.push({
      id: `pol-shared-${idx}`,
      title: shared.title,
      affinity: overlap,
      affinityLevel: affinityLevelFromScore(overlap),
      owner: 'shared',
      overlapScore: overlap,
      category: 'Common Ground',
      group: 'shared',
    });

    // Connect shared position to nodes in its quadrants
    for (const quadName of shared.quadrants) {
      const quadNode = nodes.find(n => n.group === quadName);
      if (quadNode) {
        edges.push({
          id: `pol-bridge-${idx}-${quadNode.id}`,
          sourceId: `pol-shared-${idx}`,
          targetId: quadNode.id,
          edgeType: 'relates_to',
          confidence: overlap * 0.8,
        });
      }
    }
    idx++;
  }

  // Opposes edges between opposite quadrants
  const oppositions: [string, string][] = [
    ['Libertarian Left', 'Authoritarian Right'],
    ['Authoritarian Left', 'Libertarian Right'],
  ];
  for (const [a, b] of oppositions) {
    const nodeA = nodes.find(n => n.group === a && n.affinity > 0.5);
    const nodeB = nodes.find(n => n.group === b && n.affinity > 0.5);
    if (nodeA && nodeB) {
      edges.push({
        id: `opposes-${nodeA.id}-${nodeB.id}`,
        sourceId: nodeA.id,
        targetId: nodeB.id,
        edgeType: 'opposes',
        confidence: 0.5,
      });
    }
  }

  return { nodes, edges };
}

// =============================================================================
// Scenario 5: Cross-Journey Overlap (Arithmetic → Algebra)
// Fixed: Bridge concepts in own category for proper 3-layer DAG rendering
// =============================================================================

export function makeCrossJourneyOverlap(): HexGraphData {
  resetSeed(999);
  const nodes: HexNode[] = [];
  const edges: HexEdge[] = [];
  let idx = 0;

  // Arithmetic concepts (already mastered — Layer 0 in DAG)
  const arithmeticConcepts = [
    'Number Sense', 'Addition', 'Subtraction', 'Multiplication', 'Division',
    'Place Value',
  ];

  // Bridge concepts — the shared overlap (Layer 1 in DAG)
  const bridgeConcepts = [
    'Order of Operations', 'Number Patterns', 'Fractions', 'Decimals',
    'Word Problems', 'Estimation',
  ];

  // Algebra concepts (not yet started — Layer 2 in DAG)
  const algebraConcepts = [
    'Variables & Expressions', 'Solving Equations', 'Inequalities',
    'Functions', 'Graphing Linear', 'Systems of Equations',
    'Polynomials', 'Factoring', 'Quadratics', 'Exponents',
  ];

  // Layer 0: Arithmetic (mastered, owner: self)
  for (const concept of arithmeticConcepts) {
    nodes.push({
      id: `arith-j-${idx}`,
      title: concept,
      affinity: 0.7 + seededRandom() * 0.3,
      affinityLevel: 'high',
      masteryLevel: 'apply',
      owner: 'self',
      category: 'Arithmetic',
      sourceLabel: 'Arithmetic',
    });
    idx++;
  }

  // Layer 1: Bridge concepts (shared, partially mastered)
  for (const concept of bridgeConcepts) {
    const affinity = 0.4 + seededRandom() * 0.4;
    nodes.push({
      id: `bridge-j-${idx}`,
      title: concept,
      affinity,
      affinityLevel: affinityLevelFromScore(affinity),
      masteryLevel: affinity > 0.6 ? 'understand' : 'remember',
      owner: 'shared',
      overlapScore: 0.7 + seededRandom() * 0.3,
      category: 'Bridge Concepts',
      sourceLabel: 'Shared',
    });
    idx++;
  }

  // Layer 2: Algebra (not started, owner: other)
  for (const concept of algebraConcepts) {
    nodes.push({
      id: `alg-j-${idx}`,
      title: concept,
      affinity: 0,
      affinityLevel: 'unseen',
      masteryLevel: 'not_started',
      owner: 'other',
      category: 'Algebra',
      sourceLabel: 'Algebra',
    });
    idx++;
  }

  // Edges: Arithmetic → requires → Bridge
  for (const bridgeNode of nodes.filter(n => n.category === 'Bridge Concepts')) {
    const arithNode = nodes.find(n => n.category === 'Arithmetic');
    if (arithNode) {
      edges.push({
        id: `req-${arithNode.id}-${bridgeNode.id}`,
        sourceId: arithNode.id,
        targetId: bridgeNode.id,
        edgeType: 'requires',
        confidence: 0.85,
        directed: true,
      });
    }
  }

  // Edges: Bridge → requires → Algebra
  for (const algNode of nodes.filter(n => n.category === 'Algebra')) {
    const bridgeNode = nodes.find(n => n.category === 'Bridge Concepts');
    if (bridgeNode) {
      edges.push({
        id: `req-${bridgeNode.id}-${algNode.id}`,
        sourceId: bridgeNode.id,
        targetId: algNode.id,
        edgeType: 'requires',
        confidence: 0.8,
        directed: true,
        label: 'unlocks',
      });
    }
  }

  return { nodes, edges };
}

// =============================================================================
// Scenario 6: Astronaut Attestation Goal
// =============================================================================

const ASTRONAUT_DOMAINS: { name: string; concepts: string[]; masteryBias: number }[] = [
  {
    name: 'Mathematics',
    concepts: [
      'Calculus I', 'Calculus II', 'Linear Algebra', 'Differential Equations',
      'Statistics', 'Orbital Mechanics Math',
    ],
    masteryBias: 0.5, // some math done
  },
  {
    name: 'Physics',
    concepts: [
      'Classical Mechanics', 'Thermodynamics', 'Electromagnetism', 'Optics',
      'Fluid Dynamics', 'Quantum Basics', 'Astrophysics Intro',
    ],
    masteryBias: 0.3,
  },
  {
    name: 'Aerospace Engineering',
    concepts: [
      'Aerodynamics', 'Propulsion Systems', 'Spacecraft Design', 'Orbital Mechanics',
      'Launch Systems', 'Re-entry Physics', 'Life Support Systems',
      'Thermal Protection', 'Avionics',
    ],
    masteryBias: 0.05, // barely started
  },
  {
    name: 'Flight Training',
    concepts: [
      'Private Pilot License', 'Instrument Rating', 'Multi-Engine Rating',
      'Jet Certification', 'High-Performance Aircraft', 'Test Pilot School',
      'Zero-G Training', 'Simulator Hours (1000+)',
    ],
    masteryBias: 0.0, // haven't started
  },
  {
    name: 'Medical & Fitness',
    concepts: [
      'NASA Class I Physical', 'Vision Requirements', 'Cardiovascular Fitness',
      'G-Force Tolerance', 'Psychological Evaluation', 'Wilderness Survival',
      'SCUBA Certification', 'Spacewalk Training (EVA)',
    ],
    masteryBias: 0.15,
  },
  {
    name: 'Leadership & Operations',
    concepts: [
      'Team Leadership', 'Mission Planning', 'Risk Assessment',
      'Communication Under Stress', 'Cross-Cultural Collaboration',
      'Public Speaking', 'Scientific Research Methods',
    ],
    masteryBias: 0.25,
  },
  {
    name: 'Space Science',
    concepts: [
      'Earth Observation', 'Microgravity Research', 'Space Station Operations',
      'Robotics (Canadarm)', 'Docking Procedures', 'Emergency Protocols',
      'Radiation Biology', 'Space Medicine',
    ],
    masteryBias: 0.02,
  },
];

export function makeAstronautGoal(): HexGraphData {
  resetSeed(2001); // Space Odyssey seed :)
  const nodes: HexNode[] = [];
  const edges: HexEdge[] = [];
  let idx = 0;

  for (const domain of ASTRONAUT_DOMAINS) {
    for (const concept of domain.concepts) {
      const roll = seededRandom();
      const masteryRaw = roll * domain.masteryBias;
      const masteryIdx = Math.min(7, Math.floor(masteryRaw * 8));
      const level = BLOOM_LEVELS[masteryIdx];
      const affinity = masteryRaw;

      nodes.push({
        id: `astro-${idx}`,
        title: concept,
        affinity,
        affinityLevel: affinityLevelFromScore(affinity),
        masteryLevel: level,
        category: domain.name,
        group: domain.name,
      });
      idx++;
    }
  }

  // Specific concept-to-concept prerequisite edges for meaningful lock state
  const specificPrereqs: [string, string][] = [
    // Math → Physics
    ['Calculus I', 'Classical Mechanics'],
    ['Calculus II', 'Thermodynamics'],
    ['Linear Algebra', 'Electromagnetism'],
    ['Differential Equations', 'Fluid Dynamics'],
    ['Statistics', 'Astrophysics Intro'],
    // Physics → Aerospace
    ['Classical Mechanics', 'Aerodynamics'],
    ['Thermodynamics', 'Propulsion Systems'],
    ['Fluid Dynamics', 'Spacecraft Design'],
    ['Electromagnetism', 'Avionics'],
    ['Astrophysics Intro', 'Orbital Mechanics'],
    // Aerospace → Flight
    ['Aerodynamics', 'Private Pilot License'],
    ['Avionics', 'Instrument Rating'],
    ['Propulsion Systems', 'Jet Certification'],
    ['Spacecraft Design', 'Test Pilot School'],
    // Medical → Flight
    ['NASA Class I Physical', 'Private Pilot License'],
    ['Cardiovascular Fitness', 'G-Force Tolerance'],
    ['G-Force Tolerance', 'Zero-G Training'],
    // Leadership → Space Science
    ['Team Leadership', 'Space Station Operations'],
    ['Mission Planning', 'Docking Procedures'],
    ['Risk Assessment', 'Emergency Protocols'],
    // Aerospace → Space Science
    ['Orbital Mechanics', 'Earth Observation'],
    ['Life Support Systems', 'Space Medicine'],
    // Flight → Space Science
    ['Zero-G Training', 'Spacewalk Training (EVA)'],
    ['Simulator Hours (1000+)', 'Robotics (Canadarm)'],
  ];

  const nodeByTitle = new Map(nodes.map(n => [n.title, n]));
  for (const [from, to] of specificPrereqs) {
    const src = nodeByTitle.get(from);
    const tgt = nodeByTitle.get(to);
    if (src && tgt) {
      edges.push({
        id: `astro-prereq-${src.id}-${tgt.id}`,
        sourceId: src.id,
        targetId: tgt.id,
        edgeType: 'requires',
        confidence: 0.9,
        directed: true,
      });
    }
  }

  return { nodes, edges };
}

// =============================================================================
// Scenario 7: Bloom's Progression (all 8 levels side by side)
// =============================================================================

export function makeBloomShowcase(): HexNode[] {
  const levels: HexMasteryLevel[] = [
    'not_started', 'seen', 'remember', 'understand', 'apply', 'analyze', 'evaluate', 'create',
  ];
  const labels: Record<HexMasteryLevel, string> = {
    not_started: 'Not Started',
    seen: 'Seen',
    remember: 'Remember',
    understand: 'Understand',
    apply: 'Apply (Gate)',
    analyze: 'Analyze',
    evaluate: 'Evaluate',
    create: 'Create',
  };

  const nodes: HexNode[] = [];
  let idx = 0;
  for (const level of levels) {
    // 6 hexes per level to show the color clearly
    for (let j = 0; j < 6; j++) {
      nodes.push({
        id: `bloom-${idx}`,
        title: `${labels[level]} ${j + 1}`,
        affinity: levels.indexOf(level) / 7,
        affinityLevel: affinityLevelFromScore(levels.indexOf(level) / 7),
        masteryLevel: level,
        category: labels[level],
      });
      idx++;
    }
  }
  return nodes;
}

// =============================================================================
// Scenario 8: P2P Network
// =============================================================================

export function makeP2PNetwork(): HexGraphData {
  resetSeed(42);
  const RELATIONSHIPS = ['family', 'friend', 'colleague', 'mentor', 'neighbor', 'acquaintance'];
  const INTIMACY_AFFINITY: Record<string, number> = {
    intimate: 0.95, trusted: 0.75, connection: 0.55, acquaintance: 0.35, recognition: 0.15,
  };

  const agents = [
    { name: 'You', group: 'self', isCenter: true },
    { name: 'Sarah', group: 'family' },
    { name: 'James', group: 'family' },
    { name: 'Mom', group: 'family' },
    { name: 'Dad', group: 'family' },
    { name: 'Alex', group: 'friend' },
    { name: 'Jordan', group: 'friend' },
    { name: 'Riley', group: 'friend' },
    { name: 'Casey', group: 'colleague' },
    { name: 'Morgan', group: 'colleague' },
    { name: 'Taylor', group: 'colleague' },
    { name: 'Dr. Chen', group: 'mentor' },
    { name: 'Prof. Park', group: 'mentor' },
    { name: 'Sam (next door)', group: 'neighbor' },
    { name: 'Pat (down street)', group: 'neighbor' },
    { name: 'Kim', group: 'acquaintance' },
    { name: 'Chris', group: 'acquaintance' },
    { name: 'Jamie', group: 'acquaintance' },
  ];

  const nodes: HexNode[] = agents.map((agent, i) => {
    const intimacyKeys = Object.keys(INTIMACY_AFFINITY);
    const intimacy = agent.group === 'self' ? 'intimate'
      : agent.group === 'family' ? pick(intimacyKeys.slice(0, 2))
        : agent.group === 'friend' ? pick(intimacyKeys.slice(1, 3))
          : pick(intimacyKeys.slice(2));
    const affinity = INTIMACY_AFFINITY[intimacy] || 0.3;

    return {
      id: `agent-${i}`,
      title: agent.name,
      affinity,
      affinityLevel: affinityLevelFromScore(affinity),
      group: agent.group,
      owner: agent.group === 'self' ? 'self' as const : undefined,
      subtitle: agent.group,
    };
  });

  // Connect everyone to "You" with varying confidence
  const edges: HexEdge[] = nodes.slice(1).map((node, i) => ({
    id: `social-${i}`,
    sourceId: 'agent-0',
    targetId: node.id,
    edgeType: 'social' as const,
    confidence: node.affinity * 0.9,
  }));

  // Add some inter-agent connections
  edges.push(
    { id: 'social-family-1', sourceId: 'agent-3', targetId: 'agent-4', edgeType: 'social', confidence: 0.95 }, // Mom-Dad
    { id: 'social-family-2', sourceId: 'agent-1', targetId: 'agent-2', edgeType: 'social', confidence: 0.8 }, // Sarah-James
    { id: 'social-work', sourceId: 'agent-8', targetId: 'agent-9', edgeType: 'social', confidence: 0.6 }, // Casey-Morgan
  );

  return { nodes, edges };
}

// =============================================================================
// Scenario 9: Bloom's Staircase — Curriculum Progression with Gate Line
// =============================================================================

const STAIRCASE_CATEGORIES: { name: string; concepts: { title: string; bloom: HexMasteryLevel }[] }[] = [
  {
    name: 'Number Sense',
    concepts: [
      { title: 'Counting to 100', bloom: 'create' },
      { title: 'Skip Counting', bloom: 'evaluate' },
      { title: 'Place Value', bloom: 'apply' },
      { title: 'Comparing Numbers', bloom: 'apply' },
      { title: 'Number Line', bloom: 'understand' },
      { title: 'Rounding', bloom: 'remember' },
    ],
  },
  {
    name: 'Addition & Subtraction',
    concepts: [
      { title: 'Single Digit Addition', bloom: 'create' },
      { title: 'Addition with Carrying', bloom: 'analyze' },
      { title: 'Single Digit Subtraction', bloom: 'apply' },
      { title: 'Subtraction with Borrowing', bloom: 'apply' },
      { title: 'Mental Math Strategies', bloom: 'understand' },
      { title: 'Multi-digit Addition', bloom: 'understand' },
      { title: 'Word Problems: Add/Sub', bloom: 'remember' },
      { title: 'Missing Addend', bloom: 'seen' },
    ],
  },
  {
    name: 'Multiplication',
    concepts: [
      { title: 'Multiplication as Groups', bloom: 'apply' },
      { title: 'Times Tables 2-5', bloom: 'apply' },
      { title: 'Times Tables 6-12', bloom: 'understand' },
      { title: 'Multiplication Properties', bloom: 'understand' },
      { title: 'Multi-digit Multiplication', bloom: 'remember' },
      { title: 'Factors & Multiples', bloom: 'seen' },
      { title: 'Prime Numbers', bloom: 'not_started' },
    ],
  },
  {
    name: 'Fractions',
    concepts: [
      { title: 'What is a Fraction?', bloom: 'understand' },
      { title: 'Fractions on Number Line', bloom: 'remember' },
      { title: 'Comparing Fractions', bloom: 'seen' },
      { title: 'Equivalent Fractions', bloom: 'not_started' },
      { title: 'Adding Fractions', bloom: 'not_started' },
      { title: 'Mixed Numbers', bloom: 'not_started' },
    ],
  },
  {
    name: 'Geometry',
    concepts: [
      { title: 'Basic Shapes', bloom: 'apply' },
      { title: 'Perimeter', bloom: 'remember' },
      { title: 'Area of Rectangles', bloom: 'seen' },
      { title: 'Angles', bloom: 'not_started' },
      { title: 'Symmetry', bloom: 'not_started' },
    ],
  },
];

export function makeBloomStaircase(): HexGraphData {
  resetSeed(314);
  const nodes: HexNode[] = [];
  const edges: HexEdge[] = [];
  let idx = 0;

  for (const cat of STAIRCASE_CATEGORIES) {
    let prevId: string | null = null;
    for (const concept of cat.concepts) {
      const bloomIdx = BLOOM_LEVELS.indexOf(concept.bloom);
      const affinity = bloomIdx / 7;

      const id = `stair-${idx}`;
      nodes.push({
        id,
        title: concept.title,
        affinity,
        affinityLevel: affinityLevelFromScore(affinity),
        masteryLevel: concept.bloom,
        category: cat.name,
      });

      // Sequential prereqs within category
      if (prevId) {
        edges.push({
          id: `stair-seq-${prevId}-${id}`,
          sourceId: prevId,
          targetId: id,
          edgeType: 'follows',
          confidence: 0.8,
          directed: true,
        });
      }
      prevId = id;
      idx++;
    }
  }

  // Cross-category prereqs
  const crossPrereqs: [string, string][] = [
    ['Single Digit Addition', 'Multiplication as Groups'],
    ['Times Tables 2-5', 'What is a Fraction?'],
    ['Comparing Numbers', 'Comparing Fractions'],
    ['Place Value', 'Multi-digit Multiplication'],
    ['Number Line', 'Fractions on Number Line'],
    ['Multiplication as Groups', 'Area of Rectangles'],
  ];

  const nodeByTitle = new Map(nodes.map(n => [n.title, n]));
  for (const [from, to] of crossPrereqs) {
    const src = nodeByTitle.get(from);
    const tgt = nodeByTitle.get(to);
    if (src && tgt) {
      edges.push({
        id: `stair-xreq-${src.id}-${tgt.id}`,
        sourceId: src.id,
        targetId: tgt.id,
        edgeType: 'depends_on',
        confidence: 0.9,
        directed: true,
      });
    }
  }

  return { nodes, edges };
}

// =============================================================================
// Scenario 10: Love Map — River & Sky (Relationship Intimacy Domains)
// =============================================================================

interface LoveMapDomain {
  category: string;
  riverConcepts: string[];
  skyConcepts: string[];
  sharedConcepts: string[];
}

const LOVE_DOMAINS: LoveMapDomain[] = [
  {
    category: 'Finances',
    riverConcepts: ['Monthly Budgeting', 'Retirement Planning', 'Tax Strategy'],
    skyConcepts: ['Impulse Spending Awareness', 'Charity Giving'],
    sharedConcepts: ['Shared Savings Goal', 'Emergency Fund'],
  },
  {
    category: 'Career',
    riverConcepts: ['Professional Networking', 'Mentoring Others', 'Industry Conferences'],
    skyConcepts: ['Creative Side Projects', 'Work-Life Boundaries'],
    sharedConcepts: ['Couple Career Support', 'Relocation Decisions'],
  },
  {
    category: 'Health',
    riverConcepts: ['Nutrition Planning', 'Gym Routine', 'Sleep Hygiene'],
    skyConcepts: ['Mental Health Check-ins', 'Meditation Practice'],
    sharedConcepts: ['Family Health Goals', 'Cooking Together'],
  },
  {
    category: 'Parenting',
    riverConcepts: ['Homework Help', 'Sports Coaching'],
    skyConcepts: ['Bedtime Routines', 'School Advocacy', 'Emotional Coaching'],
    sharedConcepts: ['Discipline Approach', 'Family Traditions'],
  },
  {
    category: 'Intimacy',
    riverConcepts: ['Physical Affection', 'Surprise Gestures'],
    skyConcepts: ['Date Night Planning', 'Love Languages', 'Vulnerability Sharing'],
    sharedConcepts: ['Quality Time Rituals', 'Conflict Resolution'],
  },
  {
    category: 'Shared Dreams',
    riverConcepts: ['Home Workshop', 'Travel Bucket List'],
    skyConcepts: ['Garden Design', 'Community Volunteering'],
    sharedConcepts: ['Home Renovation Plan', 'Anniversary Traditions', 'Retirement Vision'],
  },
];

export function makeLoveMap(): HexGraphData {
  resetSeed(808);
  const nodes: HexNode[] = [];
  const edges: HexEdge[] = [];
  let idx = 0;

  // Center node: the relationship itself
  nodes.push({
    id: 'love-center',
    title: 'River & Sky',
    affinity: 1,
    affinityLevel: 'high',
    owner: 'self',
    category: 'Us',
    group: 'self',
  });

  for (const domain of LOVE_DOMAINS) {
    // River's concepts
    for (const concept of domain.riverConcepts) {
      const affinity = 0.6 + seededRandom() * 0.4;
      nodes.push({
        id: `love-r-${idx}`,
        title: concept,
        affinity,
        affinityLevel: affinityLevelFromScore(affinity),
        owner: 'self',
        category: domain.category,
        sourceLabel: 'River',
      });
      idx++;
    }

    // Sky's concepts
    for (const concept of domain.skyConcepts) {
      const affinity = 0.6 + seededRandom() * 0.4;
      nodes.push({
        id: `love-s-${idx}`,
        title: concept,
        affinity,
        affinityLevel: affinityLevelFromScore(affinity),
        owner: 'other',
        category: domain.category,
        sourceLabel: 'Sky',
      });
      idx++;
    }

    // Shared concepts
    for (const concept of domain.sharedConcepts) {
      const affinity = 0.5 + seededRandom() * 0.5;
      nodes.push({
        id: `love-sh-${idx}`,
        title: concept,
        affinity,
        affinityLevel: affinityLevelFromScore(affinity),
        owner: 'shared',
        overlapScore: 0.6 + seededRandom() * 0.4,
        category: domain.category,
        sourceLabel: 'River & Sky',
      });
      idx++;
    }
  }

  // Teaches edges: River teaches Sky
  const riverTeachesSky: [string, string][] = [
    ['Retirement Planning', 'Shared Savings Goal'],
    ['Nutrition Planning', 'Family Health Goals'],
    ['Professional Networking', 'Couple Career Support'],
    ['Monthly Budgeting', 'Emergency Fund'],
  ];

  for (const [from, to] of riverTeachesSky) {
    const src = nodes.find(n => n.title === from && n.owner === 'self');
    const tgt = nodes.find(n => n.title === to && n.owner === 'shared');
    if (src && tgt) {
      edges.push({
        id: `teaches-r-${src.id}-${tgt.id}`,
        sourceId: src.id,
        targetId: tgt.id,
        edgeType: 'teaches',
        confidence: 0.75,
        directed: true,
        label: 'River teaches',
      });
    }
  }

  // Teaches edges: Sky teaches River
  const skyTeachesRiver: [string, string][] = [
    ['Love Languages', 'Quality Time Rituals'],
    ['Emotional Coaching', 'Discipline Approach'],
    ['Mental Health Check-ins', 'Cooking Together'],
    ['Date Night Planning', 'Conflict Resolution'],
  ];

  for (const [from, to] of skyTeachesRiver) {
    const src = nodes.find(n => n.title === from && n.owner === 'other');
    const tgt = nodes.find(n => n.title === to && n.owner === 'shared');
    if (src && tgt) {
      edges.push({
        id: `teaches-s-${src.id}-${tgt.id}`,
        sourceId: src.id,
        targetId: tgt.id,
        edgeType: 'teaches',
        confidence: 0.75,
        directed: true,
        label: 'Sky teaches',
      });
    }
  }

  // Social edges from center to shared concepts
  for (const n of nodes.filter(n => n.owner === 'shared')) {
    edges.push({
      id: `love-bond-${n.id}`,
      sourceId: 'love-center',
      targetId: n.id,
      edgeType: 'social',
      confidence: n.overlapScore ?? 0.7,
    });
  }

  return { nodes, edges };
}

// =============================================================================
// Scenario 11: Political Fault Lines — Bilateral Policy Tension
// =============================================================================

interface PolicyDomain {
  name: string;
  leftPositions: string[];
  rightPositions: string[];
  commonGround: string[];
  oppositions: [string, string][]; // [left position, right position]
}

const POLICY_DOMAINS: PolicyDomain[] = [
  {
    name: 'Healthcare',
    leftPositions: ['Universal Coverage', 'Public Option', 'Drug Price Controls'],
    rightPositions: ['Market Competition', 'Health Savings Accounts', 'Deregulation'],
    commonGround: ['Mental Health Funding'],
    oppositions: [['Universal Coverage', 'Market Competition'], ['Drug Price Controls', 'Deregulation']],
  },
  {
    name: 'Economy',
    leftPositions: ['Progressive Taxation', 'Worker Protections', 'Union Rights'],
    rightPositions: ['Flat Tax', 'Business Deregulation', 'Free Trade'],
    commonGround: ['Small Business Support', 'Job Training'],
    oppositions: [['Progressive Taxation', 'Flat Tax'], ['Worker Protections', 'Business Deregulation']],
  },
  {
    name: 'Environment',
    leftPositions: ['Green New Deal', 'Carbon Tax', 'Renewable Mandate'],
    rightPositions: ['Energy Independence', 'Nuclear Power', 'Market Solutions'],
    commonGround: ['Clean Water', 'Conservation'],
    oppositions: [['Carbon Tax', 'Market Solutions'], ['Renewable Mandate', 'Energy Independence']],
  },
  {
    name: 'Immigration',
    leftPositions: ['Path to Citizenship', 'DACA Protections', 'Refugee Resettlement'],
    rightPositions: ['Border Security', 'Merit-Based System', 'E-Verify Mandate'],
    commonGround: ['Visa Modernization'],
    oppositions: [['Path to Citizenship', 'Merit-Based System']],
  },
  {
    name: 'Education',
    leftPositions: ['Universal Pre-K', 'Student Debt Relief', 'Teacher Pay'],
    rightPositions: ['School Choice', 'Charter Schools', 'Local Control'],
    commonGround: ['STEM Investment', 'Vocational Training'],
    oppositions: [['Universal Pre-K', 'School Choice'], ['Student Debt Relief', 'Local Control']],
  },
  {
    name: 'Criminal Justice',
    leftPositions: ['Police Reform', 'Restorative Justice', 'Decriminalization'],
    rightPositions: ['Law & Order', 'Tough Sentencing', 'Victim Rights'],
    commonGround: ['Reentry Programs'],
    oppositions: [['Police Reform', 'Law & Order'], ['Decriminalization', 'Tough Sentencing']],
  },
];

export function makePoliticalFaultLines(): HexGraphData {
  resetSeed(1776);
  const nodes: HexNode[] = [];
  const edges: HexEdge[] = [];
  let idx = 0;

  for (const domain of POLICY_DOMAINS) {
    // Left positions
    for (const pos of domain.leftPositions) {
      const affinity = 0.5 + seededRandom() * 0.5;
      nodes.push({
        id: `pol-l-${idx}`,
        title: pos,
        affinity,
        affinityLevel: affinityLevelFromScore(affinity),
        owner: 'self',
        category: domain.name,
        group: 'Civic Left',
        sourceLabel: 'Civic Left',
        compassX: -0.5 - seededRandom() * 0.4,
        compassY: (seededRandom() - 0.5) * 1.4,
      });
      idx++;
    }

    // Right positions
    for (const pos of domain.rightPositions) {
      const affinity = 0.5 + seededRandom() * 0.5;
      nodes.push({
        id: `pol-r-${idx}`,
        title: pos,
        affinity,
        affinityLevel: affinityLevelFromScore(affinity),
        owner: 'other',
        category: domain.name,
        group: 'Civic Right',
        sourceLabel: 'Civic Right',
        compassX: 0.5 + seededRandom() * 0.4,
        compassY: (seededRandom() - 0.5) * 1.4,
      });
      idx++;
    }

    // Common ground
    for (const pos of domain.commonGround) {
      const affinity = 0.4 + seededRandom() * 0.3;
      const nodeId = `pol-cg-${idx}`;
      nodes.push({
        id: nodeId,
        title: pos,
        affinity,
        affinityLevel: affinityLevelFromScore(affinity),
        owner: 'shared',
        overlapScore: 0.5 + seededRandom() * 0.3,
        category: domain.name,
        group: 'Common Ground',
        sourceLabel: 'Common Ground',
        compassX: (seededRandom() - 0.5) * 0.4,
        compassY: (seededRandom() - 0.5) * 1.4,
      });

      // Connect common ground to both sides
      const leftNode = nodes.find(n => n.group === 'Civic Left' && n.category === domain.name);
      const rightNode = nodes.find(n => n.group === 'Civic Right' && n.category === domain.name);
      if (leftNode) {
        edges.push({
          id: `cg-l-${nodeId}-${leftNode.id}`,
          sourceId: nodeId,
          targetId: leftNode.id,
          edgeType: 'relates_to',
          confidence: 0.6,
        });
      }
      if (rightNode) {
        edges.push({
          id: `cg-r-${nodeId}-${rightNode.id}`,
          sourceId: nodeId,
          targetId: rightNode.id,
          edgeType: 'relates_to',
          confidence: 0.6,
        });
      }
      idx++;
    }

    // Opposition edges (tension lines)
    for (const [leftTitle, rightTitle] of domain.oppositions) {
      const leftNode = nodes.find(n => n.title === leftTitle && n.group === 'Civic Left');
      const rightNode = nodes.find(n => n.title === rightTitle && n.group === 'Civic Right');
      if (leftNode && rightNode) {
        edges.push({
          id: `opposes-${leftNode.id}-${rightNode.id}`,
          sourceId: leftNode.id,
          targetId: rightNode.id,
          edgeType: 'opposes',
          confidence: 0.7 + seededRandom() * 0.3,
        });
      }
    }
  }

  // Chain domains vertically: first node of each domain → first node of next domain
  // This creates DAG layers (one layer per policy domain)
  for (let d = 0; d < POLICY_DOMAINS.length - 1; d++) {
    const thisDomain = POLICY_DOMAINS[d].name;
    const nextDomain = POLICY_DOMAINS[d + 1].name;
    const thisNode = nodes.find(n => n.category === thisDomain);
    const nextNode = nodes.find(n => n.category === nextDomain);
    if (thisNode && nextNode) {
      edges.push({
        id: `domain-chain-${thisNode.id}-${nextNode.id}`,
        sourceId: thisNode.id,
        targetId: nextNode.id,
        edgeType: 'follows',
        confidence: 0.3,
        directed: true,
      });
    }
  }

  return { nodes, edges };
}
