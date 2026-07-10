---
epr-meta-version: 1
id: cascade-root-meta
root: true
rules:
  - id: zeta-root-rule
    class: inject
    when: { write: "leaf.md" }
    require-frontmatter: [id]
  - id: collide-rule
    class: inject
    when: { write: "leaf.md" }
    require-frontmatter: [id]
---
Parity fixture 3 (Task 6 / B2) — directory-form root manifest. `collide-rule` is intentionally
overridden by the nested manifest below (nearest-wins on VALUE). `zeta-root-rule` is declared
BEFORE `collide-rule` here, and alphabetically AFTER it — the ordering is deliberately
non-alphabetical so a resolver that accidentally sorts rules by id (instead of preserving
cascade/first-seen order) is caught by this fixture rather than passing by coincidence.
Shared corpus for `.claude/scripts/_lib/__tests__/epr_meta_cascade_test.py` and
`elohim/eprfs/eprfs-meta/tests/parity.rs`. Do not hand-edit one side without the other.

The `when: { write: "leaf.md" }` scope on both rules is deliberate: `src/leaf.md` is a fictional,
never-created target path used only to resolve the cascade in the parity tests. Scoping keeps these
rules from ever firing on this manifest's own creation/edits when committed — `.epr-meta` files are
live governance input to the real git/PreToolUse hooks, not inert test data (a first version of this
fixture omitted the scope and its own `deny` rule blocked its own commit).
