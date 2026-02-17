import { Component, signal } from '@angular/core';
import { RouterOutlet } from '@angular/router';

import { ElohimNavigatorComponent } from '@app/elohim/components/elohim-navigator/elohim-navigator.component';

import { ShefaSidenavComponent } from '../shefa-sidenav/shefa-sidenav.component';

@Component({
  selector: 'app-shefa-layout',
  standalone: true,
  imports: [RouterOutlet, ElohimNavigatorComponent, ShefaSidenavComponent],
  template: `
    <div class="shefa-container">
      <app-elohim-navigator [context]="'shefa'" [showSearch]="false">
        <div class="shefa-workspace" [class.sidebar-collapsed]="!sidebarOpen()">
          <!-- Mobile toggle -->
          <button
            type="button"
            class="sidebar-toggle"
            (click)="toggleSidebar()"
            aria-label="Toggle navigation menu"
          >
            <span class="material-icons">menu</span>
          </button>

          <!-- Mobile backdrop -->
          <div class="sidebar-backdrop" (click)="toggleSidebar()" role="presentation"></div>

          <!-- Sidebar -->
          <app-shefa-sidenav
            (collapseClicked)="toggleSidebar()"
            (navItemClicked)="onNavItemClicked()"
          />

          <!-- Desktop expand button -->
          <button
            type="button"
            class="sidebar-expand-btn"
            (click)="toggleSidebar()"
            aria-label="Expand sidebar"
          >
            <span class="material-icons">chevron_right</span>
          </button>

          <!-- Main content -->
          <main class="shefa-content">
            <div class="content-scroll-container">
              <router-outlet></router-outlet>
            </div>
          </main>
        </div>
      </app-elohim-navigator>
    </div>
  `,
  styles: [
    `
      .shefa-container {
        min-height: 100vh;
        display: flex;
        flex-direction: column;
        background: linear-gradient(
          135deg,
          var(--lamad-bg-primary, #0f0f1a) 0%,
          var(--lamad-bg-secondary, #1a1a2e) 100%
        );
        color: var(--lamad-text-secondary, #e2e8f0);
      }

      .shefa-workspace {
        display: flex;
        height: calc(100dvh - 64px);
        position: relative;
      }

      /* Main content area */
      .shefa-content {
        flex: 1;
        display: flex;
        flex-direction: column;
        overflow: hidden;
        min-width: 0;
      }

      .content-scroll-container {
        flex: 1;
        overflow-y: auto;
      }

      /* ---- Elements always in DOM, visibility via CSS ---- */

      /* Mobile toggle: hidden on desktop */
      .sidebar-toggle {
        display: none;
        position: absolute;
        top: 0.75rem;
        left: 0.75rem;
        z-index: 90;
        width: 40px;
        height: 40px;
        align-items: center;
        justify-content: center;
        background: var(--lamad-surface, rgba(30, 30, 46, 0.9));
        border: 1px solid var(--lamad-border, rgba(99, 102, 241, 0.15));
        border-radius: 0.5rem;
        cursor: pointer;
        color: var(--lamad-text-secondary, #e2e8f0);
      }

      .sidebar-toggle:hover {
        background: rgba(99, 102, 241, 0.12);
      }

      /* Backdrop: hidden by default */
      .sidebar-backdrop {
        display: none;
        position: fixed;
        inset: 0;
        top: 64px;
        background: rgb(0 0 0 / 40%);
        z-index: 50;
        cursor: pointer;
      }

      /* Expand button: hidden by default */
      .sidebar-expand-btn {
        display: none;
        align-items: center;
        justify-content: center;
        position: absolute;
        left: 0;
        top: 50%;
        transform: translateY(-50%);
        z-index: 25;
        padding: 1rem 0.375rem;
        background: var(--lamad-bg-secondary, #1a1a2e);
        border: 1px solid var(--lamad-border, rgba(99, 102, 241, 0.15));
        border-left: none;
        border-radius: 0 0.375rem 0.375rem 0;
        cursor: pointer;
        color: var(--lamad-text-muted, #64748b);
        transition:
          background-color 0.2s,
          color 0.2s;
      }

      .sidebar-expand-btn:hover {
        background: rgba(99, 102, 241, 0.12);
        color: var(--lamad-text-secondary, #e2e8f0);
      }

      /* ---- Desktop collapsed state ---- */

      .shefa-workspace.sidebar-collapsed app-shefa-sidenav {
        width: 0;
        overflow: hidden;
        border-right: none;
      }

      .shefa-workspace.sidebar-collapsed .sidebar-expand-btn {
        display: flex;
      }

      /* ---- Tablet & Mobile ---- */

      @media (width <= 1024px) {
        .sidebar-toggle {
          display: flex;
        }

        /* Never show desktop expand on mobile */
        .sidebar-expand-btn {
          display: none !important;
        }

        /* Sidebar becomes fixed overlay */
        app-shefa-sidenav {
          position: fixed;
          top: 64px;
          left: 0;
          bottom: 0;
          z-index: 100;
        }

        .shefa-workspace.sidebar-collapsed app-shefa-sidenav {
          transform: translateX(-100%);
          width: 260px;
        }

        /* Show backdrop when sidebar is open */
        .shefa-workspace:not(.sidebar-collapsed) .sidebar-backdrop {
          display: block;
        }

        /* Hide collapse button on mobile — use backdrop/toggle instead */
        .shefa-workspace:not(.sidebar-collapsed) .sidebar-collapse-btn {
          display: none;
        }
      }

      @media (width <= 768px) {
        app-shefa-sidenav {
          width: 100%;
        }

        .shefa-workspace.sidebar-collapsed app-shefa-sidenav {
          width: 100%;
        }
      }
    `,
  ],
})
export class ShefaLayoutComponent {
  readonly sidebarOpen = signal(true);

  toggleSidebar(): void {
    this.sidebarOpen.update(v => !v);
  }

  onNavItemClicked(): void {
    if (window.innerWidth <= 1024) {
      this.sidebarOpen.set(false);
    }
  }
}
