/**
 * User Store - Persistent storage for hosted human credentials.
 *
 * Supports both in-memory (for testing) and file-based (for dev/production)
 * storage. The interface is designed to be easily replaced with other
 * persistent storage (SQLite, Redis, etc.) later.
 *
 * Security notes:
 * - Passwords are stored as bcrypt hashes (hashing done by auth-service)
 * - Consider adding encryption at rest for the user store in production
 */

import bcrypt from 'bcrypt';
import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'fs';
import { dirname, join } from 'path';

// =============================================================================
// Types
// =============================================================================

/** Authentication method types (extensible for future passkey/OAuth) */
export type AuthMethodType = 'password' | 'passkey' | 'oauth';

/** An authentication method attached to a user */
export interface AuthMethod {
  type: AuthMethodType;
  /** For password: not used. For passkey: credential ID. For OAuth: provider name */
  credentialId?: string;
  /** For OAuth: the provider (google, github, etc.) */
  providerId?: string;
  createdAt: string;
  lastUsedAt?: string;
}

/** A stored user with credentials */
export interface StoredUser {
  /** Holochain human ID (from register_human zome call) */
  humanId: string;
  /** Holochain agent public key (hex string) */
  agentPubKey: string;
  /** Unique identifier (email or username) */
  identifier: string;
  /** Type of identifier */
  identifierType: 'email' | 'username';
  /** bcrypt hash of password (if password auth enabled) */
  passwordHash: string | null;
  /** Authentication methods available for this user */
  authMethods: AuthMethod[];
  /** When the user was created */
  createdAt: string;
  /** Last successful login */
  lastLoginAt: string | null;
}

/** Input for creating a new user */
export interface CreateUserInput {
  humanId: string;
  agentPubKey: string;
  identifier: string;
  identifierType: 'email' | 'username';
  password: string;
}

/** Result of user operations */
export interface UserStoreResult<T> {
  success: boolean;
  data?: T;
  error?: string;
}

// =============================================================================
// User Store Interface (for future abstraction)
// =============================================================================

export interface IUserStore {
  createUser(input: CreateUserInput): Promise<UserStoreResult<StoredUser>>;
  findByIdentifier(identifier: string): Promise<StoredUser | null>;
  findByHumanId(humanId: string): Promise<StoredUser | null>;
  validatePassword(identifier: string, password: string): Promise<UserStoreResult<StoredUser>>;
  updateLastLogin(humanId: string): Promise<void>;
  addAuthMethod(humanId: string, method: AuthMethod): Promise<UserStoreResult<void>>;
}

// =============================================================================
// In-Memory Implementation
// =============================================================================

/** bcrypt rounds for password hashing */
const BCRYPT_ROUNDS = 12;

/**
 * In-memory user store implementation.
 * Suitable for development and MVP; replace with persistent storage for production.
 */
export class InMemoryUserStore implements IUserStore {
  /** Users indexed by identifier (email/username) */
  private usersByIdentifier = new Map<string, StoredUser>();
  /** Users indexed by humanId for fast lookup */
  private usersByHumanId = new Map<string, StoredUser>();

  /**
   * Create a new user with password credentials.
   */
  async createUser(input: CreateUserInput): Promise<UserStoreResult<StoredUser>> {
    // Normalize identifier (lowercase for email)
    const normalizedIdentifier = input.identifierType === 'email'
      ? input.identifier.toLowerCase().trim()
      : input.identifier.trim();

    // Check for existing user
    if (this.usersByIdentifier.has(normalizedIdentifier)) {
      return {
        success: false,
        error: 'A user with this identifier already exists',
      };
    }

    if (this.usersByHumanId.has(input.humanId)) {
      return {
        success: false,
        error: 'This Holochain identity already has credentials',
      };
    }

    // Hash password
    const passwordHash = await bcrypt.hash(input.password, BCRYPT_ROUNDS);

    const now = new Date().toISOString();
    const user: StoredUser = {
      humanId: input.humanId,
      agentPubKey: input.agentPubKey,
      identifier: normalizedIdentifier,
      identifierType: input.identifierType,
      passwordHash,
      authMethods: [
        {
          type: 'password',
          createdAt: now,
        },
      ],
      createdAt: now,
      lastLoginAt: null,
    };

    // Store in both indexes
    this.usersByIdentifier.set(normalizedIdentifier, user);
    this.usersByHumanId.set(input.humanId, user);

    // Return user without exposing password hash in logs
    console.log(`[UserStore] Created user: ${normalizedIdentifier} -> ${input.humanId}`);

    return {
      success: true,
      data: user,
    };
  }

  /**
   * Find a user by their identifier (email or username).
   */
  async findByIdentifier(identifier: string): Promise<StoredUser | null> {
    const normalized = identifier.toLowerCase().trim();
    return this.usersByIdentifier.get(normalized) ?? null;
  }

