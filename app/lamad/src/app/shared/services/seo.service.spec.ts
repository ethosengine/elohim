import { TestBed } from '@angular/core/testing';
import { Router, ActivatedRoute } from '@angular/router';
import { Subject, of } from 'rxjs';
import { beforeEach, afterEach, describe, expect, it } from 'vitest';

import { SeoService } from './seo.service';

describe('SeoService (lamad bundle — base href /lamad/)', () => {
  let service: SeoService;
  let baseEl: HTMLBaseElement;

  beforeEach(() => {
    // jsdom derives document.baseURI from a <base> element — simulate the bundle mount.
    baseEl = document.createElement('base');
    baseEl.href = '/lamad/';
    document.head.prepend(baseEl);

    TestBed.configureTestingModule({
      providers: [
        SeoService,
        {
          provide: Router,
          useValue: { events: new Subject().asObservable(), url: '/path/foundations/step/0' },
        },
        {
          provide: ActivatedRoute,
          useValue: {
            firstChild: null,
            outlet: 'primary',
            data: of({}),
          },
        },
      ],
    });
    service = TestBed.inject(SeoService);
  });

  afterEach(() => {
    baseEl.remove();
    document.querySelector('link[rel="canonical"]')?.remove();
  });

  it('re-prefixes the bundle mount onto generated canonical URLs (§12 keeper)', () => {
    service.updateSeo({ title: 'Step', description: 'A step' });
    const canonical = document.querySelector<HTMLLinkElement>('link[rel="canonical"]');
    expect(canonical?.href).toBe('https://elohim.host/lamad/path/foundations/step/0');
  });

  it('mints the universal address for content canonicals', () => {
    service.updateForContent({ id: 'fct-module-01', title: 'T', contentType: 'concept' });
    const canonical = document.querySelector<HTMLLinkElement>('link[rel="canonical"]');
    expect(canonical?.href).toBe('https://elohim.host/epr/fct-module-01');
  });
});
