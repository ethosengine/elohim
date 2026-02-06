import { ComponentFixture, TestBed } from '@angular/core/testing';

import { DomInteractionService } from '../../services/dom-interaction.service';

import { HeroComponent } from './hero.component';

describe('HeroComponent', () => {
  let component: HeroComponent;
  let fixture: ComponentFixture<HeroComponent>;
  let domInteractionService: jasmine.SpyObj<DomInteractionService>;

  beforeEach(async () => {
    const domInteractionServiceSpy = jasmine.createSpyObj<DomInteractionService>(
      'DomInteractionService',
      ['setupScrollIndicator', 'setupHeroTitleAnimation']
    );

    await TestBed.configureTestingModule({
      imports: [HeroComponent],
      providers: [
        {
          provide: DomInteractionService,
          useValue: domInteractionServiceSpy,
        },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(HeroComponent);
    component = fixture.componentInstance;
    domInteractionService = TestBed.inject(
      DomInteractionService
    ) as jasmine.SpyObj<DomInteractionService>;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  describe('Property Initialization', () => {
    it('should initialize isVideoVisible to false', () => {
      expect(component.isVideoVisible).toBe(false);
    });

    it('should initialize isDeepDiveVisible to false', () => {
      expect(component.isDeepDiveVisible).toBe(false);
    });
  });

  describe('ngOnInit', () => {
    it('should call setupScrollIndicator on DomInteractionService', () => {
      fixture.detectChanges();
      expect(domInteractionService.setupScrollIndicator).toHaveBeenCalled();
    });

    it('should call setupHeroTitleAnimation on DomInteractionService', () => {
      fixture.detectChanges();
      expect(domInteractionService.setupHeroTitleAnimation).toHaveBeenCalled();
    });
  });

  describe('toggleVideo', () => {
    it('should set isVideoVisible to true', () => {
      component.toggleVideo();
      expect(component.isVideoVisible).toBe(true);
    });

    it('should set isDeepDiveVisible to false', () => {
      component.isDeepDiveVisible = true;
      component.toggleVideo();
      expect(component.isDeepDiveVisible).toBe(false);
    });

    it('should toggle video and hide deep dive together', () => {
      component.toggleVideo();
      expect(component.isVideoVisible).toBe(true);
      expect(component.isDeepDiveVisible).toBe(false);
    });
  });

  describe('toggleDeepDive', () => {
    it('should set isDeepDiveVisible to true', () => {
      component.toggleDeepDive();
      expect(component.isDeepDiveVisible).toBe(true);
    });

    it('should set isVideoVisible to false', () => {
      component.isVideoVisible = true;
      component.toggleDeepDive();
      expect(component.isVideoVisible).toBe(false);
    });

    it('should toggle deep dive and hide video together', () => {
      component.toggleDeepDive();
      expect(component.isDeepDiveVisible).toBe(true);
      expect(component.isVideoVisible).toBe(false);
    });
  });

  describe('hideVideo', () => {
    it('should set isVideoVisible to false', () => {
      component.isVideoVisible = true;
      component.hideVideo();
      expect(component.isVideoVisible).toBe(false);
    });

    it('should set isDeepDiveVisible to false', () => {
      component.isDeepDiveVisible = true;
      component.hideVideo();
      expect(component.isDeepDiveVisible).toBe(false);
    });

    it('should hide both video and deep dive', () => {
      component.isVideoVisible = true;
      component.isDeepDiveVisible = true;
      component.hideVideo();
      expect(component.isVideoVisible).toBe(false);
      expect(component.isDeepDiveVisible).toBe(false);
    });
  });
});
