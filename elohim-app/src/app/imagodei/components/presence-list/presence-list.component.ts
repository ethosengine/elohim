/**
 * PresenceListComponent - List and manage contributor presences.
 *
 * Features:
 * - Filter by state (all, unclaimed, stewarded, claimed)
 * - Create new presence for absent contributors
 * - Begin stewardship of unclaimed presences
 * - View stewardship details
 */

import { CommonModule } from '@angular/common';
import { Component, OnInit, inject, signal, computed } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { RouterModule } from '@angular/router';

// @coverage: 38.3% (2026-02-05)

import {
  type ContributorPresenceView,
  type CreatePresenceRequest,
  type PresenceState,
  getPresenceStateLabel,
} from '../../models/presence.model';
import { IdentityService } from '../../services/identity.service';
import { PresenceService } from '../../services/presence.service';

type FilterState = 'all' | PresenceState;

@Component({
  selector: 'app-presence-list',
  standalone: true,
  imports: [CommonModule, FormsModule, RouterModule],
  templateUrl: './presence-list.component.html',
  styleUrls: ['./presence-list.component.css'],
})
export class PresenceListComponent implements OnInit {
  private readonly presenceService = inject(PresenceService);
  private readonly identityService = inject(IdentityService);

  // ==========================================================================
  // Component State
  // ==========================================================================

  readonly filter = signal<FilterState>('all');
  readonly isCreating = signal(false);
  readonly showCreateForm = signal(false);
  readonly actionInProgress = signal<string | null>(null);
  readonly error = signal<string | null>(null);
  readonly successMessage = signal<string | null>(null);

  /** Form for creating a new presence */
  createForm = {
    displayName: '',
    note: '',
  };

  // ==========================================================================
  // Service Signals
  // ==========================================================================

  readonly isLoading = this.presenceService.isLoading;
  readonly myStewardedPresences = this.presenceService.myStewardedPresences;
  readonly isAuthenticated = this.identityService.isAuthenticated;
  readonly agentPubKey = this.identityService.agentPubKey;

  // ==========================================================================
  // Local State
  // ==========================================================================

  /** All loaded presences */
  private readonly allPresencesSignal = signal<ContributorPresenceView[]>([]);

  // ==========================================================================
  // Computed
  // ==========================================================================

  /** Filtered presences based on current filter */
  readonly filteredPresences = computed(() => {
    const all = this.allPresencesSignal();
    const currentFilter = this.filter();

    if (currentFilter === 'all') {
      return all;
    }
    return all.filter(p => p.presenceState === currentFilter);
  });

  /** Count by state for filter badges */
  readonly stateCounts = computed(() => {
    const all = this.allPresencesSignal();
    return {
      all: all.length,
      unclaimed: all.filter(p => p.presenceState === 'unclaimed').length,
      stewarded: all.filter(p => p.presenceState === 'stewarded').length,
      claimed: all.filter(p => p.presenceState === 'claimed').length,
    };
  });

  /** Filter options */
  readonly filterOptions: { value: FilterState; label: string }[] = [
    { value: 'all', label: 'All' },
    { value: 'unclaimed', label: 'Unclaimed' },
    { value: 'stewarded', label: 'Stewarded' },
    { value: 'claimed', label: 'Claimed' },
  ];

  // ==========================================================================
  // Lifecycle
  // ==========================================================================

  ngOnInit(): void {
    void this.loadPresences();
    void this.loadMyStewardedPresences();
  }

  // ==========================================================================
  // Data Loading
  // ==========================================================================

  /**
   * Load all presences.
   */
  async loadPresences(): Promise<void> {
    try {
      // Load presences by each state and combine
      const [unclaimed, stewarded, claimed] = await Promise.all([
        this.presenceService.getPresencesByState('unclaimed'),
        this.presenceService.getPresencesByState('stewarded'),
        this.presenceService.getPresencesByState('claimed'),
      ]);

      this.allPresencesSignal.set([...unclaimed, ...stewarded, ...claimed]);
    } catch (error) {
      console.error('[PresenceList] Failed to load presences:', error);
      this.error.set('Failed to load presences');
    }
  }

