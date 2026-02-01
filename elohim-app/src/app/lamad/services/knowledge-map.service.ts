import { Injectable } from '@angular/core';

// @coverage: 83.6% (2026-01-31)

import { map, switchMap, tap } from 'rxjs/operators';

import { Observable, of, throwError, BehaviorSubject } from 'rxjs';

import { DataLoaderService } from '@app/elohim/services/data-loader.service';
import { ElohimAgentService } from '@app/elohim/services/elohim-agent.service';
import { generateMapId } from '@app/elohim/utils';

import {
  KnowledgeMap,
  KnowledgeMapType,
  KnowledgeNode,
  KnowledgeMapIndex,
  KnowledgeMapIndexEntry,
  KnowledgeMapUpdate,
  DomainKnowledgeMap,
  PersonKnowledgeMap,
  CollectiveKnowledgeMap,
  SubjectConsent,
  MasteryLevel,
} from '../models/knowledge-map.model';

/**
 * KnowledgeMapService - Management of polymorphic knowledge maps.
 *
 * Knowledge maps represent a learner's personal relationship with learnable territory:
 * - Domain maps: Relationship with a content graph (like Elohim Protocol)
 * - Person maps: Relationship with another person (Gottman Love Maps)
 * - Collective maps: Shared knowledge within a group
 *
 * Key principles:
 * - Maps are owned by individuals (privacy by default)
 * - Person maps require subject consent for deep access
 * - Collective maps have governance for changes
 * - Elohim agents synthesize and recommend map updates
 *
 * From API Spec:
 * "The same navigation/affinity mechanics apply to all three [map types]."
 */
@Injectable({ providedIn: 'root' })
export class KnowledgeMapService {
  // In-memory map storage (prototype - production uses Holochain)
  private readonly maps = new Map<string, KnowledgeMap>();
  private mapIndex: KnowledgeMapIndex | null = null;

  // Current agent's maps
  private readonly myMapsSubject = new BehaviorSubject<KnowledgeMapIndexEntry[]>([]);
  public readonly myMaps$ = this.myMapsSubject.asObservable();

  // Current agent ID (from auth service in production)
  private currentAgentId = 'demo-learner';

  constructor(
    private readonly dataLoader: DataLoaderService,
    private readonly elohimService: ElohimAgentService
  ) {
    this.initializeDemoMaps();
  }

  // =========================================================================
  // Map Discovery
  // =========================================================================

  /**
   * Get all maps visible to the current agent.
   */
  getMapIndex(): Observable<KnowledgeMapIndex> {
    if (this.mapIndex) {
      return of(this.mapIndex);
    }

    // Build index from in-memory maps
    const entries: KnowledgeMapIndexEntry[] = [];

    this.maps.forEach(m => {
      if (this.canView(m)) {
        entries.push(this.toIndexEntry(m));
      }
    });

    const index: KnowledgeMapIndex = {
      lastUpdated: new Date().toISOString(),
      totalCount: entries.length,
      maps: entries,
    };

    this.mapIndex = index;
    return of(index);
  }

  /**
   * Get maps owned by the current agent.
   */
  getMyMaps(): Observable<KnowledgeMapIndexEntry[]> {
    return this.getMapIndex().pipe(
      map(index => index.maps.filter(m => m.ownerId === this.currentAgentId)),
      tap(maps => this.myMapsSubject.next(maps))
    );
  }

  /**
   * Get maps of a specific type.
   */
  getMapsByType(mapType: KnowledgeMapType): Observable<KnowledgeMapIndexEntry[]> {
    return this.getMapIndex().pipe(map(index => index.maps.filter(m => m.mapType === mapType)));
  }

  /**
   * Get a specific map by ID.
   */
  getMap(mapId: string): Observable<KnowledgeMap | null> {
    const m = this.maps.get(mapId);

    if (!m) {
      return of(null);
    }

    if (!this.canView(m)) {
      return throwError(() => ({
        code: 'UNAUTHORIZED',
        message: 'You do not have permission to view this map',
      }));
    }

    return of(m);
  }

  /**
   * Get a domain knowledge map with type narrowing.
   */
  getDomainMap(mapId: string): Observable<DomainKnowledgeMap | null> {
    return this.getMap(mapId).pipe(
      map(m => {
        if (m?.mapType === 'domain') {
          return m as DomainKnowledgeMap;
        }
        return null;
      })
    );
  }

