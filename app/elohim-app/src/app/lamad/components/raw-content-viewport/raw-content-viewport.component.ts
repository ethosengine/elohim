import { CommonModule, Location } from '@angular/common';
import {
  AfterViewChecked,
  ChangeDetectionStrategy,
  Component,
  ComponentRef,
  OnDestroy,
  OnInit,
  ViewChild,
  ViewContainerRef,
  inject,
  signal,
} from '@angular/core';
import { ActivatedRoute } from '@angular/router';

import { takeUntil } from 'rxjs/operators';

import { Subject } from 'rxjs';

import { ProtocolSignalBadgeComponent } from '@app/elohim/components/protocol-signal-badge/protocol-signal-badge.component';
import { DataLoaderService } from '@app/elohim/services/data-loader.service';
import { ContentNode } from '@app/lamad/models/content-node.model';
import {
  ContentRenderer,
  RendererRegistryService,
} from '@app/lamad/renderers/renderer-registry.service';

/**
 * Raw Content Viewport — the DOM-tier of the progressive protocol-viewer.
 *
 * Renders a ContentNode full-window via the existing renderer registry,
 * hosts the protocol-signal badge in a fixed corner, and offers a single
 * exit affordance. Replaces the previous `isFocusedView` mechanism that
 * lived inside `ContentViewerComponent`.
 *
 * Reachable via the route `/raw/:resourceId`.
 */
@Component({
  selector: 'app-raw-content-viewport',
  standalone: true,
  imports: [CommonModule, ProtocolSignalBadgeComponent],
  templateUrl: './raw-content-viewport.component.html',
  styleUrls: ['./raw-content-viewport.component.css'],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class RawContentViewportComponent implements OnInit, OnDestroy, AfterViewChecked {
  @ViewChild('rendererHost', { read: ViewContainerRef, static: false })
  rendererHost?: ViewContainerRef;

  readonly node = signal<ContentNode | null>(null);
  readonly error = signal<string | null>(null);

  private rendererRef: ComponentRef<ContentRenderer> | null = null;
  private pendingLoad = false;

  private readonly destroy$ = new Subject<void>();
  private readonly route = inject(ActivatedRoute);
  private readonly dataLoader = inject(DataLoaderService);
  private readonly rendererRegistry = inject(RendererRegistryService);
  private readonly location = inject(Location);

  ngOnInit(): void {
    this.route.paramMap.pipe(takeUntil(this.destroy$)).subscribe(params => {
      const resourceId = params.get('resourceId');
      if (!resourceId) {
        this.error.set('Missing resource id');
        return;
      }
      this.dataLoader
        .getContent(resourceId)
        .pipe(takeUntil(this.destroy$))
        .subscribe({
          next: node => {
            if (!node) {
              this.error.set('Content not found');
              return;
            }
            this.node.set(node);
            this.pendingLoad = true;
          },
          error: () => {
            this.error.set('Failed to load content');
          },
        });
    });
  }

  ngAfterViewChecked(): void {
    if (this.pendingLoad && this.node() && this.rendererHost) {
      this.pendingLoad = false;
      this.mountRenderer();
    }
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
    this.tearDownRenderer();
  }

  exit(): void {
    this.location.back();
  }

  private mountRenderer(): void {
    const node = this.node();
    if (!node || !this.rendererHost) return;
    this.tearDownRenderer();
    this.rendererHost.clear();

    const rendererComponent = this.rendererRegistry.getRenderer(node);
    if (!rendererComponent) return;

    this.rendererRef = this.rendererHost.createComponent(rendererComponent);
    this.rendererRef.setInput('node', node);
  }

  private tearDownRenderer(): void {
    if (this.rendererRef) {
      this.rendererRef.destroy();
      this.rendererRef = null;
    }
  }
}
