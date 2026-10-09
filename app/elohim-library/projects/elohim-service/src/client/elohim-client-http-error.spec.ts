/**
 * ElohimHttpError — a non-2xx read keeps its status and body, so a consumer can tell a reach
 * hold (403 `{"requiredReach":…}`) from an outage instead of flattening both into "not found".
 */
import { describe, expect, it } from 'vitest';

import { ElohimHttpError } from './elohim-client';

describe('ElohimHttpError', () => {
  it('carries status, body and the reach a 403 names', () => {
    const body = '{"error":"Authentication required","requiredReach":"private"}';
    const err = new ElohimHttpError(403, body);
    expect(err).toBeInstanceOf(Error);
    expect(err.name).toBe('ElohimHttpError');
    expect(err.status).toBe(403);
    expect(err.body).toBe(body);
    expect(err.message).toBe(`HTTP 403 - ${body}`);
    expect(err.requiredReach).toBe('private');
  });

  it('names no reach for a body that is not JSON, not an object, or names none', () => {
    expect(new ElohimHttpError(502, 'bad gateway').requiredReach).toBeUndefined();
    expect(new ElohimHttpError(403, '{"error":"forbidden"}').requiredReach).toBeUndefined();
    expect(new ElohimHttpError(500, '[]').requiredReach).toBeUndefined();
  });
});
