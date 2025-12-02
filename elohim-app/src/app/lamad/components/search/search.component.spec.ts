import { ComponentFixture, TestBed, fakeAsync, tick } from '@angular/core/testing';
import { SearchComponent } from './search.component';
import { ActivatedRoute } from '@angular/router';
import { provideHttpClient } from '@angular/common/http';
import { of, throwError, Subject } from 'rxjs';
import { SearchService } from '../../services/search.service';
import { SearchResult } from '../../models/search.model';

describe('SearchComponent', () => {
  let component: SearchComponent;
  let fixture: ComponentFixture<SearchComponent>;
  let searchServiceSpy: jasmine.SpyObj<SearchService>;
  let queryParamsSubject: Subject<any>;

  const mockSearchResults = [
    {
      id: 'result-1',
      title: 'Result 1',
      description: 'First result',
      contentType: 'epic',
      tags: ['tag1', 'tag2']
    },
    {
      id: 'result-2',
      title: 'Result 2',
      description: 'Second result',
      contentType: 'feature',
      tags: ['tag3']
    }
  ] as SearchResult[];

  beforeEach(async () => {
    queryParamsSubject = new Subject();

    const searchSpyObj = jasmine.createSpyObj('SearchService', ['search']);

    await TestBed.configureTestingModule({
      imports: [SearchComponent],
      providers: [
        provideHttpClient(),
        {
          provide: ActivatedRoute,
          useValue: {
            queryParams: queryParamsSubject.asObservable()
          }
        },
        { provide: SearchService, useValue: searchSpyObj }
      ]
    }).compileComponents();

    searchServiceSpy = TestBed.inject(SearchService) as jasmine.SpyObj<SearchService>;

    // Default spy return - use 'as any' to avoid full interface implementation in test
    searchServiceSpy.search.and.returnValue(of({ results: mockSearchResults } as any));

    fixture = TestBed.createComponent(SearchComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  describe('initialization', () => {
    it('should perform search if query param is present', fakeAsync(() => {
      fixture.detectChanges();
      queryParamsSubject.next({ q: 'test query' });
      tick();

      expect(component.query).toBe('test query');
      expect(searchServiceSpy.search).toHaveBeenCalledWith({ text: 'test query' });
      expect(component.hasSearched).toBeTrue();
    }));

    it('should not search if query param is empty', fakeAsync(() => {
      fixture.detectChanges();
      queryParamsSubject.next({ q: '' });
      tick();

      expect(searchServiceSpy.search).not.toHaveBeenCalled();
    }));

    it('should not search if no query param', fakeAsync(() => {
      fixture.detectChanges();
      queryParamsSubject.next({});
      tick();

      expect(searchServiceSpy.search).not.toHaveBeenCalled();
    }));
  });

  describe('performSearch', () => {
    beforeEach(() => {
      fixture.detectChanges();
    });

    it('should perform search with current query', fakeAsync(() => {
      component.query = 'test search';
      component.performSearch();
      tick();

      expect(searchServiceSpy.search).toHaveBeenCalledWith({ text: 'test search' });
      expect(component.results).toEqual(mockSearchResults);
      expect(component.hasSearched).toBeTrue();
      expect(component.isLoading).toBeFalse();
    }));

    it('should not search if query is empty', () => {
      component.query = '';
      component.performSearch();

      expect(searchServiceSpy.search).not.toHaveBeenCalled();
    });

    it('should not search if query is whitespace only', () => {
      component.query = '   ';
      component.performSearch();

      expect(searchServiceSpy.search).not.toHaveBeenCalled();
    });

    it('should handle search error', fakeAsync(() => {
      searchServiceSpy.search.and.returnValue(throwError(() => new Error('Search failed')));
      component.query = 'test';
      component.performSearch();
      tick();

      expect(component.results).toEqual([]);
      expect(component.isLoading).toBeFalse();
    }));

    it('should set isLoading during search', fakeAsync(() => {
      component.query = 'test';
      component.performSearch();

      expect(component.isLoading).toBeTrue();
      tick();
      expect(component.isLoading).toBeFalse();
    }));
  });

  describe('getNodeRoute', () => {
    it('should return correct route array', () => {
      const result = {
        id: 'test-id',
        title: 'Test',
        description: 'Desc',
        contentType: 'epic',
        tags: []
      } as unknown as SearchResult;

      const route = component.getNodeRoute(result);

      expect(route).toEqual(['/lamad/content', 'test-id']);
    });
  });

  describe('getNodeTypeIcon', () => {
    it('should return correct icon for epic', () => {
      expect(component.getNodeTypeIcon('epic')).toBe('📖');
    });

    it('should return correct icon for feature', () => {
      expect(component.getNodeTypeIcon('feature')).toBe('⚡');
    });

    it('should return correct icon for scenario', () => {
      expect(component.getNodeTypeIcon('scenario')).toBe('✓');
    });

    it('should return default icon for unknown type', () => {
      expect(component.getNodeTypeIcon('unknown')).toBe('📄');
    });
  });

  describe('template', () => {
    beforeEach(fakeAsync(() => {
      fixture.detectChanges();
    }));

    it('should display search input', () => {
      const input = fixture.nativeElement.querySelector('.search-input');
      expect(input).toBeTruthy();
    });

    it('should display search button', () => {
      const button = fixture.nativeElement.querySelector('.search-btn');
      expect(button).toBeTruthy();
    });

    it('should show results count after search', fakeAsync(() => {
      component.query = 'test';
      component.performSearch();
      tick();
      fixture.detectChanges();

      const resultsSection = fixture.nativeElement.querySelector('.results-section');
      expect(resultsSection).toBeTruthy();
      expect(resultsSection.textContent).toContain('Results (2)');
    }));

    it('should show no results message when empty', fakeAsync(() => {
      searchServiceSpy.search.and.returnValue(of({ results: [] } as any));
      component.query = 'nonexistent';
      component.performSearch();
      tick();
      fixture.detectChanges();

      const noResults = fixture.nativeElement.querySelector('.no-results');
      expect(noResults).toBeTruthy();
      expect(noResults.textContent).toContain('No results found');
    }));

    it('should display result cards', fakeAsync(() => {
      component.query = 'test';
      component.performSearch();
      tick();
      fixture.detectChanges();

      const resultCards = fixture.nativeElement.querySelectorAll('.result-card');
      expect(resultCards.length).toBe(2);
    }));

    it('should display tags on result cards', fakeAsync(() => {
      component.query = 'test';
      component.performSearch();
      tick();
      fixture.detectChanges();

      const tags = fixture.nativeElement.querySelectorAll('.tag');
      expect(tags.length).toBeGreaterThan(0);
    }));
  });
});
