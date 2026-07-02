/**
 * EprResolutionProvider — the protocol's single, ambient EPR resolution contract.
 *
 * One contract, provided once per surface, consumed by every link/preview call
 * site instead of each one hand-wiring its own transport decision. It has three
 * responsibilities, each honoring the increment-1 head-vs-body split:
 *
 *  - `resolveHead` — preview/link resolution. Reach-safe, anonymous-OK; hits the
 *    HEAD plane only. Previews NEVER fetch a body, so a reach-gated body 403/404
 *    can never blank a card. Degrades to a typed state (resolved / forbidden /
 *    missing / error) so the element renders an honest face, never a raw `epr:`.
 *  - `resolveRoute` — claims-aware in-bundle-vs-cross-bundle route minting for a
 *    ref (a thin wrapper over the bundle's route context). Pure/synchronous.
 *  - `resolveBody` — the explicit, separate, reach-gated body fetch, for the
 *    click-through/embed path only. May 403; the caller renders the forbidden
 *    affordance. This is the ONLY method that touches the gated body plane.
 *
 * Transport abstraction lives at THIS seam and nowhere else: the browser
 * implementation delegates to doorway HTTP; a Tauri implementation may hit the
 * local storage sidecar; a head-gossip implementation may come later. The
 * contract deliberately names no transport and no framework — it is plain TS so
 * Lit elements import it without pulling in Angular. Content addresses only:
 * a resolver never joins, mints, or reasons about an agent identity.
 *
 * Elements consume the provider ambiently via the `eprResolutionContext`
 * (@lit/context) installed at the surface root; an explicit `.resolver` prop on
 * an element still overrides context (backward compatible — the prop wins).
 *
 * Spec: genesis/docs/superpowers/specs/2026-07-02-epr-resolution-provider-design.md (I2)
 */

import { ContextProvider, createContext } from '@lit/context';

/**
 * Typed degradation outcome, shared by head and body resolution. Mirrors the
 * element's `EprLinkState` (increment 1) so outcomes flow through the provider
 * without translation:
 * - `resolved`  — reachable → full face
 * - `forbidden` — gated at the reader's reach → "unavailable at your reach"
 * - `missing`   — 404 / unresolvable → the only state that may show the raw ref
 * - `error`     — network / decode failure
 */
export type EprResolutionState = 'resolved' | 'forbidden' | 'missing' | 'error';

/**
 * Preview projection — everything a link/card renders, sourced from the
 * anonymous-safe EPR Head. No body bytes; carries route/href so navigation
 * never re-fetches to route.
 */
export interface EprHeadPreview {
  /** Stable content id (slug). */
  id: string;
  /** Human-readable title (falls back to the id when the head carries none). */
  title: string;
  /** Short description; null/absent when the head carries none. */
  description?: string | null;
  /** Access reach (commons/community/…) — governs provide/routing affordances. */
  reach?: string | null;
  /** Content type (epic/concept/path/…) — feeds claims-aware routing. */
  contentType?: string | null;
  /** Content format (markdown/sophia-quiz-json/…). */
  contentFormat?: string | null;
  /** Discovery tags. */
  tags?: string[];
  /** In-bundle router command segments; null = cross-bundle (navigate via href). */
  route?: string[] | null;
  /** Universal address — always navigable in every bundle. */
  href?: string;
}

/**
 * Discriminated outcome of a head-first resolution. The body being unreachable
 * can NEVER surface here — only the anonymous-safe HEAD failing degrades.
 * `head` is present on `resolved`, MAY be present on `forbidden` (the title and
 * reach are still known), and is absent on `missing` / `error`.
 */
export interface EprHeadResolution {
  state: EprResolutionState;
  head?: EprHeadPreview;
}

/**
 * Claims-aware route minting result — the navigable target for a ref in THIS
 * bundle. Returned by `resolveRoute`; `null` when the ref has no page address
 * (blob tier).
 */
export interface EprRouteResolution {
  /** In-bundle router command segments; null = cross-bundle (navigate via href). */
  route: string[] | null;
  /** Universal address — always navigable in every bundle. */
  href: string;
}

/**
 * Explicit, reach-gated body fetch outcome. Only produced by `resolveBody`
 * (click-through / embed). May be `forbidden` (403 at this reach) — the caller
 * renders the honest affordance rather than treating it as a preview failure.
 */
export interface EprBodyResolution {
  state: EprResolutionState;
  /** Content body (string for text/markdown; caller-owned shape otherwise). */
  body?: string | null;
  /** Content format the body is in, when known. */
  contentFormat?: string | null;
}

/**
 * The one ambient resolution contract. See the file header for the head-vs-body
 * split, transport abstraction, and content-address-only guarantees.
 */
export interface EprResolutionProvider {
  /** Preview/link resolution — reach-safe, anonymous-OK; hits the head plane. */
  resolveHead(ref: string): Promise<EprHeadResolution>;
  /** In-bundle vs cross-bundle route minting for a ref (claims-aware). */
  resolveRoute(ref: string, contentType?: string): EprRouteResolution | null;
  /** Body fetch — click-through/embed only; may 403; caller renders forbidden. */
  resolveBody(ref: string): Promise<EprBodyResolution>;
}

/**
 * @lit/context key for the ambient provider. Installed once at a surface root
 * (the shell's app root — the same root that installs the epr-link click
 * interceptor); every `<elohim-*>` element beneath it consumes resolution
 * through this context. `undefined` until a provider is installed, so an element
 * with neither an explicit `.resolver` prop nor an ambient provider degrades to
 * its id-only face rather than throwing.
 */
export const eprResolutionContext = createContext<EprResolutionProvider | undefined>(
  'elohim-epr-resolution-provider'
);

/**
 * Install the ambient EPR resolution provider on a surface root element.
 *
 * Symmetric with `installEprLinkInterceptor` — the shell root calls both, so
 * "what does resolving mean" has exactly one answer per surface. Attaches a
 * `@lit/context` provider that answers `context-request` events bubbling up from
 * any `<elohim-*>` element beneath `host`; those elements then resolve through
 * `provider` with no per-element wiring. Encapsulates `@lit/context` so
 * framework hosts (the Angular shell) never depend on it directly.
 *
 * Returns a teardown that detaches the value (elements degrade to their id-only
 * face). The host typically outlives the app, so teardown is rarely needed.
 */
export function installEprResolutionProvider(
  host: HTMLElement,
  provider: EprResolutionProvider
): () => void {
  const controller = new ContextProvider(host, {
    context: eprResolutionContext,
    initialValue: provider,
  });
  // The host is a plain element (not a ReactiveElement), so wake the controller
  // manually — it attaches its `context-request` listener on `hostConnected`.
  controller.hostConnected();
  return () => {
    controller.setValue(undefined);
  };
}
