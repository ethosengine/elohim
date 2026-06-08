import { SimpleChange } from '@angular/core';
import { ComponentFixture, TestBed } from '@angular/core/testing';

import { eprToUniversalHref, parseEpr } from '@elohim/service';

import { ContentNode } from '../../models/content-node.model';

import { PathViewerComponent } from './path-viewer.component';

/**
 * A 2-section / 3-item epr-composite body as the wire delivers it: a JSON
 * STRING on node.content (the renderer must parse it tolerantly).
 */
const COMPOSITE_BODY = {
  schemaVersion: 1,
  pathType: 'course',
  layout: 'sequential',
  sections: [
    {
      id: 'intro',
      title: 'Getting Oriented',
      description: 'Set the stage.',
      level: 'unit',
      estimatedDuration: '20 min',
      items: [
        {
          ref: 'epr:welcome',
          role: 'step',
          title: 'Welcome',
          narrative: 'Why this path exists.',
          learningObjectives: ['Understand the goal'],
          completionCriteria: { type: 'view' },
        },
        {
          ref: 'first-principles',
          role: 'step',
          title: 'First Principles',
          narrative: 'The ground we build on.',
          completionCriteria: { type: 'view' },
        },
      ],
    },
    {
      id: 'practice',
      title: 'Practice',
      description: 'Apply it.',
      level: 'unit',
      estimatedDuration: '45 min',
      items: [
        {
          ref: 'epr:exercise-one',
          role: 'checkpoint',
          title: 'Exercise One',
          narrative: 'Hands on the keyboard.',
          completionCriteria: { type: 'score', threshold: 0.8 },
        },
      ],
    },
  ],
};

function makeNode(content: string | object | undefined): ContentNode {
  return {
    id: 'fair-exchange',
    title: 'Fair Exchange',
    description: 'A learning path about reciprocity.',
    contentType: 'path',
    contentFormat: 'epr-composite',
    content: content as string | object,
    tags: ['path'],
    relatedNodeIds: [],
    metadata: {},
  };
}

describe('PathViewerComponent', () => {
  let component: PathViewerComponent;
  let fixture: ComponentFixture<PathViewerComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [PathViewerComponent],
      teardown: { destroyAfterEach: false },
    }).compileComponents();

    fixture = TestBed.createComponent(PathViewerComponent);
    component = fixture.componentInstance;
  });

  function render(content: string | object | undefined): HTMLElement {
    component.node = makeNode(content);
    component.ngOnChanges({ node: new SimpleChange(null, component.node, true) });
    fixture.detectChanges();
    return fixture.nativeElement as HTMLElement;
  }

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  describe('with a populated epr-composite body (JSON string)', () => {
    let el: HTMLElement;

    beforeEach(() => {
      el = render(JSON.stringify(COMPOSITE_BODY));
    });

    it('renders the root outline testid', () => {
      expect(el.querySelector('[data-testid="epr-composite-outline"]')).toBeTruthy();
    });

    it('renders the node title as an h1', () => {
      const h1 = el.querySelector('h1');
      expect(h1?.textContent).toContain('Fair Exchange');
    });

    it('renders the node description', () => {
      expect(el.textContent).toContain('A learning path about reciprocity.');
    });

    it('renders one h2 per section', () => {
      const headings = Array.from(el.querySelectorAll('h2')).map(h => h.textContent ?? '');
      expect(headings.length).toBe(2);
      expect(headings[0]).toContain('Getting Oriented');
      expect(headings[1]).toContain('Practice');
    });

    it('renders a section estimatedDuration', () => {
      expect(el.textContent).toContain('20 min');
      expect(el.textContent).toContain('45 min');
    });

    it('renders one /epr/{ref} link per item, minted via eprToUniversalHref', () => {
      const links = Array.from(el.querySelectorAll('a[data-epr-ref]')) as HTMLAnchorElement[];
      expect(links.length).toBe(3);

      const hrefs = links.map(a => a.getAttribute('href'));
      // epr:-prefixed and bare refs both normalize to the universal address.
      expect(hrefs).toContain('/epr/welcome');
      expect(hrefs).toContain('/epr/first-principles');
      expect(hrefs).toContain('/epr/exercise-one');

      // Each href equals the minter's output for the original ref (shape proof).
      expect(hrefs).toContain(eprToUniversalHref(parseEpr('epr:welcome')));
      expect(hrefs).toContain(eprToUniversalHref(parseEpr('exercise-one')));
    });

    it('gives every item link discernible text', () => {
      const links = Array.from(el.querySelectorAll('a[data-epr-ref]')) as HTMLAnchorElement[];
      for (const a of links) {
        expect((a.textContent ?? '').trim().length).toBeGreaterThan(0);
      }
    });

    it('renders item narrative', () => {
      expect(el.textContent).toContain('Why this path exists.');
    });

    it('uses real list elements for a11y (ol sections, ul items)', () => {
      expect(el.querySelector('ol')).toBeTruthy();
      expect(el.querySelector('ul')).toBeTruthy();
    });

    it('does not dump raw JSON', () => {
      expect(el.textContent).not.toContain('schemaVersion');
      expect(el.textContent).not.toContain('completionCriteria');
    });
  });

  describe('graceful states', () => {
    it('renders the empty note for an absent body (no throw)', () => {
      const act = () => render(undefined);
      expect(act).not.toThrow();
      const el = fixture.nativeElement as HTMLElement;
      expect(el.querySelector('[data-testid="epr-composite-empty"]')).toBeTruthy();
      expect(el.textContent).toContain('no sections yet');
    });

    it('renders the empty note for a malformed JSON body (no throw)', () => {
      const act = () => render('{ this is not valid json');
      expect(act).not.toThrow();
      const el = fixture.nativeElement as HTMLElement;
      expect(el.querySelector('[data-testid="epr-composite-empty"]')).toBeTruthy();
    });

    it('renders the empty note for an empty sections array (no throw)', () => {
      const act = () => render(JSON.stringify({ sections: [] }));
      expect(act).not.toThrow();
      const el = fixture.nativeElement as HTMLElement;
      expect(el.querySelector('[data-testid="epr-composite-empty"]')).toBeTruthy();
      // Still renders the title even when empty.
      expect(el.querySelector('h1')?.textContent).toContain('Fair Exchange');
    });

    it('tolerates an already-parsed object body', () => {
      const el = render(COMPOSITE_BODY);
      expect(el.querySelectorAll('a[data-epr-ref]').length).toBe(3);
    });
  });
});
