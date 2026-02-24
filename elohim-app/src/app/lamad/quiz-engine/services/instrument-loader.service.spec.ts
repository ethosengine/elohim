import { TestBed } from '@angular/core/testing';

import { of, throwError } from 'rxjs';

import { ContentService } from '@app/elohim/services/content.service';

import type { ContentNode } from '@app/lamad/models/content-node.model';

import {
  registerFromDefinition,
  getInstrument,
  getRegisteredInstrumentIds,
} from '../instruments/instrument-registry';
import type { InstrumentDefinition } from '../models/instrument-definition.model';

import { InstrumentLoaderService } from './instrument-loader.service';

// =============================================================================
// Helpers
// =============================================================================

interface InterpretResult {
  primaryType: { typeId: string; typeName: string; description?: string };
}

// =============================================================================
// Fixtures
// =============================================================================

const buildInstrumentDefinition = (
  overrides: Partial<InstrumentDefinition> = {}
): InstrumentDefinition => ({
  id: 'test-instrument',
  name: 'Test Instrument',
  version: '1.0.0',
  category: 'personality',
  framework: 'test-framework',
  description: 'A test instrument',
  subscales: [
    {
      id: 'sub-a',
      name: 'Subscale A',
      description: 'First subscale',
      dimension: 'test',
    },
    {
      id: 'sub-b',
      name: 'Subscale B',
      description: 'Second subscale',
      dimension: 'test',
    },
  ],
  resultTypes: [
    { id: 'sub-a', name: 'Type A', description: 'First type' },
    { id: 'sub-b', name: 'Type B', description: 'Second type' },
  ],
  scoringConfig: {
    method: 'highest-subscale',
    normalize: true,
  },
  display: {
    framework: 'test-framework',
    frameworkDisplayName: 'Test Framework',
    autoFeature: false,
  },
  ...overrides,
});

const buildContentNode = (
  overrides: Partial<ContentNode> = {},
  contentOverrides: Partial<InstrumentDefinition> = {}
): ContentNode =>
  ({
    id: 'node-test-instrument',
    contentType: 'instrument',
    title: 'Test Instrument',
    description: 'A test instrument',
    content: buildInstrumentDefinition(contentOverrides),
    contentFormat: 'instrument-json',
    tags: [],
    relatedNodeIds: [],
  }) as unknown as ContentNode;

const buildStringContentNode = (def: InstrumentDefinition): ContentNode =>
  ({
    id: `node-${def.id}`,
    contentType: 'instrument',
    title: def.name,
    description: def.description,
    content: JSON.stringify(def),
    contentFormat: 'instrument-json',
    tags: [],
    relatedNodeIds: [],
  }) as unknown as ContentNode;

// =============================================================================
// Tests
// =============================================================================

