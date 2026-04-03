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

import { Subject, Subscription } from 'rxjs';
import { takeUntil } from 'rxjs/operators';

import { ContentService } from '@app/lamad/services/content.service';
import { ContentNode } from '@app/lamad/models/content-node.model';
import {
  ContentRenderer,
  RendererRegistryService,
} from '@app/lamad/renderers/renderer-registry.service';
import { SeoService } from '../../../services/seo.service';
import {
  ProtocolOmnibarComponent,
  OmnibarSteward,
} from '../protocol-omnibar/protocol-omnibar.component';

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
  imports: [CommonModule, RouterModule, ProtocolOmnibarComponent],
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
  private readonly seoService = inject(SeoService);

  ngOnInit(): void {
    // Derive delivery source from current hostname
    if (typeof window !== 'undefined') {
      this.deliverySource = `doorway ${window.location.hostname}`;
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
    this.omnibarStewards = (node.stewardedBy || []).map(s => ({
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
