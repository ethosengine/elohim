/**
 * Test data factories for creating realistic test objects
 *
 * These factories create objects with sensible defaults that can be
 * overridden for specific test cases.
 */

// ============================================================================
// ID Generator
// ============================================================================

let idCounter = 0;

function generateId(prefix: string): string {
  return `${prefix}-${Date.now()}-${++idCounter}`;
}

// ============================================================================
// Content Node Factory
// ============================================================================

export interface TestContentNode {
  id: string;
  contentType: string;
  title: string;
  description: string;
  contentBody: string;
  contentFormat: string;
  tags: string[];
  relatedNodeIds: string[];
  createdAt: string;
  updatedAt: string;
  blobCid?: string;
}

export function createTestContentNode(overrides: Partial<TestContentNode> = {}): TestContentNode {
  const now = new Date().toISOString();

  return {
    id: generateId('node'),
    contentType: 'concept',
    title: 'Test Content',
    description: 'Test description for content node',
    contentBody: '# Test Content\n\nThis is test content for unit testing.',
    contentFormat: 'markdown',
    tags: ['test'],
    relatedNodeIds: [],
    createdAt: now,
    updatedAt: now,
    ...overrides,
  };
}

export function createTestQuizNode(overrides: Partial<TestContentNode> = {}): TestContentNode {
  return createTestContentNode({
    contentType: 'quiz',
    contentFormat: 'sophia',
    contentBody: JSON.stringify({
      moments: [
        {
          type: 'question',
          text: 'What is 2 + 2?',
          answers: [
            { text: '3', correct: false },
            { text: '4', correct: true },
            { text: '5', correct: false },
          ],
        },
      ],
    }),
    ...overrides,
  });
}

// ============================================================================
// Human Factory
// ============================================================================

export interface TestHuman {
  id: string;
  displayName: string;
  email?: string;
  bio?: string;
  avatarUrl?: string;
  createdAt: string;
  updatedAt: string;
}

export function createTestHuman(overrides: Partial<TestHuman> = {}): TestHuman {
  const now = new Date().toISOString();

  return {
    id: generateId('human'),
    displayName: 'Test User',
    email: 'test@example.com',
    bio: 'A test user for unit testing',
    createdAt: now,
    updatedAt: now,
    ...overrides,
  };
}

// ============================================================================
// Learning Path Factory
// ============================================================================

export interface TestPathStep {
  id: string;
  nodeId: string;
  order: number;
  title?: string;
}

export interface TestLearningPath {
  id: string;
  title: string;
  description: string;
  steps: TestPathStep[];
  createdAt: string;
  updatedAt: string;
}

export function createTestPath(overrides: Partial<TestLearningPath> = {}): TestLearningPath {
  const now = new Date().toISOString();

  return {
    id: generateId('path'),
    title: 'Test Learning Path',
    description: 'A test learning path for unit testing',
    steps: [
      { id: 'step-1', nodeId: 'concept-intro', order: 1, title: 'Introduction' },
      { id: 'step-2', nodeId: 'concept-basics', order: 2, title: 'Basics' },
      { id: 'step-3', nodeId: 'quiz-basics', order: 3, title: 'Quiz: Basics' },
    ],
    createdAt: now,
    updatedAt: now,
    ...overrides,
  };
}

// ============================================================================
// Mastery Factory
// ============================================================================

export interface TestMastery {
  nodeId: string;
  humanId: string;
  level: number;
  score: number;
  attempts: number;
  lastAttemptAt: string;
}

export function createTestMastery(overrides: Partial<TestMastery> = {}): TestMastery {
  return {
    nodeId: generateId('node'),
    humanId: generateId('human'),
    level: 0,
    score: 0,
    attempts: 0,
    lastAttemptAt: new Date().toISOString(),
    ...overrides,
  };
}

// ============================================================================
// Presence Factory
// ============================================================================

export interface TestPresence {
  humanId: string;
  status: 'online' | 'away' | 'offline';
  lastSeenAt: string;
  activity?: string;
}

export function createTestPresence(overrides: Partial<TestPresence> = {}): TestPresence {
  return {
    humanId: generateId('human'),
    status: 'online',
    lastSeenAt: new Date().toISOString(),
    ...overrides,
  };
}

// ============================================================================
// Economic Event Factory (Shefa)
// ============================================================================

export interface TestEconomicEvent {
  id: string;
  type: 'transfer' | 'appreciation' | 'allocation';
  fromId?: string;
  toId: string;
  amount: number;
  currency: string;
  description?: string;
  createdAt: string;
}

