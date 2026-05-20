import { elementUpdated, expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';

import './register.js';
import type {
  ElohimComputeTile,
  ComputeTileHubValue,
  ComputeTileDeviceValue,
} from './elohim-compute-tile.js';
import { ElohimComputeTile as ElohimComputeTileClass } from './elohim-compute-tile.js';
import { clearMediaQueries, measureLuminanceChanges } from './testing/ua-prefs.js';
import { renderInLocale, requiresLogicalProperties } from './testing/i18n.js';

// ---------------------------------------------------------------------------
// Shared fixtures
// ---------------------------------------------------------------------------

const hubValue: ComputeTileHubValue = {
  kind: 'hub',
  hubId: 'hub:household:matthew-jessica-james',
  displayLabel: "Matthew's household",
  free: 5_368_709_120n,
  used: 10_737_418_240n,
  stewarded: 12_884_901_888n,
  memberDeviceCount: 3,
};

const deviceValue: ComputeTileDeviceValue = {
  kind: 'device',
  peerId: 'peer:12D3KooWMatthewLaptopHashFakeButWellFormed',
  displayName: "Matthew's laptop",
  archetype: 'desktop',
  online: true,
  compute: {
    free: 2_147_483_648n,
    used: 5_368_709_120n,
    stewarded: 3_221_225_472n,
  },
};

const offlineDevice: ComputeTileDeviceValue = {
  kind: 'device',
  peerId: 'peer:12D3KooWOfflineNodeFakeHash',
  displayName: 'Node alpha',
  archetype: 'node',
  online: false,
  compute: null,
};

const hubOfOne: ComputeTileHubValue = {
  kind: 'hub',
  hubId: 'hub:household:solo-device',
  displayLabel: 'Solo device hub',
  free: 1_073_741_824n,
  used: 2_147_483_648n,
  stewarded: 0n,
  memberDeviceCount: 1,
};

const freshNode: ComputeTileHubValue = {
  kind: 'hub',
  hubId: 'hub:household:fresh-node',
  displayLabel: 'Fresh node',
  free: null,
  used: null,
  stewarded: null,
  memberDeviceCount: 1,
};

// ---------------------------------------------------------------------------
// Core rendering — lens coverage
// ---------------------------------------------------------------------------

describe('<elohim-compute-tile> — lens coverage', () => {
  it('renders with default lens=standard (profile default)', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    expect(el).to.exist;
    expect(el.shadowRoot).to.exist;
  });

  it('minimal lens: renders qualitative summary for hub with healthy free space', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    el.value = hubValue;
    el.profile = { ...el.profile, lens: 'minimal' };
    await elementUpdated(el);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.match(/enough|low|available/i);
  });

  it('minimal lens: renders "Running low." when free < 1GB', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    el.value = {
      ...hubValue,
      free: 500_000_000n,
    };
    el.profile = { ...el.profile, lens: 'minimal' };
    await elementUpdated(el);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('Running low.');
  });

  it('minimal lens: renders "No sample available." when free is null', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    el.value = freshNode;
    el.profile = { ...el.profile, lens: 'minimal' };
    await elementUpdated(el);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('No sample available.');
  });

  it('simple lens: renders free bytes only', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    el.value = hubValue;
    el.profile = { ...el.profile, lens: 'simple' };
    await elementUpdated(el);
    const text = el.shadowRoot!.textContent ?? '';
    // "5 GB free" or locale variant
    expect(text).to.match(/\d.*free/i);
  });

  it('standard lens: renders free-of-total tuple', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    el.value = hubValue;
    el.profile = { ...el.profile, lens: 'standard' };
    await elementUpdated(el);
    const text = el.shadowRoot!.textContent ?? '';
    // "5 GB of 15 GB free"
    expect(text).to.match(/of.*free/i);
  });

  it('detail lens: renders full triptych including stewarded', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    el.value = hubValue;
    el.profile = { ...el.profile, lens: 'detail' };
    await elementUpdated(el);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.match(/stewarded/i);
    expect(text).to.match(/used/i);
  });

  it('debug lens: renders triptych + meta section with peer/hub id', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    el.value = hubValue;
    el.profile = { ...el.profile, lens: 'debug' };
    await elementUpdated(el);
    const meta = el.shadowRoot!.querySelector('.meta');
    expect(meta).to.exist;
    const metaText = meta!.textContent ?? '';
    expect(metaText).to.include('matthew-jessica-james');
  });

  it('debug lens: shows member count for hub', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    el.value = hubValue;
    el.profile = { ...el.profile, lens: 'debug' };
    await elementUpdated(el);
    const meta = el.shadowRoot!.querySelector('.meta');
    const metaText = meta!.textContent ?? '';
    expect(metaText).to.include('3');
  });

  it('debug lens: shows lastSampleAt when set', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile last-sample-at="2026-05-20T10:00:00Z"></elohim-compute-tile>
    `);
    el.value = hubValue;
    el.profile = { ...el.profile, lens: 'debug' };
    await elementUpdated(el);
    const meta = el.shadowRoot!.querySelector('.meta');
    expect(meta!.textContent).to.include('2026-05-20T10:00:00Z');
  });

  it('trace lens: renders source-route annotation', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile
        source-route="aggregate_stewarded_bytes_by_peer + system_metrics"
      ></elohim-compute-tile>
    `);
    el.value = hubValue;
    el.profile = { ...el.profile, lens: 'trace' };
    await elementUpdated(el);
    const meta = el.shadowRoot!.querySelector('.meta');
    expect(meta!.textContent).to.include('aggregate_stewarded_bytes_by_peer');
  });
});

