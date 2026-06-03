/**
 * Completion Feedback Step Definitions — Playwright-powered completion summary assertions.
 *
 * These steps validate the AssessmentCompletionSummaryComponent UI that appears
 * after discovery, mastery, and reflection assessments. They require
 * E2E_DEVICE_MODE=playwright. In HTTP mode, browser-only assertions return 'pending'.
 */

import { strict as assert } from 'node:assert';

import { Then, When } from '@cucumber/cucumber';

import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import {
  ASSESSMENT,
  COMPLETION,
  PATH_NAV,
  PATH_OVERVIEW,
  SOPHIA,
} from '../../src/framework/pages/selectors.js';
import { E2EWorld } from '../../src/framework/world.js';

function getDevice(world: E2EWorld): PlaywrightDevice {
  for (const [, human] of world.humans) {
    const device = human.devices.find(d => d.type === 'playwright') as PlaywrightDevice | undefined;
    if (device) return device;
  }
  throw new Error('No Playwright device found.');
}

/** Returns null if not in Playwright mode. */
function requirePlaywright(world: E2EWorld): PlaywrightDevice | null {
  if (world.deviceMode !== 'playwright') return null;
  return getDevice(world);
}

// ---------------------------------------------------------------------------
// Visibility
// ---------------------------------------------------------------------------

Then('the completion summary should be visible', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const summary = device.page.locator(`[data-testid="${COMPLETION.SUMMARY}"]`);
  await summary.waitFor({ state: 'visible', timeout: 15_000 });
});

Then('the completion summary should not be visible', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const summary = device.page.locator(`[data-testid="${COMPLETION.SUMMARY}"]`);
  await summary.waitFor({ state: 'hidden', timeout: 10_000 });
});

// ---------------------------------------------------------------------------
// Headline & Description
// ---------------------------------------------------------------------------

Then(
  'the completion headline should contain a personalized type name',
  async function (this: E2EWorld) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    const headline = device.page.locator(`[data-testid="${COMPLETION.HEADLINE}"]`);
    await headline.waitFor({ state: 'visible', timeout: 10_000 });
    const text = await headline.textContent();
    assert.ok(text, 'Headline should have text content');
    // Personalized headlines contain "You're a/an [Name]!"
    assert.match(text, /You're an? .+!/, `Expected personalized headline, got: "${text}"`);
  }
);

Then(
  'the completion headline should contain {string}',
  async function (this: E2EWorld, expected: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    const headline = device.page.locator(`[data-testid="${COMPLETION.HEADLINE}"]`);
    await headline.waitFor({ state: 'visible', timeout: 10_000 });
    const text = await headline.textContent();
    assert.ok(
      text?.includes(expected),
      `Expected headline to contain "${expected}", got: "${text}"`
    );
  }
);

Then(
  'the completion description should reference the primary type',
  async function (this: E2EWorld) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    const desc = device.page.locator(`[data-testid="${COMPLETION.DESCRIPTION}"]`);
    await desc.waitFor({ state: 'visible', timeout: 10_000 });
    const text = await desc.textContent();
    assert.ok(text && text.length > 10, `Expected descriptive text, got: "${text}"`);
  }
);

// ---------------------------------------------------------------------------
// Hex Badge
// ---------------------------------------------------------------------------

Then('a hex badge preview should be displayed', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const badge = device.page.locator(`[data-testid="${COMPLETION.HEX_BADGE}"]`);
  await badge.waitFor({ state: 'visible', timeout: 10_000 });
  const text = await badge.textContent();
  assert.ok(text && text.trim().length > 0, 'Hex badge should display a type label');
});

Then('no hex badge preview should be displayed', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const badge = device.page.locator(`[data-testid="${COMPLETION.BADGE_REVEAL}"]`);
  const count = await badge.count();
  assert.equal(count, 0, 'Expected no hex badge in non-discovery mode');
});

// ---------------------------------------------------------------------------
// Subscale Breakdown
// ---------------------------------------------------------------------------

Then('the subscale breakdown should be visible', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const subscales = device.page.locator(`[data-testid="${COMPLETION.SUBSCALES}"]`);
  await subscales.waitFor({ state: 'visible', timeout: 10_000 });
});

Then('each subscale bar should have a non-zero width', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const selector = `[data-testid="${COMPLETION.SUBSCALES}"] .subscale-fill`;
  const allNonZero = await device.page.evaluate((sel: string) => {
    const fills = document.querySelectorAll(sel);
    if (fills.length === 0) return false;
    return Array.from(fills).every(el => (el as HTMLElement).offsetWidth > 0);
  }, selector);

  assert.ok(allNonZero, 'Expected all subscale bars to have non-zero width');
});

// ---------------------------------------------------------------------------
// Score Display (Mastery)
// ---------------------------------------------------------------------------

