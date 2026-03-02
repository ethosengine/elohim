/**
 * Hexagon Grid — Layout Algorithms
 *
 * Pure-function layout module. No Angular dependencies.
 * Each layout takes (nodes, edges, bounds, options) and returns positioned nodes.
 *
 * Five modes:
 *   grid    — Khan-style unit rows with category tessellation
 *   dag     — Sugiyama-inspired skill tree (top→bottom prerequisite flow)
 *   radial  — Center-out rings (hub & spoke for networks/love maps)
 *   scatter — Semantic axes (compassX/Y → canvas position)
 *   tree    — Organic diagonal hex-grid walk per category (50% scale)
 */

import { HexNode, HexEdge, HexLayoutMode } from './hexagon-grid.model';

// =============================================================================
// Types
// =============================================================================

export interface LayoutBounds {
  width: number;
  paddingTop: number;
  paddingLeft: number;
  paddingRight: number;
}

export interface LayoutOptions {
  hexRadius: number;
  hexGap: number;
  hexWidth: number;
  hexHeight: number;
  xStep: number;
  yStep: number;
  itemsPerRow: number;
}

export interface LayoutResult {
  nodes: HexNode[];
  totalHeight: number;
  /** Category clusters for drawing background hulls. */
  clusters: ClusterInfo[];
  /** Row dividers for grid layout category headers. */
  dividers: DividerInfo[];
}

export interface ClusterInfo {
  category: string;
  nodes: HexNode[];
  /** Convex hull points (expanded by hexRadius + padding). */
  hull: { x: number; y: number }[];
  /** Dominant color from the cluster's first node (for background wash). */
  dominantColor?: string;
}

export interface DividerInfo {
  label: string;
  y: number;
  x: number;
  width: number;
}

export type LayoutFn = (
  nodes: HexNode[],
  edges: HexEdge[],
  bounds: LayoutBounds,
  opts: LayoutOptions,
) => LayoutResult;

// =============================================================================
// Layout Registry
// =============================================================================

export const LAYOUTS: Record<HexLayoutMode, LayoutFn> = {
  grid: gridLayout,
  dag: dagLayout,
  radial: radialLayout,
  scatter: scatterLayout,
  tree: treeLayout,
};

// =============================================================================
// Grid Layout (enhanced with category rows + tight tessellation)
// =============================================================================

function gridLayout(
  nodes: HexNode[],
  _edges: HexEdge[],
  bounds: LayoutBounds,
  opts: LayoutOptions,
): LayoutResult {
  if (!nodes.length) return { nodes: [], totalHeight: 0, clusters: [], dividers: [] };

  const hasCategories = nodes.some(n => n.category);

  if (hasCategories) {
    return gridLayoutWithCategories(nodes, bounds, opts);
  }

  return gridLayoutFlat(nodes, bounds, opts);
}

/** Original flat honeycomb layout (backward compatible). */
function gridLayoutFlat(
  nodes: HexNode[],
  bounds: LayoutBounds,
  opts: LayoutOptions,
): LayoutResult {
  const maxItemsPerRow = Math.floor((bounds.width - opts.hexWidth) / opts.xStep);
  const safeItemsPerRow = Math.max(3, Math.min(opts.itemsPerRow, maxItemsPerRow));
  const totalGridWidth = safeItemsPerRow * opts.xStep + opts.hexWidth / 2;
  const startX = (bounds.width - totalGridWidth) / 2 + opts.hexWidth / 2;
  const startY = bounds.paddingTop;

  let maxRow = 0;

  const positioned = nodes.map((node, index) => {
    let remaining = index;
    let currentRow = 0;

    while (true) {
      const isOdd = currentRow % 2 !== 0;
      const capacity = isOdd ? safeItemsPerRow - 1 : safeItemsPerRow;
      if (remaining < capacity) break;
      remaining -= capacity;
      currentRow++;
    }

    if (currentRow > maxRow) maxRow = currentRow;

    const xOffset = (currentRow % 2) * (opts.xStep / 2);
    const x = startX + remaining * opts.xStep + xOffset;
    const y = startY + currentRow * opts.yStep;

    return { ...node, x, y };
  });

  const totalHeight = startY + maxRow * opts.yStep + opts.hexHeight / 2;

  return { nodes: positioned, totalHeight, clusters: [], dividers: [] };
}

