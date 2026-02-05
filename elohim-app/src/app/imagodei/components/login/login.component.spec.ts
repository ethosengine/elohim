/**
 * LoginComponent Tests
 *
 * Tests for hosted human authentication component.
 */

import { ComponentFixture, TestBed } from '@angular/core/testing';
import { LoginComponent } from './login.component';
import { AuthService } from '../../services/auth.service';
import { PasswordAuthProvider } from '../../services/providers/password-auth.provider';
import { IdentityService } from '../../services/identity.service';
import { DoorwayRegistryService } from '../../services/doorway-registry.service';
import { Router, ActivatedRoute } from '@angular/router';
import { signal } from '@angular/core';
import { of } from 'rxjs';
import { AUTH_IDENTIFIER_KEY } from '../../models/auth.model';

describe('LoginComponent', () => {
  let component: LoginComponent;
  let fixture: ComponentFixture<LoginComponent>;
  let mockAuthService: jasmine.SpyObj<AuthService>;
  let mockPasswordProvider: jasmine.SpyObj<PasswordAuthProvider>;
  let mockIdentityService: jasmine.SpyObj<IdentityService>;
  let mockDoorwayRegistry: jasmine.SpyObj<DoorwayRegistryService>;
  let mockRouter: jasmine.SpyObj<Router>;
  let mockActivatedRoute: jasmine.SpyObj<ActivatedRoute>;

  beforeEach(async () => {
    // Create mocks
    mockAuthService = jasmine.createSpyObj('AuthService', [
      'registerProvider',
      'hasProvider',
      'login',
      'isAuthenticated',
    ]);
    mockAuthService.hasProvider.and.returnValue(false);
    mockAuthService.isAuthenticated.and.returnValue(false);
    mockAuthService.login.and.returnValue(
      Promise.resolve({
        success: true,
        token: 'mock-token',
        humanId: 'mock-human',
        agentPubKey: 'mock-pubkey',
        expiresAt: new Date(Date.now() + 3600000).toISOString(),
        identifier: 'mock-user',
      })
    );

    mockPasswordProvider = jasmine.createSpyObj('PasswordAuthProvider', ['login', 'logout']);

    mockIdentityService = jasmine.createSpyObj(
      'IdentityService',
      ['waitForAuthenticatedState'],
      {
        mode: signal('session'),
        isAuthenticated: signal(false),
      }
    );
    mockIdentityService.waitForAuthenticatedState.and.returnValue(Promise.resolve(true));

    mockDoorwayRegistry = jasmine.createSpyObj(
      'DoorwayRegistryService',
      ['select'],
      {
        selected: signal(null),
        hasSelection: signal(false),
      }
    );

    mockRouter = jasmine.createSpyObj('Router', ['navigate']);
    mockRouter.navigate.and.returnValue(Promise.resolve(true));

    mockActivatedRoute = {
      queryParams: of({}),
    } as any;

    // Mock localStorage
    spyOn(localStorage, 'getItem').and.returnValue(null);
    spyOn(localStorage, 'setItem');
    spyOn(localStorage, 'removeItem');

    await TestBed.configureTestingModule({
      imports: [LoginComponent],
      providers: [
        { provide: AuthService, useValue: mockAuthService },
        { provide: PasswordAuthProvider, useValue: mockPasswordProvider },
        { provide: IdentityService, useValue: mockIdentityService },
        { provide: DoorwayRegistryService, useValue: mockDoorwayRegistry },
        { provide: Router, useValue: mockRouter },
        { provide: ActivatedRoute, useValue: mockActivatedRoute },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(LoginComponent);
    component = fixture.componentInstance;
  });

  // ==========================================================================
  // Component Creation
  // ==========================================================================

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  // ==========================================================================
  // Signals
  // ==========================================================================

  it('should have isLoading signal', () => {
    expect(component.isLoading).toBeDefined();
  });

  it('should have error signal', () => {
    expect(component.error).toBeDefined();
  });

  it('should have showPassword signal', () => {
    expect(component.showPassword).toBeDefined();
  });

  it('should have currentStep signal', () => {
    expect(component.currentStep).toBeDefined();
  });

  // ==========================================================================
  // Delegated Signals
  // ==========================================================================

  it('should delegate selectedDoorway from DoorwayRegistryService', () => {
    expect(component.selectedDoorway).toBeDefined();
  });

  it('should delegate hasDoorwaySelected from DoorwayRegistryService', () => {
    expect(component.hasDoorwaySelected).toBeDefined();
  });

  // ==========================================================================
  // Form State
  // ==========================================================================

  it('should initialize form state', () => {
    expect(component.form).toBeDefined();
    expect(component.form.identifier).toEqual('');
    expect(component.form.password).toEqual('');
    expect(component.form.rememberMe).toBe(true);
  });

  // ==========================================================================
  // Public Methods
  // ==========================================================================

  it('should have onLogin method', () => {
    expect(component.onLogin).toBeDefined();
    expect(typeof component.onLogin).toBe('function');
  });

  it('should have clearError method', () => {
    expect(component.clearError).toBeDefined();
    expect(typeof component.clearError).toBe('function');
  });

  it('should have togglePasswordVisibility method', () => {
    expect(component.togglePasswordVisibility).toBeDefined();
    expect(typeof component.togglePasswordVisibility).toBe('function');
  });

  it('should have goToRegister method', () => {
    expect(component.goToRegister).toBeDefined();
    expect(typeof component.goToRegister).toBe('function');
  });

  it('should have onDoorwaySelected method', () => {
    expect(component.onDoorwaySelected).toBeDefined();
    expect(typeof component.onDoorwaySelected).toBe('function');
  });

  it('should have onDoorwayPickerCancelled method', () => {
    expect(component.onDoorwayPickerCancelled).toBeDefined();
    expect(typeof component.onDoorwayPickerCancelled).toBe('function');
  });

  it('should have goBackToDoorway method', () => {
    expect(component.goBackToDoorway).toBeDefined();
    expect(typeof component.goBackToDoorway).toBe('function');
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
  // Toggle Password Visibility
  // ==========================================================================

  it('should toggle password visibility', () => {
    expect(component.showPassword()).toBe(false);
    component.togglePasswordVisibility();
    expect(component.showPassword()).toBe(true);
    component.togglePasswordVisibility();
    expect(component.showPassword()).toBe(false);
  });

  // ==========================================================================
  // Step Navigation
  // ==========================================================================

  it('should move to credentials step when doorway selected', () => {
    component.currentStep.set('doorway');
    component.onDoorwaySelected({ id: 'test', name: 'Test', url: 'http://test', region: 'us', operator: 'test', description: 'test' } as any);
    expect(component.currentStep()).toBe('credentials');
  });

  it('should navigate to home when doorway picker cancelled', () => {
    component.onDoorwayPickerCancelled();
    expect(mockRouter.navigate).toHaveBeenCalledWith(['/']);
  });

  it('should go back to doorway step', () => {
    component.currentStep.set('credentials');
    component.goBackToDoorway();
    expect(component.currentStep()).toBe('doorway');
  });

  // ==========================================================================
  // Go To Register
  // ==========================================================================

  it('should navigate to register page', () => {
    component.goToRegister();
    expect(mockRouter.navigate).toHaveBeenCalledWith(['/identity/register'], {
      queryParams: { returnUrl: '/' },
    });
  });

  // ==========================================================================
  // Initial State
  // ==========================================================================

  it('should start with doorway step', () => {
    expect(component.currentStep()).toBe('doorway');
  });

  it('should initialize with no error', () => {
    expect(component.error()).toBeNull();
  });

  it('should initialize with loading false', () => {
    expect(component.isLoading()).toBe(false);
  });

  // ==========================================================================
  // ngOnInit - Lifecycle
  // ==========================================================================

  describe('ngOnInit', () => {
    it('should register password provider if not already registered', () => {
      mockAuthService.hasProvider.and.returnValue(false);

      component.ngOnInit();

      expect(mockAuthService.registerProvider).toHaveBeenCalledWith(mockPasswordProvider);
    });

    it('should not register password provider if already registered', () => {
      mockAuthService.hasProvider.and.returnValue(true);

      component.ngOnInit();

      expect(mockAuthService.registerProvider).not.toHaveBeenCalled();
    });

    it('should get return URL from query params', (done) => {
      mockActivatedRoute.queryParams = of({ returnUrl: '/dashboard' });

      component.ngOnInit();

      setTimeout(() => {
        expect((component as any).returnUrl).toBe('/dashboard');
        done();
      }, 100);
    });

    it('should default return URL to / when not in query params', (done) => {
      mockActivatedRoute.queryParams = of({});

      component.ngOnInit();

      setTimeout(() => {
        expect((component as any).returnUrl).toBe('/');
        done();
      }, 100);
    });

    it('should pre-fill identifier from localStorage if remembered', () => {
      (localStorage.getItem as jasmine.Spy).and.returnValue('user@example.com');

      component.ngOnInit();

      expect(component.form.identifier).toBe('user@example.com');
    });

    it('should skip to credentials step if doorway already selected', () => {
      // Override the hasSelection property with a signal that returns true
      Object.defineProperty(mockDoorwayRegistry, 'hasSelection', {
        value: signal(true),
        writable: true,
        configurable: true,
      });

      // Create a new component instance that will read the updated signal
      const newFixture = TestBed.createComponent(LoginComponent);
      const newComponent = newFixture.componentInstance;

      newComponent.ngOnInit();

      expect(newComponent.currentStep()).toBe('credentials');
    });

    it('should stay on doorway step if no doorway selected', () => {
      // The default mock already has hasSelection = signal(false)
      component.ngOnInit();

      expect(component.currentStep()).toBe('doorway');
    });

    it('should redirect if already authenticated', () => {
      mockAuthService.isAuthenticated.and.returnValue(true);

      component.ngOnInit();

      expect(mockRouter.navigate).toHaveBeenCalledWith(['/']);
    });
  });

  // ==========================================================================
  // Login Functionality
  // ==========================================================================

  describe('onLogin', () => {
    beforeEach(() => {
      component.form.identifier = 'user@example.com';
      component.form.password = 'password123';
      component.form.rememberMe = true;
    });

    it('should show error when identifier is empty', async () => {
      component.form.identifier = '';

      await component.onLogin();

      expect(component.error()).toBe('Please enter your email or username.');
      expect(mockAuthService.login).not.toHaveBeenCalled();
    });

    it('should show error when identifier is only whitespace', async () => {
      component.form.identifier = '   ';

      await component.onLogin();

      expect(component.error()).toBe('Please enter your email or username.');
      expect(mockAuthService.login).not.toHaveBeenCalled();
    });

    it('should show error when password is empty', async () => {
      component.form.password = '';

      await component.onLogin();

      expect(component.error()).toBe('Please enter your password.');
      expect(mockAuthService.login).not.toHaveBeenCalled();
    });

    it('should set loading state during login', async () => {
      let resolveLogin: any;
      mockAuthService.login.and.returnValue(
        new Promise((resolve) => {
          resolveLogin = resolve;
        })
      );

      const loginPromise = component.onLogin();

      // Should be loading immediately
      expect(component.isLoading()).toBe(true);

      // Resolve the login
      resolveLogin({
        success: true,
        token: 'mock-token',
        humanId: 'mock-human',
        agentPubKey: 'mock-pubkey',
        expiresAt: new Date(Date.now() + 3600000).toISOString(),
        identifier: 'mock-user',
      });

      await loginPromise;

      // Should no longer be loading after completion
      expect(component.isLoading()).toBe(false);
    });

    it('should login successfully with valid credentials', async () => {
      await component.onLogin();

      expect(mockAuthService.login).toHaveBeenCalledWith('password', {
        type: 'password',
        identifier: 'user@example.com',
        password: 'password123',
      });
    });

    it('should trim identifier before login', async () => {
      component.form.identifier = '  user@example.com  ';

      await component.onLogin();

      expect(mockAuthService.login).toHaveBeenCalledWith('password', {
        type: 'password',
        identifier: 'user@example.com',
        password: 'password123',
      });
    });

    it('should clear password from form after successful login', async () => {
      await component.onLogin();

      expect(component.form.password).toBe('');
    });

    it('should store identifier in localStorage when rememberMe is true', async () => {
      component.form.rememberMe = true;

      await component.onLogin();

      expect(localStorage.setItem).toHaveBeenCalledWith(AUTH_IDENTIFIER_KEY, 'user@example.com');
    });

    it('should remove identifier from localStorage when rememberMe is false', async () => {
      component.form.rememberMe = false;

      await component.onLogin();

      expect(localStorage.removeItem).toHaveBeenCalledWith(AUTH_IDENTIFIER_KEY);
    });

    it('should wait for authenticated state before navigation', async () => {
      await component.onLogin();

      expect(mockIdentityService.waitForAuthenticatedState).toHaveBeenCalledWith(3000);
    });

    it('should navigate to return URL after successful login', async () => {
      (component as any).returnUrl = '/dashboard';

      await component.onLogin();

      expect(mockRouter.navigate).toHaveBeenCalledWith(['/dashboard']);
    });

    it('should display error message on login failure', async () => {
      mockAuthService.login.and.returnValue(
        Promise.resolve({
          success: false,
          error: 'Invalid credentials',
          code: 'INVALID_CREDENTIALS',
        })
      );

      await component.onLogin();

      expect(component.error()).toBe('Invalid credentials');
      expect(component.isLoading()).toBe(false);
    });

    it('should handle login exception', async () => {
      mockAuthService.login.and.returnValue(Promise.reject(new Error('Network error')));

      await component.onLogin();

      expect(component.error()).toBe('Network error');
      expect(component.isLoading()).toBe(false);
    });

    it('should handle non-Error exceptions', async () => {
      mockAuthService.login.and.returnValue(Promise.reject(new Error('Unknown error')));

      await component.onLogin();

      expect(component.error()).toBe('Login failed');
      expect(component.isLoading()).toBe(false);
    });
  });
});
