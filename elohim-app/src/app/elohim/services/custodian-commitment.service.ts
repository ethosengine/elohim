import { Injectable, inject } from '@angular/core';

// @coverage: 92.2% (2026-02-05)

import { HolochainClientService } from './holochain-client.service';

/**
 * CustodianCommitmentService
 *
 * Manages DHT interactions for custodian commitments.
 *
 * A commitment is when a custodian (node operator) explicitly agrees
 * to replicate and serve specific content.
 *
 * DHT Structure:
 * - CustodianCommitment entries link custodian → content
 * - One entry per custodian per content
 * - Expires after configured period (e.g., 30 days)
 * - Renewable by custodian
 */

export interface CustodianCommitment {
  /** Unique identifier for this commitment */
  id: string;

  /** The custodian (agent) making the commitment */
  custodianId: string;

  /** The custodian's doorway endpoint (for HTTP requests) */
  doorwayEndpoint: string;

  /** The content being committed to */
  contentId: string;

  /** Domain of content (e.g., 'elohim-protocol') */
  domain: string;

  /** Epic of content (e.g., 'governance') */
  epic: string;

  /** Replication strategy for this content */
  replicationStrategy: 'full_replica' | 'threshold' | 'erasure_coded';

  /** Parameters for threshold/erasure (e.g., "3-of-5" or "5-of-8") */
  strategyParams?: string;

  /** When commitment was created */
  createdAt: number;

  /** When commitment expires (must renew before this) */
  expiresAt: number;

  /** Is this commitment currently active? */
  isActive: boolean;

  /** Storage allocated for this commitment (bytes) */
  storageAllocated: number;

  /** Bandwidth allocated (Mbps) */
  bandwidthAllocated: number;

  /** Steward tier of custodian */
  stewardTier: 1 | 2 | 3 | 4;

  /** Metadata */
  metadata?: Record<string, any>;
}

@Injectable({
  providedIn: 'root',
})
export class CustodianCommitmentService {
  private readonly holochain = inject(HolochainClientService);

  /**
   * Get all custodians committed to specific content
   */
  async getCommitmentsForContent(contentId: string): Promise<CustodianCommitment[]> {
    try {
      const result = await this.holochain.callZome({
        zomeName: 'replication',
        fnName: 'get_custodian_commitments_for_content',
        payload: { content_id: contentId },
      });

      if (!result.success) {
        return [];
      }

      return (result.data as CustodianCommitment[]) || [];
    } catch {
      // Zome call failed - commitments query error, return empty list as fallback
      return [];
    }
  }

  /**
   * Get all commitments made by a specific custodian
   */
  async getCommitmentsByCustomian(custodianId: string): Promise<CustodianCommitment[]> {
    try {
      const result = await this.holochain.callZome({
        zomeName: 'replication',
        fnName: 'get_custodian_all_commitments',
        payload: { custodian_id: custodianId },
      });

      if (!result.success) {
        return [];
      }

      return (result.data as CustodianCommitment[]) || [];
    } catch {
      // Zome call failed - custodian commitments query error, return empty list as fallback
      return [];
    }
  }

  /**
   * Create a new commitment (custodian makes offer)
   */
  async createCommitment(
    custodianId: string,
    contentId: string,
    replicationStrategy: 'full_replica' | 'threshold' | 'erasure_coded',
    storageAllocated: number,
    bandwidthAllocated: number,
    expirationDays = 30
  ): Promise<{ success: boolean; commitmentId?: string; error?: string }> {
    try {
      const expiresAt = Date.now() + expirationDays * 24 * 60 * 60 * 1000;

      const result = await this.holochain.callZome({
        zomeName: 'replication',
        fnName: 'create_custodian_commitment',
        payload: {
          custodian_id: custodianId,
          content_id: contentId,
          replication_strategy: replicationStrategy,
          storage_allocated: storageAllocated,
          bandwidth_allocated: bandwidthAllocated,
          expires_at: expiresAt,
        },
      });

      if (!result.success) {
        return { success: false, error: result.error };
      }

      return { success: true, commitmentId: result.data as string };
    } catch (err) {
      return { success: false, error: String(err) };
    }
  }

  /**
   * Renew an existing commitment (extend expiration)
   */
  async renewCommitment(
    commitmentId: string,
    extensionDays = 30
  ): Promise<{ success: boolean; error?: string }> {
    try {
      const result = await this.holochain.callZome({
        zomeName: 'replication',
        fnName: 'renew_custodian_commitment',
        payload: {
          commitment_id: commitmentId,
          extension_days: extensionDays,
        },
      });

      if (!result.success) {
        return { success: false, error: result.error };
      }

      return { success: true };
    } catch (err) {
      return { success: false, error: String(err) };
    }
  }

  /**
   * Revoke a commitment (custodian stops serving)
   */
  async revokeCommitment(commitmentId: string): Promise<{ success: boolean; error?: string }> {
    try {
      const result = await this.holochain.callZome({
        zomeName: 'replication',
        fnName: 'revoke_custodian_commitment',
        payload: { commitment_id: commitmentId },
      });

      if (!result.success) {
        return { success: false, error: result.error };
      }

      return { success: true };
    } catch (err) {
      return { success: false, error: String(err) };
    }
  }

  /**
   * Get commitment expiring soon (within N days)
   */
  async getExpiringCommitments(
    custodianId: string,
    withinDays = 7
  ): Promise<CustodianCommitment[]> {
    try {
      const commitments = await this.getCommitmentsByCustomian(custodianId);

      const now = Date.now();
      const threshold = now + withinDays * 24 * 60 * 60 * 1000;

      return commitments.filter(c => c.expiresAt < threshold && c.isActive);
    } catch {
      // Query error - expiring commitments check failed, return empty list as fallback
      return [];
    }
  }

  /**
   * Count active commitments for a custodian
   */
  async getActiveCommitmentCount(custodianId: string): Promise<number> {
    try {
      const commitments = await this.getCommitmentsByCustomian(custodianId);
      return commitments.filter(c => c.isActive).length;
    } catch {
      // Query error - active commitment count unavailable, return 0 as fallback
      return 0;
    }
  }

  /**
   * Get total storage committed by custodian
   */
  async getTotalCommittedStorage(custodianId: string): Promise<number> {
    try {
      const commitments = await this.getCommitmentsByCustomian(custodianId);
      return commitments.filter(c => c.isActive).reduce((sum, c) => sum + c.storageAllocated, 0);
    } catch {
      // Query error - total committed storage unavailable, return 0 as fallback
      return 0;
    }
  }

  /**
   * Check if custodian is committed to specific content
   */
  async isCommittedTo(custodianId: string, contentId: string): Promise<boolean> {
    try {
      const commitments = await this.getCommitmentsForContent(contentId);
      return commitments.some(c => c.custodianId === custodianId && c.isActive);
    } catch {
      // Query error - commitment check failed, assume not committed as fallback
      return false;
    }
  }
}