describe('InstrumentLoaderService', () => {
  let service: InstrumentLoaderService;
  let contentServiceSpy: jasmine.SpyObj<ContentService>;

  beforeEach(() => {
    contentServiceSpy = jasmine.createSpyObj('ContentService', ['queryContent', 'getContent']);

    TestBed.configureTestingModule({
      providers: [InstrumentLoaderService, { provide: ContentService, useValue: contentServiceSpy }],
    });

    service = TestBed.inject(InstrumentLoaderService);
  });

  // ---------------------------------------------------------------------------
  // Service creation
  // ---------------------------------------------------------------------------

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should have initial signal values', () => {
    expect(service.loading()).toBe(false);
    expect(service.loaded()).toBe(false);
    expect(service.error()).toBeNull();
  });

  // ---------------------------------------------------------------------------
  // loadInstruments()
  // ---------------------------------------------------------------------------

  describe('loadInstruments()', () => {
    it('should load instruments from content pipeline', async () => {
      const def = buildInstrumentDefinition({ id: 'pipeline-1' });
      const node = buildContentNode({}, { id: 'pipeline-1' });
      contentServiceSpy.queryContent.and.returnValue(of([node]));

      const count = await service.loadInstruments();

      expect(count).toBe(1);
      expect(service.loaded()).toBe(true);
      expect(service.loading()).toBe(false);
      expect(service.error()).toBeNull();
      expect(getInstrument('pipeline-1')).toBeDefined();
    });

    it('should load multiple instrument nodes', async () => {
      const nodes = [
        buildContentNode({}, { id: 'multi-1', framework: 'fw-1' }),
        buildContentNode({}, { id: 'multi-2', framework: 'fw-2' }),
      ];
      contentServiceSpy.queryContent.and.returnValue(of(nodes));

      const count = await service.loadInstruments();

      expect(count).toBe(2);
      expect(getInstrument('multi-1')).toBeDefined();
      expect(getInstrument('multi-2')).toBeDefined();
    });

    it('should parse stringified content bodies', async () => {
      const def = buildInstrumentDefinition({ id: 'string-body' });
      const node = buildStringContentNode(def);
      contentServiceSpy.queryContent.and.returnValue(of([node]));

      const count = await service.loadInstruments();

      expect(count).toBe(1);
      expect(getInstrument('string-body')).toBeDefined();
    });

    it('should skip nodes with wrong contentType', async () => {
      const node = buildContentNode({}, { id: 'wrong-type' });
      (node as unknown as Record<string, unknown>)['contentType'] = 'article';
      contentServiceSpy.queryContent.and.returnValue(of([node]));

      const count = await service.loadInstruments();

      expect(count).toBe(0);
    });

    it('should skip nodes with invalid body structure', async () => {
      const node = {
        id: 'bad-body',
        contentType: 'instrument',
        title: 'Bad',
        description: '',
        content: { id: 'only-id' },
        contentFormat: 'instrument-json',
        tags: [],
        relatedNodeIds: [],
      } as unknown as ContentNode;
      contentServiceSpy.queryContent.and.returnValue(of([node]));

      const count = await service.loadInstruments();

      expect(count).toBe(0);
    });

    it('should skip nodes with unparseable string content', async () => {
      const node = {
        id: 'bad-json',
        contentType: 'instrument',
        title: 'Bad JSON',
        description: '',
        content: '{ not valid json',
        contentFormat: 'instrument-json',
        tags: [],
        relatedNodeIds: [],
      } as unknown as ContentNode;
      contentServiceSpy.queryContent.and.returnValue(of([node]));

      const count = await service.loadInstruments();

      expect(count).toBe(0);
    });

    it('should handle empty result set', async () => {
      contentServiceSpy.queryContent.and.returnValue(of([]));

      const count = await service.loadInstruments();

      expect(count).toBe(0);
      expect(service.loaded()).toBe(true);
    });

    it('should set error signal on fetch failure', async () => {
      contentServiceSpy.queryContent.and.returnValue(throwError(() => new Error('Network down')));

      const count = await service.loadInstruments();

      expect(count).toBe(0);
      expect(service.error()).toBe('Network down');
      expect(service.loaded()).toBe(false);
      expect(service.loading()).toBe(false);
    });

    it('should handle non-Error throws', async () => {
      contentServiceSpy.queryContent.and.returnValue(throwError(() => 'string error'));

      const count = await service.loadInstruments();

      expect(count).toBe(0);
      expect(service.error()).toBe('string error');
    });

    it('should prevent concurrent loads', async () => {
      contentServiceSpy.queryContent.and.returnValue(of([buildContentNode({}, { id: 'conc-1' })]));

      const promise1 = service.loadInstruments();
      const promise2 = service.loadInstruments();

      const [count1, count2] = await Promise.all([promise1, promise2]);

      // Second call returns 0 immediately because loading is in progress
      expect(count1).toBe(1);
      expect(count2).toBe(0);
    });

    it('should query for instrument contentType', async () => {
      contentServiceSpy.queryContent.and.returnValue(of([]));

      await service.loadInstruments();

      expect(contentServiceSpy.queryContent).toHaveBeenCalledWith({ contentType: 'instrument' });
    });
  });

  // ---------------------------------------------------------------------------
  // loadInstrument()
  // ---------------------------------------------------------------------------

  describe('loadInstrument()', () => {
    it('should load a single instrument by node ID', async () => {
      const node = buildContentNode({}, { id: 'single-load' });
      contentServiceSpy.getContent.and.returnValue(of(node));

      const result = await service.loadInstrument('node-single');

      expect(result).toBe(true);
      expect(getInstrument('single-load')).toBeDefined();
    });

    it('should return false when node not found', async () => {
      contentServiceSpy.getContent.and.returnValue(of(null));

      const result = await service.loadInstrument('nonexistent');

      expect(result).toBe(false);
    });

    it('should return false when node has invalid body', async () => {
      const node = {
        id: 'bad',
        contentType: 'instrument',
        content: { id: 'only-id' },
      } as unknown as ContentNode;
      contentServiceSpy.getContent.and.returnValue(of(node));

      const result = await service.loadInstrument('bad');

      expect(result).toBe(false);
    });

    it('should return false on fetch error', async () => {
      contentServiceSpy.getContent.and.returnValue(throwError(() => new Error('Not found')));

      const result = await service.loadInstrument('error-node');

      expect(result).toBe(false);
    });
  });

  // ---------------------------------------------------------------------------
  // instrumentCount signal
  // ---------------------------------------------------------------------------

  describe('instrumentCount', () => {
    it('should return a number', () => {
      expect(typeof service.instrumentCount()).toBe('number');
    });
  });
});

