# Qahal Community Directory Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a community directory grid showing faces and names, organized by household and small group, as Sprint 1 of the Governance Immune System.

**Architecture:** Add `profile_photo_url` field to Human model chain (Rust → TS), add a "list humans" endpoint (none exists today), build a `CommunityDirectoryComponent` with `FaceCardComponent` in the qahal pillar, and update seed data with household participation links. No new tables or entities — households are `Collective(family)`.

**Tech Stack:** Rust/Diesel (elohim-storage), Angular 19 standalone components, TypeScript, Vitest

---

### Task 1: Add `profile_photo_url` to Diesel schema and models

**Files:**
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs:346-358`
- Modify: `elohim/elohim-storage/src/db/models.rs:717-743`
- Create: `elohim/elohim-storage/migrations/2026-03-15-000001_add_human_profile_photo/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-03-15-000001_add_human_profile_photo/down.sql`

**Step 1: Create migration files**

```sql
-- up.sql
ALTER TABLE humans ADD COLUMN profile_photo_url TEXT;
```

```sql
-- down.sql
ALTER TABLE humans DROP COLUMN profile_photo_url;
```

**Step 2: Add field to diesel_schema.rs**

In the `humans` table macro, add after `app_id -> Text,`:
```rust
        profile_photo_url -> Nullable<Text>,
```

**Step 3: Add field to model structs**

In `Human` struct (line ~728), add before closing brace:
```rust
    pub profile_photo_url: Option<String>,
```

In `NewHuman` struct (line ~742), add before closing brace:
```rust
    pub profile_photo_url: Option<String>,
