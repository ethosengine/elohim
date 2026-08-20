/**
 * Protocol trust-surface omnibar chrome — served-HTML step definitions.
 *
 * Drives genesis/a2o/features/protocol/protocol-omnibar-chrome.feature: every
 * 2xx text/html page the doorway serves through the EPR router must carry the
 * runtime-served omnibar chrome (the EPR trust surface — the bridge toward an
 * elohim-native browser), spliced in before </body> exactly once.
 *
 * API-tier (no Playwright): a plain HTTP GET is enough — the chrome is in the
 * bytes. The GET + Content-Type steps are REUSED from
 * steps/protocol/landing-page-dogfood.steps.ts (the generic
 *   When I GET {string} from the doorway
 *   Then the doorway response Content-Type contains {string}
 * pair, which stashes the fetch on `this.doorwayResponse`). This file adds the
 * serving-precondition step and the two body-assertions that read that same
 * response and check the omnibar markers.
 *
 * Why this file owns its own status step rather than reusing the generic
 * `Then the doorway response status is {int}` (2026-08-20): when the fleet is
 * shedding, that step failed with a bare `503 !== 200` — no URL, no cause — and
 * an environment outage read exactly like a chrome regression (it was
 * mis-triaged as a code red in the elohim-genesis #1489 inventory). The
 * replacement asserts the SAME 200 but decodes the doorway's two shed shapes,
 * so the failure text itself says whether the trust-surface contract was ever
 * exercised.
 *
 * The asserted markers are the source of truth in
 * elohim/elohim-chrome-asset/src/lib.rs (CONTEXT_SCRIPT_ID +
 * element_script_path() / inject_element()) — this file mirrors exactly what
 * inject_element() emits, so a drift on either side surfaces here.
 */

import { strict as assert } from 'node:assert';

import { Then } from '@cucumber/cucumber';

/**
 * Shares the `doorwayResponse` field the reused dogfood GET step writes (same
 * runtime World instance; each steps file declares only the slice of the world
 * it touches — see landing-page-dogfood.steps.ts's DogfoodWorld).
 */
interface OmnibarChromeWorld {
  doorwayResponse?: Response;
}

/** Escape a literal string for embedding inside a `new RegExp(...)`. */
function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, String.raw`\$&`);
}

/**
 * Read the last doorway response's body without disturbing it — each call
 * clones first, so multiple body-assertions on one response are safe (the
 * reused status/Content-Type steps only touch `.status` / `.headers`, so the
 * original body stream is still intact to clone).
 */
async function servedHtml(world: OmnibarChromeWorld): Promise<string> {
  const res = world.doorwayResponse;
  assert.ok(res, 'No doorway response captured — run "When I GET ... from the doorway" first');
  return res.clone().text();
}

/**
 * Name the doorway's SHED shape, if this response is one.
 *
 * The doorway has exactly two ways of answering "I cannot serve you a page",
 * and neither one ever reaches the EPR router — so neither can say anything
 * about the omnibar splice:
 *
 *  1. `503 {"status":"catching-up","retryAfter":N}` — the upstream/admission
 *     shed gate (doorway/doorway-service/src/routes/catching_up.rs). It is
 *     content-negotiated: only an explicit `text/html` Accept gets the HTML
 *     page, so an SDK/`fetch` client (this suite) always gets the JSON.
 *  2. `503` + the degraded root page (`root_unavailable_html` in
 *     doorway/doorway-service/src/server/http.rs) — the EprRouter is empty
 *     because the storage/conductor backend is unreachable from this doorway.
 *
 * Returned as a sentence so the assertion message carries the cause instead of
 * a bare number.
 */
function shedShape(status: number, body: string): string | undefined {
  if (body.includes('"status":"catching-up"')) {
    return 'the doorway SHED this request at its upstream/admission gate (routes/catching_up.rs)';
  }
  if (body.includes('reconnecting to the substrate') || body.includes('catching its breath')) {
    return 'the doorway served its degraded root page — the EPR projection cache is empty because the storage/conductor backend is unreachable from this doorway (root_unavailable_html)';
  }
  if (status === 503) {
    return 'the doorway answered 503 without a recognised shed body';
  }
  return undefined;
}

Then(
  'the doorway serves that page — status 200, not a shed page',
  async function (this: OmnibarChromeWorld) {
    const res = this.doorwayResponse;
    assert.ok(res, 'No doorway response captured — run "When I GET ... from the doorway" first');
    if (res.status === 200) return;

    // Only read the body on the failure path — a 200 must stay intact for the
    // marker assertions that follow.
    const body = await res
      .clone()
      .text()
      .catch(() => '');
    const where = `GET ${res.url || '(url unrecorded)'} → ${res.status}, expected 200`;
    const shed = shedShape(res.status, body);
    const why = shed
      ? `${where}: ${shed}. The request never reached dispatch_to_projected_epr, so the omnibar splice this scenario pins was never exercised — the doorway/fleet is unavailable, NOT the trust surface. Re-run when the doorway is serving.`
      : `${where}. Body starts: ${body.slice(0, 240)}`;

    assert.equal(res.status, 200, why);
  }
);

Then(
  'the served page carries exactly one {string} omnibar context island',
  async function (this: OmnibarChromeWorld, islandId: string) {
    const html = await servedHtml(this);

    // Exactly one — a second injection (e.g. both serving paths splicing) would
    // mount two omnibars. The light crate's own idempotency test counts this
    // same marker; we hold the served bytes to the same invariant. Literal
    // split-count — the id is a plain string, no regex needed.
    const marker = `id="${islandId}"`;
    const occurrences = html.split(marker).length - 1;
    assert.equal(
      occurrences,
      1,
      `Expected exactly one omnibar context island (id="${islandId}"), found ${occurrences}`
    );

    // ...and it is the JSON context island inject_element() emits, not an
    // incidental id collision elsewhere in the document.
    assert.ok(
      html.includes(`<script type="application/json" id="${islandId}">`),
      `The "${islandId}" island is present but not as the application/json context island`
    );
  }
);

Then(
  'the served page references the {string} omnibar element loader',
  async function (this: OmnibarChromeWorld, loaderBase: string) {
    const html = await servedHtml(this);

    // The content-addressed, deferred loader inject_element() splices:
    //   <script src="/chrome/omni-element.<sha256hex>.js" defer></script>
    // Asserting the whole tag (not a bare substring) proves it is the live
    // loader, not a stray mention in a comment or the island JSON.
    const loader = new RegExp(
      String.raw`<script src="${escapeRegExp(loaderBase)}[0-9a-f]+\.js" defer>`
    );
    assert.match(
      html,
      loader,
      `Served HTML does not reference the deferred, content-addressed omnibar loader ("${loaderBase}<hash>.js")`
    );
  }
);
