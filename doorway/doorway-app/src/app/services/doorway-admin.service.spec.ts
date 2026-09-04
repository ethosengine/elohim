import { describe, expect, it } from 'vitest';

import { isExpectedSocketClose } from './doorway-admin.service';

/**
 * After "Logout through the browser UI" the console printed
 * `[DoorwayAdmin] WebSocket error: CloseEvent` on /threshold/ — and the a2o
 * After-hook fails a scenario on any console error. The socket had not failed;
 * it ended, because the human left. Only genuinely unexpected closes stay
 * errors.
 */
describe('isExpectedSocketClose', () => {
  it('treats anything after our own disconnect() as expected', () => {
    // Once disconnect() has run, nothing the socket reports is news — whatever
    // shape it arrives in.
    expect(isExpectedSocketClose({ code: 1006 }, true)).toBe(true);
    expect(isExpectedSocketClose(new Error('boom'), true)).toBe(true);
    expect(isExpectedSocketClose(undefined, true)).toBe(true);
  });

  it('treats normal / going-away / no-status closes as expected', () => {
    expect(isExpectedSocketClose({ code: 1000 }, false)).toBe(true); // normal
    expect(isExpectedSocketClose({ code: 1001 }, false)).toBe(true); // navigating away
    expect(isExpectedSocketClose({ code: 1005 }, false)).toBe(true); // no status
  });

  it('still reports a genuinely unexpected close as an error', () => {
    expect(isExpectedSocketClose({ code: 1006 }, false)).toBe(false); // abnormal
    expect(isExpectedSocketClose({ code: 1011 }, false)).toBe(false); // server error
    expect(isExpectedSocketClose(new Error('connection refused'), false)).toBe(false);
    expect(isExpectedSocketClose(null, false)).toBe(false);
    expect(isExpectedSocketClose('CloseEvent', false)).toBe(false);
  });
});