// ---------------------------------------------------------------------------
// Polymorphic input shapes
// ---------------------------------------------------------------------------

describe('<elohim-compute-tile> — polymorphic input shapes', () => {
  it('hub-kind: shows displayLabel as contextual label', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    el.value = hubValue;
    el.profile = { ...el.profile, lens: 'standard' };
    await elementUpdated(el);
    const label = el.shadowRoot!.querySelector('.label');
    const labelText = label!.textContent?.trim() ?? '';
    expect(labelText).to.include("Matthew's household");
  });

  it('hub-kind: falls back to hubId when no displayLabel', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    const noLabel: ComputeTileHubValue = { ...hubValue, displayLabel: undefined };
    el.value = noLabel;
    el.profile = { ...el.profile, lens: 'standard' };
    await elementUpdated(el);
    const label = el.shadowRoot!.querySelector('.label');
    const labelText = label!.textContent?.trim() ?? '';
    expect(labelText).to.include('hub:household:matthew-jessica-james');
  });

  it('device-kind: shows archetype + displayName as contextual label', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    el.value = deviceValue;
    el.profile = { ...el.profile, lens: 'standard' };
    await elementUpdated(el);
    const label = el.shadowRoot!.querySelector('.label');
    const labelText = label!.textContent?.trim() ?? '';
    expect(labelText).to.include("Matthew's laptop");
  });

  it('device-kind: shows online status dot', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    el.value = deviceValue; // online: true
    await elementUpdated(el);
    const dot = el.shadowRoot!.querySelector('.status-dot');
    expect(dot!.getAttribute('data-online')).to.equal('true');
  });

  it('device-kind: offline device shows data-online=false', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    el.value = offlineDevice;
    await elementUpdated(el);
    const dot = el.shadowRoot!.querySelector('.status-dot');
    expect(dot!.getAttribute('data-online')).to.equal('false');
  });

  it('hub-of-one: renders as hub with memberDeviceCount=1', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    el.value = hubOfOne;
    el.profile = { ...el.profile, lens: 'debug' };
    await elementUpdated(el);
    const meta = el.shadowRoot!.querySelector('.meta');
    expect(meta!.textContent).to.include('1');
  });

  it('stewarded-zero edge case: renders 0 bytes stewarded in detail lens', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    el.value = hubOfOne; // stewarded: 0n
    el.profile = { ...el.profile, lens: 'detail' };
    await elementUpdated(el);
    const detail = el.shadowRoot!.querySelector('[part="value-detail"]');
    expect(detail).to.exist;
    // Should show stewarded value (0 bytes or formatted as 0 B / 0 KB)
    expect(detail!.textContent).to.match(/stewarded/i);
  });

  it('null compute for device-kind: treats triptych fields as null', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    el.value = offlineDevice; // compute: null
    el.profile = { ...el.profile, lens: 'standard' };
    await elementUpdated(el);
    const primary = el.shadowRoot!.querySelector('[part="value-primary"]');
    // Should show '—' or a no-sample message
    expect(primary).to.exist;
  });
});

