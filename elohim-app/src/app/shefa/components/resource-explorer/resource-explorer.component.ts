import { Component, computed, inject, OnInit, signal } from '@angular/core';
import { ActivatedRoute, Router } from '@angular/router';

import { LensRegistryService } from '@app/elohim/services/lens-registry.service';
import { ResourceExplorerService } from '@app/shefa/services/resource-explorer.service';

import { ExplorerBreadcrumbComponent } from './explorer-breadcrumb.component';
import { ExplorerGridComponent } from './explorer-grid.component';
import { ExplorerSidebarComponent } from './explorer-sidebar.component';
import { LensSwitcherComponent } from './lens-switcher.component';

import type { ExplorerBreadcrumb, ExplorerNode, LensDefinition } from '@app/shefa/models';

const DEFAULT_LENS = 'folders';
const ROOT_BREADCRUMB: ExplorerBreadcrumb = { id: null, title: 'My Resources' };

@Component({
  selector: 'app-resource-explorer',
  standalone: true,
  imports: [
    ExplorerSidebarComponent,
    ExplorerBreadcrumbComponent,
    LensSwitcherComponent,
    ExplorerGridComponent,
  ],
  template: `
    <div class="explorer-container" [class.sidebar-collapsed]="!sidebarOpen()">
      <!-- Mobile toggle -->
      <button
        type="button"
        class="sidebar-toggle"
        (click)="toggleSidebar()"
        aria-label="Toggle resource tree"
        data-testid="explorer-sidebar-toggle"
      >
        <span class="material-icons">menu</span>
      </button>

      <!-- Mobile backdrop -->
      <div
        class="sidebar-backdrop"
        (click)="toggleSidebar()"
        role="presentation"
        aria-hidden="true"
        data-testid="explorer-sidebar-backdrop"
      ></div>

      <!-- Sidebar tree -->
      <app-explorer-sidebar
        [tree]="tree()"
        [activeFolderId]="currentFolderId()"
        [activeLens]="activeLens()"
        [activeLensLabel]="activeLensLabel()"
        [lenses]="lenses()"
        (folderSelected)="navigateToFolder($event)"
        (collapseClicked)="toggleSidebar()"
      />

      <!-- Desktop expand button -->
      <button
        type="button"
        class="sidebar-expand-btn"
        (click)="toggleSidebar()"
        aria-label="Expand sidebar"
        data-testid="explorer-sidebar-expand"
      >
        <span class="material-icons">chevron_right</span>
      </button>

      <!-- Main content -->
      <div class="explorer-main">
        <div class="explorer-toolbar">
          <app-lens-switcher
            [lenses]="lenses()"
            [activeLens]="activeLens()"
            (lensChanged)="switchLens($event)"
          />
          <div class="breadcrumb-area">
            <app-explorer-breadcrumb
              [breadcrumbs]="breadcrumbs()"
              (navigate)="navigateToFolder($event)"
            />
          </div>
        </div>

        <div class="explorer-content">
          @if (isLoading()) {
            <div class="loading-state">Loading resources...</div>
          } @else {
            <app-explorer-grid
              [folders]="childFolders()"
              [files]="childFiles()"
              (folderClicked)="navigateToFolder($event)"
              (fileClicked)="openFile($event)"
            />
          }
        </div>
      </div>
    </div>
  `,
  styleUrl: './resource-explorer.component.css',
})
export class ResourceExplorerComponent implements OnInit {
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly lensRegistry = inject(LensRegistryService);
  private readonly explorerService = inject(ResourceExplorerService);

  readonly sidebarOpen = signal(true);
  readonly activeLens = signal(DEFAULT_LENS);
  readonly currentFolderId = signal<string | null>(null);
  readonly breadcrumbs = signal<ExplorerBreadcrumb[]>([ROOT_BREADCRUMB]);
  readonly tree = signal<ExplorerNode[]>([]);
  readonly currentChildren = signal<ExplorerNode[]>([]);
  readonly isLoading = signal(true);

  readonly lenses = computed<LensDefinition[]>(() => this.lensRegistry.lensDefinitions());

  readonly activeLensLabel = computed(() => {
    const lens = this.lenses().find(l => l.type === this.activeLens());
    return lens?.label ?? 'Resources';
  });

  readonly childFolders = computed(() =>
    this.currentChildren().filter(n => n.nodeType === 'folder')
  );

  readonly childFiles = computed(() => this.currentChildren().filter(n => n.nodeType === 'file'));

  ngOnInit(): void {
    // Read route params
    const lensType = this.route.snapshot.paramMap.get('lensType');
    const folderId = this.route.snapshot.paramMap.get('folderId');

    if (lensType) {
      this.activeLens.set(lensType);
    }
    if (folderId) {
      this.currentFolderId.set(decodeURIComponent(folderId));
    }

    this.loadTree();
  }

  toggleSidebar(): void {
    this.sidebarOpen.update(v => !v);
  }

  switchLens(lensType: string): void {
    this.activeLens.set(lensType);
    this.currentFolderId.set(null);
    this.breadcrumbs.set([ROOT_BREADCRUMB]);
    this.loadTree();
    this.updateRoute();
  }

  navigateToFolder(folderId: string | null): void {
    this.currentFolderId.set(folderId);
    this.isLoading.set(true);

    const provider = this.lensRegistry.getProvider(this.activeLens());
    if (!provider) {
      this.isLoading.set(false);
      return;
    }

    if (folderId) {
      provider.getChildren(folderId).subscribe({
        next: children => {
          this.currentChildren.set(children);
          this.isLoading.set(false);
        },
        error: () => this.isLoading.set(false),
      });
      provider.getBreadcrumbs(folderId).subscribe(crumbs => {
        this.breadcrumbs.set(crumbs);
      });
    } else {
      // Navigate to root
      this.breadcrumbs.set([ROOT_BREADCRUMB]);
      provider.getTree().subscribe({
        next: nodes => {
          this.currentChildren.set(nodes);
          this.isLoading.set(false);
        },
        error: () => this.isLoading.set(false),
      });
    }

    this.updateRoute();

    // Close sidebar on mobile
    if (window.innerWidth <= 1024) {
      this.sidebarOpen.set(false);
    }
  }

  openFile(resourceId: string): void {
    void this.router.navigate(['/resource', resourceId]);
  }

  private loadTree(): void {
    this.isLoading.set(true);
    const provider = this.lensRegistry.getProvider(this.activeLens());
    if (!provider) {
      this.isLoading.set(false);
      return;
    }

    provider.getTree().subscribe({
      next: nodes => {
        this.tree.set(nodes);

        // If no folder is selected, show root items in grid
        if (this.currentFolderId()) {
          // Load the selected folder's children
          provider.getChildren(this.currentFolderId()!).subscribe({
            next: children => {
              this.currentChildren.set(children);
              this.isLoading.set(false);
            },
            error: () => this.isLoading.set(false),
          });
        } else {
          this.currentChildren.set(nodes);
          this.isLoading.set(false);
        }
      },
      error: () => this.isLoading.set(false),
    });
  }

  private updateRoute(): void {
    const lens = this.activeLens();
    const folder = this.currentFolderId();

    const segments = ['/shefa/resources'];
    if (lens !== DEFAULT_LENS) {
      segments.push(lens);
    }
    if (folder) {
      segments.push(encodeURIComponent(folder));
    }

    void this.router.navigate(segments, { replaceUrl: true });
  }
}