  /**
   * Get a person knowledge map with type narrowing.
   */
  getPersonMap(mapId: string): Observable<PersonKnowledgeMap | null> {
    return this.getMap(mapId).pipe(
      map(m => {
        if (m?.mapType === 'person') {
          return m as PersonKnowledgeMap;
        }
        return null;
      })
    );
  }

  /**
   * Get a collective knowledge map with type narrowing.
   */
  getCollectiveMap(mapId: string): Observable<CollectiveKnowledgeMap | null> {
    return this.getMap(mapId).pipe(
      map(m => {
        if (m?.mapType === 'collective') {
          return m as CollectiveKnowledgeMap;
        }
        return null;
      })
    );
  }

  // =========================================================================
  // Map Creation
  // =========================================================================

  /**
   * Create a new domain knowledge map.
   */
  createDomainMap(params: {
    title: string;
    contentGraphId: string;
    description?: string;
    visibility?: KnowledgeMap['visibility'];
  }): Observable<DomainKnowledgeMap> {
    const mapId = generateMapId('domain');
    const now = new Date().toISOString();

    const newMap: DomainKnowledgeMap = {
      id: mapId,
      mapType: 'domain',
      subject: {
        type: 'content-graph',
        subjectId: params.contentGraphId,
        subjectName: params.title,
      },
      ownerId: this.currentAgentId,
      title: params.title,
      description: params.description,
      visibility: params.visibility ?? 'private',
      nodes: [],
      pathIds: [],
      overallAffinity: 0,
      contentGraphId: params.contentGraphId,
      masteryLevels: new Map(),
      createdAt: now,
      updatedAt: now,
    };

    this.maps.set(mapId, newMap);
    this.invalidateIndex();

    return of(newMap);
  }

  /**
   * Create a new person knowledge map.
   * Subject consent is not required to create but limits what can be stored.
   */
  createPersonMap(params: {
    title: string;
    subjectAgentId: string;
    subjectName: string;
    relationshipType: PersonKnowledgeMap['relationshipType'];
    description?: string;
    visibility?: KnowledgeMap['visibility'];
  }): Observable<PersonKnowledgeMap> {
    const mapId = generateMapId('person');
    const now = new Date().toISOString();

    const newMap: PersonKnowledgeMap = {
      id: mapId,
      mapType: 'person',
      subject: {
        type: 'agent',
        subjectId: params.subjectAgentId,
        subjectName: params.subjectName,
      },
      ownerId: this.currentAgentId,
      title: params.title,
      description: params.description,
      visibility: params.visibility ?? 'private',
      nodes: [],
      pathIds: [],
      overallAffinity: 0,
      relationshipType: params.relationshipType,
      categories: [],
      createdAt: now,
      updatedAt: now,
    };

    this.maps.set(mapId, newMap);
    this.invalidateIndex();

    return of(newMap);
  }

  /**
   * Create a new collective knowledge map.
   * Creator becomes the initial steward.
   */
  createCollectiveMap(params: {
    title: string;
    organizationId: string;
    organizationName: string;
    description?: string;
    visibility?: KnowledgeMap['visibility'];
    governance?: CollectiveKnowledgeMap['governance'];
  }): Observable<CollectiveKnowledgeMap> {
    const mapId = generateMapId('collective');
    const now = new Date().toISOString();

    const newMap: CollectiveKnowledgeMap = {
      id: mapId,
      mapType: 'collective',
      subject: {
        type: 'organization',
        subjectId: params.organizationId,
        subjectName: params.organizationName,
      },
      ownerId: this.currentAgentId,
      title: params.title,
      description: params.description,
      visibility: params.visibility ?? 'shared',
      nodes: [],
      pathIds: [],
      overallAffinity: 0,
      members: [
        {
          agentId: this.currentAgentId,
          role: 'steward',
          joinedAt: now,
          contributionCount: 0,
        },
      ],
      governance: params.governance ?? {
        approvalModel: 'steward-only',
        membershipControl: 'steward-only',
      },
      domains: [],
      collectiveAttestations: [],
      createdAt: now,
      updatedAt: now,
    };

    this.maps.set(mapId, newMap);
    this.invalidateIndex();

    return of(newMap);
  }

  // =========================================================================
  // Map Updates
  // =========================================================================