export function createTestEconomicEvent(
  overrides: Partial<TestEconomicEvent> = {}
): TestEconomicEvent {
  return {
    id: generateId('event'),
    type: 'transfer',
    toId: generateId('human'),
    amount: 100,
    currency: 'credits',
    createdAt: new Date().toISOString(),
    ...overrides,
  };
}

// ============================================================================
// Proposal Factory (Qahal)
// ============================================================================

export interface TestProposal {
  id: string;
  title: string;
  description: string;
  status: 'draft' | 'active' | 'passed' | 'rejected';
  votesFor: number;
  votesAgainst: number;
  deadline: string;
  createdBy: string;
  createdAt: string;
}

export function createTestProposal(overrides: Partial<TestProposal> = {}): TestProposal {
  return {
    id: generateId('proposal'),
    title: 'Test Proposal',
    description: 'A test proposal for community voting',
    status: 'active',
    votesFor: 10,
    votesAgainst: 5,
    deadline: new Date(Date.now() + 7 * 24 * 60 * 60 * 1000).toISOString(), // 7 days from now
    createdBy: generateId('human'),
    createdAt: new Date().toISOString(),
    ...overrides,
  };
}

// ============================================================================
// Doorway Status Factory
// ============================================================================

export interface TestDoorwayStatus {
  healthy: boolean;
  connectedNodes: number;
  uptime: number;
  version: string;
}

export function createTestDoorwayStatus(
  overrides: Partial<TestDoorwayStatus> = {}
): TestDoorwayStatus {
  return {
    healthy: true,
    connectedNodes: 5,
    uptime: 86400,
    version: '1.0.0',
    ...overrides,
  };
}

// ============================================================================
// Staged Transaction Factory (Shefa)
// ============================================================================

export interface TestStagedTransaction {
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
  budgetId?: string;
  budgetCategoryId?: string;
  isDuplicate: boolean;
  reviewStatus: 'pending' | 'approved' | 'rejected' | 'needs-attention';
  plaidRawData: Record<string, unknown>;
  createdAt: string;
}

export function createTestStagedTransaction(
  overrides: Partial<TestStagedTransaction> = {}
): TestStagedTransaction {
  const now = new Date().toISOString();

  return {
    id: generateId('staged'),
    batchId: generateId('batch'),
    stewardId: generateId('steward'),
    plaidTransactionId: generateId('plaid-tx'),
    plaidAccountId: generateId('plaid-acct'),
    financialAssetId: generateId('asset'),
    timestamp: now,
    type: 'debit',
    amount: { value: 50.0, unit: 'USD' },
    description: 'Test transaction',
    merchantName: 'Test Merchant',
    category: 'Groceries',
    categoryConfidence: 85,
    categorySource: 'ai',
    isDuplicate: false,
    reviewStatus: 'pending',
    plaidRawData: {},
    createdAt: now,
    ...overrides,
  };
}

// ============================================================================
// Appreciation Factory (Shefa)
// ============================================================================

export interface TestAppreciationDisplay {
  id: string;
  appreciationOf: string;
  appreciatedBy: string;
  appreciationTo: string;
  quantityValue: number;
  quantityUnit: string;
  note: string | null;
  createdAt: string;
}

export function createTestAppreciation(
  overrides: Partial<TestAppreciationDisplay> = {}
): TestAppreciationDisplay {
  return {
    id: generateId('appreciation'),
    appreciationOf: generateId('content'),
    appreciatedBy: generateId('human'),
    appreciationTo: generateId('human'),
    quantityValue: 1,
    quantityUnit: 'recognition-points',
    note: null,
    createdAt: new Date().toISOString(),
    ...overrides,
  };
}

// ============================================================================
// Economic Event View Factory (Shefa/Lamad)
// ============================================================================

export interface TestEconomicEventView {
  id: string;
  action: string;
  provider: string;
  receiver: string;
  lamadEventType?: string;
  contentId?: string;
  pathId?: string;
  metadata?: Record<string, unknown>;
  resourceQuantity?: { value: number; unit: string };
  createdAt: string;
}

export function createTestEconomicEventView(
  overrides: Partial<TestEconomicEventView> = {}
): TestEconomicEventView {
  return {
    id: generateId('event'),
    action: 'use',
    provider: generateId('human'),
    receiver: generateId('content'),
    createdAt: new Date().toISOString(),
    ...overrides,
  };
}
