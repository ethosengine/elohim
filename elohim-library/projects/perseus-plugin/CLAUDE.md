# Perseus Plugin Module

## Architecture Overview

### Component Hierarchy
```
Angular App
    └── PerseusWrapperComponent (Angular)
        └── <perseus-question> (Custom Element)
            └── PerseusItemWrapper (React)
                └── ServerItemRenderer (Khan Academy Perseus)
```

### Loading Flow
1. Angular component mounts → calls registerPerseusElement()
2. React 18 loaded from CDN → Perseus CSS loaded lazily
3. UMD bundle loaded → custom element registered synchronously
4. Angular wrapper sets item property → React renders question

## Critical Patterns

### 1. Synchronous Custom Element Registration
**The UMD bundle registers `<perseus-question>` synchronously during script execution.**

❌ BAD - Polling with setTimeout:
```typescript
await loadScript(pluginUrl);
await new Promise((resolve) => {
  const check = () => {
    if (customElements.get('perseus-question')) resolve();
    else setTimeout(check, 100);  // Creates race conditions!
  };
  setTimeout(check, 100);
});
```

✅ GOOD - Synchronous check:
```typescript
await loadScript(pluginUrl);
const elementDef = customElements.get('perseus-question');
if (!elementDef) throw new Error('Element not registered');
```

**Why**: setTimeout callbacks can be "orphaned" during page reloads, causing the Promise to never resolve.

### 2. Cache-Busting Strategy
**Use a single timestamp per page load, not per function call.**

❌ BAD:
```typescript
const getPluginUrl = () => `/path/plugin.js?v=${Date.now()}`;  // Changes on every call!
```

✅ GOOD:
```typescript
const CACHE_BUST = Date.now();  // Set once at module load
const getPluginUrl = () => `/path/plugin.js?v=${CACHE_BUST}`;
```

### 3. Light DOM (No Shadow DOM)
Perseus uses Aphrodite CSS-in-JS which injects styles into `document.head`. Shadow DOM blocks these styles.

### 4. Wonder Blocks CSS Variables
Perseus uses `--wb-semanticColor-*` variables without fallbacks. Both light and dark mode must define these in `styles.css`.

## Known Bugs & Fixes

### "No question loaded" Bug
**Symptom**: Quiz shows "No question loaded" message, never renders question.

**Root Cause**: Using setTimeout polling to wait for custom element registration when the element registers synchronously.

**Fix**: Check `customElements.get()` immediately after `await loadScript()` - no polling needed.

### Browser Crashes on Reload
**Symptom**: Page crashes/freezes when reloading with dark mode enabled.

**Root Cause**: MutationObserver watching entire document with too broad a scope can create infinite loops.

**Fix**: Dark mode JS system disabled; CSS-only approach used via Wonder Blocks variables.

### Light Mode No Contrast
**Symptom**: Text invisible in light mode (white on white).

**Root Cause**: Wonder Blocks CSS variables undefined in light mode.

**Fix**: Add `--wb-semanticColor-*` definitions for `@media (prefers-color-scheme: light)` and `body[data-theme="light"]` in `styles.css`.

## File Locations

| File | Purpose |
|------|---------|
| `elohim-library/projects/perseus-plugin/src/perseus-element.tsx` | React custom element |
| `elohim-app/.../perseus/perseus-element-loader.ts` | Lazy loading, CSS injection |
| `elohim-app/.../perseus/perseus-wrapper.component.ts` | Angular wrapper |
| `elohim-app/src/styles.css` | Wonder Blocks variable overrides (lines 983-1350) |
| `elohim-app/src/assets/perseus-plugin/` | Built UMD bundle + CSS |

## Building & Deployment

```bash
cd elohim-library/projects/perseus-plugin
npm run build
cp dist/perseus-plugin.umd.js ../../elohim-app/src/assets/perseus-plugin/
cp dist/perseus.css ../../elohim-app/src/assets/perseus-plugin/
```

## Critical: SVG Bundling Requirement

The `@khanacademy/perseus` and `@khanacademy/wonder-blocks-*` packages have transitive dependencies on `@phosphor-icons/core`, which imports raw SVG files like:

```
@phosphor-icons/core/assets/bold/arrow-square-out-bold.svg
@phosphor-icons/core/assets/regular/caret-down.svg
```

### The Problem

When elohim-app runs unit tests (Karma/webpack), these SVG imports fail with:
```
Module parse failed: Unexpected token (1:0)
You may need an appropriate loader to handle this file type
```

The production build works because Angular's `application` builder uses esbuild with a `loader: { ".svg": "text" }` config, but Karma uses webpack which lacks this configuration.

### Required Solution

This perseus-plugin module **must bundle or handle SVG assets** during its build so consumers don't need special webpack configuration.

Options (in order of preference):

1. **Configure Rollup to inline SVGs** - Add a plugin like `@rollup/plugin-image` or a custom SVG loader to resolve and inline SVG imports during the library build

2. **Re-export icons** - If Perseus components need specific icons, re-export them from this module so `@phosphor-icons/core` isn't directly imported by consuming apps

3. **Mark as external + document** - If SVGs can't be bundled, clearly document that consumers need webpack SVG handling and provide a sample config

### Verification

After setting up the build, run from elohim-app:
```bash
npm test -- --watch=false --browsers=ChromeHeadless
```

The tests should pass without any "Module parse failed" errors for SVG files.