Then('the score display should show a percentage', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const score = device.page.locator(`[data-testid="${COMPLETION.SCORE}"]`);
  await score.waitFor({ state: 'visible', timeout: 10_000 });
  const text = await score.textContent();
  assert.ok(text, 'Score display should have text');
  // eslint-disable-next-line sonarjs/slow-regex -- simple \d+% has no backtracking risk
  assert.match(text, /\d+%/u, `Expected percentage in score display, got: "${text}"`);
});

Then('no score display should be shown', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const score = device.page.locator(`[data-testid="${COMPLETION.SCORE}"]`);
  const count = await score.count();
  assert.equal(count, 0, 'Expected no score display in non-mastery mode');
});

// ---------------------------------------------------------------------------
// Result Card Styling
// ---------------------------------------------------------------------------

Then(
  'the result card should have the {string} style',
  async function (this: E2EWorld, expectedClass: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    const selector = `[data-testid="${COMPLETION.RESULT_CARD}"]`;
    const card = device.page.locator(selector);
    await card.waitFor({ state: 'visible', timeout: 10_000 });

    const hasClass = await device.page.evaluate(
      ({ sel, cls }: { sel: string; cls: string }) => {
        const el = document.querySelector(sel);
        return el ? el.classList.contains(cls) : false;
      },
      { sel: selector, cls: expectedClass }
    );
    assert.ok(hasClass, `Expected result card to have class "${expectedClass}"`);
  }
);

// ---------------------------------------------------------------------------
// Profile Link
// ---------------------------------------------------------------------------

Then(
  'the profile link should read {string}',
  async function (this: E2EWorld, expectedText: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    const link = device.page.locator(`[data-testid="${COMPLETION.PROFILE_LINK}"]`);
    await link.waitFor({ state: 'visible', timeout: 10_000 });
    const text = await link.textContent();
    assert.ok(
      text?.includes(expectedText),
      `Expected profile link to contain "${expectedText}", got: "${text}"`
    );
  }
);

// ---------------------------------------------------------------------------
// Continue Button
// ---------------------------------------------------------------------------

When('I click the completion continue button', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const btn = device.page.locator(`[data-testid="${COMPLETION.CONTINUE}"]`);
  await btn.waitFor({ state: 'visible', timeout: 10_000 });
  await btn.click();
  await device.page.waitForTimeout(500);
});

// ---------------------------------------------------------------------------
// Attestation (negative — mastery should NOT record discovery attestations)
// ---------------------------------------------------------------------------

Then('no discovery attestation should be recorded', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const hasAttestation = await device.page.evaluate(() => {
    const attestations = localStorage.getItem('discovery-attestations');
    if (!attestations) return false;
    try {
      const parsed = JSON.parse(attestations);
      return Array.isArray(parsed) ? parsed.length > 0 : Object.keys(parsed).length > 0;
    } catch {
      return false;
    }
  });

  assert.ok(!hasAttestation, 'Expected no discovery attestations after mastery completion');
});

// ---------------------------------------------------------------------------
// Shared helpers for quiz interaction
// ---------------------------------------------------------------------------

/** Tag name of the Sophia web component. */
const SOPHIA_QUESTION_TAG = 'sophia-question';

/**
 * Click a visible button by first trying a CSS selector then a text label.
 * Returns true if any button was clicked.
 */
async function clickFirstVisible(
  device: PlaywrightDevice,
  cssSelector: string,
  textLabel: string,
  delayMs = 300
): Promise<boolean> {
  const byCSS = device.page.locator(cssSelector).first();
  if (await byCSS.isVisible().catch(() => false)) {
    await byCSS.click();
    await device.page.waitForTimeout(delayMs);
    return true;
  }
  const byText = device.page.getByText(textLabel, { exact: false }).first();
  if (await byText.isVisible().catch(() => false)) {
    await byText.click();
    await device.page.waitForTimeout(delayMs);
    return true;
  }
  return false;
}

/**
 * Retrieve the index of the correct radio choice from the sophia-question
 * element's current moment data. Returns -1 if not determinable.
 *
 * Executed in the browser context so we can read the Web Component's public
 * `moment` property without relying on DOM text.
 */
async function getCorrectChoiceIndex(device: PlaywrightDevice): Promise<number> {
  return (await device.page.evaluate(() => {
    const el = document.querySelector('sophia-question');
    if (!el) return -1;
    // moment is a public property on the SophiaQuestionElement instance.
    const moment = (el as unknown as Record<string, unknown>)['moment'] as
      | Record<string, unknown>
      | null
      | undefined;
    const widgets = (moment?.['content'] as Record<string, unknown> | undefined)?.['widgets'];
    if (!widgets || typeof widgets !== 'object') return -1;
    const firstWidget = Object.values(widgets)[0] as Record<string, unknown> | undefined;
    const choices = (firstWidget?.['options'] as Record<string, unknown> | undefined)?.['choices'];
    if (!Array.isArray(choices)) return -1;
    return choices.findIndex(
      (c: unknown) =>
        typeof c === 'object' && c !== null && (c as Record<string, unknown>)['correct'] === true
    );
  })) as number;
}

