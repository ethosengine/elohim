import { Component, output } from '@angular/core';
import { RouterLink, RouterLinkActive } from '@angular/router';

export interface NavItem {
  label: string;
  icon: string;
  route: string;
  exact?: boolean;
}

export interface NavGroup {
  title: string;
  items: NavItem[];
}

export const SHEFA_NAV_GROUPS: NavGroup[] = [
  {
    title: 'Primary',
    items: [
      { label: 'Overview', icon: 'home', route: '/shefa', exact: true },
      { label: 'Accounts', icon: 'account_balance', route: '/shefa/accounts' },
      { label: 'Transactions', icon: 'receipt_long', route: '/shefa/transactions' },
      { label: 'Devices', icon: 'devices', route: '/shefa/devices' },
    ],
  },
  {
    title: 'Resources',
    items: [
      { label: 'Property', icon: 'real_estate_agent', route: '/shefa/resources/property' },
      { label: 'Energy', icon: 'bolt', route: '/shefa/resources/energy' },
      { label: 'Knowledge', icon: 'school', route: '/shefa/resources/knowledge' },
    ],
  },
  {
    title: 'Community',
    items: [
      { label: 'Requests & Offers', icon: 'swap_horiz', route: '/shefa/exchange' },
      { label: 'Insurance', icon: 'shield', route: '/shefa/insurance' },
      { label: 'Constitutional', icon: 'gavel', route: '/shefa/constitutional' },
    ],
  },
  {
    title: 'Management',
    items: [
      { label: 'Network Dashboard', icon: 'monitoring', route: '/shefa/dashboard' },
      { label: 'Flow Planning', icon: 'timeline', route: '/shefa/planning' },
      { label: 'Settings', icon: 'settings', route: '/shefa/settings' },
    ],
  },
];

@Component({
  selector: 'app-shefa-sidenav',
  standalone: true,
  imports: [RouterLink, RouterLinkActive],
  template: `
    <aside class="shefa-sidenav" role="navigation" aria-label="Shefa navigation">
      <div class="sidebar-header">
        <span class="sidebar-context">Shefa</span>
        <h2 class="sidebar-title">Economy</h2>
      </div>

      <nav class="sidenav-content">
        @for (group of navGroups; track group.title) {
          <div class="nav-group">
            <span class="nav-group-title">{{ group.title }}</span>
            @for (item of group.items; track item.route) {
              <a
                class="nav-item"
                [routerLink]="item.route"
                routerLinkActive="active"
                [routerLinkActiveOptions]="{ exact: item.exact ?? false }"
                (click)="navItemClicked.emit()"
              >
                <span class="material-icons nav-icon">{{ item.icon }}</span>
                <span class="nav-label">{{ item.label }}</span>
              </a>
            }
          </div>
        }
      </nav>

      <button
        type="button"
        class="sidebar-collapse-btn"
        (click)="collapseClicked.emit()"
        aria-label="Collapse sidebar"
      >
        <span class="material-icons collapse-icon">chevron_left</span>
      </button>
    </aside>
  `,
  styles: [
    `
      .shefa-sidenav {
        width: 260px;
        flex-shrink: 0;
        display: flex;
        flex-direction: column;
        background: var(--lamad-bg-secondary, #1a1a2e);
        border-right: 1px solid var(--lamad-border, rgba(99, 102, 241, 0.15));
        overflow: hidden;
        transition:
          width 0.3s ease,
          transform 0.3s ease;
      }

      .sidebar-header {
        padding: 1.25rem;
        border-bottom: 1px solid var(--lamad-border, rgba(99, 102, 241, 0.15));
      }

      .sidebar-context {
        display: block;
        font-size: 0.75rem;
        font-weight: 500;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        color: var(--lamad-text-muted, #64748b);
        margin-bottom: 0.25rem;
      }

      .sidebar-title {
        font-size: 1.125rem;
        font-weight: 700;
        color: var(--lamad-text-primary, #f8fafc);
        margin: 0;
        line-height: 1.3;
      }

      .sidenav-content {
        flex: 1;
        overflow-y: auto;
        padding: 0.5rem 0;
        scrollbar-width: thin;
        scrollbar-color: rgba(99, 102, 241, 0.2) transparent;
      }

      .nav-group {
        padding: 0.5rem 0;
      }

      .nav-group + .nav-group {
        border-top: 1px solid var(--lamad-border, rgba(99, 102, 241, 0.1));
      }

      .nav-group-title {
        display: block;
        padding: 0.5rem 1.25rem 0.25rem;
        font-size: 0.6875rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        color: var(--lamad-text-muted, #64748b);
      }

      .nav-item {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        padding: 0.5rem 1.25rem;
        color: var(--lamad-text-secondary, #e2e8f0);
        text-decoration: none;
        font-size: 0.875rem;
        font-weight: 400;
        white-space: nowrap;
        transition:
          background-color 0.15s,
          color 0.15s;
      }

      .nav-item:hover {
        background-color: rgba(99, 102, 241, 0.08);
        color: var(--lamad-text-primary, #f8fafc);
      }

      .nav-item.active {
        background-color: rgba(99, 102, 241, 0.12);
        color: var(--lamad-accent-primary, #6366f1);
        font-weight: 500;
      }

      .nav-icon {
        font-size: 1.25rem;
        flex-shrink: 0;
        width: 1.25rem;
        text-align: center;
      }

      .nav-label {
        overflow: hidden;
        text-overflow: ellipsis;
      }

      .sidebar-collapse-btn {
        display: flex;
        align-items: center;
        justify-content: center;
        width: 100%;
        padding: 0.75rem;
        background: transparent;
        border: none;
        border-top: 1px solid var(--lamad-border, rgba(99, 102, 241, 0.1));
        cursor: pointer;
        color: var(--lamad-text-muted, #64748b);
        transition:
          background-color 0.2s,
          color 0.2s;
      }

      .sidebar-collapse-btn:hover {
        background-color: rgba(99, 102, 241, 0.08);
        color: var(--lamad-text-secondary, #e2e8f0);
      }

      .collapse-icon {
        font-size: 1.25rem;
      }
    `,
  ],
})
export class ShefaSidenavComponent {
  readonly collapseClicked = output<void>();
  readonly navItemClicked = output<void>();

  readonly navGroups = SHEFA_NAV_GROUPS;
}
