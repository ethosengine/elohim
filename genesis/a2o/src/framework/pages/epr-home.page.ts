/**
 * EprHomePage — page object for the shell-owned EPR atom home (/epr/{id}).
 *
 * The atom home is the universal address for any reachable resource: identity
 * (title/reach/notarized chips), the content itself (the focal slot), and the
 * four legs (who holds it, where it lives, how it's governed, where it came
 * from) — see genesis/a2o/features/content/epr-atom-home.feature.
 */

import { BasePage } from './base.page.js';

/** The four supporting sections that ground a resource in its network. */
export type EprHomeLeg = 'holds' | 'lives' | 'governed' | 'from';

/** Selectors for the shell-owned EPR atom home (/epr/{id}) — data-testid contract from the spec §6. */
export const EPR_HOME = {
  ROOT: 'epr-home',
  LOADING: 'epr-home-loading',
  GATE: 'epr-home-gate',
  GATE_BACK: 'epr-home-gate-back',
  ARRIVAL: 'epr-home-arrival',
  TITLE: 'epr-home-title',
  CHIP_REACH: 'epr-home-chip-reach',
  CHIP_NOTARIZED: 'epr-home-chip-notarized',
  CHIP_HELD: 'epr-home-chip-held',
  OPEN_IN_BUNDLE: 'epr-home-open-in-bundle',
  FOCAL: 'epr-home-focal',
  FOCAL_CLAIMED: 'epr-home-focal-claimed',
  YOUR_MARK: 'epr-home-your-mark',
  ADDRESS: 'epr-home-address',
  LEG: (leg: EprHomeLeg) => `epr-home-leg-${leg}`,
} as const;

export class EprHomePage extends BasePage {
  /** Wait until either the frame (ROOT) or the out-of-reach gate (GATE) is visible. */
  async waitForReady(): Promise<void> {
    await this.locate(`[data-testid="${EPR_HOME.ROOT}"], [data-testid="${EPR_HOME.GATE}"]`)
      .first()
      .waitFor({ state: 'visible', timeout: 20_000 });
  }

  async goto(appUrl: string, resourceId: string): Promise<void> {
    await this.page.goto(`${appUrl}/epr/${resourceId}`, { waitUntil: 'networkidle' });
    await this.waitForReady();
  }

  async title(): Promise<string> {
    return (await this.testId(EPR_HOME.TITLE).textContent())?.trim() ?? '';
  }

  async chipText(chip: 'reach' | 'notarized' | 'held'): Promise<string> {
    return (await this.testId(`epr-home-chip-${chip}`).textContent())?.trim() ?? '';
  }

  async focalShape(): Promise<'immersive' | 'reading' | null> {
    const cls = (await this.testId(EPR_HOME.FOCAL).getAttribute('class')) ?? '';
    if (cls.includes('epr-home__focal--immersive')) return 'immersive';
    if (cls.includes('epr-home__focal--reading')) return 'reading';
    return null;
  }

  async legText(leg: EprHomeLeg): Promise<string> {
    return (await this.testId(EPR_HOME.LEG(leg)).textContent())?.trim() ?? '';
  }

  async legVisible(leg: EprHomeLeg): Promise<boolean> {
    return this.testId(EPR_HOME.LEG(leg)).isVisible();
  }

  async legsBesideContent(): Promise<boolean> {
    const focal = await this.testId(EPR_HOME.FOCAL).boundingBox();
    const legs = await this.testId(EPR_HOME.LEG('holds')).boundingBox();
    if (!focal || !legs) return false;
    // The rail must start level with the TOP of the reading column, not just
    // anywhere alongside it (a rail that starts mid-column would still pass
    // the old "y < focal bottom" check).
    return legs.y <= focal.y + 8 && legs.x > focal.x + focal.width - 1;
  }

  async focalFullWidth(): Promise<boolean> {
    const focal = await this.testId(EPR_HOME.FOCAL).boundingBox();
    const root = await this.testId(EPR_HOME.ROOT).boundingBox();
    if (!focal || !root) return false;
    return focal.width > root.width * 0.9;
  }

  async bodyText(): Promise<string> {
    // innerText, not textContent — the latter includes hidden nodes (e.g. the
    // Angular transfer-state <script> carrying serialized JSON with fields
    // like "replicaCount", which false-matched a "the shard map and replica
    // counts stay behind a link" assertion that only means VISIBLE text).
    return this.locate('body').innerText();
  }

  async arrivalText(): Promise<string | null> {
    const chip = this.testId(EPR_HOME.ARRIVAL);
    return (await chip.count()) > 0 ? ((await chip.textContent())?.trim() ?? '') : null;
  }

  async clickRelated(resourceId: string): Promise<void> {
    await this.locate(
      `[data-testid="${EPR_HOME.LEG('lives')}"] a[data-related-id="${resourceId}"]`
    ).click();
    await this.testId(EPR_HOME.GATE).waitFor({ state: 'visible', timeout: 20_000 });
    // The related link is a plain <a href> (a full browser navigation, not
    // an SPA transition — see epr-home-legs.component.html), so the gate's
    // OWN referrer content (arrival/gate-back, sourced from the session nav
    // stack the prior atom recorded) can still be settling in the same
    // change-detection pass that made the container itself visible.
    await this.page.waitForLoadState('networkidle');
  }

  async clickOpenInBundle(): Promise<void> {
    await this.testId(EPR_HOME.OPEN_IN_BUNDLE).click();
    await this.page.waitForLoadState('networkidle');
  }

  async gateText(): Promise<string> {
    return (await this.testId(EPR_HOME.GATE).textContent())?.trim() ?? '';
  }

  async gateBackHref(): Promise<string | null> {
    return this.testId(EPR_HOME.GATE_BACK).getAttribute('href');
  }

  async has(id: string): Promise<boolean> {
    return (await this.testId(id).count()) > 0;
  }
}
