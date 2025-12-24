/**
 * Custodian Blob Distribution Service - Phase 3: P2P Blob Replication
 *
 * Integrates blob distribution with custodian commitment system for
 * peer-to-peer replication:
 * - Selects custodians based on reach level and bandwidth
 * - Creates/updates commitments for blob custody
 * - Tracks replication status across the network
 * - Implements fallback URL management via custodian endpoints
 * - Bandwidth-aware selection to avoid overloading slow peers
 */

import { Injectable } from '@angular/core';
import { Observable, of, throwError } from 'rxjs';
import { map, catchError } from 'rxjs/operators';
import { ContentBlob } from '../models/content-node.model';

/**
 * Custodian selection criteria
 */
export interface CustodianSelectionCriteria {
  reach: 'public' | 'commons' | 'trusted' | 'intimate';
  minBandwidthMbps: number;
  maxLatencyMs: number;
  minUptime: number; // 0.0-1.0
  preferredRegions?: string[];
  maxCustodians?: number;
}

/**
 * Custodian blob commitment
 */
export interface CustodianBlobCommitment {
  contentId: string;
  blobHash: string;
  custodianId: string;
  commitmentStatus: 'active' | 'pending' | 'failed' | 'expired';
  startedAt: number;
  expiresAt: number;
  bandwidth: number;
  replicationProgress: number; // 0.0-1.0
  fallbackUrl: string;
  lastVerifiedAt: number;
}

/**
 * Custodian capability
 */
export interface CustodianCapability {
  custodianId: string;
  displayName?: string;
  availableBandwidthMbps: number;
  latencyMs: number;
  uptime: number; // 0.0-1.0
  region?: string;
  maxBlobSizeGb: number;
  currentBlobCount: number;
  reachLevel: string; // public, commons, trusted, intimate
}

/**
 * Blob replication status
 */
export interface BlobReplicationStatus {
  blobHash: string;
  contentId: string;
  totalSize: number;
  primaryUrl: string;
  custodianCount: number;
  activeReplicas: number;
  replicationProgress: number;
  commitments: CustodianBlobCommitment[];
  healthStatus: 'healthy' | 'degraded' | 'critical';
}

@Injectable({
  providedIn: 'root',
})
export class CustodianBlobDistributionService {
  /** Cache of available custodians */
  private custodianCache = new Map<string, CustodianCapability>();

  /** Track active commitments */
  private blobCommitments = new Map<string, CustodianBlobCommitment[]>();

  /** Custodian health scores */
  private custodianHealthScores = new Map<string, { score: number; lastUpdated: number }>();

  constructor() {
    // Start periodic health check
    this.startHealthMonitoring();
  }

  /**
   * Select suitable custodians for blob replication.
   * Uses reach level and bandwidth constraints.
   *
   * @param blob ContentBlob to replicate
   * @param contentId ID of content containing blob
   * @param criteria Selection criteria
   * @returns Observable with selected custodians
   */
  selectCustodiansForBlob(
    blob: ContentBlob,
    contentId: string,
    criteria: CustodianSelectionCriteria
  ): Observable<CustodianCapability[]> {
    // Query available custodians (mocked for now)
    const custodians = this.getAvailableCustodians(criteria);

    // Score custodians by suitability
    const scored = custodians
      .map((c) => ({
        custodian: c,
        score: this.scoreCustodian(c, blob, criteria),
      }))
      .sort((a, b) => b.score - a.score)
      .slice(0, criteria.maxCustodians || 3)
      .map((s) => s.custodian);

    return of(scored);
  }

  /**
   * Create or update custodian commitment for blob distribution.
   * Initiates P2P blob replication.
   *
   * @param contentId Content ID
   * @param blob Blob to replicate
   * @param custodianId Custodian to receive blob
   * @param expirationDays How long to maintain commitment
   * @returns Observable with commitment
   */
  createBlobCommitment(
    contentId: string,
    blob: ContentBlob,
    custodianId: string,
    expirationDays: number = 30
  ): Observable<CustodianBlobCommitment> {
    const now = Date.now();
    const expiresAt = now + expirationDays * 24 * 60 * 60 * 1000;

    const commitment: CustodianBlobCommitment = {
      contentId,
      blobHash: blob.hash,
      custodianId,
      commitmentStatus: 'pending',
      startedAt: now,
      expiresAt,
      bandwidth: 0,
      replicationProgress: 0,
      fallbackUrl: this.generateFallbackUrl(custodianId, blob.hash),
      lastVerifiedAt: 0,
    };

    // Store commitment
    const key = `${contentId}_${blob.hash}`;
    if (!this.blobCommitments.has(key)) {
      this.blobCommitments.set(key, []);
    }
    this.blobCommitments.get(key)!.push(commitment);

    return of(commitment);
  }

