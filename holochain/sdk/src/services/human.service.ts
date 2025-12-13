/**
 * Human Service
 *
 * High-level service for human/agent management in the Elohim network.
 */

import { ZomeClient } from '../client/zome-client.js';
import {
  type CreateHumanInput,
  type HumanOutput,
} from '../types.js';

/**
 * Service for human agent management
 *
 * Provides:
 * - Human CRUD operations
 * - Affinity-based queries
 * - Learning progress tracking
 */
export class HumanService {
  private client: ZomeClient;

  constructor(client: ZomeClient) {
    this.client = client;
  }

  /**
   * Create a human entry
   */
  async create(input: CreateHumanInput): Promise<HumanOutput> {
    return this.client.createHuman(input);
  }

  /**
   * Create a human with minimal info
   */
  async createSimple(
    id: string,
    displayName: string,
    affinities: string[] = []
  ): Promise<HumanOutput> {
    return this.client.createHuman({
      id,
      display_name: displayName,
      affinities,
      profile_reach: 'public',
    });
  }

  /**
   * Get human by ID
   */
  async get(id: string): Promise<HumanOutput | null> {
    return this.client.getHumanById(id);
  }

  /**
   * Check if human exists
   */
  async exists(id: string): Promise<boolean> {
    const human = await this.client.getHumanById(id);
    return human !== null;
  }

  /**
   * Find humans by affinity
   */
  async findByAffinity(
    affinities: string[],
    limit?: number
  ): Promise<HumanOutput[]> {
    return this.client.queryHumansByAffinity({ affinities, limit });
  }

  /**
   * Find humans with any of the given affinities
   */
  async findWithAnyAffinity(
    affinities: string[],
    limit: number = 100
  ): Promise<HumanOutput[]> {
    return this.client.queryHumansByAffinity({ affinities, limit });
  }

  /**
   * Find humans with all of the given affinities
   */
  async findWithAllAffinities(
    affinities: string[],
    limit: number = 100
  ): Promise<HumanOutput[]> {
    const results = await this.client.queryHumansByAffinity({
      affinities,
      limit: limit * 2, // Fetch more to filter
    });

    // Filter to only those with all affinities
    return results
      .filter((h) =>
        affinities.every((a) => h.human.affinities.includes(a))
      )
      .slice(0, limit);
  }

  /**
   * Record content completion for a human
   */
  async recordCompletion(
    humanId: string,
    pathId: string,
    contentId: string
  ): Promise<boolean> {
    return this.client.recordContentCompletion({
      human_id: humanId,
      path_id: pathId,
      content_id: contentId,
    });
  }

  /**
   * Get common affinities between humans
   */
  async getCommonAffinities(
    humanId1: string,
    humanId2: string
  ): Promise<string[]> {
    const [human1, human2] = await Promise.all([
      this.get(humanId1),
      this.get(humanId2),
    ]);

    if (!human1 || !human2) {
      return [];
    }

    return human1.human.affinities.filter((a) =>
      human2.human.affinities.includes(a)
    );
  }

  /**
   * Find humans similar to a given human (by shared affinities)
   */
  async findSimilar(humanId: string, limit: number = 10): Promise<HumanOutput[]> {
    const human = await this.get(humanId);
    if (!human) {
      return [];
    }

    const similar = await this.findByAffinity(human.human.affinities, limit * 2);

    // Exclude self and sort by number of shared affinities
    return similar
      .filter((h) => h.human.id !== humanId)
      .sort((a, b) => {
        const aShared = a.human.affinities.filter((af) =>
          human.human.affinities.includes(af)
        ).length;
        const bShared = b.human.affinities.filter((af) =>
          human.human.affinities.includes(af)
        ).length;
        return bShared - aShared;
      })
      .slice(0, limit);
  }

  /**
   * Get all unique affinities in the network (sampling approach)
   *
   * Note: This is a heuristic - true enumeration would require a global index.
   */
  async getAllAffinities(_sampleSize: number = 100): Promise<string[]> {
    // This would need a dedicated zome function for proper enumeration
    // For now, this is a placeholder that demonstrates the API shape
    console.warn(
      '[HumanService.getAllAffinities] Not yet implemented - requires global affinity index'
    );
    return [];
  }
}