// =============================================================================
// registerFromDefinition (pure function tests)
// =============================================================================

describe('registerFromDefinition', () => {
  it('should register an instrument and return the entry', () => {
    const def = buildInstrumentDefinition({ id: 'reg-def-1', framework: 'reg-fw-1' });

    const entry = registerFromDefinition(def);

    expect(entry).toBeDefined();
    expect(entry.config.id).toBe('reg-def-1');
    expect(entry.config.name).toBe('Test Instrument');
    expect(entry.subscales.length).toBe(2);
    expect(entry.resultTypes.length).toBe(2);
    expect(entry.displayConfig?.framework).toBe('test-framework');
  });

  it('should make the instrument retrievable from registry', () => {
    const def = buildInstrumentDefinition({ id: 'reg-def-2' });

    registerFromDefinition(def);

    const retrieved = getInstrument('reg-def-2');
    expect(retrieved).toBeDefined();
    expect(retrieved!.config.name).toBe('Test Instrument');
  });

  it('should create an interpret function from scoring config', () => {
    const def = buildInstrumentDefinition({ id: 'reg-def-3' });
    const entry = registerFromDefinition(def);

    expect(entry.config.interpret).toBeDefined();
    expect(typeof entry.config.interpret).toBe('function');
  });

  // ---------------------------------------------------------------------------
  // Scoring: highest-subscale
  // ---------------------------------------------------------------------------

  describe('highest-subscale scoring', () => {
    it('should return the highest scoring subscale as primary type', () => {
      const def = buildInstrumentDefinition({
        id: 'hs-test',
        scoringConfig: { method: 'highest-subscale', normalize: true },
      });
      const entry = registerFromDefinition(def);

      const result = entry.config.interpret!({
        subscaleTotals: { 'sub-a': 10, 'sub-b': 20 },
        normalizedScores: { 'sub-a': 0.3, 'sub-b': 0.7 },
        subscaleCounts: {},
        momentCount: 5,
        momentIds: [],
        aggregatedAt: Date.now(),
      });

      expect(result).toBeDefined();
      expect((result as InterpretResult).primaryType.typeId).toBe('sub-b');
      expect((result as InterpretResult).primaryType.typeName).toBe('Type B');
    });

    it('should return null for empty scores', () => {
      const def = buildInstrumentDefinition({
        id: 'hs-empty',
        scoringConfig: { method: 'highest-subscale', normalize: true },
      });
      const entry = registerFromDefinition(def);

      const result = entry.config.interpret!({
        subscaleTotals: {},
        normalizedScores: {},
        subscaleCounts: {},
        momentCount: 0,
        momentIds: [],
        aggregatedAt: Date.now(),
      });

      expect(result).toBeNull();
    });
  });

  // ---------------------------------------------------------------------------
  // Scoring: dimensional
  // ---------------------------------------------------------------------------

  describe('dimensional scoring', () => {
    it('should use normalized scores and find highest', () => {
      const def = buildInstrumentDefinition({
        id: 'dim-test',
        scoringConfig: { method: 'dimensional', normalize: true },
      });
      const entry = registerFromDefinition(def);

      const result = entry.config.interpret!({
        subscaleTotals: { 'sub-a': 15, 'sub-b': 10 },
        normalizedScores: { 'sub-a': 0.8, 'sub-b': 0.5 },
        subscaleCounts: {},
        momentCount: 5,
        momentIds: [],
        aggregatedAt: Date.now(),
      });

      expect(result).toBeDefined();
      expect((result as InterpretResult).primaryType.typeId).toBe('sub-a');
      expect((result as InterpretResult).primaryType.typeName).toBe('Type A');
    });
  });

  // ---------------------------------------------------------------------------
  // Scoring: threshold-based
  // ---------------------------------------------------------------------------

  describe('threshold-based scoring', () => {
    const thresholdDef = (): InstrumentDefinition =>
      buildInstrumentDefinition({
        id: 'thresh-test',
        subscales: [
          { id: 'anxiety', name: 'Anxiety', description: 'Anxiety', dimension: 'att' },
          { id: 'avoidance', name: 'Avoidance', description: 'Avoidance', dimension: 'att' },
        ],
        resultTypes: [
          {
            id: 'secure',
            name: 'Secure',
            description: 'Low on both',
            subscaleProfile: { anxiety: 0, avoidance: 0 },
          },
          {
            id: 'anxious',
            name: 'Anxious',
            description: 'High anxiety',
            subscaleProfile: { anxiety: 1, avoidance: 0 },
          },
          {
            id: 'avoidant',
            name: 'Avoidant',
            description: 'High avoidance',
            subscaleProfile: { anxiety: 0, avoidance: 1 },
          },
          {
            id: 'fearful',
            name: 'Fearful',
            description: 'High on both',
            subscaleProfile: { anxiety: 1, avoidance: 1 },
          },
        ],
        scoringConfig: {
          method: 'threshold-based',
          normalize: false,
          thresholds: { anxiety: 12, avoidance: 8 },
        },
      });

    it('should classify secure (both below threshold)', () => {
      const entry = registerFromDefinition(thresholdDef());

      const result = entry.config.interpret!({
        subscaleTotals: { anxiety: 5, avoidance: 3 },
        normalizedScores: {},
        subscaleCounts: {},
        momentCount: 5,
        momentIds: [],
        aggregatedAt: Date.now(),
      });

      expect((result as InterpretResult).primaryType.typeId).toBe('secure');
      expect((result as InterpretResult).primaryType.typeName).toBe('Secure');
    });

    it('should classify anxious (anxiety above threshold)', () => {
      const entry = registerFromDefinition(thresholdDef());

      const result = entry.config.interpret!({
        subscaleTotals: { anxiety: 15, avoidance: 3 },
        normalizedScores: {},
        subscaleCounts: {},
        momentCount: 5,
        momentIds: [],
        aggregatedAt: Date.now(),
      });

      expect((result as InterpretResult).primaryType.typeId).toBe('anxious');
    });

    it('should classify avoidant (avoidance above threshold)', () => {
      const entry = registerFromDefinition(thresholdDef());

      const result = entry.config.interpret!({
        subscaleTotals: { anxiety: 5, avoidance: 12 },
        normalizedScores: {},
        subscaleCounts: {},
        momentCount: 5,
        momentIds: [],
        aggregatedAt: Date.now(),
      });

      expect((result as InterpretResult).primaryType.typeId).toBe('avoidant');
    });

    it('should classify fearful (both above threshold)', () => {
      const entry = registerFromDefinition(thresholdDef());

      const result = entry.config.interpret!({
        subscaleTotals: { anxiety: 15, avoidance: 12 },
        normalizedScores: {},
        subscaleCounts: {},
        momentCount: 5,
        momentIds: [],
        aggregatedAt: Date.now(),
      });

      expect((result as InterpretResult).primaryType.typeId).toBe('fearful');
    });

    it('should return unknown when no thresholds configured', () => {
      const def = buildInstrumentDefinition({
        id: 'thresh-no-config',
        scoringConfig: { method: 'threshold-based', normalize: false },
      });
      const entry = registerFromDefinition(def);

      const result = entry.config.interpret!({
        subscaleTotals: { 'sub-a': 10 },
        normalizedScores: {},
        subscaleCounts: {},
        momentCount: 5,
        momentIds: [],
        aggregatedAt: Date.now(),
      });

      expect(result).toBeNull();
    });
  });

  // ---------------------------------------------------------------------------
  // Scoring: unknown method
  // ---------------------------------------------------------------------------

  describe('unknown scoring method', () => {
    it('should return null for unrecognized scoring method', () => {
      const def = buildInstrumentDefinition({
        id: 'unknown-method',
        scoringConfig: { method: 'future-method' as 'highest-subscale', normalize: false },
      });
      const entry = registerFromDefinition(def);

      const result = entry.config.interpret!({
        subscaleTotals: { 'sub-a': 10 },
        normalizedScores: { 'sub-a': 0.5 },
        subscaleCounts: {},
        momentCount: 5,
        momentIds: [],
        aggregatedAt: Date.now(),
      });

      expect(result).toBeNull();
    });
  });

  // ---------------------------------------------------------------------------
  // Overwrite behavior
  // ---------------------------------------------------------------------------

  describe('overwrite behavior', () => {
    it('should overwrite existing instrument with same ID', () => {
      const def1 = buildInstrumentDefinition({ id: 'overwrite-test', name: 'Original' });
      const def2 = buildInstrumentDefinition({ id: 'overwrite-test', name: 'Updated' });

      registerFromDefinition(def1);
      registerFromDefinition(def2);

      const entry = getInstrument('overwrite-test');
      expect(entry!.config.name).toBe('Updated');
    });
  });
});

