import { CommonModule } from '@angular/common';
import { ChangeDetectorRef, Component, OnDestroy, OnInit, inject } from '@angular/core';
import { ActivatedRoute, RouterModule } from '@angular/router';

import { takeUntil } from 'rxjs/operators';

import { Subject } from 'rxjs';

import { SeoService } from '../../../services/seo.service';
import { EprFocalComponent, FocalNode } from '../epr-focal/epr-focal.component';

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
 * Used for public content delivery via /deliver/:slug. Composes <app-epr-focal>
 * for the render itself; keeps provenance/SEO here.
 */
@Component({
  selector: 'app-content-delivery',
  standalone: true,
  imports: [CommonModule, RouterModule, EprFocalComponent],
  templateUrl: './content-delivery.component.html',
  styleUrls: ['./content-delivery.component.css'],
})
export class ContentDeliveryComponent implements OnInit, OnDestroy {
  slug = '';
  node: FocalNode | null = null;
  error: string | null = null;

  // Omnibar data
  contentAddress = '';
  omnibarStewards: OmnibarSteward[] = [];
  reach = '';
  deliverySource = '';

  private readonly destroy$ = new Subject<void>();
  private readonly route = inject(ActivatedRoute);
  private readonly seoService = inject(SeoService);
  private readonly cdr = inject(ChangeDetectorRef);

  ngOnInit(): void {
    // Derive delivery source from current hostname
    if (typeof window !== 'undefined') {
      // eslint-disable-next-line no-restricted-syntax -- SSR-safe: guarded by the typeof check above
      this.deliverySource = `doorway ${window.location.hostname}`;
    }

    this.route.params.pipe(takeUntil(this.destroy$)).subscribe(params => {
      const slug = params['slug'] as string;
      if (slug) {
        this.slug = slug;
        this.error = null;
        this.node = null;
        this.cdr.markForCheck();
      }
    });
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  onNodeLoaded(node: FocalNode): void {
    this.node = node;
    this.contentAddress = node.id;
    this.reach = (node.reach as string) || 'commons';
    this.omnibarStewards = (node.stewardedBy ?? []).map(s => ({
      humanId: s.humanId,
      displayName: s.humanId,
      ratio: s.affinity ?? 0,
    }));
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
    this.cdr.markForCheck();
  }

  onNotFound(): void {
    this.error = 'Content not found';
    this.cdr.markForCheck();
  }

  onFailed(): void {
    this.error = 'Failed to load content';
    this.cdr.markForCheck();
  }
}
