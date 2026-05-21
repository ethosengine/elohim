# elohim-qahal — Package Conventions

Project-internal reference for primitive implementations in this package.
Keep entries brief; link to commits for full context.

---

## Sprint-1A: standing OR-group (omit `elohim-support`)

Capability JSDoc tags use the trimmed standings OR-group:

```
@capabilityRequiredStandings visitor | engaged | contributor | steward
```

`elohim-support` is deliberately omitted. An OR-group is satisfied by any
one standing, and `visitor` (the most permissive) is already present —
`elohim-support` is therefore redundant. Adding it would widen the tag
without changing the contract semantics.

Decision recorded at commit `d406fe32c` (Task 1.1 cleanup). Applies to all
primitives in this package.

---

## Sprint-1A: three-gate test suite structure

Each element has three spec files:

| File | Purpose |
|------|---------|
| `<element>.spec.ts` | Behavior + a11y + i18n + ua-prefs precondition gates |
| `<element>.manifest.spec.ts` | CEM manifest completeness + capabilityContract fields |

The behavior `describe` block (first block in `.spec.ts`) is the primary
gate. The precondition-gate describes (`ua-prefs`, `i18n`) follow in the
same file with `afterEach(() => clearMediaQueries())` for motion tests.

---

## Sprint-1A: single `before()` in manifest specs

Manifest spec files use one top-level `before()` to fetch and parse
`/dist/custom-elements.json`. Both `describe` blocks in the file share the
same parsed `decl` and `contract` variables. Do not add a second `before()`
or duplicate the fetch.

---

## Sprint-1A: logical CSS properties only

All CSS in this package uses logical properties (`padding-inline`,
`padding-block`, `inline-size`, `block-size`) rather than physical
properties (`padding-left`, `width`, `height`). The `requiresLogicalProperties`
checker in the i18n precondition-gate tests enforces this automatically.

---

## Sprint-1A: `willUpdate` for host-attribute reflection

Protected-tier marking is applied in `willUpdate` (not `render`) so the
`:host([protected])` CSS selector can be used without a delay. Tests that
exercise tier-change transitions must use `elementUpdated` from
`@open-wc/testing` to await the Lit update cycle after mutating a property
on a live element.

---

## JSDoc tag convention

Capability JSDoc uses `@prop` (not `@attr`) for all `@property`-decorated Lit fields. `@attr` is reserved for HTML-spec attributes that are NOT mapped via a Lit `@property` decorator. The canonical reference is `app/elohim-elements/elohim-core/src/elohim-button.ts`.