// =============================================================================
// Declarative vs Closure parity (Step 4 verification)
// =============================================================================

describe('declarative scoring parity', () => {
  // Import side-effects so static instruments are registered
  beforeAll(async () => {
    await import('../instruments/epic-domain.instrument');
    await import('../instruments/attachment-style.instrument');
    await import('../instruments/values-hierarchy.instrument');
    await import('../instruments/strengths-finder.instrument');
  });

  const buildAggregated = (
    totals: Record<string, number>,
    normalized: Record<string, number>
  ): { subscaleTotals: Record<string, number>; subscaleCounts: Record<string, number>; normalizedScores: Record<string, number>; momentCount: number; momentIds: string[]; aggregatedAt: number } => ({
    subscaleTotals: totals,
    subscaleCounts: Object.fromEntries(Object.keys(totals).map(k => [k, 1])),
    normalizedScores: normalized,
    momentCount: 5,
    momentIds: ['m1', 'm2', 'm3', 'm4', 'm5'],
    aggregatedAt: Date.now(),
  });

  describe('Epic Domain (highest-subscale)', () => {
    it('should match closure result via declarative config', () => {
      const staticEntry = getInstrument('epic-domain-discovery');
      const def = buildInstrumentDefinition({
        id: 'epic-parity-test',
        subscales: staticEntry!.subscales,
        resultTypes: staticEntry!.resultTypes,
        scoringConfig: { method: 'highest-subscale', normalize: true },
      });
      const declEntry = registerFromDefinition(def);

      const aggregated = buildAggregated(
        { governance: 10, care: 8, economic: 12, public: 6, social: 9 },
        { governance: 0.22, care: 0.18, economic: 0.27, public: 0.13, social: 0.2 }
      );

      const closureResult = staticEntry!.config.interpret!(aggregated) as InterpretResult;
      const declResult = declEntry.config.interpret!(aggregated) as InterpretResult;

      expect(declResult.primaryType.typeId).toBe(closureResult.primaryType.typeId);
      expect(declResult.primaryType.typeName).toBe(closureResult.primaryType.typeName);
      expect(declResult.primaryType.typeId).toBe('economic');
    });
  });

  describe('Attachment Style (threshold-based)', () => {
    it('should classify secure correctly', () => {
      const def = buildInstrumentDefinition({
        id: 'att-parity-secure',
        subscales: [
          { id: 'anxiety', name: 'Anxiety', description: '', dimension: 'att' },
          { id: 'avoidance', name: 'Avoidance', description: '', dimension: 'att' },
        ],
        resultTypes: [
          { id: 'secure', name: 'Secure', description: '', subscaleProfile: { anxiety: 0, avoidance: 0 } },
          { id: 'anxious-preoccupied', name: 'Anxious', description: '', subscaleProfile: { anxiety: 1, avoidance: 0 } },
          { id: 'dismissive-avoidant', name: 'Avoidant', description: '', subscaleProfile: { anxiety: 0, avoidance: 1 } },
          { id: 'fearful-avoidant', name: 'Fearful', description: '', subscaleProfile: { anxiety: 1, avoidance: 1 } },
        ],
        scoringConfig: { method: 'threshold-based', normalize: false, thresholds: { anxiety: 12, avoidance: 8 } },
      });
      const entry = registerFromDefinition(def);

      const agg = buildAggregated({ anxiety: 5, avoidance: 3 }, {});
      const result = entry.config.interpret!(agg) as InterpretResult;

      expect(result.primaryType.typeId).toBe('secure');
    });

    it('should classify fearful-avoidant correctly', () => {
      const def = buildInstrumentDefinition({
        id: 'att-parity-fearful',
        subscales: [
          { id: 'anxiety', name: 'Anxiety', description: '', dimension: 'att' },
          { id: 'avoidance', name: 'Avoidance', description: '', dimension: 'att' },
        ],
        resultTypes: [
          { id: 'secure', name: 'Secure', description: '', subscaleProfile: { anxiety: 0, avoidance: 0 } },
          { id: 'anxious-preoccupied', name: 'Anxious', description: '', subscaleProfile: { anxiety: 1, avoidance: 0 } },
          { id: 'dismissive-avoidant', name: 'Avoidant', description: '', subscaleProfile: { anxiety: 0, avoidance: 1 } },
          { id: 'fearful-avoidant', name: 'Fearful', description: '', subscaleProfile: { anxiety: 1, avoidance: 1 } },
        ],
        scoringConfig: { method: 'threshold-based', normalize: false, thresholds: { anxiety: 12, avoidance: 8 } },
      });
      const entry = registerFromDefinition(def);

      const agg = buildAggregated({ anxiety: 18, avoidance: 14 }, {});
      const result = entry.config.interpret!(agg) as InterpretResult;

      expect(result.primaryType.typeId).toBe('fearful-avoidant');
    });
  });

  describe('Values Hierarchy (dimensional)', () => {
    it('should match closure result via declarative config', () => {
      const staticEntry = getInstrument('values-hierarchy-discovery');
      const def = buildInstrumentDefinition({
        id: 'values-parity-test',
        subscales: staticEntry!.subscales,
        resultTypes: staticEntry!.resultTypes,
        scoringConfig: { method: 'dimensional', normalize: true },
      });
      const declEntry = registerFromDefinition(def);

      const aggregated = buildAggregated(
        { openness: 15, conservation: 10, 'self-transcendence': 20, 'self-enhancement': 8 },
        { openness: 0.28, conservation: 0.19, 'self-transcendence': 0.38, 'self-enhancement': 0.15 }
      );

      const closureResult = staticEntry!.config.interpret!(aggregated) as InterpretResult;
      const declResult = declEntry.config.interpret!(aggregated) as InterpretResult;

      expect(declResult.primaryType.typeId).toBe(closureResult.primaryType.typeId);
      expect(declResult.primaryType.typeId).toBe('self-transcendence');
    });
  });

  describe('Strengths Finder (highest-subscale)', () => {
    it('should match closure result via declarative config', () => {
      const staticEntry = getInstrument('strengths-finder-discovery');
      const def = buildInstrumentDefinition({
        id: 'strengths-parity-test',
        subscales: staticEntry!.subscales,
        resultTypes: staticEntry!.resultTypes,
        scoringConfig: { method: 'highest-subscale', normalize: true },
      });
      const declEntry = registerFromDefinition(def);

      const aggregated = buildAggregated(
        { creativity: 8, curiosity: 12, fairness: 6, kindness: 15, perseverance: 10, hope: 9 },
        { creativity: 0.13, curiosity: 0.2, fairness: 0.1, kindness: 0.25, perseverance: 0.17, hope: 0.15 }
      );

      const closureResult = staticEntry!.config.interpret!(aggregated) as InterpretResult;
      const declResult = declEntry.config.interpret!(aggregated) as InterpretResult;

      expect(declResult.primaryType.typeId).toBe(closureResult.primaryType.typeId);
      expect(declResult.primaryType.typeId).toBe('kindness');
    });
  });
});