  /**
   * Add a knowledge node to a map.
   */
  addNode(mapId: string, node: Omit<KnowledgeNode, 'id'>): Observable<KnowledgeNode> {
    return this.getMap(mapId).pipe(
      switchMap(m => {
        if (!m) {
          return throwError(() => ({ code: 'NOT_FOUND', message: 'Map not found' }));
        }

        if (!this.canEdit(m)) {
          return throwError(() => ({ code: 'UNAUTHORIZED', message: 'Cannot edit this map' }));
        }

        const newNode: KnowledgeNode = {
          ...node,
          id: `node-${Date.now()}-${(crypto.getRandomValues(new Uint32Array(1))[0] / 2 ** 32).toString(36).substring(2, 11)}`, // Crypto-secure random node ID
        };

        m.nodes.push(newNode);
        m.updatedAt = new Date().toISOString();
        this.recalculateAffinity(m);
        this.invalidateIndex();

        return of(newNode);
      })
    );
  }

  /**
   * Update a knowledge node.
   */
  updateNode(
    mapId: string,
    nodeId: string,
    updates: Partial<KnowledgeNode>
  ): Observable<KnowledgeNode> {
    return this.getMap(mapId).pipe(
      switchMap(m => {
        if (!m) {
          return throwError(() => ({ code: 'NOT_FOUND', message: 'Map not found' }));
        }

        if (!this.canEdit(m)) {
          return throwError(() => ({ code: 'UNAUTHORIZED', message: 'Cannot edit this map' }));
        }

        const nodeIndex = m.nodes.findIndex(n => n.id === nodeId);
        if (nodeIndex === -1) {
          return throwError(() => ({ code: 'NOT_FOUND', message: 'Node not found' }));
        }

        m.nodes[nodeIndex] = { ...m.nodes[nodeIndex], ...updates };
        m.updatedAt = new Date().toISOString();
        this.recalculateAffinity(m);
        this.invalidateIndex();

        return of(m.nodes[nodeIndex]);
      })
    );
  }

  /**
   * Remove a node from a map.
   */
  removeNode(mapId: string, nodeId: string): Observable<void> {
    return this.getMap(mapId).pipe(
      switchMap(m => {
        if (!m) {
          return throwError(() => ({ code: 'NOT_FOUND', message: 'Map not found' }));
        }

        if (!this.canEdit(m)) {
          return throwError(() => ({ code: 'UNAUTHORIZED', message: 'Cannot edit this map' }));
        }

        m.nodes = m.nodes.filter(n => n.id !== nodeId);
        m.updatedAt = new Date().toISOString();
        this.recalculateAffinity(m);
        this.invalidateIndex();

        return of(undefined);
      })
    );
  }

  /**
   * Update mastery level for a content node in a domain map.
   */
  updateMastery(mapId: string, contentNodeId: string, level: MasteryLevel): Observable<void> {
    return this.getDomainMap(mapId).pipe(
      switchMap(m => {
        if (!m) {
          return throwError(() => ({ code: 'NOT_FOUND', message: 'Domain map not found' }));
        }

        if (!this.canEdit(m)) {
          return throwError(() => ({ code: 'UNAUTHORIZED', message: 'Cannot edit this map' }));
        }

        m.masteryLevels.set(contentNodeId, level);
        m.updatedAt = new Date().toISOString();
        this.recalculateDomainAffinity(m);
        this.invalidateIndex();

        return of(undefined);
      })
    );
  }

  // =========================================================================
  // Person Map Consent
  // =========================================================================

  /**
   * Request consent from subject of a person map.
   */
  requestConsent(mapId: string, _scope: SubjectConsent['scope']): Observable<void> {
    return this.getPersonMap(mapId).pipe(
      switchMap(m => {
        if (!m) {
          return throwError(() => ({ code: 'NOT_FOUND', message: 'Person map not found' }));
        }

        // In production: send notification to subject
        // For prototype: auto-approve after delay
        return of(undefined);
      })
    );
  }

  /**
   * Grant consent to a person map (called by subject).
   */
  grantConsent(mapId: string, consent: SubjectConsent): Observable<void> {
    return this.getPersonMap(mapId).pipe(
      switchMap(m => {
        if (!m) {
          return throwError(() => ({ code: 'NOT_FOUND', message: 'Person map not found' }));
        }

        // Only the subject can grant consent
        if (m.subject.subjectId !== this.currentAgentId) {
          return throwError(() => ({
            code: 'UNAUTHORIZED',
            message: 'Only the subject can grant consent',
          }));
        }

        m.subjectConsent = consent;
        m.updatedAt = new Date().toISOString();

        return of(undefined);
      })
    );
  }

  // =========================================================================
  // Elohim Integration
  // =========================================================================