```

**Step 4: Verify it compiles**

Run: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check 2>&1 | head -30`
Expected: Compilation errors in views.rs and humans.rs (they reference the struct but don't have the new field yet). That's expected — we fix those in Tasks 2 and 3.

**Step 5: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-03-15-000001_add_human_profile_photo/ elohim/elohim-storage/src/db/diesel_schema.rs elohim/elohim-storage/src/db/models.rs
git commit -m "feat(storage): add profile_photo_url column to humans table"
```

---

### Task 2: Add `profile_photo_url` to views and input types

**Files:**
- Modify: `elohim/elohim-storage/src/views.rs:2239-2310` (HumanView, CreateHumanInputView, UpdateHumanInputView)

**Step 1: Add to HumanView struct**

Find the `HumanView` struct. Add after `location`:
```rust
    pub profile_photo_url: Option<String>,
```

**Step 2: Add to `From<Human> for HumanView` impl**

In the `impl From<Human> for HumanView` block, add the mapping:
```rust
            profile_photo_url: h.profile_photo_url,
```

**Step 3: Add to CreateHumanInputView**

Add after `location`:
```rust
    #[serde(default)]
    pub profile_photo_url: Option<String>,
```

**Step 4: Add to UpdateHumanInputView**

Add after `location`:
```rust
    #[serde(default)]
    pub profile_photo_url: Option<String>,
```

**Step 5: Commit**

```bash
git add elohim/elohim-storage/src/views.rs
git commit -m "feat(storage): add profile_photo_url to Human view and input types"
```

---

### Task 3: Update DB CRUD functions and API handlers

**Files:**
- Modify: `elohim/elohim-storage/src/db/humans.rs:18-41` (CreateHumanInput, UpdateHumanInput)
- Modify: `elohim/elohim-storage/src/db/humans.rs:108+` (update_human function)
- Modify: `elohim/elohim-storage/src/api/identity.rs` (register_human, update_me handlers)

**Step 1: Add to CreateHumanInput**

In `CreateHumanInput` struct, add:
```rust
    pub profile_photo_url: Option<String>,
```

**Step 2: Add to UpdateHumanInput**

In `UpdateHumanInput` struct, add:
```rust
    pub profile_photo_url: Option<String>,
```

**Step 3: Add to update_human function**

Find the `update_human` function. In the diesel `.set()` call, add:
```rust
        if let Some(ref url) = input.profile_photo_url {
            diesel::update(humans::table.filter(humans::id.eq(human_id)))
                .set(humans::profile_photo_url.eq(url))
                .execute(conn)?;
        }
```

Follow the exact pattern used for the other optional fields in the same function.

**Step 4: Update register_human handler**

In `identity.rs`, in the `register_human` handler where `CreateHumanInput` is constructed from `CreateHumanInputView`, add:
```rust
            profile_photo_url: input.profile_photo_url,
```

**Step 5: Update update_me handler**

In `identity.rs`, in the `update_me` handler where `UpdateHumanInput` is constructed from `UpdateHumanInputView`, add:
```rust
            profile_photo_url: input.profile_photo_url,
```

**Step 6: Verify it compiles**

Run: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check 2>&1 | tail -5`
Expected: No errors.

**Step 7: Commit**

```bash
git add elohim/elohim-storage/src/db/humans.rs elohim/elohim-storage/src/api/identity.rs
git commit -m "feat(storage): wire profile_photo_url through CRUD and API handlers"
```

---

### Task 4: Add "list humans" endpoint

**Context:** There is currently NO endpoint to list all humans. Only `get_human_by_id` and `get_human_by_agent_key` exist. The directory needs a list endpoint.

**Files:**
- Modify: `elohim/elohim-storage/src/db/humans.rs` (add list_humans function)
- Modify: `elohim/elohim-storage/src/http.rs` (add route + handler)

**Step 1: Add list_humans DB function**

In `humans.rs`, add after the existing `get_human_by_agent_key` function:

```rust
/// List all humans, optionally filtered by app_id.
pub fn list_humans(
    conn: &mut SqliteConnection,
    app_id: &str,
) -> Result<Vec<Human>, diesel::result::Error> {
    use crate::db::diesel_schema::humans::dsl;
    dsl::humans
        .filter(dsl::app_id.eq(app_id))
        .order(dsl::display_name.asc())
        .load::<Human>(conn)
}
```

**Step 2: Add HTTP handler in http.rs**

Find the identity section in http.rs (around line 5799). Add a new handler for listing humans. Follow the same pattern as `handle_list_collectives`:

The route should be `GET /db/humans` returning `{ "items": [...HumanView] }`.

Look at how `handle_list_collectives` works (around line 4427) — the list_humans handler should follow the same pattern:
1. Get connection from pool
2. Get app_id from context
3. Call `list_humans(&mut conn, app_id)`
4. Map results to `Vec<HumanView>` via `.into_iter().map(HumanView::from).collect()`
5. Return JSON `{ "items": views }`

**Step 3: Register the route**

In the route registration section of http.rs (around line 6086 near the collective routes), add:

```rust
        .route(
            Route::get("/db/humans")
                .handler("list_humans")
                .cache_ttl(300)
                .build(),
        )
```

**Step 4: Verify it compiles**

Run: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check 2>&1 | tail -5`
Expected: No errors.

**Step 5: Commit**

```bash
git add elohim/elohim-storage/src/db/humans.rs elohim/elohim-storage/src/http.rs
git commit -m "feat(storage): add GET /db/humans endpoint for community directory"
```

---

### Task 5: Regenerate TypeScript types

**Step 1: Run export_bindings**

Run: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings 2>&1 | tail -10`

Expected: Test passes. Check that `HumanView.ts`, `CreateHumanInputView.ts`, and `UpdateHumanInputView.ts` in `elohim/sdk/storage-client-ts/src/generated/` now include `profilePhotoUrl`.

**Step 2: Verify generated types**

Run: `grep profilePhotoUrl /projects/elohim/elohim/sdk/storage-client-ts/src/generated/HumanView.ts`
Expected: `profilePhotoUrl: string | null,`

**Step 3: Commit**

```bash
git add elohim/sdk/storage-client-ts/src/generated/
git commit -m "chore: regenerate TypeScript types with profilePhotoUrl"
```

---

### Task 6: Add proxy route for /db/humans

**Files:**
- Modify: `app/elohim-app/proxy.conf.mjs`

**Step 1: Check if /db/* is already proxied**

Read `app/elohim-app/proxy.conf.mjs`. If `/db` is already in the context array, no change needed — the new `/db/humans` route will be proxied automatically.

If NOT already proxied, add `/db/humans` to the context array.

**Step 2: Commit (only if change was needed)**

```bash
git add app/elohim-app/proxy.conf.mjs
git commit -m "chore: add /db/humans to dev proxy"
```

---

### Task 7: Build FaceCardComponent

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/face-card/face-card.component.ts`
- Create: `app/elohim-app/src/app/qahal/components/face-card/face-card.component.html`
- Create: `app/elohim-app/src/app/qahal/components/face-card/face-card.component.css`
- Create: `app/elohim-app/src/app/qahal/components/face-card/face-card.component.spec.ts`

**Step 1: Write the test**

```typescript
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { FaceCardComponent } from './face-card.component';

describe('FaceCardComponent', () => {
  let component: FaceCardComponent;
  let fixture: ComponentFixture<FaceCardComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [FaceCardComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(FaceCardComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    component.displayName = 'Matthew Dowell';
    component.humanId = 'human-matthew-manager';
    fixture.detectChanges();
    expect(component).toBeTruthy();
  });

  it('should compute initials from display name', () => {
    component.displayName = 'Matthew Dowell';
    component.humanId = 'human-matthew-manager';
    fixture.detectChanges();
    expect(component.initials).toBe('MD');
  });

  it('should compute single initial for single name', () => {
    component.displayName = 'Jessica';
    component.humanId = 'human-jessica-spouse';
    fixture.detectChanges();
    expect(component.initials).toBe('J');
  });

  it('should generate consistent color from name', () => {
    component.displayName = 'Matthew Dowell';
    component.humanId = 'human-matthew-manager';
    fixture.detectChanges();
    const color1 = component.avatarColor;

    component.displayName = 'Matthew Dowell';
    fixture.detectChanges();
    const color2 = component.avatarColor;

    expect(color1).toBe(color2);
  });

  it('should show subtitle when provided', () => {
    component.displayName = 'Matthew';
    component.humanId = 'human-matthew-manager';
    component.subtitle = 'Dowell Household';
    fixture.detectChanges();

    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('.subtitle')?.textContent?.trim()).toBe('Dowell Household');
  });

  it('should not show subtitle element when none provided', () => {
    component.displayName = 'Matthew';
    component.humanId = 'human-matthew-manager';
    fixture.detectChanges();

    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('.subtitle')).toBeNull();
  });

  it('should emit select event on click', () => {
    component.displayName = 'Matthew';
    component.humanId = 'human-matthew-manager';
    fixture.detectChanges();

    const spy = vi.fn();
    component.selected.subscribe(spy);

    const card = fixture.nativeElement.querySelector('.face-card');
    card.click();

    expect(spy).toHaveBeenCalledWith('human-matthew-manager');
  });
});
```

**Step 2: Run test to verify it fails**

Run: `cd /projects/elohim/app/elohim-app && pnpm exec vitest run --config vite.config.ts "face-card" 2>&1 | tail -15`
Expected: FAIL — module not found.

**Step 3: Write the component**

```typescript
// face-card.component.ts
import { CommonModule } from '@angular/common';
import {
  Component,
  computed,
  EventEmitter,
  Input,
  OnChanges,
  Output,
} from '@angular/core';

@Component({
  selector: 'app-face-card',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './face-card.component.html',
  styleUrls: ['./face-card.component.css'],
})
export class FaceCardComponent implements OnChanges {
  @Input() humanId = '';
  @Input() displayName = '';
  @Input() profilePhotoUrl: string | null = null;
  @Input() subtitle: string | null = null;
  @Input() roleTag: string | null = null;
  @Output() selected = new EventEmitter<string>();

  initials = '';
  avatarColor = '';

  private static readonly COLORS = [
    '#6366f1', '#8b5cf6', '#a855f7', '#d946ef',
    '#ec4899', '#f43f5e', '#ef4444', '#f97316',
    '#eab308', '#84cc16', '#22c55e', '#14b8a6',
    '#06b6d4', '#0ea5e9', '#3b82f6', '#6366f1',
  ];

  ngOnChanges(): void {
    this.initials = this.computeInitials(this.displayName);
    this.avatarColor = this.computeColor(this.displayName);
  }

  onClick(): void {
    this.selected.emit(this.humanId);
  }

  private computeInitials(name: string): string {
    if (!name) return '?';
    const parts = name.trim().split(/\s+/);
    if (parts.length === 1) return parts[0][0].toUpperCase();
    return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
  }

  private computeColor(name: string): string {
    let hash = 0;
    for (let i = 0; i < name.length; i++) {
      hash = name.charCodeAt(i) + ((hash << 5) - hash);
    }
    return FaceCardComponent.COLORS[Math.abs(hash) % FaceCardComponent.COLORS.length];
  }
}
```

```html
<!-- face-card.component.html -->
<div class="face-card" (click)="onClick()" (keydown.enter)="onClick()" tabindex="0" role="button">
  <div class="avatar" [style.background-color]="profilePhotoUrl ? 'transparent' : avatarColor">
    @if (profilePhotoUrl) {
      <img [src]="profilePhotoUrl" [alt]="displayName" class="avatar-img" />
    } @else {
      <span class="initials">{{ initials }}</span>
    }
  </div>
  <div class="name">{{ displayName }}</div>
  @if (subtitle) {
    <div class="subtitle">{{ subtitle }}</div>
  }
  @if (roleTag) {
    <div class="role-tag">{{ roleTag }}</div>
  }
</div>
```

```css
/* face-card.component.css */
.face-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 1rem;
  border-radius: 12px;
  cursor: pointer;
  transition: background 0.15s, transform 0.15s;
  text-align: center;
}

