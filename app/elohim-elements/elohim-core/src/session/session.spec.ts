import { expect } from '@open-wc/testing';
import { Session } from './session.js';

function clearSession() {
  document.cookie = 'elohim_session=; expires=Thu, 01 Jan 1970 00:00:00 UTC; path=/;';
}

function setSession(payload: object) {
  document.cookie = `elohim_session=${encodeURIComponent(JSON.stringify(payload))}; path=/`;
}

describe('Session', () => {
  beforeEach(() => {
    clearSession();
  });

  it('exposes null currentUser when no session cookie is present', () => {
    const s = new Session();
    expect(s.currentUser).to.be.null;
    expect(s.isAuthenticated).to.be.false;
  });

  it('parses the session cookie and exposes currentUser', () => {
    setSession({
      humanId: 'matthew',
      capabilities: ['*'],
      reach: 'authenticated',
    });
    const s = new Session();
    expect(s.currentUser?.humanId).to.equal('matthew');
    expect(s.isAuthenticated).to.be.true;
  });

  it('notifies subscribers when refreshFromCookies updates state', () => {
    const s = new Session();
    let notifyCount = 0;
    s.subscribe(() => {
      notifyCount += 1;
    });
    setSession({ humanId: 'matthew', capabilities: [], reach: 'authenticated' });
    s.refreshFromCookies();
    expect(notifyCount).to.equal(1);
    expect(s.currentUser?.humanId).to.equal('matthew');
  });

  it('does not notify subscribers when refresh produces an unchanged state', () => {
    setSession({ humanId: 'matthew', capabilities: [], reach: 'authenticated' });
    const s = new Session();
    let notifyCount = 0;
    s.subscribe(() => {
      notifyCount += 1;
    });
    s.refreshFromCookies();
    expect(notifyCount).to.equal(0);
  });

  it('unsubscribe removes the subscriber', () => {
    const s = new Session();
    let notifyCount = 0;
    const unsub = s.subscribe(() => {
      notifyCount += 1;
    });
    unsub();
    setSession({ humanId: 'matthew', capabilities: [], reach: 'authenticated' });
    s.refreshFromCookies();
    expect(notifyCount).to.equal(0);
  });

  it('returns null on malformed cookie JSON', () => {
    document.cookie = `elohim_session=${encodeURIComponent('not-json{{{')}; path=/`;
    const s = new Session();
    expect(s.currentUser).to.be.null;
  });
});
