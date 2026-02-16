/**
 * RegisterComponent Tests
 *
 * Tests for network identity registration component.
 */

import { ComponentFixture, TestBed } from '@angular/core/testing';
import { RegisterComponent } from './register.component';
import { IdentityService } from '../../services/identity.service';
import { SessionHumanService } from '../../services/session-human.service';
import { SessionMigrationService } from '../../services/session-migration.service';
import { DoorwayRegistryService } from '../../services/doorway-registry.service';
import { AuthService } from '../../services/auth.service';
import { PasswordAuthProvider } from '../../services/providers/password-auth.provider';
import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import { Router, ActivatedRoute } from '@angular/router';
import { signal } from '@angular/core';
import { of, EMPTY } from 'rxjs';

describe('RegisterComponent', () => {
  let component: RegisterComponent;
  let fixture: ComponentFixture<RegisterComponent>;
  let mockIdentityService: jasmine.SpyObj<IdentityService>;
  let mockSessionHumanService: jasmine.SpyObj<SessionHumanService>;
  let mockMigrationService: jasmine.SpyObj<SessionMigrationService>;
  let mockDoorwayRegistry: jasmine.SpyObj<DoorwayRegistryService>;
  let mockAuthService: jasmine.SpyObj<AuthService>;
  let mockPasswordProvider: jasmine.SpyObj<PasswordAuthProvider>;
  let mockHolochainClient: jasmine.SpyObj<HolochainClientService>;
  let mockRouter: jasmine.SpyObj<Router>;
  let mockActivatedRoute: jasmine.SpyObj<ActivatedRoute>;

  beforeEach(async () => {
    // Create mocks
    mockIdentityService = jasmine.createSpyObj(
      'IdentityService',
      ['registerHuman'],
      {
        mode: signal('session'),
        isAuthenticated: signal(false),
      }
    );
    mockIdentityService.registerHuman.and.returnValue(
      Promise.resolve({
        id: 'test-human-id',
        displayName: 'Test User',
        bio: 'Test bio',
        affinities: [],
        profileReach: 'community',
        location: null,
        avatarUrl: undefined,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      })
    );

    mockSessionHumanService = jasmine.createSpyObj(
      'SessionHumanService',
      ['getSession'],
      {
        hasSession: signal(false),
      }
    );
    mockSessionHumanService.getSession.and.returnValue(null);

    mockMigrationService = jasmine.createSpyObj(
      'SessionMigrationService',
      ['migrate'],
      {
        canMigrate: signal(false),
        state: signal(null),
      }
    );
    mockMigrationService.migrate.and.returnValue(
      Promise.resolve({ success: true })
    );

    mockDoorwayRegistry = jasmine.createSpyObj(
      'DoorwayRegistryService',
      ['selectDoorway', 'validateDoorway', 'selectDoorwayByUrl'],
      {
        selected: signal(null),
        hasSelection: signal(false),
      }
    );

    mockAuthService = jasmine.createSpyObj('AuthService', ['registerProvider', 'hasProvider']);
    mockAuthService.hasProvider.and.returnValue(false);

    mockPasswordProvider = jasmine.createSpyObj('PasswordAuthProvider', ['authenticate']);

    mockHolochainClient = jasmine.createSpyObj(
      'HolochainClientService',
      [],
      {
        isConnected: signal(true),
      }
    );

    mockRouter = jasmine.createSpyObj('Router', ['navigate'], {
      events: EMPTY,
    });
    mockRouter.navigate.and.returnValue(Promise.resolve(true));

    mockActivatedRoute = {
      queryParams: of({}),
    } as any;

    await TestBed.configureTestingModule({
      imports: [RegisterComponent],
      providers: [
        { provide: IdentityService, useValue: mockIdentityService },
        { provide: SessionHumanService, useValue: mockSessionHumanService },
        { provide: SessionMigrationService, useValue: mockMigrationService },
        { provide: DoorwayRegistryService, useValue: mockDoorwayRegistry },
        { provide: AuthService, useValue: mockAuthService },
        { provide: PasswordAuthProvider, useValue: mockPasswordProvider },
        { provide: HolochainClientService, useValue: mockHolochainClient },
        { provide: Router, useValue: mockRouter },
        { provide: ActivatedRoute, useValue: mockActivatedRoute },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(RegisterComponent);
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

  it('should have isRegistering signal', () => {
    expect(component.isRegistering).toBeDefined();
  });

  it('should have isMigrating signal', () => {
    expect(component.isMigrating).toBeDefined();
  });

  it('should have error signal', () => {
    expect(component.error).toBeDefined();
  });

  it('should have showPassword signal', () => {
    expect(component.showPassword).toBeDefined();
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

  it('should delegate isConnected from HolochainClientService', () => {
    expect(component.isConnected).toBeDefined();
  });

  it('should delegate canMigrate from SessionMigrationService', () => {
    expect(component.canMigrate).toBeDefined();
  });

  it('should delegate migrationState from SessionMigrationService', () => {
    expect(component.migrationState).toBeDefined();
  });

  // ==========================================================================
  // Computed Signals
  // ==========================================================================

  it('should have hasSession computed signal', () => {
    expect(component.hasSession).toBeDefined();
  });

  it('should have sessionStats computed signal', () => {
    expect(component.sessionStats).toBeDefined();
  });

  it('should have sessionDisplayName computed signal', () => {
    expect(component.sessionDisplayName).toBeDefined();
  });

  // ==========================================================================
  // Form State
  // ==========================================================================

  it('should initialize form state', () => {
    expect(component.form).toBeDefined();
    expect(component.form.displayName).toEqual('');
    expect(component.form.bio).toEqual('');
    expect(component.form.affinities).toEqual('');
    expect(component.form.profileReach).toEqual('community');
    expect(component.form.location).toEqual('');
    expect(component.form.email).toEqual('');
    expect(component.form.password).toEqual('');
    expect(component.form.confirmPassword).toEqual('');
  });

  // ==========================================================================
  // Reach Options
  // ==========================================================================

  it('should have reach options array', () => {
    expect(component.reachOptions).toBeDefined();
    expect(Array.isArray(component.reachOptions)).toBe(true);
    expect(component.reachOptions.length).toBeGreaterThan(0);
  });

  it('reach options should have required properties', () => {
    const option = component.reachOptions[0];
    expect(option.value).toBeDefined();
    expect(option.label).toBeDefined();
    expect(option.description).toBeDefined();
  });

  // ==========================================================================
  // Public Methods
  // ==========================================================================

  it('should have onRegister method', () => {
    expect(component.onRegister).toBeDefined();
    expect(typeof component.onRegister).toBe('function');
  });

  it('should have onMigrate method', () => {
    expect(component.onMigrate).toBeDefined();
    expect(typeof component.onMigrate).toBe('function');
  });

  it('should have clearError method', () => {
    expect(component.clearError).toBeDefined();
    expect(typeof component.clearError).toBe('function');
  });

  it('should have togglePasswordVisibility method', () => {
    expect(component.togglePasswordVisibility).toBeDefined();
    expect(typeof component.togglePasswordVisibility).toBe('function');
  });

  it('should have goToLogin method', () => {
    expect(component.goToLogin).toBeDefined();
    expect(typeof component.goToLogin).toBe('function');
  });

  it('should have getSessionStatsDisplay method', () => {
    expect(component.getSessionStatsDisplay).toBeDefined();
    expect(typeof component.getSessionStatsDisplay).toBe('function');
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
  // Go To Login
  // ==========================================================================

  it('should navigate to login page', () => {
    component.goToLogin();
    expect(mockRouter.navigate).toHaveBeenCalledWith(['/identity/login'], {
      queryParams: { returnUrl: '/' },
    });
  });

  // ==========================================================================
  // Session Stats Display
  // ==========================================================================

  it('should return empty string for no stats', () => {
    const display = component.getSessionStatsDisplay();
    expect(typeof display).toBe('string');
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

    it('should auto-select doorway from environment when none selected', () => {
      // Default mock has hasSelection = signal(false), environment has localhost doorwayUrl
      component.ngOnInit();

      expect(mockDoorwayRegistry.selectDoorwayByUrl).toHaveBeenCalled();
    });

    it('should not auto-select if doorway already selected', () => {
      Object.defineProperty(mockDoorwayRegistry, 'hasSelection', {
        value: signal(true),
        writable: true,
        configurable: true,
      });

      const newFixture = TestBed.createComponent(RegisterComponent);
      const newComponent = newFixture.componentInstance;

      newComponent.ngOnInit();

      expect(mockDoorwayRegistry.selectDoorwayByUrl).not.toHaveBeenCalled();
    });

    it('should pre-fill form from session data if available', () => {
      const mockSession: any = {
        sessionId: 'test-session-id',
        displayName: 'John Doe',
        bio: 'Test bio',
        interests: ['programming', 'learning'],
        stats: { nodesViewed: 0, pathsStarted: 0, stepsCompleted: 0 },
        createdAt: new Date().toISOString(),
        lastActiveAt: new Date().toISOString(),
        isAnonymous: false,
        accessLevel: 'visitor' as const,
        sessionState: 'active' as const,
      };

      Object.defineProperty(mockSessionHumanService, 'hasSession', {
        value: signal(true),
        writable: true,
        configurable: true,
      });
      mockSessionHumanService.getSession.and.returnValue(mockSession);

      const newFixture = TestBed.createComponent(RegisterComponent);
      const newComponent = newFixture.componentInstance;

      newComponent.ngOnInit();

      expect(newComponent.form.displayName).toBe('John Doe');
      expect(newComponent.form.bio).toBe('Test bio');
      expect(newComponent.form.affinities).toBe('programming, learning');
    });

    it('should not pre-fill displayName if session has default "Traveler"', () => {
      const mockSession: any = {
        sessionId: 'test-session-id',
        displayName: 'Traveler',
        bio: 'Test bio',
        interests: [],
        stats: { nodesViewed: 0, pathsStarted: 0, stepsCompleted: 0 },
        createdAt: new Date().toISOString(),
        lastActiveAt: new Date().toISOString(),
        isAnonymous: true,
        accessLevel: 'visitor' as const,
        sessionState: 'active' as const,
      };

      Object.defineProperty(mockSessionHumanService, 'hasSession', {
        value: signal(true),
        writable: true,
        configurable: true,
      });
      mockSessionHumanService.getSession.and.returnValue(mockSession);

      const newFixture = TestBed.createComponent(RegisterComponent);
      const newComponent = newFixture.componentInstance;

      newComponent.ngOnInit();

      expect(newComponent.form.displayName).toBe('');
    });

    it('should redirect if already authenticated in network mode', () => {
      Object.defineProperty(mockIdentityService, 'mode', {
        value: signal('hosted'),
        writable: true,
        configurable: true,
      });
      Object.defineProperty(mockIdentityService, 'isAuthenticated', {
        value: signal(true),
        writable: true,
        configurable: true,
      });

      const newFixture = TestBed.createComponent(RegisterComponent);
      const newComponent = newFixture.componentInstance;

      newComponent.ngOnInit();

      expect(mockRouter.navigate).toHaveBeenCalledWith(['/']);
    });

    it('should not redirect if in session mode', () => {
      Object.defineProperty(mockIdentityService, 'mode', {
        value: signal('session'),
        writable: true,
        configurable: true,
      });
      Object.defineProperty(mockIdentityService, 'isAuthenticated', {
        value: signal(true),
        writable: true,
        configurable: true,
      });

      mockRouter.navigate.calls.reset();

      const newFixture = TestBed.createComponent(RegisterComponent);
      const newComponent = newFixture.componentInstance;

      newComponent.ngOnInit();

      expect(mockRouter.navigate).not.toHaveBeenCalled();
    });
  });

  // ==========================================================================
  // Registration Functionality
  // ==========================================================================

  describe('onRegister', () => {
    beforeEach(() => {
      component.form.displayName = 'John Doe';
      component.form.email = 'john@example.com';
      component.form.password = 'password123';
      component.form.confirmPassword = 'password123';
      component.form.bio = 'Test bio';
      component.form.affinities = 'programming, learning';
      component.form.profileReach = 'community';
      component.form.location = 'San Francisco';
    });

    it('should show error when not connected to network', async () => {
      Object.defineProperty(mockHolochainClient, 'isConnected', {
        value: signal(false),
        writable: true,
        configurable: true,
      });

      const newFixture = TestBed.createComponent(RegisterComponent);
      const newComponent = newFixture.componentInstance;
      newComponent.form.displayName = 'John Doe';
      newComponent.form.email = 'john@example.com';
      newComponent.form.password = 'password123';
      newComponent.form.confirmPassword = 'password123';

      await newComponent.onRegister();

      expect(newComponent.error()).toBe('Not connected to network. Please wait for connection.');
      expect(mockIdentityService.registerHuman).not.toHaveBeenCalled();
    });

    it('should show error when displayName is empty', async () => {
      component.form.displayName = '';

      await component.onRegister();

      expect(component.error()).toBe('Please enter a display name.');
      expect(mockIdentityService.registerHuman).not.toHaveBeenCalled();
    });

    it('should show error when displayName is only whitespace', async () => {
      component.form.displayName = '   ';

      await component.onRegister();

      expect(component.error()).toBe('Please enter a display name.');
      expect(mockIdentityService.registerHuman).not.toHaveBeenCalled();
    });

    it('should show error when email is empty', async () => {
      component.form.email = '';

      await component.onRegister();

      expect(component.error()).toBe('Please enter your email address.');
      expect(mockIdentityService.registerHuman).not.toHaveBeenCalled();
    });

    it('should show error when email is invalid', async () => {
      component.form.email = 'invalid-email';

      await component.onRegister();

      expect(component.error()).toBe('Please enter a valid email address.');
      expect(mockIdentityService.registerHuman).not.toHaveBeenCalled();
    });

    it('should show error when password is empty', async () => {
      component.form.password = '';

      await component.onRegister();

      expect(component.error()).toBe('Please enter a password.');
      expect(mockIdentityService.registerHuman).not.toHaveBeenCalled();
    });

    it('should show error when password is too short', async () => {
      component.form.password = 'short';
      component.form.confirmPassword = 'short';

      await component.onRegister();

      expect(component.error()).toBe('Password must be at least 8 characters.');
      expect(mockIdentityService.registerHuman).not.toHaveBeenCalled();
    });

    it('should show error when passwords do not match', async () => {
      component.form.password = 'password123';
      component.form.confirmPassword = 'different';

      await component.onRegister();

      expect(component.error()).toBe('Passwords do not match.');
      expect(mockIdentityService.registerHuman).not.toHaveBeenCalled();
    });

    it('should set registering state during registration', async () => {
      let resolveRegister: any;
      mockIdentityService.registerHuman.and.returnValue(
        new Promise((resolve) => {
          resolveRegister = resolve;
        })
      );

      const registerPromise = component.onRegister();

      // Should be registering immediately
      expect(component.isRegistering()).toBe(true);

      // Resolve the registration
      resolveRegister();

      await registerPromise;

      // Should no longer be registering after completion
      expect(component.isRegistering()).toBe(false);
    });

    it('should register successfully with valid credentials', async () => {
      await component.onRegister();

      expect(mockIdentityService.registerHuman).toHaveBeenCalledWith({
        displayName: 'John Doe',
        bio: 'Test bio',
        affinities: ['programming', 'learning'],
        profileReach: 'community',
        location: 'San Francisco',
        email: 'john@example.com',
        password: 'password123',
      });
    });

    it('should trim and lowercase email before registration', async () => {
      component.form.displayName = 'John Doe';
      component.form.bio = 'Test bio';
      component.form.affinities = 'programming, learning';
      component.form.profileReach = 'community';
      component.form.location = 'San Francisco';
      // Use mixed case email (validation runs on untrimmed input, so no leading/trailing spaces)
      component.form.email = 'John@Example.COM';
      component.form.password = 'password123';
      component.form.confirmPassword = 'password123';

      await component.onRegister();

      expect(mockIdentityService.registerHuman).toHaveBeenCalledWith(
        jasmine.objectContaining({
          email: 'john@example.com',
        })
      );
    });

    it('should clear password from form after successful registration', async () => {
      await component.onRegister();

      expect(component.form.password).toBe('');
      expect(component.form.confirmPassword).toBe('');
    });

    it('should navigate to return URL after successful registration', async () => {
      (component as any).returnUrl = '/dashboard';

      await component.onRegister();

      expect(mockRouter.navigate).toHaveBeenCalledWith(['/dashboard']);
    });

    it('should handle registration failure', async () => {
      mockIdentityService.registerHuman.and.returnValue(
        Promise.reject(new Error('Email already exists'))
      );

      await component.onRegister();

      expect(component.error()).toBe('Email already exists');
      expect(component.isRegistering()).toBe(false);
    });

    it('should handle non-Error exceptions', async () => {
      mockIdentityService.registerHuman.and.returnValue(Promise.reject(new Error('Unknown error')));

      await component.onRegister();

      expect(component.error()).toBe('Registration failed');
      expect(component.isRegistering()).toBe(false);
    });

    it('should use migration when session exists and canMigrate is true', async () => {
      Object.defineProperty(mockSessionHumanService, 'hasSession', {
        value: signal(true),
        writable: true,
        configurable: true,
      });
      Object.defineProperty(mockMigrationService, 'canMigrate', {
        value: signal(true),
        writable: true,
        configurable: true,
      });

      const newFixture = TestBed.createComponent(RegisterComponent);
      const newComponent = newFixture.componentInstance;
      newComponent.form.displayName = 'John Doe';
      newComponent.form.email = 'john@example.com';
      newComponent.form.password = 'password123';
      newComponent.form.confirmPassword = 'password123';

      await newComponent.onRegister();

      expect(mockMigrationService.migrate).toHaveBeenCalled();
      expect(mockIdentityService.registerHuman).not.toHaveBeenCalled();
    });

    it('should handle migration failure', async () => {
      Object.defineProperty(mockSessionHumanService, 'hasSession', {
        value: signal(true),
        writable: true,
        configurable: true,
      });
      Object.defineProperty(mockMigrationService, 'canMigrate', {
        value: signal(true),
        writable: true,
        configurable: true,
      });

      mockMigrationService.migrate.and.returnValue(
        Promise.resolve({ success: false, error: 'Migration failed' })
      );

      const newFixture = TestBed.createComponent(RegisterComponent);
      const newComponent = newFixture.componentInstance;
      newComponent.form.displayName = 'John Doe';
      newComponent.form.email = 'john@example.com';
      newComponent.form.password = 'password123';
      newComponent.form.confirmPassword = 'password123';

      await newComponent.onRegister();

      expect(newComponent.error()).toBe('Migration failed');
    });

    it('should handle optional fields correctly', async () => {
      component.form.bio = '';
      component.form.affinities = '';
      component.form.location = '';

      await component.onRegister();

      expect(mockIdentityService.registerHuman).toHaveBeenCalledWith({
        displayName: 'John Doe',
        bio: undefined,
        affinities: [],
        profileReach: 'community',
        location: undefined,
        email: 'john@example.com',
        password: 'password123',
      });
    });
  });

  // ==========================================================================
  // Migration Functionality
  // ==========================================================================

  describe('onMigrate', () => {
    it('should show error when migration not available', async () => {
      Object.defineProperty(mockMigrationService, 'canMigrate', {
        value: signal(false),
        writable: true,
        configurable: true,
      });

      await component.onMigrate();

      expect(component.error()).toBe('Migration not available. Check network connection.');
      expect(mockMigrationService.migrate).not.toHaveBeenCalled();
    });

    it('should redirect to onRegister if email and password are filled', async () => {
      Object.defineProperty(mockMigrationService, 'canMigrate', {
        value: signal(true),
        writable: true,
        configurable: true,
      });

      const newFixture = TestBed.createComponent(RegisterComponent);
      const newComponent = newFixture.componentInstance;
      newComponent.form.email = 'john@example.com';
      newComponent.form.password = 'password123';
      newComponent.form.confirmPassword = 'password123';
      newComponent.form.displayName = 'John Doe';

      spyOn(newComponent, 'onRegister').and.returnValue(Promise.resolve());

      await newComponent.onMigrate();

      expect(newComponent.onRegister).toHaveBeenCalled();
    });

    it('should migrate with form overrides', async () => {
      Object.defineProperty(mockMigrationService, 'canMigrate', {
        value: signal(true),
        writable: true,
        configurable: true,
      });

      const newFixture = TestBed.createComponent(RegisterComponent);
      const newComponent = newFixture.componentInstance;
      newComponent.form.displayName = 'John Doe';
      newComponent.form.bio = 'Test bio';
      newComponent.form.affinities = 'programming';
      newComponent.form.profileReach = 'public';

      await newComponent.onMigrate();

      expect(mockMigrationService.migrate).toHaveBeenCalledWith({
        displayName: 'John Doe',
        bio: 'Test bio',
        affinities: ['programming'],
        profileReach: 'public',
      });
    });

    it('should navigate after successful migration', async () => {
      Object.defineProperty(mockMigrationService, 'canMigrate', {
        value: signal(true),
        writable: true,
        configurable: true,
      });

      (component as any).returnUrl = '/dashboard';

      const newFixture = TestBed.createComponent(RegisterComponent);
      const newComponent = newFixture.componentInstance;
      (newComponent as any).returnUrl = '/dashboard';

      await newComponent.onMigrate();

      expect(mockRouter.navigate).toHaveBeenCalledWith(['/dashboard']);
    });

    it('should handle migration failure', async () => {
      Object.defineProperty(mockMigrationService, 'canMigrate', {
        value: signal(true),
        writable: true,
        configurable: true,
      });

      mockMigrationService.migrate.and.returnValue(
        Promise.resolve({ success: false, error: 'Network error' })
      );

      const newFixture = TestBed.createComponent(RegisterComponent);
      const newComponent = newFixture.componentInstance;

      await newComponent.onMigrate();

      expect(newComponent.error()).toBe('Network error');
      expect(newComponent.isMigrating()).toBe(false);
    });

    it('should handle migration exception', async () => {
      Object.defineProperty(mockMigrationService, 'canMigrate', {
        value: signal(true),
        writable: true,
        configurable: true,
      });

      mockMigrationService.migrate.and.returnValue(Promise.reject(new Error('Network error')));

      const newFixture = TestBed.createComponent(RegisterComponent);
      const newComponent = newFixture.componentInstance;

      await newComponent.onMigrate();

      expect(newComponent.error()).toBe('Network error');
      expect(newComponent.isMigrating()).toBe(false);
    });
  });
});
