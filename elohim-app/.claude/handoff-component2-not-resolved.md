# Resolved: `Component2 not resolved` — External templateUrl Fails in Vitest

**Date**: 2026-03-04
**Project**: elohim-app (Angular 19 frontend)
**Upstream issue**: [angular/angular-cli#32055](https://github.com/angular/angular-cli/issues/32055)
**Status**: **FIXED** via `pnpm patch @angular/build@19.2.22`

## Environment

| Package | Version |
|---------|---------|
| `@angular/core` | 19.2.19 |
| `@angular/build` | 19.2.22 (patched) |
| `@analogjs/vite-plugin-angular` | 2.3.0 |
| `@analogjs/vitest-angular` | 2.3.0 |
| `vitest` | 4.0.18 |
| `vite` | 7.3.1 |

## Symptom

66 test files failed with:

```
Error: Component 'FooComponent2' is not resolved:
  - templateUrl: ./foo.component.html
Did you run and wait for 'resolveComponentResources()'?
```

Only components with external `templateUrl` were affected. Inline `template` components worked fine.

## Root Cause

The bug is in `@angular/build`'s `createJitResourceTransformer` — specifically in how it traverses the TypeScript AST.

### The Traversal Failure

In `@angular/build/src/tools/angular/transformers/jit-resource-transformer.js`, the transformer uses:

```js
const updatedSourceFile = ts.visitEachChild(sourceFile, visitNode, context);
```

During Vitest's JIT compilation mode, the TypeScript builder emits **synthetic source file nodes** that have `kind === SyntaxKind.SourceFile` (312) but **fail the `ts.isSourceFile()` type guard**. This is because the synthetic nodes don't carry the internal brand/prototype that TypeScript's type guards check for.

`visitEachChild` internally calls `isSourceFile(node)` to determine how to traverse children. When it returns `false` for a SourceFile node, `visitEachChild` doesn't know how to traverse the statements, so it returns the node unchanged — **the visitor function is never called**.

This means:
1. `visitNode` is never invoked → class declarations are never visited
2. `@Component` decorators are never processed
3. `templateUrl` is never replaced with `angular:jit:template:file;` import markers
4. The Analog plugin's subsequent string replacement (which looks for `angular:jit:` markers) finds nothing to replace
5. Angular's JIT runtime encounters an unresolved `templateUrl` and throws

### Why The `2` Suffix Appeared

Separately, `angularVitestSourcemapPlugin` runs `transformWithEsbuild(code, id, { loader: 'js' })` on all `.ts` files for sourcemap alignment. Without `keepNames: true`, esbuild renames classes (e.g., `FooComponent` → `FooComponent2`). This was a red herring — even with `keepNames: true`, the templateUrl was still unresolved because the real bug is the `visitEachChild` no-op.

### Proof

```
[JIT-VISITCOUNT] stmts=8 kind=312 isSourceFile=false 0 visits for doorway-dashboard.component.ts
```

The source file has 8 statements (including a ClassDeclaration), kind=312 IS SourceFile, but `ts.isSourceFile()` returns `false`, and `visitEachChild` makes 0 visits.

## Fix

**Patch file**: `patches/@angular__build@19.2.22.patch`

The fix adds a fallback: if `visitEachChild` was a no-op (no resource imports found despite the file having statements), manually iterate the statements:

```diff
--- a/src/tools/angular/transformers/jit-resource-transformer.js
+++ b/src/tools/angular/transformers/jit-resource-transformer.js
@@ -40,7 +40,19 @@
             return typescript_1.default.visitEachChild(node, visitNode, context);
         };
         return (sourceFile) => {
-            const updatedSourceFile = typescript_1.default.visitEachChild(sourceFile, visitNode, context);
+            // Workaround: In Vitest JIT mode, the emitted source file nodes fail
+            // ts.isSourceFile() checks (they are synthetic nodes from the builder).
+            // visitEachChild uses isSourceFile internally and skips traversal when
+            // it returns false, so the visitor never reaches class declarations.
+            // Fall back to manual statement iteration when visitEachChild is a no-op.
+            let updatedSourceFile = typescript_1.default.visitEachChild(sourceFile, visitNode, context);
+            if (resourceImportDeclarations.length === 0 && sourceFile.statements.length > 0) {
+                // visitEachChild may have been a no-op — try manual iteration
+                const updatedStatements = sourceFile.statements.map(stmt => visitNode(stmt));
+                if (resourceImportDeclarations.length > 0) {
+                    updatedSourceFile = nodeFactory.updateSourceFile(sourceFile, updatedStatements);
+                }
+            }
             if (resourceImportDeclarations.length > 0) {
```

## Results

| Metric | Before | After |
|--------|--------|-------|
| Test files failed | 70 | 2 |
| Tests failed | 1565 | 8 |
| Tests passed | 5967 | 7527 |

The 2 remaining failures (`agent.service.spec.ts`, `path-negotiation.service.spec.ts`) are pre-existing — they construct services via `new Service()` outside an injection context, which is incompatible with `inject()`.

## Upstream Actions

1. **File bug on `@angular/build`**: The `createJitResourceTransformer` should not rely on `visitEachChild` working correctly for synthetic source files. The manual iteration fallback should be upstreamed.
2. **File bug on `@analogjs/vite-plugin-angular`**: The Analog plugin's `angularVitestSourcemapPlugin` should pass `keepNames: true` to `transformWithEsbuild()` to avoid confusing class rename suffixes.

## Investigation Path (for reference)

1. Initial hypothesis: esbuild class renaming causes metadata mismatch → tested `keepNames: true` → fixed the `2` suffix but templates still unresolved
2. Checked `supportJitMode` compiler option → was `false` in `readConfiguration` → set to `true` → no effect (options were already being overridden post-cache)
3. Wrapped the JIT resource transformer to log → confirmed it's called but produces no changes
4. Added `visitNode` counter → **0 visits** despite source file having 8 statements
5. Checked `ts.isSourceFile()` → returns `false` for synthetic source file nodes
6. Replaced `visitEachChild` with manual `statements.map(visitNode)` → **all 66 failures fixed**
