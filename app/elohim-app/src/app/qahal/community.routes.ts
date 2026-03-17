import { Routes } from '@angular/router';

import { identityGuard } from '@app/imagodei/guards/identity.guard';

/**
 * Community routing - Community context app
 *
 * Routes:
 * - /community - Community home (landing page)
 * - /community/directory - Community directory
 * - /community/collective/:id - Collective detail
 * - /community/governance/sensemaking - Sensemaking page (query: entityType, entityId)
 * - /community/governance/challenges - Challenge list
 * - /community/governance/challenges/new - File a new challenge
 * - /community/governance/challenges/:id - Challenge detail
 *
 * - /community/governance/disposition - Governance disposition profile
 * - /community/governance/proxy-votes - Proxy vote notifications
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
      {
        path: 'directory',
        loadComponent: async () =>
          import('./components/community-directory/community-directory.component').then(
            m => m.CommunityDirectoryComponent
          ),
        data: {
          title: 'Community Directory',
        },
      },
      {
        path: 'collective/:id',
        loadComponent: async () =>
          import('./components/collective-detail/collective-detail.component').then(
            m => m.CollectiveDetailComponent
          ),
        data: { title: 'Collective' },
      },
      {
        path: 'governance/sensemaking',
        loadComponent: async () =>
          import('./components/sensemaking-page/sensemaking-page.component').then(
            m => m.SensemakingPageComponent
          ),
        data: { title: 'Community Sensemaking' },
      },
      {
        path: 'governance/challenges',
        loadComponent: async () =>
          import('./components/challenge-list/challenge-list.component').then(
            m => m.ChallengeListComponent
          ),
        data: { title: 'Governance Challenges' },
      },
      {
        path: 'governance/challenges/new',
        canActivate: [identityGuard],
        loadComponent: async () =>
          import('./components/challenge-route/challenge-route.component').then(
            m => m.ChallengeRouteComponent
          ),
        data: { title: 'File a Challenge' },
      },
      {
        path: 'governance/challenges/:id',
        loadComponent: async () =>
          import('./components/challenge-detail/challenge-detail.component').then(
            m => m.ChallengeDetailComponent
          ),
        data: { title: 'Challenge Detail' },
      },
      {
        path: 'governance/disposition',
        loadComponent: async () =>
          import('./components/governance-disposition/governance-disposition.component').then(
            m => m.GovernanceDispositionComponent
          ),
        data: { title: 'Governance Profile' },
      },
      {
        path: 'governance/proxy-votes',
        loadComponent: async () =>
          import('./components/proxy-vote-list/proxy-vote-list.component').then(
            m => m.ProxyVoteListComponent
          ),
        data: { title: 'Proxy Vote Notifications' },
      },
    ],
  },
];