/** Khan-style unit rows: group by category, tight tessellation within, dividers between. */
function gridLayoutWithCategories(
  nodes: HexNode[],
  bounds: LayoutBounds,
  opts: LayoutOptions,
): LayoutResult {
  // Group nodes by category (preserve order of first appearance)
  const categoryOrder: string[] = [];
  const categoryGroups = new Map<string, HexNode[]>();

  for (const node of nodes) {
    const cat = node.category || 'Uncategorized';
    if (!categoryGroups.has(cat)) {
      categoryOrder.push(cat);
      categoryGroups.set(cat, []);
    }
    categoryGroups.get(cat)!.push(node);
  }

  // Gap within categories, larger gap between
  const tightGap = 3;
  const tightXStep = opts.hexWidth + tightGap;
  const tightYStep = opts.hexHeight * 0.75 + tightGap;
  const categoryGap = 38; // Space between category blocks (includes divider)

  const maxItemsPerRow = Math.floor((bounds.width - opts.hexWidth) / tightXStep);
  const safeItemsPerRow = Math.max(3, Math.min(opts.itemsPerRow, maxItemsPerRow));
  const totalGridWidth = safeItemsPerRow * tightXStep + opts.hexWidth / 2;
  const startX = (bounds.width - totalGridWidth) / 2 + opts.hexWidth / 2;

  let currentY = bounds.paddingTop;
  const positioned: HexNode[] = [];
  const clusters: ClusterInfo[] = [];
  const dividers: DividerInfo[] = [];

  for (const cat of categoryOrder) {
    const catNodes = categoryGroups.get(cat)!;

    // Add divider
    dividers.push({
      label: cat,
      y: currentY,
      x: startX - opts.hexWidth / 2,
      width: totalGridWidth,
    });

    currentY += 40; // Space for label text (must exceed hexRadius so hexes clear the label)

    // Lay out category nodes in tight honeycomb
    let maxRowInCat = 0;
    const catPositioned: HexNode[] = [];

    for (let i = 0; i < catNodes.length; i++) {
      let remaining = i;
      let currentRow = 0;

      while (true) {
        const isOdd = currentRow % 2 !== 0;
        const capacity = isOdd ? safeItemsPerRow - 1 : safeItemsPerRow;
        if (remaining < capacity) break;
        remaining -= capacity;
        currentRow++;
      }

      if (currentRow > maxRowInCat) maxRowInCat = currentRow;

      const xOffset = (currentRow % 2) * (tightXStep / 2);
      const x = startX + remaining * tightXStep + xOffset;
      const y = currentY + currentRow * tightYStep;

      const p = { ...catNodes[i], x, y };
      catPositioned.push(p);
      positioned.push(p);
    }

    // Build cluster info for background hull
    if (catPositioned.length > 0) {
      clusters.push({
        category: cat,
        nodes: catPositioned,
        hull: computeConvexHull(catPositioned, opts.hexRadius + 4),
      });
    }

    currentY += maxRowInCat * tightYStep + opts.hexHeight / 2 + categoryGap;
  }

  return { nodes: positioned, totalHeight: currentY, clusters, dividers };
}

// =============================================================================
// DAG Layout (Sugiyama-inspired skill tree)
// =============================================================================