  /**
   * Update blob replication progress from custodian.
   * Custodians report progress as they download and seed.
   *
   * @param commitment Existing commitment
   * @param progress Progress percentage (0-100)
   * @param bandwidth Measured bandwidth
   * @returns Observable with updated commitment
   */
  updateReplicationProgress(
    commitment: CustodianBlobCommitment,
    progress: number,
    bandwidth: number
  ): Observable<CustodianBlobCommitment> {
    // Find the commitment in storage and update it
    const key = `${commitment.contentId}_${commitment.blobHash}`;
    const commitments = this.blobCommitments.get(key) || [];
    const index = commitments.findIndex(
      (c) => c.custodianId === commitment.custodianId
    );

    if (index >= 0) {
      commitments[index].replicationProgress = Math.min(progress, 100);
      commitments[index].bandwidth = bandwidth;
      commitments[index].lastVerifiedAt = Date.now();

      if (progress === 100) {
        commitments[index].commitmentStatus = 'active';
      }

      return of(commitments[index]);
    }

    // If not found in storage, return updated copy
    const updated = { ...commitment };
    updated.replicationProgress = Math.min(progress, 100);
    updated.bandwidth = bandwidth;
    updated.lastVerifiedAt = Date.now();

    if (progress === 100) {
      updated.commitmentStatus = 'active';
    }

    return of(updated);
  }

  /**
   * Get replication status for a blob.
   * Aggregates commitment status from all custodians.
   *
   * @param contentId Content ID
   * @param blobHash Blob hash to check
   * @returns Observable with replication status
   */
  getBlobReplicationStatus(contentId: string, blobHash: string): Observable<BlobReplicationStatus> {
    const key = `${contentId}_${blobHash}`;
    const commitments = this.blobCommitments.get(key) || [];

    const activeReplicas = commitments.filter((c) => c.commitmentStatus === 'active').length;
    const avgProgress =
      commitments.length > 0
        ? commitments.reduce((sum, c) => sum + c.replicationProgress, 0) / commitments.length
        : 0;

    let healthStatus: 'healthy' | 'degraded' | 'critical' = 'healthy';
    if (activeReplicas === 0) {
      healthStatus = 'critical';
    } else if (activeReplicas < Math.ceil(commitments.length / 2)) {
      healthStatus = 'degraded';
    }

    const status: BlobReplicationStatus = {
      blobHash,
      contentId,
      totalSize: 0, // Would come from blob metadata
      primaryUrl: commitments[0]?.fallbackUrl || '',
      custodianCount: commitments.length,
      activeReplicas,
      replicationProgress: avgProgress,
      commitments,
      healthStatus,
    };

    return of(status);
  }

  /**
   * Get all commitments for a blob.
   * Returns which custodians are replicating this blob.
   *
   * @param contentId Content ID
   * @param blobHash Blob hash
   * @returns Array of custodian commitments
   */
  getCommitmentsForBlob(contentId: string, blobHash: string): CustodianBlobCommitment[] {
    const key = `${contentId}_${blobHash}`;
    return this.blobCommitments.get(key) || [];
  }

  /**
   * Revoke or expire a custodian blob commitment.
   * Signals end of responsibility for blob replication.
   *
   * @param contentId Content ID
   * @param blobHash Blob hash
   * @param custodianId Custodian ID
   * @returns Observable with success status
   */
  revokeCommitment(contentId: string, blobHash: string, custodianId: string): Observable<boolean> {
    const key = `${contentId}_${blobHash}`;
    const commitments = this.blobCommitments.get(key) || [];

    const index = commitments.findIndex((c) => c.custodianId === custodianId);
    if (index >= 0) {
      commitments[index].commitmentStatus = 'expired';
      return of(true);
    }

    return of(false);
  }

  /**
   * Get fallback URLs from active custodian commitments.
   * These are URLs where custodians are seeding the blob.
   *
   * @param contentId Content ID
   * @param blobHash Blob hash
   * @returns Array of fallback URLs from custodians
   */
  getCustomerFallbackUrls(contentId: string, blobHash: string): string[] {
    const commitments = this.getCommitmentsForBlob(contentId, blobHash);

    return commitments
      .filter((c) => c.commitmentStatus === 'active')
      .map((c) => c.fallbackUrl)
      .filter((url) => url && url.length > 0);
  }

  /**
   * Select additional custodians for under-replicated blob.
   * If blob has too few replicas, select more custodians.
   *
   * @param blob ContentBlob
   * @param contentId Content ID
   * @param currentCommitments Existing commitments
   * @param minReplicas Minimum desired replicas
   * @param criteria Selection criteria
   * @returns Observable with new custodian selections
   */
  selectAdditionalCustodians(
    blob: ContentBlob,
    contentId: string,
    currentCommitments: CustodianBlobCommitment[],
    minReplicas: number,
    criteria: CustodianSelectionCriteria
  ): Observable<CustodianCapability[]> {
    const needed = Math.max(0, minReplicas - currentCommitments.length);

    if (needed === 0) {
      return of([]);
    }

    const candidates = this.getAvailableCustodians(criteria);

    // Filter out custodians already replicating this blob
    const alreadySelected = new Set(currentCommitments.map((c) => c.custodianId));
    const availableCandidates = candidates.filter((c) => !alreadySelected.has(c.custodianId));

    // Score and select top candidates
    const selected = availableCandidates
      .map((c) => ({
        custodian: c,
        score: this.scoreCustodian(c, blob, criteria),
      }))
      .sort((a, b) => b.score - a.score)
      .slice(0, needed)
      .map((s) => s.custodian);

    return of(selected);
  }

