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
  opts: LayoutOptions
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
  opts: LayoutOptions
): LayoutResult {
  if (!nodes.length) return { nodes: [], totalHeight: 0, clusters: [], dividers: [] };

  const hasCategories = nodes.some(n => n.category);

  if (hasCategories) {
    return gridLayoutWithCategories(nodes, bounds, opts);
  }

  return gridLayoutFlat(nodes, bounds, opts);
}

/** Original flat honeycomb layout (backward compatible). */
function gridLayoutFlat(nodes: HexNode[], bounds: LayoutBounds, opts: LayoutOptions): LayoutResult {
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

/** Group nodes by category preserving first-appearance order. */
function groupByCategory(nodes: HexNode[]): { order: string[]; groups: Map<string, HexNode[]> } {
  const order: string[] = [];
  const groups = new Map<string, HexNode[]>();

  for (const node of nodes) {
    const cat = node.category ?? 'Uncategorized';
    if (!groups.has(cat)) {
      order.push(cat);
      groups.set(cat, []);
    }
    groups.get(cat)!.push(node);
  }

  return { order, groups };
}

/** Place nodes in tight honeycomb within a category block. Returns max row reached. */
function placeHoneycomb(
  catNodes: HexNode[],
  startX: number,
  startY: number,
  xStep: number,
  yStep: number,
  itemsPerRow: number,
  positioned: HexNode[]
): { catPositioned: HexNode[]; maxRow: number } {
  let maxRow = 0;
  const catPositioned: HexNode[] = [];

  for (let i = 0; i < catNodes.length; i++) {
    let remaining = i;
    let currentRow = 0;

    while (true) {
      const isOdd = currentRow % 2 !== 0;
      const capacity = isOdd ? itemsPerRow - 1 : itemsPerRow;
      if (remaining < capacity) break;
      remaining -= capacity;
      currentRow++;
    }

    if (currentRow > maxRow) maxRow = currentRow;

    const xOffset = (currentRow % 2) * (xStep / 2);
    const x = startX + remaining * xStep + xOffset;
    const y = startY + currentRow * yStep;

    const p = { ...catNodes[i], x, y };
    catPositioned.push(p);
    positioned.push(p);
  }

  return { catPositioned, maxRow };
}

/** Khan-style unit rows: group by category, tight tessellation within, dividers between. */
function gridLayoutWithCategories(
  nodes: HexNode[],
  bounds: LayoutBounds,
  opts: LayoutOptions
): LayoutResult {
  const { order, groups } = groupByCategory(nodes);

  const tightGap = 3;
  const tightXStep = opts.hexWidth + tightGap;
  const tightYStep = opts.hexHeight * 0.75 + tightGap;
  const categoryGap = 38;

  const maxItemsPerRow = Math.floor((bounds.width - opts.hexWidth) / tightXStep);
  const safeItemsPerRow = Math.max(3, Math.min(opts.itemsPerRow, maxItemsPerRow));
  const totalGridWidth = safeItemsPerRow * tightXStep + opts.hexWidth / 2;
  const startX = (bounds.width - totalGridWidth) / 2 + opts.hexWidth / 2;

  let currentY = bounds.paddingTop;
  const positioned: HexNode[] = [];
  const clusters: ClusterInfo[] = [];
  const dividers: DividerInfo[] = [];

  for (const cat of order) {
    const catNodes = groups.get(cat)!;

    dividers.push({
      label: cat,
      y: currentY,
      x: startX - opts.hexWidth / 2,
      width: totalGridWidth,
    });
    currentY += 40;

    const { catPositioned, maxRow } = placeHoneycomb(
      catNodes,
      startX,
      currentY,
      tightXStep,
      tightYStep,
      safeItemsPerRow,
      positioned
    );

    if (catPositioned.length > 0) {
      clusters.push({
        category: cat,
        nodes: catPositioned,
        hull: computeConvexHull(catPositioned, opts.hexRadius + 4),
      });
    }

    currentY += maxRow * tightYStep + opts.hexHeight / 2 + categoryGap;
  }

  return { nodes: positioned, totalHeight: currentY, clusters, dividers };
}

// =============================================================================
// DAG Layout (Sugiyama-inspired skill tree)
// =============================================================================

interface DagGraph {
  nodeMap: Map<string, HexNode>;
  inDegree: Map<string, number>;
  children: Map<string, string[]>;
  parents: Map<string, string[]>;
}

/** Build directed adjacency graph for DAG layout. */
function buildDagGraph(nodes: HexNode[], edges: HexEdge[]): DagGraph {
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
    inDegree.set(e.targetId, (inDegree.get(e.targetId) ?? 0) + 1);
  }

  return { nodeMap, inDegree, children, parents };
}

