# Common Mistakes in Elohim Project Code Sessions

This document captures recurring mistake patterns identified across multiple code sessions in the Elohim project. Understanding these patterns helps prevent similar issues in future development.

## 1. ASYNC/TIMING ISSUES IN TESTS (Most Common)

### Pattern
Confusion about when to use `fakeAsync/tick` vs `async/whenStable` and the timing of `detectChanges()` relative to observable emissions.

### Specific Examples

**File**: `elohim-app/src/app/lamad/components/module-viewer/module-viewer.component.spec.ts`

Multiple iterations were needed to fix test timing:
- **Attempt 1** (commit 7898f0c): Used `fakeAsync/tick` - didn't properly handle Observable subscriptions
- **Attempt 2** (commit 193372e): Switched to `async/whenStable`
- **Attempt 3** (commit c980748): Emitted route params BEFORE `detectChanges()`
- **Attempt 4** (commit 3430353): Reversed to `detectChanges()` FIRST, then emit params

**Correct pattern** (lines 109-115):
```typescript
fixture.detectChanges(); // Initialize component and set up subscription
paramsSubject.next({ id: 'value-scanner' }); // Emit params to trigger loadModule
fixture.detectChanges(); // Process changes from loadModule
await fixture.whenStable();
```

### Root Cause
Angular's component initialization cycle requires:
1. First `detectChanges()` to trigger `ngOnInit()` and set up subscriptions
2. Then emit observable values to trigger subscription callbacks
3. Second `detectChanges()` to process component state changes
4. `whenStable()` to wait for all async operations

### Solution
- Use `async/await` with `fixture.whenStable()` for async tests (avoid fakeAsync)
- For route params: **detectChanges → emit → detectChanges → whenStable**

---

## 2. TEST POLLUTION FROM SHARED STATE

### Pattern
Tests failing intermittently because previous tests left state in localStorage or other global objects.

### Specific Examples

**File**: `elohim-app/src/app/services/theme.service.spec.ts`

**Mistake** (commit 92a2a63):
```typescript
// WRONG: Clear after service initialization
TestBed.inject(ThemeService);
localStorage.clear();
```

**Fix**:
```typescript
// CORRECT: Clear before service initialization
localStorage.clear();
TestBed.inject(ThemeService);
```

**File**: `elohim-app/src/app/services/config.service.spec.ts` (lines 27-33)
```typescript
afterEach(() => {
  httpMock.verify();
  // Restore original environment values
  (environment as any).production = originalEnvironment.production;
  (environment as any).logLevel = originalEnvironment.logLevel;
  (environment as any).environment = originalEnvironment.environment;
});
```

### Root Cause
The ThemeService reads from localStorage in its constructor via `loadTheme()`. Clearing localStorage after service creation was too late, causing tests to see values from previous tests.

### Solution
- Always clear shared state (localStorage, sessionStorage, environment) in `beforeEach`
- Clear state BEFORE initializing services that read from it

---

## 3. MISSING TEST PROVIDERS/DEPENDENCIES

### Pattern
Components fail to initialize because required Angular providers are missing from test setup.

### Specific Examples

**Missing RouterLink provider** (commit f2dc355):
- **File**: `elohim-app/src/app/components/footer/footer.component.spec.ts`
- **Error**: Component uses `RouterLink` directive but test didn't provide routing
- **Fix**: Added `provideRouter([])` to providers array (line 14)

**Missing DomSanitizer** (commit 0369a68):
- **File**: `elohim-app/src/app/lamad/components/module-viewer/module-viewer.component.spec.ts`
- **Error**: Component requires DomSanitizer injection but test didn't provide it
- **Fix**: Added mock DomSanitizer (lines 83-84, 96)

**Missing HttpClient**:
- **Files**: Multiple component specs
- **Fix**: Added `provideHttpClient()` and `provideHttpClientTesting()`

**Missing DestroyRef for takeUntilDestroyed** (commit 3430353):
- **Error**: Component uses `takeUntilDestroyed()` operator but test didn't mock DestroyRef properly
- **Fix**: Created proper DestroyRef mock with onDestroy callback function

