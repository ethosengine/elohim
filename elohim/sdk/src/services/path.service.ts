/**
 * Path Service
 *
 * High-level service for learning path operations.
 */

import { ZomeClient } from '../client/zome-client.js';
import {
  type CreatePathInput,
  type AddPathStepInput,
  type PathWithSteps,
  type PathIndex,
  type PathGenerationCriteria,
} from '../types.js';
import type { ActionHash } from '@holochain/client';

/**
 * Service for learning path management
 *
 * Provides:
 * - Path CRUD operations
 * - Step management
 * - Path generation helpers
 */
export class PathService {
  private client: ZomeClient;

  constructor(client: ZomeClient) {
    this.client = client;
  }

  /**
   * Create a learning path
   */
  async create(input: CreatePathInput): Promise<ActionHash> {
    return this.client.createPath(input);
  }

  /**
   * Create a path with sensible defaults
   */
  async createSimple(
    id: string,
    title: string,
    description: string,
    difficulty: 'beginner' | 'intermediate' | 'advanced' = 'beginner'
  ): Promise<ActionHash> {
    return this.client.createPath({
      id,
      version: '1.0.0',
      title,
      description,
      difficulty,
      visibility: 'public',
      path_type: 'introduction',
      tags: [],
    });
  }

  /**
   * Add a step to a path
   */
  async addStep(input: AddPathStepInput): Promise<ActionHash> {
    return this.client.addPathStep(input);
  }

  /**
   * Add multiple steps to a path (in order)
   */
  async addSteps(
    pathId: string,
    steps: Array<{
      resourceId: string;
      stepType?: string;
      title?: string;
      narrative?: string;
      isOptional?: boolean;
    }>
  ): Promise<ActionHash[]> {
    const results: ActionHash[] = [];

    for (let i = 0; i < steps.length; i++) {
      const step = steps[i];
      const hash = await this.client.addPathStep({
        path_id: pathId,
        order_index: i,
        step_type: step.stepType ?? 'read',
        resource_id: step.resourceId,
        step_title: step.title,
        step_narrative: step.narrative,
        is_optional: step.isOptional ?? false,
      });
      results.push(hash);
    }

    return results;
  }

  /**
   * Get a path with all its steps
   */
  async get(pathId: string): Promise<PathWithSteps | null> {
    return this.client.getPathWithSteps(pathId);
  }

  /**
   * Get all paths (index)
   */
  async getAll(): Promise<PathIndex> {
    return this.client.getAllPaths();
  }

  /**
   * Check if a path exists
   */
  async exists(pathId: string): Promise<boolean> {
    const path = await this.client.getPathWithSteps(pathId);
    return path !== null;
  }

  /**
   * Delete a path (removes links, enables re-creation)
   */
  async delete(pathId: string): Promise<boolean> {
    return this.client.deletePath(pathId);
  }

  /**
   * Get paths by difficulty
   */
  async getByDifficulty(
    difficulty: 'beginner' | 'intermediate' | 'advanced'
  ): Promise<PathIndex> {
    const allPaths = await this.client.getAllPaths();
    return {
      ...allPaths,
      paths: allPaths.paths.filter((p) => p.difficulty === difficulty),
      total_count: allPaths.paths.filter((p) => p.difficulty === difficulty)
        .length,
    };
  }

  /**
   * Get paths that include specific content
   */
  async getPathsContaining(contentId: string): Promise<PathIndex> {
    const allPaths = await this.client.getAllPaths();
    const matchingPaths = [];

    for (const pathEntry of allPaths.paths) {
      const fullPath = await this.client.getPathWithSteps(pathEntry.id);
      if (fullPath) {
        const hasContent = fullPath.steps.some(
          (s) => s.step.resource_id === contentId
        );
        if (hasContent) {
          matchingPaths.push(pathEntry);
        }
      }
    }

    return {
      ...allPaths,
      paths: matchingPaths,
      total_count: matchingPaths.length,
    };
  }

  /**
   * Generate path ID from criteria (deterministic)
   *
   * Creates a stable ID based on the generation criteria.
   */
  generatePathId(criteria: PathGenerationCriteria): string {
    const parts = [
      criteria.epic.toLowerCase().replace(/\s+/g, '-'),
      criteria.user_type.toLowerCase().replace(/\s+/g, '-'),
      criteria.difficulty,
    ];
    return parts.join('-');
  }

  /**
   * Get step count for a path
   */
  async getStepCount(pathId: string): Promise<number> {
    const path = await this.client.getPathWithSteps(pathId);
    return path?.steps.length ?? 0;
  }
}
