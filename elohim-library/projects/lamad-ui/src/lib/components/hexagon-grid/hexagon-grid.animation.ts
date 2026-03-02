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
  private onFrame: () => void;

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
      if (n.targetX !== undefined && n.x !== undefined && Math.abs(n.targetX - n.x) > SNAP_THRESHOLD) {
        needsAnimation = true;
        break;
      }
      if (n.targetY !== undefined && n.y !== undefined && Math.abs(n.targetY - n.y) > SNAP_THRESHOLD) {
        needsAnimation = true;
        break;
      }
      if (n.targetScale !== undefined && n.scale !== undefined && Math.abs(n.targetScale - n.scale) > SNAP_THRESHOLD_UNIT) {
        needsAnimation = true;
        break;
      }
      if (n.targetOpacity !== undefined && n.opacity !== undefined && Math.abs(n.targetOpacity - n.opacity) > SNAP_THRESHOLD_UNIT) {
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

  private tick(nodes: HexNode[]): void {
    if (!this.running) return;

    let allConverged = true;

    for (const n of nodes) {
      // Position
      if (n.targetX !== undefined && n.x !== undefined) {
        const dx = n.targetX - n.x;
        if (Math.abs(dx) > SNAP_THRESHOLD) {
          n.x += dx * LERP;
          allConverged = false;
        } else {
          n.x = n.targetX;
        }
      }

      if (n.targetY !== undefined && n.y !== undefined) {
        const dy = n.targetY - n.y;
        if (Math.abs(dy) > SNAP_THRESHOLD) {
          n.y += dy * LERP;
          allConverged = false;
        } else {
          n.y = n.targetY;
        }
      }

      // Scale
      if (n.targetScale !== undefined) {
        const currentScale = n.scale ?? 1;
        const ds = n.targetScale - currentScale;
        if (Math.abs(ds) > SNAP_THRESHOLD_UNIT) {
          n.scale = currentScale + ds * LERP;
          allConverged = false;
        } else {
          n.scale = n.targetScale;
        }
      }

      // Opacity
      if (n.targetOpacity !== undefined) {
        const currentOpacity = n.opacity ?? 1;
        const dop = n.targetOpacity - currentOpacity;
        if (Math.abs(dop) > SNAP_THRESHOLD_UNIT) {
          n.opacity = currentOpacity + dop * LERP;
          allConverged = false;
        } else {
          n.opacity = n.targetOpacity;
        }
      }
    }

    this.onFrame();

    if (allConverged) {
      this.running = false;
      this.rafId = null;
    } else {
      this.rafId = requestAnimationFrame(() => this.tick(nodes));
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