  /**
   * Get custodian capability information.
   * Includes bandwidth, latency, uptime metrics.
   *
   * @param custodianId Custodian ID to query
   * @returns Observable with capability info
   */
  getCustodianCapability(custodianId: string): Observable<CustodianCapability | null> {
    const cached = this.custodianCache.get(custodianId);
    if (cached) {
      return of(cached);
    }

    // In production, would query Holochain for custodian info
    // For now, return mock data
    return of(null);
  }

  /**
   * Probe custodian for availability and health.
   * Checks if custodian is online and accepting new blob commitments.
   *
   * @param custodianId Custodian ID
   * @returns Observable with health status
   */
  probeCustodianHealth(custodianId: string): Observable<{
    online: boolean;
    acceptingBlobs: boolean;
    bandwidth: number;
    latency: number;
  }> {
    // Would make HTTP request to custodian endpoint
    return of({
      online: true,
      acceptingBlobs: true,
      bandwidth: 10,
      latency: 50,
    });
  }

  /**
   * Get average replication progress across all custodians.
   *
   * @param contentId Content ID
   * @param blobHash Blob hash
   * @returns Progress percentage (0-100)
   */
  getAverageReplicationProgress(contentId: string, blobHash: string): number {
    const commitments = this.getCommitmentsForBlob(contentId, blobHash);

    if (commitments.length === 0) {
      return 0;
    }

    return commitments.reduce((sum, c) => sum + c.replicationProgress, 0) / commitments.length;
  }

  /**
   * Get number of active replicas.
   *
   * @param contentId Content ID
   * @param blobHash Blob hash
   * @returns Number of active custodians
   */
  getActiveReplicaCount(contentId: string, blobHash: string): number {
    const commitments = this.getCommitmentsForBlob(contentId, blobHash);
    return commitments.filter((c) => c.commitmentStatus === 'active').length;
  }

  /**
   * Get custodian that is "best" for serving blob.
   * Picks custodian with best bandwidth and lowest latency.
   *
   * @param contentId Content ID
   * @param blobHash Blob hash
   * @returns Fallback URL to best custodian, or null if none available
   */
  getBestCustodianUrl(contentId: string, blobHash: string): string | null {
    const commitments = this.getCommitmentsForBlob(contentId, blobHash).filter(
      (c) => c.commitmentStatus === 'active'
    );

    if (commitments.length === 0) {
      return null;
    }

    // Sort by bandwidth (highest first)
    return commitments.sort((a, b) => b.bandwidth - a.bandwidth)[0].fallbackUrl;
  }

  // =========================================================================
  // Private Helper Methods
  // =========================================================================

  /**
   * Get available custodians matching criteria.
   */
  private getAvailableCustodians(criteria: CustodianSelectionCriteria): CustodianCapability[] {
    // In production, would query DHT for custodians matching criteria
    // For now, return empty array (stub implementation)
    return [];
  }

  /**
   * Score custodian's suitability for replicating blob.
   */
  private scoreCustodian(
    custodian: CustodianCapability,
    blob: ContentBlob,
    criteria: CustodianSelectionCriteria
  ): number {
    let score = 0;

    // Bandwidth suitability (more bandwidth = better, up to 10x needed)
    const bandwidthNeeded = (blob.bitrateMbps || 5) * 2; // 2x overhead
    const bandwidthScore = Math.min(custodian.availableBandwidthMbps / bandwidthNeeded, 1.0);
    score += bandwidthScore * 40; // 40% weight

    // Latency suitability (lower latency = better)
    const latencyScore = Math.max(0, 1.0 - custodian.latencyMs / criteria.maxLatencyMs);
    score += latencyScore * 30; // 30% weight

    // Uptime reliability (higher uptime = better)
    const uptimeScore = custodian.uptime;
    score += uptimeScore * 20; // 20% weight

    // Storage availability (can fit the blob)
    const canStore = custodian.maxBlobSizeGb * 1024 * 1024 * 1024 >= blob.sizeBytes;
    score += canStore ? 10 : 0; // 10% weight

    // Regional preference (if specified)
    if (criteria.preferredRegions && custodian.region && criteria.preferredRegions.includes(custodian.region)) {
      score += 5; // Bonus points
    }

    return score;
  }

  /**
   * Generate fallback URL for custodian blob endpoint.
   */
  private generateFallbackUrl(custodianId: string, blobHash: string): string {
    // Would generate URL based on custodian's endpoint
    return `https://custodian-${custodianId}.example.com/blob/${blobHash}`;
  }

  /**
   * Start periodic health monitoring of custodians.
   */
  private startHealthMonitoring(): void {
    // Run health checks every 5 minutes
    setInterval(() => {
      this.performHealthCheck();
    }, 5 * 60 * 1000);
  }

  /**
   * Perform periodic health check on custodians.
   */
  private performHealthCheck(): void {
    // Would probe all custodians and update health scores
    // Currently stubbed
  }
}
