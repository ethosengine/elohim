---
epr-meta-version: 1
id: root-dir-form
root: true
rules:
  - id: root-only-rule
    class: inject
    when: { write: "new.md" }
    require-frontmatter: [id]
---
Parity fixture 1 (Task 6 / B2) — directory-form root manifest, no nested cascade.
Shared corpus for `.claude/scripts/_lib/__tests__/epr_meta_cascade_test.py` and
`elohim/eprfs/eprfs-meta/tests/parity.rs`. Do not hand-edit one side without the other.

The `when: { write: "new.md" }` scope on the rule below is deliberate: `notes/new.md` is a
fictional, never-created target path used only to resolve the cascade in the parity tests. Scoping
the rule to that literal filename keeps this fixture's own `deny`/`ask`/`inject` rules from ever
firing on the fixture MANIFEST's own creation/edits when this tree is committed — `.epr-meta` files
are live governance input to the real git/PreToolUse hooks, not inert test data, so a fixture rule
with no `when` scope self-applies to its own commit.
