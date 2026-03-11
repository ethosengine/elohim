# Progress Data Migration Guide

## Overview

The `ProgressMigrationService` migrates existing learning path progress data to support **cross-path completion tracking** (Khan Academy-style shared completion).

## Background

### Before Migration (Phase 1)
- Progress tracked per-path with `completedStepIndices`
- Completing content in one path didn't affect other paths
- No way to show "already mastered" content across multiple paths

### After Migration (Phase 2)
- Global content completion tracked in special `__global__` progress record
- Content completed in ANY path shows as completed in ALL paths
- Enables Khan Academy-style shared completion views

## What the Migration Does

1. **Scans localStorage** for all existing progress records
2. **Extracts resourceIds** from completed steps in each path
3. **Populates `__global__` progress** with `completedContentIds` array
4. **Preserves existing progress** - no data is lost or overwritten

## How to Run the Migration

### Option 1: Angular Component (Recommended for Production)

Add this to your admin dashboard or settings page:

```typescript
import { Component } from '@angular/core';
import { ProgressMigrationService } from '../services/progress-migration.service';

@Component({
  selector: 'app-migration-dashboard',
  template: `
    <div class="migration-panel">
      <h2>Progress Data Migration</h2>

      <button (click)="previewMigration()">Preview Migration</button>
      <button (click)="runMigration()">Run Migration</button>
      <button (click)="verifyMigration()">Verify Migration</button>

      <pre *ngIf="result">{{ result | json }}</pre>
    </div>
  `
})
export class MigrationDashboardComponent {
  result: any;

  constructor(private migrationService: ProgressMigrationService) {}

  previewMigration() {
    this.migrationService.previewMigration().subscribe(
      result => this.result = result
    );
  }

  runMigration() {
    if (confirm('Run progress migration? This will modify localStorage.')) {
      this.migrationService.migrateAllProgress().subscribe(
        result => this.result = result
      );
    }
  }

  verifyMigration() {
    this.migrationService.verifyMigration().subscribe(
      result => this.result = result
    );
  }
}
```

### Option 2: Browser DevTools Console

Open browser console on your app and run:

```javascript
// Get the migration service from Angular injector
const injector = ng.probe(document.querySelector('app-root')).injector;
const migrationService = injector.get('ProgressMigrationService');

// Preview what will be migrated (dry run)
migrationService.previewMigration().subscribe(
  result => console.log('Preview:', result)
);

// Run the actual migration
migrationService.migrateAllProgress().subscribe(
  result => console.log('Migration complete:', result)
);

// Verify migration succeeded
migrationService.verifyMigration().subscribe(
  result => console.log('Verification:', result)
);
```

### Option 3: One-time Startup Migration

Add to your `AppComponent` to run automatically on first load:

```typescript
export class AppComponent implements OnInit {
  constructor(private migrationService: ProgressMigrationService) {}

  ngOnInit() {
    // Check if migration already ran
    const migrationKey = 'lamad-migration-v1-completed';
    if (!localStorage.getItem(migrationKey)) {
      this.migrationService.migrateAllProgress().subscribe(
        result => {
          console.log('Progress migration completed:', result);
          localStorage.setItem(migrationKey, 'true');
        },
        error => {
          console.error('Migration failed:', error);
        }
      );
    }
  }
}
```

## Migration Methods

### `previewMigration()`
**Dry run** - shows what would be migrated without making changes.

Returns:
```typescript
{
  agents: Array<{
    agentId: string;
    pathCount: number;
    estimatedContentNodes: number;
  }>;
  totalAgents: number;
  totalPaths: number;
  estimatedContentNodes: number;
}
```

### `migrateAllProgress()`
**Runs the migration** - creates/updates `__global__` progress records.

Returns:
```typescript
{
  agentsMigrated: number;
  pathsMigrated: number;
  contentNodesMigrated: number;
  errors: string[];
}
```

### `verifyMigration()`
**Validates migration** - checks all agents have `__global__` progress.

Returns:
```typescript
{
  valid: boolean;
  agentsWithProgress: number;
  agentsWithGlobalProgress: number;
  missingGlobalProgress: string[];
}
```

## Example Output

### Preview
```json
{
  "totalAgents": 3,
  "totalPaths": 8,
  "estimatedContentNodes": 142,
  "agents": [
    {
      "agentId": "session-abc123",
      "pathCount": 3,
      "estimatedContentNodes": 45
    },
    ...
  ]
}
```

### Migration
```json
{
  "agentsMigrated": 3,
  "pathsMigrated": 8,
  "contentNodesMigrated": 89,
  "errors": []
}
```

Note: `contentNodesMigrated` is often less than `estimatedContentNodes` because the same content appears in multiple paths.

### Verification
```json
{
  "valid": true,
  "agentsWithProgress": 3,
  "agentsWithGlobalProgress": 3,
  "missingGlobalProgress": []
}
```

## Safety

- **Non-destructive**: Existing progress records are NOT modified
- **Idempotent**: Safe to run multiple times
- **Mergeable**: If `__global__` already exists, content IDs are merged (no duplicates)
- **Error handling**: Individual agent failures don't stop entire migration

## Rollback

If needed, you can remove the `__global__` progress records:

```javascript
// Remove all __global__ progress records
for (let i = localStorage.length - 1; i >= 0; i--) {
  const key = localStorage.key(i);
  if (key?.includes('__global__')) {
    localStorage.removeItem(key);
  }
}
```

## When to Run

- **After deploying**: Cross-path completion feature (Phase 2)
- **Before UI handoff**: Ensures UI developers have complete data
- **For existing users**: Migrates their historical progress

## Notes

- Migration scans ALL agents in localStorage (not just current user)
- Typical migration takes <1 second for 10 agents
- No server-side changes needed - localStorage only
- Compatible with both session and authenticated users
