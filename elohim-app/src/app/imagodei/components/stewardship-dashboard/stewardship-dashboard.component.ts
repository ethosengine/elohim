/**
 * StewardshipDashboardComponent - View and manage content stewardship.
 *
 * Philosophy (from Manifesto Part IV-C):
 * "Content isn't ever owned by who might create it, it's stewarded by whoever
 * has the most relational connection to the content itself."
 *
 * Features:
 * - View content being stewarded with allocation ratios
 * - Recognition accumulated totals
 * - Pending disputes indicator
 * - Quick actions: claim stewardship, view details
 */

import { CommonModule } from '@angular/common';
import { Component, OnInit, inject, signal, computed } from '@angular/core';
import { RouterModule } from '@angular/router';

// @coverage: 98.0% (2026-02-04)

import {
  type StewardshipAllocation,
  type GovernanceState,
} from '@app/lamad/models/stewardship-allocation.model';
import {
  StewardshipAllocationService,
  type StewardPortfolio,
} from '@app/lamad/services/stewardship-allocation.service';

import { IdentityService } from '../../services/identity.service';
import { PresenceService } from '../../services/presence.service';

/** Display-ready allocation with content info */
interface AllocationDisplay {
  allocation: StewardshipAllocation;
  contentTitle: string;
  stateLabel: string;
  stateColor: string;
}

@Component({
  selector: 'app-stewardship-dashboard',
  standalone: true,
  imports: [CommonModule, RouterModule],
  templateUrl: './stewardship-dashboard.component.html',
  styleUrls: ['./stewardship-dashboard.component.css'],
})
export class StewardshipDashboardComponent implements OnInit {
  private readonly identityService = inject(IdentityService);
  private readonly presenceService = inject(PresenceService);
  private readonly stewardshipService = inject(StewardshipAllocationService);

  // ===========================================================================
  // State
  // ===========================================================================

  readonly isLoading = signal(true);
  readonly error = signal<string | null>(null);
  readonly portfolio = signal<StewardPortfolio | null>(null);
  readonly allocations = signal<AllocationDisplay[]>([]);

  // ===========================================================================
  // Computed State
  // ===========================================================================

  /** Human profile for display */
  readonly profile = computed(() => this.identityService.profile());

  /** Presence ID for the current user (if linked) */
  readonly presenceId = computed(() => {
    // The human's presence ID is typically derived from their identity
    // For bootstrap, we use a convention: human-id → presence-{human-id}
    const humanId = this.identityService.humanId();
    return humanId ?? null;
  });

  /** Total recognition accumulated */
  readonly totalRecognition = computed(() => {
    const p = this.portfolio();
    return p ? p.totalRecognition : 0;
  });

  /** Number of content pieces stewarded */
  readonly contentCount = computed(() => {
    const p = this.portfolio();
    return p ? p.contentCount : 0;
  });

  /** Number of active disputes */
  readonly disputeCount = computed(() => {
    const p = this.portfolio();
    return p ? p.activeDisputeCount : 0;
  });

  /** Whether user has any allocations */
  readonly hasAllocations = computed(() => this.allocations().length > 0);

  // ===========================================================================
  // Lifecycle
  // ===========================================================================

  ngOnInit(): void {
    this.loadPortfolio();
  }

  // ===========================================================================
  // Data Loading
  // ===========================================================================

  /**
   * Load the steward's portfolio.
   */
  loadPortfolio(): void {
    const presenceId = this.presenceId();
    if (!presenceId) {
      this.isLoading.set(false);
      this.error.set('No steward presence linked to your account.');
      return;
    }

    this.isLoading.set(true);
    this.error.set(null);

    this.stewardshipService.getStewardPortfolio(presenceId).subscribe({
      next: portfolio => {
        this.portfolio.set(portfolio);
        this.allocations.set(portfolio.allocations.map(a => this.toAllocationDisplay(a)));
        this.isLoading.set(false);
      },
      error: _err => {
        this.error.set('Failed to load stewardship portfolio.');
        this.isLoading.set(false);
      },
    });
  }

  /**
   * Refresh portfolio data.
   */
  refresh(): void {
    this.loadPortfolio();
  }

  // ===========================================================================
  // Display Helpers
  // ===========================================================================

  /**
   * Convert allocation to display format.
   */
  private toAllocationDisplay(allocation: StewardshipAllocation): AllocationDisplay {
    return {
      allocation,
      contentTitle: this.formatContentId(allocation.contentId),
      stateLabel: this.getStateLabel(allocation.governanceState),
      stateColor: this.getStateColor(allocation.governanceState),
    };
  }

  /**
   * Format content ID for display (fallback when title not available).
   */
  private formatContentId(contentId: string): string {
    // Convert kebab-case to Title Case
    return contentId
      .split('-')
      .map(word => word.charAt(0).toUpperCase() + word.slice(1))
      .join(' ');
  }

  /**
   * Get human-readable label for governance state.
   */
  private getStateLabel(state: GovernanceState): string {
    const labels: Record<GovernanceState, string> = {
      active: 'Active',
      disputed: 'Disputed',
      pending_review: 'Pending Review',
      superseded: 'Superseded',
    };
    return labels[state] ?? state;
  }

  /**
   * Get color class for governance state.
   */
  private getStateColor(state: GovernanceState): string {
    const colors: Record<GovernanceState, string> = {
      active: 'green',
      disputed: 'orange',
      pending_review: 'blue',
      superseded: 'gray',
    };
    return colors[state] ?? 'gray';
  }

  /**
   * Format allocation ratio as percentage.
   */
  formatRatio(ratio: number): string {
    return `${Math.round(ratio * 100)}%`;
  }

  /**
   * Format recognition value for display.
   */
  formatRecognition(value: number): string {
    if (value >= 1000000) {
      return `${(value / 1000000).toFixed(1)}M`;
    }
    if (value >= 1000) {
      return `${(value / 1000).toFixed(1)}K`;
    }
    return value.toFixed(0);
  }

  /**
   * Clear error message.
   */
  clearError(): void {
    this.error.set(null);
  }
}