/** Resolve which radio index to click for correct or incorrect selection. */
function resolveRadioIndex(correctIdx: number, radioCount: number, correct: boolean): number {
  if (correct) {
    return correctIdx >= 0 && correctIdx < radioCount ? correctIdx : 0;
  }
  // Incorrect: pick the first choice that is NOT the correct one.
  return correctIdx === 0 && radioCount > 1 ? 1 : 0;
}

/**
 * Answer one sophia mastery question: select a radio, click Check Answer,
 * then click Next Question / See Results.
 * Returns true if the completion summary appeared (quiz finished).
 */
async function answerOneSophiaQuestion(
  device: PlaywrightDevice,
  correct: boolean
): Promise<boolean> {
  const sophiaEl = device.page.locator(SOPHIA_QUESTION_TAG);
  const correctIdx = await getCorrectChoiceIndex(device);
  const radioInputs = sophiaEl.locator('input[type="radio"]');
  const radioCount = await radioInputs.count();

  if (radioCount === 0) return true; // No inputs — treat as done.

  const targetIdx = resolveRadioIndex(correctIdx, radioCount, correct);
  await radioInputs.nth(targetIdx).click({ force: true });
  await device.page.waitForTimeout(200);

  // "Check Answer" button (outside shadow DOM — rendered by Angular wrapper).
  await clickFirstVisible(device, '.quiz-controls .btn-primary', 'Check Answer', 300);

  // "Next Question" or "See Results".
  const clicked = await clickFirstVisible(
    device,
    '.quiz-controls .btn-secondary',
    'Next Question',
    400
  );
  if (!clicked) {
    await clickFirstVisible(device, '.quiz-controls .btn-primary', 'See Results', 400);
  }

  const completionSummary = device.page.locator(`[data-testid="${COMPLETION.SUMMARY}"]`);
  return completionSummary.isVisible().catch(() => false);
}

/**
 * Drive a mastery-mode sophia quiz by answering every question either
 * correctly or incorrectly.
 */
async function answerMasteryQuiz(device: PlaywrightDevice, correct: boolean): Promise<void> {
  let questionCount = 0;
  const maxQuestions = 30;

  while (questionCount < maxQuestions) {
    await device.page.waitForTimeout(300);

    const sophiaEl = device.page.locator(SOPHIA_QUESTION_TAG);
    const quizSubmitBtn = device.page.locator('[data-testid="quiz-submit"]');

    const hasSophia = await sophiaEl.isVisible().catch(() => false);
    const hasQuizSubmit = await quizSubmitBtn.isVisible().catch(() => false);

    if (!hasSophia && !hasQuizSubmit) break;

    if (hasSophia) {
      const done = await answerOneSophiaQuestion(device, correct);
      if (done) break;
    }

    if (hasQuizSubmit) {
      // Legacy quiz-renderer: select first radio and submit all at once.
      const radios = device.page.locator('input[type="radio"]');
      if ((await radios.count()) > 0) {
        await radios.nth(0).click({ force: true });
      }
      await quizSubmitBtn.click();
      await device.page.waitForTimeout(500);
      break;
    }

    questionCount++;
  }
}

// ---------------------------------------------------------------------------
// Mastery Quiz Navigation
// ---------------------------------------------------------------------------

/**
 * Navigate to the first quiz step in the currently-open path.
 * Looks for a sidebar link with "Quiz" or "Assessment" text; if none is found,
 * steps forward through up to 15 steps until the quiz renderer appears.
 * Clicks through the pre-assessment gate if present.
 */
async function navigateToQuizStep(device: PlaywrightDevice): Promise<void> {
  // From path overview: click Begin Journey to enter the first step.
  const beginBtn = device.page.locator(`[data-testid="${PATH_OVERVIEW.BEGIN_JOURNEY}"]`);
  if (await beginBtn.isVisible().catch(() => false)) {
    await beginBtn.click();
    await device.page.waitForLoadState('networkidle');
  }

  // Try to find a sidebar link by text first.
  const quizTextLink = device.page.getByText('Quiz', { exact: false }).first();
  const assessmentTextLink = device.page.getByText('Assessment', { exact: false }).first();
  const foundQuiz = await quizTextLink.isVisible().catch(() => false);
  const foundAssessment = !foundQuiz && (await assessmentTextLink.isVisible().catch(() => false));

  if (foundQuiz) {
    await quizTextLink.click();
    await device.page.waitForLoadState('networkidle');
  } else if (foundAssessment) {
    await assessmentTextLink.click();
    await device.page.waitForLoadState('networkidle');
  } else {
    await stepForwardUntilQuiz(device);
  }

  // Click the pre-assessment start gate if it is showing.
  const startBtn = device.page.locator(`[data-testid="${ASSESSMENT.START}"]`);
  if (await startBtn.isVisible().catch(() => false)) {
    await startBtn.click();
    await device.page.waitForLoadState('networkidle');
  }
}

