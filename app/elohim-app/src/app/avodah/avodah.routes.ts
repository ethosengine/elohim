import { Routes } from '@angular/router';

import { identityGuard } from '@app/imagodei/guards/identity.guard';

/**
 * Avodah routing - Work management pillar
 *
 * Routes:
 * - /avodah - Home (landing page)
 * - /avodah/projects - Project list
 * - /avodah/projects/:id/board - Kanban board
 * - /avodah/projects/:id/backlog - Backlog table
 * - /avodah/projects/:id/tasks - Recurring task list
 */
export const AVODAH_ROUTES: Routes = [
  {
    path: '',
    loadComponent: async () =>
      import('./components/avodah-layout/avodah-layout.component').then(
        m => m.AvodahLayoutComponent
      ),
    children: [
      {
        path: '',
        loadComponent: async () =>
          import('./components/avodah-home/avodah-home.component').then(m => m.AvodahHomeComponent),
        data: { title: 'Avodah — Work Management' },
      },
      {
        path: 'projects',
        loadComponent: async () =>
          import('./components/project-list/project-list.component').then(
            m => m.ProjectListComponent
          ),
        data: { title: 'Avodah — Projects' },
      },
      {
        path: 'projects/:id/board',
        canActivate: [identityGuard],
        loadComponent: async () =>
          import('./components/project-board/project-board.component').then(
            m => m.ProjectBoardComponent
          ),
        data: { title: 'Avodah — Board' },
      },
      {
        path: 'projects/:id/backlog',
        canActivate: [identityGuard],
        loadComponent: async () =>
          import('./components/project-backlog/project-backlog.component').then(
            m => m.ProjectBacklogComponent
          ),
        data: { title: 'Avodah — Backlog' },
      },
      {
        path: 'projects/:id/stories/:storyId',
        canActivate: [identityGuard],
        loadComponent: async () =>
          import('./components/story-detail/story-detail.component').then(
            m => m.StoryDetailComponent
          ),
        data: { title: 'Avodah — Story' },
      },
      {
        path: 'projects/:id/tasks',
        canActivate: [identityGuard],
        loadComponent: async () =>
          import('./components/task-list/task-list.component').then(m => m.TaskListComponent),
        data: { title: 'Avodah — Tasks' },
      },
    ],
  },
];