### Root Cause
Incomplete mental model of dependency injection - not recognizing all dependencies including directives and operators.

### Solution
- When component uses RouterLink, add `provideRouter([])`
- When component uses HttpClient, add `provideHttpClient()` and `provideHttpClientTesting()`
- When component uses `takeUntilDestroyed()`, ensure DestroyRef is properly mocked
- Check template for directive dependencies (RouterLink, NgIf, etc.)

---

## 4. MEMORY LEAKS AND CLEANUP ISSUES

### Pattern
Components set up event listeners, observers, and subscriptions but don't clean them up properly.

### Good Examples (Cleanup Done Correctly)

**File**: `elohim-app/src/app/components/home/home.component.ts` (lines 55-65)
```typescript
ngOnDestroy() {
  if (this.scrollListener) {
    window.removeEventListener('scroll', this.scrollListener);
  }
  if (this.intersectionObserver) {
    this.intersectionObserver.disconnect();
  }
  if (this.rafId) {
    cancelAnimationFrame(this.rafId);
  }
}
```

**File**: `elohim-app/src/app/components/theme-toggle/theme-toggle.component.ts` (lines 24-26)
```typescript
ngOnDestroy(): void {
  this.themeSubscription?.unsubscribe();
}
```

### Better Pattern (Automatic Cleanup)

**File**: `elohim-app/src/app/lamad/components/module-viewer/module-viewer.component.ts` (lines 32, 40-52)
```typescript
private readonly destroy$ = new Subject<void>();

ngOnInit(): void {
  this.route.params
    .pipe(takeUntil(this.destroy$))
    .subscribe(params => {
      const moduleId = params['id'];
      this.loadModule(moduleId);
    });
}

ngOnDestroy(): void {
  this.destroy$.next();
  this.destroy$.complete();
}
```

### Issues Found

**File**: `elohim-app/src/app/components/hero/hero.component.ts`
- **Issue**: Uses `setTimeout()` (lines 37, 51, 57) but never clears timers in ngOnDestroy
- **Risk**: Low (timers are short-lived) but not best practice

### Root Cause
Observable subscription management not using proper cleanup patterns.

### Solution
- Use `Subject + takeUntil` pattern for automatic subscription cleanup
- Always implement `ngOnDestroy()` when component creates subscriptions, timers, or listeners
- Prefer `takeUntil()` over manual `unsubscribe()`

---

## 5. IMPROPER MOCK INITIALIZATION

### Pattern
BehaviorSubjects initialized with wrong initial values causing tests to fail.

### Example
**File**: `elohim-app/src/app/lamad/components/module-viewer/module-viewer.component.spec.ts`

**Initial mistake** (commit c980748):
```typescript
paramsSubject = new BehaviorSubject<any>({}); // Empty object
```

**Fix** (commit bae09b5):
```typescript
paramsSubject = new BehaviorSubject({ id: 'value-scanner' }); // Proper initial value
```

### Root Cause
Component's `ngOnInit()` subscribes to route params immediately, so BehaviorSubject's initial value matters and must be realistic.

### Solution
- Initialize BehaviorSubjects with realistic default values
- Match the shape of data the component expects
- Consider what happens when subscription fires immediately

---

## 6. HTTP TESTING MODULE MISUSE

### Pattern
Not calling `httpMock.verify()` in afterEach, leading to undetected pending requests.

### Good Examples

**File**: `elohim-app/src/app/services/config.service.spec.ts` (lines 27-28)
```typescript
afterEach(() => {
  httpMock.verify();
  // ...
});
```

**File**: `elohim-app/src/app/lamad/services/document-graph.service.spec.ts` (lines 22-24)
```typescript
afterEach(() => {
  httpMock.verify();
});
```

### Solution
- Always call `httpMock.verify()` in `afterEach` for HTTP tests
- This ensures no unexpected HTTP requests were made
- Catches missing `expectOne()` calls

---

## 7. TYPESCRIPT TYPE SAFETY ISSUES