function dagLayout(
  nodes: HexNode[],
  edges: HexEdge[],
  bounds: LayoutBounds,
  opts: LayoutOptions,
): LayoutResult {
  if (!nodes.length) return { nodes: [], totalHeight: 0, clusters: [], dividers: [] };

  // 1. Build directed adjacency from prerequisite edges
  const dagEdgeTypes = new Set(['requires', 'depends_on', 'follows', 'contains']);
  const directedEdges = edges.filter(e => dagEdgeTypes.has(e.edgeType));

  const nodeMap = new Map(nodes.map(n => [n.id, n]));
  const inDegree = new Map<string, number>();
  const children = new Map<string, string[]>();
  const parents = new Map<string, string[]>();

  for (const n of nodes) {
    inDegree.set(n.id, 0);
    children.set(n.id, []);
    parents.set(n.id, []);
  }

  for (const e of directedEdges) {
    if (!nodeMap.has(e.sourceId) || !nodeMap.has(e.targetId)) continue;
    children.get(e.sourceId)!.push(e.targetId);
    parents.get(e.targetId)!.push(e.sourceId);
    inDegree.set(e.targetId, (inDegree.get(e.targetId) || 0) + 1);
  }

  // 2. Topological sort (Kahn's algorithm) → assign layers
  const layers = new Map<string, number>();
  const queue: string[] = [];

  for (const [id, deg] of inDegree) {
    if (deg === 0) {
      queue.push(id);
      layers.set(id, 0);
    }
  }

  // If no roots found (all nodes have incoming edges = cycle), pick first node
  if (queue.length === 0 && nodes.length > 0) {
    queue.push(nodes[0].id);
    layers.set(nodes[0].id, 0);
  }

  let head = 0;
  while (head < queue.length) {
    const current = queue[head++];
    const currentLayer = layers.get(current) || 0;

    for (const childId of children.get(current) || []) {
      const childLayer = Math.max(layers.get(childId) || 0, currentLayer + 1);
      layers.set(childId, childLayer);

      const newDeg = (inDegree.get(childId) || 1) - 1;
      inDegree.set(childId, newDeg);
      if (newDeg <= 0 && !queue.includes(childId)) {
        queue.push(childId);
      }
    }
  }

  // Orphan nodes (not reached by toposort) go to layer 0
  for (const n of nodes) {
    if (!layers.has(n.id)) {
      layers.set(n.id, 0);
    }
  }

  // 3. Group nodes by layer
  const layerGroups = new Map<number, HexNode[]>();
  let maxLayer = 0;

  for (const n of nodes) {
    const layer = layers.get(n.id) || 0;
    if (layer > maxLayer) maxLayer = layer;
    if (!layerGroups.has(layer)) layerGroups.set(layer, []);
    layerGroups.get(layer)!.push(n);
  }

  // 4. Sort within layers: by category first, then barycenter ordering
  for (let layer = 0; layer <= maxLayer; layer++) {
    const layerNodes = layerGroups.get(layer) || [];

    // Sort by category first
    layerNodes.sort((a, b) => (a.category || '').localeCompare(b.category || ''));

    // Barycenter ordering: sort by average X position of parents
    if (layer > 0) {
      const barycenter = new Map<string, number>();
      for (const n of layerNodes) {
        const parentNodes = (parents.get(n.id) || [])
          .map(pid => nodeMap.get(pid))
          .filter(p => p?.x !== undefined);
        if (parentNodes.length > 0) {
          const avgX = parentNodes.reduce((sum, p) => sum + (p!.x || 0), 0) / parentNodes.length;
          barycenter.set(n.id, avgX);
        }
      }

      // Stable sort: nodes with barycenter sort by it; others keep category order
      layerNodes.sort((a, b) => {
        const catCmp = (a.category || '').localeCompare(b.category || '');
        if (catCmp !== 0) return catCmp;
        const ba = barycenter.get(a.id);
        const bb = barycenter.get(b.id);
        if (ba !== undefined && bb !== undefined) return ba - bb;
        return 0;
      });
    }

    layerGroups.set(layer, layerNodes);
  }

  // 5. Position assignment
  const layerSpacing = opts.hexHeight * 2.8;
  const nodeSpacing = opts.hexWidth * 1.4;
  const startY = bounds.paddingTop + 20;

  const positioned: HexNode[] = [];
  const clusters: ClusterInfo[] = [];
  const dividers: DividerInfo[] = [];

  // Track categories for cluster info
  const categoryNodes = new Map<string, HexNode[]>();

  for (let layer = 0; layer <= maxLayer; layer++) {
    const layerNodes = layerGroups.get(layer) || [];
    const layerWidth = layerNodes.length * nodeSpacing;
    const layerStartX = (bounds.width - layerWidth) / 2 + nodeSpacing / 2;
    const y = startY + layer * layerSpacing;

    for (let i = 0; i < layerNodes.length; i++) {
      const x = layerStartX + i * nodeSpacing;
      const p = { ...layerNodes[i], x, y };
      positioned.push(p);

      // Update the nodeMap so barycenter can reference positions
      nodeMap.set(p.id, p);

      // Track for clusters
      const cat = p.category || '';
      if (cat) {
        if (!categoryNodes.has(cat)) categoryNodes.set(cat, []);
        categoryNodes.get(cat)!.push(p);
      }
    }
  }

  // Build cluster hulls
  for (const [cat, catNodes] of categoryNodes) {
    if (catNodes.length >= 2) {
      clusters.push({
        category: cat,
        nodes: catNodes,
        hull: computeConvexHull(catNodes, opts.hexRadius + 6),
      });
    }
  }

  const totalHeight = startY + maxLayer * layerSpacing + opts.hexHeight;

  return { nodes: positioned, totalHeight, clusters, dividers };
}

