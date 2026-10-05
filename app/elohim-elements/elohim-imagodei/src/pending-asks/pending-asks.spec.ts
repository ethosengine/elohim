import { expect } from '@open-wc/testing';

import { DeviceConsentController } from '../device-consent/controller.js';
import type { ConsentMemory } from '../device-consent/progress.js';

import {
  PendingAsksController,
  pendingAskConsentClient,
  pendingAskRequestParam,
} from './controller.js';
import { approvalIsFor, listsNothingLine, timeLeft, whoFor } from './words.js';
import type {
  PendingAskView,
  PendingAsksClient,
  PendingAsksView,
  PendingDecideRequest,
} from './wire.js';

const PERSON = {
  identifier: 'matthew',
  humanId: 'h-4c1d',
  displayName: 'Matthew',
  identityRoot: 'uhCkkroot',
  identityFingerprint: 'uhCkk…root',
};

const ASK: PendingAskView = {
  number: 3,
  label: 'laptop',
  deviceKey: 'uhCAkdevice',
  deviceFingerprint: 'uhCAk…3FOt',
  askedActs: ['device.enroll', 'device.bind-root'],
  deviceRootFingerprint: 'uhCkk…PrcW',
  secondsLeft: 250,
  state: { kind: 'unassigned' },
  stateWords: 'This node has no identity yet.',
  addressedHere: true,
  forIdentity: PERSON,
};

const LISTED: PendingAsksView = {
  carrier: 'private-network',
  approver: 'uhCAkme',
  speaksFor: { kind: 'person', ...PERSON },
  speaksForWords: 'An approval here is for matthew (Matthew), identity uhCkk…root.',
  asks: [ASK],
};

const AGREED = {
  returnTarget: { kind: 'display' as const, value: 'K7QF-2MXD#state' },
  expiresAt: Date.now() + 60_000,
  consentCid: 'bafyreiconsent',
  controllers: { required: 1, signed: 1 },
  witnesses: [],
};

function client(
  list: unknown,
  decide: unknown = {
    ok: true,
    body: { number: 3, decidedBy: 'answer', agreed: AGREED, handedBack: { taken: true } },
  }
) {
  const decided: PendingDecideRequest[] = [];
  const c: PendingAsksClient = {
    list: async () => list as never,
    decide: async body => {
      decided.push(body);
      return decide as never;
    },
  };
  return { c, decided };
}

const memory = (): ConsentMemory => ({
  recall: () => null,
  remember: () => undefined,
  forget: () => undefined,
});

describe('PendingAsksController — devices asking over a private network', () => {
  async function phaseFor(list: unknown) {
    const ctl = new PendingAsksController({ client: client(list).c, onChange: () => undefined });
    await ctl.read();
    return ctl.state.phase;
  }

  it('lists the asks for a node that speaks for a person', async () => {
    expect(await phaseFor({ ok: true, body: LISTED })).to.equal('listed');
  });

  it('lists nothing, with one line, for a node that speaks for nobody or cannot say', async () => {
    for (const kind of ['nobody', 'unknown'] as const) {
      expect(
        await phaseFor({ ok: true, body: { ...LISTED, speaksFor: { kind }, asks: [] } })
      ).to.equal('lists-nothing');
    }
  });

  it('says nothing without a private network, or without an answer', async () => {
    expect(await phaseFor({ ok: true, body: { ...LISTED, carrier: 'absent' } })).to.equal('absent');
    expect(await phaseFor({ ok: false, status: 404, body: null })).to.equal('unavailable');
  });
});

describe('deciding an ask with the approval page’s own card and controller', () => {
  function review(decide?: unknown) {
    const { c, decided } = client({ ok: true, body: LISTED }, decide);
    const handBacks: string[] = [];
    const ctl = new DeviceConsentController({
      requestParam: pendingAskRequestParam(ASK),
      holder: { relation: 'this-device' },
      client: pendingAskConsentClient(c, ASK),
      memory: memory(),
      signIn: () => undefined,
      handBack: url => {
        handBacks.push(url);
      },
      onChange: () => undefined,
    });
    return { ctl, decided, handBacks };
  }

  it('shows what the node listed without asking it again', async () => {
    const { ctl, decided } = review();
    await ctl.start();
    expect(ctl.state.phase).to.equal('review');
    expect(ctl.state.view).to.deep.include({ label: 'laptop', deviceFingerprint: 'uhCAk…3FOt' });
    expect(decided).to.have.length(0);
  });

  it('decides with only the acts the person agreed to, and reads a delivered code as handed back', async () => {
    const { ctl, decided, handBacks } = review();
    await ctl.start();
    await ctl.approve({ agreedActs: ['device.enroll'] });
    expect(decided).to.deep.equal([{ ask: '3', answer: { agreedActs: ['device.enroll'] } }]);
    expect(ctl.state.phase).to.equal('handed-back');
    expect(handBacks).to.have.length(0);
  });

  it('shows the code to type in when the node could not hand it back', async () => {
    const { ctl } = review({
      ok: true,
      body: {
        number: 3,
        decidedBy: 'answer',
        agreed: AGREED,
        handedBack: { taken: false, error: 'gone' },
      },
    });
    await ctl.start();
    await ctl.approve({ agreedActs: ['device.enroll'] });
    expect(ctl.state.phase).to.equal('code');
    expect(ctl.state.code).to.equal('K7QF-2MXD#state');
  });

  it('declining makes no call', async () => {
    const { ctl, decided } = review();
    await ctl.start();
    ctl.decline();
    expect(ctl.state.phase).to.equal('declined');
    expect(decided).to.have.length(0);
  });
});

describe('the panel’s words', () => {
  it('leads with the sign-in word and never shows the record id', () => {
    expect(whoFor(PERSON)).to.equal('⁨matthew (Matthew)⁩');
    expect(whoFor({ ...PERSON, identifier: undefined })).to.equal('⁨Matthew⁩');
    expect(whoFor({ ...PERSON, identifier: undefined, displayName: undefined })).to.equal(
      undefined
    );
    expect(approvalIsFor(PERSON)).to.equal(
      'An approval here is for ⁨matthew (Matthew)⁩, identity uhCkk…root.'
    );
    expect(
      approvalIsFor({ ...PERSON, identifier: undefined, displayName: undefined })
    ).not.to.include('h-4c1d');
  });

  it('gives a node that lists nothing its own line', () => {
    const nobody = {
      ...LISTED,
      speaksFor: { kind: 'nobody' as const },
      speaksForWords: 'This node speaks for nobody.',
    };
    expect(listsNothingLine(nobody)).to.equal('This node speaks for nobody.');
    expect(listsNothingLine(LISTED)).to.equal(undefined);
  });

  it('says how long is left plainly', () => {
    expect(timeLeft(250)).to.equal('About 4 minutes left');
    expect(timeLeft(70)).to.equal('About 1 minute left');
    expect(timeLeft(40)).to.equal('40 seconds left');
    expect(timeLeft(0)).to.equal('No time left');
  });
});
