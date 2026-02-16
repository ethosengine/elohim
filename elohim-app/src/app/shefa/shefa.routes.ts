import { Routes } from '@angular/router';

/**
 * Shefa routing - Economy context app
 *
 * Routes:
 * - /shefa - Economy home (landing page)
 * - /shefa/accounts - Plaid banking accounts (placeholder)
 * - /shefa/transactions - hREA economic events (placeholder)
 * - /shefa/devices - Device stewardship
 * - /shefa/dashboard - Network health and custodian metrics (operator view)
 * - /shefa/resources/* - Property, Energy, Knowledge stewardship (placeholders)
 * - /shefa/exchange - Requests & Offers marketplace (placeholder)
 * - /shefa/insurance - Mutual insurance pools (placeholder)
 * - /shefa/constitutional - Constitutional spending limits (placeholder)
 * - /shefa/planning - Value flow planning (placeholder)
 * - /shefa/settings - Shefa settings (placeholder)
 */
export const SHEFA_ROUTES: Routes = [
  {
    path: '',
    loadComponent: async () =>
      import('./components/shefa-layout/shefa-layout.component').then(m => m.ShefaLayoutComponent),
    children: [
      {
        path: '',
        loadComponent: async () =>
          import('./components/shefa-home/shefa-home.component').then(m => m.ShefaHomeComponent),
        data: {
          title: 'Shefa - Economics of Human Flourishing',
          seo: {
            title: 'Shefa Economy',
            description:
              'Economic coordination layer implementing ValueFlows patterns for multi-dimensional value tracking.',
          },
        },
      },
      {
        path: 'accounts',
        loadComponent: async () =>
          import('./components/shared/shefa-placeholder.component').then(
            m => m.ShefaPlaceholderComponent
          ),
        data: {
          title: 'Shefa - Accounts',
          placeholder: {
            title: 'Accounts',
            description:
              'Connect and manage your financial accounts through Plaid integration for a unified view of your economic resources.',
            features: [
              'Plaid banking integration',
              'Multi-account overview',
              'Balance tracking and history',
              'Account categorization',
            ],
          },
        },
      },
      {
        path: 'transactions',
        loadComponent: async () =>
          import('./components/shared/shefa-placeholder.component').then(
            m => m.ShefaPlaceholderComponent
          ),
        data: {
          title: 'Shefa - Transactions',
          placeholder: {
            title: 'Transactions',
            description:
              'Track economic events using hREA (Resource-Event-Agent) patterns for transparent, auditable value flows.',
            features: [
              'hREA economic event logging',
              'Transaction categorization',
              'Value flow visualization',
              'Export and reporting',
            ],
          },
        },
      },
      {
        path: 'devices',
        loadComponent: async () =>
          import('./components/device-stewardship/device-stewardship.component').then(
            m => m.DeviceStewardshipComponent
          ),
        data: {
          title: 'Shefa - Your Stewardship',
        },
      },
      {
        path: 'dashboard',
        loadComponent: async () =>
          import('../elohim/components/shefa-dashboard/shefa-dashboard.component').then(
            m => m.ShefaDashboardComponent
          ),
        data: {
          title: 'Shefa Dashboard - Network Metrics',
          seo: {
            title: 'Shefa Dashboard',
            description: 'Network health overview and custodian performance metrics for operators.',
          },
        },
      },
      {
        path: 'resources/property',
        loadComponent: async () =>
          import('./components/shared/shefa-placeholder.component').then(
            m => m.ShefaPlaceholderComponent
          ),
        data: {
          title: 'Shefa - Property',
          placeholder: {
            title: 'Property',
            description:
              'Manage real estate and physical property stewardship with transparent ownership records.',
            features: [
              'Property registry',
              'Stewardship agreements',
              'Maintenance scheduling',
              'Community land trust integration',
            ],
          },
        },
      },
      {
        path: 'resources/energy',
        loadComponent: async () =>
          import('./components/shared/shefa-placeholder.component').then(
            m => m.ShefaPlaceholderComponent
          ),
        data: {
          title: 'Shefa - Energy',
          placeholder: {
            title: 'Energy',
            description:
              'Track energy production, consumption, and sharing across the community network.',
            features: [
              'Solar and renewable tracking',
              'Energy credit exchange',
              'Grid contribution metrics',
              'Consumption analytics',
            ],
          },
        },
      },
      {
        path: 'resources/knowledge',
        loadComponent: async () =>
          import('./components/shared/shefa-placeholder.component').then(
            m => m.ShefaPlaceholderComponent
          ),
        data: {
          title: 'Shefa - Knowledge',
          placeholder: {
            title: 'Knowledge',
            description:
              'Value and steward intellectual contributions, curricula, and shared knowledge resources.',
            features: [
              'Knowledge asset registry',
              'Contribution tracking',
              'Curriculum stewardship',
              'Attribution and provenance',
            ],
          },
        },
      },
      {
        path: 'exchange',
        loadComponent: async () =>
          import('./components/shared/shefa-placeholder.component').then(
            m => m.ShefaPlaceholderComponent
          ),
        data: {
          title: 'Shefa - Requests & Offers',
          placeholder: {
            title: 'Requests & Offers',
            description:
              'Create and browse requests and offers within your community using ValueFlows intent patterns.',
            features: [
              'Post requests and offers',
              'Matching algorithm',
              'Commitment tracking',
              'Fulfillment verification',
            ],
          },
        },
      },
      {
        path: 'insurance',
        loadComponent: async () =>
          import('./components/shared/shefa-placeholder.component').then(
            m => m.ShefaPlaceholderComponent
          ),
        data: {
          title: 'Shefa - Insurance',
          placeholder: {
            title: 'Mutual Insurance',
            description:
              'Participate in community mutual insurance pools for shared risk management.',
            features: [
              'Pool creation and membership',
              'Risk assessment dashboard',
              'Claims processing',
              'Premium calculation',
            ],
          },
        },
      },
      {
        path: 'constitutional',
        loadComponent: async () =>
          import('./components/shared/shefa-placeholder.component').then(
            m => m.ShefaPlaceholderComponent
          ),
        data: {
          title: 'Shefa - Constitutional Limits',
          placeholder: {
            title: 'Constitutional Limits',
            description:
              'Define and enforce constitutional spending and resource allocation limits for transparent governance.',
            features: [
              'Spending limit rules',
              'Allocation constraints',
              'Governance proposal integration',
              'Audit trail and compliance',
            ],
          },
        },
      },
      {
        path: 'planning',
        loadComponent: async () =>
          import('./components/shared/shefa-placeholder.component').then(
            m => m.ShefaPlaceholderComponent
          ),
        data: {
          title: 'Shefa - Flow Planning',
          placeholder: {
            title: 'Flow Planning',
            description:
              'Plan and simulate value flows to optimize resource allocation and economic coordination.',
            features: [
              'Value flow modeling',
              'Scenario simulation',
              'Resource optimization',
              'Coordination planning',
            ],
          },
        },
      },
      {
        path: 'settings',
        loadComponent: async () =>
          import('./components/shared/shefa-placeholder.component').then(
            m => m.ShefaPlaceholderComponent
          ),
        data: {
          title: 'Shefa - Settings',
          placeholder: {
            title: 'Settings',
            description: 'Configure your Shefa economy preferences and integrations.',
            features: [
              'Currency preferences',
              'Notification settings',
              'Integration connections',
              'Privacy controls',
            ],
          },
        },
      },
    ],
  },
];