  /**
   * Load presences I'm stewarding.
   */
  async loadMyStewardedPresences(): Promise<void> {
    if (!this.isAuthenticated()) return;

    try {
      await this.presenceService.getMyStewardedPresences();
    } catch (error) {
      // Intentionally silent - stewarded presence load failure is non-critical for list display
      console.warn('[PresenceList] Non-critical stewarded presences load failed:', error);
    }
  }

  // ==========================================================================
  // Filter Actions
  // ==========================================================================

  /**
   * Set the current filter.
   */
  setFilter(newFilter: FilterState): void {
    this.filter.set(newFilter);
  }

  // ==========================================================================
  // Create Presence
  // ==========================================================================

  /**
   * Show the create form.
   */
  openCreateForm(): void {
    this.createForm = { displayName: '', note: '' };
    this.showCreateForm.set(true);
    this.error.set(null);
  }

  /**
   * Hide the create form.
   */
  closeCreateForm(): void {
    this.showCreateForm.set(false);
  }

  /**
   * Create a new presence for an absent contributor.
   */
  async createPresence(): Promise<void> {
    if (!this.createForm.displayName.trim()) {
      this.error.set('Display name is required');
      return;
    }

    this.isCreating.set(true);
    this.error.set(null);

    try {
      const request: CreatePresenceRequest = {
        displayName: this.createForm.displayName.trim(),
        note: this.createForm.note.trim() || undefined,
      };

      await this.presenceService.createPresence(request);

      this.showCreateForm.set(false);
      this.successMessage.set('Presence created successfully!');
      setTimeout(() => this.successMessage.set(null), 3000);

      // Reload presences
      await this.loadPresences();
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to create presence';
      this.error.set(errorMessage);
    } finally {
      this.isCreating.set(false);
    }
  }

  // ==========================================================================
  // Stewardship Actions
  // ==========================================================================

  /**
   * Begin stewarding an unclaimed presence.
   */
  async beginStewardship(presence: ContributorPresenceView): Promise<void> {
    if (presence.presenceState !== 'unclaimed') {
      this.error.set('Can only steward unclaimed presences');
      return;
    }

    this.actionInProgress.set(presence.id);
    this.error.set(null);

    try {
      await this.presenceService.beginStewardship(presence.id);

      this.successMessage.set(`You are now stewarding ${presence.displayName}`);
      setTimeout(() => this.successMessage.set(null), 3000);

      // Reload presences
      await Promise.all([this.loadPresences(), this.loadMyStewardedPresences()]);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to begin stewardship';
      this.error.set(errorMessage);
    } finally {
      this.actionInProgress.set(null);
    }
  }

  /**
   * Check if current user is the steward of a presence.
   */
  isMyStewarded(presence: ContributorPresenceView): boolean {
    return presence.stewardId === this.agentPubKey();
  }

  // ==========================================================================
  // Display Helpers
  // ==========================================================================

  /**
   * Get state label for display.
   */
  getStateLabel(state: PresenceState): string {
    return getPresenceStateLabel(state);
  }

  /**
   * Get state badge class.
   */
  getStateBadgeClass(state: PresenceState): string {
    return `state-badge state-${state}`;
  }

  /**
   * Format date for display.
   */
  formatDate(dateString: string | undefined | null): string {
    if (!dateString) return '';
    try {
      return new Date(dateString).toLocaleDateString(undefined, {
        year: 'numeric',
        month: 'short',
        day: 'numeric',
      });
    } catch {
      return dateString;
    }
  }

  /**
   * Clear messages.
   */
  clearMessages(): void {
    this.error.set(null);
    this.successMessage.set(null);
  }
}