// ---------------------------------------------------------------------------
// Content states
// ---------------------------------------------------------------------------

describe('<elohim-compute-tile> — content states', () => {
  it('empty state: renders when no value set', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    // value defaults to null — triggers empty state
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.match(/no compute data|available/i);
  });

  it('empty state: renders when tileState="empty"', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile tile-state="empty"></elohim-compute-tile>
    `);
    el.value = hubValue;
    await elementUpdated(el);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.match(/no compute data/i);
  });

  it('loading state: renders loading message', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile tile-state="loading"></elohim-compute-tile>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.match(/loading/i);
  });

  it('error state: renders error message', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile tile-state="error"></elohim-compute-tile>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.match(/unable|error/i);
  });

  it('stale state: renders staleness warning', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile tile-state="stale"></elohim-compute-tile>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.match(/stale/i);
  });

  it('offline state: renders offline message', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile tile-state="offline"></elohim-compute-tile>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.match(/offline/i);
  });

  it('state banner uses role=status and aria-live=polite', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile tile-state="loading"></elohim-compute-tile>
    `);
    const container = el.shadowRoot!.querySelector('.container');
    expect(container!.getAttribute('role')).to.equal('status');
    expect(container!.getAttribute('aria-live')).to.equal('polite');
  });
});

// ---------------------------------------------------------------------------
// Textuality: symbolic mode
// ---------------------------------------------------------------------------

describe('<elohim-compute-tile> — symbolic textuality mode', () => {
  it('symbolic mode: renders a gauge bar at simple lens', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    el.value = hubValue;
    el.profile = { ...el.profile, lens: 'simple', textuality: 'symbolic' };
    await elementUpdated(el);
    const gauge = el.shadowRoot!.querySelector('[part="gauge"]');
    expect(gauge).to.exist;
  });

  it('symbolic mode: renders a gauge bar at standard lens', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    el.value = hubValue;
    el.profile = { ...el.profile, lens: 'standard', textuality: 'symbolic' };
    await elementUpdated(el);
    const gauge = el.shadowRoot!.querySelector('[part="gauge"]');
    expect(gauge).to.exist;
  });

  it('symbolic mode: gauge fill percentage is non-zero when free > 0', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    el.value = hubValue;
    el.profile = { ...el.profile, lens: 'standard', textuality: 'symbolic' };
    await elementUpdated(el);
    const gaugeFill = el.shadowRoot!.querySelector('.gauge-fill') as HTMLElement;
    expect(gaugeFill).to.exist;
    const style = gaugeFill.getAttribute('style') ?? '';
    expect(style).to.include('inline-size');
    // Should not be 0%
    expect(style).to.not.include('inline-size: 0%');
  });

  it('symbolic mode: gauge is aria-hidden (decorative)', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    el.value = hubValue;
    el.profile = { ...el.profile, lens: 'standard', textuality: 'symbolic' };
    await elementUpdated(el);
    const gauge = el.shadowRoot!.querySelector('[part="gauge"]');
    expect(gauge!.getAttribute('aria-hidden')).to.equal('true');
  });

  it('textual mode: gauge is NOT rendered', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    el.value = hubValue;
    el.profile = { ...el.profile, lens: 'standard', textuality: 'textual' };
    await elementUpdated(el);
    const gauge = el.shadowRoot!.querySelector('[part="gauge"]');
    expect(gauge).to.be.null;
  });
});