/** BFS propagation: assign layer depths through DAG children. */
function dagPropagateLayers(queue: string[], layers: Map<string, number>, graph: DagGraph): void {
  let head = 0;
  while (head < queue.length) {
    const current = queue[head++];
    const currentLayer = layers.get(current) ?? 0;

    for (const childId of graph.children.get(current) ?? []) {
      const childLayer = Math.max(layers.get(childId) ?? 0, currentLayer + 1);
      layers.set(childId, childLayer);
      const newDeg = (graph.inDegree.get(childId) ?? 1) - 1;
      graph.inDegree.set(childId, newDeg);
      if (newDeg <= 0 && !queue.includes(childId)) {
        queue.push(childId);
      }
    }
  }
}

/** Topological sort via Kahn's algorithm → layer assignment. */
function dagTopoSort(
  nodes: HexNode[],
  graph: DagGraph
): { layers: Map<string, number>; maxLayer: number } {
  const layers = new Map<string, number>();
  const queue: string[] = [];

  for (const [id, deg] of graph.inDegree) {
    if (deg === 0) {
      queue.push(id);
      layers.set(id, 0);
    }
  }

  if (queue.length === 0 && nodes.length > 0) {
    queue.push(nodes[0].id);
    layers.set(nodes[0].id, 0);
  }

  dagPropagateLayers(queue, layers, graph);

  for (const n of nodes) {
    if (!layers.has(n.id)) layers.set(n.id, 0);
  }

  let maxLayer = 0;
  for (const l of layers.values()) {
    if (l > maxLayer) maxLayer = l;
  }

  return { layers, maxLayer };
}

/** Sort nodes within DAG layers by category then barycenter. */
function dagSortLayers(
  layerGroups: Map<number, HexNode[]>,
  maxLayer: number,
  graph: DagGraph
): void {
  const { nodeMap, parents } = graph;

  for (let layer = 0; layer <= maxLayer; layer++) {
    const layerNodes = layerGroups.get(layer) ?? [];
    layerNodes.sort((a, b) => (a.category ?? '').localeCompare(b.category ?? ''));

    if (layer > 0) {
      const barycenter = new Map<string, number>();
      for (const n of layerNodes) {
        const parentNodes = (parents.get(n.id) ?? [])
          .map(pid => nodeMap.get(pid))
          .filter(p => p?.x !== undefined);
        if (parentNodes.length > 0) {
          const avgX = parentNodes.reduce((sum, p) => sum + (p!.x ?? 0), 0) / parentNodes.length;
          barycenter.set(n.id, avgX);
        }
      }

      layerNodes.sort((a, b) => {
        const catCmp = (a.category ?? '').localeCompare(b.category ?? '');
        if (catCmp !== 0) return catCmp;
        const ba = barycenter.get(a.id);
        const bb = barycenter.get(b.id);
        if (ba !== undefined && bb !== undefined) return ba - bb;
        return 0;
      });
    }

    layerGroups.set(layer, layerNodes);
  }
}

/** Position nodes within each DAG layer. */
function dagPositionNodes(
  layerGroups: Map<number, HexNode[]>,
  maxLayer: number,
  graph: DagGraph,
  bounds: LayoutBounds,
  opts: LayoutOptions
): { positioned: HexNode[]; categoryNodes: Map<string, HexNode[]> } {
  const layerSpacing = opts.hexHeight * 2.8;
  const nodeSpacing = opts.hexWidth * 1.4;
  const startY = bounds.paddingTop + 20;
  const positioned: HexNode[] = [];
  const categoryNodes = new Map<string, HexNode[]>();

  for (let layer = 0; layer <= maxLayer; layer++) {
    const layerNodes = layerGroups.get(layer) ?? [];
    const layerWidth = layerNodes.length * nodeSpacing;
    const layerStartX = (bounds.width - layerWidth) / 2 + nodeSpacing / 2;
    const y = startY + layer * layerSpacing;

    for (let i = 0; i < layerNodes.length; i++) {
      const x = layerStartX + i * nodeSpacing;
      const p = { ...layerNodes[i], x, y };
      positioned.push(p);
      graph.nodeMap.set(p.id, p);

      const cat = p.category ?? '';
      if (cat) {
        if (!categoryNodes.has(cat)) categoryNodes.set(cat, []);
        categoryNodes.get(cat)!.push(p);
      }
    }
  }

  return { positioned, categoryNodes };
}

