/**
 * LensRegistryService - Cross-pillar lens registration for resource exploration.
 *
 * Lives in the elohim pillar because it's a cross-pillar coordination service.
 * Pillars register their LensProviders at bootstrap:
 * - shefa registers the "Folders" lens (by resource type/category)
 * - lamad registers the "Scope & Sequence" lens (path hierarchy)
 * - qahal could register a "Governance" lens (future)
 *
 * The ResourceExplorerComponent queries this registry for available lenses.
 */

import { Injectable, signal, computed } from '@angular/core';

import type {
  LensProvider,
  LensDefinition,
  ResourceCategory,
} from '@app/shefa/models/resource-explorer.model';

@Injectable({ providedIn: 'root' })
export class LensRegistryService {
  private readonly providers = signal<Map<string, LensProvider>>(new Map());

  /** All registered lens definitions (lightweight, for the switcher UI) */
  readonly lensDefinitions = computed<LensDefinition[]>(() =>
    Array.from(this.providers().values()).map(p => ({
      type: p.type,
      label: p.label,
      icon: p.icon,
      description: p.description,
      appliesTo: p.appliesTo,
    }))
  );

  /**
   * Register a lens provider. Called by pillar services at bootstrap.
   * If a provider with the same type already exists, it is replaced.
   */
  register(provider: LensProvider): void {
    this.providers.update(map => {
      const next = new Map(map);
      next.set(provider.type, provider);
      return next;
    });
  }

  /**
   * Unregister a lens provider by type.
   */
  unregister(type: string): void {
    this.providers.update(map => {
      const next = new Map(map);
      next.delete(type);
      return next;
    });
  }

  /**
   * Get a specific lens provider by type.
   */
  getProvider(type: string): LensProvider | undefined {
    return this.providers().get(type);
  }

  /**
   * Get lens definitions filtered by resource category.
   * Returns only lenses that apply to the given category.
   */
  getLensesForCategory(category: ResourceCategory): LensDefinition[] {
    return this.lensDefinitions().filter(lens => lens.appliesTo.includes(category));
  }
}
