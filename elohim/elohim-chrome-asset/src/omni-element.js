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
    // Same-origin-relative path (but not a scheme-relative `//host`).
    if (s.charAt(0) === '/' && s.charAt(1) !== '/') return s;
    if (/^https?:/i.test(s)) return s;
    return fb;
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
    '#elohim-omni .omni-resilience { display: inline-flex; align-items: center; ' +
    'color: var(--omni-fg, rgba(20,22,30,0.96)); padding: 0 0.25rem; ' +
    'background: transparent; border: none; font: inherit; cursor: pointer; }\n' +
    '#elohim-omni .omni-resilience-neutral { font-size: 12px; line-height: 1; }\n' +
    // Drilldown card: top-chrome affordances fold DOWN, inline-start-aligned
    // (omnibar spec §11 tooltip-direction convention — never up off-screen).
    '#elohim-omni .omni-resilience-card { position: absolute; top: 100%; ' +
    'inset-inline-start: 0; margin-top: 0.4rem; min-width: 15rem; max-width: 20rem; ' +
    'padding: 0.6rem 0.75rem; text-align: start; ' +
    'background: var(--omni-bg, rgba(255,255,255,0.96)); ' +
    'color: var(--omni-fg, rgba(20,22,30,0.96)); ' +
    'border: 1px solid var(--omni-border, rgba(20,22,30,0.14)); border-radius: 8px; ' +
    'box-shadow: var(--omni-shadow, 0 2px 10px rgba(0,0,0,0.12)); ' +
    'backdrop-filter: blur(10px); }\n' +
    '#elohim-omni .omni-resilience-card p { margin: 0 0 0.35rem 0; }\n' +
    '#elohim-omni .omni-resilience-headline { font-weight: 700; }\n' +
    '#elohim-omni .omni-resilience-reassure { color: var(--omni-muted, rgba(20,22,30,0.55)); }\n' +
    '#elohim-omni .omni-resilience-facts { display: block; ' +
    'color: var(--omni-muted, rgba(20,22,30,0.55)); font-size: 11px; }\n' +
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
    return css;
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
    html +=
      '<button type="button" class="' +
      chipClass +
      '" data-omni-action="toggle" aria-label="Elohim Protocol — click for details">';
    html += '<span class="omni-glyph" aria-hidden="true">⯂</span>';
    html += '<span class="omni-label">elohim-protocol</span>';
    html += '</button>';
    html += '</div>';

    // === EXPANDED TOOLBAR ===
    html += '<div class="omni-toolbar" role="region" aria-label="Protocol context">';

    // Back nav (server-known target only; plain link, no client router).
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

    // Resilience indicator — neutral glyph; JS swaps it on first expand from
    // the live household snapshot, and click drills down into the card.
    html += '<span class="omni-resilience-wrap">';
    html +=
      '<button type="button" class="omni-resilience" ' +
      'data-omni-action="resilience-toggle" aria-expanded="false" ' +
      'aria-label="Resilience indicator">';
    html +=
      '<span class="omni-resilience-neutral" data-omni-resilience-glyph ' +
      'title="Resilience snapshot unavailable">◉</span>';
    html += '</button>';
    html += '<div class="omni-resilience-card" data-omni-resilience-card hidden></div>';
    html += '</span>';

    // Forward nav (server-known target only).
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

    // Account link — hidden + data-omni-account hook when unauthenticated.
    var accountHref = htmlEscape(safeHref(ctx.accountHref || '/account', '/account'));
    var hiddenAttr = ctx.authenticated ? '' : ' hidden';
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
    if (next === 'expanded') maybeLoadResilience(omni);
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
    var base = '/api/v1/resilience/' + encodeURIComponent(slug);
    var opts = { headers: { accept: 'application/json' }, credentials: 'same-origin' };
    window
      .fetch(base + '/household', opts)
      .then(function (resp) {
        if (resp && resp.ok) return resp.json();
        return window.fetch(base, opts).then(function (r2) {
          if (!r2 || !r2.ok) return null;
          return r2.json();
        });
      })
      .then(function (data) {
        if (!data) {
          omni.setAttribute('data-omni-resilience-loaded', 'unmatched');
          return;
        }
        applyResilience(omni, data);
      })
      .catch(function () {
        /* best-effort; leave the neutral glyph. */
        omni.setAttribute('data-omni-resilience-loaded', 'unmatched');
      });
  }

  /// Glyph ladder over the ResilienceSnapshotView contract:
  /// protectionStatus "protected" → ● · "partial" → ◐ · "at-risk" → ○.
  var RESILIENCE_GLYPHS = { protected: '●', partial: '◐', 'at-risk': '○' };

  function applyResilience(omni, data) {
    try {
      // Real wire contract (resilience-snapshot-view.schema.json):
      // protectionStatus + coverageShortfall (+ feltStatus on /household).
      var status =
        typeof data.protectionStatus === 'string' ? data.protectionStatus : null;
      var felt = data.feltStatus && typeof data.feltStatus === 'object' ? data.feltStatus : null;
      var headline = felt && typeof felt.headline === 'string' ? felt.headline : null;
      var glyph = status ? RESILIENCE_GLYPHS[status] || null : null;
      var label = headline || (status ? 'Resilience: ' + status : null);
      if (!glyph && !label) {
        omni.setAttribute('data-omni-resilience-loaded', 'unmatched');
        return;
      }
      omni.setAttribute('data-omni-resilience-loaded', 'applied');
      var marks = omni.querySelectorAll('[data-omni-resilience-glyph]');
      for (var i = 0; i < marks.length; i++) {
        if (glyph) marks[i].textContent = glyph;
        if (label) marks[i].setAttribute('title', label);
      }
      renderResilienceCard(omni, data, status, felt);
    } catch (e) {
      /* never let a projection-shape surprise break the page. */
    }
  }

  /// Fill the drilldown card from the snapshot. Text via textContent only —
  /// snapshot strings never touch innerHTML.
  function renderResilienceCard(omni, data, status, felt) {
    var card = omni.querySelector('[data-omni-resilience-card]');
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

  function toggleResilienceCard(actionEl) {
    var omni = document.getElementById(OMNI_ID);
    if (!omni) return;
    var card = omni.querySelector('[data-omni-resilience-card]');
    if (!card || !card.firstChild) return; // nothing loaded — keep neutral, no empty flyout
    var open = !card.hasAttribute('hidden');
    if (open) {
      card.setAttribute('hidden', '');
    } else {
      card.removeAttribute('hidden');
    }
    actionEl.setAttribute('aria-expanded', open ? 'false' : 'true');
  }

  // ── Event delegation ────────────────────────────────────────────────────────
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
    } else if (action === 'resilience-toggle') {
      toggleResilienceCard(actionEl);
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
      // The element cannot safely infer env tier / auth / nav client-side; the
      // inline-context path (doorway) supplies those. Sensible defaults here.
      envTier: '',
      showEnv: false,
      authenticated: false,
      accountHref: '/account',
      showThemeToggle: true,
      navBack: null,
      navForward: null
    };
  }

  // Fetch the EPR context. Returns a Promise<context|null>.
  function fetchContext(slug) {
    if (typeof window.fetch !== 'function') return Promise.resolve(null);
    var url = '/db/content/' + encodeURIComponent(slug);
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

      applyStyle(ctx.theme || null);

      var container = document.createElement('div');
      container.id = OMNI_ID;
      container.setAttribute('data-omni-state', 'pill');
      container.setAttribute('data-omni-resilience-slug', ctx.slug);
      container.innerHTML = buildMarkup(ctx);
      document.body.appendChild(container);

      container.addEventListener('click', onClick);
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

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
