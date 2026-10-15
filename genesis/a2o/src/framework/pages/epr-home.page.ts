/**
 * EprHomePage — page object for the shell-owned EPR atom home (/epr/{id}).
 *
 * The atom home is the universal address for any reachable resource: identity
 * (title/reach/notarized chips), the content itself (the focal slot), and the
 * four legs (who holds it, where it lives, how it's governed, where it came
 * from) — see genesis/a2o/features/content/epr-atom-home.feature.
 */

import { BasePage } from './base.page.js';
import { EPR_HOME, FEEDBACK_GATE, type EprHomeLeg, type EprHomeSection } from './selectors.js';

// The selectors live in the shared registry (selectors.ts); re-exported here so
// existing `import { EPR_HOME } from './epr-home.page.js'` callers keep working.
export { EPR_HOME, type EprHomeLeg, type EprHomeSection } from './selectors.js';

/** One "See also" row as rendered: its visible text, inference source and link. */
export interface SeeAlsoRow {
  text: string;
  inference: string | null;
  href: string | null;
}

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

  async legText(leg: EprHomeSection): Promise<string> {
    return (await this.testId(EPR_HOME.LEG(leg)).textContent())?.trim() ?? '';
  }

  async legVisible(leg: EprHomeSection): Promise<boolean> {
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

  // ───────────────────────────────────────────────────────────────────────
  // Folds (spec §11.3) — closed by default, opened by toggle or #fragment
  // ───────────────────────────────────────────────────────────────────────

  /** Visible label of a leg's fold toggle (e.g. "Network detail"). */
  async foldToggleText(leg: EprHomeLeg): Promise<string> {
    const toggle = this.testId(EPR_HOME.FOLD_TOGGLE(leg));
    return (await toggle.count()) > 0 ? (await toggle.first().innerText()).trim() : '';
  }

  async foldOpen(leg: EprHomeLeg): Promise<boolean> {
    return this.testId(EPR_HOME.FOLD(leg)).first().isVisible();
  }

  /** Open a leg's fold with its toggle (a no-op when it is open or the leg has no toggle). */
  async openFold(leg: EprHomeLeg): Promise<void> {
    if (await this.foldOpen(leg)) return;
    const toggle = this.testId(EPR_HOME.FOLD_TOGGLE(leg)).first();
    if ((await toggle.count()) === 0) return;
    await toggle.click();
    await this.testId(EPR_HOME.FOLD(leg)).first().waitFor({ state: 'visible', timeout: 10_000 });
  }

  /** Visible text of an element by test id, or '' when it is absent or hidden. */
  async visibleText(id: string): Promise<string> {
    const el = this.testId(id).first();
    if ((await el.count()) === 0 || !(await el.isVisible())) return '';
    return (await el.innerText()).trim();
  }

  async isVisible(id: string): Promise<boolean> {
    const el = this.testId(id).first();
    return (await el.count()) > 0 && el.isVisible();
  }

  async top(id: string): Promise<number | null> {
    return (await this.testId(id).first().boundingBox())?.y ?? null;
  }

  // ── Who holds it → Network detail ──

  /** Press "Check it's still whole" and return the one-line answer. */
  async verifyWhole(): Promise<string> {
    await this.testId(EPR_HOME.VERIFY).first().click();
    const result = this.testId(EPR_HOME.VERIFY_RESULT).first();
    await result.waitFor({ state: 'visible', timeout: 30_000 });
    return (await result.innerText()).trim();
  }

  /** Number of cells in the "Where the pieces are" grid. */
  async pieceCellCount(): Promise<number> {
    return this.testId(EPR_HOME.PIECES).first().locator(':scope > *, :scope [data-shard]').count();
  }

  // ── Where it came from ──

  async stewardRows(): Promise<string[]> {
    const rows = this.testId(EPR_HOME.STEWARD);
    const out: string[] = [];
    const n = await rows.count();
    for (let i = 0; i < n; i += 1) out.push((await rows.nth(i).innerText()).trim());
    return out;
  }

  async attestationCount(): Promise<number> {
    return this.testId(EPR_HOME.ATTESTATION).count();
  }

  // ── Where this lives → See also ──

  async seeAlsoRows(): Promise<SeeAlsoRow[]> {
    const cards = this.testId(EPR_HOME.SEE_ALSO)
      .first()
      .locator(`[data-testid="${EPR_HOME.SEE_ALSO_CARD}"]`);
    const rows: SeeAlsoRow[] = [];
    const n = await cards.count();
    for (let i = 0; i < n; i += 1) {
      const card = cards.nth(i);
      rows.push({
        text: (await card.innerText()).trim(),
        inference: await card.getAttribute(EPR_HOME.SEE_ALSO_INFERENCE_ATTR),
        // The row is the link, or carries one. Inline arrow: no named function
        // may cross into the browser (tsx keepNames → __name is undefined there).
        href: await card.evaluate((el: Element) => {
          const a = el.closest('a') ?? el.querySelector('a');
          return a ? a.getAttribute('href') : null;
        }),
      });
    }
    return rows;
  }

  // ── How it's governed ──

  /** Open the gate-feedback trigger's menu inside the "Raise a concern" host. */
  async openConcernMenu(): Promise<void> {
    const host = this.testId(EPR_HOME.RAISE_CONCERN).first();
    await host.waitFor({ state: 'visible', timeout: 15_000 });
    await host.locator(`[data-testid="${FEEDBACK_GATE.TRIGGER_BTN}"]`).first().click();
    await this.testId(FEEDBACK_GATE.TRIGGER_MENU)
      .first()
      .waitFor({ state: 'visible', timeout: 5_000 });
  }

  async concernTriggerCount(): Promise<number> {
    return this.testId(EPR_HOME.LEG('governed'))
      .first()
      .locator(`[data-testid="${FEEDBACK_GATE.TRIGGER_BTN}"]`)
      .count();
  }
}
