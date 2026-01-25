import { ComponentFixture, TestBed, fakeAsync, tick } from '@angular/core/testing';

import { of, Subject } from 'rxjs';

import {
  GovernanceSignalService,
  ReactionCounts,
} from '@app/elohim/services/governance-signal.service';
import {
  EmotionalReactionType,
  EmotionalReactionConstraints,
  MediatedReaction,
  DEFAULT_REACTION_CONSTRAINTS,
  EMOTIONAL_REACTION_DESCRIPTIONS,
} from '@app/lamad/models/feedback-profile.model';

import { ReactionBarComponent } from './reaction-bar.component';

describe('ReactionBarComponent', () => {
  let component: ReactionBarComponent;
  let fixture: ComponentFixture<ReactionBarComponent>;
  let mockSignalService: jasmine.SpyObj<GovernanceSignalService>;
  let signalChanges$: Subject<any>;

  const mockReactionCounts: ReactionCounts = {
    total: 15,
    byType: {
      moved: 5,
      grateful: 4,
      inspired: 3,
      hopeful: 2,
      grieving: 1,
      challenged: 0,
      concerned: 0,
      uncomfortable: 0,
    },
    byCategory: { supportive: 14, critical: 1 },
  };

  beforeEach(async () => {
    signalChanges$ = new Subject();
    mockSignalService = jasmine.createSpyObj('GovernanceSignalService', [
      'recordReaction',
      'getReactionCounts',
      'recordMediationProceed',
    ], {
      signalChanges$: signalChanges$.asObservable(),
    });
    mockSignalService.getReactionCounts.and.returnValue(of(mockReactionCounts));
    mockSignalService.recordReaction.and.returnValue(of(true));
    mockSignalService.recordMediationProceed.and.returnValue(of(true));

    await TestBed.configureTestingModule({
      imports: [ReactionBarComponent],
      providers: [{ provide: GovernanceSignalService, useValue: mockSignalService }],
    }).compileComponents();

    fixture = TestBed.createComponent(ReactionBarComponent);
    component = fixture.componentInstance;
    component.contentId = 'content-1';
  });

  afterEach(() => {
    fixture.destroy();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  describe('initialization', () => {
    it('should have default values', () => {
      expect(component.showCounts).toBeTrue();
      expect(component.compact).toBeFalse();
      expect(component.contentType).toBe('learning-content');
    });

    it('should build available reactions on init', fakeAsync(() => {
      fixture.detectChanges();
      tick();

      expect(component.availableReactions.length).toBeGreaterThan(0);
    }));

    it('should load reaction counts on init', fakeAsync(() => {
      fixture.detectChanges();
      tick();

      expect(mockSignalService.getReactionCounts).toHaveBeenCalledWith('content-1');
      expect(component.reactionCounts).toEqual(mockReactionCounts);
    }));

    it('should have no user reaction initially', () => {
      expect(component.userReaction).toBeNull();
    });
  });

  describe('reaction types', () => {
    it('should have 8 reaction types defined', () => {
      const reactionTypes: EmotionalReactionType[] = [
        'moved', 'grateful', 'inspired', 'hopeful',
        'grieving', 'challenged', 'concerned', 'uncomfortable',
      ];
      reactionTypes.forEach(type => {
        expect((component as any).reactionIcons[type]).toBeDefined();
      });
    });

    it('should have icons for all reaction types', () => {
      expect((component as any).reactionIcons['moved']).toBe('💫');
      expect((component as any).reactionIcons['grateful']).toBe('🙏');
      expect((component as any).reactionIcons['inspired']).toBe('✨');
      expect((component as any).reactionIcons['hopeful']).toBe('🌱');
      expect((component as any).reactionIcons['grieving']).toBe('🕊️');
      expect((component as any).reactionIcons['challenged']).toBe('🤔');
      expect((component as any).reactionIcons['concerned']).toBe('⚠️');
      expect((component as any).reactionIcons['uncomfortable']).toBe('😟');
    });
  });

  describe('buildAvailableReactions()', () => {
    it('should use allowedReactions when provided', fakeAsync(() => {
      component.allowedReactions = ['moved', 'grateful'];
      fixture.detectChanges();
      tick();

      const types = component.availableReactions.map(r => r.type);
      expect(types).toContain('moved');
      expect(types).toContain('grateful');
    }));

    it('should use constraints when provided', fakeAsync(() => {
      component.constraints = {
        permittedTypes: ['inspired', 'hopeful'],
        mediatedTypes: [],
        requireAttribution: false,
        authorCanHide: false,
        criticalRequiresReasoning: false,
      };
      fixture.detectChanges();
      tick();

      const types = component.availableReactions.map(r => r.type);
      expect(types).toContain('inspired');
      expect(types).toContain('hopeful');
    }));

    it('should include mediated types', fakeAsync(() => {
      const mediatedReaction: MediatedReaction = {
        type: 'concerned',
        constitutionalReasoning: 'This reaction may impact learning',
        mediationPrompt: 'Are you sure?',
        proceedBehavior: {
          visibleToOthers: false,
          visibleToAuthor: true,
          loggedForMonitoring: false,
          affectsTrustScore: false,
          consequenceExplanation: 'Your reaction will be logged.',
        },
      };
      component.constraints = {
        permittedTypes: ['moved'],
        mediatedTypes: [mediatedReaction],
        requireAttribution: false,
        authorCanHide: false,
        criticalRequiresReasoning: false,
      };
      fixture.detectChanges();
      tick();

      const concernedReaction = component.availableReactions.find(r => r.type === 'concerned');
      expect(concernedReaction).toBeDefined();
      expect(concernedReaction?.mediated).toBeTrue();
    }));

    it('should set correct labels for reactions', fakeAsync(() => {
      component.allowedReactions = ['moved'];
      fixture.detectChanges();
      tick();

      const movedReaction = component.availableReactions.find(r => r.type === 'moved');
      expect(movedReaction?.label).toBe('Moved');
    }));

    it('should include descriptions from model', fakeAsync(() => {
      component.allowedReactions = ['moved'];
      fixture.detectChanges();
      tick();

      const movedReaction = component.availableReactions.find(r => r.type === 'moved');
      expect(movedReaction?.description).toBe(EMOTIONAL_REACTION_DESCRIPTIONS['moved']);
    }));
  });

  describe('onReactionClick()', () => {
    beforeEach(fakeAsync(() => {
      component.allowedReactions = ['moved', 'grateful'];
      fixture.detectChanges();
      tick();
    }));

    it('should submit reaction directly for non-mediated reactions', fakeAsync(() => {
      const movedReaction = component.availableReactions.find(r => r.type === 'moved')!;
      component.onReactionClick(movedReaction);
      tick();

      expect(mockSignalService.recordReaction).toHaveBeenCalledWith(
        'content-1',
        jasmine.objectContaining({ type: 'moved' })
      );
    }));

    it('should show mediation dialog for mediated reactions', fakeAsync(() => {
      const mediatedReaction: MediatedReaction = {
        type: 'concerned',
        constitutionalReasoning: 'Test reasoning',
        mediationPrompt: 'Are you sure?',
        proceedBehavior: {
          visibleToOthers: false,
          visibleToAuthor: true,
          loggedForMonitoring: false,
          affectsTrustScore: false,
          consequenceExplanation: 'Your reaction will be logged.',
        },
      };
      component.constraints = {
        permittedTypes: ['concerned'],
        mediatedTypes: [mediatedReaction],
        requireAttribution: false,
        authorCanHide: false,
        criticalRequiresReasoning: false,
      };
      (component as any).buildAvailableReactions();
      tick();

      const concernedReaction = component.availableReactions.find(r => r.type === 'concerned')!;
      component.onReactionClick(concernedReaction);

      expect(component.showMediationDialog).toBeTrue();
      expect(component.mediationContext?.reaction.type).toBe('concerned');
    }));

    it('should set user reaction on successful submission', fakeAsync(() => {
      const movedReaction = component.availableReactions.find(r => r.type === 'moved')!;
      component.onReactionClick(movedReaction);
      tick();

      expect(component.userReaction).toBe('moved');
    }));

    it('should refresh counts after reaction', fakeAsync(() => {
      mockSignalService.getReactionCounts.calls.reset();
      const movedReaction = component.availableReactions.find(r => r.type === 'moved')!;
      component.onReactionClick(movedReaction);
      tick();

      expect(mockSignalService.getReactionCounts).toHaveBeenCalled();
    }));
  });

  describe('mediation dialog', () => {
    let mediatedReaction: any;

    beforeEach(fakeAsync(() => {
      const mediationConfig: MediatedReaction = {
        type: 'concerned',
        constitutionalReasoning: 'Test reasoning',
        mediationPrompt: 'Are you sure?',
        suggestedAlternatives: ['challenged'],
        proceedBehavior: {
          visibleToOthers: true,
          visibleToAuthor: true,
          loggedForMonitoring: false,
          affectsTrustScore: false,
          consequenceExplanation: 'Your reaction will be visible.',
        },
      };
      component.constraints = {
        permittedTypes: ['concerned'],
        mediatedTypes: [mediationConfig],
        requireAttribution: false,
        authorCanHide: false,
        criticalRequiresReasoning: false,
      };
      fixture.detectChanges();
      tick();
      mediatedReaction = component.availableReactions.find(r => r.type === 'concerned')!;
    }));

    it('should log mediation proceed when user proceeds', fakeAsync(() => {
      component.onReactionClick(mediatedReaction);
      component.onMediationResponse(true);
      tick();

      expect(mockSignalService.recordMediationProceed).toHaveBeenCalledWith(
        jasmine.objectContaining({
          contentId: 'content-1',
          reactionType: 'concerned',
          proceededAnyway: true,
        })
      );
    }));

    it('should submit reaction when user proceeds with visible behavior', fakeAsync(() => {
      component.onReactionClick(mediatedReaction);
      component.onMediationResponse(true);
      tick();

      expect(mockSignalService.recordReaction).toHaveBeenCalledWith(
        'content-1',
        jasmine.objectContaining({ type: 'concerned' })
      );
    }));

    it('should not submit but set userReaction when proceed is not visible', fakeAsync(() => {
      const invisibleMediationConfig: MediatedReaction = {
        type: 'uncomfortable',
        constitutionalReasoning: 'Test',
        mediationPrompt: 'Are you sure?',
        proceedBehavior: {
          visibleToOthers: false,
          visibleToAuthor: true,
          loggedForMonitoring: false,
          affectsTrustScore: false,
          consequenceExplanation: 'Your reaction will not be visible.',
        },
      };
      component.constraints = {
        permittedTypes: ['uncomfortable'],
        mediatedTypes: [invisibleMediationConfig],
        requireAttribution: false,
        authorCanHide: false,
        criticalRequiresReasoning: false,
      };
      (component as any).buildAvailableReactions();
      tick();

      const invisibleReaction = component.availableReactions.find(r => r.type === 'uncomfortable')!;
      component.onReactionClick(invisibleReaction);
      component.onMediationResponse(true);
      tick();

      expect(component.userReaction).toBe('uncomfortable');
    }));

    it('should log and submit alternative when user chooses alternative', fakeAsync(() => {
      component.onReactionClick(mediatedReaction);
      component.onMediationResponse(false, 'challenged');
      tick();

      expect(mockSignalService.recordMediationProceed).toHaveBeenCalledWith(
        jasmine.objectContaining({
          proceededAnyway: false,
          alternativeChosen: 'challenged',
        })
      );
      expect(mockSignalService.recordReaction).toHaveBeenCalledWith(
        'content-1',
        jasmine.objectContaining({ type: 'challenged' })
      );
    }));

    it('should close dialog on any response', fakeAsync(() => {
      component.onReactionClick(mediatedReaction);
      expect(component.showMediationDialog).toBeTrue();

      component.onMediationResponse(true);
      tick();

      expect(component.showMediationDialog).toBeFalse();
      expect(component.mediationContext).toBeNull();
    }));

    it('should handle response without mediation context', () => {
      component.mediationContext = null;
      expect(() => component.onMediationResponse(true)).not.toThrow();
    });
  });

  describe('closeMediationDialog()', () => {
    it('should close dialog and clear context', () => {
      component.showMediationDialog = true;
      component.mediationContext = { reaction: {} as any, mediationConfig: {} as any };

      component.closeMediationDialog();

      expect(component.showMediationDialog).toBeFalse();
      expect(component.mediationContext).toBeNull();
    });
  });

  describe('getReactionDisplay()', () => {
    beforeEach(fakeAsync(() => {
      component.allowedReactions = ['moved', 'grateful'];
      fixture.detectChanges();
      tick();
    }));

    it('should return display for valid type', () => {
      const display = component.getReactionDisplay('moved');
      expect(display).toBeDefined();
      expect(display?.icon).toBe('💫');
    });

    it('should return undefined for invalid type', () => {
      const display = component.getReactionDisplay('invalid' as EmotionalReactionType);
      expect(display).toBeUndefined();
    });
  });

  describe('getReactionCount()', () => {
    beforeEach(fakeAsync(() => {
      fixture.detectChanges();
      tick();
    }));

    it('should return count for reaction type', () => {
      expect(component.getReactionCount('moved')).toBe(5);
      expect(component.getReactionCount('grateful')).toBe(4);
    });

    it('should return 0 for types with no counts', () => {
      expect(component.getReactionCount('concerned')).toBe(0);
    });

    it('should return 0 when no reaction counts loaded', () => {
      component.reactionCounts = null;
      expect(component.getReactionCount('moved')).toBe(0);
    });
  });

  describe('isUserReaction()', () => {
    it('should return true when matches user reaction', () => {
      component.userReaction = 'moved';
      expect(component.isUserReaction('moved')).toBeTrue();
    });

    it('should return false when does not match', () => {
      component.userReaction = 'grateful';
      expect(component.isUserReaction('moved')).toBeFalse();
    });

    it('should return false when no user reaction', () => {
      component.userReaction = null;
      expect(component.isUserReaction('moved')).toBeFalse();
    });
  });

  describe('signal changes subscription', () => {
    it('should refresh counts when relevant signal change occurs', fakeAsync(() => {
      fixture.detectChanges();
      tick();

      mockSignalService.getReactionCounts.calls.reset();

      signalChanges$.next({
        type: 'reaction',
        contentId: 'content-1',
      });
      tick();

      expect(mockSignalService.getReactionCounts).toHaveBeenCalled();
    }));

    it('should not refresh for different content', fakeAsync(() => {
      fixture.detectChanges();
      tick();

      mockSignalService.getReactionCounts.calls.reset();

      signalChanges$.next({
        type: 'reaction',
        contentId: 'different-content',
      });
      tick();

      expect(mockSignalService.getReactionCounts).not.toHaveBeenCalled();
    }));

    it('should not refresh for different signal type', fakeAsync(() => {
      fixture.detectChanges();
      tick();

      mockSignalService.getReactionCounts.calls.reset();

      signalChanges$.next({
        type: 'graduated-feedback',
        contentId: 'content-1',
      });
      tick();

      expect(mockSignalService.getReactionCounts).not.toHaveBeenCalled();
    }));
  });
});