// =============================================================================
// Radial Layout (center-out rings)
// =============================================================================

function radialLayout(
  nodes: HexNode[],
  edges: HexEdge[],
  bounds: LayoutBounds,
  opts: LayoutOptions,
): LayoutResult {
  if (!nodes.length) return { nodes: [], totalHeight: 0, clusters: [], dividers: [] };

  // 1. Find center node
  const centerNode =
    nodes.find(n => n.owner === 'self') ||
    nodes.find(n => n.group === 'self') ||
    nodes.reduce((best, n) => (n.affinity > best.affinity ? n : best), nodes[0]);

  // 2. Assign rings
  const rings = new Map<string, number>();
  rings.set(centerNode.id, 0);

  // Strategy depends on data: owner-based or edge-based
  const hasOwners = nodes.some(n => n.owner);

  if (hasOwners) {
    // Owner-based rings: shared=0 (around center), self=1, other=2
    for (const n of nodes) {
      if (n.id === centerNode.id) continue;
      if (n.owner === 'shared') rings.set(n.id, 1);
      else if (n.owner === 'self') rings.set(n.id, 2);
      else if (n.owner === 'other') rings.set(n.id, 3);
      else rings.set(n.id, 2);
    }
  } else {
    // BFS from center using edges
    const adj = new Map<string, string[]>();
    for (const n of nodes) adj.set(n.id, []);
    for (const e of edges) {
      if (adj.has(e.sourceId) && adj.has(e.targetId)) {
        adj.get(e.sourceId)!.push(e.targetId);
        adj.get(e.targetId)!.push(e.sourceId);
      }
    }

    const bfsQueue = [centerNode.id];
    let bfsHead = 0;
    while (bfsHead < bfsQueue.length) {
      const current = bfsQueue[bfsHead++];
      const currentRing = rings.get(current) || 0;
      for (const neighbor of adj.get(current) || []) {
        if (!rings.has(neighbor)) {
          rings.set(neighbor, currentRing + 1);
          bfsQueue.push(neighbor);
        }
      }
    }

    // Orphans → outermost ring
    const maxRing = Math.max(...Array.from(rings.values()), 1);
    for (const n of nodes) {
      if (!rings.has(n.id)) rings.set(n.id, maxRing + 1);
    }
  }

  // 3. Group by ring and sort within rings by group/category
  const ringGroups = new Map<number, HexNode[]>();
  for (const n of nodes) {
    const ring = rings.get(n.id) || 0;
    if (!ringGroups.has(ring)) ringGroups.set(ring, []);
    ringGroups.get(ring)!.push(n);
  }

  // Sort within each ring by group then category for arc clustering
  for (const [, ringNodes] of ringGroups) {
    ringNodes.sort((a, b) => {
      const groupCmp = (a.group || '').localeCompare(b.group || '');
      if (groupCmp !== 0) return groupCmp;
      return (a.category || '').localeCompare(b.category || '');
    });
  }

  // 4. Position calculation
  const maxRing = Math.max(...Array.from(rings.values()));
  const ringSpacing = opts.hexRadius * 4.5;
  const centerX = bounds.width / 2;
  const centerY = bounds.paddingTop + (maxRing + 1) * ringSpacing;

  const positioned: HexNode[] = [];
  const clusters: ClusterInfo[] = [];

  // Track groups for cluster hulls
  const groupNodes = new Map<string, HexNode[]>();

  for (const [ring, ringNodes] of ringGroups) {
    if (ring === 0) {
      // Center node
      for (const n of ringNodes) {
        const p = { ...n, x: centerX, y: centerY, scale: 1.5 };
        positioned.push(p);
        if (n.group) {
          if (!groupNodes.has(n.group)) groupNodes.set(n.group, []);
          groupNodes.get(n.group)!.push(p);
        }
      }
    } else {
      const radius = ring * ringSpacing;
      const angleOffset = (ring * 0.3); // Stagger rings to avoid radial alignment
      const count = ringNodes.length;

      for (let i = 0; i < count; i++) {
        const angle = angleOffset + (i / count) * Math.PI * 2;
        const x = centerX + radius * Math.cos(angle);
        const y = centerY + radius * Math.sin(angle);
        const p = { ...ringNodes[i], x, y };
        positioned.push(p);

        if (ringNodes[i].group) {
          if (!groupNodes.has(ringNodes[i].group!)) groupNodes.set(ringNodes[i].group!, []);
          groupNodes.get(ringNodes[i].group!)!.push(p);
        }
      }
    }
  }

  // Build cluster hulls for groups
  for (const [group, gNodes] of groupNodes) {
    if (gNodes.length >= 2) {
      clusters.push({
        category: group,
        nodes: gNodes,
        hull: computeConvexHull(gNodes, opts.hexRadius + 4),
      });
    }
  }

  const totalHeight = centerY + (maxRing + 1) * ringSpacing + opts.hexHeight;

  return { nodes: positioned, totalHeight, clusters, dividers: [] };
}

