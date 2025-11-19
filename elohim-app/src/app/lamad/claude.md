# Lamad Learning Platform - Vision & Architecture

## Overview

**Lamad** (לָמַד - Hebrew: "to learn/teach") is a graph-based learning platform inspired by Khan Academy's "World of Math" Meaning Map and Gottman's Love Maps. It provides a domain-agnostic, extensible content model with user affinity tracking, attestation-based achievement unlocking, and orientation-based navigation toward a target subject.

This isn't just documentation - it's the first implementation of **Elohim Social Medium principles applied to learning**: where attention is sacred, reach is earned, visibility itself can be earned, and human flourishing is protected through design.

### Conceptual Inspirations

**Khan Academy's "World of Math":**
- Target subject = "World of Math" (mastery goal)
- Categories = Math domains (algebra, geometry, etc.)
- Skills = Individual concepts to master
- Mastery challenge = Determines next skills based on your progress
- Our implementation: Replace "World of Math" with "The Elohim Protocol", epics with domains, content nodes with skills

**Gottman's Love Maps:**
- Target subject = Understanding another person
- Content nodes = Facts, preferences, history, dreams
- Affinity = How well you know different aspects of them
- Orientation = What to learn next to deepen the relationship
- Attestations = "Closest Intimate Partner" - earned through the journey of building trust
- Our abstraction: Any subject (person, skill, concept) can be mapped this way

**Zelda: Breath of the Wild's Fog of War:**
- Target = Mapping Hyrule, defeating Ganon
- Sheikah Towers = Visible in distance (orientation) but details locked until you journey there
- Climbing the tower = Earning access through effort
- Map reveals = Progressive revelation of content you've proven ready for
- Our abstraction: Content visibility itself can be earned, protecting while enabling discovery

### Why "Lamad"?

**Etymology**: לָמַד (lamad) is Hebrew for "to learn" or "to teach"

**Thematic Consistency**: Aligns with **Elohim** (Hebrew: God/gods), maintaining the protocol's linguistic identity

**Bidirectional Nature**: Captures both learning AND teaching through contribution - the essence of the Elohim Social Medium where:
- Learning = Building affinity and earning attestations
- Teaching = Contributing content that earns reach

**Beyond Documentation**: "Docs" is passive (reading information). "Lamad" is active (a journey of growth, mastery, and contribution).

**Encodes the Vision**: Like the Social Medium epic states - *"Where the medium itself encodes love"* - the name **Lamad** encodes:
- Learning as sacred journey
- Teaching as earned contribution
- Mastery through attestation
- The hero's journey to difficult truths
- Guardians of human flourishing

## Core Vision

### Reading Experience First

> "If we can't clearly enjoy reading the story of the living docs, and exploring the features of those epics, then the graph won't be much help yet"

The primary focus is on creating an excellent reading and browsing experience. Graph visualization is a future enhancement, not the foundation.

### Domain-Agnostic Content Model

Rather than rigid type hierarchies (Epic → Feature → Scenario), we use a **generic content model** similar to WordPress posts. This allows the system to be extended to any domain beyond software documentation.

### Scalar Affinity Tracking

Instead of discrete classifications like "practiced" or "mastered", we track the **abstract relationship strength** between a user and content using a scalar value (0.0 to 1.0). This remains domain-agnostic and can be interpreted differently across contexts.

## Key Concepts

### ContentNode

The generic content container that replaces rigid type hierarchies:

```typescript
interface ContentNode {
  id: string;
  contentType: string;        // Domain-specific: 'epic', 'feature', 'scenario', 'article', etc.
  title: string;
  description: string;
  content: string;
  contentFormat: ContentFormat; // 'markdown' | 'gherkin' | 'html' | 'plaintext'
  tags: string[];
  sourcePath?: string;
  relatedNodeIds: string[];
  metadata: ContentMetadata;   // Flexible key-value pairs for domain-specific extensions
  createdAt?: Date;
  updatedAt?: Date;
}

interface ContentMetadata {
  category?: string;
  authors?: string[];
  version?: string;
  status?: string;
  priority?: number;
  [key: string]: any;  // Domain-specific extensions
}
```

**Why generic?** Allows extension to any content domain without code changes. New content types are just different `contentType` values with custom metadata.

### Affinity

