/**
 * Banking Store - IndexedDB Storage for Banking Bridge
 *
 * All Plaid-related data is stored locally in IndexedDB, completely
 * isolated from Holochain. Only approved transactions become EconomicEvents
 * on the DHT.
 *
 * This is intentional: banking credentials and staging data are personal
 * convenience, not network signals.
 */

const DB_NAME = 'elohim-banking-bridge';
const DB_VERSION = 1;

// Object store names
const STORES = {
  CONNECTIONS: 'plaid_connections',
  BATCHES: 'import_batches',
  STAGED: 'staged_transactions',
  RULES: 'transaction_rules',
  CORRECTIONS: 'correction_records',
} as const;

export interface PlaidConnectionLocal {
  id: string;
  connectionNumber: string;
  stewardId: string;
  plaidItemId: string;
  plaidAccessTokenEncrypted: string; // AES-GCM encrypted
  plaidInstitutionId: string;
  institutionName: string;
  linkedAccounts: PlaidAccountLinkLocal[];
  status: 'active' | 'requires-reauth' | 'disconnected' | 'error';
  lastSyncedAt?: string;
  errorMessage?: string;
  createdAt: string;
  updatedAt: string;
}

export interface PlaidAccountLinkLocal {
  plaidAccountId: string;
  plaidAccountName: string;
  plaidAccountSubtype: string;
  financialAssetId: string;
  balanceAmount: number;
  currency: string;
  lastLinkedAt: string;
}

export interface ImportBatchLocal {
  id: string;
  batchNumber: string;
  stewardId: string;
  connectionId: string;
  accountIds: string[];
  dateRange: { start: string; end: string };
  totalTransactions: number;
  newTransactions: number;
  duplicateTransactions: number;
  errorTransactions: number;
  status:
    | 'fetching'
    | 'categorizing'
    | 'staged'
    | 'reviewing'
    | 'approved'
    | 'completed'
    | 'rejected';
  stagedTransactionIds: string[];
  aiCategorizationEnabled: boolean;
  aiCategorizationCompletedAt?: string;
  reviewedBy?: string;
  reviewedAt?: string;
  reviewNotes?: string;
  createdAt: string;
  completedAt?: string;
}

export interface StagedTransactionLocal {
  id: string;
  batchId: string;
  stewardId: string;
  plaidTransactionId: string;
  plaidAccountId: string;
  financialAssetId: string;
  timestamp: string;
  type: 'debit' | 'credit' | 'transfer' | 'fee';
  amount: { value: number; unit: string };
  description: string;
  merchantName?: string;
  category: string;
  categoryConfidence: number;
  categorySource: 'ai' | 'plaid' | 'manual' | 'rule';
  suggestedCategories?: {
    category: string;
    confidence: number;
    reasoning?: string;
    source: 'ai' | 'pattern' | 'keyword' | 'historical';
  }[];
  budgetId?: string;
  budgetCategoryId?: string;
  isDuplicate: boolean;
  duplicateOfTransactionId?: string;
  duplicateConfidence?: number;
  reviewStatus: 'pending' | 'approved' | 'rejected' | 'needs-attention';
  reviewedBy?: string;
  reviewedAt?: string;
  economicEventId?: string; // Set after approval → Holochain
  plaidRawData: Record<string, unknown>;
  createdAt: string;
}

export interface TransactionRuleLocal {
  id: string;
  ruleNumber: string;
  stewardId: string;
  name: string;
  description?: string;
  matchType: 'exact' | 'contains' | 'starts-with' | 'regex' | 'merchant' | 'amount-range';
  matchField: 'description' | 'merchant' | 'amount' | 'account';
  matchValue: string | number;
  matchValueMax?: number;
  action: 'categorize' | 'flag' | 'auto-approve' | 'auto-reject';
  targetCategory?: string;
  targetBudgetId?: string;
  learnFromCorrections: boolean;
  accuracyRate?: number;
  priority: number;
  enabled: boolean;
  createdAt: string;
  updatedAt?: string;
}

export interface CorrectionRecordLocal {
  id: string;
  stewardId: string;
  transactionDescription: string;
  merchantName?: string;
  transactionAmount: number;
  originalCategory: string;
  originalConfidence: number;
  correctedCategory: string;
  correctionReason?: string;
  improvedAccuracy: boolean;
  ruleCreatedFrom: boolean;
  timestamp: string;
}

/**
 * BankingStore provides IndexedDB operations for all banking bridge data.
 * Singleton pattern - one database connection shared across the app.
 */
export class BankingStore {
  private static instance: BankingStore | null = null;
  private db: IDBDatabase | null = null;
  private initPromise: Promise<void> | null = null;

  private constructor() {
    // Private constructor enforces singleton pattern. Use getInstance() to get the instance.
  }

  static getInstance(): BankingStore {
    BankingStore.instance ??= new BankingStore();
    return BankingStore.instance;
  }

