/* elohim omni-element — the runtime-served, self-contained EPR omnibar element.
 *
 * Hand-written vanilla JS (no framework, no build step). ONE self-contained file
 * loaded via a single <script> in ANY context — browser (doorway SSR shell),
 * /deliver CSR, and the Tauri desktop static SPA (index.html). It self-mounts,
 * acquires the wrapped EPR's context (inline-injected OR fetched), renders the
 * rich `protocol-omni` omnibar markup, applies the EPR theme, and wires the
 * behavior — all client-side, identically everywhere.
 *
 * This supersedes the older `omni-enhance.js` (which enhanced server-rendered
 * markup): the element is render + theme + behavior in one file, so there is no
 * separate server-splice to enhance. The behavior contract is preserved verbatim
 * (data-omni-* hooks, the THEME_KEY persistence, the elohim-theme-changed event,
 * the lazy /api/v1/resilience/{slug} fetch) so any sibling that depended on the
 * enhance behavior still works.
 *
 * On first expand the element resolves, lazily and best-effort, everything the
 * inline context island does NOT carry: the resilience snapshot, and the EPR's
 * BACKING CLAIMS + nav neighbors from the two Category-C EPR projections
 * (/api/v1/epr/{slug}/nav-context and /api/v1/epr/{slug}/raw). The island's job
 * is only what the client cannot know (the authoritative slug, authenticated);
 * everything else is the element's own client-side resolution.
 *
 * Served content-addressed at /chrome/omni-element.{sha256}.js — immutable.
 * Defensive by construction: every step is guarded; if context cannot be
 * acquired, or the bar is already mounted, the script no-ops. It must never
 * throw uncaught.
 *
 * Design ports (faithful, from the landed Rust):
 *   - markup ............ elohim-render::chrome::omnibar::render_omnibar_markup
 *   - style / --omni-* ... elohim-render::chrome::omnibar::render_omnibar_style
 *   - base palette ....... elohim-render::chrome::theme::base_palette
 *   - behavior ........... elohim-render::chrome::omni-enhance.js
 */
