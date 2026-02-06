import { Routes } from '@angular/router';

/**
 * Community routing - Community context app
 *
 * Routes:
 * - /community - Community home (landing page)
 *
 * Future routes:
 * - /community/human - Community-specific profile settings
 * - /community/governance - Governance dashboard
 * - /community/places - Place-based community coordination
 */
export const COMMUNITY_ROUTES: Routes = [
  {
    path: '',
    loadComponent: async () =>
      import('./components/community-layout/community-layout.component').then(
        m => m.CommunityLayoutComponent
      ),
    children: [
      {
        path: '',
        loadComponent: async () =>
          import('./components/community-home/community-home.component').then(
            m => m.CommunityHomeComponent
          ),
        data: {
          title: 'Community & Governance',
          seo: {
            title: 'Community',
            description:
              'Community coordination layer implementing consent-based relationships and collective governance.',
          },
        },
      },
    ],
  },
];
