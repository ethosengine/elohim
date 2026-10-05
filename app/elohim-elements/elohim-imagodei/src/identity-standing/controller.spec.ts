import { expect } from '@open-wc/testing';

import type { SameOriginResult } from '../same-origin.js';
import { beginCommand, shellQuote } from '../terminal.js';

import { BEGIN_UNAVAILABLE, IdentityStandingController } from './controller.js';
import { IDENTITY_CODE, identityFailureFor, isStandingView } from './logic.js';
import { standingLine } from './words.js';
import {
  IDENTITY_BEGIN_PATH,
  IDENTITY_STANDING_PATH,
  createIdentityStandingClient,
  type IdentityBeginRequest,
  type IdentityBeginResponse,
  type IdentityStandingClient,
  type IdentityStandingView,
} from './wire.js';

const ALONE: IdentityStandingView = {
  identityRoot: 'uhCAkroot',
  authority: 'uhCEkauthority',
  networkDna: 'uhC0kdna',
  controllers: ['uhCAkworkspace'],
  controllerCount: 1,
  required: 1,
  thisNodeIsController: true,
  restsOnThisNodeAlone: true,
};

const SEVERAL: IdentityStandingView = {
  ...ALONE,
  controllers: ['uhCAkworkspace', 'uhCAkhome', 'uhCAklaptop'],
  controllerCount: 3,
  restsOnThisNodeAlone: false,
};

type Result<T> = SameOriginResult<T>;

const ok = <T>(body: T): Result<T> => ({ ok: true, body });
const refused = (status: number, code: string): Result<never> => ({
  ok: false,
  status,
  body: { error: 'no', code },
});

const BEGUN: IdentityBeginResponse = {
  standing: ALONE,
  session: { id: 'sess-1', humanId: 'h-1', identifier: 'h-1' },
  created: { human: true, authority: true, session: true },
};

describe('IdentityStandingController — what an identity rests on, and beginning it', () => {
  let begins: IdentityBeginRequest[];
  let standingResult: () => Promise<Result<IdentityStandingView>>;
  let beginResult: () => Promise<Result<IdentityBeginResponse>>;

  beforeEach(() => {
    begins = [];
    standingResult = async () => ok(ALONE);
    beginResult = async () => ok(BEGUN);
  });

  function screen() {
    const client: IdentityStandingClient = {
      standing: async () => standingResult(),
      begin: async body => {
        begins.push(body);
        return beginResult();
      },
    };
    return new IdentityStandingController({ client, onChange: () => undefined });
  }

  it('reads what the identity rests on', async () => {
    const c = screen();
    await c.read();
    expect(c.state.phase).to.equal('standing');
    expect(c.state.standing).to.deep.equal(ALONE);
  });

  it('offers to begin when the node has no identity yet', async () => {
    standingResult = async () => refused(409, IDENTITY_CODE.unbootstrapped);
    const c = screen();
    await c.read();
    expect(c.state.phase).to.equal('begin');
  });

  it('tells no one is signed in apart from no identity', async () => {
    standingResult = async () => refused(401, IDENTITY_CODE.notSignedIn);
    const c = screen();
    await c.read();
    expect(c.state.phase).to.equal('not-signed-in');
  });

  it('says nothing when the host does not answer these routes', async () => {
    for (const r of [
      { ok: false as const, status: 404, body: null },
      { ok: false as const, status: 0, body: null },
      ok({ not: 'a standing view' }),
    ]) {
      standingResult = async () => r as Result<IdentityStandingView>;
      const c = screen();
      await c.read();
      expect(c.state.phase).to.equal('unavailable');
    }
  });

  it('keeps a refusal in the node’s own words', async () => {
    standingResult = async () => refused(503, IDENTITY_CODE.signingUnavailable);
    const c = screen();
    await c.read();
    expect(c.state).to.include({ phase: 'refused', refusalCode: 'consent_signing_unavailable' });
  });

  it('begins with only the name, then shows what the identity rests on', async () => {
    standingResult = async () => refused(409, IDENTITY_CODE.unbootstrapped);
    const c = screen();
    await c.read();
    expect(await c.begin('  Matthew  ')).to.equal(true);
    expect(begins).to.deep.equal([{ displayName: 'Matthew' }]);
    expect(c.state.phase).to.equal('standing');
    expect(c.state.standing).to.deep.equal(ALONE);
    expect(c.state.created).to.deep.equal(BEGUN.created);
  });

  it('shows a wait while the key is being made, and never sends a begin twice', async () => {
    let release!: (r: Result<IdentityBeginResponse>) => void;
    beginResult = () => new Promise(resolve => (release = resolve));
    const c = screen();
    c.offerBegin();
    const first = c.begin('Matthew');
    expect(c.state.phase).to.equal('beginning');
    expect(await c.begin('Matthew')).to.equal(false);
    release(ok(BEGUN));
    await first;
    expect(begins).to.have.length(1);
    expect(await c.begin('Matthew')).to.equal(false);
    expect(begins).to.have.length(1);
  });

  it('sends nothing for a blank name', async () => {
    const c = screen();
    c.offerBegin();
    expect(await c.begin('   ')).to.equal(false);
    expect(begins).to.have.length(0);
    expect(c.state).to.include({ phase: 'begin', beginRefusal: IDENTITY_CODE.nameMalformed });
  });

  for (const code of [
    IDENTITY_CODE.nameMalformed,
    IDENTITY_CODE.callerNotLocal,
    IDENTITY_CODE.signingUnavailable,
    IDENTITY_CODE.originRefused,
  ]) {
    it(`keeps the form, with the reason, on ${code}`, async () => {
      beginResult = async () => refused(code === IDENTITY_CODE.nameMalformed ? 400 : 403, code);
      const c = screen();
      c.offerBegin();
      expect(await c.begin('Matthew')).to.equal(false);
      expect(c.state).to.include({ phase: 'begin', beginRefusal: code, displayName: 'Matthew' });
    });
  }

  it('treats a begin with no usable answer as unavailable, and a throwing client too', async () => {
    beginResult = async () => ({ ok: false, status: 404, body: null });
    const c = screen();
    c.offerBegin();
    await c.begin('Matthew');
    expect(c.state.beginRefusal).to.equal(BEGIN_UNAVAILABLE);
    beginResult = () => Promise.reject(new Error('boom'));
    await c.begin('Matthew');
    expect(c.state.beginRefusal).to.equal(BEGIN_UNAVAILABLE);
  });

  it('does not offer to begin over an identity that exists', async () => {
    const c = screen();
    await c.read();
    c.offerBegin();
    expect(c.state.phase).to.equal('standing');
  });
});