// =============================================================================
// Scatter Layout (semantic axes)
// =============================================================================

function scatterLayout(
  nodes: HexNode[],
  _edges: HexEdge[],
  bounds: LayoutBounds,
  opts: LayoutOptions,
): LayoutResult {
  if (!nodes.length) return { nodes: [], totalHeight: 0, clusters: [], dividers: [] };

  const padding = 60;
  const usableWidth = bounds.width - padding * 2;
  const usableHeight = Math.max(usableWidth * 0.8, 300); // Roughly square aspect

  const positioned: HexNode[] = [];
  const fallbackNodes: HexNode[] = [];

  for (const n of nodes) {
    if (n.compassX !== undefined && n.compassY !== undefined) {
      const x = padding + ((n.compassX + 1) / 2) * usableWidth;
      const y = bounds.paddingTop + padding + ((1 - n.compassY) / 2) * usableHeight;
      positioned.push({ ...n, x, y });
    } else {
      fallbackNodes.push(n);
    }
  }

  // Collision resolution: spiral jitter for overlapping nodes
  const minDist = opts.hexRadius * 2.5;
  for (let i = 0; i < positioned.length; i++) {
    for (let j = i + 1; j < positioned.length; j++) {
      const dx = positioned[j].x! - positioned[i].x!;
      const dy = positioned[j].y! - positioned[i].y!;
      const dist = Math.sqrt(dx * dx + dy * dy);
      if (dist < minDist) {
        // Push j outward along the angle from i to j
        const angle = Math.atan2(dy, dx);
        const push = minDist - dist + 2;
        positioned[j] = {
          ...positioned[j],
          x: positioned[j].x! + Math.cos(angle) * push,
          y: positioned[j].y! + Math.sin(angle) * push,
        };
      }
    }
  }

  // Fallback nodes in a small grid at the bottom
  if (fallbackNodes.length > 0) {
    const fallbackY = bounds.paddingTop + padding + usableHeight + 40;
    const fallbackResult = gridLayoutFlat(fallbackNodes, { ...bounds, paddingTop: fallbackY }, opts);
    positioned.push(...fallbackResult.nodes);
  }

  const totalHeight = bounds.paddingTop + padding * 2 + usableHeight + (fallbackNodes.length ? 100 : 0);

  return { nodes: positioned, totalHeight, clusters: [], dividers: [] };
}

// =============================================================================
// Tree Layout (organic diagonal growth per category)
// =============================================================================

