import { TestBed } from '@angular/core/testing';

import { BehaviorSubject } from 'rxjs';

import { BannerService } from './banner.service';
import { BannerNotice, BannerNoticeProvider } from '../models/banner-notice.model';

function makeNotice(overrides: Partial<BannerNotice> = {}): BannerNotice {
  return {
    id: 'notice-1',
    providerId: 'test-provider',
    severity: 'info',
    priority: 'info',
    contexts: ['global'],
    title: 'Test Notice',
    dismissible: true,
    createdAt: new Date(),
    ...overrides,
  };
}

function makeProvider(
  id: string,
  notices: BannerNotice[] = []
): BannerNoticeProvider & { notices$$: BehaviorSubject<BannerNotice[]> } {
  const subject = new BehaviorSubject<BannerNotice[]>(notices);
  return {
    providerId: id,
    notices$: subject.asObservable(),
    notices$$: subject,
    dismissNotice: jasmine.createSpy('dismissNotice'),
    handleAction: jasmine.createSpy('handleAction'),
  };
}

describe('BannerService', () => {
  let service: BannerService;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    service = TestBed.inject(BannerService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should emit empty array when no providers registered', done => {
    service.allNotices$.subscribe(notices => {
      expect(notices).toEqual([]);
      done();
    });
  });

  it('should emit notices from a registered provider', done => {
    const notice = makeNotice();
    const provider = makeProvider('test', [notice]);

    service.registerProvider(provider);

    service.allNotices$.subscribe(notices => {
      expect(notices.length).toBe(1);
      expect(notices[0].id).toBe('notice-1');
      done();
    });
  });

  it('should merge notices from multiple providers', done => {
    const p1 = makeProvider('p1', [makeNotice({ id: 'n1', providerId: 'p1' })]);
    const p2 = makeProvider('p2', [makeNotice({ id: 'n2', providerId: 'p2' })]);

    service.registerProvider(p1);
    service.registerProvider(p2);

    service.allNotices$.subscribe(notices => {
      expect(notices.length).toBe(2);
      done();
    });
  });

  it('should deduplicate notices by id', done => {
    const notice = makeNotice({ id: 'dup' });
    const p1 = makeProvider('p1', [{ ...notice, providerId: 'p1' }]);
    const p2 = makeProvider('p2', [{ ...notice, providerId: 'p2' }]);

    service.registerProvider(p1);
    service.registerProvider(p2);

    service.allNotices$.subscribe(notices => {
      expect(notices.length).toBe(1);
      done();
    });
  });

  it('should sort by priority then severity', done => {
    const system = makeNotice({ id: 'sys', priority: 'system', severity: 'warning' });
    const info = makeNotice({ id: 'inf', priority: 'info', severity: 'info' });
    const agent = makeNotice({ id: 'agt', priority: 'agent', severity: 'error' });

    const provider = makeProvider('p', [info, system, agent]);
    service.registerProvider(provider);

    service.allNotices$.subscribe(notices => {
      expect(notices.map(n => n.id)).toEqual(['sys', 'agt', 'inf']);
      done();
    });
  });

  it('should filter notices by context', done => {
    const global = makeNotice({ id: 'g', contexts: ['global'] });
    const lamad = makeNotice({ id: 'l', contexts: ['lamad'] });
    const shefa = makeNotice({ id: 's', contexts: ['shefa'] });

    const provider = makeProvider('p', [global, lamad, shefa]);
    service.registerProvider(provider);

    service.noticesForContext$('lamad').subscribe(notices => {
      expect(notices.map(n => n.id)).toEqual(['g', 'l']);
      done();
    });
  });

  it('should delegate dismissNotice to the correct provider', () => {
    const notice = makeNotice({ providerId: 'p1' });
    const provider = makeProvider('p1', [notice]);
    service.registerProvider(provider);

    service.dismissNotice(notice);

    expect(provider.dismissNotice).toHaveBeenCalledWith('notice-1');
  });

  it('should delegate handleAction to the correct provider', () => {
    const notice = makeNotice({ providerId: 'p1' });
    const provider = makeProvider('p1', [notice]);
    service.registerProvider(provider);

    service.handleAction(notice, 'learn-more');

    expect(provider.handleAction).toHaveBeenCalledWith('notice-1', 'learn-more');
  });

  it('should stop emitting notices when provider is unregistered', done => {
    const provider = makeProvider('p1', [makeNotice()]);
    service.registerProvider(provider);
    service.unregisterProvider('p1');

    service.allNotices$.subscribe(notices => {
      expect(notices).toEqual([]);
      done();
    });
  });
});