  /**
   * Request Elohim synthesis of knowledge map updates.
   *
   * Elohim agents can analyze learning patterns and suggest
   * map updates, new connections, or affinity adjustments.
   */
  requestElohimSynthesis(mapId: string): Observable<KnowledgeMapUpdate[]> {
    return this.getMap(mapId).pipe(
      switchMap(m => {
        if (!m) {
          return throwError(() => ({ code: 'NOT_FOUND', message: 'Map not found' }));
        }

        // Invoke Elohim with knowledge-map-synthesis capability
        return this.elohimService.invoke({
          requestId: `synth-${Date.now()}`,
          targetElohimId: 'auto',
          capability: 'knowledge-map-synthesis',
          params: {
            type: 'knowledge-map-synthesis',
            mapId,
            subjectType: m.mapType,
            subjectId: m.subject.subjectId,
          },
          requesterId: this.currentAgentId,
          priority: 'normal',
          requestedAt: new Date().toISOString(),
        });
      }),
      map(response => {
        if (response.status !== 'fulfilled') {
          return [];
        }

        // Extract suggested updates from Elohim response
        const payload = response.payload as any;
        if (payload?.type !== 'knowledge-map-update') {
          return [];
        }

        // Generate update suggestions based on Elohim analysis
        // In production, Elohim returns actual analysis
        return this.generateDemoSuggestions(mapId);
      })
    );
  }

  // =========================================================================
  // Helper Methods
  // =========================================================================

  /**
   * Check if current agent can view a map.
   */
  private canView(m: KnowledgeMap): boolean {
    if (m.visibility === 'public') return true;
    if (m.ownerId === this.currentAgentId) return true;
    if (m.visibility === 'shared' && m.sharedWith?.includes(this.currentAgentId)) return true;

    // For person maps, subject can view if mutual
    if (m.mapType === 'person') {
      const pm = m as PersonKnowledgeMap;
      if (pm.visibility === 'mutual' && pm.subject.subjectId === this.currentAgentId) return true;
      if (
        pm.subjectConsent?.transparencyLevel !== 'none' &&
        pm.subject.subjectId === this.currentAgentId
      )
        return true;
    }

    // For collective maps, members can view
    if (m.mapType === 'collective') {
      const cm = m as CollectiveKnowledgeMap;
      if (cm.members.some(member => member.agentId === this.currentAgentId)) return true;
    }

    return false;
  }

  /**
   * Check if current agent can edit a map.
   */
  private canEdit(m: KnowledgeMap): boolean {
    if (m.ownerId === this.currentAgentId) return true;

    // For collective maps, check role
    if (m.mapType === 'collective') {
      const cm = m as CollectiveKnowledgeMap;
      const member = cm.members.find(mem => mem.agentId === this.currentAgentId);
      if (member && (member.role === 'steward' || member.role === 'contributor')) {
        return true;
      }
    }

    return false;
  }

  /**
   * Convert map to index entry.
   */
  private toIndexEntry(m: KnowledgeMap): KnowledgeMapIndexEntry {
    return {
      id: m.id,
      mapType: m.mapType,
      title: m.title,
      subjectName: m.subject.subjectName,
      ownerId: m.ownerId,
      ownerName: m.ownerId, // In production: resolve to display name
      visibility: m.visibility,
      overallAffinity: m.overallAffinity,
      nodeCount: m.nodes.length,
      updatedAt: m.updatedAt,
    };
  }

  /**
   * Recalculate overall affinity for a map.
   */
  private recalculateAffinity(m: KnowledgeMap): void {
    if (m.nodes.length === 0) {
      m.overallAffinity = 0;
      return;
    }

    const totalAffinity = m.nodes.reduce((sum, n) => sum + n.affinity, 0);
    m.overallAffinity = totalAffinity / m.nodes.length;
  }

  /**
   * Recalculate affinity for domain map based on mastery levels.
   * Uses Bloom's Taxonomy levels (not_started → create).
   */
  private recalculateDomainAffinity(m: DomainKnowledgeMap): void {
    const masteryWeights: Record<MasteryLevel, number> = {
      not_started: 0,
      seen: 0.1,
      remember: 0.2,
      understand: 0.35,
      apply: 0.5, // Attestation gate
      analyze: 0.7,
      evaluate: 0.85,
      create: 1.0,
    };

    if (m.masteryLevels.size === 0) {
      m.overallAffinity = 0;
      return;
    }

    let totalAffinity = 0;
    m.masteryLevels.forEach(level => {
      totalAffinity += masteryWeights[level];
    });

    m.overallAffinity = totalAffinity / m.masteryLevels.size;
  }

