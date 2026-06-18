import { TestBed } from '@angular/core/testing';
import { provideRouter, ActivatedRoute } from '@angular/router';
import { provideHttpClient } from '@angular/common/http';
import { of } from 'rxjs';
import { describe, it, expect } from 'vitest';

import { ContentViewerComponent } from './content-viewer.component';
import { RendererRegistryService } from '../../renderers/renderer-registry.service';
import { MarkdownRendererComponent } from '../../renderers/markdown-renderer/markdown-renderer.component';
import type { ContentNode } from '../../models/content-node.model';
import { ContentService } from '../../services/content.service';
import { DataLoaderService } from '../../services/data-loader.service';
import { TrustBadgeService } from '../../services/trust-badge.service';
import { PathContextService } from '../../services/path-context.service';
import { StewardshipAllocationService } from '../../services/stewardship-allocation.service';
import { SignalHarnessService } from '../../services/signal-harness.service';
import { HouseholdResilienceService } from '../../services/household-resilience.service';
import { ContentEditorService } from '../../content-io/services/content-editor.service';
import { SeoService } from '../../shared/services/seo.service';
import {
  LAMAD_AFFINITY_TRACKING,
  LAMAD_EPR_NAV,
  LAMAD_EPR_RESOLVER,
  LAMAD_GOVERNANCE_SIGNAL,
  LAMAD_GOVERNANCE,
} from '../../interfaces/cross-pillar.interface';
import { LAMAD_AGENT } from '../../interfaces/agent.interface';
import { LAMAD_STORAGE_CLIENT } from '../../interfaces/storage.interface';
import { ResilienceService as LibResilienceService } from '@elohim/service/public-api';
import { GovernanceApiService } from '@elohim/service';
import { AttentionTrackerService, EVENT_API, AGENT_CONTEXT } from '@elohim/rea-runtime';

/**
 * Regression guard for the renderer-registration gap.
 *
 * `/epr/:resourceId` and `/resource/:resourceId` mount ContentViewerComponent
 * STANDALONE (outside LamadLayoutComponent). Renderer registration is a
 * side-effect of injecting RendererInitializerService — historically only
 * triggered by LamadLayoutComponent. When the viewer was used outside that
 * layout, the RendererRegistryService stayed empty, getRenderer('markdown')
 * returned null, and markdown content dropped to the raw <pre> fallback
 * (headers lost, text overflowing). See content-viewer.component.html fallback.
 *
 * This test uses the REAL RendererRegistryService + RendererInitializerService
 * (no spy) and asserts that simply creating the viewer — as the route does —
 * registers the manifest-driven renderers.
 */
describe('ContentViewerComponent — renderer registration (standalone route)', () => {
  it('registers manifest renderers (markdown) when mounted outside LamadLayout', () => {
    TestBed.configureTestingModule({
      imports: [ContentViewerComponent],
      providers: [
        provideHttpClient(),
        provideRouter([]),
        // RendererRegistryService + RendererInitializerService are deliberately
        // NOT overridden — we exercise the real registration path.
        { provide: ContentService, useValue: {} },
        { provide: DataLoaderService, useValue: {} },
        { provide: TrustBadgeService, useValue: {} },
        { provide: PathContextService, useValue: { context$: of(null) } },
        { provide: StewardshipAllocationService, useValue: {} },
        { provide: SignalHarnessService, useValue: {} },
        { provide: HouseholdResilienceService, useValue: {} },
        { provide: ContentEditorService, useValue: {} },
        { provide: SeoService, useValue: {} },
        { provide: LAMAD_AFFINITY_TRACKING, useValue: { changes$: of(null) } },
        { provide: LAMAD_AGENT, useValue: {} },
        { provide: LAMAD_GOVERNANCE, useValue: {} },
        { provide: LAMAD_GOVERNANCE_SIGNAL, useValue: {} },
        { provide: LAMAD_EPR_RESOLVER, useValue: {} },
        { provide: LAMAD_EPR_NAV, useValue: {} },
        { provide: LAMAD_STORAGE_CLIENT, useValue: {} },
        { provide: LibResilienceService, useValue: {} },
        { provide: GovernanceApiService, useValue: {} },
        { provide: AttentionTrackerService, useValue: {} },
        { provide: EVENT_API, useValue: {} },
        { provide: AGENT_CONTEXT, useValue: {} },
        {
          provide: ActivatedRoute,
          useValue: { params: of({}), fragment: of(null), snapshot: { paramMap: { get: () => null } } },
        },
      ],
    });

    // Creating the component is exactly what the lazy /epr route does.
    TestBed.createComponent(ContentViewerComponent);

    const registry = TestBed.inject(RendererRegistryService);
    expect(registry.canRender('markdown')).toBe(true);

    // Close the "registry knows markdown ≠ viewer renders it" gap: getRenderer
    // must resolve to the exact component ContentViewer.loadRenderer() will
    // instantiate. Before the fix this returned null → raw <pre> fallback.
    const markdownNode = { contentFormat: 'markdown' } as ContentNode;
    expect(registry.getRenderer(markdownNode)).toBe(MarkdownRendererComponent);
  });
});
