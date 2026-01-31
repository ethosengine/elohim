/**
 * CreatePresenceComponent - Create a contributor presence for an absent contributor.
 *
 * Philosophy:
 * - Anyone can create a presence for someone not yet in the network
 * - Recognition accumulates even while unclaimed
 * - This enables attributing work to contributors who may join later
 *
 * Use cases:
 * - Attributing historical contributions (e.g., Lynn Foster's hREA work)
 * - Crediting external collaborators
 * - Preparing for a contributor's arrival
 */

import { CommonModule } from '@angular/common';
import { Component, inject, signal, computed, output } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Router, RouterModule } from '@angular/router';

// @coverage: 100.0% (2026-02-04)

import { ContentService } from '@app/lamad/services/content.service';

import {
  type CreatePresenceRequest,
  type ExternalIdentifier,
  type ExternalIdentifierProvider,
  ExternalIdentifierProviders,
  getProviderLabel,
  getProviderIcon,
} from '../../models/presence.model';
import { PresenceService } from '../../services/presence.service';

/** External identifier being edited */
interface IdentifierEntry {
  id: string;
  provider: ExternalIdentifierProvider | string;
  value: string;
}

@Component({
  selector: 'app-create-presence',
  standalone: true,
  imports: [CommonModule, FormsModule, RouterModule],
  templateUrl: './create-presence.component.html',
  styleUrls: ['./create-presence.component.css'],
})
export class CreatePresenceComponent {
  private readonly presenceService = inject(PresenceService);
  private readonly contentService = inject(ContentService);
  private readonly router = inject(Router);

  // ===========================================================================
  // Outputs
  // ===========================================================================

  /** Emitted when presence is created successfully */
  readonly created = output<string>();

  /** Emitted when creation is cancelled */
  readonly cancelled = output<void>();

  // ===========================================================================
  // Form State
  // ===========================================================================

  readonly displayName = signal('');
  readonly note = signal('');

  /** External identifiers being added */
  readonly identifiers = signal<IdentifierEntry[]>([]);

  /** Current identifier being added */
  readonly newIdentifier = signal<IdentifierEntry>({
    id: '',
    provider: 'github',
    value: '',
  });

  /** Establishing content IDs */
  readonly establishingContentIds = signal<string[]>([]);

  /** Content search query for typeahead */
  readonly contentSearch = signal('');

  /** Content search results */
  readonly contentResults = signal<{ id: string; title: string }[]>([]);

  // ===========================================================================
  // Component State
  // ===========================================================================

  readonly isSubmitting = signal(false);
  readonly error = signal<string | null>(null);
  readonly showContentSearch = signal(false);

  // ===========================================================================
  // Template Helpers
  // ===========================================================================

  /** Available provider options */
  readonly providerOptions = Object.values(ExternalIdentifierProviders);

  readonly getProviderLabel = getProviderLabel;
  readonly getProviderIcon = getProviderIcon;

  /** Whether form is valid */
  readonly isValid = computed(() => {
    return this.displayName().trim().length >= 2;
  });

  // ===========================================================================
  // Actions
  // ===========================================================================

  /**
   * Add an external identifier to the list.
   */
  addIdentifier(): void {
    const current = this.newIdentifier();
    if (!current.value.trim()) {
      return;
    }

    this.identifiers.update(list => [
      ...list,
      {
        id: `id-${Date.now()}`,
        provider: current.provider,
        value: current.value.trim(),
      },
    ]);

    // Reset new identifier input
    this.newIdentifier.set({
      id: '',
      provider: current.provider, // Keep same provider for convenience
      value: '',
    });
  }

  /**
   * Remove an identifier from the list.
   */
  removeIdentifier(id: string): void {
    this.identifiers.update(list => list.filter(i => i.id !== id));
  }

  /**
   * Update the new identifier provider.
   */
  setProvider(provider: string): void {
    this.newIdentifier.update(current => ({
      ...current,
      provider: provider as ExternalIdentifierProvider,
    }));
  }

  /**
   * Update the new identifier value.
   */
  setIdentifierValue(value: string): void {
    this.newIdentifier.update(current => ({
      ...current,
      value,
    }));
  }

  /**
   * Handle provider change event.
   */
  onProviderChange(event: Event): void {
    const value = (event.target as HTMLSelectElement).value;
    this.setProvider(value);
  }

  /**
   * Handle identifier input event.
   */
  onIdentifierInput(event: Event): void {
    const value = (event.target as HTMLInputElement).value;
    this.setIdentifierValue(value);
  }

  /**
   * Search for content to establish the presence.
   */
  searchContent(): void {
    const query = this.contentSearch().trim();
    if (query.length < 2) {
      this.contentResults.set([]);
      return;
    }

    this.contentService.searchContent(query).subscribe({
      next: results => {
        this.contentResults.set(results.slice(0, 10).map(c => ({ id: c.id, title: c.title })));
      },
      error: () => {
        this.contentResults.set([]);
      },
    });
  }

  /**
   * Add content to establishing content list.
   */
  addEstablishingContent(contentId: string): void {
    if (!this.establishingContentIds().includes(contentId)) {
      this.establishingContentIds.update(ids => [...ids, contentId]);
    }
    this.contentSearch.set('');
    this.contentResults.set([]);
    this.showContentSearch.set(false);
  }

  /**
   * Remove content from establishing list.
   */
  removeEstablishingContent(contentId: string): void {
    this.establishingContentIds.update(ids => ids.filter(id => id !== contentId));
  }

  /**
   * Toggle content search visibility.
   */
  toggleContentSearch(): void {
    this.showContentSearch.update(v => !v);
    if (!this.showContentSearch()) {
      this.contentSearch.set('');
      this.contentResults.set([]);
    }
  }

  /**
   * Submit the form to create a presence.
   */
  async onSubmit(): Promise<void> {
    if (!this.isValid()) {
      this.error.set('Please enter a display name (at least 2 characters).');
      return;
    }

    this.isSubmitting.set(true);
    this.error.set(null);

    try {
      // Build external identifiers
      const externalIdentifiers: ExternalIdentifier[] = this.identifiers().map(i => ({
        provider: i.provider,
        value: i.value,
      }));

      const request: CreatePresenceRequest = {
        displayName: this.displayName().trim(),
        externalIdentifiers: externalIdentifiers.length > 0 ? externalIdentifiers : undefined,
        establishingContentIds:
          this.establishingContentIds().length > 0 ? this.establishingContentIds() : undefined,
        note: this.note().trim() || undefined,
      };

      const presence = await this.presenceService.createPresence(request);

      // Emit success and navigate
      this.created.emit(presence.id);
      void this.router.navigate(['/identity/presences', presence.id]);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to create presence';
      this.error.set(errorMessage);
    } finally {
      this.isSubmitting.set(false);
    }
  }

  /**
   * Cancel and go back.
   */
  onCancel(): void {
    this.cancelled.emit();
    void this.router.navigate(['/identity/presences']);
  }

  /**
   * Clear error message.
   */
  clearError(): void {
    this.error.set(null);
  }
}