  /**
   * Find a user by their Holochain human ID.
   */
  async findByHumanId(humanId: string): Promise<StoredUser | null> {
    return this.usersByHumanId.get(humanId) ?? null;
  }

  /**
   * Validate password and return user if successful.
   */
  async validatePassword(
    identifier: string,
    password: string
  ): Promise<UserStoreResult<StoredUser>> {
    const user = await this.findByIdentifier(identifier);

    if (!user) {
      // Use generic error to prevent user enumeration
      return {
        success: false,
        error: 'Invalid credentials',
      };
    }

    if (!user.passwordHash) {
      return {
        success: false,
        error: 'Password authentication not enabled for this user',
      };
    }

    const isValid = await bcrypt.compare(password, user.passwordHash);

    if (!isValid) {
      return {
        success: false,
        error: 'Invalid credentials',
      };
    }

    return {
      success: true,
      data: user,
    };
  }

  /**
   * Update last login timestamp.
   */
  async updateLastLogin(humanId: string): Promise<void> {
    const user = this.usersByHumanId.get(humanId);
    if (user) {
      user.lastLoginAt = new Date().toISOString();
    }
  }

  /**
   * Add an authentication method to a user (for future passkey/OAuth support).
   */
  async addAuthMethod(
    humanId: string,
    method: AuthMethod
  ): Promise<UserStoreResult<void>> {
    const user = this.usersByHumanId.get(humanId);

    if (!user) {
      return {
        success: false,
        error: 'User not found',
      };
    }

    // Check for duplicate auth method
    const exists = user.authMethods.some(m =>
      m.type === method.type &&
      m.credentialId === method.credentialId &&
      m.providerId === method.providerId
    );

    if (exists) {
      return {
        success: false,
        error: 'Authentication method already exists',
      };
    }

    user.authMethods.push(method);
    console.log(`[UserStore] Added ${method.type} auth method to user ${humanId}`);

    return { success: true };
  }

  /**
   * Get stats for debugging/monitoring.
   */
  getStats(): { userCount: number } {
    return {
      userCount: this.usersByIdentifier.size,
    };
  }

  /**
   * Clear all users (for testing).
   */
  clear(): void {
    this.usersByIdentifier.clear();
    this.usersByHumanId.clear();
  }
}

// =============================================================================
// File-Based Implementation
// =============================================================================

/** Data structure for JSON file storage */
interface UserStoreData {
  version: number;
  users: StoredUser[];
}

/**
 * File-based user store implementation.
 * Persists users to a JSON file for durability across restarts.
 */
export class FileUserStore implements IUserStore {
  /** Users indexed by identifier (email/username) */
  private usersByIdentifier = new Map<string, StoredUser>();
  /** Users indexed by humanId for fast lookup */
  private usersByHumanId = new Map<string, StoredUser>();
  /** Path to the JSON file */
  private readonly filePath: string;

  constructor(filePath: string) {
    this.filePath = filePath;
    this.load();
  }

  /**
   * Load users from file.
   */
  private load(): void {
    if (!existsSync(this.filePath)) {
      console.log(`[UserStore] No existing data file at ${this.filePath}`);
      return;
    }

    try {
      const data = readFileSync(this.filePath, 'utf-8');
      const parsed = JSON.parse(data) as UserStoreData;

      for (const user of parsed.users) {
        const normalized = user.identifier.toLowerCase().trim();
        this.usersByIdentifier.set(normalized, user);
        this.usersByHumanId.set(user.humanId, user);
      }

      console.log(`[UserStore] Loaded ${parsed.users.length} users from ${this.filePath}`);
    } catch (err) {
      console.error(`[UserStore] Failed to load from ${this.filePath}:`, err);
    }
  }

  /**
   * Save users to file.
   */
  private save(): void {
    try {
      // Ensure directory exists
      const dir = dirname(this.filePath);
      if (!existsSync(dir)) {
        mkdirSync(dir, { recursive: true });
      }

      const data: UserStoreData = {
        version: 1,
        users: Array.from(this.usersByIdentifier.values()),
      };

      writeFileSync(this.filePath, JSON.stringify(data, null, 2), 'utf-8');
    } catch (err) {
      console.error(`[UserStore] Failed to save to ${this.filePath}:`, err);
    }
  }

