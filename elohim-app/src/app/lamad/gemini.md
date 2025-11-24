# Lamad Routing & Integration Guide

Ref: `LAMAD_API_SPECIFICATION_v1.0.md` - Part 1: URL Strategy

## Objective
Align the Angular Router configuration with the v1.0 specification.

## Tasks

### 1. Update `lamad.routes.ts`
- [ ] Refactor routes to match the spec:
  ```typescript
  const routes: Routes = [
    { path: '', component: LamadHomeComponent }, // Landing
    { path: 'path/:pathId/step/:stepIndex', component: PathNavigatorComponent }, // The Journey
    { path: 'path/:pathId', component: PathOverviewComponent }, // The Map
    { path: 'resource/:resourceId', component: ResourceViewerComponent }, // The Territory (Direct)
    { path: 'explore', component: GraphExplorerComponent }, // Research
    { path: 'me', loadChildren: ... } // Agent Profile
  ];
  ```
- [ ] **Note**: The existing `content/:id` route should be deprecated or mapped to `resource/:resourceId`.

### 2. Layout Integration
- [ ] Ensure `LamadLayoutComponent` supports the new views.
- [ ] The "Journey" view (PathNavigator) might need a specialized layout (distraction-free) compared to the "Explore" view.

### 3. Route Guards
- [ ] Implement `AuthGuard` for `/me/*` routes.
- [ ] Implement `AttestationGuard` for restricted resources (if checking at route level, though usually handled by Service returning "Locked" state).

## Holochain Alignment
- Ensure route parameters (IDs) are treated as opaque strings. They will eventually be Hash Strings (e.g., `uhCkk...`). Do not rely on them being integers or specific slugs.