function treeLayout(
  nodes: HexNode[],
  _edges: HexEdge[],
  bounds: LayoutBounds,
  opts: LayoutOptions,
): LayoutResult {
  if (!nodes.length) return { nodes: [], totalHeight: 0, clusters: [], dividers: [] };

  // Group by category (preserve first-appearance order)
  const categoryOrder: string[] = [];
  const categoryGroups = new Map<string, HexNode[]>();

  for (const node of nodes) {
    const cat = node.category || 'Uncategorized';
    if (!categoryGroups.has(cat)) {
      categoryOrder.push(cat);
      categoryGroups.set(cat, []);
    }
    categoryGroups.get(cat)!.push(node);
  }

  // 50% scale spacing
  const scaledRadius = opts.hexRadius * 0.5;
  const scaledWidth = Math.sqrt(3) * scaledRadius;
  const scaledHeight = 2 * scaledRadius;
  const treeGap = 3;
  const tXStep = scaledWidth + treeGap;
  const tYStep = scaledHeight * 0.75 + treeGap;
  const categoryGap = 38;

  // Grid-like horizontal centering
  const maxItemsPerRow = Math.floor((bounds.width - scaledWidth) / tXStep);
  const safeItemsPerRow = Math.max(3, Math.min(opts.itemsPerRow, maxItemsPerRow));
  const totalGridWidth = safeItemsPerRow * tXStep + scaledWidth / 2;
  const startX = (bounds.width - totalGridWidth) / 2 + scaledWidth / 2;

  let currentY = bounds.paddingTop;
  const positioned: HexNode[] = [];
  const clusters: ClusterInfo[] = [];
  const dividers: DividerInfo[] = [];

  for (const cat of categoryOrder) {
    const catNodes = categoryGroups.get(cat)!;

    dividers.push({
      label: cat,
      y: currentY,
      x: startX - scaledWidth / 2,
      width: totalGridWidth,
    });

    currentY += 40;

    const catPositioned = growOrganicTree(catNodes, startX, currentY, tXStep, tYStep, cat);
    positioned.push(...catPositioned);

    // Compute bounding box of placed nodes for cluster hull & next category offset
    let maxY = currentY;
    for (const n of catPositioned) {
      if (n.y! > maxY) maxY = n.y!;
    }

    if (catPositioned.length > 0) {
      clusters.push({
        category: cat,
        nodes: catPositioned,
        hull: computeConvexHull(catPositioned, scaledRadius + 4),
      });
    }

    currentY = maxY + scaledHeight / 2 + categoryGap;
  }

  return { nodes: positioned, totalHeight: currentY, clusters, dividers };
}

/**
 * Grow nodes as a diagonal hex-grid walk from a root at top-left.
 * Uses a seeded PRNG (mulberry32) for deterministic organic shapes.
 */
function growOrganicTree(
  nodes: HexNode[],
  startX: number,
  startY: number,
  tXStep: number,
  tYStep: number,
  seed: string,
): HexNode[] {
  if (!nodes.length) return [];

  const rng = mulberry32(hashString(seed));

  // 6 hex neighbor offsets for pointy-top (col, row deltas)
  // Primary growth: lower-right, lower-left, right (biased downward)
  const primaryDirs: [number, number][] = [
    [1, 1],   // lower-right
    [0, 1],   // lower-left (odd-row shift makes this diagonal)
    [1, 0],   // right
  ];
  const secondaryDirs: [number, number][] = [
    [-1, 1],  // far lower-left
    [-1, 0],  // left
    [0, -1],  // upper-left
  ];

  const occupied = new Set<string>();
  const frontier: number[] = []; // indices into result array

  const key = (col: number, row: number) => `${col},${row}`;

  // Hex grid position to pixel (pointy-top, odd-row offset)
  const toPixel = (col: number, row: number) => ({
    x: startX + col * tXStep + (row % 2) * (tXStep / 2),
    y: startY + row * tYStep,
  });

  // Place root at grid (0, 0)
  const result: HexNode[] = [];
  const positions: [number, number][] = []; // grid coords per result index
  const rootPos: [number, number] = [0, 0];

  occupied.add(key(0, 0));
  const rootPixel = toPixel(0, 0);
  result.push({ ...nodes[0], x: rootPixel.x, y: rootPixel.y, scale: 0.5 });
  positions.push(rootPos);
  frontier.push(0);

  for (let i = 1; i < nodes.length; i++) {
    let placed = false;

    // Pick a parent from frontier (biased toward recent nodes)
    const shuffledFrontier = shuffleArray([...frontier], rng);

    for (const parentIdx of shuffledFrontier) {
      const [pc, pr] = positions[parentIdx];

      // Shuffle primary dirs, then try secondary
      const dirs = [
        ...shuffleArray([...primaryDirs], rng),
        ...shuffleArray([...secondaryDirs], rng),
      ];

      for (const [dc, dr] of dirs) {
        const nc = pc + dc;
        const nr = pr + dr;
        if (occupied.has(key(nc, nr))) continue;

        occupied.add(key(nc, nr));
        const pixel = toPixel(nc, nr);
        result.push({ ...nodes[i], x: pixel.x, y: pixel.y, scale: 0.5 });
        positions.push([nc, nr]);
        frontier.push(result.length - 1);
        placed = true;
        break;
      }

      if (placed) break;
    }

    // Fallback: shouldn't happen with 6 directions, but safety net
    if (!placed) {
      const pixel = toPixel(i, 0);
      result.push({ ...nodes[i], x: pixel.x, y: pixel.y, scale: 0.5 });
      positions.push([i, 0]);
      occupied.add(key(i, 0));
    }

    // Trim frontier to keep it organic (keep last ~8 nodes + occasional trunk branch)
    if (frontier.length > 10) {
      // 30% chance to keep an old node for trunk branching
      if (rng() > 0.3) {
        frontier.shift();
      }
    }
  }

  return result;
}