  /**
   * Invalidate the cached index.
   */
  private invalidateIndex(): void {
    this.mapIndex = null;
  }

  /**
   * Set current agent (for auth integration).
   */
  setCurrentAgent(agentId: string): void {
    this.currentAgentId = agentId;
  }

  /**
   * Generate demo suggestions (placeholder for Elohim analysis).
   */
  private generateDemoSuggestions(mapId: string): KnowledgeMapUpdate[] {
    // In production, Elohim analyzes the map and returns real suggestions
    return [
      {
        mapId,
        operation: 'add-node',
        data: {
          category: 'suggested',
          title: 'AI-suggested connection',
          content: 'Based on your learning patterns, consider exploring this related concept.',
          affinity: 0.5,
          tags: ['suggested', 'elohim-synthesis'],
          relatedNodeIds: [],
        },
        source: {
          type: 'inference',
          timestamp: new Date().toISOString(),
          confidence: 0.75,
        },
        timestamp: new Date().toISOString(),
      },
    ];
  }

  /**
   * Initialize demo maps for prototype.
   */
  private initializeDemoMaps(): void {
    const now = new Date().toISOString();

    // Demo domain map
    const domainMap: DomainKnowledgeMap = {
      id: 'map-domain-elohim-protocol',
      mapType: 'domain',
      subject: {
        type: 'content-graph',
        subjectId: 'elohim-protocol-graph',
        subjectName: 'The Elohim Protocol',
      },
      ownerId: 'demo-learner',
      title: 'My Elohim Protocol Journey',
      description: 'Personal knowledge map tracking my understanding of the Elohim Protocol',
      visibility: 'private',
      nodes: [
        {
          id: 'node-manifesto-core',
          category: 'foundational',
          title: 'Core Manifesto Principles',
          content: 'Love as committed action, constitutional governance, decentralized identity',
          affinity: 0.8,
          relatedNodeIds: [],
          tags: ['manifesto', 'principles'],
        },
        {
          id: 'node-elohim-agents',
          category: 'technical',
          title: 'Elohim Agent Architecture',
          content:
            'Autonomous AI agents bound to constitutional principles, operating at different layers',
          affinity: 0.6,
          relatedNodeIds: ['node-manifesto-core'],
          tags: ['elohim', 'agents', 'architecture'],
        },
      ],
      pathIds: ['elohim-protocol'],
      overallAffinity: 0.7,
      contentGraphId: 'elohim-protocol-graph',
      masteryLevels: new Map([
        ['manifesto', 'apply'],
        ['governance-epic', 'understand'],
        ['hardware-spec', 'not_started'],
      ]),
      createdAt: now,
      updatedAt: now,
    };

    this.maps.set(domainMap.id, domainMap);

    // Demo collective map (for protocol learners community)
    const collectiveMap: CollectiveKnowledgeMap = {
      id: 'map-collective-learners',
      mapType: 'collective',
      subject: {
        type: 'organization',
        subjectId: 'elohim-protocol-learners',
        subjectName: 'Elohim Protocol Learners',
      },
      ownerId: 'steward-curriculum',
      title: 'Collective Learning Insights',
      description: 'Shared knowledge and insights from the learning community',
      visibility: 'shared',
      sharedWith: ['demo-learner'],
      nodes: [
        {
          id: 'cnode-faq',
          category: 'community',
          title: 'Common Questions',
          content: 'Frequently asked questions about implementing Elohim principles',
          affinity: 0.9,
          relatedNodeIds: [],
          tags: ['faq', 'community', 'learning'],
        },
      ],
      pathIds: [],
      overallAffinity: 0.85,
      members: [
        { agentId: 'steward-curriculum', role: 'steward', joinedAt: now, contributionCount: 15 },
        { agentId: 'demo-learner', role: 'contributor', joinedAt: now, contributionCount: 2 },
      ],
      governance: {
        approvalModel: 'steward-only',
        membershipControl: 'member-invite',
      },
      domains: [
        {
          id: 'domain-governance',
          title: 'Governance Insights',
          description: 'Collective understanding of governance principles',
          stewards: ['steward-governance'],
          nodes: [],
          affinity: 0.8,
        },
      ],
      collectiveAttestations: [],
      createdAt: now,
      updatedAt: now,
    };

    this.maps.set(collectiveMap.id, collectiveMap);
  }
}
