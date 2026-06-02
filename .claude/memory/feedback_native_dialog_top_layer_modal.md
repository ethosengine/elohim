---
name: feedback_native_dialog_top_layer_modal
description: "Native <dialog> + showModal() is the fix for 'modal slides behind / off-page' — position:fixed is NOT viewport-relative under a transformed/filtered/contain:paint ancestor; the top-layer dialog escapes all stacking contexts. Migration gotchas: assert :modal not z-index; synthetic Escape is a no-op; backdrop-click via event.target."
metadata:
  node_type: memory
  type: feedback
  originSessionId: 2026-05-07-feedback-dialogue-panel
cites:
  - app/elohim-app/src/app/elohim/components/gate-feedback/gate-feedback-modal.component.ts
---

**Native `<dialog>` + `showModal()` is the fix for "modal slides behind / off-page" — `position: fixed` is NOT viewport-relative under a transformed ancestor.**

**Why:** any ancestor with `transform`, `filter`, `perspective`, `will-change`, or `contain: paint` becomes the **containing block** for `position: fixed` descendants, so viewport centering breaks (lesson-view's `overflow:hidden` plus path-navigator transforms caused it). The robust fix is the native `<dialog>` element with `dialog.showModal()`, which renders into the browser **top layer** — above all stacking contexts, unaffected by ancestor transforms/overflow, with native `::backdrop` dimming and UA auto-centering.

**Migration gotchas (the non-obvious ones):**
- **No traditional z-index.** A top-layer `<dialog>` has `getComputedStyle().zIndex === "auto"` → `NaN`. Don't assert on z-index — assert the `:modal` pseudo-class instead.
- **Synthetic Escape is a no-op.** A synthetic `new KeyboardEvent('Escape')` does NOT trigger the UA Escape handler. Test the dismiss path via the `(close)` event / `dialog.close()`, not by dispatching Escape.
- **Backdrop-click detection** is `event.target === dialogEl` (the click landed on the dialog's own box, i.e. the backdrop area outside the content).

**How to apply:**
- If a modal centers fine in isolation but slides off-screen / behind content when embedded, suspect a transformed/`contain:paint` ancestor capturing `position:fixed` — reach for native `<dialog>` + `showModal()` rather than chasing z-index/portal hacks.
- Canonical live example: `gate-feedback-modal.component.ts` (top-layer `<dialog>`, `::backdrop` rule, `event.target` backdrop check, `(close)` event).

Canonical watch-out also planted in the `genesis/a2o/` frontend conventions (the three migration gotchas). Frontend sibling of the Che render-loop tooling [[project_che_browser_feedback_loop]].