### Pattern
Overuse of `any` type defeating TypeScript's type checking.

### Occurrences
Found 23 instances of `: any` or `any[]` across 15 files.

### Examples

**Bad**:
```typescript
let mockDocumentGraphService: any;
```

**Better**:
```typescript
let mockDocumentGraphService: jasmine.SpyObj<DocumentGraphService>;
```

### Root Cause
Taking shortcuts during test setup instead of creating proper type-safe mocks.

### Solution
- Use `jasmine.SpyObj<ServiceType>` for service mocks
- Define proper interfaces for test data
- Only use `any` when absolutely necessary (e.g., testing edge cases)

---

## 8. CONSOLE STATEMENTS LEFT IN CODE

### Pattern
Debug console.log/console.error statements left in production code.

### Files with Console Statements
- `elohim-app/src/app/lamad/services/document-graph.service.ts`
- `elohim-app/src/app/lamad/services/affinity-tracking.service.ts`
- `elohim-app/src/app/lamad/components/lamad-layout/lamad-layout.component.ts`
- `elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.ts`

### Good Example

**File**: `elohim-app/src/app/services/theme.service.ts` (lines 76, 92)
```typescript
// Appropriate use of console.warn() for error handling
console.warn('Invalid theme value:', value);
```

### Solution
- Remove debug console.log statements before committing
- Use proper logging service for production logging
- console.warn() and console.error() are acceptable for error handling
- Consider ESLint rule to prevent console.log

---

## 9. STANDALONE COMPONENT IMPORT ISSUES

### Pattern
When using standalone components, forgetting that dependencies must be imported, not just provided.

### Solution
- For standalone components, use `imports: [ComponentName]` not `declarations`
- All dependencies must be explicitly imported
- This pattern was learned - all test files properly handle this now

---

## ROOT CAUSES SUMMARY

1. **Lack of understanding of Angular's change detection lifecycle** - Multiple iterations needed to get detectChanges() timing right
2. **Test isolation not enforced** - Shared state like localStorage and environment variables polluting tests
3. **Incomplete mental model of dependency injection** - Missing providers for directives and services
4. **Observable subscription management** - Not using proper cleanup patterns (takeUntil, unsubscribe)
5. **Async testing confusion** - fakeAsync vs async/await with whenStable
6. **Insufficient type safety** - Overuse of `any` type
7. **Test-first approach not followed** - Tests added after implementation, missing edge cases

---

## RECOMMENDATIONS FOR FUTURE SESSIONS

### Testing Best Practices
1. Always clear shared state (localStorage, environment) in `beforeEach`
2. Use `async/await` with `fixture.whenStable()` for async tests (avoid fakeAsync)
3. For route params: **detectChanges → emit → detectChanges → whenStable**
4. Always verify all providers/imports are included for standalone components
5. Call `httpMock.verify()` in `afterEach` for HTTP tests
6. Initialize BehaviorSubjects with realistic default values

### Code Quality
7. Use `Subject + takeUntil` pattern for automatic subscription cleanup
8. Use proper TypeScript types instead of `any`
9. Remove console.log statements; use proper logging service
10. Always implement `ngOnDestroy()` when creating subscriptions, timers, or listeners
11. Clear all timers and event listeners in `ngOnDestroy()`

### Development Process
12. Write tests first to understand component lifecycle requirements
13. Review dependencies before writing tests
14. Run tests in isolation to catch shared state issues
15. Use ESLint rules to catch common mistakes automatically

---

## Related Commits

Key commits that fixed these patterns:
- `92a2a63` - Fix flaky ThemeService test by clearing localStorage before service initialization
- `3430353` - Improve cleanup and memory management
- `193372e` - Replace fakeAsync/tick with async/whenStable in ModuleViewer tests
- `bae09b5` - Use BehaviorSubject for route params in ModuleViewerComponent test
- `f2dc355` - Fix FooterComponent test - add provideRouter for RouterLink dependency
- `0369a68` - Add DomSanitizer mock to ModuleViewerComponent test
