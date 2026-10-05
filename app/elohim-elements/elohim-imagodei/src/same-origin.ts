/**
 * One JSON call to the node that served this page, at a same-origin path.
 * The shared wires (device approval, identity standing) go through it, so
 * no shared piece ever names a host.
 *
 * Framework-free and Lit-free.
 */

/**
 * What a call came back with. `status` 0 means no answer at all (offline,
 * blocked, the node unreachable); `body` is whatever JSON the refusal
 * carried, or null.
 */
export type SameOriginResult<T> =
  | { ok: true; body: T }
  | { ok: false; status: number; body: unknown };

export interface SameOriginOptions {
  /** Defaults to the global `fetch`. */
  fetch?: typeof fetch;
  /**
   * Extra headers per call — how a host that keeps its session outside a
   * cookie (a bearer token) proves it. Cookies on this origin are always sent.
   */
  headers?: () => Record<string, string>;
}

/** GET `path`, or POST `body` as JSON to it, on this page's own origin. */
export async function sameOriginJson<T>(
  options: SameOriginOptions,
  method: 'GET' | 'POST',
  path: string,
  body?: unknown
): Promise<SameOriginResult<T>> {
  const doFetch = options.fetch ?? globalThis.fetch.bind(globalThis);
  const extra = options.headers?.() ?? {};
  let response: Response;
  try {
    response = await doFetch(
      path,
      method === 'POST'
        ? {
            method,
            credentials: 'same-origin',
            headers: { 'Content-Type': 'application/json', ...extra },
            body: JSON.stringify(body),
          }
        : { method, credentials: 'same-origin', headers: extra }
    );
  } catch {
    return { ok: false, status: 0, body: null };
  }
  const payload: unknown = await response.json().catch(() => null);
  return response.ok
    ? { ok: true, body: payload as T }
    : { ok: false, status: response.status, body: payload };
}
