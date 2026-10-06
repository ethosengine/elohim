import { makeProof } from './session-key/proof.js';

import type { SessionSigner } from './session-key/signer.js';

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
  /**
   * What to prove a POST with (this browser's session key), or none. Given,
   * every POST carries a `DPoP` proof over the exact body bytes it sends.
   */
  prove?: () => Promise<SessionSigner | null>;
  /** Defaults to the global `fetch`. */
  fetch?: typeof fetch;
  /**
   * Extra headers per call — how a host that keeps its session outside a
   * cookie (a bearer token) proves it. Cookies on this origin are always sent.
   */
  headers?: () => Record<string, string>;
}

const ENCODER = new TextEncoder();

/** The proof refusal after which one fresh proof is worth sending (clock skew). */
const STALE = 'session_proof_stale';

/** The request URL a proof names: absolute, as sent, without a fragment. */
function requestUrl(path: string): string {
  const url = new URL(path, globalThis.location?.href ?? 'http://localhost/');
  url.hash = '';
  return url.href;
}

const codeOf = (payload: unknown): unknown => (payload as { code?: unknown } | null)?.code;

/**
 * GET `path`, or POST `body` as JSON to it, on this page's own origin. The
 * body is serialised once; that string is what is hashed into the proof and
 * what is sent. A proof the node calls stale is made afresh and sent once
 * more — never again, and no other refusal is retried.
 */
export async function sameOriginJson<T>(
  options: SameOriginOptions,
  method: 'GET' | 'POST',
  path: string,
  body?: unknown,
  extraHeaders?: Record<string, string>
): Promise<SameOriginResult<T>> {
  const doFetch = options.fetch ?? globalThis.fetch.bind(globalThis);
  const text = method === 'POST' && body !== undefined ? JSON.stringify(body) : '';
  const signer =
    method === 'POST' && options.prove ? await options.prove().catch(() => null) : null;

  const send = async (): Promise<{ response: Response; payload: unknown } | null> => {
    const headers: Record<string, string> = { ...options.headers?.(), ...extraHeaders };
    if (signer) {
      headers['DPoP'] = await makeProof(signer, {
        htm: method,
        htu: requestUrl(path),
        body: ENCODER.encode(text),
      });
    }
    if (method === 'POST' && text) headers['Content-Type'] = 'application/json';
    try {
      const response = await doFetch(path, {
        method,
        credentials: 'same-origin',
        headers,
        ...(method === 'POST' && text ? { body: text } : {}),
      });
      return { response, payload: await response.json().catch(() => null) };
    } catch {
      return null;
    }
  };

  let answer = await send();
  if (signer && answer?.response.status === 401 && codeOf(answer.payload) === STALE) {
    answer = await send();
  }
  if (!answer) return { ok: false, status: 0, body: null };
  const { response, payload } = answer;
  return response.ok
    ? { ok: true, body: payload as T }
    : { ok: false, status: response.status, body: payload };
}
