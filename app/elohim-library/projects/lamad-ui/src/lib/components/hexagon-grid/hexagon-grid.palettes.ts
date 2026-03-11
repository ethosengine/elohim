/**
 * Hexagon Grid — Color Palettes
 *
 * Four palettes for four ways of seeing knowledge:
 *
 *   domain  — "Where am I strong?" (existing 4-tier, warm→cool)
 *   bloom   — "How deeply do I know this?" (8-tier Bloom's taxonomy)
 *   person  — "What do we share?" (red self / blue other / purple overlap)
 *   network — "Who am I connected to?" (hue by group, saturation by affinity)
 *
 * Each palette returns a fill color and optional glow configuration.
 * The freshness overlay is applied as a post-process on any palette.
 */

import { HexNode, HexMasteryLevel } from './hexagon-grid.model';

// =============================================================================
// Palette Output
// =============================================================================

export interface HexColorResult {
  fill: string;
  glowColor?: string;
  glowBlur?: number;
  /** Optional ring color drawn as a 2px stroke around the hex. */
  ringColor?: string;
  ringWidth?: number;
}

// =============================================================================
// Domain Palette (existing behavior, enhanced)
// =============================================================================

const DOMAIN_COLORS: Record<string, string> = {
  high: '#34d399', // Emerald green — strong affinity
  medium: '#fbbf24', // Amber gold — growing
  low: '#fb923c', // Warm orange — encountered
  unseen: 'rgba(148, 163, 184, 0.6)', // Slate — unvisited
};

const DOMAIN_GLOW: Record<string, { color: string; blur: number } | null> = {
  high: { color: '#34d399', blur: 15 },
  medium: { color: '#fbbf24', blur: 8 },
  low: null,
  unseen: null,
};

export function domainPalette(node: HexNode): HexColorResult {
  const level = node.affinityLevel ?? 'unseen';
  const glow = DOMAIN_GLOW[level];
  return {
    fill: DOMAIN_COLORS[level] ?? DOMAIN_COLORS['unseen'],
    glowColor: glow?.color,
    glowBlur: glow?.blur,
  };
}

// =============================================================================
// Bloom's Taxonomy Palette
// =============================================================================

/**
 * 8-tier progression: gray → warm → GREEN GATE → cool → purple
 *
 * The "apply" level is the attestation gate — it gets a distinctive green ring
 * to signal "you've crossed the threshold into participation." The "create"
 * level gets a purple ring — the pinnacle of generative mastery.
 *
 * These are accent colors (saturated) rather than the pastel backgrounds from
 * mastery-visualization.ts, because hexagons are small and need punch.
 */
const BLOOM_FILL: Record<HexMasteryLevel, string> = {
  not_started: 'rgba(148, 163, 184, 0.5)', // Slate fog
  seen: '#fdba74', // Soft peach — just glimpsed
  remember: '#fb923c', // Orange — can recall
  understand: '#fbbf24', // Gold — comprehends
  apply: '#4ade80', // Bright green — GATE
  analyze: '#38bdf8', // Sky blue — dissects
  evaluate: '#22d3ee', // Cyan — judges
  create: '#c084fc', // Purple — creates
};

const BLOOM_GLOW: Partial<Record<HexMasteryLevel, { color: string; blur: number }>> = {
  apply: { color: '#22c55e', blur: 14 },
  analyze: { color: '#38bdf8', blur: 10 },
  evaluate: { color: '#22d3ee', blur: 12 },
  create: { color: '#a855f7', blur: 16 },
};

const BLOOM_RING: Partial<Record<HexMasteryLevel, { color: string; width: number }>> = {
  apply: { color: '#16a34a', width: 2.5 }, // Green gate ring
  create: { color: '#9333ea', width: 2.5 }, // Purple pinnacle ring
};

export function bloomPalette(node: HexNode): HexColorResult {
  const level = node.masteryLevel ?? affinityToBloom(node.affinityLevel);
  const glow = BLOOM_GLOW[level];
  const ring = BLOOM_RING[level];
  return {
    fill: BLOOM_FILL[level],
    glowColor: glow?.color,
    glowBlur: glow?.blur,
    ringColor: ring?.color,
    ringWidth: ring?.width,
  };
}