/** djb2 string hash → 32-bit unsigned integer. */
function hashString(s: string): number {
  let hash = 5381;
  for (let i = 0; i < s.length; i++) {
    hash = ((hash << 5) + hash + s.charCodeAt(i)) >>> 0;
  }
  return hash;
}

/** Mulberry32 seeded PRNG. Returns a function that yields [0, 1) floats. */
function mulberry32(seed: number): () => number {
  let t = seed >>> 0;
  return () => {
    t = (t + 0x6d2b79f5) >>> 0;
    let r = Math.imul(t ^ (t >>> 15), 1 | t);
    r = (r + Math.imul(r ^ (r >>> 7), 61 | r)) ^ r;
    return ((r ^ (r >>> 14)) >>> 0) / 4294967296;
  };
}

/** Fisher-Yates shuffle using provided rng. */
function shuffleArray<T>(arr: T[], rng: () => number): T[] {
  for (let i = arr.length - 1; i > 0; i--) {
    const j = Math.floor(rng() * (i + 1));
    [arr[i], arr[j]] = [arr[j], arr[i]];
  }
  return arr;
}

// =============================================================================
// Geometry Helpers
// =============================================================================

/** Compute convex hull of node positions, expanded by padding. */
function computeConvexHull(
  nodes: HexNode[],
  padding: number,
): { x: number; y: number }[] {
  const points = nodes
    .filter(n => n.x !== undefined && n.y !== undefined)
    .map(n => ({ x: n.x!, y: n.y! }));

  if (points.length === 0) return [];
  if (points.length === 1) {
    // Single point → hexagonal hull
    const cx = points[0].x;
    const cy = points[0].y;
    return Array.from({ length: 6 }, (_, i) => {
      const angle = (Math.PI / 180) * (60 * i - 30);
      return { x: cx + padding * Math.cos(angle), y: cy + padding * Math.sin(angle) };
    });
  }

  // Graham scan convex hull
  const sorted = [...points].sort((a, b) => a.x - b.x || a.y - b.y);

  const cross = (o: { x: number; y: number }, a: { x: number; y: number }, b: { x: number; y: number }) =>
    (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x);

  const lower: { x: number; y: number }[] = [];
  for (const p of sorted) {
    while (lower.length >= 2 && cross(lower[lower.length - 2], lower[lower.length - 1], p) <= 0)
      lower.pop();
    lower.push(p);
  }

  const upper: { x: number; y: number }[] = [];
  for (const p of sorted.reverse()) {
    while (upper.length >= 2 && cross(upper[upper.length - 2], upper[upper.length - 1], p) <= 0)
      upper.pop();
    upper.push(p);
  }

  const hull = [...lower.slice(0, -1), ...upper.slice(0, -1)];

  // Expand hull outward by padding
  if (hull.length < 3) {
    // Fallback for collinear points
    const minX = Math.min(...points.map(p => p.x)) - padding;
    const maxX = Math.max(...points.map(p => p.x)) + padding;
    const minY = Math.min(...points.map(p => p.y)) - padding;
    const maxY = Math.max(...points.map(p => p.y)) + padding;
    return [
      { x: minX, y: minY },
      { x: maxX, y: minY },
      { x: maxX, y: maxY },
      { x: minX, y: maxY },
    ];
  }

  // Compute centroid
  const cx = hull.reduce((s, p) => s + p.x, 0) / hull.length;
  const cy = hull.reduce((s, p) => s + p.y, 0) / hull.length;

  // Expand each point away from centroid
  return hull.map(p => {
    const dx = p.x - cx;
    const dy = p.y - cy;
    const dist = Math.sqrt(dx * dx + dy * dy) || 1;
    return {
      x: p.x + (dx / dist) * padding,
      y: p.y + (dy / dist) * padding,
    };
  });
}