(function () {
  'use strict';

  var THEME_KEY = 'elohim-theme';
  var OMNI_ID = 'elohim-omni';
  var STYLE_ID = 'elohim-omni-style';
  var CONTEXT_SCRIPT_ID = 'elohim-omni-context';
  var LANDING_SLUG = 'elohim-host-landing';

  // ── HTML escape (port of omnibar.rs::html_escape — the XSS guard) ───────────
  // Escapes the five significant characters for text + double-quoted attribute
  // contexts. `&` is handled first so emitted entities are not double-escaped.
  function htmlEscape(raw) {
    if (raw == null) return '';
    return String(raw)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#x27;');
  }

  // ── safeHref (port of omnibar.rs::safe_href — scheme allowlist) ─────────────
  // Allow only same-origin-relative (`/…`) or http(s) absolute hrefs. Anything
  // else — `javascript:`, `data:`, `vbscript:`, a scheme-relative `//host` — is
  // dropped to the supplied safe default (or `null` to omit the link). Defensive:
  // the active paths hardcode these, but a future composition layer that feeds an
  // EPR-derived href must not be able to smuggle `javascript:` past htmlEscape.
  function safeHref(raw, fallback) {
    var fb = fallback === undefined ? '/' : fallback;
    if (raw == null) return fb;
    var s = String(raw);
    // Browsers strip tabs/newlines and treat a backslash as a slash, so
    // `/\host` and `/<TAB>/host` both become external `//host`. Refuse them.
    if (/[\\\u0000-\u001f\u007f]/.test(s)) return fb;
    // Same-origin-relative path (but not a scheme-relative `//host`).
    if (s.charAt(0) === '/' && s.charAt(1) !== '/') return s;
    if (/^https?:/i.test(s)) return s;
    return fb;
  }

  // encodeURIComponent throws URIError on a lone surrogate. Never let that
  // escape: an unencodable value is reported as null and the caller degrades.
  var SYNC_CUE_MS = 1200;

  // PURE on the island: when the answering doorway (the x-elohim-served-by
  // value) differs from the delivery path's doorway hop, move that hop to it.
  // The hop's DID belonged to the old doorway, so it is dropped. Returns true
  // only when the path changed. An absent value, no doorway hop, or the same id
  // changes nothing.
  function adoptServedBy(content, servedBy) {
    var id = typeof servedBy === 'string' ? servedBy.trim() : '';
    if (!id || !content || !content.delivery || !Array.isArray(content.delivery.path)) return false;
    var path = content.delivery.path;
    for (var i = 0; i < path.length; i++) {
      var hop = path[i];
      if (hop && hop.role === 'doorway') {
        if (hop.id === id) return false;
        hop.id = id;
        hop.reportedBy = 'doorway';
        delete hop.did;
        return true;
      }
    }
    return false;
  }

  function playSyncCue() {
    MODEL.cuePlays = (MODEL.cuePlays || 0) + 1;
    var marks = document.querySelectorAll ? document.querySelectorAll('[data-omni-sync]') : [];
    for (var i = 0; i < marks.length; i++) {
      var m = marks[i];
      if (m.hasAttribute('data-playing')) continue;
      m.removeAttribute('hidden');
      m.setAttribute('data-playing', '');
      window.setTimeout(
        guard(function (mark) {
          return function () {
            mark.removeAttribute('data-playing');
            mark.setAttribute('hidden', '');
          };
        }(m)),
        SYNC_CUE_MS
      );
    }
  }

  // Called with every readings response. Plays the cue once per change, then
  // rebuilds what the path feeds (the Delivery row/card and the EPR links).
  function noteServedBy(resp) {
    var v = resp && resp.headers && typeof resp.headers.get === 'function' ? resp.headers.get('x-elohim-served-by') : null;
    if (!adoptServedBy(MODEL.content, v)) return;
    playSyncCue();
    refreshPageInfo();
  }

  // A readings fetch that also notes which doorway answered. Pass-through: the
  // caller sees the same response.
  function tapFetch(url, opts) {
    return window.fetch(url, opts).then(function (resp) {
      try {
        noteServedBy(resp);
      } catch (e) {
        /* the cue is best-effort. */
      }
      return resp;
    });
  }

  function readingUrl(c, path) {
    var b = c && c.readingsBase;
    var sb = typeof b === 'string' && b ? safeHref(b, null) : null;
    return sb ? sb.replace(/\/+$/, '') + path : path;
  }

  function safeEncode(v) {
    try {
      return encodeURIComponent(String(v == null ? '' : v));
    } catch (e) {
      return null;
    }
  }

  // Wrap an event handler so nothing can throw uncaught from an interaction.
  function guard(fn) {
    return function () {
      try {
        return fn.apply(this, arguments);
      } catch (e) {
        /* the chrome never throws uncaught. */
      }
    };
  }

  // ── truncate_address (port of omnibar.rs: <=20 passthrough; head6...tail6) ──
  function truncateAddress(address) {
    var s = String(address == null ? '' : address);
    // Match the Rust char-count semantics for the common (BMP) case.
    var chars = Array.from(s);
    if (chars.length <= 20) return s;
    var head = chars.slice(0, 6).join('');
    var tail = chars.slice(chars.length - 6).join('');
    return head + '...' + tail;
  }

  // ── Base palette (port of theme.rs::base_palette — verbatim RGBA) ───────────
  var BASE_PALETTE = {
    light: {
      bg: 'rgba(255, 255, 255, 0.92)',
      fg: 'rgba(20, 22, 30, 0.96)',
      muted: 'rgba(20, 22, 30, 0.55)',
      border: 'rgba(20, 22, 30, 0.14)',
      accent: 'rgba(20, 22, 30, 0.96)',
      shadow: '0 1px 6px rgba(0, 0, 0, 0.08)',
      envRing: '#d97706'
    },
    dark: {
      bg: 'rgba(22, 23, 28, 0.92)',
      fg: 'rgba(232, 234, 240, 0.96)',
      muted: 'rgba(232, 234, 240, 0.55)',
      border: 'rgba(232, 234, 240, 0.16)',
      accent: 'rgba(232, 234, 240, 0.96)',
      shadow: '0 1px 6px rgba(0, 0, 0, 0.35)',
      envRing: '#d97706'
    }
  };

  // The seven --omni-* tokens, in render order. Each maps a CSS var suffix to a
  // ThemeTokens field name (camelCase on the JSON boundary).
  var TOKEN_VARS = [
    ['bg', 'bg'],
    ['fg', 'fg'],
    ['muted', 'muted'],
    ['border', 'border'],
    ['accent', 'accent'],
    ['shadow', 'shadow'],
    ['env-ring', 'envRing']
  ];

  // ── Structural / layout CSS (port of omnibar.rs::OMNI_LAYOUT_CSS) ───────────
  // Every color/shadow/border reads a var(--omni-*) with the verbatim original
  // value as the fallback, so the bar paints even if a token is unset.
  var OMNI_LAYOUT_CSS =
    "#elohim-omni { position: fixed; inset: 0 0 auto 0; z-index: 2147483000; " +
    "font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', " +
    "Roboto, Helvetica, Arial, sans-serif; font-size: 12px; line-height: 1.2; " +
    'pointer-events: none; }\n' +
    '#elohim-omni .omni-chip, #elohim-omni .omni-toolbar { pointer-events: auto; }\n' +
    '#elohim-omni .omni-pill-group { display: block; }\n' +
    '#elohim-omni .omni-chip { position: absolute; top: 0.45rem; left: 50%; ' +
    'transform: translateX(-50%); display: inline-flex; align-items: center; gap: 0.4rem; ' +
    'padding: 0.28rem 0.7rem; background: var(--omni-bg, rgba(255,255,255,0.92)); ' +
    'color: var(--omni-fg, rgba(20,22,30,0.96)); ' +
    'border: 1px solid var(--omni-border, rgba(20,22,30,0.14)); border-radius: 999px; ' +
    'box-shadow: var(--omni-shadow, 0 1px 6px rgba(0,0,0,0.08)); backdrop-filter: blur(8px); ' +
    'font: inherit; cursor: pointer; }\n' +
    '#elohim-omni .omni-chip-env { box-shadow: 0 0 0 2px var(--omni-env-ring, #d97706), ' +
    'var(--omni-shadow, 0 1px 6px rgba(0,0,0,0.08)); }\n' +
    '#elohim-omni .omni-glyph { font-size: 13px; line-height: 1; ' +
    'color: var(--omni-accent, rgba(20,22,30,0.96)); }\n' +
    '#elohim-omni .omni-label { color: var(--omni-fg, rgba(20,22,30,0.96)); }\n' +
    '#elohim-omni .omni-toolbar { position: absolute; top: 0; left: 0; right: 0; ' +
    'display: none; align-items: center; gap: 0.75rem; padding: 0.5rem 1rem; ' +
    'background: var(--omni-bg, rgba(255,255,255,0.92)); ' +
    'color: var(--omni-fg, rgba(20,22,30,0.96)); ' +
    'border-bottom: 1px solid var(--omni-border, rgba(20,22,30,0.14)); ' +
    'box-shadow: var(--omni-shadow, 0 1px 6px rgba(0,0,0,0.08)); backdrop-filter: blur(10px); }\n' +
    "#elohim-omni[data-omni-state='expanded'] .omni-toolbar { display: flex; }\n" +
    "#elohim-omni[data-omni-state='expanded'] .omni-pill-group { display: none; }\n" +
    '#elohim-omni .omni-toolbar button, #elohim-omni .omni-toolbar a { ' +
    'background: transparent; border: 1px solid var(--omni-border, rgba(20,22,30,0.14)); ' +
    'border-radius: 6px; padding: 0.25rem 0.6rem; font: inherit; ' +
    'color: var(--omni-fg, rgba(20,22,30,0.96)); text-decoration: none; cursor: pointer; }\n' +
    '#elohim-omni .omni-toolbar button:hover, #elohim-omni .omni-toolbar a:hover { ' +
    'border-color: var(--omni-accent, rgba(20,22,30,0.96)); }\n' +
    '#elohim-omni .omni-epr-group { display: inline-flex; align-items: center; gap: 0.35rem; }\n' +
    '#elohim-omni .omni-epr { display: inline-flex; align-items: center; gap: 0.5rem; }\n' +
    '#elohim-omni .omni-epr-label { color: var(--omni-muted, rgba(20,22,30,0.55)); }\n' +
    '#elohim-omni .omni-epr-value { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }\n' +
    '#elohim-omni .omni-env { display: inline-flex; align-items: center; gap: 0.35rem; ' +
    'border: 1px solid var(--omni-env-ring, #d97706); }\n' +
    '#elohim-omni .omni-env-tier { text-transform: uppercase; font-weight: 700; ' +
    'font-size: 10px; letter-spacing: 0.04em; }\n' +
    '#elohim-omni .omni-env-build { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }\n' +
    '#elohim-omni .omni-resilience-wrap { position: relative; display: inline-flex; }\n' +
    '#elohim-omni .omni-resilience { position: relative; display: inline-flex; align-items: center; ' +
    'color: var(--omni-fg, rgba(20,22,30,0.96)); padding: 0 0.25rem; ' +
    'background: transparent; border: none; font: inherit; cursor: pointer; }\n' +
    '#elohim-omni .omni-resilience-neutral { font-size: 12px; line-height: 1; }\n' +
    '#elohim-omni .omni-resilience-neutral svg { display: block; }\n' +
    // The witness mark: calm, never alarming. Amber ("shown, awaiting the
    // network's witness") is a HOLLOW ring; green ("witnessed") is FILLED, so
    // the two differ by shape as well as colour.
    '#elohim-omni [data-witness="amber"] { --omni-witness: var(--omni-witness-amber, #D4A03E); }\n' +
    '#elohim-omni [data-witness="green"] { --omni-witness: var(--omni-witness-green, #3F8F5A); }\n' +
    '#elohim-omni .omni-trustmark[data-witness]::after, #elohim-omni .omni-chip-witness { ' +
    'box-sizing: border-box; width: 8px; height: 8px; border-radius: 50%; flex: 0 0 auto; ' +
    'border: 1.5px solid var(--omni-witness, transparent); }\n' +
    '#elohim-omni .omni-trustmark[data-witness]::after { content: ""; position: absolute; ' +
    'inset-block-start: -3px; inset-inline-end: -2px; }\n' +
    '#elohim-omni [data-witness="green"].omni-chip-witness, ' +
    '#elohim-omni .omni-trustmark[data-witness="green"]::after { background: var(--omni-witness); }\n' +
    // PAGE INFO: one native popover (top layer, so the content frame can never
    // cover it), anchored under the trust mark by script.
    '#elohim-omni .omni-pi { position: fixed; inset: auto; margin: 0; pointer-events: auto; ' +
    'box-sizing: border-box; width: 21rem; max-width: calc(100vw - 1rem); ' +
    'max-height: min(70vh, 30rem); overflow: auto; padding: 0.35rem; text-align: start; ' +
    'background: var(--omni-bg, rgba(255,255,255,0.96)); color: var(--omni-fg, rgba(20,22,30,0.96)); ' +
    'border: 1px solid var(--omni-border, rgba(20,22,30,0.14)); border-radius: 12px; ' +
    'box-shadow: var(--omni-shadow, 0 2px 10px rgba(0,0,0,0.12)); backdrop-filter: blur(10px); ' +
    'font: inherit; }\n' +
    '#elohim-omni .omni-pi[hidden] { display: none; }\n' +
    '#elohim-omni .omni-pi button { font: inherit; color: inherit; background: transparent; ' +
    'border: 0; cursor: pointer; text-align: start; }\n' +
    '#elohim-omni .omni-pi button:hover, #elohim-omni .omni-pi button:focus-visible { ' +
    'background: var(--omni-border, rgba(20,22,30,0.14)); }\n' +
    '#elohim-omni .omni-pi button:focus-visible { outline: 2px solid var(--omni-accent, rgba(20,22,30,0.96)); ' +
    'outline-offset: -2px; }\n' +
    '#elohim-omni .omni-pi-row { display: grid; grid-template-columns: 1.5rem minmax(0, 1fr) auto; ' +
    'align-items: center; column-gap: 0.5rem; width: 100%; padding: 0.5rem 0.55rem; border-radius: 8px; }\n' +
    '#elohim-omni .omni-pi-icon { text-align: center; font-size: 14px; line-height: 1; }\n' +
    '#elohim-omni .omni-pi-label { display: block; font-weight: 600; }\n' +
    '#elohim-omni .omni-pi-reading { display: block; overflow: hidden; text-overflow: ellipsis; ' +
    'white-space: nowrap; color: var(--omni-muted, rgba(20,22,30,0.55)); }\n' +
    '#elohim-omni .omni-pi-head { display: flex; align-items: center; gap: 0.4rem; padding: 0.15rem 0.15rem 0.4rem; }\n' +
    '#elohim-omni .omni-pi-back { padding: 0.3rem 0.55rem; border-radius: 8px; }\n' +
    '#elohim-omni .omni-pi-title { margin: 0; font-size: 12px; font-weight: 700; }\n' +
    '#elohim-omni .omni-pi-body { padding: 0 0.55rem 0.4rem; }\n' +
    '#elohim-omni .omni-pi-body p { margin: 0 0 0.4rem 0; }\n' +
    '#elohim-omni .omni-pi-muted { color: var(--omni-muted, rgba(20,22,30,0.55)); }\n' +
    '#elohim-omni .omni-pi-fact { display: flex; align-items: baseline; gap: 0.5rem; margin: 0 0 0.3rem 0; }\n' +
    '#elohim-omni .omni-pi-fact-label { flex: 0 0 5.5rem; color: var(--omni-muted, rgba(20,22,30,0.55)); font-size: 11px; }\n' +
    '#elohim-omni .omni-pi-fact-value { min-width: 0; overflow-wrap: anywhere; }\n' +
    '#elohim-omni .omni-pi-mono { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }\n' +
    '#elohim-omni .omni-pi-cap { display: grid; grid-template-columns: minmax(0, 1fr) auto; ' +
    'align-items: center; gap: 0.1rem 0.5rem; margin: 0 0 0.5rem 0; }\n' +
    '#elohim-omni .omni-pi-cap-name { font-weight: 600; }\n' +
    '#elohim-omni .omni-pi-cap-state { grid-column: 1 / -1; color: var(--omni-muted, rgba(20,22,30,0.55)); font-size: 11px; }\n' +
    '#elohim-omni .omni-pi .omni-pi-switch { min-width: 2.75rem; padding: 0.15rem 0.5rem; text-align: center; ' +
    'border: 1px solid var(--omni-border, rgba(20,22,30,0.14)); border-radius: 999px; }\n' +
    '#elohim-omni .omni-pi .omni-pi-switch[aria-checked="true"] { ' +
    'border-color: var(--omni-accent, rgba(20,22,30,0.96)); }\n' +
    '#elohim-omni .omni-pi-foot { padding: 0.4rem 0.55rem 0.3rem; ' +
    'border-top: 1px solid var(--omni-border, rgba(20,22,30,0.14)); }\n' +
    '#elohim-omni .omni-pi a { color: var(--omni-fg, rgba(20,22,30,0.96)); text-decoration: none; ' +
    'border-bottom: 1px solid var(--omni-border, rgba(20,22,30,0.14)); }\n' +
    '#elohim-omni .omni-pi a:hover { border-bottom-color: var(--omni-accent, rgba(20,22,30,0.96)); }\n' +
    '#elohim-omni .omni-pi-resilience p { margin: 0 0 0.35rem 0; }\n' +
    '#elohim-omni .omni-resilience-headline { font-weight: 700; }\n' +
    '#elohim-omni .omni-resilience-reassure { color: var(--omni-muted, rgba(20,22,30,0.55)); }\n' +
    '#elohim-omni .omni-resilience-facts { display: block; ' +
    'color: var(--omni-muted, rgba(20,22,30,0.55)); font-size: 11px; }\n' +
    '#elohim-omni .omni-nav-slot { display: inline-flex; align-items: center; }\n' +
    '#elohim-omni .omni-claims-row { display: flex; align-items: baseline; ' +
    'gap: 0.5rem; margin-bottom: 0.3rem; }\n' +
    '#elohim-omni .omni-claims-leg { flex: 0 0 auto; min-width: 5.5rem; ' +
    'color: var(--omni-muted, rgba(20,22,30,0.55)); font-size: 11px; }\n' +
    '#elohim-omni .omni-epr-link { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; ' +
    'color: var(--omni-fg, rgba(20,22,30,0.96)); text-decoration: none; ' +
    'border-bottom: 1px solid var(--omni-border, rgba(20,22,30,0.14)); }\n' +
    // The sync cue: a quiet mark for about 1.2 s. Reduced motion keeps the mark, still.
    '#elohim-omni .omni-chip-sync { display: inline-block; font-size: 11px; line-height: 1; ' +
    'color: var(--omni-muted, rgba(20,22,30,0.55)); }\n' +
    '#elohim-omni .omni-chip-sync[hidden] { display: none; }\n' +
    '@media (prefers-reduced-motion: no-preference) { ' +
    '#elohim-omni .omni-chip-sync[data-playing] { animation: omni-sync 1.2s linear 1; } ' +
    '@keyframes omni-sync { to { transform: rotate(360deg); } } }\n' +
    '@media (pointer: coarse) { #elohim-omni .omni-pi-row, #elohim-omni .omni-pi .omni-pi-back, ' +
    '#elohim-omni .omni-pi .omni-pi-switch { min-height: 44px; min-width: 44px; } }\n' +
    '@media (prefers-reduced-motion: no-preference) { ' +
    '#elohim-omni .omni-pi[data-page-info-open] { animation: omni-pi-in 120ms ease-out; } ' +
    '@keyframes omni-pi-in { from { opacity: 0; } to { opacity: 1; } } }\n' +
    '@media (prefers-reduced-transparency: reduce) { #elohim-omni .omni-pi, #elohim-omni .omni-chip, ' +
    '#elohim-omni .omni-toolbar { backdrop-filter: none; } }\n' +
    '@media (forced-colors: active) { ' +
    '#elohim-omni .omni-pi { background: Canvas; color: CanvasText; border-color: CanvasText; } ' +
    '#elohim-omni .omni-pi button:hover, #elohim-omni .omni-pi button:focus-visible { ' +
    'background: Highlight; color: HighlightText; } ' +
    '#elohim-omni [data-witness] { --omni-witness: CanvasText; } }\n' +
    '#elohim-omni .omni-collapse { margin-left: auto; }\n';

  // ── CSS-value allowlist (port of omnibar.rs::is_safe_token_value) ───────────
  // A theme token is read raw from the EPR content node's metadata.theme and
  // spliced into a client-side <style>. Reject any value that could break out of
  // the `--omni-*: <value>;` declaration or smuggle a fetch/expression: a `;`,
  // `}`, `{`, `<`, or `@` ends/escapes the declaration or rule, and `url(` /
  // `expression(` pull in external/active content. Legitimate tokens
  // (`rgba(...)`, `#hex`, named colors, the `0 1px 6px rgba(0,0,0,0.08)`
  // box-shadow) contain none of these, so the deny-list passes them untouched.
  function isSafeTokenValue(value) {
    if (typeof value !== 'string') return false;
    if (/[;{}<@]/.test(value)) return false;
    if (/url\s*\(/i.test(value)) return false;
    if (/expression\s*\(/i.test(value)) return false;
    return true;
  }

  // ── Token-block / style synthesis (port of render_omnibar_style) ────────────
  function tokenBlock(tokens, indent) {
    var pad = indent || '  ';
    var out = '';
    for (var i = 0; i < TOKEN_VARS.length; i++) {
      var suffix = TOKEN_VARS[i][0];
      var field = TOKEN_VARS[i][1];
      var value = tokens && tokens[field] != null ? tokens[field] : BASE_PALETTE.light[field];
      // Reject an unsafe token value (CSS-injection guard) → base-palette default.
      if (!isSafeTokenValue(value)) value = BASE_PALETTE.light[field];
      out += pad + '--omni-' + suffix + ': ' + value + ';\n';
    }
    return out;
  }

  // Resolve the (light, dark) token sets the style should bind, mirroring
  // resolve_token_sets: EPR tokens occupy the slot matching their scheme, the
  // base palette supplies the other slot.
  function resolveTokenSets(theme) {
    if (!theme || !theme.tokens) {
      return { light: BASE_PALETTE.light, dark: BASE_PALETTE.dark };
    }
    var scheme = normalizeScheme(theme.colorScheme);
    if (scheme === 'dark') {
      // The EPR's tokens are its dark set; base supplies the light default.
      return { light: BASE_PALETTE.light, dark: theme.tokens };
    }
    // light | auto: EPR tokens are the light set; base supplies the dark override.
    return { light: theme.tokens, dark: BASE_PALETTE.dark };
  }

  function normalizeScheme(scheme) {
    return scheme === 'dark' || scheme === 'light' ? scheme : 'auto';
  }

  // Build the scoped style text (port of render_omnibar_style authority chain).
  function buildStyle(theme) {
    var sets = resolveTokenSets(theme);
    var scheme = normalizeScheme(theme && theme.colorScheme);
    var css = '';

    // (1) :root default — light for auto/light, dark for dark.
    var rootTokens = scheme === 'dark' ? sets.dark : sets.light;
    css += ':root {\n' + tokenBlock(rootTokens) + '}\n';

    // (2) Auto only: prefers-color-scheme dark override with the dark set.
    if (scheme === 'auto') {
      css += '@media (prefers-color-scheme: dark) {\n';
      css += '  :root:not([data-theme]) {\n';
      css += tokenBlock(sets.dark, '    ');
      css += '  }\n';
      css += '}\n';
    }

    // (3) data-theme override hooks — the client toggle always wins.
    css += ":root[data-theme='light'] {\n" + tokenBlock(sets.light) + '}\n';
    css += ":root[data-theme='dark'] {\n" + tokenBlock(sets.dark) + '}\n';

    css += OMNI_LAYOUT_CSS;
    // The content's themeColor tints the chip accent only where it stays legible.
    var accent = legibleAccent(theme && theme.themeColor, scheme);
    if (accent) css += '#elohim-omni .omni-chip { --omni-accent: ' + accent + '; }\n';
    return css;
  }

  // PURE. "#rgb", "#rrggbb" or "rgb(r, g, b)" -> [r, g, b]; anything else null.
  function parseColor(s) {
    if (typeof s !== 'string') return null;
    var v = s.trim();
    var m = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(v);
    if (m) {
      var h = m[1].length === 3 ? m[1].replace(/./g, '$&$&') : m[1];
      return [parseInt(h.slice(0, 2), 16), parseInt(h.slice(2, 4), 16), parseInt(h.slice(4, 6), 16)];
    }
    m = /^rgb\(\s*(\d{1,3})\s*,\s*(\d{1,3})\s*,\s*(\d{1,3})\s*\)$/i.exec(v);
    if (m) {
      var rgb = [Number(m[1]), Number(m[2]), Number(m[3])];
      return rgb.every(function (x) { return x <= 255; }) ? rgb : null;
    }
    return null;
  }
  function luminance(rgb) {
    var c = rgb.map(function (x) {
      var s = x / 255;
      return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
    });
    return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
  }
  function contrast(a, b) {
    var la = luminance(a);
    var lb = luminance(b);
    return (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05);
  }
  // PURE. The canonical "rgb(r, g, b)" of a theme colour when it has at least
  // 3:1 contrast against the chip background(s) of the scheme; '' otherwise
  // (unparseable colours are ignored).
  function legibleAccent(color, scheme) {
    var rgb = parseColor(color);
    if (!rgb) return '';
    var light = [255, 255, 255];
    var dark = [22, 23, 28];
    var bgs = scheme === 'dark' ? [dark] : scheme === 'light' ? [light] : [light, dark];
    for (var i = 0; i < bgs.length; i++) if (contrast(rgb, bgs[i]) < 3) return '';
    return 'rgb(' + rgb.join(', ') + ')';
  }

  function applyStyle(theme) {
    try {
      var existing = document.getElementById(STYLE_ID);
      if (existing && existing.parentNode) existing.parentNode.removeChild(existing);
      var style = document.createElement('style');
      style.id = STYLE_ID;
      style.textContent = buildStyle(theme);
      (document.head || document.documentElement).appendChild(style);
    } catch (e) {
      /* styling is best-effort; the var() fallbacks paint a usable bar. */
    }
  }

  // ── Markup synthesis (port of render_omnibar_markup) ────────────────────────
  // Returns the inner HTML for #elohim-omni. Every interpolated value is
  // HTML-escaped. Behavior is expressed as data-omni-* hooks the listener wires.
  function buildMarkup(ctx) {
    var slug = ctx.slug || '';
    var title = ctx.title || '';
    var slugAttr = htmlEscape(slug);
    var titleAttr = htmlEscape(title);
    var truncated = htmlEscape(truncateAddress(slug));

    var envTier = ctx.envTier || '';
    var buildMarker = ctx.buildMarker || '';
    var envVisible =
      !!ctx.showEnv && envTier !== '' && envTier !== 'production';
    var shortBuild = buildMarker ? Array.from(buildMarker).slice(0, 7).join('') : '';

    var html = '';

    // === COLLAPSED CHIP: the `elohim-protocol` pill ===
    html += '<div class="omni-pill-group">';
    var chipClass = envVisible ? 'omni-chip omni-chip-env' : 'omni-chip';
    var island = contentOf(ctx);
    var witness = island && (island.witness === 'green' || island.witness === 'amber') ? island.witness : '';
    var witnessNote =
      witness === 'amber'
        ? ' (shown, awaiting the network\'s witness)'
        : witness === 'green'
          ? ' (witnessed by the network)'
          : '';
    html +=
      '<button type="button" class="' +
      chipClass +
      '" data-omni-action="toggle" aria-label="' +
      htmlEscape('Elohim Protocol — click for details' + witnessNote) +
      '">';
    html += '<span class="omni-glyph" aria-hidden="true">⯂</span>';
    html += '<span class="omni-label">elohim-protocol</span>';
    html +=
      '<span class="omni-chip-sync" data-omni-sync role="img" ' +
      'aria-label="Delivery moved to another doorway of this name" hidden>↻</span>';
    if (witness) {
      html += '<span class="omni-chip-witness" data-witness="' + witness + '" aria-hidden="true"></span>';
    }
    html += '</button>';
    html += '</div>';

    // === EXPANDED TOOLBAR ===
    html += '<div class="omni-toolbar" role="region" aria-label="Protocol context">';

    // Back nav SLOT. The island may supply the target (ctx.navBack); when it
    // does not, the element resolves it itself from the EPR nav-context API on
    // first expand (`fillNavSlot`) — the slot is the insertion point, so the
    // toolbar order is identical either way.
    html += '<span class="omni-nav-slot" data-omni-nav-slot="back">';
    var navBack = ctx.navBack;
    var navBackHref = navBack ? safeHref(navBack.href, null) : null;
    if (navBackHref) {
      var bh = htmlEscape(navBackHref);
      var bl = htmlEscape(navBack.label || '');
      html +=
        '<a href="' +
        bh +
        '" class="omni-nav omni-nav-back" title="' +
        bl +
        '" aria-label="' +
        bl +
        '">← ' +
        bl +
        '</a>';
    }
    html += '</span>';

    // EPR group: copy-CID button + (optional) env-context badge.
    html += '<span class="omni-epr-group">';
    html +=
      '<button type="button" class="omni-epr" data-omni-action="copy" ' +
      'data-omni-copy-value="' +
      slugAttr +
      '" title="Copy content identifier for ' +
      titleAttr +
      '" aria-label="Copy content identifier">';
    html += '<span class="omni-epr-label">EPR</span>';
    html += '<code class="omni-epr-value">' + truncated + '</code>';
    html += '</button>';

    if (envVisible) {
      var tier = htmlEscape(envTier);
      html +=
        '<a href="/doorway/elohim" class="omni-env" title="Serving environment: ' +
        tier +
        '" aria-label="Serving environment: ' +
        tier +
        '">';
      html += '<span class="omni-env-tier">' + tier + '</span>';
      if (shortBuild) {
        html += '<code class="omni-env-build">' + htmlEscape(shortBuild) + '</code>';
      }
      html += '</a>';
    }
    html += '</span>';

    // The TRUST MARK. The resilience dial is the one glyph on the bar that opens
    // PAGE INFO (Chrome's page-info idiom); it carries the witness (amber|green)
    // from the island. The standalone claims button is gone — claims live in
    // page info — so the bar's root stays quiet. The popover itself is emitted
    // after the toolbar (a sibling), not here.
    html += '<span class="omni-resilience-wrap">';
    html +=
      '<button type="button" class="omni-resilience omni-trustmark" ' +
      'data-omni-action="page-info" aria-expanded="false" aria-haspopup="dialog" ' +
      'aria-controls="' + PI_ID + '" aria-label="' +
      htmlEscape('Page info' + witnessNote) + '" title="Page info"' +
      (witness ? ' data-witness="' + witness + '"' : '') + '>';
    html +=
      '<span class="omni-resilience-neutral" data-omni-resilience-glyph ' +
      'title="Resilience snapshot unavailable">◉</span>';
    html += '</button>';
    html += '</span>';

    // Forward nav SLOT (island target if supplied, else nav-context-resolved).
    html += '<span class="omni-nav-slot" data-omni-nav-slot="forward">';
    var navFwd = ctx.navForward;
    var navFwdHref = navFwd ? safeHref(navFwd.href, null) : null;
    if (navFwdHref) {
      var fh = htmlEscape(navFwdHref);
      var fl = htmlEscape(navFwd.label || '');
      html +=
        '<a href="' +
        fh +
        '" class="omni-nav omni-nav-forward" title="' +
        fl +
        '" aria-label="' +
        fl +
        '">' +
        fl +
        ' →</a>';
    }
    html += '</span>';

    // Fair-trade receipt — what was traded for this visit, in the words a
    // friend would use, with the precise form ONE request away (never the first
    // thing said). Absent when the doorway that answered accounted for nothing:
    // a chrome that always showed a receipt line would be asserting an exchange
    // nobody recorded.
    var receipt = ctx.receipt;
    if (receipt && receipt.href) {
      var receiptHref = htmlEscape(safeHref(receipt.href, ''));
      if (receiptHref) {
        html +=
          '<a href="' +
          receiptHref +
          '" class="omni-receipt" data-omni-receipt title="' +
          htmlEscape(receipt.sentence || '') +
          '">' +
          htmlEscape(receipt.label || 'What this visit traded') +
          '</a>';
      }
    }

    // Account link — hidden + data-omni-account hook when unauthenticated.
    var accountHref = htmlEscape(safeHref(ctx.accountHref || '/account', '/account'));
    var hiddenAttr = isSignedIn(ctx) ? '' : ' hidden';
    html +=
      '<a href="' +
      accountHref +
      '" class="omni-account" data-omni-account aria-label="Account"' +
      hiddenAttr +
      '>◐</a>';

    // Theme toggle (opt-in; default on).
    if (ctx.showThemeToggle !== false) {
      html +=
        '<button type="button" class="omni-theme" data-omni-action="theme-toggle" ' +
        'aria-label="Toggle theme">◑</button>';
    }

    // Build marker inline — only when present AND not shown in the env badge.
    if (buildMarker && !envVisible) {
      html += '<span class="omni-marker">via ' + htmlEscape(buildMarker) + '</span>';
    }

    // Collapse.
    html +=
      '<button type="button" class="omni-collapse" data-omni-action="toggle" ' +
      'aria-label="Collapse">×</button>';

    html += '</div>';

    // PAGE INFO popover: native top-layer element, sibling of the toolbar so the
    // toolbar's own button styles and display rules never touch it.
    html +=
      '<div id="' + PI_ID + '" class="omni-pi" popover="manual" role="dialog" ' +
      'aria-label="Page info" data-testid="page-info"></div>';
    return html;
  }

  // ── Theme restore (port of omni-enhance.js) ─────────────────────────────────
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

  // ── aria-live announcer ─────────────────────────────────────────────────────
  var liveRegion = null;
  function announce(msg) {
    try {
      if (!liveRegion) {
        liveRegion = document.createElement('div');
        liveRegion.setAttribute('aria-live', 'polite');
        liveRegion.setAttribute('aria-atomic', 'true');
        liveRegion.style.cssText =
          'position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap;';
        document.body.appendChild(liveRegion);
      }
      liveRegion.textContent = '';
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
    if (next !== 'expanded') closePageInfo(false);
    if (next === 'expanded') loadFacing(omni);
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

  // ── Theme toggle (flip, persist, broadcast) ────────────────────────────────
  function toggleTheme() {
    var root = document.documentElement;
    var current = root.dataset.theme === 'dark' ? 'dark' : 'light';
    var next = current === 'dark' ? 'light' : 'dark';
    root.dataset.theme = next;
    try {
      if (window.localStorage) window.localStorage.setItem(THEME_KEY, next);
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
  // data-omni-resilience-loaded is TRI-STATE so the DOM itself testifies:
  // "loading" (fetch in flight / re-entry guard) → "applied" (glyph+card
  // populated) or "unmatched" (fetch failed OR payload had no mappable
  // fields — the state that distinguishes network trouble from a
  // contract mismatch at a glance).
  function maybeLoadResilience(omni) {
    if (!omni || omni.getAttribute('data-omni-resilience-loaded')) return;
    var slug = omni.getAttribute('data-omni-resilience-slug');
    if (!slug || typeof window.fetch !== 'function') return;
    omni.setAttribute('data-omni-resilience-loaded', 'loading');

    // Prefer the /household variant (felt-status idiom, omnibar spec §11);
    // fall back to the base snapshot for older storage builds.
    var encSlug = safeEncode(slug);
    if (encSlug === null) {
      omni.setAttribute('data-omni-resilience-loaded', 'unmatched');
      return;
    }
    var base = readingUrl(MODEL.ctx, '/api/v1/resilience/' + encSlug);
    var opts = { headers: { accept: 'application/json' }, credentials: 'same-origin' };
    function fetchBase() {
      return tapFetch(base, opts).then(function (r2) {
        if (!r2 || !r2.ok) return null;
        return r2.json();
      });
    }
    tapFetch(base + '/household', opts)
      .then(function (resp) {
        if (resp && resp.ok) {
          // A 200 whose body fails to parse (e.g. an older build's catch-all
          // serving the HTML shell) still falls back to the base snapshot.
          return resp.json().catch(fetchBase);
        }
        return fetchBase();
      })
      .then(function (data) {
        if (!data) {
          omni.setAttribute('data-omni-resilience-loaded', 'unmatched');
          refreshPageInfo();
          return;
        }
        applyResilience(omni, data);
      })
      .catch(function () {
        /* best-effort; leave the neutral glyph. */
        omni.setAttribute('data-omni-resilience-loaded', 'unmatched');
      });
  }

  // ── Resilience dial — deterministic three-plane glyph ─────────────────────
  // One dial, three complications in FIXED slots (watch-face discipline —
  // nothing moves, only state changes; same snapshot → same drawing; every
  // state keeps the vertical mirror axis). Design sheet:
  // app/graphos/design-assets/curated/omni-resilience-dial/index.html
  //   bezel ring  = doorway plane (reach hint): solid=local · +crown=local-hub
  //                 · dashed=remote
  //   orbit triad = p2p plane (replication vs the content's floor): symmetric
  //                 configs 0={} 1={bottom} 2={top pair + chord} 3=all; the
  //                 triad CLOSES only when the floor is met (protected)
  //   center mark = dht plane (trust): commitment-backed → amber 4-point star
  //                 (the elohim seal), else hollow dot
  // Unmeasured stays honest: all orbit slots faint, never a fake verdict.
  var SVG_NS = 'http://www.w3.org/2000/svg';
  var DIAL_AMBER = 'var(--omni-amber, #D4A03E)';

  function svgEl(name, attrs) {
    var e = document.createElementNS(SVG_NS, name);
    for (var k in attrs) {
      if (Object.prototype.hasOwnProperty.call(attrs, k)) e.setAttribute(k, String(attrs[k]));
    }
    return e;
  }

  // Doorway-plane hint from the serving location (client-side, best-effort):
  // loopback (Tauri sidecar, local dev stack) → local; mDNS *.local or
  // private-LAN host → local-hub; anything else → remote.
  function reachHint() {
    try {
      var h = (window.location && window.location.hostname) || '';
      if (h === 'localhost' || h === '127.0.0.1' || h === '[::1]' || h === 'tauri.localhost') {
        return 'local';
      }
      if (
        /\.local$/.test(h) ||
        /^10\./.test(h) ||
        /^192\.168\./.test(h) ||
        /^172\.(1[6-9]|2\d|3[01])\./.test(h)
      ) {
        return 'hub';
      }
      return 'remote';
    } catch (e) {
      return 'remote';
    }
  }

  function renderDial(state) {
    var s = svgEl('svg', { width: 18, height: 18, viewBox: '0 0 24 24', 'aria-hidden': 'true' });
    var cx = 12;
    var cy = 12;
    // bezel — doorway plane
    var bezel = svgEl('circle', {
      cx: cx, cy: cy, r: 10.5, fill: 'none', stroke: 'currentColor', 'stroke-width': 1.6
    });
    if (state.reach === 'remote') {
      bezel.setAttribute('stroke-dasharray', '2.4 2.2');
      bezel.setAttribute('opacity', '0.65');
    }
    s.appendChild(bezel);
    if (state.reach === 'hub') {
      // the "crown" at 12 o'clock — the home-hub antenna, on the mirror axis
      s.appendChild(svgEl('line', {
        x1: 12, y1: 0.2, x2: 12, y2: 3.4,
        stroke: 'currentColor', 'stroke-width': 1.6, 'stroke-linecap': 'round'
      }));
    }
    // orbit triad — p2p plane (slots: 0=topL 210° · 1=topR 330° · 2=bottom 90°)
    var R = 6.8;
    var pts = [210, 330, 90].map(function (a) {
      var r = (a * Math.PI) / 180;
      return { x: cx + R * Math.cos(r), y: cy + R * Math.sin(r) };
    });
    var lit = state.measured ? Math.max(0, Math.min(3, state.lit)) : 0;
    // symmetric configuration per count — a state display, not an accumulation
    var CONFIG = [[], [2], [0, 1], [0, 1, 2]];
    var on = CONFIG[lit];
    var stroke = state.sealed ? DIAL_AMBER : 'currentColor';
    var chords =
      lit >= 2 ? (lit === 3 && state.floorMet ? [[0, 1], [1, 2], [2, 0]] : [[0, 1]]) : [];
    for (var c = 0; c < chords.length; c++) {
      s.appendChild(svgEl('line', {
        x1: pts[chords[c][0]].x, y1: pts[chords[c][0]].y,
        x2: pts[chords[c][1]].x, y2: pts[chords[c][1]].y,
        stroke: stroke, 'stroke-width': 1.2, opacity: 0.9
      }));
    }
    for (var i = 0; i < pts.length; i++) {
      var isOn = on.indexOf(i) !== -1;
      s.appendChild(svgEl('circle', {
        cx: pts[i].x, cy: pts[i].y, r: isOn ? 2.0 : 1.4,
        fill: isOn ? stroke : 'none',
        stroke: isOn ? 'none' : 'currentColor', 'stroke-width': 1.0,
        opacity: isOn ? 1 : state.measured ? 0.4 : 0.22
      }));
    }
    // center — dht trust plane
    if (state.sealed) {
      var d = '';
      for (var j = 0; j < 8; j++) {
        var a = -Math.PI / 2 + (j * Math.PI) / 4;
        var r = j % 2 === 0 ? 3.4 : 1.05;
        d += (j === 0 ? 'M' : 'L') + (cx + r * Math.cos(a)).toFixed(2) + ' ' +
          (cy + r * Math.sin(a)).toFixed(2);
      }
      s.appendChild(svgEl('path', { d: d + 'Z', fill: DIAL_AMBER }));
    } else {
      s.appendChild(svgEl('circle', {
        cx: cx, cy: cy, r: 1.5, fill: 'none',
        stroke: 'currentColor', 'stroke-width': 1, opacity: 0.55
      }));
    }
    return s;
  }

  // Map the ResilienceSnapshotView wire shape onto the dial's three planes.
  function dialStateFromSnapshot(data, status, felt) {
    var floor = felt && felt.floor && typeof felt.floor === 'object' ? felt.floor : null;
    var has =
      floor && typeof floor.hasHouseholds === 'number'
        ? floor.hasHouseholds
        : typeof data.stewardingCollectives === 'number'
          ? data.stewardingCollectives
          : 0;
    return {
      reach: reachHint(),
      measured: data.distributionState !== 'unmeasured',
      lit: Math.min(3, has),
      floorMet: status === 'protected',
      sealed:
        typeof data.commitmentBackedCollectives === 'number' &&
        data.commitmentBackedCollectives > 0
    };
  }

  // Paint the dial on every glyph mark. One holders snapshot feeds the dial and
  // the Holders row/card alike.
  function paintDial(omni, ds, label) {
    var marks = omni.querySelectorAll('[data-omni-resilience-glyph]');
    for (var i = 0; i < marks.length; i++) {
      while (marks[i].firstChild) marks[i].removeChild(marks[i].firstChild);
      marks[i].appendChild(renderDial(ds));
      if (label) marks[i].setAttribute('title', label);
      // page-model legibility: the DOM testifies each plane's state
      marks[i].setAttribute('data-omni-dial-reach', ds.reach);
      marks[i].setAttribute('data-omni-dial-lit', String(ds.lit));
      marks[i].setAttribute('data-omni-dial-floor', ds.floorMet ? 'met' : 'short');
      marks[i].setAttribute('data-omni-dial-sealed', ds.sealed ? '1' : '0');
      marks[i].setAttribute('data-omni-dial-measured', ds.measured ? '1' : '0');
    }
  }

  // PURE. The dial's planes from the facing's holders lens. The facing carries
  // no commitment-backed count, so the seal is not drawn from it.
  function dialStateFromHolders(lens) {
    return {
      reach: reachHint(),
      measured: lens.reassurance !== 'not-yet-seen',
      lit: Math.max(0, Math.min(3, Number(lens.holders) || 0)),
      floorMet: lens.reassurance === 'protected',
      sealed: false
    };
  }

  function applyResilience(omni, data) {
    try {
      // Real wire contract (resilience-snapshot-view.schema.json):
      // protectionStatus + coverageShortfall (+ feltStatus on /household).
      var status =
        typeof data.protectionStatus === 'string' ? data.protectionStatus : null;
      var felt = data.feltStatus && typeof data.feltStatus === 'object' ? data.feltStatus : null;
      var headline = felt && typeof felt.headline === 'string' ? felt.headline : null;
      var label = headline || (status ? 'Resilience: ' + status : null);
      if (!status && !label) {
        omni.setAttribute('data-omni-resilience-loaded', 'unmatched');
        refreshPageInfo();
        return;
      }
      omni.setAttribute('data-omni-resilience-loaded', 'applied');
      paintDial(omni, dialStateFromSnapshot(data, status, felt), label);
      MODEL.resilience = { data: data, status: status, felt: felt };
      refreshPageInfo();
    } catch (e) {
      /* never let a projection-shape surprise break the page — but never
       * strand the marker on "loading" either (a post-"applied" throw keeps
       * its applied state; only an early throw downgrades). */
      if (omni.getAttribute('data-omni-resilience-loaded') === 'loading') {
        omni.setAttribute('data-omni-resilience-loaded', 'unmatched');
      }
    }
  }

  /// Fill the drilldown card from the snapshot. Text via textContent only —
  /// snapshot strings never touch innerHTML.
  function renderResilienceCard(card, data, status, felt) {
    if (!card) return;
    while (card.firstChild) card.removeChild(card.firstChild);

    function line(cls, text) {
      if (!text) return;
      var p = document.createElement('p');
      p.className = cls;
      p.textContent = text;
      card.appendChild(p);
    }

    line('omni-resilience-headline', (felt && felt.headline) || (status ? 'Resilience: ' + status : null));
    line('omni-resilience-reassure', felt && felt.reassurance);

    var facts = [];
    if (status) facts.push('protection: ' + status);
    if (typeof data.coverageShortfall === 'number' && data.coverageShortfall > 0) {
      facts.push('coverage shortfall: ' + data.coverageShortfall);
    }
    if (typeof data.stewardingCollectives === 'number') {
      facts.push('stewarding collectives: ' + data.stewardingCollectives);
    }
    if (typeof data.diversityScore === 'number') {
      facts.push('diversity: ' + data.diversityScore);
    }
    if (typeof data.distributionState === 'string') {
      facts.push('distribution: ' + data.distributionState);
    }
    if (facts.length) {
      var span = document.createElement('span');
      span.className = 'omni-resilience-facts';
      span.textContent = facts.join(' · ');
      card.appendChild(span);
    }
  }

  // ── Backing claims (EPR nav-context + raw coupling legs) ──────────────────
  // The EPR an omnibar wraps is not a bare page: it is coupled to the claims
  // that back it — a STORY leg (knowledge), a VALUE leg, a GOVERNANCE leg —
  // and it sits inside a graph (what it is part of, what it relates to, what
  // it supersedes / was superseded by). Two read-only Category-C projections
  // already serve exactly that, and the element resolves BOTH itself on first
  // expand (same trigger as the resilience snapshot):
  //   GET /api/v1/epr/{slug|cid}/nav-context → prev · next · partOf[] · related[]
  //   GET /api/v1/epr/{slug|cid}/raw         → coupling{knowledge,value,governance}
  // Both accept a schema_key slug as well as a CIDv1, so the omnibar's slug
  // addresses them directly. They are fetched in PARALLEL with INDEPENDENT
  // failure handling: either one alone still fills what it knows, and neither
  // can break the page.
  //
  // data-omni-claims-loaded is TRI-STATE (mirrors the resilience marker so the
  // DOM itself testifies): "loading" (fetches in flight / re-entry guard) →
  // "applied" (at least one claim link rendered) or "unmatched" (both
  // projections absent/404/empty — nothing to show, so the group stays hidden
  // and the omnibar stays quiet).
  var CLAIM_LEGS = [
    ['knowledge', 'story'],
    ['value', 'value'],
    ['governance', 'governance']
  ];
  var CLAIM_REF_CAP = 4;

  // An EPR ref is either an EprNavRef object ({cid,label?,resilienceTier?} —
  // epr-nav-context-view.schema.json) or a bare CID string (the /raw coupling
  // legs + reversePartOf). Normalize to the CID.
  function refCid(ref) {
    if (!ref) return '';
    if (typeof ref === 'string') return ref;
    return typeof ref.cid === 'string' ? ref.cid : '';
  }

  // The universal EPR address (doorway §12.1): every claim link targets
  // /epr/{cid}, which serves the lens-complete shell viewer for that atom.
  function eprHref(cid) {
    var e = safeEncode(cid);
    return e === null ? null : safeHref('/epr/' + e, null);
  }

  // Build a claim link element. textContent only — projection strings never
  // touch innerHTML — and the href goes through the safeHref allowlist.
  function eprLinkEl(ref) {
    var cid = refCid(ref);
    if (!cid) return null;
    var href = eprHref(cid);
    if (!href) return null;
    var a = document.createElement('a');
    a.className = 'omni-epr-link';
    a.setAttribute('href', href);
    a.setAttribute('data-omni-epr-link', cid);
    a.setAttribute('title', cid);
    var label = ref && typeof ref.label === 'string' && ref.label ? ref.label : truncateAddress(cid);
    a.textContent = label;
    if (ref && typeof ref.resilienceTier === 'string') {
      a.setAttribute('data-omni-epr-tier', ref.resilienceTier);
    }
    return a;
  }

  // Fill a nav slot the inline island did NOT supply. A slot with a child is
  // island-authored and is never overwritten — the island still wins when it
  // speaks; the element only resolves what was left to it.
  function fillNavSlot(omni, dir, ref) {
    var slot = omni.querySelector('[data-omni-nav-slot="' + dir + '"]');
    if (!slot || slot.firstChild) return false;
    var cid = refCid(ref);
    if (!cid) return false;
    var href = eprHref(cid);
    if (!href) return false;
    var label = (ref && typeof ref.label === 'string' && ref.label) || truncateAddress(cid);
    var a = document.createElement('a');
    a.className = 'omni-nav omni-nav-' + dir;
    a.setAttribute('href', href);
    a.setAttribute('title', label);
    a.setAttribute('aria-label', label);
    a.setAttribute('data-omni-nav-resolved', 'nav-context');
    a.textContent = dir === 'back' ? '← ' + label : label + ' →';
    slot.appendChild(a);
    return true;
  }

  // Render the claims rows into `card`. Returns the number of links rendered (0 ⇒ nothing
  // to show; the caller keeps the group hidden).
  function renderClaimsCard(card, nav, raw) {
    if (!card) return 0;
    while (card.firstChild) card.removeChild(card.firstChild);
    var count = 0;

    function row(legKey, legLabel, ref) {
      var link = eprLinkEl(ref);
      if (!link) return;
      var r = document.createElement('div');
      r.className = 'omni-claims-row';
      r.setAttribute('data-omni-claim-leg', legKey);
      var l = document.createElement('span');
      l.className = 'omni-claims-leg';
      l.textContent = legLabel;
      r.appendChild(l);
      r.appendChild(link);
      card.appendChild(r);
      count += 1;
    }

    // (1) The three coupling legs — the EPR's backing claims (EprRawView).
    var coupling = raw && raw.coupling && typeof raw.coupling === 'object' ? raw.coupling : {};
    for (var i = 0; i < CLAIM_LEGS.length; i++) {
      row(CLAIM_LEGS[i][0], CLAIM_LEGS[i][1], coupling[CLAIM_LEGS[i][0]]);
    }

    // (2) part of — nav-context's partOf refs, else /raw's reversePartOf CIDs
    //     (independent failure handling: whichever projection answered).
    var partOf = nav && Array.isArray(nav.partOf) && nav.partOf.length ? nav.partOf : null;
    if (!partOf && raw && Array.isArray(raw.reversePartOf)) partOf = raw.reversePartOf;
    var j;
    if (partOf) {
      for (j = 0; j < partOf.length && j < CLAIM_REF_CAP; j++) {
        row('part-of', j === 0 ? 'part of' : '', partOf[j]);
      }
    }

    // (3) related — outbound coupling targets + claim CIDs (nav-context).
    if (nav && Array.isArray(nav.related)) {
      for (j = 0; j < nav.related.length && j < CLAIM_REF_CAP; j++) {
        row('related', j === 0 ? 'related' : '', nav.related[j]);
      }
    }

    return count;
  }

  function applyClaims(omni, nav, raw) {
    try {
      if (nav) {
        fillNavSlot(omni, 'back', nav.prev);
        fillNavSlot(omni, 'forward', nav.next);
      }
      // Counted against a detached node; the rows are rendered into page info.
      var count = renderClaimsCard(document.createElement('div'), nav, raw);
      MODEL.nav = nav;
      MODEL.raw = raw;
      MODEL.claimsCount = count;
      omni.setAttribute('data-omni-claims-count', String(count));
      // Nothing recorded (or neither projection answered): the Claims row says
      // so honestly instead of offering an empty flyout.
      omni.setAttribute('data-omni-claims-loaded', count ? 'applied' : 'unmatched');
      refreshPageInfo();
    } catch (e) {
      /* never let a projection-shape surprise break the page — and never
       * strand the marker on "loading". */
      if (omni.getAttribute('data-omni-claims-loaded') === 'loading') {
        omni.setAttribute('data-omni-claims-loaded', 'unmatched');
        refreshPageInfo();
      }
    }
  }

  function maybeLoadClaims(omni) {
    if (!omni || omni.getAttribute('data-omni-claims-loaded')) return;
    var slug = omni.getAttribute('data-omni-slug');
    if (!slug || typeof window.fetch !== 'function') return;
    omni.setAttribute('data-omni-claims-loaded', 'loading');

    var encSlug = safeEncode(slug);
    if (encSlug === null) {
      omni.setAttribute('data-omni-claims-loaded', 'unmatched');
      return;
    }
    var base = readingUrl(MODEL.ctx, '/api/v1/epr/' + encSlug);
    var opts = { headers: { accept: 'application/json' }, credentials: 'same-origin' };
    // Each leg swallows its own failure (404 on an un-seeded atom is the
    // COMMON case today) and resolves to null — one absent projection never
    // denies the other.
    function get(url) {
      return tapFetch(url, opts)
        .then(function (resp) {
          if (!resp || !resp.ok) return null;
          return resp.json().catch(function () {
            return null;
          });
        })
        .catch(function () {
          return null;
        });
    }

    Promise.all([get(base + '/nav-context'), get(base + '/raw')])
      .then(function (both) {
        applyClaims(omni, both[0], both[1]);
      })
      .catch(function () {
        omni.setAttribute('data-omni-claims-loaded', 'unmatched');
      });
  }

  // ── Island content, capabilities and the frame (the trust edge) ─────────────
  // The doorway projects a `content` object into the island (see the island
  // contract). The chrome loads FIRST and then creates the content frame from
  // it, computing the frame's sandbox/allow itself. Nothing here decides
  // authority: the record declares, the person narrows, the chrome only
  // intersects the two.
  //
  // The vocabulary is GENERATED from the SDK schemas (frame-capability and
  // page-info-section) and spliced in when the crate is built; there is no
  // hand-written table. Unspliced (a bare source file), it is empty: no
  // capability is ever given and the base sandbox applies.
  var VOCAB = /*__CHROME_VOCABULARY__*/null;
  // Plain-language names + the attribute each capability maps to. ORDER is the
  // order of the emitted attributes (deterministic output). Device capabilities
  // are never given without the person's own grant (grants[key] === true).
  var CAPABILITIES = ((VOCAB && VOCAB.frameCapabilities) || []).map(function (r) {
    return { key: r.value, name: r.label, sandbox: r.sandbox, allow: r.allow, device: r.device };
  });

  // Is this capability given, given the person's grants? Ordinary capabilities
  // are given unless the person narrowed them (false); device capabilities only
  // when the person granted them (true).
  function capGranted(cap, grants) {
    return cap.device ? grants[cap.key] === true : grants[cap.key] !== false;
  }

  // Boundaries that give the content an origin of its own, apart from the chrome.
  // Only on these may a trustful frame keep its own storage (allow-same-origin).
  var ORIGIN_SEPARATING = ['origin-pair', 'scheme-per-record', 'separate-webview'];

  // PURE: the frame's sandbox/allow from the island content + the person's
  // grants + the medium's isolation. Exposed for tests
  // (window.__elohimOmni.frameAttributes).
  //   trustful  -> what the record declares, intersected with the person's
  //                narrowing; plus its own storage ONLY when the medium
  //                separates the content's origin from the chrome's.
  //   anything else (trustless) -> script only, nothing extra.
  // A boundary of none, opaque-sandbox, or a missing isolation never gets
  // the content's own origin. The frame is never given a way to navigate the
  // top page or to open windows that escape the sandbox.
  function frameAttributes(content, grants, isolation) {
    var c = content && typeof content === 'object' ? content : {};
    var g = grants && typeof grants === 'object' ? grants : {};
    var iso = isolation || c.isolation;
    var boundary = iso && typeof iso === 'object' && typeof iso.boundary === 'string' ? iso.boundary : 'none';
    var trustful = c.mode === 'trustful';
    var ownOrigin = trustful && ORIGIN_SEPARATING.indexOf(boundary) !== -1;
    var declared = trustful && Array.isArray(c.capabilities) ? c.capabilities : [];
    var sandbox = ['allow-scripts'];
    if (ownOrigin) sandbox.push('allow-same-origin');
    var allow = [];
    var granted = [];
    for (var i = 0; i < CAPABILITIES.length; i++) {
      var cap = CAPABILITIES[i];
      var given = declared.indexOf(cap.key) !== -1 && capGranted(cap, g);
      if (given) {
        granted.push(cap.key);
        if (cap.sandbox) sandbox.push(cap.sandbox);
      }
      // Anything not given is DENIED explicitly: on the same-origin fallback the
      // device features otherwise default to 'self', so omission would not deny.
      if (cap.allow) allow.push(given ? cap.allow : cap.allow + " 'none'");
    }
    return {
      mode: trustful ? 'trustful' : 'trustless',
      sandbox: sandbox.join(' '),
      allow: allow.join('; '),
      granted: granted,
      boundary: boundary,
      ownOrigin: ownOrigin
    };
  }

  // The person's narrowing, in this browser (visitor and hosted session):
  //   localStorage['elohim-grants:<contentAddress>'] = { "<capability>": true|false }
  // Keyed by content address so a new bundle starts from its record's
  // declaration again. The earlier recordId key is migrated once, then dropped.
  function grantsKeyFor(content, recordId) {
    var addr = content && typeof content.contentAddress === 'string' ? content.contentAddress : '';
    return 'elohim-grants:' + (addr || recordId);
  }

  function readGrants(content, recordId) {
    var out = {};
    try {
      var store = window.localStorage;
      var key = grantsKeyFor(content, recordId);
      var raw = store && store.getItem(key);
      var legacy = 'elohim-grants:' + recordId;
      if (!raw && store && key !== legacy) {
        raw = store.getItem(legacy);
        if (raw) {
          store.setItem(key, raw);
          store.removeItem(legacy);
        }
      }
      var parsed = raw ? JSON.parse(raw) : null;
      if (parsed && typeof parsed === 'object') {
        for (var i = 0; i < CAPABILITIES.length; i++) {
          var k = CAPABILITIES[i].key;
          if (typeof parsed[k] === 'boolean') out[k] = parsed[k];
        }
      }
    } catch (e) {
      /* storage may be unavailable; no narrowing is read. */
    }
    return out;
  }

  function writeGrant(content, recordId, key, value) {
    try {
      var grants = readGrants(content, recordId);
      grants[key] = value;
      if (!window.localStorage) return false;
      window.localStorage.setItem(grantsKeyFor(content, recordId), JSON.stringify(grants));
      return true;
    } catch (e) {
      return false;
    }
  }

  // Is the chrome itself inside another page's frame? Any failure to tell is
  // treated as framed. A framed chrome can be driven by clicks it did not
  // invite, so it only reads grants; it does not change them.
  function isFramed() {
    try {
      return window.top !== window;
    } catch (e) {
      return true;
    }
  }

  // PURE-ish. Flip one capability and confirm the choice was kept: the stored
  // state is read back, and only a change that persisted counts. `granted` is
  // the state after the attempt, so a caller never shows an unkept choice.
  function changeGrant(content, recordId, key) {
    var cap = null;
    for (var i = 0; i < CAPABILITIES.length; i++) if (CAPABILITIES[i].key === key) cap = CAPABILITIES[i];
    if (!cap) return { ok: false, granted: false };
    var before = capGranted(cap, readGrants(content, recordId));
    // Second guard beside the disabled switches: no write while framed.
    if (isFramed()) return { ok: false, granted: before, framed: true, name: cap.name };
    var wrote = writeGrant(content, recordId, key, !before);
    var after = capGranted(cap, readGrants(content, recordId));
    return { ok: wrote && after === !before, granted: after, name: cap.name };
  }

  // The island's `content` object, or null on a non-content shell.
  function contentOf(island) {
    var c = island && island['content'];
    return c && typeof c === 'object' ? c : null;
  }

  var FRAME_CONTAINER_ID = 'elohim-content-frame';
  var FRAME_TESTID = 'content-app-frame';

  // Build the content iframe: sandbox/allow are set BEFORE src so the very
  // first navigation already carries them.
  function buildFrame(content, container, recordId) {
    var f = content && content.frame && typeof content.frame === 'object' ? content.frame : null;
    var src = safeHref(
      f && typeof f.src === 'string' ? f.src : container.getAttribute('data-frame-src'),
      null
    );
    if (!src) return null;
    var title =
      (f && typeof f.title === 'string' && f.title) ||
      container.getAttribute('data-frame-title') ||
      'Content';
    var attrs = frameAttributes(content, readGrants(content, recordId));
    var ifr = document.createElement('iframe');
    ifr.setAttribute('data-testid', FRAME_TESTID);
    ifr.setAttribute('title', title);
    ifr.setAttribute('sandbox', attrs.sandbox);
    if (attrs.allow) ifr.setAttribute('allow', attrs.allow);
    ifr.setAttribute('data-frame-mode', attrs.mode);
    ifr.style.cssText = 'display:block;width:100%;height:100%;border:0;';
    ifr.setAttribute('src', src);
    return ifr;
  }

  function mountFrame(content, recordId) {
    try {
      var container = document.getElementById(FRAME_CONTAINER_ID);
      if (!container || !content || !content.frame) return;
      if (container.querySelector('iframe[data-testid="' + FRAME_TESTID + '"]')) return;
      var ifr = buildFrame(content, container, recordId);
      if (ifr) container.appendChild(ifr);
    } catch (e) {
      /* the frame is best-effort; the chrome stays. */
    }
  }

  // A permission changed: the frame's attributes only apply at navigation, so
  // the frame is replaced (reloaded) with the new computation.
  function reloadFrame() {
    try {
      var container = document.getElementById(FRAME_CONTAINER_ID);
      if (!container || !MODEL.content) return;
      var old = container.querySelector('iframe[data-testid="' + FRAME_TESTID + '"]');
      var next = buildFrame(MODEL.content, container, MODEL.recordId);
      if (!next) return;
      if (old && old.parentNode) old.parentNode.replaceChild(next, old);
      else container.appendChild(next);
    } catch (e) {
      /* best-effort */
    }
  }

  // ── Page info (the trust mark's popover) ────────────────────────────────────
  // Chrome's page-info idiom: a list of one-line ROWS; a row opens a CARD with
  // a back arrow; every card ends in a link to the record's EPR page. State is
  // one module-level model; the DOM is re-rendered from it (textContent only).
  var MODEL = {
    ctx: null,
    content: null,
    recordId: '',
    resilience: null,
    nav: null,
    raw: null,
    claimsCount: null,
    facing: null,
    facingAt: 0,
    facingInflight: false,
    facingFailed: false,
    holdersLens: null,
    themeGiven: false,
    notice: '',
    view: 'list',
    lastRow: null
  };
  var PI_ID = 'elohim-page-info';
  var ROW_ICONS = {
    source: '◆', delivery: '⇄', holders: '⌂', steward: '⚖', claims: '◈', permissions: '⚙', yours: '◐'
  };
  var ROW_META = {};
  (function () {
    var secs = (VOCAB && VOCAB.pageInfoSections) || [];
    for (var s = 0; s < secs.length; s++) {
      ROW_META[secs[s].value] = { icon: ROW_ICONS[secs[s].value] || '·', label: secs[s].label };
    }
    // Unspliced source: every section still has a label (its id).
    for (var id in ROW_ICONS) {
      if (Object.prototype.hasOwnProperty.call(ROW_ICONS, id) && !ROW_META[id]) {
        ROW_META[id] = { icon: ROW_ICONS[id], label: id.charAt(0).toUpperCase() + id.slice(1) };
      }
    }
  })();

  function el(tag, cls, text) {
    var e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text != null) e.textContent = text;
    return e;
  }

  function pageInfoEl() {
    return document.getElementById(PI_ID);
  }

  // Who is looking: viewer.kind when the island carries it, else the
  // deprecated boolean.
  function isSignedIn(c) {
    var v = c && c.viewer;
    if (v && typeof v === 'object') return v.kind === 'device-agent' || v.kind === 'hosted-session';
    return !!(c && c.authenticated);
  }

  // ── Session check (web2 sign-in keeps its token in this origin's storage) ──
  // The shell navigation carries no credential, so the island honestly says
  // visitor. After mount, once per page load, a stored token is checked against
  // the `me` endpoint the doorway advertises; a 200 makes the viewer a hosted
  // session for THIS UI only. The island is not mutated, the token goes only to
  // the same-origin endpoint, and any other answer leaves the viewer a visitor
  // (and the token alone: clearing it is the app's job).
  var AUTH_TOKEN_KEY = 'elohim-auth-token';

  function shouldCheckSession(c) {
    return !!(c && c.viewer && c.viewer.kind === 'visitor' && c.medium === 'web2-doorway');
  }

  function readAuthToken() {
    try {
      var v = window.localStorage && window.localStorage.getItem(AUTH_TOKEN_KEY);
      return typeof v === 'string' && v ? v : '';
    } catch (e) {
      return '';
    }
  }

  function checkSession() {
    if (MODEL.sessionChecked) return Promise.resolve(false);
    MODEL.sessionChecked = true;
    if (!shouldCheckSession(MODEL.ctx) || typeof window.fetch !== 'function') return Promise.resolve(false);
    var token = readAuthToken();
    if (!token) return Promise.resolve(false);
    return window
      .fetch('/.well-known/elohim-auth', { headers: { accept: 'application/json' }, credentials: 'same-origin' })
      .then(function (r) {
        return r && r.ok ? r.json() : null;
      })
      .then(function (doc) {
        var me = doc && doc.endpoints && typeof doc.endpoints.me === 'string' ? doc.endpoints.me : '';
        // Origin-relative only: the token never leaves this origin.
        var href = me && me.charAt(0) === '/' ? safeHref(me, null) : null;
        if (!href) return false;
        return window
          .fetch(href, {
            headers: { accept: 'application/json', authorization: 'Bearer ' + token },
            credentials: 'omit'
          })
          .then(function (resp) {
            if (!resp || resp.status !== 200) return false;
            MODEL.sessionViewer = true;
            try {
              var acct = document.querySelector('[data-omni-account]');
              if (acct) acct.removeAttribute('hidden');
            } catch (e) {
              /* the row still appears */
            }
            refreshPageInfo();
            return true;
          });
      })
      .catch(function () {
        return false;
      });
  }

  function pageInfoRows() {
    var rows = ['source'];
    if (hopsOf(deliveryOf()).length) rows.push('delivery');
    rows.push('holders', 'steward', 'claims');
    var c = MODEL.content;
    if (c && (c.frame || c.format === 'html5-app')) rows.push('permissions');
    if (isSignedIn(MODEL.ctx) || MODEL.sessionViewer) rows.push('yours');
    return rows;
  }

  // PURE. The holders reading as a sentence: the fold's own headline verbatim
  // when it came with one; the unknown-state sentence only when there is no
  // reading at all; otherwise the plain count.
  function holdersLine(h) {
    if (h && typeof h === 'object' && typeof h.holders === 'number') {
      if (typeof h.headline === 'string' && h.headline) return h.headline;
      if (h.holders >= 1) {
        var m = typeof h.target === 'number' ? h.target : h.holders;
        return 'Held by ' + h.holders + ' of ' + m + ' households';
      }
      return 'Not yet held by any household';
    }
    return 'Held by households · see where it lives';
  }

  // PURE. "5 minutes ago" for an ISO time, or '' when unreadable.
  function relativeTime(iso, nowMs) {
    var then = Date.parse(iso);
    if (isNaN(then)) return '';
    var s = Math.max(0, Math.round((nowMs - then) / 1000));
    if (s < 45) return 'just now';
    var m = Math.round(s / 60);
    if (m < 90) return m + (m === 1 ? ' minute ago' : ' minutes ago');
    var h = Math.round(m / 60);
    if (h < 36) return h + (h === 1 ? ' hour ago' : ' hours ago');
    var d = Math.round(h / 24);
    return d + (d === 1 ? ' day ago' : ' days ago');
  }

  function witnessSentence(w) {
    if (w === 'amber') {
      return "Shown as soon as its bytes matched its address; awaiting the network's witness.";
    }
    if (w === 'green') return 'Witnessed by the network.';
    return '';
  }

  // Facts about THIS delivery, hop by hop, each reported by a named party.
  // Absent fields stay absent; nothing is guessed.
  function deliveryOf() {
    var d = MODEL.content && MODEL.content.delivery;
    return d && typeof d === 'object' ? d : null;
  }
  function str(v) {
    return typeof v === 'string' && v ? v : '';
  }
  function hopsOf(d) {
    var path = d && Array.isArray(d.path) ? d.path : [];
    return path.filter(function (h) {
      return h && typeof h === 'object' && typeof h.role === 'string';
    });
  }
  function hopOfRole(d, role) {
    var hops = hopsOf(d);
    for (var i = 0; i < hops.length; i++) if (hops[i].role === role) return hops[i];
    return null;
  }
  // A peer is named by its key; a human label would come only from the person's
  // own roster, never from a server's configuration (no roster is wired yet).
  function peerWord(key) {
    return str(key) ? 'peer ' + truncateAddress(key) : '';
  }
  // PURE. One hop as a sentence.
  function hopLine(h) {
    if (h.role === 'self') return 'From your own peer';
    if (h.role === 'holder') {
      return 'From ' + (peerWord(h.peer) || 'a holding peer') + (str(h.transport) ? ' over ' + str(h.transport) : '');
    }
    if (h.role === 'doorway') {
      return 'Through ' + (str(h.name) || 'a doorway') + (str(h.id) ? ' (doorway ' + str(h.id) + ')' : '');
    }
    return '';
  }
  // PURE. The one-line reading for the Delivery row.
  function deliveryReading(d) {
    var self = hopOfRole(d, 'self');
    var door = hopOfRole(d, 'doorway');
    var holder = hopOfRole(d, 'holder');
    if (!self && !door && !holder) return 'How this reached you';
    var line = self ? 'From your own peer' : door ? 'Reached you through ' + (str(door.name) || 'a doorway') : '';
    if (holder && !self) line += (line ? ' from ' : 'From ') + (peerWord(holder.peer) || 'a holding peer');
    return line;
  }

  function governanceRef() {
    var c = MODEL.raw && MODEL.raw.coupling && typeof MODEL.raw.coupling === 'object'
      ? MODEL.raw.coupling
      : null;
    return c && c.governance ? c.governance : null;
  }

  function declaredCaps() {
    var c = MODEL.content;
    return c && Array.isArray(c.capabilities) ? c.capabilities : [];
  }

  // PURE. Says only what is known: with no content object it names the record
  // and claims neither verification nor witness.
  function sourceReading(content, recordId) {
    if (!content || !content.contentAddress) return 'Record · ' + truncateAddress(recordId);
    var w =
      content.witness === 'green'
        ? 'Witnessed'
        : content.witness === 'amber'
          ? 'Awaiting witness'
          : 'Verified bytes';
    return w + ' · ' + truncateAddress(content.contentAddress);
  }

  // The holders reading to show: the facing's lens when it has one, else the
  // island's.
  function holdersReading() {
    if (MODEL.holdersLens) return MODEL.holdersLens;
    return MODEL.content ? MODEL.content.holders : null;
  }

  function stewardsOf() {
    var s = MODEL.facing && MODEL.facing.steward;
    return s && Array.isArray(s.stewards) ? s.stewards : null;
  }

  function rowReading(row) {
    var c = MODEL.content;
    if (row === 'source') return sourceReading(c, MODEL.recordId);
    if (row === 'delivery') return deliveryReading(deliveryOf());
    if (row === 'holders') return holdersLine(holdersReading());
    if (row === 'steward') {
      var reach = c && typeof c.reach === 'string' && c.reach ? c.reach : '';
      var ss = stewardsOf();
      var parts = [];
      if (reach) parts.push('Reach: ' + reach);
      if (ss) parts.push(ss.length ? ss.length + (ss.length === 1 ? ' steward' : ' stewards') : 'no stewards recorded');
      if (!parts.length) return 'Who stewards this is not yet known';
      return parts.join(' · ');
    }
    if (row === 'claims') {
      var n = MODEL.claimsCount;
      if (n == null) return MODEL.facing ? 'Claims not yet known' : 'The claims that back this record';
      if (n === 0) return 'No claims recorded yet';
      return n + (n === 1 ? ' claim backs' : ' claims back') + ' this record';
    }
    if (row === 'permissions') {
      var d = declaredCaps().length;
      if (!d) return 'Asks for nothing';
      if (!c.frame) return 'Declares ' + d;
      var fa = frameAttributes(c, readGrants(c, MODEL.recordId));
      return 'Asks for ' + d + ' · ' + fa.granted.length + ' allowed';
    }
    return 'Your relation to this content';
  }

  // PURE. Relative, so it opens on the same doorway; carries the delivery when
  // known. A value that cannot be encoded is omitted; an unencodable record id
  // degrades to the site root rather than throwing.
  function buildEprHref(recordId, delivery, row) {
    // NOTE: ?via=&peer= are the hops the EPR home currently parses; the path
    // itself is not yet a query shape.
    var q = [];
    var door = hopOfRole(delivery, 'doorway');
    var holder = hopOfRole(delivery, 'holder');
    var via = door && str(door.id) ? safeEncode(str(door.id)) : null;
    var peer = holder && str(holder.peer) ? safeEncode(str(holder.peer)) : null;
    if (via) q.push('via=' + via);
    if (peer) q.push('peer=' + peer);
    var rid = safeEncode(recordId);
    if (rid === null) return '/';
    return safeHref('/epr/' + rid + (q.length ? '?' + q.join('&') : '') + '#' + row, '/');
  }
  function eprPageHref(row) {
    return buildEprHref(MODEL.recordId, deliveryOf(), row);
  }

  function fact(body, label, value, mono) {
    var d = el('div', 'omni-pi-fact');
    d.appendChild(el('span', 'omni-pi-fact-label', label));
    d.appendChild(el('span', 'omni-pi-fact-value' + (mono ? ' omni-pi-mono' : ''), value));
    body.appendChild(d);
  }

  function cardSource(body) {
    var c = MODEL.content;
    fact(body, 'Record', MODEL.recordId, true);
    if (c) {
      if (c.contentAddress) fact(body, 'Content address', c.contentAddress, true);
      if (c.format) fact(body, 'Format', c.format);
      if (c.entry) fact(body, 'Entry', c.entry, true);
    }
    if (MODEL.ctx && MODEL.ctx.buildMarker) fact(body, 'Build', MODEL.ctx.buildMarker, true);
    var sentence = witnessSentence(c && c.witness);
    if (sentence) {
      var p = el('p', 'omni-pi-witness', sentence);
      p.setAttribute('data-witness', c.witness);
      body.appendChild(p);
    }
    var receipt = MODEL.ctx && MODEL.ctx.receipt;
    var rh = receipt && receipt.href ? safeHref(receipt.href, null) : null;
    if (rh) {
      var p2 = el('p', 'omni-pi-muted');
      var a = el('a', null, (receipt && receipt.label) || 'What this visit traded');
      a.setAttribute('href', rh);
      p2.appendChild(a);
      body.appendChild(p2);
    }
  }

  function cardDelivery(body) {
    var d = deliveryOf();
    var hops = hopsOf(d);
    if (!hops.length) return;
    for (var i = 0; i < hops.length; i++) {
      var h = hops[i];
      var line = hopLine(h);
      if (!line) continue;
      var p = el('p', 'omni-pi-hop', line);
      p.setAttribute('data-delivery-hop', h.role);
      body.appendChild(p);
      if (h.role === 'doorway' && str(h.did)) fact(body, 'Doorway DID', str(h.did), true);
      if (h.role === 'holder' && str(h.peer)) fact(body, 'Peer key', str(h.peer), true);
      // Hearsay is said plainly: a hop reported by someone other than itself.
      if (str(h.reportedBy) && h.reportedBy !== h.role) {
        body.appendChild(el('p', 'omni-pi-muted', 'Reported by the ' + str(h.reportedBy) + ', not by this party itself.'));
      }
    }
    var chk = d && d.checked && typeof d.checked === 'object' ? d.checked : null;
    if (chk && str(chk.by)) {
      var who = chk.by === 'worker' ? 'the worker' : chk.by === 'peer' ? 'the peer' : str(chk.by);
      body.appendChild(el('p', 'omni-pi-checked', 'Checked by ' + who + ' against its content address'));
    }
    var rh = d && str(d.receipt) ? safeHref(str(d.receipt), null) : null;
    if (rh) {
      var pr = el('p', 'omni-pi-muted');
      var a = el('a', null, 'What this visit traded');
      a.setAttribute('href', rh);
      pr.appendChild(a);
      body.appendChild(pr);
    }
  }

  function cardHolders(body) {
    var h = holdersReading();
    body.appendChild(el('p', 'omni-resilience-headline', holdersLine(h)));
    if (h && typeof h === 'object' && str(h.asOf)) {
      var ago = relativeTime(h.asOf, Date.now());
      if (ago) body.appendChild(el('p', 'omni-pi-muted', 'as of ' + ago));
    }
    if (MODEL.holdersLens) {
      var by = Array.isArray(MODEL.holdersLens.heldBy) ? MODEL.holdersLens.heldBy.length : 0;
      if (by) fact(body, 'Held by', by + (by === 1 ? ' collective' : ' collectives'));
      return;
    }
    if (MODEL.facing) return; // the facing answered without a holders reading: the line above says so
    var box = el('div', 'omni-pi-resilience');
    box.setAttribute('data-page-info-resilience', '');
    if (MODEL.resilience) {
      renderResilienceCard(box, MODEL.resilience.data, MODEL.resilience.status, MODEL.resilience.felt);
    } else {
      var st = (document.getElementById(OMNI_ID) || document.body).getAttribute('data-omni-resilience-loaded');
      box.appendChild(
        el('p', 'omni-pi-muted', st === 'unmatched'
          ? 'No live reading of the holders is available right now.'
          : 'Reading where this record lives…')
      );
    }
    body.appendChild(box);
  }

  function claimsState() {
    var omni = document.getElementById(OMNI_ID);
    return omni ? omni.getAttribute('data-omni-claims-loaded') : null;
  }

  function pctOf(r) {
    return typeof r === 'number' ? (r <= 1 ? Math.round(r * 100) + '%' : String(r)) : '';
  }

  function cardSteward(body) {
    var c = MODEL.content;
    fact(body, 'Reach', c && c.reach ? c.reach : 'not stated');
    var ss = stewardsOf();
    if (ss && ss.length) {
      for (var i = 0; i < ss.length; i++) {
        var s = ss[i];
        var name = str(s.displayName) || truncateAddress(str(s.presenceId));
        var bits = [str(s.contributionType), pctOf(s.allocationRatio), str(s.governanceState)].filter(Boolean);
        var d0 = el('div', 'omni-pi-fact');
        d0.setAttribute('data-steward', str(s.presenceId));
        d0.appendChild(el('span', 'omni-pi-fact-label', name));
        d0.appendChild(el('span', 'omni-pi-fact-value', bits.join(' · ')));
        body.appendChild(d0);
      }
    } else if (ss) {
      body.appendChild(el('p', 'omni-pi-muted', 'No stewards are recorded for this record yet.'));
    } else {
      body.appendChild(el('p', 'omni-pi-muted', MODEL.facing || MODEL.facingFailed
        ? 'Who stewards this record is not yet known.'
        : 'Reading who stewards this record…'));
    }
    var gov = governanceRef();
    var link = gov ? eprLinkEl(gov) : null;
    if (link) {
      var d = el('div', 'omni-pi-fact');
      d.setAttribute('data-omni-claim-leg', 'governance');
      d.appendChild(el('span', 'omni-pi-fact-label', 'Governance'));
      d.appendChild(link);
      body.appendChild(d);
    }
  }

  // PURE. A proof class in plain words.
  function proofClassWord(pc) {
    return (
      { witness: 'witnessed', 'self-attest': 'self-attested', 'audit-signature': 'audit signature', computational: 'computed' }[pc] ||
      str(pc) ||
      'not stated'
    );
  }

  function cardClaims(body) {
    var lens = MODEL.facing ? MODEL.facing.claims : null;
    var list = lens && Array.isArray(lens.attestations) ? lens.attestations : null;
    if (list && list.length) {
      var shown = Math.min(list.length, 20);
      for (var i = 0; i < shown; i++) {
        var a = list[i];
        var item = el('div', 'omni-pi-claim');
        item.setAttribute('data-claim-id', str(a.id));
        item.setAttribute('data-revoked', a.revoked ? 'true' : 'false');
        item.appendChild(el('p', 'omni-resilience-headline', str(a.title) || str(a.kind) || 'Claim'));
        if (str(a.issuerCid)) fact(item, 'Issuer', truncateAddress(str(a.issuerCid)), true);
        fact(item, 'Proof', proofClassWord(a.proofClass));
        if (str(a.createdAt)) fact(item, 'Made', str(a.createdAt).slice(0, 10));
        if (a.revoked) item.appendChild(el('p', 'omni-pi-revoked', 'Withdrawn: this claim was revoked.'));
        body.appendChild(item);
      }
      if (list.length > shown) body.appendChild(el('p', 'omni-pi-muted', 'and ' + (list.length - shown) + ' more'));
    } else if (list) {
      body.appendChild(el('p', 'omni-pi-muted', 'No claims are recorded for this record yet.'));
    } else if (MODEL.facing || MODEL.facingFailed) {
      body.appendChild(el('p', 'omni-pi-muted', 'What backs this record is not yet known.'));
    } else {
      body.appendChild(el('p', 'omni-pi-muted', 'Reading the claims…'));
    }
    var box = el('div', 'omni-pi-claims');
    if (renderClaimsCard(box, MODEL.nav, MODEL.raw)) body.appendChild(box);
  }

  function capState(cap, declared, granted, trustless) {
    if (trustless) return declared ? 'Declared — not given to this content.' : 'Not given.';
    if (!declared) return 'Not declared — its record does not declare this.';
    if (cap.device) {
      return granted ? 'Allowed — you granted this.' : 'Declared — waiting for your grant.';
    }
    return granted ? 'Allowed — its record declares this.' : 'Turned off by you.';
  }

  function cardPermissions(body) {
    var declaredList = declaredCaps();
    var framed = !!(MODEL.content && MODEL.content.frame);
    // One source of truth: what the frame is actually given.
    var fa = frameAttributes(MODEL.content, readGrants(MODEL.content, MODEL.recordId));
    var trustless = fa.mode !== 'trustful';
    if (MODEL.notice) {
      var note = el('p', 'omni-pi-notice', MODEL.notice);
      note.setAttribute('role', 'status');
      body.appendChild(note);
    }
    var pageFramed = isFramed();
    if (pageFramed) {
      var fp = el('p', 'omni-pi-muted', 'To change what this content may do, open it in its own tab. ');
      var fl = el('a', null, 'Open in its own tab');
      var here = safeHref(window.location && window.location.href, null);
      if (here) {
        fl.setAttribute('href', here);
        fl.setAttribute('target', '_blank');
        fl.setAttribute('rel', 'noopener');
        fl.setAttribute('data-page-info-open-top', '');
        fp.appendChild(fl);
      }
      body.appendChild(fp);
    }
    if (!framed) {
      body.appendChild(el('p', 'omni-pi-muted', 'This page does not frame the content. These are what its record declares.'));
    } else if (trustless) {
      body.appendChild(
        el('p', 'omni-pi-muted', 'Runs with nothing extra: this is not Elohim Protocol content.')
      );
    } else if (!fa.ownOrigin) {
      body.appendChild(
        el('p', 'omni-pi-muted', fa.boundary === 'opaque-sandbox'
          ? 'It runs in an opaque sandbox, so it keeps no storage of its own.'
          : "Its files are served from this page's own origin, so the chrome does not give it storage of its own.")
      );
    }
    for (var i = 0; i < CAPABILITIES.length; i++) {
      var cap = CAPABILITIES[i];
      var declared = declaredList.indexOf(cap.key) !== -1;
      if (cap.device && !declared) continue; // device capabilities only when declared
      var granted = framed && fa.granted.indexOf(cap.key) !== -1;
      var row = el('div', 'omni-pi-cap');
      row.setAttribute('data-page-info-cap', cap.key);
      row.setAttribute('data-cap-state', !declared || trustless ? 'not-declared' : granted ? 'allowed' : 'off');
      row.appendChild(el('span', 'omni-pi-cap-name', cap.name));
      if (declared && framed) {
        var sw = el('button', 'omni-pi-switch', granted ? 'On' : 'Off');
        sw.setAttribute('type', 'button');
        sw.setAttribute('role', 'switch');
        sw.setAttribute('aria-checked', granted ? 'true' : 'false');
        sw.setAttribute('aria-label', cap.name);
        sw.setAttribute('data-page-info-cap-toggle', cap.key);
        if (trustless || pageFramed) sw.setAttribute('disabled', '');
        row.appendChild(sw);
      }
      row.appendChild(el('span', 'omni-pi-cap-state', !framed
        ? (declared ? 'Declared by its record.' : 'Not declared.')
        : capState(cap, declared, granted, trustless)));
      body.appendChild(row);
    }
    if (framed && !pageFramed) {
      body.appendChild(
        el('p', 'omni-pi-muted', 'Your choices are kept in this browser. Changing one reloads the content.')
      );
    }
  }

  function cardYours(body) {
    var accountHref = safeHref((MODEL.ctx && MODEL.ctx.accountHref) || '/account', '/account');
    var p = el('p');
    var a = el('a', null, 'Your account');
    a.setAttribute('href', accountHref);
    p.appendChild(a);
    body.appendChild(p);
    body.appendChild(
      el('p', 'omni-pi-muted', 'Your bookmark, your progress, whether your household holds this, and your own claims are not shown here yet.')
    );
  }

  var CARD_BUILDERS = {
    source: cardSource,
    delivery: cardDelivery,
    holders: cardHolders,
    steward: cardSteward,
    claims: cardClaims,
    permissions: cardPermissions,
    yours: cardYours
  };

  function renderRows(pi) {
    var group = el('div', 'omni-pi-rows');
    group.setAttribute('role', 'group');
    group.setAttribute('aria-label', 'Page info');
    var rows = pageInfoRows();
    for (var i = 0; i < rows.length; i++) {
      var r = rows[i];
      var b = el('button', 'omni-pi-row');
      b.setAttribute('type', 'button');
      b.setAttribute('data-page-info-row', r);
      var icon = el('span', 'omni-pi-icon', ROW_META[r].icon);
      icon.setAttribute('aria-hidden', 'true');
      var text = el('span');
      text.appendChild(el('span', 'omni-pi-label', ROW_META[r].label));
      text.appendChild(el('span', 'omni-pi-reading', rowReading(r)));
      var chev = el('span', 'omni-pi-chevron', '›');
      chev.setAttribute('aria-hidden', 'true');
      b.appendChild(icon);
      b.appendChild(text);
      b.appendChild(chev);
      group.appendChild(b);
    }
    pi.appendChild(group);
  }

  function renderCard(pi, row) {
    var meta = ROW_META[row];
    var card = el('section', 'omni-pi-card');
    card.setAttribute('data-page-info-card', row);
    card.setAttribute('role', 'region');
    card.setAttribute('aria-label', meta.label);
    var head = el('div', 'omni-pi-head');
    var back = el('button', 'omni-pi-back', '←');
    back.setAttribute('type', 'button');
    back.setAttribute('data-page-info-back', '');
    back.setAttribute('aria-label', 'Back to page info');
    head.appendChild(back);
    head.appendChild(el('h2', 'omni-pi-title', meta.label));
    card.appendChild(head);
    var body = el('div', 'omni-pi-body');
    CARD_BUILDERS[row](body);
    card.appendChild(body);
    var foot = el('div', 'omni-pi-foot');
    var link = el('a', null, 'See this record on its EPR page →');
    link.setAttribute('href', eprPageHref(row));
    link.setAttribute('data-page-info-epr-link', row);
    foot.appendChild(link);
    card.appendChild(foot);
    pi.appendChild(card);
  }

  // Re-render from the model, keeping keyboard focus on the same control.
  function renderPageInfo() {
    var pi = pageInfoEl();
    if (!pi) return;
    var keep = null;
    var active = document.activeElement;
    if (active && pi.contains(active)) {
      keep =
        active.getAttribute('data-page-info-row') != null
          ? '[data-page-info-row="' + active.getAttribute('data-page-info-row') + '"]'
          : active.hasAttribute('data-page-info-back')
            ? '[data-page-info-back]'
            : active.hasAttribute('data-page-info-epr-link')
              ? '[data-page-info-epr-link]'
              : active.getAttribute('data-page-info-cap-toggle') != null
              ? '[data-page-info-cap-toggle="' + active.getAttribute('data-page-info-cap-toggle') + '"]'
              : null;
    }
    while (pi.firstChild) pi.removeChild(pi.firstChild);
    pi.setAttribute('data-page-info-view', MODEL.view);
    if (MODEL.view === 'list' || !CARD_BUILDERS[MODEL.view]) renderRows(pi);
    else renderCard(pi, MODEL.view);
    if (keep) {
      var again = pi.querySelector(keep);
      if (again && typeof again.focus === 'function') again.focus();
    }
  }

  // A projection answered while page info is open: refresh in place.
  function refreshPageInfo() {
    try {
      var pi = pageInfoEl();
      if (pi && pi.hasAttribute('data-page-info-open')) renderPageInfo();
    } catch (e) {
      /* best-effort */
    }
  }

  function trustMarkEl() {
    return document.querySelector('[data-omni-action="page-info"]');
  }

  function isPageInfoOpen() {
    var pi = pageInfoEl();
    return !!(pi && pi.hasAttribute('data-page-info-open'));
  }

  function positionPageInfo(pi, mark) {
    try {
      var r = mark.getBoundingClientRect();
      var w = pi.offsetWidth || 336;
      var rtl = window.getComputedStyle(document.documentElement).direction === 'rtl';
      var left = rtl ? r.right - w : r.left;
      var max = (window.innerWidth || 0) - w - 8;
      if (left > max) left = max;
      if (left < 8) left = 8;
      pi.style.top = Math.round(r.bottom + 6) + 'px';
      pi.style.left = Math.round(left) + 'px';
    } catch (e) {
      /* positioning is best-effort. */
    }
  }

  function openPageInfo(omni) {
    var pi = pageInfoEl();
    var mark = trustMarkEl();
    if (!pi || !mark) return;
    MODEL.view = 'list';
    renderPageInfo();
    // The projections are read lazily, on first open (same trigger as expand).
    loadFacing(omni);
    if (typeof pi.showPopover === 'function') {
      try {
        pi.showPopover();
      } catch (e) {
        pi.removeAttribute('hidden');
      }
    } else {
      pi.removeAttribute('hidden');
    }
    pi.setAttribute('data-page-info-open', '');
    mark.setAttribute('aria-expanded', 'true');
    positionPageInfo(pi, mark);
    var first = pi.querySelector('[data-page-info-row]');
    if (first) first.focus();
  }

  function closePageInfo(refocus) {
    var pi = pageInfoEl();
    var mark = trustMarkEl();
    if (!pi || !pi.hasAttribute('data-page-info-open')) return;
    pi.removeAttribute('data-page-info-open');
    if (typeof pi.hidePopover === 'function') {
      try {
        pi.hidePopover();
      } catch (e) {
        pi.setAttribute('hidden', '');
      }
    } else {
      pi.setAttribute('hidden', '');
    }
    MODEL.view = 'list';
    if (mark) {
      mark.setAttribute('aria-expanded', 'false');
      if (refocus && typeof mark.focus === 'function') mark.focus();
    }
  }

  function togglePageInfo() {
    var omni = document.getElementById(OMNI_ID);
    if (!omni) return;
    if (isPageInfoOpen()) closePageInfo(true);
    else openPageInfo(omni);
  }

  function openCard(row) {
    MODEL.notice = '';
    MODEL.view = row;
    MODEL.lastRow = row;
    renderPageInfo();
    var pi = pageInfoEl();
    var back = pi && pi.querySelector('[data-page-info-back]');
    if (back) back.focus();
    // The Permissions/Steward/Claims cards read lazily; make sure they have begun.
    var omni = document.getElementById(OMNI_ID);
    if (omni) loadFacing(omni);
  }

  function backToList() {
    var row = MODEL.lastRow;
    MODEL.view = 'list';
    renderPageInfo();
    var pi = pageInfoEl();
    var target = (row && pi && pi.querySelector('[data-page-info-row="' + row + '"]')) ||
      (pi && pi.querySelector('[data-page-info-row]'));
    if (target) target.focus();
  }

  function toggleCapability(key) {
    if (declaredCaps().indexOf(key) === -1) return;
    if (!MODEL.content || !MODEL.content.frame || frameAttributes(MODEL.content, {}).mode !== 'trustful') return;
    var r = changeGrant(MODEL.content, MODEL.recordId, key);
    if (!r.ok) {
      // The choice was not kept: the card keeps the state that still holds.
      MODEL.notice = r.framed
        ? 'To change what this content may do, open it in its own tab.'
        : "That choice couldn't be kept in this browser, so nothing changed.";
      renderPageInfo();
      announce(MODEL.notice);
      return;
    }
    MODEL.notice = '';
    renderPageInfo();
    reloadFrame();
    announce(r.name + (r.granted ? ' turned on. The content is reloading.' : ' turned off. The content is reloading.'));
  }

  function onPageInfoKey(evt) {
    if (!isPageInfoOpen()) return;
    if (evt.key === 'Escape') {
      evt.preventDefault();
      closePageInfo(true);
      return;
    }
    if (MODEL.view !== 'list') return;
    var k = evt.key;
    if (k !== 'ArrowDown' && k !== 'ArrowUp' && k !== 'Home' && k !== 'End') return;
    var pi = pageInfoEl();
    if (!pi || !pi.contains(document.activeElement)) return;
    var rows = Array.prototype.slice.call(pi.querySelectorAll('[data-page-info-row]'));
    if (!rows.length) return;
    var idx = rows.indexOf(document.activeElement);
    var next = k === 'Home' ? 0 : k === 'End' ? rows.length - 1 : k === 'ArrowDown'
      ? (idx + 1) % rows.length
      : (idx <= 0 ? rows.length - 1 : idx - 1);
    evt.preventDefault();
    rows[next].focus();
  }

  // Outside pointer (anywhere not in the popover or on the trust mark) closes.
  function onOutsidePointer(evt) {
    if (!isPageInfoOpen()) return;
    var t = evt.target;
    if (t && typeof t.closest === 'function' && (t.closest('#' + PI_ID) || t.closest('[data-omni-action="page-info"]'))) {
      return;
    }
    closePageInfo(false);
  }

  // A click inside the content frame never reaches this document; focus
  // moving into the frame is the signal instead.
  function onWindowBlur() {
    window.setTimeout(guard(function () {
      var a = document.activeElement;
      if (a && a.tagName === 'IFRAME') closePageInfo(false);
    }), 0);
  }

  // The witness (amber|green) painted on the collapsed chip and the trust mark.
  function applyWitness(omni, content) {
    var w = content && (content.witness === 'green' || content.witness === 'amber') ? content.witness : '';
    if (!w) return;
    var chip = omni.querySelector('.omni-chip');
    if (chip && !chip.querySelector('.omni-chip-witness')) {
      var dot = document.createElement('span');
      dot.className = 'omni-chip-witness';
      dot.setAttribute('aria-hidden', 'true');
      chip.appendChild(dot);
    }
    var marks = omni.querySelectorAll('.omni-chip, .omni-chip-witness, [data-omni-action="page-info"]');
    for (var i = 0; i < marks.length; i++) marks[i].setAttribute('data-witness', w);
  }

  // PURE. green iff the record's anchor is a non-blank string (the one
  // witness rule), else amber.
  function witnessFromAnchor(anchor) {
    return typeof anchor === 'string' && anchor.trim() !== '' ? 'green' : 'amber';
  }

  var KNOWN_CAPS = function () {
    return CAPABILITIES.map(function (c) {
      return c.key;
    });
  };

  // PURE. The content a page-info needs, built from a fetched record when the
  // island carried none. No frame, no isolation, no delivery: it states only
  // what the record says, never what a delivery did.
  function contentFromRecord(node) {
    if (!node || typeof node !== 'object') return null;
    var addr = str(node.blobHash) || str(node.serverBlobHash) || str(node.blobCid);
    if (!addr || !str(node.id)) return null;
    var fmt = str(node.contentFormat);
    var body = null;
    if (typeof node.contentBody === 'string' && node.contentBody) {
      try {
        body = JSON.parse(node.contentBody);
      } catch (e) {
        body = null;
      }
    }
    var known = KNOWN_CAPS();
    var caps =
      fmt === 'html5-app' && body && Array.isArray(body.capabilities)
        ? body.capabilities.filter(function (k) {
            return known.indexOf(k) !== -1;
          })
        : [];
    var out = {
      recordId: str(node.id),
      contentAddress: addr,
      format: fmt,
      mode: 'trustful',
      capabilities: caps,
      holders: null
    };
    if (fmt === 'html5-app') out.entry = (body && str(body.entryPoint)) || 'index.html';
    if (str(node.reach)) out.reach = str(node.reach);
    if ('dhtAnchorHash' in node) out.witness = witnessFromAnchor(node.dhtAnchorHash);
    return out;
  }

  // ── The content-chrome facing ───────────────────────────────────────────────
  // GET {readingsBase}/api/v1/epr/{recordId}/chrome folds the witness, holders,
  // claims, steward and coupling legs in one read. A lens the facing did not
  // compute is ABSENT (never null, never zero), and absent means "not yet
  // known", not "none". One fetch serves the first open (and the bar's
  // expand); a reopen reuses it for FACING_TTL_MS, matching the route's cache.
  // A facing that cannot be read falls back to the separate record, resilience
  // and claims reads, once.
  var FACING_TTL_MS = 30000;

  function loadFacing(omni) {
    if (!omni || MODEL.facingInflight) return;
    if (MODEL.facingFailed) {
      legacyLoads(omni);
      return;
    }
    if (MODEL.facing && Date.now() - MODEL.facingAt < FACING_TTL_MS) return;
    var enc = safeEncode(MODEL.recordId);
    if (enc === null || typeof window.fetch !== 'function') {
      MODEL.facingFailed = true;
      legacyLoads(omni);
      return;
    }
    MODEL.facingInflight = true;
    var first = !MODEL.facing;
    if (first) {
      if (!omni.getAttribute('data-omni-resilience-loaded')) omni.setAttribute('data-omni-resilience-loaded', 'loading');
      if (!omni.getAttribute('data-omni-claims-loaded')) omni.setAttribute('data-omni-claims-loaded', 'loading');
    }
    maybeLoadNav(omni);
    tapFetch(readingUrl(MODEL.ctx, '/api/v1/epr/' + enc + '/chrome'), {
        headers: { accept: 'application/json' },
        credentials: 'same-origin'
      })
      .then(function (r) {
        return r && r.ok ? r.json() : null;
      })
      .then(function (view) {
        MODEL.facingInflight = false;
        if (view && typeof view === 'object' && typeof view.recordId === 'string') {
          applyFacing(omni, view);
          return;
        }
        facingUnavailable(omni);
      })
      .catch(function () {
        MODEL.facingInflight = false;
        facingUnavailable(omni);
      });
  }

  function facingUnavailable(omni) {
    if (MODEL.facing) return; // an earlier reading stands
    MODEL.facingFailed = true;
    if (omni.getAttribute('data-omni-resilience-loaded') === 'loading') omni.removeAttribute('data-omni-resilience-loaded');
    if (omni.getAttribute('data-omni-claims-loaded') === 'loading') omni.removeAttribute('data-omni-claims-loaded');
    legacyLoads(omni);
  }

  function legacyLoads(omni) {
    maybeLoadResilience(omni);
    maybeLoadClaims(omni);
    if (!MODEL.legacyRecordRead) {
      MODEL.legacyRecordRead = true;
      readRecord(omni);
    }
  }

  // Back/forward neighbours are not part of the facing; read nav-context alone.
  function maybeLoadNav(omni) {
    if (MODEL.navRequested || typeof window.fetch !== 'function') return;
    MODEL.navRequested = true;
    var enc = safeEncode(MODEL.recordId);
    if (enc === null) return;
    tapFetch(readingUrl(MODEL.ctx, '/api/v1/epr/' + enc + '/nav-context'), {
        headers: { accept: 'application/json' },
        credentials: 'same-origin'
      })
      .then(function (r) {
        return r && r.ok ? r.json() : null;
      })
      .then(function (nav) {
        if (!nav || typeof nav !== 'object') return;
        MODEL.nav = nav;
        fillNavSlot(omni, 'back', nav.prev);
        fillNavSlot(omni, 'forward', nav.next);
        refreshPageInfo();
      })
      .catch(function () {
        /* best-effort */
      });
  }

  // PURE. Content from a facing, for a page whose island carried none.
  function contentFromFacing(view) {
    if (!view || typeof view !== 'object') return null;
    var addr = str(view.contentAddress);
    if (!addr || !str(view.recordId)) return null;
    var known = KNOWN_CAPS();
    var app = view.app && typeof view.app === 'object' ? view.app : null;
    var out = {
      recordId: str(view.recordId),
      contentAddress: addr,
      format: str(view.format),
      mode: view.mode === 'trustful' ? 'trustful' : 'trustless',
      capabilities:
        app && Array.isArray(app.capabilities)
          ? app.capabilities.filter(function (k) {
              return known.indexOf(k) !== -1;
            })
          : [],
      holders: null
    };
    if (app && str(app.entry)) out.entry = str(app.entry);
    if (str(view.reach)) out.reach = str(view.reach);
    if (view.witness === 'green' || view.witness === 'amber') out.witness = view.witness;
    return out;
  }

  function applyFacing(omni, view) {
    try {
      MODEL.facing = view;
      MODEL.facingAt = Date.now();
      // Witness: re-read on every facing; the island's reading is replaced.
      if (MODEL.content) {
        if (view.witness === 'green' || view.witness === 'amber') MODEL.content.witness = view.witness;
      } else {
        var derived = contentFromFacing(view);
        if (derived) MODEL.content = derived;
      }
      // Holders: ONE snapshot feeds the dial and the Holders row/card.
      var lens = view.holders && typeof view.holders === 'object' ? view.holders : null;
      MODEL.holdersLens = lens;
      if (lens) {
        paintDial(omni, dialStateFromHolders(lens), str(lens.headline));
        omni.setAttribute('data-omni-resilience-loaded', 'applied');
      } else {
        omni.setAttribute('data-omni-resilience-loaded', 'unmatched');
      }
      // Claims: attestations (+ coupling legs). Absent lens = not yet known.
      var cl = view.claims && typeof view.claims === 'object' && Array.isArray(view.claims.attestations)
        ? view.claims.attestations
        : null;
      MODEL.claimsCount = cl ? cl.length : null;
      MODEL.raw = view.legs && typeof view.legs === 'object' ? { coupling: view.legs } : null;
      var legCount = renderClaimsCard(document.createElement('div'), null, MODEL.raw);
      var total = (cl ? cl.length : 0) + legCount;
      omni.setAttribute('data-omni-claims-count', String(total));
      omni.setAttribute('data-omni-claims-loaded', total ? 'applied' : 'unmatched');
      // Theme: the record's own hint, only when the island gave none.
      if (!MODEL.themeGiven && view.app && view.app.themeHint) {
        applyStyle(normalizeThemeHint(view.app.themeHint));
      }
      applyWitness(omni, MODEL.content);
      refreshPageInfo();
    } catch (e) {
      /* never let a projection-shape surprise break the page. */
    }
  }

  // Re-read the record when page info opens: recompute the witness from its
  // anchor (amber must not stick), and build the content when no island gave one.
  function readRecord(omni) {
    if (!MODEL.recordId || typeof window.fetch !== 'function') return;
    var enc = safeEncode(MODEL.recordId);
    if (enc === null) return;
    var opts = { headers: { accept: 'application/json' }, credentials: 'same-origin' };
    tapFetch(readingUrl(MODEL.ctx, '/db/content/' + enc), opts)
      .then(function (r) {
        return r && r.ok ? r.json() : null;
      })
      .then(function (node) {
        if (!node || typeof node !== 'object') return;
        if (MODEL.content) {
          if ('dhtAnchorHash' in node) MODEL.content.witness = witnessFromAnchor(node.dhtAnchorHash);
        } else {
          var derived = contentFromRecord(node);
          if (!derived) return;
          MODEL.content = derived;
        }
        applyWitness(omni, MODEL.content);
        refreshPageInfo();
      })
      .catch(function () {
        /* best-effort: the island's reading stands. */
      });
  }

  function normalizeThemeHint(t) {
    if (typeof t === 'string') return t === 'dark' || t === 'light' ? { colorScheme: t } : null;
    return t && typeof t === 'object' ? t : null;
  }

  // ── Event delegation ────────────────────────────────────────────────────────
  function onClick(evt) {
    var target = evt.target;
    if (!target || typeof target.closest !== 'function') return;
    // Page-info controls (rows, back, permission switches).
    var rowEl = target.closest('[data-page-info-row]');
    if (rowEl) {
      openCard(rowEl.getAttribute('data-page-info-row'));
      return;
    }
    if (target.closest('[data-page-info-back]')) {
      backToList();
      return;
    }
    var capEl = target.closest('[data-page-info-cap-toggle]');
    if (capEl) {
      toggleCapability(capEl.getAttribute('data-page-info-cap-toggle'));
      return;
    }
    var actionEl = target.closest('[data-omni-action]');
    if (!actionEl) return;
    var action = actionEl.getAttribute('data-omni-action');
    if (action === 'toggle') {
      toggleOmni(document.getElementById(OMNI_ID));
    } else if (action === 'copy') {
      copyValue(actionEl);
    } else if (action === 'theme-toggle') {
      toggleTheme();
    } else if (action === 'page-info') {
      togglePageInfo();
    }
  }

  // ── Context acquisition ─────────────────────────────────────────────────────
  // Prefer an inline JSON island injected by the runtime (the doorway path);
  // else fetch the current EPR's content node and read metadata.theme. The
  // landing `/` resolves to LANDING_SLUG.
  function readInlineContext() {
    try {
      var el = document.getElementById(CONTEXT_SCRIPT_ID);
      if (!el || !el.textContent) return null;
      var parsed = JSON.parse(el.textContent);
      return parsed && typeof parsed === 'object' ? parsed : null;
    } catch (e) {
      return null;
    }
  }

  // Resolve the EPR slug for the current location. Root path → the landing slug;
  // otherwise the last non-empty path segment (e.g. /resource/<cid> → <cid>,
  // /deliver/<slug> → <slug>). Best-effort; null if undeterminable.
  function slugFromLocation() {
    try {
      var path = (window.location && window.location.pathname) || '/';
      var trimmed = path.replace(/\/+$/, '');
      if (trimmed === '' || trimmed === '/') return LANDING_SLUG;
      var segs = trimmed.split('/').filter(function (s) {
        return s.length > 0;
      });
      if (!segs.length) return LANDING_SLUG;
      return decodeURIComponent(segs[segs.length - 1]);
    } catch (e) {
      return LANDING_SLUG;
    }
  }

  // Map a fetched content node (the /db/content/{slug} wire shape) to a context.
  function contextFromContentNode(slug, node) {
    var meta = (node && node.metadata) || {};
    var theme = meta.theme || null;
    var buildMarker = node && (node.blobHash || node.serverBlobHash) ? node.blobHash || node.serverBlobHash : '';
    return {
      slug: slug,
      title: (node && node.title) || '',
      theme: theme,
      buildMarker: buildMarker,
      // The element cannot safely infer env tier / auth client-side; the
      // inline-context path (doorway) supplies those. Sensible defaults here.
      envTier: '',
      showEnv: false,
      authenticated: false,
      accountHref: '/account',
      showThemeToggle: true,
      // Nav is NOT deferred to a producer: neither producer supplies it today,
      // and the mutual deferral (each side waiting for the other) is exactly
      // why the nav links never rendered. The element resolves nav ITSELF from
      // GET /api/v1/epr/{slug}/nav-context on first expand (`fillNavSlot`), for
      // BOTH acquisition paths. These nulls just mean "no island-supplied
      // target" — the nav slots stay empty until the projection answers.
      navBack: null,
      navForward: null
    };
  }

  // Fetch the EPR context. Returns a Promise<context|null>.
  function fetchContext(slug) {
    if (typeof window.fetch !== 'function') return Promise.resolve(null);
    var encSlug = safeEncode(slug);
    if (encSlug === null) return Promise.resolve(null);
    var url = '/db/content/' + encSlug;
    return window
      .fetch(url, { headers: { accept: 'application/json' }, credentials: 'same-origin' })
      .then(function (resp) {
        if (!resp || !resp.ok) return null;
        return resp.json();
      })
      .then(function (node) {
        if (!node) return null;
        return contextFromContentNode(slug, node);
      })
      .catch(function () {
        return null;
      });
  }

  // ── Mount ───────────────────────────────────────────────────────────────────
  function mount(ctx) {
    try {
      if (!ctx || !ctx.slug) return;
      if (document.getElementById(OMNI_ID)) return; // idempotent: already mounted.
      if (!document.body) return;

      var island = contentOf(ctx);
      var themeIn = ctx.theme != null ? ctx.theme : island && island['theme'] != null ? island['theme'] : null;
      MODEL.themeGiven = themeIn != null;
      applyStyle(normalizeThemeHint(themeIn));
      MODEL.ctx = ctx;
      MODEL.content = island;
      MODEL.recordId = (island && typeof island.recordId === 'string' && island.recordId) || ctx.slug;

      var container = document.createElement('div');
      container.id = OMNI_ID;
      container.setAttribute('data-omni-state', 'pill');
      container.setAttribute('data-omni-resilience-slug', ctx.slug);
      // The EPR address every lazy projection fetch is keyed on (resilience
      // keeps its own historical attribute; claims + nav read this one).
      container.setAttribute('data-omni-slug', ctx.slug);
      container.innerHTML = buildMarkup(ctx);
      document.body.appendChild(container);

      container.addEventListener('click', guard(onClick));

      var pi = pageInfoEl();
      // No native popover support: fall back to a plain hidden/shown element.
      if (pi && typeof pi.showPopover !== 'function') pi.setAttribute('hidden', '');
      applyWitness(container, island);
      document.addEventListener('keydown', guard(onPageInfoKey));
      document.addEventListener('pointerdown', guard(onOutsidePointer), true);
      window.addEventListener('blur', guard(onWindowBlur));
      window.addEventListener('resize', guard(function () {
        var open = pageInfoEl();
        var mark = trustMarkEl();
        if (open && mark && isPageInfoOpen()) positionPageInfo(open, mark);
      }));

      // The chrome is on the page; only now does the content frame begin.
      mountFrame(island, MODEL.recordId);
      // After paint: the session check never sits on the chip's path.
      window.setTimeout(guard(checkSession), 0);
    } catch (e) {
      /* mounting must never throw uncaught. */
    }
  }

  function init() {
    restoreTheme();

    var inline = readInlineContext();
    if (inline) {
      // Inline context may omit slug; fall back to the location-derived slug.
      if (!inline.slug) inline.slug = slugFromLocation();
      mount(inline);
      return;
    }

    var slug = slugFromLocation();
    if (!slug) return;
    fetchContext(slug).then(function (ctx) {
      if (ctx) mount(ctx);
    });
  }

  // Test hook: the pure computations, for the node tests (no DOM needed).
  try {
    window.__elohimOmni = {
      frameAttributes: frameAttributes,
      holdersLine: holdersLine,
      safeHref: safeHref,
      safeEncode: safeEncode,
      buildEprHref: buildEprHref,
      sourceReading: sourceReading,
      deliveryReading: deliveryReading,
      hopLine: hopLine,
      relativeTime: relativeTime,
      witnessFromAnchor: witnessFromAnchor,
      contentFromRecord: contentFromRecord,
      isSignedIn: isSignedIn,
      readingUrl: readingUrl,
      readGrants: readGrants,
      writeGrant: writeGrant,
      adoptServedBy: adoptServedBy,
      checkSession: checkSession,
      pageInfoRows: pageInfoRows,
      noteServedBy: noteServedBy,
      model: MODEL,
      changeGrant: changeGrant,
      isFramed: isFramed,
      contentFromFacing: contentFromFacing,
      dialStateFromHolders: dialStateFromHolders,
      proofClassWord: proofClassWord,
      legibleAccent: legibleAccent,
      parseColor: parseColor
    };
  } catch (e) {
    /* exposure is best-effort. */
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
