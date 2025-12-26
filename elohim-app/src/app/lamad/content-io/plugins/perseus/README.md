# Perseus Quiz Plugin

This plugin integrates Khan Academy's Perseus quiz framework for interactive assessments.

## Dependencies

Perseus requires React and several Khan Academy packages:

```bash
npm install react react-dom @types/react @types/react-dom --legacy-peer-deps
npm install @khanacademy/perseus @khanacademy/math-input --legacy-peer-deps
npm install underscore jquery aphrodite --legacy-peer-deps
```

## Architecture

The plugin uses a layered approach to integrate React into Angular:

```
PerseusFormatPlugin (Angular service)
    └── PerseusRendererComponent (Angular component)
        └── PerseusWrapperComponent (Angular-to-CustomElement bridge)
            └── <perseus-question> (Custom Element / Web Component)
                └── PerseusItemWrapper (React component)
                    └── Perseus ItemRenderer (Khan Academy React)
```

## Lazy Loading

To avoid bundling React in the main Angular bundle:

1. The `perseus-element-loader.ts` provides async registration
2. React and Perseus are dynamically imported only when a quiz is rendered
3. The custom element encapsulates React, allowing framework-agnostic usage

## Enabling Perseus

1. Install dependencies (see above)
2. Uncomment the import in `content-io.module.ts`:
   ```typescript
   import { PerseusFormatPlugin } from './plugins/perseus/perseus-format.plugin';
   ```
3. Uncomment the registration:
   ```typescript
   registry.register(new PerseusFormatPlugin());
   ```

## Alternative: External Bundle

For production, consider building Perseus as a separate bundle:

1. Create `perseus-bundle.ts` entry point
2. Use esbuild/rollup to bundle React + Perseus + dependencies
3. Load the bundle via script tag when needed
4. Register the custom element from the external bundle

This approach keeps the main Angular bundle lean while supporting rich quiz functionality.

## File Structure

- `perseus-format.plugin.ts` - Unified plugin (import/export/validate/render)
- `perseus-renderer.component.ts` - Angular renderer with quiz UI
- `perseus-wrapper.component.ts` - Angular-to-CustomElement bridge
- `perseus-element-loader.ts` - Lazy loading without React types
- `perseus-element.tsx` - React custom element (requires React)
- `perseus-item.model.ts` - TypeScript types for Perseus items