// ---------------------------------------------------------------------------
// A11y precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-compute-tile> — a11y precondition gate', () => {
  it('passes axe-core scan in idle+hub state at standard lens', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    el.value = hubValue;
    el.profile = { ...el.profile, lens: 'standard' };
    await elementUpdated(el);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe-core scan in loading state', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile tile-state="loading"></elohim-compute-tile>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe-core scan in error state', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile tile-state="error"></elohim-compute-tile>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe-core scan in detail lens (full triptych)', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    el.value = hubValue;
    el.profile = { ...el.profile, lens: 'detail' };
    await elementUpdated(el);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe-core scan in debug lens (with meta section)', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile last-sample-at="2026-05-20T10:00:00Z"></elohim-compute-tile>
    `);
    el.value = hubValue;
    el.profile = { ...el.profile, lens: 'debug' };
    await elementUpdated(el);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('container has role=group and a meaningful aria-label (non-state render)', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    el.value = hubValue;
    el.profile = { ...el.profile, lens: 'standard' };
    await elementUpdated(el);
    const container = el.shadowRoot!.querySelector('.container');
    expect(container!.getAttribute('role')).to.equal('group');
    const ariaLabel = container!.getAttribute('aria-label') ?? '';
    expect(ariaLabel.length).to.be.greaterThan(0);
    expect(ariaLabel).to.match(/compute tile/i);
  });

  it('status-dot is aria-hidden (decorative)', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    el.value = deviceValue;
    await elementUpdated(el);
    const dot = el.shadowRoot!.querySelector('.status-dot');
    expect(dot!.getAttribute('aria-hidden')).to.equal('true');
  });
});

// ---------------------------------------------------------------------------
// UA-prefs precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-compute-tile> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS gates gauge transition behind prefers-reduced-motion / update media queries', () => {
    const cssText = (
      ElohimComputeTileClass as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.contain('prefers-reduced-motion: no-preference');
    expect(cssText).to.contain('update: fast');
  });

  it('CSS transition appears only inside the motion/update guard', () => {
    const cssText = (
      ElohimComputeTileClass as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const transitionIdx = cssText.indexOf('transition:');
    const mediaIdx = cssText.indexOf('@media (prefers-reduced-motion: no-preference)');
    expect(transitionIdx).to.be.greaterThan(mediaIdx);
  });

  it('includes forced-colors CSS override block', () => {
    const cssText = (
      ElohimComputeTileClass as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.contain('forced-colors: active');
  });

  it('forced-colors block uses Canvas and CanvasText system colors', () => {
    const cssText = (
      ElohimComputeTileClass as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const fcStart = cssText.indexOf('forced-colors: active');
    const snippet = cssText.slice(fcStart, fcStart + 500);
    expect(snippet).to.include('Canvas');
    expect(snippet).to.include('CanvasText');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    el.value = hubValue;
    await elementUpdated(el);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });

  it('container has min touch target (44×44 px minimum not enforced here — tile is not a button)', async () => {
    // The tile is a display primitive (not interactive by default).
    // It does not need a 44×44 touch target. This test asserts the tile
    // still renders with a non-zero size.
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    el.value = hubValue;
    await elementUpdated(el);
    const container = el.shadowRoot!.querySelector('.container') as HTMLElement;
    const rect = container.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
    expect(rect.height).to.be.greaterThan(0);
  });
});

// ---------------------------------------------------------------------------
// I18n precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-compute-tile> — i18n precondition gate', () => {
  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimComputeTileClass as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });

  it('renders in en locale without crashing', async () => {
    const el = await renderInLocale<ElohimComputeTile>(
      'en',
      html`
        <elohim-compute-tile></elohim-compute-tile>
      `
    );
    el.value = hubValue;
    el.profile = { ...el.profile, locale: 'en', lens: 'standard' };
    await elementUpdated(el);
    expect(el.shadowRoot!.textContent).to.match(/free/i);
  });

  it('renders in RTL document direction (he-IL) without crashing', async () => {
    const el = await renderInLocale<ElohimComputeTile>(
      'he-IL',
      html`
        <elohim-compute-tile></elohim-compute-tile>
      `
    );
    el.value = hubValue;
    el.profile = { ...el.profile, locale: 'he', lens: 'standard' };
    await elementUpdated(el);
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const container = el.shadowRoot!.querySelector('.container') as HTMLElement;
    const rect = container.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
  });

  it('state banner strings do not appear outside Lit template expressions (no hardcoded literals)', async () => {
    // Structural: the CSS text should contain no user-visible English words
    // that were not put there by the @media string context.
    // We verify by inspecting that the loading/error/stale messages come
    // through the msg() call path (tested via rendered output, not CSS).
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile tile-state="loading"></elohim-compute-tile>
    `);
    const cssText = (
      ElohimComputeTileClass as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    // The CSS text should NOT contain user-visible state messages
    expect(cssText).to.not.include('Loading compute data');
    expect(cssText).to.not.include('Unable to load');
    expect(cssText).to.not.include('Device is offline');
    expect(cssText).to.not.include('No compute data');
    // The rendered HTML should contain the message (proving it comes from render())
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.match(/loading/i);
  });
});