/** Build convex hulls for node categories. */
function buildCategoryHulls(
  categoryNodes: Map<string, HexNode[]>,
  hullPadding: number
): ClusterInfo[] {
  const clusters: ClusterInfo[] = [];
  for (const [cat, catNodes] of categoryNodes) {
    if (catNodes.length >= 2) {
      clusters.push({
        category: cat,
        nodes: catNodes,
        hull: computeConvexHull(catNodes, hullPadding),
      });
    }
  }
  return clusters;
}

function dagLayout(
  nodes: HexNode[],
  edges: HexEdge[],
  bounds: LayoutBounds,
  opts: LayoutOptions
): LayoutResult {
  if (!nodes.length) return { nodes: [], totalHeight: 0, clusters: [], dividers: [] };

  const graph = buildDagGraph(nodes, edges);
  const { layers, maxLayer } = dagTopoSort(nodes, graph);

  // Group nodes by layer
  const layerGroups = new Map<number, HexNode[]>();
  for (const n of nodes) {
    const layer = layers.get(n.id) ?? 0;
    if (!layerGroups.has(layer)) layerGroups.set(layer, []);
    layerGroups.get(layer)!.push(n);
  }

  dagSortLayers(layerGroups, maxLayer, graph);

  const { positioned, categoryNodes } = dagPositionNodes(
    layerGroups,
    maxLayer,
    graph,
    bounds,
    opts
  );

  const clusters = buildCategoryHulls(categoryNodes, opts.hexRadius + 6);

  const layerSpacing = opts.hexHeight * 2.8;
  const startY = bounds.paddingTop + 20;
  return {
    nodes: positioned,
    totalHeight: startY + maxLayer * layerSpacing + opts.hexHeight,
    clusters,
    dividers: [],
  };
}

// =============================================================================
// Radial Layout (center-out rings)
// =============================================================================

/** Assign ring indices by owner (shared=1, self=2, other=3). */
function assignRingsByOwner(
  nodes: HexNode[],
  centerNodeId: string,
  rings: Map<string, number>
): void {
  for (const n of nodes) {
    if (n.id === centerNodeId) continue;
    if (n.owner === 'shared') rings.set(n.id, 1);
    else if (n.owner === 'self') rings.set(n.id, 2);
    else if (n.owner === 'other') rings.set(n.id, 3);
    else rings.set(n.id, 2);
  }
}

