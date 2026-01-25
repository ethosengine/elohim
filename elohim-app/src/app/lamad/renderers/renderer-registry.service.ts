import { Injectable, Type, EventEmitter } from '@angular/core';

import { ContentNode } from '../models/content-node.model';

/**
 * Interface for content renderers.
 * Each renderer is an Angular component that can display a specific content format.
 *
 * All renderers receive a `node` input and may emit events for interaction.
 */
export interface ContentRenderer {
  /** The content node to render - set via component input */
  node: ContentNode;
}

/**
 * Extended interface for interactive renderers that emit completion events.
 * Used by quiz-renderer and other assessment-type content.
 */
export interface InteractiveRenderer extends ContentRenderer {
  /** Emitted when the user completes an interactive element */
  complete: EventEmitter<RendererCompletionEvent>;
}

/**
 * Event emitted when a renderer completes an interactive action.
 * Used to update affinity/mastery tracking.
 */
export interface RendererCompletionEvent {
  /** Type of completion (quiz, simulation, video, etc.) */
  type: 'quiz' | 'simulation' | 'video' | 'exercise';

  /** Whether the user passed/succeeded */
  passed: boolean;

  /** Score as percentage (0-100) */
  score: number;

  /** Optional detailed results */
  details?: Record<string, unknown>;
}

/**
 * Registry entry mapping format to component.
 */
interface RendererEntry {
  formats: string[];
  component: Type<any>;
  priority: number; // Higher priority = checked first
}

/**
 * RendererRegistryService - Maps content formats to Angular components.
 *
 * Usage:
 * 1. Register renderers at app startup
 * 2. Call getRenderer(node) to get the appropriate component
 * 3. Use ViewContainerRef.createComponent() to instantiate
 */
@Injectable({ providedIn: 'root' })
export class RendererRegistryService {
  private readonly renderers: RendererEntry[] = [];

  /**
   * Register a renderer component for specific formats.
   *
   * @param formats Array of content formats this renderer handles
   * @param component The Angular component class
   * @param priority Higher = checked first (default 0)
   */
  register(formats: string[], component: Type<any>, priority = 0): void {
    this.renderers.push({ formats, component, priority });
    // Sort by priority descending
    this.renderers.sort((a, b) => b.priority - a.priority);
  }

  /**
   * Get the appropriate component for a content node.
   *
   * @param node The ContentNode to render
   * @returns Component class or null if no match
   */
  getRenderer(node: ContentNode): Type<any> | null {
    for (const entry of this.renderers) {
      if (entry.formats.includes(node.contentFormat)) {
        return entry.component;
      }
    }
    return null; // Caller should use fallback
  }

  /**
   * Check if any renderer can handle this format.
   */
  canRender(format: string): boolean {
    return this.renderers.some(entry => entry.formats.includes(format));
  }

  /**
   * Get all registered formats (for debugging/info).
   */
  getSupportedFormats(): string[] {
    const formats = new Set<string>();
    this.renderers.forEach(entry => {
      entry.formats.forEach(f => formats.add(f));
    });
    return Array.from(formats);
  }

  /**
   * Check if the registry has any renderers registered.
   */
  isInitialized(): boolean {
    return this.renderers.length > 0;
  }
}
