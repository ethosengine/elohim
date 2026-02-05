/**
 * Mastery Visualization Constants
 *
 * Visual representation of Bloom's Taxonomy mastery levels.
 * Used for UI components like mastery badges, progress grids, and charts.
 *
 * Color palette follows a natural progression:
 * - Gray (not started) → warm tones (learning) → green (gate) → cool tones (mastery)
 *
 * The "apply" level is the attestation gate - visually highlighted in green
 * to indicate the threshold for participation privileges.
 */

import type { MasteryLevel } from './content-mastery.model';

// @coverage: 45.2% (2026-02-05)

// =============================================================================
// Color Palettes
// =============================================================================

/**
 * Primary colors for each mastery level.
 * Used for badges, progress indicators, and chart fills.
 */
export const MASTERY_COLORS: Record<MasteryLevel, string> = {
  not_started: '#e0e0e0', // Gray - no progress
  seen: '#fff3e0', // Light orange - just viewed
  remember: '#ffe0b2', // Orange - can recall
  understand: '#fff9c4', // Yellow - comprehends
  apply: '#c8e6c9', // Green - GATE LEVEL (can apply)
  analyze: '#b3e5fc', // Light blue - can analyze
  evaluate: '#b2ebf2', // Cyan - can evaluate
  create: '#e1bee7', // Purple - can create
};

/**
 * Darker accent colors for borders and emphasis.
 */
export const MASTERY_ACCENT_COLORS: Record<MasteryLevel, string> = {
  not_started: '#9e9e9e',
  seen: '#ffb74d',
  remember: '#ff9800',
  understand: '#ffc107',
  apply: '#4caf50', // Green accent - gate level
  analyze: '#03a9f4',
  evaluate: '#00bcd4',
  create: '#9c27b0',
};

/**
 * Text colors for labels on mastery backgrounds.
 */
export const MASTERY_TEXT_COLORS: Record<MasteryLevel, string> = {
  not_started: '#616161',
  seen: '#e65100',
  remember: '#e65100',
  understand: '#f57f17',
  apply: '#1b5e20',
  analyze: '#01579b',
  evaluate: '#006064',
  create: '#4a148c',
};

// =============================================================================
// Icons
// =============================================================================

/**
 * Icon names for each mastery level.
 * Uses Material Design icon names (compatible with mat-icon).
 */
export const MASTERY_ICONS: Record<MasteryLevel, string> = {
  not_started: 'radio_button_unchecked',
  seen: 'visibility',
  remember: 'psychology',
  understand: 'lightbulb',
  apply: 'construction', // Gate level - tools/application
  analyze: 'analytics',
  evaluate: 'balance',
  create: 'stars',
};

/**
 * Alternative icon set using outlined variants.
 */
export const MASTERY_ICONS_OUTLINED: Record<MasteryLevel, string> = {
  not_started: 'radio_button_unchecked',
  seen: 'visibility_outlined',
  remember: 'psychology_outlined',
  understand: 'lightbulb_outlined',
  apply: 'construction_outlined',
  analyze: 'analytics_outlined',
  evaluate: 'balance_outlined',
  create: 'stars_outlined',
};

// =============================================================================
// Labels
// =============================================================================

/**
 * Display labels for each mastery level.
 */
export const MASTERY_LABELS: Record<MasteryLevel, string> = {
  not_started: 'Not Started',
  seen: 'Seen',
  remember: 'Remember',
  understand: 'Understand',
  apply: 'Apply',
  analyze: 'Analyze',
  evaluate: 'Evaluate',
  create: 'Create',
};

/**
 * Short labels for compact displays.
 */
export const MASTERY_SHORT_LABELS: Record<MasteryLevel, string> = {
  not_started: '-',
  seen: 'S',
  remember: 'R',
  understand: 'U',
  apply: 'A',
  analyze: 'An',
  evaluate: 'E',
  create: 'C',
};

/**
 * Descriptions for each mastery level (Bloom's Taxonomy).
 */
export const MASTERY_DESCRIPTIONS: Record<MasteryLevel, string> = {
  not_started: 'Content not yet viewed',
  seen: 'Viewed the content',
  remember: 'Can recall key facts and concepts',
  understand: 'Can explain ideas and concepts',
  apply: 'Can use knowledge in new situations',
  analyze: 'Can draw connections and organize parts',
  evaluate: 'Can justify decisions and make judgments',
  create: 'Can produce new or original work',
};

// =============================================================================
// Progress & Animation
// =============================================================================

