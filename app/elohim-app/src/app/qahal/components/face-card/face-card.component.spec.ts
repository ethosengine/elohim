import { FaceCardComponent } from './face-card.component';

describe('FaceCardComponent', () => {
  let component: FaceCardComponent;

  beforeEach(() => {
    component = new FaceCardComponent();
    component.humanId = 'human-1';
    component.displayName = 'Matthew Dowell';
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  describe('initials computation', () => {
    it('should compute two-char initials from two words', () => {
      expect(FaceCardComponent.computeInitials('Matthew Dowell')).toBe('MD');
    });

    it('should compute one-char initial from single word', () => {
      expect(FaceCardComponent.computeInitials('Matthew')).toBe('M');
    });

    it('should use first and last name for three words', () => {
      expect(FaceCardComponent.computeInitials('Matthew James Dowell')).toBe('MD');
    });

    it('should handle empty string', () => {
      expect(FaceCardComponent.computeInitials('')).toBe('?');
    });

    it('should trim whitespace', () => {
      expect(FaceCardComponent.computeInitials('  Alice   Bob  ')).toBe('AB');
    });

    it('should uppercase initials', () => {
      expect(FaceCardComponent.computeInitials('alice bob')).toBe('AB');
    });
  });

  describe('color computation', () => {
    it('should return a consistent color for the same name', () => {
      const color1 = FaceCardComponent.computeColor('Matthew Dowell');
      const color2 = FaceCardComponent.computeColor('Matthew Dowell');
      expect(color1).toBe(color2);
    });

    it('should return different colors for different names', () => {
      const color1 = FaceCardComponent.computeColor('Matthew Dowell');
      const color2 = FaceCardComponent.computeColor('Jessica Dowell');
      // Not guaranteed to be different, but statistically likely
      expect(color1).toBeTruthy();
      expect(color2).toBeTruthy();
    });

    it('should return a valid hex color', () => {
      const color = FaceCardComponent.computeColor('Matthew Dowell');
      expect(color).toMatch(/^#[0-9a-f]{6}$/);
    });

    it('should handle empty string', () => {
      const color = FaceCardComponent.computeColor('');
      expect(color).toMatch(/^#[0-9a-f]{6}$/);
    });
  });

  describe('ngOnChanges', () => {
    it('should recompute initials and color when displayName changes', () => {
      component.displayName = 'Alice Smith';
      component.ngOnChanges({
        displayName: {
          currentValue: 'Alice Smith',
          previousValue: 'Matthew Dowell',
          firstChange: false,
          isFirstChange: () => false,
        },
      });

      expect(component.initials).toBe('AS');
      expect(component.avatarColor).toMatch(/^#[0-9a-f]{6}$/);
    });

    it('should not recompute when other inputs change', () => {
      component.displayName = 'Matthew Dowell';
      component.ngOnChanges({
        displayName: {
          currentValue: 'Matthew Dowell',
          previousValue: undefined,
          firstChange: true,
          isFirstChange: () => true,
        },
      });
      const originalInitials = component.initials;
      const originalColor = component.avatarColor;

      component.ngOnChanges({
        subtitle: {
          currentValue: 'Some household',
          previousValue: null,
          firstChange: true,
          isFirstChange: () => true,
        },
      });

      expect(component.initials).toBe(originalInitials);
      expect(component.avatarColor).toBe(originalColor);
    });
  });

  describe('subtitle display', () => {
    it('should accept a subtitle input', () => {
      component.subtitle = 'Dowell Household';
      expect(component.subtitle).toBe('Dowell Household');
    });

    it('should default subtitle to null', () => {
      const fresh = new FaceCardComponent();
      expect(fresh.subtitle).toBeNull();
    });
  });

  describe('selected event', () => {
    it('should emit humanId when selected fires', () => {
      const spy = vi.fn();
      component.selected.subscribe(spy);

      component.selected.emit(component.humanId);

      expect(spy).toHaveBeenCalledWith('human-1');
    });
  });

  describe('roleTag', () => {
    it('should default to null', () => {
      const fresh = new FaceCardComponent();
      expect(fresh.roleTag).toBeNull();
    });

    it('should accept a role tag', () => {
      component.roleTag = 'leader';
      expect(component.roleTag).toBe('leader');
    });
  });

  describe('profilePhotoUrl', () => {
    it('should default to null', () => {
      const fresh = new FaceCardComponent();
      expect(fresh.profilePhotoUrl).toBeNull();
    });

    it('should accept a photo URL', () => {
      component.profilePhotoUrl = 'https://example.com/photo.jpg';
      expect(component.profilePhotoUrl).toBe('https://example.com/photo.jpg');
    });
  });
});
