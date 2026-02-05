/**
 * CreatePresenceComponent Tests
 *
 * Tests for creating contributor presence component.
 */

import { ComponentFixture, TestBed } from '@angular/core/testing';
import { CreatePresenceComponent } from './create-presence.component';
import { PresenceService } from '../../services/presence.service';
import { ContentService } from '@app/lamad/services/content.service';
import { Router } from '@angular/router';
import { signal } from '@angular/core';

describe('CreatePresenceComponent', () => {
  let component: CreatePresenceComponent;
  let fixture: ComponentFixture<CreatePresenceComponent>;
  let mockPresenceService: jasmine.SpyObj<PresenceService>;
  let mockContentService: jasmine.SpyObj<ContentService>;
  let mockRouter: jasmine.SpyObj<Router>;

  beforeEach(async () => {
    // Create mocks
    mockPresenceService = jasmine.createSpyObj('PresenceService', ['createPresence']);

    mockContentService = jasmine.createSpyObj('ContentService', ['searchContent']);

    mockRouter = jasmine.createSpyObj('Router', ['navigate']);

    await TestBed.configureTestingModule({
      imports: [CreatePresenceComponent],
      providers: [
        { provide: PresenceService, useValue: mockPresenceService },
        { provide: ContentService, useValue: mockContentService },
        { provide: Router, useValue: mockRouter },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(CreatePresenceComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  // ==========================================================================
  // Component Creation
  // ==========================================================================

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  // ==========================================================================
  // Outputs
  // ==========================================================================

  it('should have created output', () => {
    expect(component.created).toBeDefined();
  });

  it('should have cancelled output', () => {
    expect(component.cancelled).toBeDefined();
  });

  // ==========================================================================
  // Signals
  // ==========================================================================

  it('should have identifiers signal', () => {
    expect(component.identifiers).toBeDefined();
  });

  it('should have newIdentifier signal', () => {
    expect(component.newIdentifier).toBeDefined();
  });

  it('should have establishingContentIds signal', () => {
    expect(component.establishingContentIds).toBeDefined();
  });

  it('should have contentSearch signal', () => {
    expect(component.contentSearch).toBeDefined();
  });

  it('should have contentResults signal', () => {
    expect(component.contentResults).toBeDefined();
  });

  it('should have isSubmitting signal', () => {
    expect(component.isSubmitting).toBeDefined();
  });

  it('should have error signal', () => {
    expect(component.error).toBeDefined();
  });

  it('should have showContentSearch signal', () => {
    expect(component.showContentSearch).toBeDefined();
  });

  // ==========================================================================
  // Form State
  // ==========================================================================

  it('should initialize form state', () => {
    expect(component.displayName).toBeDefined();
    expect(component.displayName()).toEqual('');
    expect(component.note).toBeDefined();
    expect(component.note()).toEqual('');
  });

  // ==========================================================================
  // Public Methods
  // ==========================================================================

  it('should have addIdentifier method', () => {
    expect(component.addIdentifier).toBeDefined();
    expect(typeof component.addIdentifier).toBe('function');
  });

  it('should have removeIdentifier method', () => {
    expect(component.removeIdentifier).toBeDefined();
    expect(typeof component.removeIdentifier).toBe('function');
  });

  it('should have onSubmit method', () => {
    expect(component.onSubmit).toBeDefined();
    expect(typeof component.onSubmit).toBe('function');
  });

  it('should have onCancel method', () => {
    expect(component.onCancel).toBeDefined();
    expect(typeof component.onCancel).toBe('function');
  });

  it('should have searchContent method', () => {
    expect(component.searchContent).toBeDefined();
    expect(typeof component.searchContent).toBe('function');
  });

  it('should have clearError method', () => {
    expect(component.clearError).toBeDefined();
    expect(typeof component.clearError).toBe('function');
  });

  it('should have getProviderLabel method', () => {
    expect(component.getProviderLabel).toBeDefined();
    expect(typeof component.getProviderLabel).toBe('function');
  });

  it('should have getProviderIcon method', () => {
    expect(component.getProviderIcon).toBeDefined();
    expect(typeof component.getProviderIcon).toBe('function');
  });

  // ==========================================================================
  // Initial State
  // ==========================================================================

  it('should initialize identifiers as empty array', () => {
    expect(component.identifiers()).toEqual([]);
  });

  it('should initialize with no error', () => {
    expect(component.error()).toBeNull();
  });

  it('should initialize isSubmitting as false', () => {
    expect(component.isSubmitting()).toBe(false);
  });

  it('should initialize showContentSearch as false', () => {
    expect(component.showContentSearch()).toBe(false);
  });

  it('should initialize content results as empty', () => {
    expect(component.contentResults()).toEqual([]);
  });

  // ==========================================================================
  // Clear Error
  // ==========================================================================

  it('should clear error message', () => {
    component.error.set('Some error');
    component.clearError();
    expect(component.error()).toBeNull();
  });

  // ==========================================================================
  // Cancel
  // ==========================================================================

  it('should emit cancelled when onCancel is called', (done) => {
    component.cancelled.subscribe(() => {
      done();
    });
    component.onCancel();
  });

  // ==========================================================================
  // New Identifier Initialization
  // ==========================================================================

  it('should have new identifier with default provider github', () => {
    const newId = component.newIdentifier();
    expect(newId.provider).toBe('github');
  });

  // ==========================================================================
  // Form Validation
  // ==========================================================================

  describe('Form Validation', () => {
    it('should be invalid when display name is empty', () => {
      component.displayName.set('');
      expect(component.isValid()).toBe(false);
    });

    it('should be invalid when display name is too short', () => {
      component.displayName.set('A');
      expect(component.isValid()).toBe(false);
    });

    it('should be valid when display name is 2+ characters', () => {
      component.displayName.set('AB');
      expect(component.isValid()).toBe(true);
    });

    it('should be valid with longer names', () => {
      component.displayName.set('John Doe');
      expect(component.isValid()).toBe(true);
    });

    it('should trim whitespace when validating', () => {
      component.displayName.set('  ');
      expect(component.isValid()).toBe(false);

      component.displayName.set('  AB  ');
      expect(component.isValid()).toBe(true);
    });
  });

  // ==========================================================================
  // Add Identifier
  // ==========================================================================

  describe('Add Identifier', () => {
    it('should add identifier to list', () => {
      component.newIdentifier.set({
        id: '',
        provider: 'github',
        value: 'johndoe',
      });

      component.addIdentifier();

      const identifiers = component.identifiers();
      expect(identifiers.length).toBe(1);
      expect(identifiers[0].provider).toBe('github');
      expect(identifiers[0].value).toBe('johndoe');
    });

    it('should generate unique ID for each identifier', (done) => {
      component.newIdentifier.set({ id: '', provider: 'github', value: 'user1' });
      component.addIdentifier();

      // Small delay to ensure different timestamps
      setTimeout(() => {
        component.newIdentifier.set({ id: '', provider: 'github', value: 'user2' });
        component.addIdentifier();

        const identifiers = component.identifiers();
        expect(identifiers[0].id).not.toBe(identifiers[1].id);
        done();
      }, 10);
    });

    it('should trim identifier value', () => {
      component.newIdentifier.set({
        id: '',
        provider: 'github',
        value: '  johndoe  ',
      });

      component.addIdentifier();

      expect(component.identifiers()[0].value).toBe('johndoe');
    });

    it('should reset new identifier value after adding', () => {
      component.newIdentifier.set({
        id: '',
        provider: 'github',
        value: 'johndoe',
      });

      component.addIdentifier();

      expect(component.newIdentifier().value).toBe('');
    });

    it('should keep provider after adding identifier', () => {
      component.newIdentifier.set({
        id: '',
        provider: 'gitlab',
        value: 'johndoe',
      });

      component.addIdentifier();

      expect(component.newIdentifier().provider).toBe('gitlab');
    });

    it('should not add identifier with empty value', () => {
      component.newIdentifier.set({
        id: '',
        provider: 'github',
        value: '',
      });

      component.addIdentifier();

      expect(component.identifiers().length).toBe(0);
    });

    it('should not add identifier with whitespace-only value', () => {
      component.newIdentifier.set({
        id: '',
        provider: 'github',
        value: '   ',
      });

      component.addIdentifier();

      expect(component.identifiers().length).toBe(0);
    });

    it('should add multiple identifiers', () => {
      component.newIdentifier.set({ id: '', provider: 'github', value: 'user1' });
      component.addIdentifier();

      component.newIdentifier.set({ id: '', provider: 'gitlab', value: 'user2' });
      component.addIdentifier();

      component.newIdentifier.set({ id: '', provider: 'email', value: 'user@example.com' });
      component.addIdentifier();

      expect(component.identifiers().length).toBe(3);
    });
  });

  // ==========================================================================
  // Remove Identifier
  // ==========================================================================

  describe('Remove Identifier', () => {
    beforeEach(() => {
      component.identifiers.set([
        { id: 'id-1', provider: 'github', value: 'user1' },
        { id: 'id-2', provider: 'gitlab', value: 'user2' },
        { id: 'id-3', provider: 'email', value: 'user@example.com' },
      ]);
    });

    it('should remove identifier by id', () => {
      component.removeIdentifier('id-2');

      const identifiers = component.identifiers();
      expect(identifiers.length).toBe(2);
      expect(identifiers.find(i => i.id === 'id-2')).toBeUndefined();
    });

    it('should preserve other identifiers', () => {
      component.removeIdentifier('id-2');

      const identifiers = component.identifiers();
      expect(identifiers.find(i => i.id === 'id-1')).toBeDefined();
      expect(identifiers.find(i => i.id === 'id-3')).toBeDefined();
    });

    it('should handle removing non-existent id', () => {
      component.removeIdentifier('non-existent');

      expect(component.identifiers().length).toBe(3);
    });

    it('should remove all identifiers one by one', () => {
      component.removeIdentifier('id-1');
      expect(component.identifiers().length).toBe(2);

      component.removeIdentifier('id-2');
      expect(component.identifiers().length).toBe(1);

      component.removeIdentifier('id-3');
      expect(component.identifiers().length).toBe(0);
    });
  });

  // ==========================================================================
  // Provider Selection
  // ==========================================================================

  describe('Provider Selection', () => {
    it('should update provider', () => {
      component.setProvider('gitlab');
      expect(component.newIdentifier().provider).toBe('gitlab');
    });

    it('should handle provider change event', () => {
      const event = {
        target: { value: 'email' },
      } as unknown as Event;

      component.onProviderChange(event);

      expect(component.newIdentifier().provider).toBe('email');
    });
  });

  // ==========================================================================
  // Identifier Value Input
  // ==========================================================================

  describe('Identifier Value Input', () => {
    it('should update identifier value', () => {
      component.setIdentifierValue('newuser');
      expect(component.newIdentifier().value).toBe('newuser');
    });

    it('should handle input event', () => {
      const event = {
        target: { value: 'inputuser' },
      } as unknown as Event;

      component.onIdentifierInput(event);

      expect(component.newIdentifier().value).toBe('inputuser');
    });
  });

  // ==========================================================================
  // Content Search
  // ==========================================================================

  describe('Content Search', () => {
    it('should clear results for empty query', () => {
      component.contentSearch.set('');
      component.searchContent();

      expect(component.contentResults().length).toBe(0);
    });

    it('should clear results for short query', () => {
      component.contentSearch.set('a');
      component.searchContent();

      expect(component.contentResults().length).toBe(0);
    });

    it('should search content for valid query', () => {
      const mockResults = [
        { id: 'content-1', title: 'Test Content 1' },
        { id: 'content-2', title: 'Test Content 2' },
      ];

      mockContentService.searchContent.and.returnValue({
        subscribe: (handlers: any) => {
          handlers.next(mockResults);
          return { unsubscribe: () => {} };
        },
      } as any);

      component.contentSearch.set('test');
      component.searchContent();

      expect(mockContentService.searchContent).toHaveBeenCalledWith('test');
    });

    it('should limit results to 10 items', () => {
      const mockResults = Array.from({ length: 20 }, (_, i) => ({
        id: `content-${i}`,
        title: `Content ${i}`,
      }));

      mockContentService.searchContent.and.returnValue({
        subscribe: (handlers: any) => {
          handlers.next(mockResults);
          return { unsubscribe: () => {} };
        },
      } as any);

      component.contentSearch.set('test');
      component.searchContent();

      expect(component.contentResults().length).toBe(10);
    });

    it('should handle search error', () => {
      mockContentService.searchContent.and.returnValue({
        subscribe: (handlers: any) => {
          handlers.error(new Error('Search failed'));
          return { unsubscribe: () => {} };
        },
      } as any);

      component.contentSearch.set('test');
      component.searchContent();

      expect(component.contentResults().length).toBe(0);
    });
  });

  // ==========================================================================
  // Establishing Content Management
  // ==========================================================================

  describe('Establishing Content', () => {
    it('should add content ID to establishing list', () => {
      component.addEstablishingContent('content-123');

      expect(component.establishingContentIds()).toContain('content-123');
    });

    it('should not add duplicate content IDs', () => {
      component.addEstablishingContent('content-123');
      component.addEstablishingContent('content-123');

      expect(component.establishingContentIds().length).toBe(1);
    });

    it('should clear search after adding content', () => {
      component.contentSearch.set('test query');
      component.addEstablishingContent('content-123');

      expect(component.contentSearch()).toBe('');
    });

    it('should clear results after adding content', () => {
      component.contentResults.set([{ id: 'c1', title: 'Test' }]);
      component.addEstablishingContent('content-123');

      expect(component.contentResults().length).toBe(0);
    });

    it('should hide search after adding content', () => {
      component.showContentSearch.set(true);
      component.addEstablishingContent('content-123');

      expect(component.showContentSearch()).toBe(false);
    });

    it('should remove content from establishing list', () => {
      component.establishingContentIds.set(['content-1', 'content-2', 'content-3']);
      component.removeEstablishingContent('content-2');

      const ids = component.establishingContentIds();
      expect(ids.length).toBe(2);
      expect(ids).not.toContain('content-2');
    });

    it('should handle removing non-existent content ID', () => {
      component.establishingContentIds.set(['content-1']);
      component.removeEstablishingContent('non-existent');

      expect(component.establishingContentIds().length).toBe(1);
    });
  });

  // ==========================================================================
  // Toggle Content Search
  // ==========================================================================

  describe('Toggle Content Search', () => {
    it('should toggle search visibility', () => {
      expect(component.showContentSearch()).toBe(false);

      component.toggleContentSearch();
      expect(component.showContentSearch()).toBe(true);

      component.toggleContentSearch();
      expect(component.showContentSearch()).toBe(false);
    });

    it('should clear search when hiding', () => {
      component.showContentSearch.set(true);
      component.contentSearch.set('test query');
      component.contentResults.set([{ id: 'c1', title: 'Test' }]);

      component.toggleContentSearch();

      expect(component.contentSearch()).toBe('');
      expect(component.contentResults().length).toBe(0);
    });

    it('should not clear search when showing', () => {
      component.showContentSearch.set(false);
      component.contentSearch.set('test query');

      component.toggleContentSearch();

      expect(component.contentSearch()).toBe('test query');
    });
  });

  // ==========================================================================
  // Form Submission - Success
  // ==========================================================================

  describe('Form Submission - Success', () => {
    beforeEach(() => {
      mockPresenceService.createPresence.and.returnValue(
        Promise.resolve({ id: 'presence-123' } as any)
      );
    });

    it('should submit form with valid data', async () => {
      component.displayName.set('John Doe');
      component.note.set('Test note');

      await component.onSubmit();

      expect(mockPresenceService.createPresence).toHaveBeenCalledWith(
        jasmine.objectContaining({
          displayName: 'John Doe',
          note: 'Test note',
        })
      );
    });

    it('should include external identifiers in request', async () => {
      component.displayName.set('John Doe');
      component.identifiers.set([
        { id: 'id-1', provider: 'github', value: 'johndoe' },
        { id: 'id-2', provider: 'email', value: 'john@example.com' },
      ]);

      await component.onSubmit();

      expect(mockPresenceService.createPresence).toHaveBeenCalledWith(
        jasmine.objectContaining({
          externalIdentifiers: [
            { provider: 'github', value: 'johndoe' },
            { provider: 'email', value: 'john@example.com' },
          ],
        })
      );
    });

    it('should include establishing content IDs in request', async () => {
      component.displayName.set('John Doe');
      component.establishingContentIds.set(['content-1', 'content-2']);

      await component.onSubmit();

      expect(mockPresenceService.createPresence).toHaveBeenCalledWith(
        jasmine.objectContaining({
          establishingContentIds: ['content-1', 'content-2'],
        })
      );
    });

    it('should emit created event with presence ID', (done) => {
      component.displayName.set('John Doe');

      component.created.subscribe((presenceId: string) => {
        expect(presenceId).toBe('presence-123');
        done();
      });

      void component.onSubmit();
    });

    it('should navigate to presence detail page', async () => {
      component.displayName.set('John Doe');

      await component.onSubmit();

      expect(mockRouter.navigate).toHaveBeenCalledWith(['/identity/presences', 'presence-123']);
    });

    it('should set isSubmitting during submission', async () => {
      component.displayName.set('John Doe');

      const submitPromise = component.onSubmit();
      expect(component.isSubmitting()).toBe(true);

      await submitPromise;
      expect(component.isSubmitting()).toBe(false);
    });

    it('should clear error on successful submission', async () => {
      component.displayName.set('John Doe');
      component.error.set('Previous error');

      await component.onSubmit();

      expect(component.error()).toBeNull();
    });

    it('should trim display name', async () => {
      component.displayName.set('  John Doe  ');

      await component.onSubmit();

      expect(mockPresenceService.createPresence).toHaveBeenCalledWith(
        jasmine.objectContaining({
          displayName: 'John Doe',
        })
      );
    });

    it('should trim note', async () => {
      component.displayName.set('John Doe');
      component.note.set('  Test note  ');

      await component.onSubmit();

      expect(mockPresenceService.createPresence).toHaveBeenCalledWith(
        jasmine.objectContaining({
          note: 'Test note',
        })
      );
    });

    it('should omit note if empty', async () => {
      component.displayName.set('John Doe');
      component.note.set('');

      await component.onSubmit();

      const callArgs = mockPresenceService.createPresence.calls.mostRecent().args[0];
      expect(callArgs.note).toBeUndefined();
    });

    it('should omit externalIdentifiers if empty', async () => {
      component.displayName.set('John Doe');
      component.identifiers.set([]);

      await component.onSubmit();

      const callArgs = mockPresenceService.createPresence.calls.mostRecent().args[0];
      expect(callArgs.externalIdentifiers).toBeUndefined();
    });

    it('should omit establishingContentIds if empty', async () => {
      component.displayName.set('John Doe');
      component.establishingContentIds.set([]);

      await component.onSubmit();

      const callArgs = mockPresenceService.createPresence.calls.mostRecent().args[0];
      expect(callArgs.establishingContentIds).toBeUndefined();
    });
  });

  // ==========================================================================
  // Form Submission - Validation Errors
  // ==========================================================================

  describe('Form Submission - Validation', () => {
    it('should not submit if display name is empty', async () => {
      component.displayName.set('');

      await component.onSubmit();

      expect(mockPresenceService.createPresence).not.toHaveBeenCalled();
      expect(component.error()).toContain('display name');
    });

    it('should not submit if display name is too short', async () => {
      component.displayName.set('A');

      await component.onSubmit();

      expect(mockPresenceService.createPresence).not.toHaveBeenCalled();
      expect(component.error()).toContain('at least 2 characters');
    });

    it('should set error message for invalid form', async () => {
      component.displayName.set('');

      await component.onSubmit();

      expect(component.error()).toBeTruthy();
    });
  });

  // ==========================================================================
  // Form Submission - Service Errors
  // ==========================================================================

  describe('Form Submission - Service Errors', () => {
    it('should handle service error', async () => {
      component.displayName.set('John Doe');
      mockPresenceService.createPresence.and.returnValue(
        Promise.reject(new Error('Network error'))
      );

      await component.onSubmit();

      expect(component.error()).toBe('Network error');
    });

    it('should handle non-Error exception', async () => {
      component.displayName.set('John Doe');
      mockPresenceService.createPresence.and.returnValue(Promise.reject('String error'));

      await component.onSubmit();

      expect(component.error()).toBe('Failed to create presence');
    });

    it('should set isSubmitting to false after error', async () => {
      component.displayName.set('John Doe');
      mockPresenceService.createPresence.and.returnValue(Promise.reject(new Error('Error')));

      await component.onSubmit();

      expect(component.isSubmitting()).toBe(false);
    });

    it('should not navigate on error', async () => {
      component.displayName.set('John Doe');
      mockPresenceService.createPresence.and.returnValue(Promise.reject(new Error('Error')));

      await component.onSubmit();

      expect(mockRouter.navigate).not.toHaveBeenCalled();
    });
  });

  // ==========================================================================
  // Cancel Action
  // ==========================================================================

  describe('Cancel Action', () => {
    it('should navigate to presences list', () => {
      component.onCancel();

      expect(mockRouter.navigate).toHaveBeenCalledWith(['/identity/presences']);
    });
  });
});