  async init(): Promise<void> {
    if (this.db) return;
    if (this.initPromise) return this.initPromise;

    this.initPromise = new Promise((resolve, reject) => {
      const request = indexedDB.open(DB_NAME, DB_VERSION);

      request.onerror = () => {
        reject(request.error);
      };

      request.onsuccess = () => {
        this.db = request.result;

        resolve();
      };

      request.onupgradeneeded = event => {
        const db = (event.target as IDBOpenDBRequest).result;
        this.createStores(db);
      };
    });

    return this.initPromise;
  }

  private createStores(db: IDBDatabase): void {
    // PlaidConnections - indexed by stewardId
    if (!db.objectStoreNames.contains(STORES.CONNECTIONS)) {
      const store = db.createObjectStore(STORES.CONNECTIONS, { keyPath: 'id' });
      store.createIndex('stewardId', 'stewardId', { unique: false });
      store.createIndex('status', 'status', { unique: false });
    }

    // ImportBatches - indexed by connectionId, stewardId
    if (!db.objectStoreNames.contains(STORES.BATCHES)) {
      const store = db.createObjectStore(STORES.BATCHES, { keyPath: 'id' });
      store.createIndex('connectionId', 'connectionId', { unique: false });
      store.createIndex('stewardId', 'stewardId', { unique: false });
      store.createIndex('status', 'status', { unique: false });
    }

    // StagedTransactions - indexed by batchId, plaidTransactionId
    if (!db.objectStoreNames.contains(STORES.STAGED)) {
      const store = db.createObjectStore(STORES.STAGED, { keyPath: 'id' });
      store.createIndex('batchId', 'batchId', { unique: false });
      store.createIndex('plaidTransactionId', 'plaidTransactionId', { unique: true });
      store.createIndex('reviewStatus', 'reviewStatus', { unique: false });
      store.createIndex('stewardId', 'stewardId', { unique: false });
    }

    // TransactionRules - indexed by stewardId
    if (!db.objectStoreNames.contains(STORES.RULES)) {
      const store = db.createObjectStore(STORES.RULES, { keyPath: 'id' });
      store.createIndex('stewardId', 'stewardId', { unique: false });
      store.createIndex('enabled', 'enabled', { unique: false });
    }

    // CorrectionRecords - indexed by stewardId
    if (!db.objectStoreNames.contains(STORES.CORRECTIONS)) {
      const store = db.createObjectStore(STORES.CORRECTIONS, { keyPath: 'id' });
      store.createIndex('stewardId', 'stewardId', { unique: false });
      store.createIndex('merchantName', 'merchantName', { unique: false });
    }
  }

  // ==========================================================================
  // GENERIC CRUD OPERATIONS
  // ==========================================================================

  private async getStore(
    storeName: string,
    mode: IDBTransactionMode = 'readonly'
  ): Promise<IDBObjectStore> {
    await this.init();
    if (!this.db) throw new Error('Database not initialized');
    const tx = this.db.transaction(storeName, mode);
    return tx.objectStore(storeName);
  }