/**
 * Progress percentage for each level (for progress bars).
 */
export const MASTERY_PROGRESS: Record<MasteryLevel, number> = {
  not_started: 0,
  seen: 14, // 1/7
  remember: 28, // 2/7
  understand: 42, // 3/7
  apply: 57, // 4/7 - gate level
  analyze: 71, // 5/7
  evaluate: 85, // 6/7
  create: 100, // 7/7
};

/**
 * CSS class names for each mastery level.
 */
export const MASTERY_CSS_CLASSES: Record<MasteryLevel, string> = {
  not_started: 'mastery-not-started',
  seen: 'mastery-seen',
  remember: 'mastery-remember',
  understand: 'mastery-understand',
  apply: 'mastery-apply mastery-gate',
  analyze: 'mastery-analyze',
  evaluate: 'mastery-evaluate',
  create: 'mastery-create',
};

// =============================================================================
// Freshness Visualization
// =============================================================================

/**
 * Colors for freshness indicators.
 */
export const FRESHNESS_COLORS = {
  fresh: '#4caf50', // Green - recently engaged
  stale: '#ff9800', // Orange - needs review
  critical: '#f44336', // Red - needs relearning
};

/**
 * Icons for freshness states.
 */
export const FRESHNESS_ICONS = {
  fresh: 'check_circle',
  stale: 'warning',
  critical: 'error',
};

// =============================================================================
// Challenge Result Visualization
// =============================================================================

/**
 * Colors for level change indicators.
 */
export const LEVEL_CHANGE_COLORS = {
  up: '#4caf50', // Green - level up
  down: '#f44336', // Red - level down
  same: '#9e9e9e', // Gray - no change
};

/**
 * Icons for level change indicators.
 */
export const LEVEL_CHANGE_ICONS = {
  up: 'arrow_upward',
  down: 'arrow_downward',
  same: 'remove',
};

// =============================================================================
// Helper Functions
// =============================================================================

/**
 * Get color for a mastery level.
 */
export function getMasteryColor(level: MasteryLevel): string {
  return MASTERY_COLORS[level] ?? MASTERY_COLORS.not_started;
}

/**
 * Get accent color for a mastery level.
 */
export function getMasteryAccentColor(level: MasteryLevel): string {
  return MASTERY_ACCENT_COLORS[level] ?? MASTERY_ACCENT_COLORS.not_started;
}

/**
 * Get icon for a mastery level.
 */
export function getMasteryIcon(level: MasteryLevel): string {
  return MASTERY_ICONS[level] ?? MASTERY_ICONS.not_started;
}

/**
 * Get label for a mastery level.
 */
export function getMasteryLabel(level: MasteryLevel): string {
  return MASTERY_LABELS[level] ?? 'Unknown';
}

/**
 * Get description for a mastery level.
 */
export function getMasteryDescription(level: MasteryLevel): string {
  return MASTERY_DESCRIPTIONS[level] ?? '';
}

/**
 * Get CSS classes for a mastery level.
 */
export function getMasteryCssClasses(level: MasteryLevel): string {
  return MASTERY_CSS_CLASSES[level] ?? 'mastery-not-started';
}

/**
 * Check if level is at or above the gate (apply).
 */
export function isAtOrAboveGate(level: MasteryLevel): boolean {
  const gateIndex = 4; // apply = index 4
  const levels: MasteryLevel[] = [
    'not_started',
    'seen',
    'remember',
    'understand',
    'apply',
    'analyze',
    'evaluate',
    'create',
  ];
  return levels.indexOf(level) >= gateIndex;
}

/**
 * Get freshness color based on score.
 */
export function getFreshnessColor(freshness: number): string {
  if (freshness >= 0.7) return FRESHNESS_COLORS.fresh;
  if (freshness >= 0.4) return FRESHNESS_COLORS.stale;
  return FRESHNESS_COLORS.critical;
}

/**
 * Get freshness icon based on score.
 */
export function getFreshnessIcon(freshness: number): string {
  if (freshness >= 0.7) return FRESHNESS_ICONS.fresh;
  if (freshness >= 0.4) return FRESHNESS_ICONS.stale;
  return FRESHNESS_ICONS.critical;
}

/**
 * Get level change color.
 */
export function getLevelChangeColor(change: 'up' | 'down' | 'same'): string {
  return LEVEL_CHANGE_COLORS[change];
}

/**
 * Get level change icon.
 */
export function getLevelChangeIcon(change: 'up' | 'down' | 'same'): string {
  return LEVEL_CHANGE_ICONS[change];
}