.face-card:hover {
  background: var(--surface-elevated, #f5f5f5);
  transform: translateY(-2px);
}

.face-card:focus-visible {
  outline: 2px solid var(--primary, #6366f1);
  outline-offset: 2px;
}

.avatar {
  width: 80px;
  height: 80px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 0.5rem;
  overflow: hidden;
}

.avatar-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.initials {
  color: white;
  font-size: 1.5rem;
  font-weight: 600;
  user-select: none;
}

.name {
  font-size: 0.9375rem;
  font-weight: 500;
  color: var(--text-primary, #1a1a1a);
  line-height: 1.3;
}

.subtitle {
  font-size: 0.8125rem;
  color: var(--text-secondary, #666);
  margin-top: 0.125rem;
}

.role-tag {
  display: inline-block;
  font-size: 0.6875rem;
  font-weight: 500;
  color: var(--primary, #6366f1);
  background: var(--primary-bg, #eef2ff);
  padding: 0.125rem 0.5rem;
  border-radius: 999px;
  margin-top: 0.25rem;
}

@media (prefers-color-scheme: dark) {
  .face-card:hover {
    background: var(--surface-elevated, #2a2a2a);
  }

  .name {
    color: var(--text-primary, #f5f5f5);
  }

  .subtitle {
    color: var(--text-secondary, #999);
  }

  .role-tag {
    background: var(--primary-bg, #1e1b4b);
  }
}
```

**Step 4: Run tests**

Run: `cd /projects/elohim/app/elohim-app && pnpm exec vitest run --config vite.config.ts "face-card" 2>&1 | tail -15`
Expected: All tests PASS.

**Step 5: Commit**

```bash
git add app/elohim-app/src/app/qahal/components/face-card/
git commit -m "feat(qahal): add FaceCardComponent with initials avatar and color hash"
```

---

### Task 8: Build CommunityDirectoryComponent

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/community-directory/community-directory.component.ts`
- Create: `app/elohim-app/src/app/qahal/components/community-directory/community-directory.component.html`
- Create: `app/elohim-app/src/app/qahal/components/community-directory/community-directory.component.css`
- Create: `app/elohim-app/src/app/qahal/components/community-directory/community-directory.component.spec.ts`

**Step 1: Write the test**

```typescript
import { HttpClientTestingModule, HttpTestingController } from '@angular/common/http/testing';
import { ComponentFixture, TestBed, fakeAsync, tick } from '@angular/core/testing';
import { CommunityDirectoryComponent } from './community-directory.component';

describe('CommunityDirectoryComponent', () => {
  let component: CommunityDirectoryComponent;
  let fixture: ComponentFixture<CommunityDirectoryComponent>;
  let httpMock: HttpTestingController;

  const mockHumans = [
    {
      id: 'human-matthew-manager',
      displayName: 'Matthew',
      bio: 'Founder',
      profilePhotoUrl: null,
      affinities: [],
      profileReach: 'community',
      location: null,
      agentPubKey: null,
      appId: 'test',
      createdAt: '2026-01-01',
      updatedAt: '2026-01-01',
    },
    {
      id: 'human-jessica-spouse',
      displayName: 'Jessica',
      bio: null,
      profilePhotoUrl: null,
      affinities: [],
      profileReach: 'network',
      location: null,
      agentPubKey: null,
      appId: 'test',
      createdAt: '2026-01-01',
      updatedAt: '2026-01-01',
    },
  ];

  const mockCollectives = [
    {
      id: 'household-dowell',
      name: 'Dowell Household',
      governanceLayer: 'family',
      description: 'Core family',
      reach: 'private',
      metadata: null,
      constitutionalParentId: null,
      createdBy: null,
      createdAt: '2026-01-01',
      updatedAt: '2026-01-01',
      dissolvedAt: null,
    },
  ];

  const mockParticipants = [
    {
      id: 'p1',
      collectiveId: 'household-dowell',
      humanId: 'human-matthew-manager',
      intimacyLevel: 'intimate',
      roleContext: null,
      governanceWeight: 1.0,
      consentState: 'consented',
      metadata: null,
      joinedAt: '2026-01-01',
      updatedAt: '2026-01-01',
      departedAt: null,
    },
    {
      id: 'p2',
      collectiveId: 'household-dowell',
      humanId: 'human-jessica-spouse',
      intimacyLevel: 'intimate',
      roleContext: 'spouse',
      governanceWeight: 1.0,
      consentState: 'consented',
      metadata: null,
      joinedAt: '2026-01-01',
      updatedAt: '2026-01-01',
      departedAt: null,
    },
  ];

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [CommunityDirectoryComponent, HttpClientTestingModule],
    }).compileComponents();

    fixture = TestBed.createComponent(CommunityDirectoryComponent);
    component = fixture.componentInstance;
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should load humans and collectives on init', fakeAsync(() => {
    fixture.detectChanges();

    const humansReq = httpMock.expectOne('/db/humans');
    humansReq.flush({ items: mockHumans });

    const collectivesReq = httpMock.expectOne(
      (req) => req.url === '/api/v1/collectives' || req.url.includes('/db/collectives')
    );
    collectivesReq.flush({ items: mockCollectives });

    tick();

    expect(component.humans().length).toBe(2);
    expect(component.households().length).toBe(1);
  }));

  it('should default to "all" view', () => {
    expect(component.activeView()).toBe('all');
  });

  it('should switch views', () => {
    component.setView('households');
    expect(component.activeView()).toBe('households');
  });
});
```

**Step 2: Run test to verify it fails**

Run: `cd /projects/elohim/app/elohim-app && pnpm exec vitest run --config vite.config.ts "community-directory" 2>&1 | tail -15`
Expected: FAIL — module not found.

**Step 3: Write the component**

```typescript
// community-directory.component.ts
import { CommonModule } from '@angular/common';
import { HttpClient } from '@angular/common/http';
import { Component, OnInit, inject, signal, computed } from '@angular/core';
import { firstValueFrom } from 'rxjs';

import { FaceCardComponent } from '../face-card/face-card.component';

interface HumanEntry {
  id: string;
  displayName: string;
  bio: string | null;
  profilePhotoUrl: string | null;
  affinities: string[];
  profileReach: string;
  location: string | null;
}

interface CollectiveEntry {
  id: string;
  name: string;
  governanceLayer: string;
  description: string | null;
}

interface ParticipantEntry {
  collectiveId: string;
  humanId: string;
  roleContext: string | null;
}

type DirectoryView = 'all' | 'households' | 'groups';

@Component({
  selector: 'app-community-directory',
  standalone: true,
  imports: [CommonModule, FaceCardComponent],
  templateUrl: './community-directory.component.html',
  styleUrls: ['./community-directory.component.css'],
})
export class CommunityDirectoryComponent implements OnInit {
  private readonly http = inject(HttpClient);

  readonly humans = signal<HumanEntry[]>([]);
  readonly households = signal<CollectiveEntry[]>([]);
  readonly groups = signal<CollectiveEntry[]>([]);
  readonly participantsByCollective = signal<Map<string, ParticipantEntry[]>>(new Map());
  readonly activeView = signal<DirectoryView>('all');
  readonly loading = signal(true);

  /** Map from humanId → household name, for the "all" view subtitle */
  readonly humanHousehold = computed(() => {
    const map = new Map<string, string>();
    const participants = this.participantsByCollective();
    for (const household of this.households()) {
      const members = participants.get(household.id) ?? [];
      for (const m of members) {
        map.set(m.humanId, household.name);
      }
    }
    return map;
  });

  /** Humans not in any household — shown in "Individuals" section of households view */
  readonly individualsNotInHousehold = computed(() => {
    const inHousehold = this.humanHousehold();
    return this.humans().filter(h => !inHousehold.has(h.id));
  });

  async ngOnInit(): Promise<void> {
    try {
      const [humansRes, collectivesRes] = await Promise.all([
        firstValueFrom(this.http.get<{ items: HumanEntry[] }>('/db/humans')),
        firstValueFrom(
          this.http.get<{ items: CollectiveEntry[] }>('/api/v1/collectives')
        ),
      ]);

      this.humans.set(humansRes.items);

      const allCollectives = collectivesRes.items.filter(c => !c.dissolvedAt);
      this.households.set(allCollectives.filter(c => c.governanceLayer === 'family'));
      this.groups.set(allCollectives.filter(c => c.governanceLayer !== 'family'));

      // Load participants for all collectives
      const participantMap = new Map<string, ParticipantEntry[]>();
      const relevantCollectives = [...this.households(), ...this.groups()];

      await Promise.all(
        relevantCollectives.map(async (coll) => {
          try {
            const res = await firstValueFrom(
              this.http.get<{ items: ParticipantEntry[] }>(
                `/db/collectives/${coll.id}/participants`
              )
            );
            participantMap.set(coll.id, res.items);
          } catch {
            participantMap.set(coll.id, []);
          }
        })
      );

      this.participantsByCollective.set(participantMap);
    } finally {
      this.loading.set(false);
    }
  }

  setView(view: DirectoryView): void {
    this.activeView.set(view);
  }

  getHuman(humanId: string): HumanEntry | undefined {
    return this.humans().find(h => h.id === humanId);
  }

  getMembers(collectiveId: string): ParticipantEntry[] {
    return this.participantsByCollective().get(collectiveId) ?? [];
  }

  onFaceSelected(humanId: string): void {
    // TODO: navigate to profile detail
    console.log('Selected:', humanId);
  }
}
```

```html
<!-- community-directory.component.html -->
<div class="directory">
  <div class="directory-header">
    <h1>Community Directory</h1>
    <div class="view-tabs" role="tablist">
      <button
        role="tab"
        [class.active]="activeView() === 'all'"
        [attr.aria-selected]="activeView() === 'all'"
        (click)="setView('all')"
      >
        All
      </button>
      <button
        role="tab"
        [class.active]="activeView() === 'households'"
        [attr.aria-selected]="activeView() === 'households'"
        (click)="setView('households')"
      >
        Households
      </button>
      <button
        role="tab"
        [class.active]="activeView() === 'groups'"
        [attr.aria-selected]="activeView() === 'groups'"
        (click)="setView('groups')"
      >
        Groups
      </button>
    </div>
  </div>

  @if (loading()) {
    <div class="loading">Loading community...</div>
  } @else {
    <!-- All Members View -->
    @if (activeView() === 'all') {
      <div class="face-grid">
        @for (human of humans(); track human.id) {
          <app-face-card
            [humanId]="human.id"
            [displayName]="human.displayName"
            [profilePhotoUrl]="human.profilePhotoUrl"
            [subtitle]="humanHousehold().get(human.id) ?? null"
            (selected)="onFaceSelected($event)"
          />
        }
      </div>
    }

    <!-- Households View -->
    @if (activeView() === 'households') {
      @for (household of households(); track household.id) {
        <div class="group-section">
          <h2 class="group-header">{{ household.name }}</h2>
          <div class="face-grid">
            @for (participant of getMembers(household.id); track participant.humanId) {
              @if (getHuman(participant.humanId); as human) {
                <app-face-card
                  [humanId]="human.id"
                  [displayName]="human.displayName"
                  [profilePhotoUrl]="human.profilePhotoUrl"
                  [roleTag]="participant.roleContext"
                  (selected)="onFaceSelected($event)"
                />
              }
            }
          </div>
        </div>
      }
      @if (individualsNotInHousehold().length > 0) {
        <div class="group-section">
          <h2 class="group-header">Individuals</h2>
          <div class="face-grid">
            @for (human of individualsNotInHousehold(); track human.id) {
              <app-face-card
                [humanId]="human.id"
                [displayName]="human.displayName"
                [profilePhotoUrl]="human.profilePhotoUrl"
                (selected)="onFaceSelected($event)"
              />
            }
          </div>
        </div>
      }
    }

    <!-- Groups View -->
    @if (activeView() === 'groups') {
      @for (group of groups(); track group.id) {
        <div class="group-section">
          <h2 class="group-header">{{ group.name }}</h2>
          @if (group.description) {
            <p class="group-description">{{ group.description }}</p>
          }
          <div class="face-grid">
            @for (participant of getMembers(group.id); track participant.humanId) {
              @if (getHuman(participant.humanId); as human) {
                <app-face-card
                  [humanId]="human.id"
                  [displayName]="human.displayName"
                  [profilePhotoUrl]="human.profilePhotoUrl"
                  [roleTag]="participant.roleContext"
                  (selected)="onFaceSelected($event)"
                />
              }
            }
          </div>
        </div>
      }
    }

    @if (humans().length === 0) {
      <div class="empty-state">
        <p>No community members yet.</p>
      </div>
    }
  }
</div>
```

```css
/* community-directory.component.css */
.directory {
  max-width: 960px;
  margin: 0 auto;
  padding: 1.5rem;
}

.directory-header {
  margin-bottom: 2rem;
}

.directory-header h1 {
  font-size: 1.75rem;
  margin: 0 0 1rem 0;
  color: var(--text-primary, #1a1a1a);
}

.view-tabs {
  display: flex;
  gap: 0.25rem;
  background: var(--surface-elevated, #f0f0f0);
  border-radius: 8px;
  padding: 0.25rem;
  width: fit-content;
}

.view-tabs button {
  padding: 0.5rem 1rem;
  border: none;
  background: transparent;
  border-radius: 6px;
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--text-secondary, #666);
  cursor: pointer;
  transition: all 0.15s;
}

.view-tabs button.active {
  background: var(--surface, #fff);
  color: var(--text-primary, #1a1a1a);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

.face-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
  gap: 0.5rem;
}

.group-section {
  margin-bottom: 2rem;
}

.group-header {
  font-size: 1.125rem;
  font-weight: 600;
  color: var(--text-primary, #1a1a1a);
  margin: 0 0 0.75rem 0;
  padding-bottom: 0.5rem;
  border-bottom: 1px solid var(--border, #e5e5e5);
}

.group-description {
  font-size: 0.8125rem;
  color: var(--text-secondary, #666);
  margin: -0.5rem 0 0.75rem 0;
}

.loading,
.empty-state {
  text-align: center;
  padding: 3rem;
  color: var(--text-secondary, #666);
}

@media (prefers-color-scheme: dark) {
  .directory-header h1 {
    color: var(--text-primary, #f5f5f5);
  }

  .view-tabs {
    background: var(--surface-elevated, #2a2a2a);
  }

  .view-tabs button.active {
    background: var(--surface, #333);
    color: var(--text-primary, #f5f5f5);
  }

  .group-header {
    color: var(--text-primary, #f5f5f5);
    border-color: var(--border, #444);
  }
}
```

**Step 4: Run tests**

Run: `cd /projects/elohim/app/elohim-app && pnpm exec vitest run --config vite.config.ts "community-directory" 2>&1 | tail -15`
Expected: All tests PASS.

**Step 5: Commit**

```bash
git add app/elohim-app/src/app/qahal/components/community-directory/
git commit -m "feat(qahal): add CommunityDirectoryComponent with all/households/groups views"
```

---

### Task 9: Wire directory route and update community home

**Files:**
- Modify: `app/elohim-app/src/app/qahal/community.routes.ts`
- Modify: `app/elohim-app/src/app/qahal/components/community-home/community-home.component.ts`

**Step 1: Add directory route**

Add a `directory` child route to `COMMUNITY_ROUTES`:

```typescript
      {
        path: 'directory',
        loadComponent: async () =>
          import('./components/community-directory/community-directory.component').then(
            m => m.CommunityDirectoryComponent
          ),
        data: {
          title: 'Community Directory',
          seo: {
            title: 'Community Directory',
            description: 'See your community members, organized by household and group.',
          },
        },
      },
```

**Step 2: Update community home to link to directory**

In `community-home.component.ts`, replace the "Coming Soon" placeholder section with a link to the directory. Change the first feature card (Graduated Intimacy) to a router link to `/community/directory`:

Add `RouterModule` to imports if not already there. Add a prominent link:
```html
<a routerLink="directory" class="nav-link">👥 Community Directory</a>
```

**Step 3: Commit**

```bash
git add app/elohim-app/src/app/qahal/community.routes.ts app/elohim-app/src/app/qahal/components/community-home/
git commit -m "feat(qahal): wire directory route and link from community home"
```

---

### Task 10: Update barrel exports

**Files:**
- Modify: `app/elohim-app/src/app/qahal/index.ts`

**Step 1: Export new components**

Add exports for the new components:

```typescript
export { FaceCardComponent } from './components/face-card/face-card.component';
export { CommunityDirectoryComponent } from './components/community-directory/community-directory.component';
```

**Step 2: Commit**

```bash
git add app/elohim-app/src/app/qahal/index.ts
git commit -m "chore(qahal): export FaceCardComponent and CommunityDirectoryComponent"
```

---

### Task 11: Update seed data with household participation

**Context:** The collectives.json already has `household-dowell` and `household-eden`. The account-packages already have `collectives` arrays linking humans to collectives. Verify that the existing seed data correctly represents the two families Matthew described:
- Dowell household: Matthew + Jessica + 1 child (James) = 3 people
- One other family: 2 people (check which existing household fits — Eden has Adam + Eve)

**Files:**
- Possibly modify: `genesis/data/collectives/collectives.json` (if household descriptions need updating)
- Possibly modify: `genesis/data/account-packages/*.json` (if participation links are missing)

**Step 1: Audit existing seed data**

Check that these account packages have `collectives` entries linking to households:
- `matthew-manager.json` → should have `household-dowell`
- `jessica-spouse.json` → should have `household-dowell`
- `james-son.json` → should have `household-dowell`
- Check `adam-firstman.json` and `eve-firstwoman.json` → should have `household-eden`

Run: `grep -l "household-" genesis/data/account-packages/*.json`

**Step 2: Add any missing participation links**

If any account package is missing its household collective, add it to the `collectives` array at the bottom of the file:

```json
{
  "collectiveId": "household-dowell",
  "roleContext": "child",
  "intimacyLevel": "family"
}
```

**Step 3: Verify collective count matches design**

The design calls for 2 households (~5 people) + 1-2 small groups. The existing seed data has 27 collectives — most won't be seeded on current infrastructure but that's fine. The directory will show whatever gets seeded.

**Step 4: Commit (only if changes were made)**

```bash
git add genesis/data/
git commit -m "chore(seed): verify household participation links for community directory"
```

---

### Task 12: Run full test suite and verify

**Step 1: Run qahal component tests**

Run: `cd /projects/elohim/app/elohim-app && pnpm exec vitest run --config vite.config.ts "qahal" 2>&1 | tail -20`
Expected: All tests PASS (face-card + community-directory + existing qahal tests).

**Step 2: Run lint**

Run: `cd /projects/elohim/app/elohim-app && pnpm run lint 2>&1 | tail -10`
Expected: No errors in new files.

**Step 3: Verify Rust compiles**

Run: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check 2>&1 | tail -5`
Expected: No errors.

**Step 4: Commit any fixes**

If lint or tests revealed issues, fix and commit.

---

### Reference: Key file paths

**Rust (elohim-storage):**
- `elohim/elohim-storage/src/db/diesel_schema.rs` — table macro
- `elohim/elohim-storage/src/db/models.rs` — Human, NewHuman structs (~line 717)
- `elohim/elohim-storage/src/db/humans.rs` — CRUD functions
- `elohim/elohim-storage/src/views.rs` — HumanView, InputViews (~line 2239)
- `elohim/elohim-storage/src/api/identity.rs` — HTTP handlers
- `elohim/elohim-storage/src/http.rs` — route registration (~line 5799)

**Generated types:**
- `elohim/sdk/storage-client-ts/src/generated/HumanView.ts`

**Angular (qahal pillar):**
- `app/elohim-app/src/app/qahal/community.routes.ts` — routing
- `app/elohim-app/src/app/qahal/components/` — components dir
- `app/elohim-app/src/app/qahal/index.ts` — barrel exports

**Seed data:**
- `genesis/data/collectives/collectives.json` — collective definitions
- `genesis/data/account-packages/*.json` — per-human packages with collective participation