/** Assign ring indices via BFS from center node. */
function assignRingsByBFS(
  nodes: HexNode[],
  edges: HexEdge[],
  centerNodeId: string,
  rings: Map<string, number>
): void {
  const adj = new Map<string, string[]>();
  for (const n of nodes) adj.set(n.id, []);
  for (const e of edges) {
    if (adj.has(e.sourceId) && adj.has(e.targetId)) {
      adj.get(e.sourceId)!.push(e.targetId);
      adj.get(e.targetId)!.push(e.sourceId);
    }
  }

  const bfsQueue = [centerNodeId];
  let bfsHead = 0;
  while (bfsHead < bfsQueue.length) {
    const current = bfsQueue[bfsHead++];
    const currentRing = rings.get(current) ?? 0;
    for (const neighbor of adj.get(current) ?? []) {
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

/** Group nodes by ring index and sort within rings by group then category. */
function groupAndSortRings(nodes: HexNode[], rings: Map<string, number>): Map<number, HexNode[]> {
  const ringGroups = new Map<number, HexNode[]>();
  for (const n of nodes) {
    const ring = rings.get(n.id) ?? 0;
    if (!ringGroups.has(ring)) ringGroups.set(ring, []);
    ringGroups.get(ring)!.push(n);
  }

  for (const [, ringNodes] of ringGroups) {
    ringNodes.sort((a, b) => {
      const groupCmp = (a.group ?? '').localeCompare(b.group ?? '');
      if (groupCmp !== 0) return groupCmp;
      return (a.category ?? '').localeCompare(b.category ?? '');
    });
  }

  return ringGroups;
}

/** Track a positioned node into group clusters. */
function trackGroupNode(
  node: HexNode,
  positioned: HexNode,
  groupNodes: Map<string, HexNode[]>
): void {
  if (node.group) {
    if (!groupNodes.has(node.group)) groupNodes.set(node.group, []);
    groupNodes.get(node.group)!.push(positioned);
  }
}

/** Position nodes along concentric rings. */
function positionRingNodes(
  ringGroups: Map<number, HexNode[]>,
  centerX: number,
  centerY: number,
  ringSpacing: number,
  groupNodes: Map<string, HexNode[]>
): HexNode[] {
  const positioned: HexNode[] = [];

  for (const [ring, ringNodes] of ringGroups) {
    if (ring === 0) {
      for (const n of ringNodes) {
        const p = { ...n, x: centerX, y: centerY, scale: 1.5 };
        positioned.push(p);
        trackGroupNode(n, p, groupNodes);
      }
    } else {
      const radius = ring * ringSpacing;
      const angleOffset = ring * 0.3;
      const count = ringNodes.length;

      for (let i = 0; i < count; i++) {
        const angle = angleOffset + (i / count) * Math.PI * 2;
        const x = centerX + radius * Math.cos(angle);
        const y = centerY + radius * Math.sin(angle);
        const p = { ...ringNodes[i], x, y };
        positioned.push(p);
        trackGroupNode(ringNodes[i], p, groupNodes);
      }
    }
  }

  return positioned;
}

function radialLayout(
  nodes: HexNode[],
  edges: HexEdge[],
  bounds: LayoutBounds,
  opts: LayoutOptions
): LayoutResult {
  if (!nodes.length) return { nodes: [], totalHeight: 0, clusters: [], dividers: [] };

  // 1. Find center node
  const centerNode =
    nodes.find(n => n.owner === 'self') ??
    nodes.find(n => n.group === 'self') ??
    nodes.reduce((best, n) => (n.affinity > best.affinity ? n : best), nodes[0]);

  // 2. Assign rings
  const rings = new Map<string, number>();
  rings.set(centerNode.id, 0);

  if (nodes.some(n => n.owner)) {
    assignRingsByOwner(nodes, centerNode.id, rings);
  } else {
    assignRingsByBFS(nodes, edges, centerNode.id, rings);
  }

  // 3. Group by ring and sort
  const ringGroups = groupAndSortRings(nodes, rings);

  // 4. Position calculation
  const maxRing = Math.max(...Array.from(rings.values()));
  const ringSpacing = opts.hexRadius * 4.5;
  const centerX = bounds.width / 2;
  const centerY = bounds.paddingTop + (maxRing + 1) * ringSpacing;

  const groupNodes = new Map<string, HexNode[]>();
  const positioned = positionRingNodes(ringGroups, centerX, centerY, ringSpacing, groupNodes);

  const clusters = buildCategoryHulls(groupNodes, opts.hexRadius + 4);

  const totalHeight = centerY + (maxRing + 1) * ringSpacing + opts.hexHeight;

  return { nodes: positioned, totalHeight, clusters, dividers: [] };
}

// =============================================================================
// Scatter Layout (semantic axes)
// =============================================================================

/** Push apart overlapping nodes using spiral jitter. */
function resolveScatterCollisions(positioned: HexNode[], minDist: number): void {
  for (let i = 0; i < positioned.length; i++) {
    for (let j = i + 1; j < positioned.length; j++) {
      const dx = positioned[j].x! - positioned[i].x!;
      const dy = positioned[j].y! - positioned[i].y!;
      const dist = Math.sqrt(dx * dx + dy * dy);
      if (dist < minDist) {
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
}

function scatterLayout(
  nodes: HexNode[],
  _edges: HexEdge[],
  bounds: LayoutBounds,
  opts: LayoutOptions
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

  resolveScatterCollisions(positioned, opts.hexRadius * 2.5);

  // Fallback nodes in a small grid at the bottom
  if (fallbackNodes.length > 0) {
    const fallbackY = bounds.paddingTop + padding + usableHeight + 40;
    const fallbackResult = gridLayoutFlat(
      fallbackNodes,
      { ...bounds, paddingTop: fallbackY },
      opts
    );
    positioned.push(...fallbackResult.nodes);
  }

  const totalHeight =
    bounds.paddingTop + padding * 2 + usableHeight + (fallbackNodes.length ? 100 : 0);

  return { nodes: positioned, totalHeight, clusters: [], dividers: [] };
}

// =============================================================================
// Tree Layout (organic diagonal growth per category)
// =============================================================================

function treeLayout(
  nodes: HexNode[],
  _edges: HexEdge[],
  bounds: LayoutBounds,
  opts: LayoutOptions
): LayoutResult {
  if (!nodes.length) return { nodes: [], totalHeight: 0, clusters: [], dividers: [] };

  // Group by category (preserve first-appearance order)
  const categoryOrder: string[] = [];
  const categoryGroups = new Map<string, HexNode[]>();

  for (const node of nodes) {
    const cat = node.category ?? 'Uncategorized';
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

/** Try all directions from a parent hex position to find an empty neighbor. */
function tryPlaceFromParent(
  parentPos: [number, number],
  primaryDirs: [number, number][],
  secondaryDirs: [number, number][],
  rng: () => number,
  occupied: Set<string>,
  key: (col: number, row: number) => string
): [number, number] | null {
  const dirs = [...shuffleArray([...primaryDirs], rng), ...shuffleArray([...secondaryDirs], rng)];
  const [pc, pr] = parentPos;

  for (const [dc, dr] of dirs) {
    const nc = pc + dc;
    const nr = pr + dr;
    if (occupied.has(key(nc, nr))) continue;

    occupied.add(key(nc, nr));
    return [nc, nr];
  }
  return null;
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
  seed: string
): HexNode[] {
  if (!nodes.length) return [];

  const rng = mulberry32(hashString(seed));

  // 6 hex neighbor offsets for pointy-top (col, row deltas)
  // Primary growth: lower-right, lower-left, right (biased downward)
  const primaryDirs: [number, number][] = [
    [1, 1], // lower-right
    [0, 1], // lower-left (odd-row shift makes this diagonal)
    [1, 0], // right
  ];
  const secondaryDirs: [number, number][] = [
    [-1, 1], // far lower-left
    [-1, 0], // left
    [0, -1], // upper-left
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
    let placedPos: [number, number] | null = null;

    // Pick a parent from frontier (biased toward recent nodes)
    const shuffledFrontier = shuffleArray([...frontier], rng);
    for (const parentIdx of shuffledFrontier) {
      placedPos = tryPlaceFromParent(
        positions[parentIdx],
        primaryDirs,
        secondaryDirs,
        rng,
        occupied,
        key
      );
      if (placedPos) break;
    }

    // Fallback: shouldn't happen with 6 directions, but safety net
    if (!placedPos) {
      placedPos = [i, 0];
      occupied.add(key(i, 0));
    }

    const pixel = toPixel(placedPos[0], placedPos[1]);
    result.push({ ...nodes[i], x: pixel.x, y: pixel.y, scale: 0.5 });
    positions.push(placedPos);
    frontier.push(result.length - 1);

    // Trim frontier to keep it organic (keep last ~8 nodes + occasional trunk branch)
    if (frontier.length > 10 && rng() > 0.3) {
      frontier.shift();
    }
  }

  return result;
}

/** djb2 string hash → 32-bit unsigned integer. */
function hashString(s: string): number {
  let hash = 5381;
  for (let i = 0; i < s.length; i++) {
    hash = ((hash << 5) + hash + (s.codePointAt(i) ?? 0)) >>> 0;
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
function computeConvexHull(nodes: HexNode[], padding: number): { x: number; y: number }[] {
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

  const cross = (
    o: { x: number; y: number },
    a: { x: number; y: number },
    b: { x: number; y: number }
  ) => (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x);

  const lower: { x: number; y: number }[] = [];
  for (const p of sorted) {
    while (lower.length >= 2 && cross(lower.at(-2)!, lower.at(-1)!, p) <= 0) lower.pop();
    lower.push(p);
  }

  const upper: { x: number; y: number }[] = [];
  const sortedReversed = [...sorted].reverse();
  for (const p of sortedReversed) {
    while (upper.length >= 2 && cross(upper.at(-2)!, upper.at(-1)!, p) <= 0) upper.pop();
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