A **scalar value (0.0 - 1.0)** representing the strength of the user's relationship with a piece of content.

- `0.0` = Unseen, no engagement
- `0.01-0.33` = Low affinity (viewed once or twice)
- `0.34-0.66` = Medium affinity (multiple views, some engagement)
- `0.67-1.0` = High affinity (well-known, frequently referenced)

**Why scalar?** Avoids imposing domain-specific meanings like "practiced" or "mastered". Each domain can interpret the scalar differently.

**Auto-tracking:** First view automatically increments affinity from 0.0 to 0.2 for simple UX.

### Meaning Map View - Inspired by Khan Academy's "World of Math" and Gottman's Love Maps

The Meaning Map is a navigation and discovery interface that helps users journey toward a **target subject** - an overarching learning goal or understanding they want to achieve.

#### Core Concepts

**Target Subject**: The destination or mastery goal the user wants to reach. Examples:
- "Understanding the Elohim Protocol manifesto" (our prototype)
- A specific skill (like "factoring equations" in Khan Academy)
- Knowledge about another person (Gottman's love maps)
- Any concept or learning objective

For the Elohim Documentation prototype, the target subject is **pre-defined as "The Elohim Protocol"** (like "World of Math" in Khan Academy). In the abstract implementation, users would explicitly select their target: "I want to understand X."

**Taxonomy/Path (Scope and Sequence)**: The curated journey between the user's current position and the target subject. This is the "suggested path that tells a cohesive story." For our prototype, this path is **pre-authored** (like a global course curriculum) to guide users through the Elohim Protocol in a meaningful sequence.

**Affinity**: As the user explores nodes along (or off) the suggested path, they develop deeper **affinity** with that content. This tracks their actual engagement and relationship strength with each piece of content (0.0-1.0 scalar). Think of this as your *footprints* - where you've been, what you've explored.

**Attestations (Achievements/Credentials)**: Proof of capacity earned through completing journeys. Like badges in Khan Academy or licenses in ham radio, attestations represent demonstrated mastery, responsibility, or trust. Examples:
- "4th grade math mastery" (educational achievement)
- "Ham Radio Technician/General/Extra" (skill certification)
- "AS, BS, MS, Dr. Degree Equivalent" (academic credentials)
- "Closest Intimate Partner" (relationship trust - powerful!)
- "Civic Organizer Level 2" (proven community contribution)
- "Trauma Support Capacity" (emotional maturity for sensitive spaces)

Your unique path from A to B, validated by attestations and contributions along the way, becomes the proof of your capacity to handle more advanced, sensitive, or complex content.

**Orientation**: A derived metric that represents how well-positioned a particular node is to help the user progress toward the target subject, given their current affinity levels and earned attestations across the graph.

Think of it like Khan Academy's "mastery challenge" - which skills should be presented next is determined by:
- Your affinity with related content (what you've already explored)
- Your earned attestations (what you've proven you can handle)
- The collection of content in the subject you're trying to master
- Your position relative to the target subject

In our case:
- **Target Subject**: "The Elohim Protocol" (like "World of Math")
- **Epics**: Major domains like "algebra" - macro stories within the protocol
- **Content Nodes**: Specific skills/concepts to learn on the way to appreciating the vision

#### Fog of War: Earned Access to Content

Inspired by *The Legend of Zelda: Breath of the Wild's* Sheikah towers: You can see towers in the distance (orientation - you know advanced content exists), but you don't get the detailed map until you've journeyed across the landscape and climbed the tower (earned access through proven capacity).

**Progressive Revelation Model:**
- **Visible but minimal**: You see that advanced content exists - a title, category, or teaser
- **Locked with requirements**: Hovering/clicking shows what attestations you need to access full details
- **Journey highlighted**: The path to earn required attestations becomes part of your suggested path
- **Access granted on proof**: Once you've earned the attestations, the content fully reveals

**Why Some Content Requires Earned Access:**

From the Elohim Social Medium epic: "Speech is free but reach is earned." Extended further: *Visibility itself can be earned.*

Some content requires proven capacity to even see it:
- **Child safety**: Age-inappropriate content invisible until developmental milestones proven
- **Sensitive topics**: Trauma support spaces require "emotional maturity" attestations
- **Complex subjects**: Advanced political organizing requires civic engagement proof
- **Intimate relationships**: Deep personal content requires "trusted relationship" attestations
- **Professional domains**: Medical/legal content requires expertise credentials
- **Difficult truths**: Content that humans naturally wrestle with (the Elohim Protocol manifesto's deeper implications) requires the hero's journey to discover responsibly

**Access Control Negotiation:**

Content nodes can specify access requirements through **smart contracts for human flourishing** (negotiated by Elohim agents, expressed in plain text):

```
This content requires:
- Attestation: "Civic Engagement - Level 2"
  (earned through: neighborhood organizing + district contribution)
- OR Community endorsement from 3 current members
- AND Affinity > 0.5 with prerequisite content: [node-ids]

Steward: @neighborhood-council
Revocable: Yes (on community vote or harmful behavior)
```

Access requirements can be:
- **Author-defined**: Content creator specifies requirements
- **Community-governed**: Spaces collectively decide protection needs
- **Protocol-enforced**: Constitutional defaults for categories (child safety, etc.)
- **Agent-negotiated**: Elohim agents analyze content and suggest appropriate requirements
- **Dynamically adjusted**: Requirements can tighten during crisis or ease as trust builds

**Earning Attestations: Fun and Meaningfully Challenging**

Like Khan Academy's practice modules, earning attestations is designed to be engaging rather than burdensome:

**Practice/Exercise Nodes**: Special content types that create attestations upon completion:
- **Comprehension checks**: Demonstrate understanding of prerequisite concepts through reflection, synthesis, or application
- **Real-world application**: Use what you've learned in actual scenarios (like Emma's bus advocacy proving civic capacity in the Social Medium epic)
- **Community contribution**: Earn endorsements from others who've proven capacity - social proof of responsibility
- **Time and consistency**: Some attestations require sustained engagement over time - wisdom can't be rushed

**Journey as Proof**: Your unique path itself becomes the credential:
- Starting affinity with node A: 0.0
- Completed 7 practice exercises: +7 attestations
- Contributed solution that helped 3 neighbors: +community endorsement
- Maintained consistent engagement for 30 days: +consistency attestation
- Final affinity with node A: 0.8
- **Achievement unlocked**: "Civic Organizer - Level 2"

This achievement/badge now serves as a key to unlock content that requires civic maturity.

**Elohim as Guardian of Human Flourishing**:

The system helps you prove and attest to your capacity for responsibility. It makes growth:
- **Visible**: You see your progress toward attestations
- **Meaningful**: Attestations unlock real capabilities and access
- **Challenging**: Requirements are substantive, not trivial checkboxes
- **Fun**: The journey feels like exploration and discovery, not testing
- **Protective**: It prevents premature exposure to content you're not ready for

The "hero's journey to discover difficult truths" becomes encoded in the very structure of the Meaning Map.

**The Constellation Metaphor**: Think of a constellation of stars, where one star leads to the next. Each star (node) has its own connections and constellations of **varied content types** (like planets, moons, other stars).

Content within nodes can include multiple formats/categories:
- Features (BDD scenarios)
- Exercises
- Articles/posts
- Text files
- Images
- Videos
- Quizzes

Importantly, any particular piece of content could **bridge or be reused** across different subjects/contexts - like how a word can have different meanings based on context. This creates a rich, interconnected graph where content serves multiple learning paths.

#### In Our Implementation

For Elohim Protocol documentation, content nodes contain:
- **Epics** (macro stories) - the overarching narratives
- **User stories** within those epics - smaller narrative chunks
- **Scenarios** (.feature files) - executable specifications and examples

#### User Experience

The Meaning Map presents a **suggested path** toward the target subject, but allows exploration:

1. User sees the next recommended nodes on their path (sorted by orientation + affinity)
2. At each node, they can choose: follow the path, or explore something they're curious about
3. As they explore, affinity deepens with each node they engage
4. The path dynamically adjusts based on their exploration
5. Their **original intent** (the target subject) continues to **orient** them, keeping the core goal in focus even as they wander

#### What It Shows

- Content grouped by category/taxonomy
- Affinity levels with color coding (user's engagement history)
- Path progress bar (journey toward target subject)
- **List sorted by affinity AND orientation** - showing the most meaningful sequence toward the target subject to encourage productive exploration
- Compact at-a-glance graph visualization (future enhancement)

**Why hierarchical first?** Provides familiar list/browse UX that tells a cohesive story. Builds foundation for richer graph visualization later.

## Architecture Decisions

### Why Generic ContentNode Over Rigid Types?

**Problem:** DocumentNode → EpicNode → FeatureNode → ScenarioNode creates a rigid hierarchy that doesn't extend to other domains.

**Solution:** Single ContentNode type with flexible `contentType` and `metadata` fields.

**Benefits:**
- Add new content types without code changes
- Cross-domain applicability
- Simpler mental model
- Easier to maintain

### Why Scalar Affinity Over Discrete States?

**Problem:** Classifications like "practiced" and "mastered" impose domain-specific semantics.

**Solution:** Abstract 0.0-1.0 scalar representing relationship strength.

**Benefits:**
- Domain-agnostic
- Fine-grained tracking
- Natural progression
- Can derive discrete states if needed: `affinity > 0.67 ? 'mastered' : affinity > 0.33 ? 'practiced' : 'learning'`

### Why localStorage + Demo User?

**Problem:** Real user authentication adds complexity we don't need yet.

**Solution:** Hardcoded 'demo-user' with localStorage persistence.

**Benefits:**
- Rapid prototyping
- No auth overhead
- Focus on core UX
- Easy to upgrade to real user context later

**Trade-offs:**
- Single user per browser
- No cross-device sync
- Not production-ready for multi-user

### Why Meaning Map (Hierarchical View) Before Full Graph Visualization?

**Decision:** Build Meaning Map (list/tree view with suggested path) before full interactive graph visualization.

**Rationale:**
- Reading experience is foundational
- List views with curated paths tell cohesive stories
- Easier to implement and iterate on path/orientation logic
- Graph visualization requires solid content, affinity data, and orientation metrics first
- Users need to understand the "constellation" before navigating it visually

**Current Implementation:** Hierarchical list sorted by affinity, with pre-authored path through Elohim Protocol content.

**Future Enhancement:** Full graph visualization showing relationships, orientation vectors toward target subject, and alternative path options.

## Current Implementation

### Routes

**Composite Identifier URL Strategy**

Lamad uses composite identifiers in URLs to create self-documenting, graph-friendly navigation:

#### Pattern: `type:id:type:id:type:id...`

**Node Views** (even number of segments):
```
/lamad/epic:social-medium                                    → Epic content + collections
/lamad/epic:social-medium:feature:affinity                   → Feature content + collections
/lamad/epic:social-medium:feature:affinity:scenario:emma-bus → Scenario content (leaf)
```

**Collection Views** (odd number of segments):
```
/lamad/epic                                  → All epics
/lamad/epic:social-medium:feature            → All features in this epic
/lamad/epic:social-medium:feature:affinity:scenario → All scenarios in this feature
```

**Special Routes**:
```
/lamad                 → Home (landing, stats, epics list)
/lamad/map            → Meaning Map (full graph visualization)
/lamad/search         → Search interface
/lamad/content/:id    → Direct content access (fallback, backwards compatible)
```

**Query Parameters** (context enrichment):
```
?target=elohim-protocol  → Target subject for orientation
?attestation=civic-2     → Attestation journey tracking
?step=3                  → Step in suggested path
?from=node-id            → Source node (breadcrumb)
?depth=2                 → Graph traversal depth
```

#### Why Composite Identifiers?

1. **Self-Documenting**: URL explicitly shows types and relationships
   - `epic:social-medium` is clearly an epic, not ambiguous
   - Graph structure visible in URL

2. **Graph Database Alignment**: Similar to Neo4j/graph DB notation
   - Natural fit for DocumentGraph model
   - Easy to parse into path segments

3. **Domain-Agnostic**: Works for any content type
   - Not tied to epic→feature→scenario hierarchy
   - Extensible: `organization:acme:team:engineering:member:jane`

4. **Collection Views Built-In**: Odd segments = list view
   - `/lamad/epic:social-medium:feature` = "show me all features"
   - No separate route patterns needed

5. **Type Safety**: Parse validates type matches actual node
   - `epic:123` must point to an actual epic node
   - Prevents type confusion

#### Navigation Flow

1. **Home** (`/lamad`) → List of epics
2. Click epic → **Node view** (`/lamad/epic:social-medium`)
   - Shows epic content
   - Displays features collection below
3. Click feature → **Node view** (`/lamad/epic:social-medium:feature:affinity`)
   - Shows feature content
   - Displays scenarios collection below
4. Click scenario → **Node view** (`/lamad/epic:social-medium:feature:affinity:scenario:emma-bus`)
   - Shows scenario content (leaf node, no children)

#### Breadcrumbs

Automatically derived from composite path:
```
Home / Epic: Social Medium / Feature: Affinity Tracking / Scenario: Emma's Bus Advocacy
```

Type labels (`Epic:`, `Feature:`) extracted from path segments.

### Key Components

#### MeaningMapComponent (formerly MissionMapComponent)
- Hierarchical tree view organized by categories
- Color-coded affinity indicators (unseen/low/medium/high)
- Category progress bars showing journey toward target subject
- Expand/collapse sections
- Content sorted by affinity (lowest first to encourage exploration)
- **Future:** Sort by affinity AND orientation toward target subject ("The Elohim Protocol")
- **Future:** Highlight suggested path nodes (pre-authored sequence)
- **Future:** Show path alternatives and branching points
- **Future:** Fog-of-war UI elements:
  - Locked content shows title/teaser but not full details
  - Lock icon with tooltip showing required attestations
  - "Journey to unlock" button that highlights prerequisite path
  - Progress indicators toward earning required attestations
  - Visual distinction between "accessible", "locked but visible", and "completely hidden" content

#### ContentViewerComponent
- Unified viewer for all content types (markdown, Gherkin, HTML, plaintext)
- Auto-tracks views (increments affinity)
- Manual affinity controls (increment/decrement buttons)
- Circular affinity indicator with gradient colors
- Related content section
- Renders based on `contentFormat` field

#### DocsHomeComponent
- Welcome section with system overview
- Affinity statistics dashboard
- Distribution visualization (unseen/low/medium/high)
- Primary CTA to Meaning Map
- Search link

### Services

#### AffinityTrackingService
- localStorage persistence
- Hardcoded 'demo-user' ID
- Observable pattern (BehaviorSubject)
- Methods:
  - `getAffinity(nodeId): number`
  - `setAffinity(nodeId, value): void`
  - `incrementAffinity(nodeId, delta): void`
  - `trackView(nodeId): void` - Auto-increment to 0.2 on first view
  - `getStats(nodes): AffinityStats` - Aggregate statistics

#### DocumentGraphService
- Builds content graph from source files
- Parses markdown epics (.md)
- Parses Gherkin features (.feature)
- Manages node relationships
- Observable graph state

#### NavigationService
- Parses composite identifier URLs into navigation context
- Manages hierarchical navigation state through the graph
- Distinguishes between node views and collection views
- Observable navigation context (BehaviorSubject)
- Methods:
  - `navigateTo(type, id, options)` - Navigate to a node using composite identifier
  - `navigateToCollection(type, parentPath)` - Navigate to collection view
  - `navigateToHome()` - Navigate to home (epics list)
  - `navigateUp()` - Navigate up one level in hierarchy
  - `parsePathSegments(compositePath)` - Parse composite URL into context
  - `getBreadcrumbs(context)` - Build breadcrumb trail with type labels
- Key Features:
  - **View Mode Detection**: Odd segments = collection view, even = node view
  - **Type Validation**: Ensures URL type matches actual node type
  - **Children Resolution**: Gets children based on node type and relationships
  - **Parent Path Tracking**: Maintains full composite path for nested navigation

### Models

#### user-affinity.model.ts
- `UserAffinity` - User-to-content affinity mappings
- `AffinityStats` - Aggregate statistics
- Distribution buckets (unseen/low/medium/high)
- Per-category and per-type statistics
- **Future:** Add orientation metrics and target subject tracking
- **Future:** Add suggested path progress tracking
- **Future:** Add attestation tracking and achievement unlocking

#### attestations.model.ts (Future)
- `Attestation` - Earned achievements/credentials/badges
- `AttestationRequirement` - What's needed to earn an attestation
- `ContentAccessRequirement` - What attestations unlock which content
- `AttestationJourney` - The unique path taken to earn an attestation (proof of capacity)
- Achievement types: educational, skill-based, relational, civic, professional, time-based

#### content-node.model.ts
- `ContentNode` - Generic content container
- `ContentMetadata` - Flexible metadata structure
- `ContentFormat` - Supported formats enum
- `ContentGraph` - Graph structure
- `RelationshipType` - Node relationship types

#### Adapters

**document-node.adapter.ts** - Bidirectional conversion between legacy DocumentNode and new ContentNode for backward compatibility during migration.

## Content Sources

### Markdown Epics (.md files)
- Narrative documentation
- Vision and context
- Located in project documentation directories

### Gherkin Features (.feature files)
- Behavior-Driven Development scenarios
- Executable specifications
- Located in feature directories
- Parsed by GherkinParser

### Future Content Types
The generic model supports any content type:
- API documentation
- Tutorial articles
- Code examples
- Video transcripts
- Design documents

## Navigation Flow

```
Home (Stats & Overview)
  ↓
Meaning Map (Browse by Category)
  ↓
Content Viewer (Read & Track Affinity)
```

Simple, focused user journey prioritizing reading experience.

## Future Considerations

### Attestation System & Earned Access
- **Attestation data model**: Track earned achievements, credentials, badges with journey provenance
- **Access requirement negotiation**: Smart contracts (plain text, agent-negotiated) defining what attestations unlock which content
- **Practice/exercise nodes**: Special content types that create attestations upon completion
- **Journey tracking**: Record the unique path taken to earn each attestation as proof of capacity
- **Credential wallet UI**: Display earned attestations, progress toward next achievements
- **Fog-of-war visualization**: Progressive revelation of locked content in Meaning Map
- **Steward/revocation system**: Content authors and community spaces can revoke access based on behavior
- **Cross-domain attestations**: Educational, skill-based, relational ("Closest Intimate Partner"!), civic, professional credentials
- **Time-based attestations**: Some achievements require sustained engagement over time (wisdom can't be rushed)
- **Community endorsement**: Social proof as attestation mechanism

### Orientation & Path Navigation
- **Orientation calculation**: Derive metric showing how well-positioned each node is to help user progress toward target subject, considering both affinity and attestations
- **Target subject selection**: Allow users to explicitly choose their learning goal (for prototype, hardcoded as "The Elohim Protocol")
- **Suggested path highlighting**: Visual indicators showing pre-authored path nodes
- **Path branching**: Show alternative routes and curiosity-driven detours
- **Dynamic path adjustment**: Recalculate suggested path based on user's actual exploration patterns AND earned attestations
- **Path progress tracking**: Show completion percentage toward target subject
- **Attestation-aware pathfinding**: Suggest journeys that help earn required attestations for locked content user wants to access

### Graph Visualization
- Visual node-edge representation showing constellation of content
- Interactive exploration with orientation vectors pointing toward target subject
- Force-directed layout that respects suggested path structure
- Relationship highlighting (prerequisites, bridges, alternatives)
- Multiple content types within nodes (constellation metaphor: planets, moons, stars)
- Deferred until reading experience and orientation logic are solid

### Real User Context
- Replace hardcoded 'demo-user' with actual authentication
- Per-user affinity tracking
- Cross-device synchronization
- Backend persistence

### Content Constellation & Reusability
The constellation metaphor allows rich content composition:
- **Multiple content types within nodes**: A single node can contain varied formats (text, images, videos, exercises, quizzes)
- **Cross-context reusability**: Content can bridge multiple subjects/paths - like a word with different meanings based on context
- **Hierarchical and networked**: Content can be both nested (planets with moons) and interconnected (constellations)
- **Domain-agnostic categories**: Features, exercises, articles, media all treated as generic content types

### Additional Content Types
The generic model is ready for:
- Blog posts and articles
- Step-by-step tutorials
- API references and code examples
- Video content and transcripts
- Interactive demos and exercises
- Quizzes and assessments
- Images and diagrams
- Audio content (podcasts, lectures)

### Test Status Integration
- Pull test results from CI/CD pipelines
- Display test status in Meaning Map
- Filter by passing/failing tests
- Track test coverage as metadata

### Epic Manifest Loading
Currently only feature manifest is loaded. Epic manifest parsing is in place but not integrated.

## Design Principles

1. **Generic over specific** - Extensibility through abstraction
2. **Scalar over discrete** - Fine-grained, domain-agnostic measurement
3. **Simple over complex** - localStorage before database, demo user before auth
4. **Reading before graphing** - Content experience is foundational
5. **Progressive enhancement** - Build solid foundation, add visualization later

## For Future Agents

When working on this feature:

1. **Maintain domain-agnosticism** - Don't hardcode software-specific assumptions
2. **Use scalar affinity** - Avoid adding discrete state classifications
3. **Extend through metadata** - Don't modify ContentNode interface for domain-specific needs
4. **Test reading experience first** - UX quality trumps visualization complexity
5. **Keep navigation simple** - Three-step flow: Home → Meaning Map → Content
6. **Remember the target subject** - All orientation and path decisions should serve the user's journey toward their learning goal
7. **Pre-author paths for prototype** - Use hardcoded suggested paths (like a global curriculum) before building dynamic path generation
8. **Think in constellations** - Content can be nested, varied in type, and reused across different contexts
9. **Affinity deepens, attestations prove, orientation guides** - Affinity tracks engagement history, attestations demonstrate capacity, orientation shows the way forward
10. **Make growth visible and fun** - Earning attestations should feel like exploration and discovery, not testing
11. **Protect through progressive revelation** - Fog-of-war isn't just game design, it's human flourishing encoded
12. **Journey is the credential** - The unique path taken + contributions made = proof of capacity
13. **Smart contracts in plain text** - Access requirements should be negotiable by agents and readable by humans

## Questions?

If you're unsure about architectural decisions, refer back to these principles:
- Would this work for non-software documentation? (WordPress test)
- Does this impose domain-specific semantics? (Generic test)
- Does this improve the reading experience? (UX test)
- Can this be added through metadata? (Extension test)
- Does it help the user progress toward their target subject? (Orientation test)

### Key Distinctions to Remember

**Affinity vs Attestations vs Orientation**:
- **Affinity** = Historical engagement (where you've been, what you've explored) - your footprints
- **Attestations** = Proven capacity (what you've demonstrated mastery of) - your credentials
- **Orientation** = Directional guidance (where you should go next to reach your target, considering your attestations) - your compass
- Think: Affinity tracks the journey, attestations prove the growth, orientation guides the way forward

**Target Subject vs Content Node**:
- **Target Subject** = Overarching learning goal ("Understanding the Elohim Protocol")
- **Content Node** = Individual piece of content along the journey (an epic, feature, scenario)
- Think: Target subject is the destination, content nodes are waypoints

**Suggested Path vs Graph**:
- **Suggested Path** = Pre-authored, cohesive sequence that tells a story
- **Graph** = All possible connections and relationships between content
- Think: Path is the highway, graph is the full road network

**Mission Map vs Meaning Map**:
- **Mission Map** = Simple hierarchical browse/list view
- **Meaning Map** = Curated journey with target subject, orientation, and paths
- The Meaning Map is our implementation - it's not just browsing, it's guided discovery toward understanding

**Visibility vs Reach vs Access** (from Elohim Social Medium):
- **Visibility** = What content you can see exists (fog-of-war: some content locked until attestations earned)
- **Access** = What content you can read/consume (requires appropriate attestations)
- **Reach** = How far your contributions can travel (requires community trust and evidence)
- "Speech is free, but reach is earned" extends to "Visibility itself can be earned"
- Children, the vulnerable, and the developing gradually earn access as they prove capacity
- Protects against premature exposure while enabling growth

#### The Living Documentation as Elohim Social Medium

The Meaning Map is the first implementation of the Elohim Social Medium principles applied to documentation:

From the epic: *"Speech is free but reach is earned. Where attention is sacred and data is sovereign. Where communities own their spaces. Where the medium itself encodes love."*

In our context:
- Documentation nodes = Contributions in the social medium
- Earned access = The same trust-based visibility controls that protect children and prevent extremism
- Attestations = Proof of capacity that earns both access (to read) and reach (to contribute)
- The hero's journey = The path toward understanding difficult truths, encoded in progressive revelation
- Guardians of human flourishing = The system helps you prove responsibility while protecting against premature exposure

This isn't just documentation - it's a proof-of-concept for how **attention becomes sacred, reach is earned, and human flourishing is protected through design**.