  /**
   * Create a new user with password credentials.
   */
  async createUser(input: CreateUserInput): Promise<UserStoreResult<StoredUser>> {
    // Normalize identifier (lowercase for email)
    const normalizedIdentifier = input.identifierType === 'email'
      ? input.identifier.toLowerCase().trim()
      : input.identifier.trim();

    // Check for existing user
    if (this.usersByIdentifier.has(normalizedIdentifier)) {
      return {
        success: false,
        error: 'A user with this identifier already exists',
      };
    }

    if (this.usersByHumanId.has(input.humanId)) {
      return {
        success: false,
        error: 'This Holochain identity already has credentials',
      };
    }

    // Hash password
    const passwordHash = await bcrypt.hash(input.password, BCRYPT_ROUNDS);

    const now = new Date().toISOString();
    const user: StoredUser = {
      humanId: input.humanId,
      agentPubKey: input.agentPubKey,
      identifier: normalizedIdentifier,
      identifierType: input.identifierType,
      passwordHash,
      authMethods: [
        {
          type: 'password',
          createdAt: now,
        },
      ],
      createdAt: now,
      lastLoginAt: null,
    };

    // Store in both indexes
    this.usersByIdentifier.set(normalizedIdentifier, user);
    this.usersByHumanId.set(input.humanId, user);

    // Persist to file
    this.save();

    console.log(`[UserStore] Created user: ${normalizedIdentifier} -> ${input.humanId}`);

    return {
      success: true,
      data: user,
    };
  }

  /**
   * Find a user by their identifier (email or username).
   */
  async findByIdentifier(identifier: string): Promise<StoredUser | null> {
    const normalized = identifier.toLowerCase().trim();
    return this.usersByIdentifier.get(normalized) ?? null;
  }

  /**
   * Find a user by their Holochain human ID.
   */
  async findByHumanId(humanId: string): Promise<StoredUser | null> {
    return this.usersByHumanId.get(humanId) ?? null;
  }

  /**
   * Validate password and return user if successful.
   */
  async validatePassword(
    identifier: string,
    password: string
  ): Promise<UserStoreResult<StoredUser>> {
    const user = await this.findByIdentifier(identifier);

    if (!user) {
      return {
        success: false,
        error: 'Invalid credentials',
      };
    }

    if (!user.passwordHash) {
      return {
        success: false,
        error: 'Password authentication not enabled for this user',
      };
    }

    const isValid = await bcrypt.compare(password, user.passwordHash);

    if (!isValid) {
      return {
        success: false,
        error: 'Invalid credentials',
      };
    }

    return {
      success: true,
      data: user,
    };
  }

  /**
   * Update last login timestamp.
   */
  async updateLastLogin(humanId: string): Promise<void> {
    const user = this.usersByHumanId.get(humanId);
    if (user) {
      user.lastLoginAt = new Date().toISOString();
      this.save();
    }
  }

  /**
   * Add an authentication method to a user.
   */
  async addAuthMethod(
    humanId: string,
    method: AuthMethod
  ): Promise<UserStoreResult<void>> {
    const user = this.usersByHumanId.get(humanId);

    if (!user) {
      return {
        success: false,
        error: 'User not found',
      };
    }

    const exists = user.authMethods.some(m =>
      m.type === method.type &&
      m.credentialId === method.credentialId &&
      m.providerId === method.providerId
    );

    if (exists) {
      return {
        success: false,
        error: 'Authentication method already exists',
      };
    }

    user.authMethods.push(method);
    this.save();
    console.log(`[UserStore] Added ${method.type} auth method to user ${humanId}`);

    return { success: true };
  }

  /**
   * Get stats for debugging/monitoring.
   */
  getStats(): { userCount: number } {
    return {
      userCount: this.usersByIdentifier.size,
    };
  }

  /**
   * Clear all users.
   */
  clear(): void {
    this.usersByIdentifier.clear();
    this.usersByHumanId.clear();
    this.save();
  }
}

// =============================================================================
// Singleton Instance
// =============================================================================

/** Global user store instance */
let userStoreInstance: IUserStore | null = null;

/** Default data directory */
const DEFAULT_DATA_DIR = join(process.cwd(), 'data');
const DEFAULT_USER_FILE = join(DEFAULT_DATA_DIR, 'users.json');

/**
 * Initialize the user store.
 * Call this once at startup with optional config.
 *
 * @param options - Configuration options
 * @param options.filePath - Path to users.json file (default: ./data/users.json)
 * @param options.inMemory - Use in-memory store (for testing)
 */
export function initUserStore(options?: {
  filePath?: string;
  inMemory?: boolean;
}): IUserStore {
  if (options?.inMemory) {
    userStoreInstance = new InMemoryUserStore();
    console.log('[UserStore] Using in-memory store');
  } else {
    const filePath = options?.filePath ?? DEFAULT_USER_FILE;
    userStoreInstance = new FileUserStore(filePath);
    console.log(`[UserStore] Using file-based store: ${filePath}`);
  }
  return userStoreInstance;
}

/**
 * Get the global user store instance.
 * Initializes with file-based store if not already initialized.
 */
export function getUserStore(): IUserStore {
  if (!userStoreInstance) {
    return initUserStore();
  }
  return userStoreInstance;
}

/**
 * Reset the user store (for testing).
 */
export function resetUserStore(): void {
  if (userStoreInstance && 'clear' in userStoreInstance) {
    (userStoreInstance as InMemoryUserStore | FileUserStore).clear();
  }
  userStoreInstance = null;
}