describe('identity standing rules', () => {
  it('reads only refusals in the node’s own words', () => {
    expect(
      identityFailureFor({ ok: false, status: 403, body: { code: 'consent_caller_not_local' } })
    ).to.deep.equal({
      kind: 'refused',
      code: 'consent_caller_not_local',
    });
    expect(
      identityFailureFor({ ok: false, status: 403, body: { code: 'something_else' } })
    ).to.deep.equal({
      kind: 'unavailable',
    });
    expect(identityFailureFor({ ok: false, status: 404, body: '<html>' })).to.deep.equal({
      kind: 'unavailable',
    });
  });

  it('needs the counts a line is built from', () => {
    expect(isStandingView(ALONE)).to.equal(true);
    expect(isStandingView({ ...ALONE, controllerCount: '1' })).to.equal(false);
    expect(isStandingView(null)).to.equal(false);
  });
});

describe('standingLine — a fact, never a warning', () => {
  const words = { inSentence: 'this device' };

  it('one steward, this one', () => {
    expect(standingLine(ALONE, words)).to.equal('Your identity rests on this device alone.');
  });

  it('one steward, another one', () => {
    expect(
      standingLine({ ...ALONE, thisNodeIsController: false, restsOnThisNodeAlone: false }, words)
    ).to.equal('Your identity rests on one of your own devices, not on this device.');
  });

  it('several stewards: any one of them can approve', () => {
    expect(standingLine(SEVERAL, words)).to.equal(
      'Your identity rests on 3 of your own devices, this device among them. Any one of them can approve a new device.'
    );
  });

  it('several stewards and a quorum the person set up', () => {
    expect(standingLine({ ...SEVERAL, required: 2 }, words)).to.equal(
      'Your identity rests on 3 of your own devices, this device among them. 2 of them must agree to approve a new device.'
    );
  });

  it('never reads as a shortfall, and says nothing without stewards', () => {
    for (const v of [ALONE, SEVERAL]) {
      expect(standingLine(v, words)).not.to.match(/only|warning|not enough|risk|backup/i);
    }
    expect(standingLine({ ...ALONE, controllers: [], controllerCount: 0 }, words)).to.equal(
      undefined
    );
    expect(standingLine(null, words)).to.equal(undefined);
  });
});

describe('the terminal commands to copy', () => {
  it('quotes the name as one shell word', () => {
    expect(beginCommand('Matthew')).to.equal("epr identity begin --name 'Matthew'");
    expect(beginCommand("O'Brien")).to.equal(String.raw`epr identity begin --name 'O'\''Brien'`);
    expect(shellQuote('a b')).to.equal("'a b'");
  });
});

describe('createIdentityStandingClient — same origin, nothing else', () => {
  it('GETs standing and POSTs begin as JSON to the node that served the page', async () => {
    const seen: { url: string; init: RequestInit }[] = [];
    const fetchFn = (async (url: string, init: RequestInit) => {
      seen.push({ url, init });
      return new Response(JSON.stringify(url.endsWith('begin') ? BEGUN : ALONE), {
        status: url.endsWith('begin') ? 201 : 200,
        headers: { 'content-type': 'application/json' },
      });
    }) as unknown as typeof fetch;
    const client = createIdentityStandingClient({ fetch: fetchFn });

    expect(await client.standing()).to.deep.equal({ ok: true, body: ALONE });
    expect(await client.begin({ displayName: 'Matthew' })).to.deep.equal({ ok: true, body: BEGUN });
    expect(seen[0]!.url).to.equal(IDENTITY_STANDING_PATH);
    expect(seen[0]!.init.method).to.equal('GET');
    expect(seen[1]!.url).to.equal(IDENTITY_BEGIN_PATH);
    expect(seen[1]!.init.method).to.equal('POST');
    expect(seen[1]!.init.credentials).to.equal('same-origin');
    expect((seen[1]!.init.headers as Record<string, string>)['Content-Type']).to.equal(
      'application/json'
    );
    expect(JSON.parse(seen[1]!.init.body as string)).to.deep.equal({ displayName: 'Matthew' });
  });
});
