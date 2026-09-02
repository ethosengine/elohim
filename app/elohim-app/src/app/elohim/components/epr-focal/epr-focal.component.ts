import { CommonModule } from '@angular/common';
import {
  AfterViewChecked,
  ChangeDetectionStrategy,
  ChangeDetectorRef,
  Component,
  ComponentRef,
  EventEmitter,
  Input,
  OnChanges,
  OnDestroy,
  Output,
  SimpleChanges,
  ViewChild,
  ViewContainerRef,
  inject,
} from '@angular/core';

import { takeUntil } from 'rxjs/operators';

import { Subject } from 'rxjs';

import { ContentNode } from '@app/lamad/models/content-node.model';
import { RendererInitializerService } from '@app/lamad/renderers/renderer-initializer.service';
import {
  ContentRenderer,
  RendererRegistryService,
} from '@app/lamad/renderers/renderer-registry.service';
import { ContentService } from '@app/lamad/services/content.service';

/**
 * The node shape the focal slot loads and hands back. Shell consumers import
 * THIS alias so the lamad content substrate is referenced from one shell file
 * only (the cross-workspace import ratchet counts specifiers per file).
 */
export type FocalNode = ContentNode;

/**
 * EprFocalComponent — the focal render slot of the EPR atom home.
 *
 * Extracted from ContentDeliveryComponent (count-neutral under the import
 * ratchet): slug in, registered renderer hosted, node handed back to the
 * frame. Owns NO chrome, NO legs, NO provenance — only the content itself.
 * Renderer registration is manifest-driven through RendererInitializerService,
 * which must be injected here because this slot mounts outside LamadLayout.
 */
@Component({
  selector: 'app-epr-focal',
  standalone: true,
  imports: [CommonModule],
  changeDetection: ChangeDetectionStrategy.OnPush,
  templateUrl: './epr-focal.component.html',
  styleUrl: './epr-focal.component.css',
})
export class EprFocalComponent implements OnChanges, AfterViewChecked, OnDestroy {
  @Input({ required: true }) slug!: string;
  @Output() readonly nodeLoaded = new EventEmitter<FocalNode>();
  @Output() readonly notFound = new EventEmitter<string>();
  @Output() readonly failed = new EventEmitter<string>();

  @ViewChild('rendererHost', { read: ViewContainerRef, static: false })
  rendererHost!: ViewContainerRef;

  node: FocalNode | null = null;
  isLoading = true;
  hasRegisteredRenderer = false;

  private rendererRef: ComponentRef<ContentRenderer> | null = null;
  private pendingRendererLoad = false;
  private readonly destroy$ = new Subject<void>();
  private readonly contentService = inject(ContentService);
  private readonly rendererRegistry = inject(RendererRegistryService);
  // Injecting triggers manifest-driven renderer registration (side effect).
  private readonly _rendererInit = inject(RendererInitializerService);
  private readonly cdr = inject(ChangeDetectorRef);

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['slug'] && this.slug) this.load(this.slug);
  }

  ngAfterViewChecked(): void {
    if (this.pendingRendererLoad && this.node && this.rendererHost) {
      this.pendingRendererLoad = false;
      this.loadRenderer();
    }
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
    this.destroyRenderer();
  }

  getStringContent(content: string | object): string {
    return typeof content === 'string' ? content : JSON.stringify(content, null, 2);
  }

  private load(slug: string): void {
    this.isLoading = true;
    this.node = null;
    this.destroyRenderer();
    this.contentService
      .getContentBySlug(slug)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: node => {
          this.isLoading = false;
          if (!node) {
            this.notFound.emit(slug);
            this.cdr.markForCheck();
            return;
          }
          this.node = node;
          this.pendingRendererLoad = true;
          this.nodeLoaded.emit(node);
          this.cdr.markForCheck();
        },
        error: () => {
          this.isLoading = false;
          this.failed.emit(slug);
          this.cdr.markForCheck();
        },
      });
  }

  private loadRenderer(): void {
    if (!this.node || !this.rendererHost) return;
    this.destroyRenderer();
    this.rendererHost.clear();
    const rendererComponent = this.rendererRegistry.getRenderer(this.node);
    if (!rendererComponent) {
      this.hasRegisteredRenderer = false;
      this.cdr.markForCheck();
      return;
    }
    this.hasRegisteredRenderer = true;
    this.rendererRef = this.rendererHost.createComponent(rendererComponent);
    this.rendererRef.setInput('node', this.node);
    this.cdr.markForCheck();
  }

  private destroyRenderer(): void {
    if (this.rendererRef) {
      this.rendererRef.destroy();
      this.rendererRef = null;
    }
  }
}
