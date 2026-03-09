---
name: page-model
description: Audit Angular component legibility — data-testid coverage for tests/agents, accessible labeling for people. Scans templates, reports gaps, suggests fixes, and escalates components that need redesign.
---

# Page Model

Audit whether Angular components are legible — to tests, to agents, to screen readers. Scans templates for `data-testid` coverage and accessible labeling. Reports gaps, suggests fixes, and escalates structural issues.

## When to Use

- After creating or modifying Angular component templates
- During quality sweeps touching UI code
- Before writing new page objects (to know what's modeled)
- When the `[TestID]` or `[a11y]` post-edit hook reports gaps on a file you're touching

## The Convention

### data-testid Naming

Format: `{scope}-{purpose}`

| Scope | Source App | Example |
|-------|-----------|---------|
| `threshold-` | doorway-app (OAuth login) | `threshold-identifier`, `threshold-submit` |
| `login-` | elohim-app (federated login) | `login-federated-id`, `login-password` |
| `profile-` | elohim-app (navigator shell) | `profile-bubble`, `profile-tray` |
| `logout-` | elohim-app (navigator shell) | `logout-button` |
| `{feature}-` | any component | `content-title`, `mastery-score` |

Rules:
- Scope = the component/feature context (not the Angular selector)
- Purpose = what the element does, not its CSS class
- Use kebab-case, never camelCase
- Prefer short, descriptive values over long compound names

### What Needs a data-testid

Interactive elements that tests or agents would target:
- `<input>` — form inputs
- `<button>` — action triggers
- `<select>`, `<textarea>` — form controls
- `<a [routerLink]>` — navigation links
- Elements with `(click)=` bindings — clickable non-buttons
- Error/status banners — assertion targets
- Key content containers — elements tests wait on for page-ready signals

### What Does NOT Need a data-testid

- Decorative elements (icons, dividers, spacers)
- Wrapper/layout divs with no behavioral significance
- Elements inside `*ngFor` (use parent container or `:nth-child` in tests)
- Third-party component wrappers (test the host, not the inner DOM)

## The Contract

### selectors.ts (test-side registry)

`genesis/a2o/src/framework/pages/selectors.ts` is the central registry of all test IDs. Each constant group maps to one component's `data-testid` values:

```typescript
export const THRESHOLD = {
  IDENTIFIER: 'threshold-identifier',
  PASSWORD: 'threshold-password',
  SUBMIT: 'threshold-submit',
  ERROR: 'threshold-error',
} as const;
```

When adding new `data-testid` attributes to a template, add matching constants to `selectors.ts`.

### Page Objects

Page objects in `genesis/a2o/src/framework/pages/` consume selectors via `BasePage.testId()`:

```typescript
class ThresholdLoginPage extends BasePage {
  async login(identifier: string, password: string) {
    await this.testId(THRESHOLD.IDENTIFIER).fill(identifier);
    await this.testId(THRESHOLD.PASSWORD).fill(password);
    await this.testId(THRESHOLD.SUBMIT).click();
  }
}
```

New page objects should follow the same pattern: import from `selectors.ts`, use `testId()` helper, extend `BasePage`.

## Running a Full Audit

### Quick scan — single component

```bash
# Count interactive elements vs data-testid coverage
grep -cE '<(input|button|select|textarea)\b' path/to/component.html
grep -c 'data-testid=' path/to/component.html
```

### Full scan — all templates across both apps

```bash
# Find all component templates with testid gap reporting
find elohim-app/src doorway-app/src -name '*.component.html' -exec sh -c '
  for f; do
    total=$(grep -cE "<(input|button|select|textarea)\b" "$f" 2>/dev/null) || total=0
    covered=$(grep -c "data-testid=" "$f" 2>/dev/null) || covered=0
    [ "$total" -gt 0 ] && [ "$covered" -lt "$total" ] && echo "$covered/$total  $f"
  done
' _ {} +
```

### Inline templates (.component.ts)

For components with `template:` in the TypeScript file:

```bash
# Find .component.ts files with inline templates containing interactive elements
grep -rl 'template:' elohim-app/src doorway-app/src --include='*.component.ts' 2>/dev/null | while read -r f; do
  total=$(grep -cE '<(input|button|select|textarea)\b' "$f" 2>/dev/null) || total=0
  covered=$(grep -c 'data-testid=' "$f" 2>/dev/null) || covered=0
  [ "$total" -gt 0 ] && [ "$covered" -lt "$total" ] && echo "$covered/$total  $f (inline)"
done
```

## Key Files

| File | Role |
|------|------|
| `genesis/a2o/src/framework/pages/selectors.ts` | Test-side ID registry (the contract) |
| `genesis/a2o/src/framework/pages/base.page.ts` | BasePage with `testId()` helper |
| `genesis/a2o/src/framework/pages/index.ts` | Barrel export for page objects |
| `.claude/hooks/testid-audit.py` | Post-edit hook (fires on component edits) |

## Workflow: Adding Test IDs to a New Component

1. Identify interactive elements in the template
2. Choose scope prefix (component feature name, kebab-case)
3. Add `data-testid="{scope}-{purpose}"` to each element
4. Add constants to `selectors.ts` under a new group
5. (Optional) Create a page object in `genesis/a2o/src/framework/pages/`
6. (Optional) Update step definitions to use the page object

## Accessibility (a11y)

The same hook that checks `data-testid` also checks accessible labeling. The rule: every interactive element a person can reach should be describable to a screen reader.

### Philosophy: HTML First, aria-label Last

`aria-label` is a patch, not a solution. The priority order when the hook flags an a11y gap:

1. **Fix the HTML** — use `<label for="id">`, semantic elements, visible text content
2. **If the design makes good HTML impossible** — escalate for component redesign
3. **Only as a last resort** — add `aria-label` for genuinely icon-only elements where visible text would harm the design (close buttons, icon toggles)

If you encounter a component where accessibility requires hacking around bad HTML structure, **escalate it** rather than papering over it. The component needs redesign so it works beautifully for everyone.

### What the Hook Checks

| Element | Accessible if... | Flagged when... |
|---------|-------------------|-----------------|
| `<input>`, `<select>`, `<textarea>` | Has `<label for="id">` (best) or `aria-label` (fallback) | No label association found |
| `<button>` (icon-only) | Has `aria-label` | Class hints icon-only but no `aria-label` |
| `<button>` (with text) | Text content is the label | Not flagged (assumed accessible) |
| Custom `(click)` element | Is a `<button>` instead (best), or has `role` + `aria-label` | Missing either |
| Checkbox/radio | Wrapped in `<label>` (implicit) | Not flagged (convention) |

### Preferred Fixes (best to worst)

```html
<!-- BEST: Proper <label> association — visible, clickable, accessible -->
<label for="search">Search</label>
<input id="search" [(ngModel)]="query" />

<!-- GOOD: Visually hidden label when design needs no visible label -->
<label for="search" class="sr-only">Search content</label>
<input id="search" [(ngModel)]="query" />

<!-- OK: aria-label for genuinely icon-only elements -->
<button class="dismiss" aria-label="Dismiss error" (click)="clearError()">x</button>

<!-- ESCALATE: Don't do this — if a <div> needs role="button", make it a <button> -->
<!-- Bad:  <div (click)="select()" role="button" aria-label="Select"> -->
<!-- Good: <button (click)="select()">Select</button> -->
```

### What NOT to Do

- Don't use `aria-label` to compensate for missing `<label>` elements — add the label
- Don't add `role="button"` to a `<div>` — use a `<button>` element instead
- Don't duplicate visible text in `aria-label` (screen readers would read it twice)
- Don't use `aria-label` on non-interactive containers
- Don't patch over structural problems — escalate the component for redesign

## Future: Model Mode

The `data-testid` attributes are the foundation for a planned "model mode" — a dev overlay where every testable component shows its identity, test hooks, and source link. The same attributes that enable automated testing will enable agent-readable page introspection at runtime.
