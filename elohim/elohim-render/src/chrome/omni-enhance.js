/* elohim omni-enhance — progressive enhancement for the native runtime chrome.
 *
 * Hand-written vanilla JS (no framework, no build step). Wires the behavior
 * HOOKS that elohim-render::chrome::omnibar emits as `data-omni-*` attributes:
 *   - data-omni-action="toggle"        → expand/collapse #elohim-omni
 *   - data-omni-action="copy"          → clipboard write of data-omni-copy-value
 *   - data-omni-action="theme-toggle"  → flip html[data-theme], persist, dispatch
 *   - data-omni-resilience-slug        → lazy resilience fetch on first expand
 *
 * Served content-addressed at /chrome/omni-enhance.{sha256}.js — immutable.
 * Defensive by construction: every lookup is null-guarded; if the omnibar is
 * absent the script no-ops. It must never throw uncaught.
 */
(function () {
  'use strict';

  var THEME_KEY = 'elohim-theme';
  var OMNI_ID = 'elohim-omni';

  // ── Theme restore (runs immediately, before any interaction) ───────────────
  // Read the persisted theme and apply it to <html data-theme>. The native
  // chrome's CSS reacts to this attribute; absent ⇒ leave the server default
  // (which honors prefers-color-scheme).
  function restoreTheme() {
    try {
      var stored = window.localStorage ? window.localStorage.getItem(THEME_KEY) : null;
      if (stored === 'light' || stored === 'dark') {
        document.documentElement.dataset.theme = stored;
      }
    } catch (e) {
      /* localStorage may throw (private mode, disabled); ignore. */
    }
  }

  // ── aria-live announcer (lazily created, shared) ───────────────────────────
  var liveRegion = null;
  function announce(msg) {
    try {
      if (!liveRegion) {
        liveRegion = document.createElement('div');
        liveRegion.setAttribute('aria-live', 'polite');
        liveRegion.setAttribute('aria-atomic', 'true');
        // Visually hidden but available to assistive tech.
        liveRegion.style.cssText =
          'position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap;';
        document.body.appendChild(liveRegion);
      }
      liveRegion.textContent = '';
      // Re-set on a microtask so repeat messages re-announce.
      window.setTimeout(function () {
        liveRegion.textContent = msg;
      }, 30);
    } catch (e) {
      /* announcement is best-effort. */
    }
  }

  // ── Toggle expand/collapse ─────────────────────────────────────────────────
  function toggleOmni(omni) {
    if (!omni) return;
    var state = omni.getAttribute('data-omni-state');
    var next = state === 'expanded' ? 'pill' : 'expanded';
    omni.setAttribute('data-omni-state', next);
    if (next === 'expanded') {
      maybeLoadResilience(omni);
    }
  }

  // ── Copy the EPR address to the clipboard ──────────────────────────────────
  function copyValue(el) {
    if (!el) return;
    var value = el.getAttribute('data-omni-copy-value');
    if (!value) return;
    var clip = navigator.clipboard;
    if (clip && typeof clip.writeText === 'function') {
      clip.writeText(value).then(
        function () {
          announce('EPR address copied');
        },
        function () {
          announce('Copy failed');
        }
      );
    } else {
      announce('Clipboard unavailable');
    }
  }

  // ── Theme toggle (flip, persist, broadcast the ThemeStore contract) ────────
  function toggleTheme() {
    var root = document.documentElement;
    var current = root.dataset.theme === 'dark' ? 'dark' : 'light';
    var next = current === 'dark' ? 'light' : 'dark';
    root.dataset.theme = next;
    try {
      if (window.localStorage) {
        window.localStorage.setItem(THEME_KEY, next);
      }
    } catch (e) {
      /* persistence is best-effort. */
    }
    try {
      window.dispatchEvent(
        new CustomEvent('elohim-theme-changed', { detail: { theme: next } })
      );
    } catch (e) {
      /* CustomEvent should always be available; guard anyway. */
    }
    announce('Theme set to ' + next);
  }

  // ── Lazy resilience snapshot (best-effort, on first expand only) ───────────
  // The omnibar ships a neutral glyph (◯). On first expand we fetch the
  // resilience projection for the EPR slug and, if it resolves, swap the glyph.
  // On ANY failure we leave the neutral glyph — never throw, never block.
  function maybeLoadResilience(omni) {
    if (!omni || omni.getAttribute('data-omni-resilience-loaded') === '1') return;
    var slug = omni.getAttribute('data-omni-resilience-slug');
    if (!slug || typeof window.fetch !== 'function') return;
    // Mark first so a rapid re-expand doesn't double-fetch.
    omni.setAttribute('data-omni-resilience-loaded', '1');

    var url = '/api/v1/resilience/' + encodeURIComponent(slug);
    window
      .fetch(url, { headers: { accept: 'application/json' }, credentials: 'same-origin' })
      .then(function (resp) {
        if (!resp || !resp.ok) return null;
        return resp.json();
      })
      .then(function (data) {
        if (!data) return;
        applyResilience(omni, data);
      })
      .catch(function () {
        /* best-effort; leave the neutral glyph. */
      });
  }

  function applyResilience(omni, data) {
    try {
      // Defensive read: the projection shape may evolve. Prefer an explicit
      // glyph, else derive from a standing/reach hint, else leave neutral.
      var glyph = typeof data.glyph === 'string' ? data.glyph : null;
      var label =
        typeof data.standing === 'string'
          ? data.standing
          : typeof data.reach === 'string'
            ? data.reach
            : null;
      if (!glyph && !label) return;
      var marks = omni.querySelectorAll('[data-omni-resilience-glyph]');
      for (var i = 0; i < marks.length; i++) {
        if (glyph) marks[i].textContent = glyph;
        if (label) marks[i].setAttribute('title', label);
      }
    } catch (e) {
      /* never let a projection-shape surprise break the page. */
    }
  }

  // ── Event delegation (one listener; survives re-render) ────────────────────
  function onClick(evt) {
    var target = evt.target;
    if (!target || typeof target.closest !== 'function') return;
    var actionEl = target.closest('[data-omni-action]');
    if (!actionEl) return;
    var action = actionEl.getAttribute('data-omni-action');
    if (action === 'toggle') {
      toggleOmni(document.getElementById(OMNI_ID));
    } else if (action === 'copy') {
      copyValue(actionEl);
    } else if (action === 'theme-toggle') {
      toggleTheme();
    }
  }

  function init() {
    restoreTheme();
    // Delegate from the omnibar container if present, else no-op gracefully.
    var omni = document.getElementById(OMNI_ID);
    if (!omni) return;
    omni.addEventListener('click', onClick);
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