/** Map 4-tier affinity to nearest Bloom level (fallback when masteryLevel absent). */
function affinityToBloom(level: string): HexMasteryLevel {
  switch (level) {
    case 'high':
      return 'apply';
    case 'medium':
      return 'understand';
    case 'low':
      return 'seen';
    default:
      return 'not_started';
  }
}

// =============================================================================
// Person / Love Map Palette
// =============================================================================

/**
 * Red (self) → Blue (other) → Purple (shared)
 *
 * Inspired by Gottman's Love Maps and the user's vision of seeing
 * "where two people's knowledge overlaps" as gradients of purple.
 *
 * Saturation and brightness scale with affinity/overlapScore so
 * deeply-known nodes glow brighter than surface acquaintance.
 */

function personPaletteSelf(intensity: number): HexColorResult {
  const r = Math.round(200 + 55 * intensity);
  const g = Math.round(60 * (1 - intensity));
  const b = Math.round(60 * (1 - intensity));
  return {
    fill: `rgba(${r}, ${g}, ${b}, ${0.5 + intensity * 0.5})`,
    glowColor: intensity > 0.7 ? `rgba(239, 68, 68, 0.8)` : undefined,
    glowBlur: intensity > 0.7 ? 12 : undefined,
  };
}

function personPaletteOther(intensity: number): HexColorResult {
  const r = Math.round(60 * (1 - intensity));
  const g = Math.round(80 + 40 * intensity);
  const b = Math.round(180 + 75 * intensity);
  return {
    fill: `rgba(${r}, ${g}, ${b}, ${0.5 + intensity * 0.5})`,
    glowColor: intensity > 0.7 ? `rgba(59, 130, 246, 0.8)` : undefined,
    glowBlur: intensity > 0.7 ? 12 : undefined,
  };
}

function personPaletteShared(node: HexNode, intensity: number): HexColorResult {
  const overlap = node.overlapScore ?? intensity;
  const r = Math.round(120 + 80 * overlap);
  const g = Math.round(40 + 30 * (1 - overlap));
  const b = Math.round(160 + 95 * overlap);
  return {
    fill: `rgba(${r}, ${g}, ${b}, ${0.55 + overlap * 0.45})`,
    glowColor: overlap > 0.6 ? `rgba(168, 85, 247, 0.9)` : undefined,
    glowBlur: overlap > 0.6 ? 14 : undefined,
    ringColor: overlap > 0.8 ? '#a855f7' : undefined,
    ringWidth: overlap > 0.8 ? 2 : undefined,
  };
}

function personPalette(node: HexNode): HexColorResult {
  const owner = node.owner ?? 'self';
  const intensity = Math.max(0.15, node.affinity ?? 0);

  switch (owner) {
    case 'self':
      return personPaletteSelf(intensity);
    case 'other':
      return personPaletteOther(intensity);
    case 'shared':
      return personPaletteShared(node, intensity);
    default:
      return personPaletteSelf(intensity);
  }
}

// =============================================================================
// Network Palette
// =============================================================================

/**
 * For P2P agent networks and organizational views.
 * Hue derived from group name (deterministic hash), saturation from affinity.
 * Gives each cluster/community a distinct color family.
 */

function networkPalette(node: HexNode): HexColorResult {
  const hue = node.group ? hashToHue(node.group) : 210; // Default blue
  const aff = node.affinity ?? 0;
  const saturation = 30 + aff * 60; // 30–90%
  const lightness = 35 + aff * 25; // 35–60%
  const alpha = 0.5 + aff * 0.5;

  const fill = `hsla(${hue}, ${saturation}%, ${lightness}%, ${alpha})`;
  const glowIntensity = aff > 0.7;

  return {
    fill,
    glowColor: glowIntensity ? `hsl(${hue}, ${saturation}%, ${lightness + 15}%)` : undefined,
    glowBlur: glowIntensity ? 12 : undefined,
  };
}

/** Simple string → hue hash (deterministic, well-distributed). */
function hashToHue(str: string): number {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    hash = (str.codePointAt(i) ?? 0) + ((hash << 5) - hash);
  }
  return ((hash % 360) + 360) % 360;
}

// =============================================================================
// Palette Registry
// =============================================================================

export type PaletteFn = (node: HexNode) => HexColorResult;

export const PALETTES: Record<string, PaletteFn> = {
  domain: domainPalette,
  bloom: bloomPalette,
  person: personPalette,
  network: networkPalette,
};