  private async wrapRequest<T>(request: IDBRequest<T>): Promise<T> {
    return new Promise((resolve, reject) => {
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
  }

  // ==========================================================================
  // PLAID CONNECTIONS
  // ==========================================================================

  async saveConnection(connection: PlaidConnectionLocal): Promise<void> {
    const store = await this.getStore(STORES.CONNECTIONS, 'readwrite');
    await this.wrapRequest(store.put(connection));
  }

  async getConnection(id: string): Promise<PlaidConnectionLocal | undefined> {
    const store = await this.getStore(STORES.CONNECTIONS);
    return this.wrapRequest(store.get(id));
  }

  async getConnectionsByAgent(stewardId: string): Promise<PlaidConnectionLocal[]> {
    const store = await this.getStore(STORES.CONNECTIONS);
    const index = store.index('stewardId');
    return this.wrapRequest(index.getAll(stewardId));
  }

  async deleteConnection(id: string): Promise<void> {
    const store = await this.getStore(STORES.CONNECTIONS, 'readwrite');
    await this.wrapRequest(store.delete(id));
  }

  // ==========================================================================
  // IMPORT BATCHES
  // ==========================================================================

  async saveBatch(batch: ImportBatchLocal): Promise<void> {
    const store = await this.getStore(STORES.BATCHES, 'readwrite');
    await this.wrapRequest(store.put(batch));
  }

  async getBatch(id: string): Promise<ImportBatchLocal | undefined> {
    const store = await this.getStore(STORES.BATCHES);
    return this.wrapRequest(store.get(id));
  }

  async getBatchesByConnection(connectionId: string): Promise<ImportBatchLocal[]> {
    const store = await this.getStore(STORES.BATCHES);
    const index = store.index('connectionId');
    return this.wrapRequest(index.getAll(connectionId));
  }

  async getBatchesByStatus(status: ImportBatchLocal['status']): Promise<ImportBatchLocal[]> {
    const store = await this.getStore(STORES.BATCHES);
    const index = store.index('status');
    return this.wrapRequest(index.getAll(status));
  }

  async deleteBatch(id: string): Promise<void> {
    const store = await this.getStore(STORES.BATCHES, 'readwrite');
    await this.wrapRequest(store.delete(id));
  }

  // ==========================================================================
  // STAGED TRANSACTIONS
  // ==========================================================================

  async saveStaged(transaction: StagedTransactionLocal): Promise<void> {
    const store = await this.getStore(STORES.STAGED, 'readwrite');
    await this.wrapRequest(store.put(transaction));
  }

  async saveStagedBulk(transactions: StagedTransactionLocal[]): Promise<void> {
    const store = await this.getStore(STORES.STAGED, 'readwrite');
    for (const tx of transactions) {
      store.put(tx);
    }
  }

  async getStaged(id: string): Promise<StagedTransactionLocal | undefined> {
    const store = await this.getStore(STORES.STAGED);
    return this.wrapRequest(store.get(id));
  }

  async getStagedByBatch(batchId: string): Promise<StagedTransactionLocal[]> {
    const store = await this.getStore(STORES.STAGED);
    const index = store.index('batchId');
    return this.wrapRequest(index.getAll(batchId));
  }

  async getStagedByStatus(
    status: StagedTransactionLocal['reviewStatus']
  ): Promise<StagedTransactionLocal[]> {
    const store = await this.getStore(STORES.STAGED);
    const index = store.index('reviewStatus');
    return this.wrapRequest(index.getAll(status));
  }

  async getStagedPending(): Promise<StagedTransactionLocal[]> {
    return this.getStagedByStatus('pending');
  }

  async checkDuplicate(plaidTransactionId: string): Promise<StagedTransactionLocal | undefined> {
    const store = await this.getStore(STORES.STAGED);
    const index = store.index('plaidTransactionId');
    return this.wrapRequest(index.get(plaidTransactionId));
  }

  async deleteStaged(id: string): Promise<void> {
    const store = await this.getStore(STORES.STAGED, 'readwrite');
    await this.wrapRequest(store.delete(id));
  }

  async deleteStagedByBatch(batchId: string): Promise<void> {
    const transactions = await this.getStagedByBatch(batchId);
    const store = await this.getStore(STORES.STAGED, 'readwrite');
    for (const tx of transactions) {
      store.delete(tx.id);
    }
  }

  // ==========================================================================
  // TRANSACTION RULES
  // ==========================================================================

  async saveRule(rule: TransactionRuleLocal): Promise<void> {
    const store = await this.getStore(STORES.RULES, 'readwrite');
    await this.wrapRequest(store.put(rule));
  }

  async getRule(id: string): Promise<TransactionRuleLocal | undefined> {
    const store = await this.getStore(STORES.RULES);
    return this.wrapRequest(store.get(id));
  }

  async getRulesByAgent(stewardId: string): Promise<TransactionRuleLocal[]> {
    const store = await this.getStore(STORES.RULES);
    const index = store.index('stewardId');
    const rules = await this.wrapRequest(index.getAll(stewardId));
    // Sort by priority (higher first)
    return rules.sort((a, b) => b.priority - a.priority);
  }

  async getEnabledRules(stewardId: string): Promise<TransactionRuleLocal[]> {
    const allRules = await this.getRulesByAgent(stewardId);
    return allRules.filter(r => r.enabled);
  }

  async deleteRule(id: string): Promise<void> {
    const store = await this.getStore(STORES.RULES, 'readwrite');
    await this.wrapRequest(store.delete(id));
  }

  // ==========================================================================
  // CORRECTION RECORDS
  // ==========================================================================

  async saveCorrection(correction: CorrectionRecordLocal): Promise<void> {
    const store = await this.getStore(STORES.CORRECTIONS, 'readwrite');
    await this.wrapRequest(store.put(correction));
  }

  async getCorrectionsByMerchant(merchantName: string): Promise<CorrectionRecordLocal[]> {
    const store = await this.getStore(STORES.CORRECTIONS);
    const index = store.index('merchantName');
    return this.wrapRequest(index.getAll(merchantName));
  }

  async getCorrectionsByAgent(stewardId: string): Promise<CorrectionRecordLocal[]> {
    const store = await this.getStore(STORES.CORRECTIONS);
    const index = store.index('stewardId');
    return this.wrapRequest(index.getAll(stewardId));
  }

  // ==========================================================================
  // CLEANUP
  // ==========================================================================

  async clearAllData(): Promise<void> {
    await this.init();
    if (!this.db) return;

    const tx = this.db.transaction(Object.values(STORES), 'readwrite');
    for (const storeName of Object.values(STORES)) {
      tx.objectStore(storeName).clear();
    }
  }

  async clearCompletedBatches(olderThanDays = 30): Promise<number> {
    const batches = await this.getBatchesByStatus('completed');
    const cutoff = Date.now() - olderThanDays * 24 * 60 * 60 * 1000;
    let deleted = 0;

    for (const batch of batches) {
      if (batch.completedAt && new Date(batch.completedAt).getTime() < cutoff) {
        await this.deleteStagedByBatch(batch.id);
        await this.deleteBatch(batch.id);
        deleted++;
      }
    }

    return deleted;
  }
}

// Export singleton
export const bankingStore = BankingStore.getInstance();