// ---------------------------------------------------------------------------
// prop/attribute contract
// ---------------------------------------------------------------------------

describe('<elohim-compute-tile> — prop/attribute contract', () => {
  it('tileState defaults to "idle"', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    expect(el.tileState).to.equal('idle');
  });

  it('tileState="loading" reflects on attribute', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile tile-state="loading"></elohim-compute-tile>
    `);
    expect(el.tileState).to.equal('loading');
    expect(el.getAttribute('tile-state')).to.equal('loading');
  });

  it('lastSampleAt attribute is read correctly', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile last-sample-at="2026-05-20T10:00:00Z"></elohim-compute-tile>
    `);
    expect(el.lastSampleAt).to.equal('2026-05-20T10:00:00Z');
  });

  it('sourceRoute attribute is read correctly', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile source-route="aggregate_stewarded_bytes_by_peer"></elohim-compute-tile>
    `);
    expect(el.sourceRoute).to.equal('aggregate_stewarded_bytes_by_peer');
  });

  it('value prop accepts ComputeTileHubValue', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    el.value = hubValue;
    await elementUpdated(el);
    expect((el.value as ComputeTileHubValue).kind).to.equal('hub');
  });

  it('value prop accepts ComputeTileDeviceValue', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile></elohim-compute-tile>
    `);
    el.value = deviceValue;
    await elementUpdated(el);
    expect((el.value as ComputeTileDeviceValue).kind).to.equal('device');
  });

  it('hidden attribute removes element from layout', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile hidden></elohim-compute-tile>
    `);
    const rect = el.getBoundingClientRect();
    expect(rect.width).to.equal(0);
    expect(rect.height).to.equal(0);
  });
});

// ---------------------------------------------------------------------------
// Slot
// ---------------------------------------------------------------------------

describe('<elohim-compute-tile> — slots', () => {
  it('named label slot overrides contextual label content', async () => {
    const el = await fixture<ElohimComputeTile>(html`
      <elohim-compute-tile>
        <span slot="label">Custom label</span>
      </elohim-compute-tile>
    `);
    el.value = hubValue;
    await elementUpdated(el);
    const labelSlot = el.shadowRoot!.querySelector('slot[name="label"]') as HTMLSlotElement;
    expect(labelSlot).to.exist;
    const assigned = labelSlot.assignedNodes({ flatten: true });
    const text = assigned
      .map(n => n.textContent)
      .join('')
      .trim();
    expect(text).to.include('Custom label');
  });
});