/**
 * Get the color result for a node using the specified palette.
 * Applies freshness overlay when showFreshness is true.
 */
export function getHexColor(
  node: HexNode,
  colorMode: string,
  showFreshness: boolean
): HexColorResult {
  const palette = PALETTES[colorMode] ?? domainPalette;
  const result = palette(node);

  // Freshness overlay — desaturate and dim stale content
  if (showFreshness && node.freshness !== undefined && node.freshness < 1) {
    result.fill = applyFreshness(result.fill, node.freshness);
    if (result.glowBlur) {
      result.glowBlur = result.glowBlur * node.freshness;
    }
  }

  return result;
}

// =============================================================================
// Freshness Overlay
// =============================================================================

/**
 * Applies freshness decay to a color string.
 *
 * Fresh (0.7–1.0): Full color, no change
 * Stale (0.4–0.7): Desaturated, slight opacity reduction
 * Critical (<0.4): Heavily washed out, low opacity
 *
 * The visual effect: nodes that need refresher training appear ghostly,
 * like fog creeping back over Territory you once knew well.
 */
function applyFreshness(color: string, freshness: number): string {
  // Parse the color to extract components
  const parsed = parseColor(color);
  if (!parsed) return color;

  const { r, g, b, a } = parsed;

  if (freshness >= 0.7) {
    // Fresh — no modification
    return color;
  }

  // Calculate desaturation factor (0 = full gray, 1 = full color)
  const satFactor =
    freshness < 0.4 ? (freshness / 0.4) * 0.5 : 0.5 + ((freshness - 0.4) / 0.3) * 0.5;

  // Desaturate toward gray
  const gray = 0.299 * r + 0.587 * g + 0.114 * b;
  const nr = Math.round(gray + (r - gray) * satFactor);
  const ng = Math.round(gray + (g - gray) * satFactor);
  const nb = Math.round(gray + (b - gray) * satFactor);

  // Reduce opacity slightly for stale content
  const alphaFactor = freshness < 0.4 ? 0.6 : 0.6 + ((freshness - 0.4) / 0.3) * 0.4;
  const na = a * alphaFactor;

  return `rgba(${nr}, ${ng}, ${nb}, ${na.toFixed(2)})`;
}

/** Naive color parser for rgba() and hex strings. */
function parseColor(color: string): { r: number; g: number; b: number; a: number } | null {
  // rgba(r, g, b, a)
  const rgbaMatch = /rgba?\((\d+),\s*(\d+),\s*(\d+)(?:,\s*([\d.]+))?\)/.exec(color);
  if (rgbaMatch) {
    return {
      r: Number.parseInt(rgbaMatch[1]),
      g: Number.parseInt(rgbaMatch[2]),
      b: Number.parseInt(rgbaMatch[3]),
      a: rgbaMatch[4] ? Number.parseFloat(rgbaMatch[4]) : 1,
    };
  }

  // hsla — convert to RGB (simplified)
  const hslaMatch = /hsla?\((\d+),\s*([\d.]+)%,\s*([\d.]+)%(?:,\s*([\d.]+))?\)/.exec(color);
  if (hslaMatch) {
    const { r, g, b } = hslToRgb(
      Number.parseInt(hslaMatch[1]),
      Number.parseFloat(hslaMatch[2]),
      Number.parseFloat(hslaMatch[3])
    );
    return { r, g, b, a: hslaMatch[4] ? Number.parseFloat(hslaMatch[4]) : 1 };
  }

  // #hex
  const hexMatch = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(color);
  if (hexMatch) {
    return {
      r: Number.parseInt(hexMatch[1], 16),
      g: Number.parseInt(hexMatch[2], 16),
      b: Number.parseInt(hexMatch[3], 16),
      a: 1,
    };
  }

  return null;
}

function hslToRgb(h: number, s: number, l: number): { r: number; g: number; b: number } {
  s /= 100;
  l /= 100;
  const k = (n: number) => (n + h / 30) % 12;
  const a = s * Math.min(l, 1 - l);
  const f = (n: number) => l - a * Math.max(-1, Math.min(k(n) - 3, 9 - k(n), 1));
  return {
    r: Math.round(255 * f(0)),
    g: Math.round(255 * f(8)),
    b: Math.round(255 * f(4)),
  };
}
