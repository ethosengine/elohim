import { ComponentFixture, TestBed } from '@angular/core/testing';
import { FocusedViewToggleComponent } from './focused-view-toggle.component';

describe('FocusedViewToggleComponent', () => {
  let component: FocusedViewToggleComponent;
  let fixture: ComponentFixture<FocusedViewToggleComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [FocusedViewToggleComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(FocusedViewToggleComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  describe('Component Properties', () => {
    it('should have isActive input property', () => {
      expect(component.isActive).toBeDefined();
      expect(typeof component.isActive).toBe('boolean');
    });

    it('should have toggled output EventEmitter', () => {
      expect(component.toggled).toBeDefined();
      expect(component.toggled.emit).toBeDefined();
    });

    it('should initialize with isActive as false', () => {
      expect(component.isActive).toBe(false);
    });
  });

  describe('toggle() method', () => {
    it('should have toggle method', () => {
      expect(component.toggle).toBeDefined();
      expect(typeof component.toggle).toBe('function');
    });

    it('should emit toggled output when toggle is called', (done) => {
      component.toggled.subscribe((value: boolean) => {
        expect(value).toBe(true);
        done();
      });

      component.toggle();
    });

    it('should emit negated isActive value when toggled', (done) => {
      component.isActive = true;
      component.toggled.subscribe((value: boolean) => {
        expect(value).toBe(false);
        done();
      });

      component.toggle();
    });

    it('should emit true when isActive is false', (done) => {
      component.isActive = false;
      component.toggled.subscribe((value: boolean) => {
        expect(value).toBe(true);
        done();
      });

      component.toggle();
    });
  });

  describe('Template', () => {
    it('should render a button element', () => {
      const button = fixture.nativeElement.querySelector('button.focused-view-btn');
      expect(button).toBeTruthy();
    });

    it('should have aria-label attribute', () => {
      const button = fixture.nativeElement.querySelector('button.focused-view-btn');
      expect(button.getAttribute('aria-label')).toBeDefined();
    });

    it('should have aria-pressed attribute', () => {
      const button = fixture.nativeElement.querySelector('button.focused-view-btn');
      expect(button.getAttribute('aria-pressed')).toBeDefined();
    });

    it('should call toggle on button click', (done) => {
      spyOn(component, 'toggle');
      const button = fixture.nativeElement.querySelector('button.focused-view-btn');

      button.click();

      expect(component.toggle).toHaveBeenCalled();
      done();
    });
  });
});
