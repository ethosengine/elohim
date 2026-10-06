/**
 * How the native portal says what an identity rests on. The page may be
 * open on another machine (signed in over a session), where "this device"
 * would read as the browser's own machine; so the device is named from the
 * node's side, which is true wherever the page is open.
 */

import type { StandingHostWords } from 'elohim-imagodei/identity-standing';

export const NODE_SIDE: StandingHostWords = {
  inSentence: 'the device that holds your key',
  alone: 'one device alone: the device that holds your key',
  notThisOne: 'it is not the device serving this page',
};
