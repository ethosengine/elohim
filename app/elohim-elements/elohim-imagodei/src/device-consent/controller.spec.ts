import { expect } from '@open-wc/testing';

import { DeviceConsentController, type DeviceConsentPageState } from './controller.js';
import { KEY_HOLDER_SIGN_STEP, NODE_CODE } from './logic.js';
import type { ConsentMemory, RememberedConsent } from './progress.js';
import {
  CONSENT_AGREE_PATH,
  CONSENT_VIEW_PATH,
  createDeviceConsentClient,
  type ConsentAgreeRequest,
  type ConsentAgreeResponse,
  type ConsentViewResponse,
  type ConsentWireResult,
  type DeviceConsentClient,
  type GrantRequestJson,
} from './wire.js';

const settle = () => new Promise<void>(resolve => setTimeout(resolve, 0));

function encode(value: unknown): string {
  return btoa(JSON.stringify(value)).replace(/\+/g, '-').replace(/\//g, '_').replace(/=/g, '');
}

const GRANT_REQUEST = { clientId: 'epr-cli', label: 'workspace', acts: ['device.enroll'] };
const PARAM = encode(GRANT_REQUEST);

const VIEW: ConsentViewResponse = {
  clientId: 'epr-cli',
  label: 'workspace',
  deviceFingerprint: 'uhCAk…3FOt',
  deviceRootFingerprint: 'uhCkk…PrcW',
  askedActs: ['device.enroll', 'device.bind-root'],
};

const answer = (partial: Partial<ConsentAgreeResponse> = {}): ConsentAgreeResponse => ({
  returnTarget: { kind: 'display', value: 'K7QF-2MXD' },
  expiresAt: Date.now() + 60_000,
  consentCid: 'bafyreiconsent',
  controllers: { required: 1, signed: 1 },
  witnesses: [{ id: 'this-device-sign', act: 'signed', relation: 'this-device', state: 'done' }],
  ...partial,
});

const ok = <T>(body: T): ConsentWireResult<T> => ({ ok: true, body });
const refused = (status: number, code?: string): ConsentWireResult<never> => ({
  ok: false,
  status,
  body: code ? { error: 'no', code } : null,
});

/** A memory that lives as long as the spec, standing in for one tab. */
function tabMemory(): ConsentMemory & { store: Map<string, RememberedConsent> } {
  const store = new Map<string, RememberedConsent>();
  return {
    store,
    recall: key => store.get(key) ?? null,
    remember: (key, value) => {
      store.set(key, structuredClone(value));
    },
    forget: key => {
      store.delete(key);
    },
  };
}

interface Calls {
  view: GrantRequestJson[];
  agree: ConsentAgreeRequest[];
  signIn: number;
  handBack: string[];
}

describe('DeviceConsentController — the approval page both portals mount', () => {
  let memory: ReturnType<typeof tabMemory>;
  let calls: Calls;
  let viewResult: () => Promise<ConsentWireResult<ConsentViewResponse>>;
  let agreeResult: () => Promise<ConsentWireResult<ConsentAgreeResponse>>;

  beforeEach(() => {
    memory = tabMemory();
    calls = { view: [], agree: [], signIn: 0, handBack: [] };
    viewResult = async () => ok(VIEW);
    agreeResult = async () => ok(answer());
  });

  function page(param: string | null = PARAM) {
    const states: DeviceConsentPageState[] = [];
    const client: DeviceConsentClient = {
      view: async request => {
        calls.view.push(request);
        return viewResult();
      },
      agree: async body => {
        calls.agree.push(body);
        return agreeResult();
      },
    };
    const controller = new DeviceConsentController({
      requestParam: param,
      holder: { relation: 'this-device' },
      client,
      memory,
      signIn: () => {
        calls.signIn++;
      },
      handBack: url => {
        calls.handBack.push(url);
      },
      onChange: state => {
        states.push(state);
      },
    });
    return { controller, states };
  }

  async function reviewing() {
    const p = page();
    await p.controller.start();
    expect(p.controller.state.phase).to.equal('review');
    return p;
  }

  describe('reading the request', () => {
    for (const [label, param] of [
      ['absent', null],
      ['not base64url', '!!!'],
      ['not JSON', 'bm90IGpzb24'],
    ] as const) {
      it(`refuses an ${label} request without calling the node`, async () => {
        const { controller } = page(param);
        await controller.start();
        expect(calls.view).to.have.length(0);
        expect(controller.state.phase).to.equal('refused');
        expect(controller.state.refusalCode).to.equal('request_unreadable');
      });
    }

    it('asks the node with the request exactly as the link carried it', async () => {
      const { controller } = await reviewing();
      expect(calls.view).to.deep.equal([GRANT_REQUEST]);
      expect(controller.state.view).to.deep.equal(VIEW);
      expect(controller.state.trail).to.equal(null);
    });

    it('shows the node’s refusal and does not remember it', async () => {
      viewResult = async () => refused(400, 'request_acts_incoherent');
      const { controller } = page();
      await controller.start();
      expect(controller.state.refusalCode).to.equal('request_acts_incoherent');
      expect(memory.store.size).to.equal(0);
    });

    it('sends the person to sign in when the node says no one is signed in', async () => {
      viewResult = async () => refused(403, NODE_CODE.notSignedIn);
      const { controller } = page();
      await controller.start();
      expect(calls.signIn).to.equal(1);
      expect(controller.state.refusalCode).to.equal(NODE_CODE.notSignedIn);
    });
  });

  describe('approving', () => {
    it('shows a wait on the key holder while it signs, and nothing else', async () => {
      let release!: (r: ConsentWireResult<ConsentAgreeResponse>) => void;
      agreeResult = () => new Promise(resolve => (release = resolve));
      const { controller } = await reviewing();

      const done = controller.approve({ agreedActs: ['device.enroll'] });
      expect(controller.state.phase).to.equal('signing');
      expect(controller.state.trail).to.deep.equal([
        { id: KEY_HOLDER_SIGN_STEP, act: 'signed', relation: 'this-device', state: 'working' },
      ]);
      release(ok(answer()));
      await done;
    });

    it('signs only what was agreed and shows the code, with the node’s witnesses and count', async () => {
      const response = answer();
      agreeResult = async () => ok(response);
      const { controller } = await reviewing();
      await controller.approve({ agreedActs: ['device.enroll'] });

      expect(calls.agree).to.deep.equal([
        { request: GRANT_REQUEST, agreedActs: ['device.enroll'] },
      ]);
      expect(controller.state.phase).to.equal('code');
      expect(controller.state.code).to.equal('K7QF-2MXD');
      expect(controller.state.expiresAt).to.equal(response.expiresAt);
      expect(controller.state.trail).to.deep.equal(response.witnesses);
      expect(controller.state.standing).to.deep.equal({ kind: 'single' });
      expect(calls.handBack).to.have.length(0);
    });

    it('hands the code to a terminal on this machine', async () => {
      const url = 'http://127.0.0.1:53682/callback?code=c0de&state=s';
      agreeResult = async () => ok(answer({ returnTarget: { kind: 'redirect', url } }));
      const { controller } = await reviewing();
      await controller.approve({ agreedActs: ['device.enroll'] });
      expect(controller.state.phase).to.equal('handed-back');
      expect(calls.handBack).to.deep.equal([url]);
    });

    it('never follows a redirect to anywhere but this machine’s terminal', async () => {
      agreeResult = async () =>
        ok(answer({ returnTarget: { kind: 'redirect', url: 'https://evil.example/collect' } }));
      const { controller } = await reviewing();
      await controller.approve({ agreedActs: ['device.enroll'] });
      expect(calls.handBack).to.have.length(0);
      expect(controller.state.refusalCode).to.equal('return_path_refused');
    });

    it('sends one approval even if approve fires twice', async () => {
      const { controller } = await reviewing();
      await Promise.all([
        controller.approve({ agreedActs: ['device.enroll'] }),
        controller.approve({ agreedActs: ['device.enroll'] }),
      ]);
      expect(calls.agree).to.have.length(1);
    });

    it('reports a quorum the person set up that is not yet met, and still shows the code', async () => {
      agreeResult = async () => ok(answer({ controllers: { required: 2, signed: 1 } }));
      const { controller } = await reviewing();
      await controller.approve({ agreedActs: ['device.enroll'] });
      expect(controller.state.phase).to.equal('code');
      expect(controller.state.standing).to.deep.equal({
        kind: 'short',
        more: 1,
        required: 2,
        signed: 1,
      });
    });

    it('marks the key holder failed on a refusal and remembers it', async () => {
      agreeResult = async () => refused(400, 'act_unknown');
      const { controller } = await reviewing();
      await controller.approve({ agreedActs: ['device.enroll'] });
      expect(controller.state.refusalCode).to.equal('act_unknown');
      expect(controller.state.trail?.[0]?.state).to.equal('failed');
      expect(memory.recall(PARAM)?.outcome).to.deep.equal({
        phase: 'refused',
        code: 'act_unknown',
      });
    });

    for (const code of [NODE_CODE.signingUnavailable, NODE_CODE.identityUnbootstrapped]) {
      it(`on ${code}: says so, shows no witness, and lets the link ask again later`, async () => {
        agreeResult = async () => refused(409, code);
        const first = await reviewing();
        await first.controller.approve({ agreedActs: ['device.enroll'] });
        expect(first.controller.state.refusalCode).to.equal(code);
        expect(first.controller.state.trail).to.equal(null);
        expect(memory.store.size).to.equal(0);

        const again = page();
        await again.controller.start();
        expect(again.controller.state.phase).to.equal('review');
      });
    }

    it('when a witness asks the person to sign in again: says why, sends them only when they choose', async () => {
      agreeResult = async () => ({
        ok: false,
        status: 401,
        body: {
          error: 'sign in again',
          code: NODE_CODE.reauthenticationAsked,
          reason: '  It has been a while since you signed in on this device.  ',
        },
      });
      const { controller } = await reviewing();
      await controller.approve({ agreedActs: ['device.enroll'] });
      expect(controller.state.phase).to.equal('refused');
      expect(controller.state.refusalCode).to.equal('consent_reauthentication_asked');
      expect(controller.state.refusalReason).to.equal(
        'It has been a while since you signed in on this device.'
      );
      expect(controller.state.trail).to.equal(null);
      expect(calls.signIn).to.equal(0);
      expect(memory.store.size).to.equal(0);

      controller.signInAgain();
      expect(calls.signIn).to.equal(1);

      // Back from signing in: the same link asks again; nothing was resent.
      const again = page();
      await again.controller.start();
      expect(again.controller.state.phase).to.equal('review');
      expect(calls.agree).to.have.length(1);
    });

    it('offers signing in again only after a witness asked for it', async () => {
      const { controller } = await reviewing();
      controller.signInAgain();
      expect(calls.signIn).to.equal(0);
    });

    it('on consent_caller_not_local: says so, shows no witness, keeps nothing', async () => {
      agreeResult = async () => refused(403, NODE_CODE.callerNotLocal);
      const { controller } = await reviewing();
      await controller.approve({ agreedActs: ['device.enroll'] });
      expect(controller.state.refusalCode).to.equal('consent_caller_not_local');
      expect(controller.state.trail).to.equal(null);
      expect(memory.store.size).to.equal(0);
    });

    it('comes back to review after beginning, and sends nothing until approved again', async () => {
      agreeResult = async () => refused(409, NODE_CODE.identityUnbootstrapped);
      const { controller } = await reviewing();
      await controller.approve({ agreedActs: ['device.enroll'] });
      expect(controller.state.refusalCode).to.equal(NODE_CODE.identityUnbootstrapped);

      await controller.resume();
      expect(controller.state.phase).to.equal('review');
      expect(controller.state.refusalCode).to.equal(undefined);
      expect(calls.view).to.have.length(1);
      expect(calls.agree).to.have.length(1);

      agreeResult = async () => ok(answer());
      await controller.approve({ agreedActs: ['device.enroll'] });
      expect(controller.state.phase).to.equal('code');
      expect(calls.agree).to.have.length(2);
    });

    it('does not resume past a refusal after which something may have been signed', async () => {
      agreeResult = async () => refused(400, 'act_unknown');
      const { controller } = await reviewing();
      await controller.approve({ agreedActs: ['device.enroll'] });
      await controller.resume();
      expect(controller.state.phase).to.equal('refused');
      expect(controller.state.refusalCode).to.equal('act_unknown');
    });

    it('on no one signed in: forgets the attempt and sends the person to sign in', async () => {
      agreeResult = async () => refused(401);
      const { controller } = await reviewing();
      await controller.approve({ agreedActs: ['device.enroll'] });
      expect(calls.signIn).to.equal(1);
      expect(controller.state.refusalCode).to.equal(NODE_CODE.notSignedIn);
      expect(controller.state.trail).to.equal(null);
      expect(memory.store.size).to.equal(0);
    });

    it('treats a client that throws as no answer', async () => {
      agreeResult = () => Promise.reject(new Error('boom'));
      const { controller } = await reviewing();
      await controller.approve({ agreedActs: ['device.enroll'] });
      expect(controller.state.refusalCode).to.equal('consent_unavailable');
    });
  });

  it('declining sends nothing and shows no code', async () => {
    const { controller } = await reviewing();
    controller.decline();
    expect(controller.state.phase).to.equal('declined');
    expect(controller.state.code).to.equal(undefined);
    expect(controller.state.refusalCode).to.equal(undefined);
    expect(calls.agree).to.have.length(0);
  });

  describe('leaving and coming back', () => {
    it('shows the same code, trail and count again without asking or approving again', async () => {
      const first = await reviewing();
      await first.controller.approve({ agreedActs: ['device.enroll'] });

      const again = page();
      await again.controller.start();
      expect(calls.view).to.have.length(1);
      expect(calls.agree).to.have.length(1);
      expect(again.controller.state.phase).to.equal('code');
      expect(again.controller.state.code).to.equal('K7QF-2MXD');
      expect(again.controller.state.trail).to.deep.equal(first.controller.state.trail);
      expect(again.controller.state.standing).to.deep.equal({ kind: 'single' });
    });

    it('does not re-submit an approval that was still being signed', async () => {
      agreeResult = () => new Promise(() => undefined); // never answers
      const first = await reviewing();
      first.controller.approve({ agreedActs: ['device.enroll'] }).catch(() => undefined);
      await settle();

      const again = page();
      await again.controller.start();
      expect(again.controller.state.refusalCode).to.equal('approval_interrupted');
      await again.controller.approve({ agreedActs: ['device.enroll'] });
      expect(calls.agree).to.have.length(1);
    });

    it('remembers a decline', async () => {
      const first = await reviewing();
      first.controller.decline();
      const again = page();
      await again.controller.start();
      expect(again.controller.state.phase).to.equal('declined');
    });

    it('forgets the code itself once it has expired', async () => {
      const first = await reviewing();
      await first.controller.approve({ agreedActs: ['device.enroll'] });
      first.controller.expired();
      expect(memory.recall(PARAM)?.outcome).to.deep.equal({
        phase: 'code',
        expiresAt: first.controller.state.expiresAt,
      });
    });
  });
});

describe('createDeviceConsentClient — same origin, nothing else', () => {
  interface Seen {
    url: string;
    init: RequestInit;
  }

  function client(reply: () => Promise<Response>, headers?: () => Record<string, string>) {
    const seen: Seen[] = [];
    const fetchFn = (async (url: string, init: RequestInit) => {
      seen.push({ url, init });
      return reply();
    }) as unknown as typeof fetch;
    return { seen, wire: createDeviceConsentClient({ fetch: fetchFn, headers }) };
  }

  const json = (status: number, body: unknown) =>
    new Response(JSON.stringify(body), {
      status,
      headers: { 'content-type': 'application/json' },
    });

  it('posts the request unchanged to the same-origin view path', async () => {
    const { seen, wire } = client(async () => json(200, VIEW));
    const result = await wire.view(GRANT_REQUEST);
    expect(result).to.deep.equal({ ok: true, body: VIEW });
    expect(seen[0]!.url).to.equal(CONSENT_VIEW_PATH);
    expect(seen[0]!.init.method).to.equal('POST');
    expect(seen[0]!.init.credentials).to.equal('same-origin');
    expect(JSON.parse(seen[0]!.init.body as string)).to.deep.equal(GRANT_REQUEST);
  });

  it('posts the request and the agreed acts to the agree path, with the host’s headers', async () => {
    const { seen, wire } = client(
      async () => json(200, answer()),
      () => ({ Authorization: 'Bearer t0ken' })
    );
    await wire.agree({ request: GRANT_REQUEST, agreedActs: ['device.enroll'] });
    expect(seen[0]!.url).to.equal(CONSENT_AGREE_PATH);
    expect((seen[0]!.init.headers as Record<string, string>)['Authorization']).to.equal(
      'Bearer t0ken'
    );
  });

  it('returns a refusal’s status and body', async () => {
    const { wire } = client(async () =>
      json(409, { error: 'x', code: 'consent_signing_unavailable' })
    );
    expect(await wire.view(GRANT_REQUEST)).to.deep.equal({
      ok: false,
      status: 409,
      body: { error: 'x', code: 'consent_signing_unavailable' },
    });
  });

  it('reports no answer as status 0', async () => {
    const { wire } = client(() => Promise.reject(new TypeError('offline')));
    expect(await wire.view(GRANT_REQUEST)).to.deep.equal({ ok: false, status: 0, body: null });
  });
});
