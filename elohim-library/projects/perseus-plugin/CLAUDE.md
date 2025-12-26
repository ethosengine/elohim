# Perseus Plugin Module

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
