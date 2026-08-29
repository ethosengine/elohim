import { CommonModule } from '@angular/common';
import {
  AfterViewChecked,
  Component,
  ComponentRef,
  OnDestroy,
  OnInit,
  ViewChild,
  ViewContainerRef,
  inject,
} from '@angular/core';
import { ActivatedRoute, RouterModule } from '@angular/router';

import { takeUntil } from 'rxjs/operators';

import { Subject, Subscription } from 'rxjs';

import { ContentNode } from '@app/lamad/models/content-node.model';
import { RendererInitializerService } from '@app/lamad/renderers/renderer-initializer.service';
import {
  ContentRenderer,
  RendererRegistryService,
} from '@app/lamad/renderers/renderer-registry.service';
import { ContentService } from '@app/lamad/services/content.service';

import { SeoService } from '../../../services/seo.service';

/**
 * Steward attribution row for the delivery view's provenance data.
 *
 * NOTE: The omnibar that consumed this is now runtime chrome (server-rendered
 * by the doorway), not a per-route Angular component — see
 * genesis/docs/superpowers/specs/2026-06-26-native-rust-epr-shell-ssr-design.md.
 * These fields stay populated as the provenance source the runtime chrome reads
 * once /deliver is brought under SSR (currently a CSR route — coverage gap).
 */
interface OmnibarSteward {
  humanId: string;
  displayName: string;
  ratio: number;
}

/**
 * ContentDeliveryComponent — Full-page content delivery with protocol omnibar.
 *
 * This is NOT the learning viewer. It renders a ContentNode as the entire page,
 * with no Angular app chrome — just the content and a provenance toolbar.
 * Used for public content delivery via /deliver/:slug.
 */
@Component({
  selector: 'app-content-delivery',
  standalone: true,
  imports: [CommonModule, RouterModule],
  templateUrl: './content-delivery.component.html',
  styleUrls: ['./content-delivery.component.css'],
})
export class ContentDeliveryComponent implements OnInit, OnDestroy, AfterViewChecked {
  node: ContentNode | null = null;
  isLoading = true;
  error: string | null = null;

  // Omnibar data
  contentAddress = '';
  omnibarStewards: OmnibarSteward[] = [];
  reach = '';
  deliverySource = '';

  // Renderer hosting
  @ViewChild('rendererHost', { read: ViewContainerRef, static: false })
  rendererHost!: ViewContainerRef;
  private rendererRef: ComponentRef<ContentRenderer> | null = null;
  private rendererSubscription: Subscription | null = null;
  hasRegisteredRenderer = false;
  private pendingRendererLoad = false;

  private readonly destroy$ = new Subject<void>();
  private readonly route = inject(ActivatedRoute);
  private readonly contentService = inject(ContentService);
  private readonly rendererRegistry = inject(RendererRegistryService);
  // Injecting RendererInitializerService triggers manifest-driven renderer
  // registration. This full-page delivery view (/deliver/:slug) is mounted
  // OUTSIDE LamadLayoutComponent, so it must trigger registration itself —
  // otherwise getRenderer() returns null and markdown drops to the raw fallback.
  private readonly _rendererInit = inject(RendererInitializerService);
  private readonly seoService = inject(SeoService);

  ngOnInit(): void {
    // Derive delivery source from current hostname
    // eslint-disable-next-line no-restricted-syntax -- SSR-safe: inside typeof-equivalent guard (globalThis.window property access is undefined, not a throw, in the SSR runtime)
    if (globalThis.window !== undefined) {
      // eslint-disable-next-line no-restricted-syntax -- SSR-safe: guarded by the globalThis.window existence check above
      this.deliverySource = `doorway ${globalThis.location.hostname}`;
    }

    this.route.params.pipe(takeUntil(this.destroy$)).subscribe(params => {
      const slug = params['slug'] as string;
      if (slug) {
        this.loadContent(slug);
      }
    });
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
    this.destroyRenderer();
  }

  ngAfterViewChecked(): void {
    if (this.pendingRendererLoad && this.node && this.rendererHost) {
      this.pendingRendererLoad = false;
      this.loadRenderer();
    }
  }

  private loadContent(slug: string): void {
    this.isLoading = true;
    this.error = null;

    this.contentService
      .getContentBySlug(slug)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: node => {
          if (!node) {
            this.error = 'Content not found';
            this.isLoading = false;
            return;
          }

          this.node = node;
          this.populateOmnibar(node);
          this.updateSeo(node);
          this.isLoading = false;
          this.pendingRendererLoad = true;
        },
        error: () => {
          this.error = 'Failed to load content';
          this.isLoading = false;
        },
      });
  }

  private populateOmnibar(node: ContentNode): void {
    this.contentAddress = node.id;
    this.reach = (node.reach as string) || 'commons';
    this.omnibarStewards = (node.stewardedBy ?? []).map(s => ({
      humanId: s.humanId,
      displayName: s.humanId,
      ratio: s.affinity ?? 0,
    }));
  }

  private updateSeo(node: ContentNode): void {
    this.seoService.updateForContent({
      id: node.id,
      title: node.title,
      summary: node.description,
      contentType: node.contentType,
      thumbnailUrl: node.metadata?.['thumbnailUrl'],
      authors: node.metadata?.['authors'],
      createdAt: node.createdAt,
      updatedAt: node.updatedAt,
    });
  }

  private loadRenderer(): void {
    if (!this.node || !this.rendererHost) return;

    this.destroyRenderer();
    this.rendererHost.clear();

    const rendererComponent = this.rendererRegistry.getRenderer(this.node);
    if (!rendererComponent) {
      this.hasRegisteredRenderer = false;
      return;
    }

    this.hasRegisteredRenderer = true;
    this.rendererRef = this.rendererHost.createComponent(rendererComponent);
    this.rendererRef.setInput('node', this.node);
  }

  private destroyRenderer(): void {
    if (this.rendererSubscription) {
      this.rendererSubscription.unsubscribe();
      this.rendererSubscription = null;
    }
    if (this.rendererRef) {
      this.rendererRef.destroy();
      this.rendererRef = null;
    }
  }

  getStringContent(content: string | object): string {
    return typeof content === 'string' ? content : JSON.stringify(content, null, 2);
  }
}
