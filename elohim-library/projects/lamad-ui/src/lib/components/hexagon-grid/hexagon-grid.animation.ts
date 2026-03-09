/**
 * Hexagon Grid — Animation Loop
 *
 * Runs outside Angular zone for zero-overhead canvas animation.
 * Lerp-interpolates x, y, scale, opacity toward target values.
 * Auto-stops when all nodes converge (zero idle CPU).
 */

import { HexNode } from './hexagon-grid.model';

/** Lerp factor per frame — higher = faster snap, lower = smoother. */
const LERP = 0.12;

/** Convergence threshold in pixels. */
const SNAP_THRESHOLD = 0.5;

/** Convergence threshold for opacity/scale (unitless). */
const SNAP_THRESHOLD_UNIT = 0.01;

export class HexAnimationLoop {
  private rafId: number | null = null;
  private running = false;
  private readonly onFrame: () => void;

  constructor(onFrame: () => void) {
    this.onFrame = onFrame;
  }

  /**
   * Set target positions on nodes and start the animation loop.
   * Call this after a layout change — nodes' x/y are current positions,
   * targetX/targetY are where they should end up.
   */
  start(nodes: HexNode[]): void {
    // Only start if there's something to animate
    let needsAnimation = false;
    for (const n of nodes) {
      if (
        n.targetX !== undefined &&
        n.x !== undefined &&
        Math.abs(n.targetX - n.x) > SNAP_THRESHOLD
      ) {
        needsAnimation = true;
        break;
      }
      if (
        n.targetY !== undefined &&
        n.y !== undefined &&
        Math.abs(n.targetY - n.y) > SNAP_THRESHOLD
      ) {
        needsAnimation = true;
        break;
      }
      if (
        n.targetScale !== undefined &&
        n.scale !== undefined &&
        Math.abs(n.targetScale - n.scale) > SNAP_THRESHOLD_UNIT
      ) {
        needsAnimation = true;
        break;
      }
      if (
        n.targetOpacity !== undefined &&
        n.opacity !== undefined &&
        Math.abs(n.targetOpacity - n.opacity) > SNAP_THRESHOLD_UNIT
      ) {
        needsAnimation = true;
        break;
      }
    }

    if (!needsAnimation) {
      // Snap immediately
      this.snapAll(nodes);
      this.onFrame();
      return;
    }

    if (!this.running) {
      this.running = true;
      this.tick(nodes);
    }
  }

  stop(): void {
    this.running = false;
    if (this.rafId !== null) {
      cancelAnimationFrame(this.rafId);
      this.rafId = null;
    }
  }

  destroy(): void {
    this.stop();
  }

  /** Lerp a single value toward its target. Returns true if still converging. */
  private static lerpValue(
    current: number,
    target: number,
    threshold: number
  ): { value: number; converging: boolean } {
    const delta = target - current;
    if (Math.abs(delta) > threshold) {
      return { value: current + delta * LERP, converging: true };
    }
    return { value: target, converging: false };
  }

  /** Lerp all animated properties on a single node. Returns true if still converging. */
  private static lerpNode(n: HexNode): boolean {
    let converging = false;

    if (n.targetX !== undefined && n.x !== undefined) {
      const r = HexAnimationLoop.lerpValue(n.x, n.targetX, SNAP_THRESHOLD);
      n.x = r.value;
      converging = converging || r.converging;
    }

    if (n.targetY !== undefined && n.y !== undefined) {
      const r = HexAnimationLoop.lerpValue(n.y, n.targetY, SNAP_THRESHOLD);
      n.y = r.value;
      converging = converging || r.converging;
    }

    if (n.targetScale !== undefined) {
      const r = HexAnimationLoop.lerpValue(n.scale ?? 1, n.targetScale, SNAP_THRESHOLD_UNIT);
      n.scale = r.value;
      converging = converging || r.converging;
    }

    if (n.targetOpacity !== undefined) {
      const r = HexAnimationLoop.lerpValue(n.opacity ?? 1, n.targetOpacity, SNAP_THRESHOLD_UNIT);
      n.opacity = r.value;
      converging = converging || r.converging;
    }

    return converging;
  }

  private tick(nodes: HexNode[]): void {
    if (!this.running) return;

    let anyConverging = false;
    for (const n of nodes) {
      if (HexAnimationLoop.lerpNode(n)) {
        anyConverging = true;
      }
    }

    this.onFrame();

    if (anyConverging) {
      this.rafId = requestAnimationFrame(() => this.tick(nodes));
    } else {
      this.running = false;
      this.rafId = null;
    }
  }

  private snapAll(nodes: HexNode[]): void {
    for (const n of nodes) {
      if (n.targetX !== undefined) n.x = n.targetX;
      if (n.targetY !== undefined) n.y = n.targetY;
      if (n.targetScale !== undefined) n.scale = n.targetScale;
      if (n.targetOpacity !== undefined) n.opacity = n.targetOpacity;
    }
  }
}