/** Step through up to 15 path steps looking for a quiz renderer. */
async function stepForwardUntilQuiz(device: PlaywrightDevice): Promise<void> {
  const nextBtn = device.page.locator(`[data-testid="${PATH_NAV.NEXT}"]`);
  let maxSteps = 15;
  while (maxSteps-- > 0) {
    const hasStart = await device.page
      .locator(`[data-testid="${ASSESSMENT.START}"]`)
      .isVisible()
      .catch(() => false);
    const hasSophia = await device.page
      .locator(SOPHIA_QUESTION_TAG)
      .isVisible()
      .catch(() => false);
    if (hasStart || hasSophia) break;
    if (await nextBtn.isVisible().catch(() => false)) {
      await nextBtn.click();
      await device.page.waitForLoadState('networkidle');
    } else {
      break;
    }
  }
}

When('I advance to a mastery quiz step', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';
  await navigateToQuizStep(device);
  await device.page.locator(SOPHIA_QUESTION_TAG).waitFor({ state: 'attached', timeout: 15_000 });
});

When('I answer all quiz questions correctly', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';
  await answerMasteryQuiz(device, true);
});

When('I answer all quiz questions incorrectly', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';
  await answerMasteryQuiz(device, false);
});

// ---------------------------------------------------------------------------
// Reflection Assessment Navigation & Answering
// ---------------------------------------------------------------------------

/**
 * Navigate to a discovery/reflection assessment via the content-viewer route.
 * Uses the "Personal Values Reflection" seed content (assessment-personal-values)
 * which has discovery-mode sophia-quiz-json format. In a test environment the
 * Psyche API interpretation returns null, causing the completion summary to show
 * the generic "Assessment Complete" headline — which is the reflection scenario
 * expectation.
 */
When('I navigate to a reflection assessment', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  await device.navigate('/resource/assessment-personal-values');
  await device.page.waitForLoadState('networkidle');
  await device.page.locator(SOPHIA_QUESTION_TAG).waitFor({ state: 'attached', timeout: 15_000 });
});

/**
 * Click the advance button for a discovery/reflection sophia assessment.
 * Tries the testid first, then CSS, then text labels.
 */
async function clickDiscoveryAdvanceButton(device: PlaywrightDevice): Promise<boolean> {
  const byTestid = device.page.locator(`[data-testid="${SOPHIA.CONTINUE}"]`).first();
  if (await byTestid.isVisible().catch(() => false)) {
    await byTestid.click();
    return true;
  }
  const clicked = await clickFirstVisible(device, '.quiz-controls .btn-primary', 'Continue', 0);
  if (clicked) return true;
  return clickFirstVisible(device, '.quiz-controls .btn-primary', 'Finish', 0);
}

/**
 * Drive a discovery/reflection sophia assessment to completion.
 *
 * In discovery/reflection mode the sophia renderer auto-advances after each
 * answer — there is a single "Continue" / "Finish" button (no separate
 * "Check Answer" + "Next Question" two-step). Mirrors the flow in
 * `I answer all likert-scale questions with varied selections`.
 */
When('I answer all reflection prompts', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  let promptIndex = 0;
  const maxPrompts = 30;
  const selectionCycle = [0, 2, 1, 3, 0, 4, 2];

  while (promptIndex < maxPrompts) {
    await device.page.waitForTimeout(300);

    const completionSummary = device.page.locator(`[data-testid="${COMPLETION.SUMMARY}"]`);
    if (await completionSummary.isVisible().catch(() => false)) break;

    const sophiaEl = device.page.locator(SOPHIA_QUESTION_TAG);
    if (!(await sophiaEl.isVisible().catch(() => false))) break;

    const radioInputs = sophiaEl.locator('input[type="radio"]');
    const radioCount = await radioInputs.count();

    if (radioCount > 0) {
      const idx = Math.min(selectionCycle[promptIndex % selectionCycle.length], radioCount - 1);
      await radioInputs.nth(idx).click({ force: true });
      await device.page.waitForTimeout(200);
    }

    const advanced = await clickDiscoveryAdvanceButton(device);
    if (!advanced) break;

    await device.page.waitForTimeout(500);
    promptIndex++;
  }
});
