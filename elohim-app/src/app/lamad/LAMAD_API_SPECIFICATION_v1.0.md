# Lamad API Specification v1.0
## Interface Control Document for Path-Centric Learning Infrastructure

**Document Version:** 1.0  
**Last Updated:** 2024-11-24  
**Status:** Definitive Specification

---

## Part 0: The Vision

### Overview

Lamad (לָמַד - Hebrew: "to learn/teach") is a graph-based learning platform inspired by Khan Academy's "World of Math" Meaning Map and Gottman's Love Maps. It provides a domain-agnostic, extensible content model with user affinity tracking, attestation-based achievement unlocking, and orientation-based navigation toward a target subject.

This isn't just documentation - it's the first implementation of Elohim Social Medium principles applied to learning: where attention is sacred, reach is earned, visibility itself can be earned, and human flourishing is protected through design.

### Conceptual Inspirations

**Khan Academy's "World of Math":**

The World of Math interface establishes a target subject representing the mastery goal - in their case, mathematical proficiency across all domains. Content is organized into categories representing major mathematical fields like algebra, geometry, and calculus. Within each category are individual skills representing specific concepts to master. The mastery challenge system determines your next recommended skills based on your demonstrated progress and understanding. 

Our implementation adapts this model by replacing "World of Math" with "The Elohim Protocol" as the target subject. What Khan calls categories, we call epics - major domains of understanding within the protocol. Individual content nodes serve the role of skills - atomic units of knowledge to be mastered. The system tracks your journey toward comprehensive understanding of the protocol's vision for human flourishing through technology.

**Gottman's Love Maps:**

John Gottman's research on relationships introduced the concept of Love Maps - the mental space where you store detailed knowledge about your partner. The target subject is understanding another person in their full complexity. Content nodes include facts about their life, their preferences and dislikes, their personal history and formative experiences, and their dreams and aspirations for the future.

Affinity in this model represents how well you know different aspects of your partner. Orientation guides you toward what to learn next to deepen the relationship - perhaps you know their childhood well but not their current stressors at work. Attestations in this domain might include "Closest Intimate Partner" - a status earned through the sustained journey of building trust and deep knowledge.

Our abstraction recognizes that any subject, whether a person, a skill domain, or a conceptual framework, can be mapped this way. Learning is fundamentally about building relationship - with ideas, with practices, with communities of understanding.

**Zelda: Breath of the Wild's Fog of War:**

The game establishes a target of mapping the kingdom of Hyrule and ultimately defeating Ganon. Sheikah Towers are visible from a distance, providing orientation toward goals, but the surrounding terrain details remain obscured until you make the journey to climb them. Climbing the tower requires effort and often solving environmental puzzles - you earn access through demonstrated capability. Once climbed, the tower reveals the surrounding map progressively, showing you content you've proven yourself ready to navigate.

Our abstraction applies this principle to knowledge: content visibility itself can be earned, protecting learners from overwhelm while enabling discovery. You can see that advanced concepts exist (providing orientation), but the details remain appropriately gated until you've built the necessary foundation. This isn't artificial scarcity - it's cognitive respect and pedagogical wisdom encoded into the architecture.

### Why "Lamad"?

The name carries multiple layers of meaning that align with the protocol's philosophy:

**Etymology:** לָמַד (lamad) is Hebrew for "to learn" or "to teach" - a single word encompassing both sides of the educational exchange.

**Thematic Consistency:** The name aligns with Elohim (Hebrew: אֱלֹהִים - divine messengers or judges), maintaining the protocol's linguistic identity rooted in concepts of guidance, wisdom, and sacred trust.

**Bidirectional Nature:** The word captures both learning AND teaching through contribution. This reflects the essence of the Elohim Social Medium where learning builds affinity (relationship with content) and teaching earns reach (the right to guide others). Knowledge flows in both directions.

**Beyond Documentation:** "Docs" suggests passive consumption of information. "Lamad" is active - a journey of growth, mastery, and contribution. It's not about reading what exists; it's about becoming through engagement.

**Encodes the Vision:** As the Social Medium epic states, "Where the medium itself encodes love," the name Lamad encodes our core principles: learning as a sacred journey, teaching as earned contribution, mastery through attestation, the hero's journey to difficult truths, and guardians of human flourishing rather than extractors of attention.

### Terminology Distinction: Elohim vs. Lamad

To avoid architectural confusion, we maintain a strict distinction between the actors and the medium:

**Elohim: The Active Agents**

These are the real-time, intelligent entities operating within the system. Examples include community_elohim (stewarding shared knowledge spaces), family_elohim (protecting household learning priorities), and personal_agent (your individual guide through the knowledge graph). They negotiate access based on your attestations and readiness. They track patterns in how you learn and what you struggle with. They facilitate coordination between your learning goals and available paths. They are the "ghost in the machine" - the intelligence layer that makes the static structure dynamic and responsive.

**Lamad: The Maps and Paths of Meaning**

This is the graph-based learning platform and content repository. It defines the node types (epics, features, scenarios, concepts) and their metadata. It is the map showing the territory of available knowledge. It records the footprints - your affinity relationships with different concepts showing where you've traveled and how deeply you've engaged. It is the "machine" that the ghosts (Elohim agents) inhabit and curate. Without Elohim, Lamad is a library. With Elohim, Lamad becomes a living learning environment.

This separation matters because it clarifies what lives where in the architecture. Content nodes, path structures, and relationship graphs are Lamad concerns. Decision-making about what to show you next, how to adapt explanations to your context, and when to unlock advanced material - those are Elohim concerns. The specification you're reading now defines Lamad. Future specifications will define how Elohim agents interact with and animate this structure.

### Core Design Philosophy

Lamad operates on three fundamental separations of concern that distinguish it from traditional learning management systems:

**Territory** represents the immutable content nodes that exist in the knowledge graph. These are the atomic units of knowledge - videos explaining concepts, documents detailing processes, simulations allowing experimentation, interactive exercises building skills, book chapters providing depth, and any other educational resource imaginable. Territory is content-agnostic, meaning the system doesn't hardcode specific content types but uses a renderer registry that can be extended to handle new formats. Territory is renderer-extensible, allowing organizations to add support for emerging media types like VR experiences or haptic interfaces without modifying core system code. In Holochain terminology, these are entries in the distributed hash table that can be referenced by their content-addressed hashes, making them immutable and verifiable.

**Journey** represents the curated Paths that add narrative meaning and sequence to Territory resources. A path is fundamentally different from a playlist or collection. A playlist simply groups related items. A path provides context about why each resource matters at this specific point in a learner's progression. It explains how this concept builds on what came before and prepares for what comes next. It frames the resource in terms of the learner's goals and current understanding. The same resource - say, a video about active listening - might appear in a marriage counseling path ("hearing your partner's dreams beneath their words"), a workplace leadership path ("understanding employee concerns to build trust"), and a conflict resolution path ("de-escalating tension through genuine presence"). Same content, completely different meanings based on the journey context.

**Traveler** represents the Agents - whether human users, organizations, or AI entities - whose progress and perspectives shape their experience of the system. Each traveler has their own source chain, a personal record of their learning journey including which steps they've completed, when they engaged with content, how deeply they understood it (affinity), what attestations they've earned proving capacity, and what reflections or notes they've recorded. Progress tracking is private by default and lives on the traveler's own chain in Holochain deployment, giving them full sovereignty over their learning data. No institution owns your educational history - you do. You can choose to share progress with teachers, employers, or communities, but the default is private. This stands in sharp contrast to traditional learning management systems where the institution controls and owns all learner data centrally.

This three-part architecture - Territory holding knowledge, Journey creating meaning, Traveler maintaining sovereignty - forms the foundation for everything that follows in this specification.

---

## Purpose and Audience

This document defines the URL strategy, data models, and integration patterns for the Lamad learning platform. It serves as the architectural contract between frontend views, backend services, and future peer-to-peer (Holochain) deployment.

**Intended Audience:** This specification is written for developers implementing Lamad features, AI assistants (Claude, Gemini) generating code based on project context, and technical stakeholders evaluating the architecture.

**How to Use This Document:** When implementing any feature in Lamad, begin by consulting the relevant section of this specification. Treat the URL patterns, data models, and service interfaces as binding contracts. Any deviation from these patterns should be documented as a deliberate architectural decision with justification.

---

## Design Philosophy

Lamad operates on three fundamental separations of concern that distinguish it from traditional learning management systems:

**Territory** represents the immutable content nodes that exist in the knowledge graph. These are the atomic units of knowledge - videos, documents, simulations, interactive exercises, books, and any other educational resource. Territory is content-agnostic and renderer-extensible, meaning new content types can be added without modifying core system architecture. In Holochain terminology, these are entries in the distributed hash table that can be referenced by their content-addressed hashes.

**Journey** represents the curated paths that add narrative meaning and sequence to Territory resources. A path is not merely a playlist or a collection. It provides context about why each resource matters at this specific point in a learner's progression. The same resource might appear in multiple paths with completely different narratives. This separation allows content to be reused across diverse learning contexts without duplication. Paths are first-class entities that can be created, forked, remixed, and shared by teachers, organizations, AI agents, or any other curator.

**Traveler** represents the agents (human users, organizations, AI entities) whose progress and perspectives shape their experience of the system. Each traveler has their own source chain of learning events, attestations earned, and affinity relationships with content. Progress tracking is private by default and lives on the traveler's own chain, giving them full sovereignty over their learning data. This approach differs fundamentally from centralized LMS platforms where the institution owns all learner data.

This architecture makes Lamad fundamentally different from Khan Academy (content-first library) or Kolibri (resource distribution). Lamad is a **meaning-making infrastructure** where the journey through knowledge is as important as the knowledge itself.

---

## Part 1: URL Strategy and Routing

The URL architecture reflects the design philosophy. Most URLs describe journeys (paths), with secondary support for exploring territory (resources) and understanding travelers (agents). The patterns are designed to remain stable even as the underlying implementation migrates from centralized servers to peer-to-peer Holochain infrastructure.

### 1.1 Primary User Experience - Path Navigation

The overwhelming majority of learner interactions follow curated paths. The URL structure makes this pattern primary and explicit.

**Route Pattern:**
```
/lamad/path:{pathId}/step:{stepIndex}
```

The pathId is an opaque identifier that might be a human-readable slug in the prototype (like "gottman-love-map") but will become a content-addressed hash in production Holochain deployment (like "uhCkkXyz123"). Developers should treat this as an opaque string and never parse its internal structure. The stepIndex is a zero-based integer representing position in the path sequence. Step zero is always the first step of any path.

**Examples:**
```
/lamad/path:default-elohim-protocol/step:0
/lamad/path:gottman-love-map/step:5
/lamad/path:vocational-training-software/step:12
/lamad/path:depolarization-bridge-left-right/step:3
```

**Behavior Contract:**

When this URL is requested, the system executes a specific sequence of operations. First, the PathService loads the LearningPath entity identified by pathId. This returns the path metadata (title, description, purpose, estimated duration) and the complete sequence of steps. However, the service does not load the actual content for every step at this point. That would violate the lazy loading constraint.

Second, the service looks up the specific step at the given stepIndex. The PathStep structure contains a resourceId that points to a ContentNode in the Territory. The service fetches this specific content node, but importantly, it does not fetch content for neighboring steps unless explicitly requested.

Third, the view layer receives both the PathStep context (the step-specific narrative about why this resource matters in this journey) and the ContentNode itself (the actual educational content to be rendered). The view overlays the path-specific narrative on top of the generic content. This means the same video about active listening might appear in both a marriage counseling path and a workplace leadership path, but with completely different framing and learning objectives.

Fourth, the view provides Previous and Next navigation buttons that stay within this path context. Clicking Next increments the stepIndex and navigates to the next URL. The browser back button works naturally because each step has its own URL. The navigation stays within the path - users do not accidentally jump to unrelated resources.

**Critical Constraint - Lazy Loading and Fog of War:**

The service must NOT preload content beyond stepIndex plus two. This is not a performance optimization that can be disabled. It is a fundamental architectural constraint that serves multiple purposes. For learners, it prevents cognitive overload by showing only what is immediately relevant. For the system, it makes graph traversal expensive, which is precisely what we want. Even a superintelligent AI agent cannot "see" the entire graph without paying the computational cost of walking through it step by step with declared purpose. This fog of war principle ensures that exploration requires intentionality rather than being a free zero-cost operation.

**Path Overview Route:**
```
/lamad/path:{pathId}
```

This displays the path landing page before the learner commits to following it. The view shows path metadata including title, full description, estimated duration, difficulty level, prerequisite paths if any, and a high-level outline of the step sequence. The outline shows step titles but does not load the full content for each step. This gives learners enough information to decide if this path fits their needs without forcing them to load dozens or hundreds of content resources.

The path overview is also where learners can see their progress if they have previously started this path. A progress indicator shows which steps they have completed. A "Continue" button takes them directly to their current step, while a "Start from Beginning" button allows restarting.

### 1.2 Secondary Access - Direct Resource Viewing

Sometimes learners need to access a specific resource outside of any path context. This happens when following citations from academic papers, responding to shared links from colleagues, or exploring the Territory graph directly as researchers or path creators.

**Route Pattern:**
```
/lamad/resource:{resourceId}
```

Like pathId, the resourceId is opaque and will become a content hash in production. The prototype uses human-readable identifiers for clarity during development.

**Examples:**
```
/lamad/resource:epic:social-medium
/lamad/resource:simulation:evolution-of-trust
/lamad/resource:video:gottman-dreams-within-conflict
/lamad/resource:organization:sensorica
```

**Behavior Contract:**

The ContentService loads the ContentNode entity identified by resourceId. The view displays this content in its generic form without any path-specific narrative overlay. Instead of saying "This step teaches you about cooperation in the context of building trust in your marriage," it shows the generic description like "An interactive simulation exploring game theory and the emergence of cooperation."

The view includes a "This resource appears in these paths" section that shows back-links to any paths containing this resource. Each back-link indicates which step number in that path uses this resource, allowing the learner to see the different contexts where this same content might be valuable. For example, the Evolution of Trust simulation might appear in a game theory course, a conflict resolution training path, and a negotiation skills path, each with different pedagogical framing.

The view also shows related resources based on graph relationships. By default, this shows resources that are one hop away (directly linked). The relationships might be semantic (this implements that concept), hierarchical (this belongs to that collection), or sequential (this comes before that). Clicking on a related resource navigates to that resource's direct access URL.

**Important Note:** Direct resource access does NOT show path-specific learning objectives, completion tracking, or navigation within a path sequence. Those features only exist when viewing a resource through a path context. This distinction helps learners understand when they are following a curated journey versus exploring the raw territory.

**Future Holochain Migration:**

In production deployment, resourceId will be a content-addressed hash like "uhCkk8f4j3k2l...xyz" rather than a human-readable slug. The URL pattern remains identical. The routing logic already treats resourceId as an opaque string, so this migration requires no code changes in the routing layer. Only the identifier format changes.

### 1.3 Agent Context and Progress

Agent-specific views show personalized information about a traveler's learning journey. These routes handle authentication, progress tracking, and the learning frontier where a traveler is ready to explore next.

**Critical Security Principle:**

Agent identity is derived from the Authentication Context, not from the URL path. This prevents ID enumeration attacks where a malicious user could try different agent identifiers in URLs to discover information about other learners. The authenticated agent's identity comes from a session token or JWT that the backend verifies, never from a URL parameter that could be manipulated.

**Route Patterns for Current User:**
```
/lamad/me/paths/following
/lamad/me/paths/completed
/lamad/me/paths/created
/lamad/me/learning-frontier
/lamad/me/settings
/lamad/me/attestations
```

All routes beginning with "/lamad/me/" require authentication. Unauthenticated requests receive a redirect to the login page with a return URL so they can resume after authenticating. The "me" segment is resolved by the backend to the authenticated agent's identifier. The frontend never needs to know or handle the agent ID directly.

**Following Paths Route:**

Shows paths the agent has started but not completed. Each path displays a progress indicator showing which steps are done, the current step where they left off, and estimated time remaining based on their pace. A "Continue" button takes them directly to their current step in each path.

**Completed Paths Route:**

Shows paths the agent has finished. Each completed path displays completion date, time taken, and any attestations earned through completing that path. Learners can revisit completed paths to refresh their knowledge, and the system tracks that this is review rather than initial learning.

**Created Paths Route:**

For agents who have path-creator attestations, this shows paths they have authored. Each path displays analytics like how many other learners are following it, where learners tend to drop off, and which steps have highest affinity scores. Path creators can edit their paths or create new versions based on this data.

**Learning Frontier Route:**

This is one of the most powerful views in the system. The learning frontier shows resources where the agent has met all prerequisites but has not yet explored the content. It answers the question "What am I ready to learn next?" by analyzing the agent's completed steps, earned attestations, and affinity patterns against the graph of available content. The frontier might suggest several options, allowing the agent to choose based on current interest or need. The system might also explain why certain content is not yet on the frontier (missing prerequisites, locked by attestation requirements).

**Settings Route:**

Allows agents to configure their experience, including privacy settings (what information is publicly visible), notification preferences, and learning goals (which paths or subjects they want to prioritize).

**Attestations Route:**

Displays all attestations the agent has earned, organized by type (educational, skill-based, relational, civic, time-based). Each attestation shows when it was earned, through which path or activity, and what capabilities or content it unlocks. This serves as both a credential wallet and a progress visualization.

**Public Agent Profiles:**
```
/lamad/agent:{agentId}/profile
/lamad/agent:{agentId}/paths/created
```

These routes allow viewing public information about other agents. The profile shows only information the agent has chosen to make public, such as their bio, published paths they have created, and public attestations they have earned. Private progress on paths is never exposed through these routes. An agent viewing their own public profile sees it as others would see it, helping them understand what they are sharing.

The paths/created route shows paths this agent has authored that are marked as public visibility. Private or organization-scoped paths do not appear here unless the viewer has appropriate access.

### 1.4 Graph Exploration and Research

Advanced users like researchers, path creators, and AI agents can explore the knowledge graph using query-based operations. These routes respect attestation requirements and computational cost limits because graph traversal is expensive by design.

**Route Pattern:**
```
/lamad/explore
```

Graph exploration uses query parameters to specify the exploration operation rather than encoding traversal operations in the URL path. This keeps the URL structure clean and makes the intent explicit.

**Query Parameters:**
```
?focus={resourceId}          // Center point of exploration
&depth={1|2|3}              // How many graph hops to traverse
&relationship={type}         // Filter edges by relationship type
&view={graph|list|tree}     // Visualization mode
```

**Examples:**
```
/lamad/explore?focus=epic:social-medium&depth=1&view=graph
/lamad/explore?focus=concept:mutual-aid&depth=2&relationship=implements
/lamad/explore?focus=scenario:meal-prep&depth=1&view=list
```

**Behavior Contract:**

The ExplorationService receives the query parameters and validates that the requesting agent has appropriate attestations for the requested depth. Depth one (immediate neighbors) is available to all authenticated users. Depth two requires the "graph-researcher" attestation. Depth three requires "advanced-researcher" attestation and is heavily rate-limited because it can traverse large portions of the graph.

The service returns a subgraph centered on the focus resource. For depth one, this includes the focus resource and all directly connected resources. For depth two, it includes those plus resources connected to the first-hop neighbors. The response includes both nodes and edges with their relationship types.

The view parameter determines how this subgraph is displayed. Graph view renders an interactive force-directed graph visualization using D3.js or similar, where nodes can be dragged, clicked for details, and filtered by type. List view shows a hierarchical outline organized by relationship type. Tree view shows an expandable tree structure if the subgraph is acyclic.

Each exploration query logs its computational cost including nodes traversed, edges examined, and milliseconds elapsed. This cost is visible to the user and counts against their rate limit. Users without advanced attestations can perform ten depth-one explorations per hour. Researchers with attestations can perform twenty-five depth-two explorations per hour. The rate limit resets hourly.

**Pathfinding Query (Future Enhancement):**
```
/lamad/explore?from={resourceA}&to={resourceB}&algorithm={shortest|semantic}
```

This finds paths through the knowledge graph between two resources. The shortest algorithm uses Dijkstra's algorithm to find the minimum number of hops. The semantic algorithm considers relationship types and prefers meaningful pedagogical sequences over mere connectivity. This requires "path-creator" attestation and is rate-limited to five queries per hour because it can be very computationally expensive on large graphs.

The result is a suggested sequence of resources that could become a new learning path. The path creator can review this suggestion, add narrative to each step, and publish it as a new curated path.

### 1.5 Path Creation and Management

Users with appropriate attestations can author new paths, fork existing paths to create variants, or analyze the effectiveness of paths they have created.

**Route Patterns:**
```
/lamad/path/new
/lamad/path:{pathId}/edit
/lamad/path:{pathId}/fork
/lamad/path:{pathId}/analytics
```

**New Path Creation Route:**

Displays the path creation interface where authors can define path metadata (title, description, purpose, estimated duration), search the Territory for resources to include, drag resources into a sequence to define steps, and add narrative context to each step explaining why this resource matters at this point in the journey.

The interface supports searching by resource type, tags, or full-text content. Clicking on a search result shows a preview of the resource. The author can add it to the path sequence, and then write the step-specific narrative, learning objectives, and completion criteria. The interface makes it clear that they are not modifying the resource itself, only adding a layer of context about how it fits into this particular path.

**Edit Path Route:**

Allows modification of paths the authenticated agent has created. The agent can reorder steps using drag-and-drop, add or remove steps, edit step narratives, or update path metadata. If the path has already been published and other learners are following it, the edit creates a new version rather than modifying the existing version. Learners who were following the old version can choose to upgrade to the new version or continue with the version they started.

**Fork Path Route:**

Creates a copy of an existing path owned by the current agent. This is useful when someone wants to create a variant of a well-designed path for a different audience or purpose. For example, a teacher might fork a "Introduction to Programming" path designed for adults and adapt it for middle school students by changing the step narratives to reference age-appropriate examples and adjusting the pacing.

The forked path maintains a reference to its parent path (the forkedFrom field in the data model), creating a lineage that helps the community understand how paths evolve and build on each other.

**Analytics Route:**

Shows effectiveness metrics for paths the agent has created. The analytics include how many learners have started this path, completion rates overall and per-step (identifying where learners drop off), average time per step compared to estimates, affinity patterns showing which steps learners engage with most deeply, and feedback or ratings if the path collects them.

Path creators can use these insights to improve their paths. If many learners drop off at step seven, perhaps that step needs better prerequisites or clearer learning objectives. If a step has much lower affinity than surrounding steps, perhaps the content does not fit well or the narrative does not motivate engagement.

### 1.6 Landing Page and Discovery

**Route Pattern:**
```
/lamad
```

The landing page is fundamentally path-centric rather than content-centric. This distinguishes Lamad from traditional learning platforms that display catalogs of courses or libraries of resources.

**Behavior Contract:**

For unauthenticated visitors, the page displays featured paths curated by the organization running the Lamad instance. These might be paths addressing common learning goals, paths that have high completion rates and positive feedback, or paths that serve strategic organizational priorities.

For authenticated learners, the page shows a personalized view. At the top is a "My Learning" section showing active paths with progress indicators and quick links to continue where they left off. Below that is a "Recommended for You" section suggesting paths based on the learner's completed paths, attestations earned, and stated learning goals. The recommendation algorithm considers both prerequisite relationships (what are you now ready for) and semantic similarity (if you liked this, you might like that).

The page also provides search and browse interfaces for discovering new paths. Learners can search by keyword, filter by tags or difficulty level, or browse by category (relational learning, vocational training, civic education, etc.). Each path preview shows title, description, estimated duration, prerequisite paths if any, and social proof like number of learners who have completed it.

Importantly, the landing page does NOT display raw lists of epics, features, resources, or other Territory artifacts. Those are accessible through the exploration interface for researchers and path creators, but the default learner experience emphasizes curated journeys over raw content browsing. This reflects the core philosophy that meaning comes from the path through knowledge, not from the knowledge artifacts themselves.

### 1.7 Knowledge Maps - Polymorphic Learning Territory

Knowledge maps extend the path-centric navigation to support three distinct but architecturally similar learning contexts: domain knowledge (like Khan Academy's World of Math), person knowledge (like Gottman's Love Maps), and collective knowledge (organizational intelligence).

**Route Patterns:**
```
/lamad/map:{mapId}                           # View a specific knowledge map
/lamad/map:{mapId}/node:{nodeId}             # View a node within a map
/lamad/maps/mine                              # My knowledge maps
/lamad/maps/shared                            # Maps shared with me
/lamad/maps/new?type={domain|person|collective}  # Create new map
```

**Map Type-Specific Routes:**
```
# Domain maps (learning a subject)
/lamad/map:domain:{subjectId}                # Map for a content graph

# Person maps (learning about someone - Gottman Love Maps)
/lamad/map:person:{subjectAgentId}           # Map about a specific person
/lamad/maps/people                            # All my person maps

# Collective maps (organizational knowledge)
/lamad/map:collective:{orgId}                # Map for an organization
/lamad/maps/collectives                       # Collective maps I'm part of
```

**Examples:**
```
/lamad/map:domain:elohim-protocol            # My understanding of the protocol
/lamad/map:person:agent-sarah                # My knowledge map about Sarah
/lamad/map:collective:acme-corp              # ACME's collective knowledge base
/lamad/maps/people                           # All my relationship maps
```

**Behavior Contract:**

For domain maps, the system loads the user's personalized view of the content graph. This includes their affinity levels for each node, their mastery progression, and their personal annotations. The map view shows the same content as the graph explorer but overlays the user's relationship with each piece of knowledge.

For person maps, privacy is paramount. The map is private by default and only visible to the mapper. If the subject grants consent, both parties can see each other's maps (mutual visibility). The map structure follows Gottman's categories: life history, current stressors, dreams/aspirations, values/beliefs, preferences/dislikes, friends/family, work/career, and custom categories.

For collective maps, access depends on membership. Members with appropriate roles can view and contribute. The map represents shared knowledge owned by the collective, not by any individual.

**Consent Routes for Person Maps:**
```
/lamad/consent/requests                      # View pending consent requests
/lamad/consent/grant:{requestId}             # Grant consent to a request
/lamad/consent/revoke:{mapId}                # Revoke previously granted consent
```

When Agent A creates a person map about Agent B, Agent B receives a consent request. Agent B can grant access at various scopes: public-info (only what B shares publicly), shared-only (what B explicitly shares with A), or full-access (deep knowledge mapping permitted). Agent B can also set transparency level: none, categories-only, full-read, or collaborative.

### 1.8 Path Extensions - Learner-Owned Customization

Path extensions allow learners to personalize curated paths without modifying the original. This enables community contribution, A/B testing of variations, and adaptive learning.

**Route Patterns:**
```
/lamad/path:{pathId}/extensions              # View extensions for a path
/lamad/path:{pathId}/extend                  # Create new extension
/lamad/extension:{extensionId}               # View specific extension
/lamad/extension:{extensionId}/edit          # Edit my extension
/lamad/extension:{extensionId}/fork          # Fork someone else's extension
/lamad/extension:{extensionId}/propose       # Propose upstream merge
```

**Examples:**
```
/lamad/path:elohim-protocol/extensions       # Community extensions to the protocol
/lamad/path:elohim-protocol/extend           # Create my own extension
/lamad/extension:ext-sarah-deep-dive         # View Sarah's extension
/lamad/extension:ext-sarah-deep-dive/fork    # Fork Sarah's approach
```

**Behavior Contract:**

When viewing a path with extensions enabled, the system overlays the learner's extension (if any) on top of the base path. Inserted steps appear at their designated positions. Annotations appear alongside the original step content. Reorderings affect step sequence. Exclusions hide steps from view.

Extensions are version-pinned to specific path versions. If the base path updates and the extension no longer applies cleanly, the system shows warnings about conflicts. The learner can update their extension to match the new version or continue with the old version.

Shared extensions appear in the extension catalog for each path. Other learners can browse extensions, see usage statistics, and fork extensions they find valuable. Extension authors can propose their changes for upstream merge, where path maintainers review and potentially incorporate community contributions.

**Collaborative Path Routes:**
```
/lamad/path:{pathId}/collaborate             # View collaboration settings
/lamad/path:{pathId}/proposals               # View pending proposals
/lamad/path:{pathId}/propose                 # Submit a proposal
/lamad/proposal:{proposalId}                 # View specific proposal
/lamad/proposal:{proposalId}/vote            # Vote on proposal
```

Collaborative paths have multiple authors with role-based permissions. Proposals go through review before being incorporated. This enables team-created training materials, community-curated paths, and mentor-mentee co-creation.

---

## Part 2: Data Models and Contracts

The data models define the shape of entities in the system. These models are designed to migrate cleanly from relational databases in the prototype to Holochain entries in production without requiring restructuring.

### 2.1 LearningPath Entity

The path is a first-class entity that sequences Territory resources with added narrative meaning. Paths are immutable once published, with edits creating new versions, which enables lineage tracking and prevents breaking experiences for learners who are midway through a path.

```typescript
interface LearningPath {
  // Identity and versioning
  id: string;                    // Opaque identifier (slug in prototype, hash in production)
  version: string;                // Semantic versioning (e.g., "1.2.0")
  
  // Descriptive metadata
  title: string;                  // Human-readable name
  description: string;            // Full description of what this path teaches
  purpose: string;                // Why this path exists - the learning goal
  
  // Authorship and lineage
  createdBy: string;              // Agent identifier of the creator
  contributors: string[];         // Agent identifiers of those who have contributed
  forkedFrom?: string;            // Parent path ID if this is a fork (creates lineage)
  createdAt: Date;                // Timestamp of initial creation
  updatedAt: Date;                // Timestamp of last modification
  
  // The journey structure - this is the heart of the path
  steps: PathStep[];              // Ordered sequence defining the learning journey
  
  // Classification and discovery
  tags: string[];                 // Searchable tags for categorization
  difficulty: 'beginner' | 'intermediate' | 'advanced';
  estimatedDuration: string;      // Human-readable like "2 weeks" or "6 months"
  
  // Access control
  visibility: 'public' | 'organization' | 'private';
  
  // Prerequisites and outcomes
  prerequisitePaths?: string[];   // Other paths that should be completed first
  attestationsGranted?: string[]; // Attestations earned upon path completion
}
```

**Implementation Notes:**

The id field is treated as opaque by all code. In the prototype, it might be a human-readable slug like "gottman-love-map" for developer convenience. In production Holochain deployment, it will be a content-addressed action hash. The routing and service layer code should never parse or depend on the format of this identifier.

The version field uses semantic versioning to track path evolution. When a published path is edited, the edit creates a new version (incrementing the appropriate number based on the scope of changes). Learners following version 1.0 can choose to upgrade to version 1.1, or they can continue with the version they started. This prevents breaking changes for learners midway through.

The steps array defines the journey. Order matters. The system renders steps in array order, and the stepIndex in URLs refers to array position. Steps cannot be skipped unless marked as optional, ensuring learners follow the intended sequence.

The prerequisitePaths array creates dependencies between paths. A learner cannot start a path if they have not completed its prerequisites. This allows building progressive learning sequences where advanced paths build on foundations laid by earlier paths. The system can use this information to recommend sensible learning sequences.

The attestationsGranted array specifies what credentials a learner earns by completing this entire path. Individual steps might grant attestations too (specified in the PathStep), but this field describes path-level attestations that recognize completion of the entire journey.

### 2.2 PathStep Structure

Each step in a path adds context to a Territory resource, explaining why this particular resource matters at this point in the learner's journey.

```typescript
interface PathStep {
  // Position in sequence
  order: number;                  // Zero-based index defining step position
  
  // Reference to Territory
  resourceId: string;             // Identifier of the ContentNode this step uses
  
  // The context overlay - this is what makes a step more than just a resource reference
  stepTitle: string;              // How this step is framed in THIS path
  stepNarrative: string;          // Why this resource matters HERE, in this journey
  
  // Learning guidance specific to this step in this path
  learningObjectives: string[];   // What the learner should achieve at this step
  reflectionPrompts?: string[];   // Questions to deepen understanding
  practiceExercises?: string[];   // Optional hands-on activities
  
  // Metadata
  estimatedTime?: string;         // Expected time like "15 minutes" or "1 hour"
  optional: boolean;              // Can this step be skipped?
  
  // Alternative resources
  alternativeResourceIds?: string[];  // Other resources serving the same purpose
  
  // Completion and attestation
  completionCriteria: string[];   // How to determine if step is complete
  attestationRequired?: string;   // Must possess this attestation to view step
  attestationGranted?: string;    // Earn this attestation upon step completion
}
```

**Implementation Notes:**

The order field must match the step's position in the path's steps array. The first step has order zero. This redundancy helps catch bugs where array indices and order fields get out of sync.

The resourceId points to a ContentNode in the Territory. This is just a reference. The step does not contain the resource content itself. Multiple steps across multiple paths can reference the same resourceId, each with different narratives and objectives. This enables content reuse without duplication.

The stepTitle and stepNarrative are what distinguish a step from a mere resource reference. The title might be completely different from the resource's own title. For example, a resource titled "Active Listening Techniques" might have a step title of "Hearing Your Partner's Dreams" in a marriage path, but "Understanding Customer Needs" in a sales training path. The narrative explains why this particular resource matters at this point in this specific journey.

The learningObjectives are step-specific and path-specific, not generic properties of the resource. The same resource might have different learning objectives depending on the path context. In a game theory course, the Evolution of Trust simulation might have objectives about understanding Nash equilibria. In a conflict resolution path, the objectives might focus on recognizing cooperation patterns in relationships.

The alternativeResourceIds allow offering different resources that serve the same pedagogical purpose. A step might offer both a text article and a video lecture covering the same material, allowing learners to choose based on their preference or accessibility needs. The path narrative remains the same regardless of which alternative is chosen.

The attestationRequired field creates access control. If a step requires an attestation that the learner has not earned, the system shows a locked state with information about how to earn the required attestation. This creates natural progression gates that prevent learners from jumping to advanced material before building necessary foundations.

The attestationGranted field allows steps to grant attestations upon completion. This might be automatic (completing the step grants the attestation) or conditional (passing a quiz embedded in the step grants the attestation). Attestations granted at the step level are typically more granular than path-level attestations, representing mastery of specific concepts rather than completion of an entire journey.

### 2.3 ContentNode Entity

The content node is the atomic unit of knowledge in the Territory. It is content-agnostic, meaning the system can render any type of educational material without hardcoding specific content types.

```typescript
interface ContentNode {
  // Identity
  id: string;                     // Opaque identifier (slug in prototype, hash in production)
  
  // Core descriptive metadata
  title: string;                  // Generic title (not path-specific)
  description: string;            // Generic description (not path-specific)
  
  // Content classification for rendering
  contentType: string;            // Domain classification: 'simulation', 'video', 'book-chapter'
  contentFormat: string;          // Technical format: 'html5-app', 'mp4', 'epub', 'markdown'
  
  // The content payload - interpretation depends on contentFormat
  content: string | object;       // Could be markdown text, URL, IPFS hash, or JSON config
  
  // Graph relationships
  tags: string[];                 // Searchable tags for categorization
  relatedNodeIds: string[];       // Simple bidirectional relationships
  
  // Rendering hints and configuration (extensible metadata)
  metadata: {
    // How the renderer should embed or display content
    embedStrategy?: 'iframe' | 'native' | 'web-component';
    
    // Required browser capabilities
    requiredCapabilities?: string[];  // e.g., ['webgl', 'webxr', 'audio']
    
    // Security policy for iframe embedding
    securityPolicy?: {
      sandbox?: string[];         // Iframe sandbox attributes
      csp?: string;               // Content Security Policy header
    };
    
    // Generic descriptive metadata
    author?: string;
    source?: string;
    license?: string;
    estimatedTime?: string;
    
    // Fully extensible for domain-specific needs
    [key: string]: any;
  };
  
  // Timestamps
  createdAt?: Date;
  updatedAt?: Date;
}
```

**Implementation Notes:**

The id field follows the same opaqueness principle as LearningPath. Never parse it or depend on its format.

The contentType is a domain classification describing what kind of educational resource this is from a pedagogical perspective. Examples include "simulation", "lecture", "book-chapter", "exercise", "quiz", "discussion-prompt", "case-study". This helps path creators find appropriate resources and helps learners understand what kind of engagement is expected.

The contentFormat is a technical classification describing how to render this resource. Examples include "markdown", "html5-app", "video", "audio", "epub", "vr-scene", "model-3d". The renderer registry uses this field to determine which rendering component should handle the content.

The content field's type is "string or object" because its interpretation depends entirely on the contentFormat. For markdown content, it is a string containing the markdown text. For video content, it might be a URL to the video file or an IPFS hash. For html5-app content, it is a URL to load in an iframe. For quiz content, it might be a JSON object describing the questions and correct answers. Renderers are responsible for interpreting this field appropriately.

The metadata object is fully extensible. The fields listed in the type definition are commonly used fields, but domain-specific renderers can add their own metadata fields as needed. For example, a VR renderer might look for metadata.vrSettings containing initialization parameters. A quiz renderer might look for metadata.passingScore. The core system does not enforce any particular metadata schema beyond the common fields shown.

The embedStrategy hint tells renderers how to display the content. The "iframe" strategy creates an isolated iframe element for security. The "native" strategy uses browser-native elements like video or audio tags. The "web-component" strategy loads a custom element. Renderers can ignore this hint if they have their own preferred strategy.

The requiredCapabilities array allows content to declare what browser features it needs. If a learner's browser lacks required capabilities, the system can show a warning or offer alternative resources. For example, VR content requiring WebXR can check if the browser supports it and offer a 2D video fallback if not.

### 2.4 Agent Entity

Agents are travelers in the system - human users, organizations, or AI entities that create paths, follow paths, and earn attestations.

```typescript
interface Agent {
  // Identity
  id: string;                     // Opaque identifier (becomes AgentPubKey in Holochain)
  displayName: string;            // Public name shown to other users
  type: 'human' | 'organization' | 'ai-agent';
  
  // Public profile information
  bio?: string;
  avatar?: string;                // URL or IPFS hash to profile image
  
  // For relationship-based agents (learning together as a unit)
  represents?: {
    type: 'marital' | 'parent-child' | 'mentor-mentee' | 'team';
    members: string[];            // Agent IDs of individuals in this relationship
  };
  
  // Path involvement (these are references, not embedded data)
  pathsCreated: string[];         // Path IDs authored by this agent
  pathsFollowing: string[];       // Path IDs currently in progress
  pathsCompleted: string[];       // Path IDs finished
  
  // Attestations earned
  attestations: string[];         // Attestation IDs proving competencies
  
  // Privacy settings
  profileVisibility: 'public' | 'organization' | 'private';
  
  // Timestamps
  createdAt: Date;
  lastActiveAt: Date;
}
```

**Implementation Notes:**

The id field will become an AgentPubKey in Holochain deployment. This is a cryptographic public key that uniquely identifies the agent and allows verifying their signatures on entries they create. In the prototype, it can be a database primary key or UUID.

The type field distinguishes between human learners, organizational entities (like a school or company), and AI agents. Different types might have different capabilities. For example, AI agents might have different rate limits on graph queries, or organizations might be able to create organization-scoped paths.

The represents field handles an interesting use case where learning happens in relational contexts. A married couple might create a joint agent that represents their relationship and tracks their progress through a marriage enrichment path together. A parent-child dyad might track their progress through parenting education. The members array lists the individual agent IDs that comprise this relationship agent.

The pathsCreated, pathsFollowing, and pathsCompleted arrays store references to paths, not the full path data. This keeps the agent entity lightweight. To get details about a path, the system must fetch it separately. This design makes it easy to migrate to Holochain where these would be links from the agent entry to path entries.

The attestations array lists attestation IDs the agent has earned. Each attestation can be verified independently by checking the attestation entry that proves the agent met the earning criteria. In Holochain, this would be a cryptographically verifiable chain of evidence.

The profileVisibility controls what information is shown in the public agent profile route. A "private" visibility means other users cannot see this agent's profile at all. An "organization" visibility means only members of the same organization can see the profile. A "public" visibility makes the profile discoverable to anyone.

### 2.5 AgentProgress Entity

Progress tracking records a traveler's journey through a specific path. This data lives on the agent's private source chain in Holochain, giving them full sovereignty over their learning data.

```typescript
interface AgentProgress {
  // Identity - which agent on which path
  agentId: string;                // The traveler
  pathId: string;                 // The journey
  
  // Progress state
  currentStepIndex: number;       // Where they are now
  completedStepIndices: number[]; // Which steps they have finished
  
  // Timing information
  startedAt: Date;                // When they began this path
  lastActivityAt: Date;           // Most recent interaction
  completedAt?: Date;             // When they finished (if complete)
  
  // Affinity tracking - relationship strength with content
  stepAffinity: Map<number, number>;  // Step index to affinity value (0.0 to 1.0)
  
  // Personal learning artifacts
  stepNotes: Map<number, string>;     // Personal notes on each step
  reflectionResponses: Map<number, string[]>;  // Answers to reflection prompts
  
  // Attestations earned through this specific path
  attestationsEarned: string[];
}
```

**Implementation Notes:**

The agentId and pathId together form a composite key uniquely identifying this progress record. An agent can have progress records for multiple paths simultaneously, and multiple agents can have progress records for the same path.

The currentStepIndex indicates where the agent is in the path sequence. When they navigate to "/lamad/path:id/step:N" and N equals currentStepIndex, they are continuing where they left off. If N is less than currentStepIndex, they are reviewing previous material. If N is greater than currentStepIndex, the system should either prevent access (if steps are sequential) or allow jumping ahead (if the path allows non-linear progression).

The completedStepIndices array tracks which steps have been marked complete. This is separate from currentStepIndex because an agent might be at step ten but have only completed steps zero through seven if they skipped or have not finished some steps. The array allows sparse completion tracking.

The stepAffinity map provides a scalar measure of relationship strength between the agent and each step. Affinity starts at zero when a step is first viewed, increases as the agent spends time with the content, and can be explicitly incremented when the agent finds material particularly valuable. Affinity values range from zero (never seen) to one (deeply mastered). This provides a much more nuanced progress metric than binary "complete/incomplete" tracking.

The stepNotes map allows agents to record their own thoughts, questions, or insights on each step. These are completely private and never shared unless the agent explicitly chooses to publish them. This supports active learning practices like the Cornell note-taking system or personal knowledge management workflows.

The reflectionResponses map stores the agent's answers to reflection prompts provided by the path. If a step asks "How might you apply this concept in your own relationship?" the agent's answer is stored here. These responses help consolidate learning and provide evidence of engagement.

The attestationsEarned array tracks attestations granted specifically through this path. This is separate from the agent's global attestations list because it provides context about where attestations came from. An agent might have the "game-theory-basics" attestation, and this array would show they earned it by completing this specific path rather than some other route.

**Critical Holochain Note:**

In production Holochain deployment, AgentProgress entries live entirely on the agent's private source chain. They are NOT published to the DHT unless the agent explicitly chooses to share progress data. This gives agents full sovereignty over their learning data. They can prove to third parties that they completed a path (by revealing specific progress entries), but the default is private. This contrasts with traditional LMS platforms where the institution owns all learner data centrally.

### 2.6 KnowledgeMap Entity

Knowledge maps are polymorphic containers for learnable territory. The same navigation and affinity mechanics apply to all three types: domain maps (content graphs), person maps (Gottman Love Maps), and collective maps (organizational knowledge).

```typescript
interface KnowledgeMap {
  // Identity
  id: string;                     // Opaque identifier
  mapType: 'domain' | 'person' | 'collective';

  // What is being mapped
  subject: {
    type: 'content-graph' | 'agent' | 'organization';
    subjectId: string;
    subjectName: string;
  };

  // Ownership
  ownerId: string;                // Who created/owns this map
  title: string;
  description?: string;

  // Access control
  visibility: 'private' | 'mutual' | 'shared' | 'public';
  sharedWith?: string[];          // Agent IDs when visibility is 'shared'

  // The knowledge structure
  nodes: KnowledgeNode[];
  pathIds: string[];              // Paths through this map's territory

  // Overall relationship strength
  overallAffinity: number;        // 0.0 to 1.0

  // Timestamps
  createdAt: Date;
  updatedAt: Date;
}

interface KnowledgeNode {
  id: string;                     // Unique within the map
  category: string;               // Category this knowledge belongs to
  title: string;
  content: string;
  source?: KnowledgeSource;       // Where this knowledge came from
  affinity: number;               // Confidence/familiarity (0.0 to 1.0)
  lastVerified?: Date;            // When last confirmed accurate
  relatedNodeIds: string[];       // Connections within the map
  tags: string[];
}

interface KnowledgeSource {
  type: 'direct-observation' | 'conversation' | 'shared-content' | 'inference' | 'external';
  sourceId?: string;
  timestamp: Date;
  confidence: number;
}
```

**PersonKnowledgeMap - Gottman Love Maps Implementation:**

```typescript
interface PersonKnowledgeMap extends KnowledgeMap {
  mapType: 'person';

  subject: {
    type: 'agent';
    subjectId: string;            // The person being mapped
    subjectName: string;
  };

  // Relationship context
  relationshipType: 'spouse' | 'partner' | 'parent' | 'child' | 'sibling' |
                    'friend' | 'mentor' | 'mentee' | 'colleague' | 'other';

  // Consent from the subject
  subjectConsent?: {
    granted: boolean;
    scope: 'public-info' | 'shared-only' | 'full-access';
    grantedAt?: Date;
    expiresAt?: Date;
    transparencyLevel: 'none' | 'categories-only' | 'full-read' | 'collaborative';
  };

  // Gottman-inspired categories
  categories: PersonKnowledgeCategory[];

  // If subject also maps the owner
  reciprocalMapId?: string;
}

type PersonKnowledgeCategoryType =
  | 'life-history'         // Past experiences, childhood, formative events
  | 'current-stressors'    // Present challenges, worries, pressures
  | 'dreams-aspirations'   // Future hopes, goals, ambitions
  | 'values-beliefs'       // Core principles, worldview, ethics
  | 'preferences-dislikes' // Daily preferences, pet peeves, favorites
  | 'friends-family'       // Social network, important relationships
  | 'work-career'          // Professional life, skills, ambitions
  | 'health-wellbeing'     // Physical/mental health, self-care
  | 'communication-style'  // How they express and receive love/feedback
  | 'conflict-patterns'    // How they handle disagreement
  | 'love-language'        // Primary ways of giving/receiving love
  | 'custom';              // User-defined categories
```

**Implementation Notes:**

The mapType discriminator enables polymorphic behavior while maintaining a unified navigation experience. The same affinity mechanics, node visualization, and path integration work across all three map types.

For person maps, consent is crucial. Without explicit consent from the subject, the mapper is limited to publicly available information. Consent can be revoked at any time, and the subject can choose what transparency level they want into what the mapper has recorded.

The reciprocalMapId field enables mutual mapping. When both parties in a relationship map each other, the system can surface opportunities for shared understanding: "You both have high affinity for 'dreams-aspirations' but Sarah's map shows a node about your career that doesn't appear in your own reflection."

**Holochain Considerations:**

Person maps live on the mapper's private source chain by default. Only when consent is granted (and stored as a link on the DHT) can the subject access the map. Collective maps use capability tokens to manage membership and editing permissions.

### 2.7 PathExtension Entity

Path extensions allow learners to personalize curated paths without modifying the original. This enables community contribution while preserving canonical curation.

```typescript
interface PathExtension {
  // Identity
  id: string;
  basePathId: string;             // The canonical path being extended
  basePathVersion: string;        // Pinned to specific version

  // Ownership
  extendedBy: string;             // Agent who created extension
  title: string;
  description?: string;

  // The modifications
  insertions: PathStepInsertion[];
  annotations: PathStepAnnotation[];
  reorderings: PathStepReorder[];
  exclusions: PathStepExclusion[];

  // Access control
  visibility: 'private' | 'shared' | 'public';
  sharedWith?: string[];

  // Lineage
  forkedFrom?: string;            // If forked from another extension
  forks?: string[];               // Extensions forked from this one

  // Upstream contribution
  upstreamProposal?: {
    status: 'draft' | 'submitted' | 'under-review' | 'accepted' | 'rejected' | 'partial';
    submittedAt?: Date;
    response?: string;
    acceptedParts?: string[];
  };

  // Timestamps
  createdAt: Date;
  updatedAt: Date;
}

interface PathStepInsertion {
  id: string;
  afterStepIndex: number;         // Insert after this step (-1 for beginning)
  steps: PathStep[];              // The steps to insert
  rationale?: string;
  source?: {
    type: 'self' | 'ai-suggestion' | 'community' | 'instructor';
    sourceId?: string;
  };
}

interface PathStepAnnotation {
  id: string;
  stepIndex: number;              // Which step this annotates
  type: 'note' | 'question' | 'insight' | 'connection' | 'struggle' |
        'breakthrough' | 'application' | 'disagreement';
  content: string;
  additionalResources?: {
    title: string;
    url?: string;
    resourceId?: string;          // If it's a Lamad content node
  }[];
  personalDifficulty?: 'easier' | 'as-expected' | 'harder';
  actualTime?: string;            // Time actually spent vs estimated
  createdAt: Date;
}

interface PathStepReorder {
  id: string;
  fromIndex: number;
  toIndex: number;
  rationale?: string;
}

interface PathStepExclusion {
  id: string;
  stepIndex: number;
  reason: 'already-mastered' | 'not-relevant' | 'prerequisite-missing' |
          'too-advanced' | 'accessibility' | 'other';
  notes?: string;
}
```

**Implementation Notes:**

Extensions are version-pinned to prevent breaking when the base path updates. If the base path version changes, the extension may show warnings about conflicts that need resolution.

The annotation types enable rich personal learning artifacts. A 'disagreement' annotation allows respectful scholarly dissent without modifying the canonical path. A 'breakthrough' annotation captures aha moments that could help other learners.

Upstream proposals enable community contribution. When an extension author believes their modifications improve the path for everyone, they can submit a proposal to the path maintainers. Partial acceptance allows cherry-picking specific insertions or annotations.

**CollaborativePath - Multi-Author Creation:**

```typescript
interface CollaborativePath {
  pathId: string;                 // Same as LearningPath.id

  collaborationType: 'sequential' | 'parallel' | 'review-required' | 'open';

  roles: Map<string, 'owner' | 'editor' | 'suggester' | 'reviewer' | 'viewer'>;

  pendingProposals: PathProposal[];

  settings: {
    requireApproval: boolean;
    minApprovals?: number;
    approvers?: string[];
    allowAnonymousSuggestions: boolean;
    notifyOnChange: boolean;
  };
}

interface PathProposal {
  id: string;
  proposedBy: string;
  changeType: 'add-step' | 'edit-step' | 'remove-step' | 'reorder' | 'edit-metadata';
  change: {
    step?: PathStep;
    stepIndex?: number;
    newIndex?: number;
    metadata?: Record<string, unknown>;
  };
  rationale: string;
  status: 'pending' | 'approved' | 'rejected' | 'withdrawn';
  votes?: Map<string, 'approve' | 'reject' | 'abstain'>;
  comments?: { authorId: string; content: string; createdAt: Date; }[];
  createdAt: Date;
  resolvedAt?: Date;
}
```

Collaborative paths enable shared authorship. Teams can create training materials together. Communities can curate knowledge collectively. Mentors and mentees can co-create personalized learning journeys.

---

## Part 3: Service Layer Contracts

The service layer provides the business logic that sits between the routing layer and the data layer. These interfaces define the contracts that view components can depend on without worrying about implementation details like whether data comes from a database, Holochain DHT, or local storage.

### 3.1 PathService

The path service manages learning path entities and navigation through paths.

```typescript
interface PathService {
  /**
   * Retrieve a complete path by its identifier.
   * Returns the full path metadata and step sequence.
   * Does NOT load the actual content for each step (lazy loading principle).
   * 
   * @param pathId - Opaque identifier for the path
   * @returns Promise resolving to the LearningPath entity
   * @throws NotFoundError if pathId does not exist
   */
  getPath(pathId: string): Promise<LearningPath>;
  
  /**
   * Get a specific step within a path along with its referenced content.
   * This is the primary API call for rendering /lamad/path:{id}/step:{index}.
   * Returns both the step context (narrative, objectives) and the content node.
   * 
   * @param pathId - Opaque identifier for the path
   * @param stepIndex - Zero-based index of the step in the sequence
   * @returns Promise resolving to a PathStepView combining step and content
   * @throws NotFoundError if pathId does not exist
   * @throws OutOfRangeError if stepIndex is invalid for this path
   */
  getPathStep(pathId: string, stepIndex: number): Promise<PathStepView>;
  
  /**
   * List paths available to the current user.
   * Supports filtering by tags, difficulty, visibility, and other criteria.
   * Results are paginated for performance.
   * 
   * @param filters - Optional filtering criteria
   * @param pagination - Page number and size
   * @returns Promise resolving to filtered paths and pagination metadata
   */
  listPaths(
    filters?: PathFilters,
    pagination?: PaginationParams
  ): Promise<PathListResult>;
  
  /**
   * Create a new learning path.
   * Requires the caller to have path-creator attestation.
   * 
   * @param path - The path structure to create
   * @returns Promise resolving to the new path's identifier
   * @throws UnauthorizedError if caller lacks path-creator attestation
   * @throws ValidationError if path structure is invalid
   */
  createPath(path: LearningPath): Promise<string>;
  
  /**
   * Fork an existing path to create a variant owned by the current user.
   * The forked path maintains a reference to its parent for lineage tracking.
   * 
   * @param pathId - The path to fork
   * @returns Promise resolving to the forked path's identifier
   * @throws NotFoundError if pathId does not exist
   * @throws UnauthorizedError if path is private and caller lacks access
   */
  forkPath(pathId: string): Promise<string>;
  
  /**
   * Update an existing path.
   * Can only be performed by the path creator.
   * If the path is published and has learners, creates a new version.
   * 
   * @param pathId - The path to update
   * @param updates - Partial path structure with fields to change
   * @returns Promise resolving when update is complete
   * @throws NotFoundError if pathId does not exist
   * @throws UnauthorizedError if caller is not the path creator
   */
  updatePath(pathId: string, updates: Partial<LearningPath>): Promise<void>;
  
  /**
   * Delete a path.
   * Can only be performed by the path creator.
   * Cannot delete if learners are actively following the path.
   * 
   * @param pathId - The path to delete
   * @returns Promise resolving when deletion is complete
   * @throws NotFoundError if pathId does not exist
   * @throws UnauthorizedError if caller is not the path creator
   * @throws ConflictError if learners are following this path
   */
  deletePath(pathId: string): Promise<void>;
}

/**
 * View model combining path step context with content node.
 * This is what gets rendered when viewing /lamad/path:{id}/step:{index}.
 */
interface PathStepView {
  // The step structure from the path
  step: PathStep;
  
  // The actual content referenced by the step
  content: ContentNode;
  
  // Navigation context
  hasPrevious: boolean;
  hasNext: boolean;
  previousStepIndex?: number;
  nextStepIndex?: number;
  
  // Progress for authenticated user (only if user is following this path)
  isCompleted?: boolean;
  affinity?: number;
  notes?: string;
}

interface PathFilters {
  tags?: string[];
  difficulty?: string;
  visibility?: string;
  createdBy?: string;
  searchText?: string;
}

interface PaginationParams {
  page: number;
  pageSize: number;
}

interface PathListResult {
  paths: LearningPath[];
  totalCount: number;
  currentPage: number;
  totalPages: number;
}
```

**Implementation Guidance:**

The getPath method should load the path structure from storage but should NOT eagerly load all the ContentNodes referenced by the steps. That would violate lazy loading. The returned LearningPath has resourceId strings in its steps, but those are just references. The content is not fetched until getPathStep is called for a specific index.

The getPathStep method is where lazy loading happens. It fetches exactly two things: the PathStep structure for the requested index, and the ContentNode referenced by that step's resourceId. It then combines these into a PathStepView that has everything the UI needs to render the step. The method also computes navigation context (hasPrevious, hasNext) by checking if stepIndex is at the boundaries of the steps array.

If the current user is authenticated and following this path, the method also loads their progress data and includes isCompleted and affinity in the returned view. This allows the UI to show "You completed this step on [date]" or "Your affinity with this content: 73%". If the user is not following this path, these fields are undefined.

The listPaths method supports filtering and pagination because large path catalogs need efficient browsing. The filters parameter allows combining multiple criteria. For example, show all beginner-level paths tagged "vocational-training" that are publicly visible. The pagination ensures that fetching page one does not require loading all paths in the system. The result includes metadata about total count and pages so the UI can render page navigation controls.

The createPath, forkPath, updatePath, and deletePath methods all require authorization checks. The service layer verifies that the caller has appropriate permissions before performing these mutations. These methods also validate that the path structure is well-formed (steps have unique orders, resourceIds exist, etc.) before persisting.

### 3.2 ContentService

The content service manages Territory resources and graph relationships.

```typescript
interface ContentService {
  /**
   * Retrieve a content node by its identifier.
   * Used for direct resource access via /lamad/resource:{id}.
   * 
   * @param resourceId - Opaque identifier for the content
   * @returns Promise resolving to the ContentNode entity
   * @throws NotFoundError if resourceId does not exist
   */
  getContent(resourceId: string): Promise<ContentNode>;
  
  /**
   * Find which paths contain a given resource.
   * Returns paths along with the step index where this resource appears.
   * Used for the "appears in paths" back-links feature.
   * 
   * @param resourceId - The resource to search for
   * @returns Promise resolving to array of path-step pairs
   */
  getContainingPaths(resourceId: string): Promise<Array<{
    path: LearningPath;
    stepIndex: number;
  }>>;
  
  /**
   * Get resources related to a given resource via graph relationships.
   * By default returns all relationship types within one hop.
   * Can filter by specific relationship type.
   * 
   * @param resourceId - The resource to start from
   * @param relationshipType - Optional filter for specific relationship
   * @returns Promise resolving to related resources
   */
  getRelatedResources(
    resourceId: string,
    relationshipType?: string
  ): Promise<ContentNode[]>;
  
  /**
   * Search content nodes by text query.
   * Searches title, description, tags, and optionally full content text.
   * Results are ranked by relevance.
   * 
   * @param query - Search query string
   * @param options - Search options like field weights and pagination
   * @returns Promise resolving to search results with scores
   */
  searchContent(
    query: string,
    options?: SearchOptions
  ): Promise<SearchResult[]>;
  
  /**
   * Create a new content node.
   * Requires content-creator attestation or organization membership.
   * 
   * @param content - The content structure to create
   * @returns Promise resolving to the new content's identifier
   * @throws UnauthorizedError if caller lacks appropriate attestation
   * @throws ValidationError if content structure is invalid
   */
  createContent(content: ContentNode): Promise<string>;
  
  /**
   * Update an existing content node.
   * Can only be performed by the content creator or stewards.
   * 
   * @param resourceId - The content to update
   * @param updates - Partial content structure with fields to change
   * @returns Promise resolving when update is complete
   * @throws NotFoundError if resourceId does not exist
   * @throws UnauthorizedError if caller lacks update permission
   */
  updateContent(resourceId: string, updates: Partial<ContentNode>): Promise<void>;
}

interface SearchOptions {
  includeContent?: boolean;  // Search full text content, not just metadata
  maxResults?: number;
  pagination?: PaginationParams;
}

interface SearchResult {
  content: ContentNode;
  score: number;  // Relevance score for ranking
  highlights?: string[];  // Text snippets showing matches
}
```

**Implementation Guidance:**

The getContent method is straightforward. It fetches a single content node and returns it. The important thing is that it does NOT fetch related resources or containing paths unless specifically asked via the other methods. This maintains separation of concerns.

The getContainingPaths method implements the back-links feature. It searches all paths in the system to find which ones reference this resourceId in their steps array. This is potentially expensive on large datasets, so implementations should consider indexing strategies. In Holochain, this would be implemented via links from the content entry to path entries, making the query efficient.

The getRelatedResources method explores the graph starting from a resource. The relationshipType parameter allows filtering. For example, "implements" would return only concepts that this resource implements, "requires" would return prerequisites, "follows" would return suggested next content. Without a filter, it returns all relationships, which might be useful for a general "related content" section.

The searchContent method provides full-text search across the Territory. The includeContent option controls whether to search just metadata (fast) or full content text (slow but comprehensive). For large content like books, searching full text might be prohibitively expensive, so the default is metadata only. The results include relevance scores and optional highlights showing matching text snippets.

### 3.3 AgentService

The agent service manages traveler profiles, progress tracking, and attestations.

```typescript
interface AgentService {
  /**
   * Get the currently authenticated agent.
   * Used to resolve "/lamad/me/" routes to the actual agent.
   * 
   * @returns Promise resolving to the authenticated agent
   * @throws UnauthenticatedError if no agent is logged in
   */
  getCurrentAgent(): Promise<Agent>;
  
  /**
   * Get public profile of another agent.
   * Only returns information the agent has marked as public.
   * 
   * @param agentId - The agent to retrieve
   * @returns Promise resolving to the agent's public profile
   * @throws NotFoundError if agentId does not exist
   */
  getAgentProfile(agentId: string): Promise<Agent>;
  
  /**
   * Get an agent's progress on a specific path.
   * Only returns data if the agent is the current user or has shared publicly.
   * 
   * @param agentId - The agent whose progress to retrieve
   * @param pathId - The path to check progress on
   * @returns Promise resolving to progress data
   * @throws NotFoundError if agent or path does not exist
   * @throws UnauthorizedError if agent's progress is private
   */
  getAgentProgress(agentId: string, pathId: string): Promise<AgentProgress>;
  
  /**
   * Mark a step as completed.
   * Creates or updates the agent's progress record for this path.
   * May grant attestations if the step specifies attestationGranted.
   * 
   * @param pathId - The path containing the step
   * @param stepIndex - The step being completed
   * @returns Promise resolving when completion is recorded
   */
  completeStep(pathId: string, stepIndex: number): Promise<void>;
  
  /**
   * Update affinity for a step.
   * Affinity increases through viewing time, reflection, practice.
   * Delta can be positive (increase) or negative (rare, but possible).
   * 
   * @param pathId - The path containing the step
   * @param stepIndex - The step whose affinity to update
   * @param delta - Amount to change affinity (typically 0.1 to 0.3)
   * @returns Promise resolving when affinity is updated
   */
  updateAffinity(
    pathId: string,
    stepIndex: number,
    delta: number
  ): Promise<void>;
  
  /**
   * Get the agent's learning frontier.
   * Returns resources where prerequisites are met but content not yet completed.
   * This answers "What am I ready to learn next?"
   * 
   * @returns Promise resolving to frontier resources
   */
  getLearningFrontier(): Promise<ContentNode[]>;
  
  /**
   * Check if the agent has a specific attestation.
   * Used for access control and capability checking.
   * 
   * @param attestationId - The attestation to check for
   * @returns Promise resolving to true if agent has attestation
   */
  hasAttestation(attestationId: string): Promise<boolean>;
  
  /**
   * Grant an attestation to the agent.
   * Records proof of how the attestation was earned.
   * 
   * @param attestationId - The attestation to grant
   * @param earnedVia - Context of how it was earned (path, quiz, etc.)
   * @param proof - Optional evidence or signature
   * @returns Promise resolving when attestation is granted
   */
  grantAttestation(
    attestationId: string,
    earnedVia: string,
    proof?: string
  ): Promise<void>;
}
```

**Implementation Guidance:**

The getCurrentAgent method extracts the agent identifier from the authentication context (JWT token, session cookie, or similar) and fetches the corresponding Agent entity. This is how "/lamad/me/" routes are resolved. If no authentication is present, it throws UnauthenticatedError, which should trigger a redirect to login.

The getAgentProfile method enforces privacy controls. It only returns fields that the target agent has marked as publicly visible. If the profile visibility is "private" and the caller is not the agent themselves, it throws UnauthorizedError. This prevents profile enumeration attacks.

The completeStep method has several responsibilities. First, it creates or updates the AgentProgress record for this path, adding the stepIndex to completedStepIndices. Second, it checks if the step has attestationGranted specified. If so, it calls grantAttestation to issue that credential. Third, it emits an REA event recording this completion for economic accounting. Fourth, it updates the agent's learning frontier since completing a step might unlock new content.

The updateAffinity method modifies the scalar relationship between agent and content. It is called automatically when an agent spends time on a step (viewing duration increases affinity). It can also be called explicitly when an agent marks content as particularly valuable. Affinity values are clamped to the range zero to one to maintain consistency.

The getLearningFrontier method is complex. It must analyze the agent's completed steps, earned attestations, and affinity patterns, then query the graph to find resources where all prerequisites are satisfied. Prerequisites might be explicit (path prerequisite relationships, step attestation requirements) or implicit (resources are one hop from high-affinity content). The result is a ranked list of suggested next steps personalized to this agent's journey.

### 3.4 ExplorationService

The exploration service handles graph queries, pathfinding, and research operations.

```typescript
interface ExplorationService {
  /**
   * Explore the neighborhood around a resource.
   * Returns the focus resource plus neighbors within specified depth.
   * Depth greater than one requires attestations.
   * 
   * @param params - Exploration parameters
   * @returns Promise resolving to a subgraph view
   * @throws UnauthorizedError if depth exceeds permitted level
   * @throws RateLimitError if query quota exceeded
   */
  exploreNeighborhood(params: {
    focus: string;
    depth: number;
    relationshipFilter?: string;
  }): Promise<GraphView>;
  
  /**
   * Find a path through the graph between two resources.
   * Uses graph algorithms like Dijkstra or semantic similarity.
   * Requires path-creator attestation.
   * 
   * @param params - Pathfinding parameters
   * @returns Promise resolving to sequence of resource IDs forming a path
   * @throws UnauthorizedError if caller lacks path-creator attestation
   * @throws NotFoundError if no path exists between resources
   */
  findPath(params: {
    from: string;
    to: string;
    algorithm: 'shortest' | 'semantic';
  }): Promise<string[]>;
  
  /**
   * Estimate computational cost of a query before executing it.
   * Helps users understand resource requirements and avoid expensive queries.
   * 
   * @param operation - Name of the operation (exploreNeighborhood, findPath, etc.)
   * @param params - Parameters for the operation
   * @returns Promise resolving to cost estimate
   */
  estimateCost(operation: string, params: any): Promise<QueryCost>;
}

/**
 * View model for graph exploration results.
 * Represents a subgraph centered on a focus resource.
 */
interface GraphView {
  // Center of exploration
  focus: ContentNode;
  
  // Neighboring resources organized by hop distance
  neighbors: Map<number, ContentNode[]>;  // hop distance -> nodes at that distance
  
  // Relationships between nodes in the subgraph
  edges: Array<{
    source: string;
    target: string;
    relationshipType: string;
  }>;
  
  // Query metadata
  metadata: {
    nodesReturned: number;
    depthTraversed: number;
    computeTimeMs: number;
    resourceCredits: number;  // For REA accounting
  };
}

/**
 * Computational cost estimate for a query.
 */
interface QueryCost {
  estimatedNodes: number;
  estimatedTimeMs: number;
  resourceCredits: number;  // For REA accounting
  attestationRequired?: string;
  rateLimitImpact: string;  // "1 of 10 queries remaining this hour"
}
```

**Implementation Guidance:**

The exploreNeighborhood method is the core graph traversal operation. It must check that the caller has appropriate attestations for the requested depth. Depth one is allowed for all authenticated users. Depth two and higher require progressively more advanced attestations. The method also checks rate limits. If the user has exhausted their query quota for the current hour, it throws RateLimitError with information about when the limit resets.

The traversal should be breadth-first, building up the neighbors map with sets of resources at each hop distance. At depth zero is just the focus resource. At depth one are resources with direct links to the focus. At depth two are resources linked to depth-one neighbors. The edges array captures the relationships between all these nodes.

The metadata in the response makes the computational cost visible. The UI should display this information so users understand that graph queries are not free. This serves the fog-of-war principle by making exploration intentional rather than casual.

The findPath method implements graph pathfinding algorithms. The "shortest" algorithm uses Dijkstra's to find minimum hops. The "semantic" algorithm considers relationship types and might prefer a longer path that follows pedagogically meaningful connections over a shorter path through arbitrary relationships. This method requires path-creator attestation because pathfinding can be very expensive on large graphs.

The estimateCost method allows users to preview how expensive an operation will be before executing it. This is particularly important for pathfinding or deep explorations that might traverse thousands of nodes. The estimate includes how much of their rate limit quota the query will consume, allowing them to decide if they want to spend that quota now or save it.

---

## Part 4: Content Rendering Strategy

The rendering system is designed for extensibility. New content types can be added without modifying core application code. This is accomplished through the Renderer Registry pattern.

### 4.1 The Renderer Registry Pattern

The renderer registry is a singleton service that maps content formats to rendering components. When the UI needs to display a ContentNode, it queries the registry for an appropriate renderer.

```typescript
/**
 * Interface that all content renderers must implement.
 * A renderer knows how to display a specific content format.
 */
interface ContentRenderer {
  /**
   * Can this renderer handle the given content node?
   * Typically checks the contentFormat field.
   * 
   * @param node - The content to potentially render
   * @returns True if this renderer can display this content
   */
  canRender(node: ContentNode): boolean;
  
  /**
   * Render the content into the provided DOM container.
   * Should be idempotent (safe to call multiple times).
   * 
   * @param node - The content to render
   * @param container - The HTML element to render into
   * @returns Promise that resolves when rendering is complete
   */
  render(node: ContentNode, container: HTMLElement): Promise<void>;
  
  /**
   * Optional cleanup when content is no longer visible.
   * Use for stopping videos, removing event listeners, etc.
   */
  cleanup?(): void;
  
  /**
   * Optional hook called when user completes viewing.
   * Return true if auto-completion is appropriate for this content type.
   */
  onComplete?(): boolean;
}

/**
 * The renderer registry maintains the mapping from formats to renderers.
 */
class RendererRegistry {
  private renderers: ContentRenderer[] = [];
  
  /**
   * Register a renderer with the system.
   * Typically called during application initialization.
   * 
   * @param renderer - The renderer to register
   */
  register(renderer: ContentRenderer): void {
    this.renderers.push(renderer);
  }
  
  /**
   * Find a renderer capable of displaying the given content.
   * Checks renderers in registration order until one returns true from canRender.
   * 
   * @param node - The content needing rendering
   * @returns A renderer capable of displaying this content
   * @throws NoRendererError if no registered renderer can handle this content
   */
  getRenderer(node: ContentNode): ContentRenderer {
    const renderer = this.renderers.find(r => r.canRender(node));
    
    if (!renderer) {
      // Fall back to error renderer that shows "cannot display this format"
      return new FallbackRenderer();
    }
    
    return renderer;
  }
}
```

### 4.2 Built-in Renderers

The system ships with renderers for common educational content formats. These handle the majority of use cases without requiring custom extensions.

**MarkdownRenderer** handles text documents written in Markdown format. It converts the markdown syntax to HTML with syntax highlighting for code blocks and math rendering for LaTeX equations. This is the most common format for text-based educational content.

**VideoRenderer** handles video content in formats like MP4, WebM, or YouTube embeds. It uses either the browser's native video element with controls or embeds third-party players like YouTube's iframe API. The renderer respects accessibility requirements by ensuring controls are keyboard-navigable and supporting captions if provided in the content metadata.

**AudioRenderer** handles audio content like podcasts or recorded lectures. It uses the native audio element and can display custom controls with playback speed adjustment and timestamp navigation if those features are specified in metadata.

**HTML5AppRenderer** handles interactive HTML5 applications like simulations, games, or data visualizations. It creates an iframe with appropriate sandboxing for security, loads the application URL from the content field, and configures the iframe according to the metadata security policy. This is how content like Nicky Case's "Evolution of Trust" gets embedded.

**EPUBRenderer** handles e-book content in EPUB format. It integrates with a JavaScript EPUB reader library to provide a reading experience with pagination, highlighting, and note-taking. The renderer can restore the reader's position if they return to this content later.

**AttestationRenderer** handles special content nodes that grant attestations upon completion. This might be a quiz that must be passed, an exercise that must be submitted, or simply an "I attest I have completed this" button. The renderer displays the attestation challenge and communicates with the AgentService to grant the attestation when criteria are met.

### 4.3 Custom Renderers

Organizations or developers can extend the system by implementing custom renderers for specialized content types.

**Example - VR Scene Renderer:**

```typescript
class VRSceneRenderer implements ContentRenderer {
  canRender(node: ContentNode): boolean {
    return node.contentFormat === 'vr-scene' && this.hasWebXRSupport();
  }
  
  async render(node: ContentNode, container: HTMLElement): Promise<void> {
    // Check if browser supports WebXR
    if (!this.hasWebXRSupport()) {
      this.renderFallback(node, container);
      return;
    }
    
    // Create VR scene using A-Frame or Three.js
    const scene = await VRFramework.loadScene(node.content);
    scene.mount(container);
    
    // Add VR entry button
    const enterButton = document.createElement('button');
    enterButton.textContent = 'Enter VR';
    enterButton.onclick = () => scene.enterVR();
    container.appendChild(enterButton);
  }
  
  private hasWebXRSupport(): boolean {
    return 'xr' in navigator && navigator.xr !== undefined;
  }
  
  private renderFallback(node: ContentNode, container: HTMLElement): void {
    // Show 2D preview or video walkthrough if VR not available
    container.innerHTML = '<p>VR content requires WebXR support. Showing 2D preview instead.</p>';
    // Load fallback content from node.metadata.fallback
  }
  
  cleanup(): void {
    // Dispose of VR resources, exit immersive mode
  }
}

// Register during app initialization
rendererRegistry.register(new VRSceneRenderer());
```

**Example - 3D Model Viewer Renderer:**

```typescript
class ModelViewerRenderer implements ContentRenderer {
  canRender(node: ContentNode): boolean {
    return node.contentFormat === 'model-3d';
  }
  
  async render(node: ContentNode, container: HTMLElement): Promise<void> {
    // Use model-viewer web component
    const viewer = document.createElement('model-viewer');
    viewer.src = node.content;  // URL to .glb or .gltf file
    viewer.setAttribute('auto-rotate', 'true');
    viewer.setAttribute('camera-controls', 'true');
    
    // Apply metadata configuration
    if (node.metadata.modelSettings) {
      Object.entries(node.metadata.modelSettings).forEach(([key, value]) => {
        viewer.setAttribute(key, value as string);
      });
    }
    
    container.appendChild(viewer);
  }
}

rendererRegistry.register(new ModelViewerRenderer());
```

### 4.4 Integration with Views

View components that display content should use the renderer registry rather than hardcoding format-specific logic.

```typescript
// Example Angular component for displaying path steps
@Component({
  selector: 'app-step-viewer',
  template: `
    <div class="step-context">
      <h2>{{stepView.step.stepTitle}}</h2>
      <p>{{stepView.step.stepNarrative}}</p>
    </div>
    <div #contentContainer class="content-display"></div>
    <div class="step-actions">
      <button (click)="markComplete()">Complete Step</button>
    </div>
  `
})
export class StepViewerComponent implements OnInit, OnDestroy {
  @Input() stepView: PathStepView;
  @ViewChild('contentContainer') contentContainer: ElementRef;
  
  private renderer: ContentRenderer;
  
  constructor(
    private rendererRegistry: RendererRegistry
  ) {}
  
  ngOnInit(): void {
    // Get appropriate renderer for this content
    this.renderer = this.rendererRegistry.getRenderer(this.stepView.content);
    
    // Render the content
    this.renderer.render(
      this.stepView.content,
      this.contentContainer.nativeElement
    );
  }
  
  ngOnDestroy(): void {
    // Cleanup when component is destroyed
    if (this.renderer.cleanup) {
      this.renderer.cleanup();
    }
  }
}
```

This approach keeps view components format-agnostic. They do not need to know about markdown versus video versus VR content. They just ask the registry for a renderer and delegate to it. This makes the system extensible without requiring changes to view code.

---

## Part 5: Future Integration Patterns

This section describes how the prototype architecture migrates to production infrastructure. The patterns are designed to minimize disruption when transitioning from centralized to distributed deployment.

### 5.1 Holochain Deployment Strategy

The migration to Holochain happens in phases, allowing validation at each stage before proceeding to the next.

**Phase 1: Prototype with Traditional Backend**

The initial prototype uses a traditional REST API backed by PostgreSQL or similar relational database. LearningPath entities are stored as database rows with a JSON column for the steps array. ContentNode entities are database rows with the content field storing either inline markdown or URLs to external resources. AgentProgress data is stored in the database associated with user accounts.

This phase proves out the URL routing, service interfaces, and rendering system without the complexity of peer-to-peer infrastructure. It allows rapid iteration on the user experience and path authoring tools.

**Phase 2: Holochain Bridge - Hybrid Architecture**

In this phase, we introduce a Holochain DNA alongside the traditional backend. The web application can communicate with both systems. Path creation moves to Holochain while content and progress stay in the database initially.

A LearningPath becomes a Holochain entry type. When someone creates a path, the data is written to Holochain instead of the database. The entry contains all path metadata and the steps array. Each step's resourceId still points to content in the traditional database.

The web app now makes calls to both the REST API (for content) and the Holochain conductor (for paths). This validates that the data models translate correctly to Holochain entries and that the application can work with both data sources simultaneously.

**Phase 3: Content Migration to Holochain**

ContentNode entities migrate to Holochain entries. Small content like markdown documents is stored inline in the entry. Large content like videos or EPUBs is stored on IPFS with the hash recorded in the content field.

The system now has path metadata and content metadata entirely in Holochain. Only agent progress remains in the centralized database. The web app talks primarily to Holochain with fallback to the database only for progress tracking.

**Phase 4: Full Peer-to-Peer with Agent Sovereignty**

AgentProgress moves to private entries on each agent's source chain. Progress data no longer goes to any centralized server. Each agent runs a local Holochain conductor (or connects to a conductor run by their organization). The web app becomes a thin client that talks exclusively to the local conductor.

At this point, the system is fully peer-to-peer. Agents have complete data sovereignty. Organizations can run their own nodes to steward content. The network is resilient to any single node failure.

**URL Stability Across Migration:**

The URL patterns defined in Part 1 remain stable throughout this migration. The pathId parameter changes from human-readable slugs to content-addressed action hashes, but the routing logic treats it as an opaque string regardless. The same code that handles "/lamad/path:gottman-love-map/step:5" works for "/lamad/path:uhCkkXyz123/step:5" without modification.

**Data Model Mapping to Holochain:**

LearningPath becomes a Holochain entry with entry type "learning_path". The steps array is stored in the entry itself. Links of type "path_contains_step" connect the path entry to content entries, with the link tag storing the step index and narrative.

ContentNode becomes a Holochain entry with entry type "content_node". For large content, the content field stores an IPFS hash rather than inline data. The system uses Holochain's built-in DHT to distribute content metadata while heavy files stay in IPFS.

AgentProgress becomes private entries on agent source chains. An entry type "agent_progress" records progress on each path. The entry is not published to the DHT by default, keeping progress data private unless the agent explicitly chooses to share it.

Attestations become verifiable credentials stored as entries with cryptographic proofs. The entry includes the agent's public key, the attestation ID, proof of how it was earned, and a signature. Validation rules verify that attestations were legitimately earned according to the system's criteria.

### 5.2 REA Integration for Economic Accounting

REA (Resource-Event-Agent) ontology provides a framework for tracking the economic value created and exchanged in the learning ecosystem. This integration makes visible the care work that typically goes unrewarded in traditional systems.

**REA Entities in Learning Context:**

Resources in the REA model include learning content (the Territory), attention and time spent by learners, the labor of path creators and content authors, and computational resources consumed by graph queries.

Events are the economic activities that transfer or transform resources. These include viewing events when a learner engages with content, completion events when steps are finished, creation events when paths or content are authored, and query events when the graph is explored.

Agents are the economic actors. They include learners who consume content and invest time, creators who author paths and content, organizations that steward content and provide infrastructure, and AI agents that build custom paths.

**Economic Flows:**

When a learner completes a step, an economic event is recorded with resource inputs (time and attention spent) and resource outputs (knowledge gained, measured by affinity increase). This creates a record of value creation.

When a path creator authors a new path, an economic event records the resource input (creator's labor time) and resource output (the published path that others can follow). If the system has a contribution credit system, the creator might earn credits for this contribution.

When many learners complete a path successfully, the aggregate value created flows back to recognize the path creator's contribution. High-quality paths that help many people learn effectively create more value than low-quality paths with high dropout rates.

**Implementation Pattern:**

The service layer methods that modify state should emit REA events. For example, AgentService.completeStep should call an REA event logging service after updating the progress record. The event captures what happened, who did it, what resources were involved, and what value was created or transferred.

```typescript
interface REAEventService {
  logEvent(event: EconomicEvent): Promise<void>;
}

interface EconomicEvent {
  eventType: 'view' | 'complete' | 'create' | 'query';
  agent: string;  // Who performed the action
  inputResources: Array<{
    type: string;
    quantity: number;
    unit: string;
  }>;
  outputResources: Array<{
    type: string;
    quantity: number;
    unit: string;
  }>;
  timestamp: Date;
  context: any;  // Additional event-specific data
}
```

This integration is orthogonal to core functionality. The system works without REA. Adding REA means instrumenting service methods to emit events, but does not change their behavior. This makes it a feature that can be added later without breaking existing functionality.

**Future Value Flows:**

With REA event data, the system can answer questions like: How much learning value has this path created? How much labor has this creator contributed? What is the return on investment for stewarding content? These insights enable new economic models like contribution-based compensation, value-based pricing, or recognition systems that reward effective teaching.

### 5.3 AI Agent Path Customization

AI agents have the potential to democratize path creation, making personalized learning journeys available to everyone rather than just those who can afford human tutors or instructional designers.

**Agent Autonomy with Constraints:**

Elohim AI agents can traverse the knowledge graph and generate custom paths, but they face the same computational constraints as human researchers. They must declare their purpose before traversing. They must acknowledge estimated cost before executing queries. They are rate-limited to prevent resource abuse. All their actions are logged to public audit trails.

These constraints serve multiple purposes. They prevent runaway AI agents from consuming excessive resources. They force intentionality into AI exploration, ensuring that even superintelligent agents must have reasons for their actions. They create accountability through public logging.

**Path Generation API:**

```typescript
interface AIPathBuilder {
  /**
   * Generate a custom path from A to B with narrative context.
   * The AI agent analyzes the graph to find an effective route,
   * then generates step narratives explaining why each resource matters.
   * 
   * @param params - Path generation parameters
   * @returns Promise resolving to a new learning path
   */
  generatePath(params: {
    from: string;            // Starting resource (what learner already knows)
    to: string;              // Target resource (what learner wants to learn)
    purpose: string;         // Why this path is needed
    forAgent?: string;       // Personalize for specific agent's background
    maxSteps?: number;       // Limit path length
    difficulty?: string;     // Target difficulty level
    excludeNodes?: string[]; // Resources to avoid
  }): Promise<LearningPath>;
  
  /**
   * Suggest next best step for an agent on their current path.
   * Takes into account progress, affinity, and attestations.
   * 
   * @param agentId - The learner to make suggestions for
   * @param pathId - The path they are currently following
   * @returns Promise resolving to suggested next step
   */
  suggestNextStep(
    agentId: string,
    pathId: string
  ): Promise<PathStep>;
  
  /**
   * Create personalized variant of an existing path.
   * Adapts narratives and pacing for the agent's specific context.
   * 
   * @param pathId - Path to personalize
   * @param forAgent - Agent to personalize for
   * @returns Promise resolving to customized path
   */
  personalizePath(
    pathId: string,
    forAgent: string
  ): Promise<LearningPath>;
}
```

**Example Usage Scenario:**

A learner says "I want to learn Holochain, but I am coming from SQL databases and do not know much about distributed systems." The AI agent analyzes the learner's profile, sees their high affinity with SQL concepts, identifies the target (Holochain fundamentals), and generates a custom bridge path.

The path might include steps like "From Tables to Entries: How Holochain Stores Data" with a narrative explaining how DHT entries are analogous to database rows but content-addressed. Another step might be "From Foreign Keys to Links: Relationships in Holochain" explaining how Holochain links serve similar purposes to SQL foreign keys but with different semantics.

Each step's narrative is generated to leverage the learner's existing mental models while building toward the target knowledge. The AI cannot just say "here are ten Holochain tutorials" because that ignores the learner's context. It must create a pedagogically sound sequence with narratives that scaffold understanding.

**Implementation Approach:**

The AI path builder is a separate service that calls ExplorationService to query the graph and PathService to create paths. It uses graph algorithms to find potential routes, then applies language models to generate step narratives. It respects the same rate limits and attestation requirements as human researchers.

Generated paths are stored like any other path and can be forked, edited, or extended by humans. This creates a collaboration between AI efficiency (quickly finding routes through large knowledge graphs) and human insight (adding emotional intelligence and cultural context to narratives).

### 5.4 Attestation System Architecture

Attestations are cryptographic proofs of capacity. They unlock content, enable capabilities, and build verifiable credentials.

**Attestation Types:**

Educational attestations are earned by passing assessments that validate understanding. These might be quizzes, essays, or projects demonstrating mastery of concepts.

Skill-based attestations are earned by demonstrating practical application. These might be coding challenges, case study analyses, or portfolio reviews.

Relational attestations are earned through interpersonal engagement. These might be peer endorsements, mentor certifications, or community participation.

Civic attestations are earned by contributing to the commons. These might be creating paths, stewarding content, or helping other learners.

Time-based attestations require sustained engagement over extended periods. These recognize that some capacities (wisdom, maturity, persistence) cannot be rushed.

**Attestation Data Model:**

```typescript
interface Attestation {
  id: string;
  title: string;
  description: string;
  
  // Earning criteria
  requirementType: 'quiz-pass' | 'project-completion' | 'peer-review' | 'time-based' | 'contribution';
  requirementConfig: any;  // Type-specific configuration
  
  // What this attestation unlocks
  grantsCapabilities: string[];  // Like 'graph-researcher', 'path-creator', 'advanced-query'
  grantsAccessTo?: string[];     // Resource IDs requiring this attestation
  
  // Verification approach
  verificationType: 'self-reported' | 'automated' | 'peer-reviewed' | 'steward-signed';
  
  // Revocation mechanics (some attestations can be revoked if trust is violated)
  canBeRevoked: boolean;
  revokedBy?: string[];  // Agent IDs of stewards who can revoke
}

interface AgentAttestation {
  agentId: string;
  attestationId: string;
  earnedAt: Date;
  earnedVia: string;  // Path or event that granted this
  proof?: string;     // Cryptographic proof or evidence
  revokedAt?: Date;
  revocationReason?: string;
}
```

**Integration with Path System:**

Path steps can specify attestationRequired to gate content. When a learner tries to view that step, the system checks if they have the required attestation. If not, it shows a locked state with explanation of how to earn it. This creates natural prerequisites.

Path steps can specify attestationGranted to reward completion. When the learner completes the step and meets criteria (might be automatic or might require passing a test), the system grants the attestation. This creates progression.

Paths themselves can list prerequisitePaths that grant required attestations, creating a dependency graph. To access an advanced path, you must first complete a foundational path that grants the necessary credentials.

**Implementation Pattern:**

Attestation checking happens in AgentService before returning content. If required attestations are missing, the response indicates locked status with information about earning paths.

Attestation granting happens after completion validation. If a step grants an attestation and the learner meets criteria, AgentService.grantAttestation is called with proof of completion.

In Holochain deployment, attestations are verifiable entries with cryptographic signatures. Network validation rules check that attestations were legitimately earned. This prevents false claims of capacity.

**Revocation Mechanics:**

Some attestations can be revoked if the holder violates trust. For example, a "graph-researcher" attestation might be revoked if the holder abuses query capabilities. The attestation specifies who can revoke (usually content stewards or community governance). Revocation creates an entry with timestamp and reason, preserving an audit trail.

---

## Part 6: Developer Guidelines and Best Practices

### 6.1 Working with This Specification

**For Frontend Developers:**

Implement view components according to the URL contracts in Part 1. Never hardcode navigation patterns that bypass these routes. Use the PathStepView and GraphView models as the data contracts between services and views.

Treat pathId and resourceId as opaque strings. Never parse them to extract information. Never construct them by concatenating strings. Always get them from API responses or route parameters.

Use the RendererRegistry for all content display. Never hardcode "if contentFormat equals markdown then do X" logic in views. Register renderers during initialization and delegate to them.

Keep navigation within path context. When displaying a path step, Previous and Next buttons should navigate within that path. Do not jump to unrelated content without explicit user action.

**For Backend Developers:**

Implement service interfaces exactly as specified in Part 3. Method signatures are contracts. Parameter types and return types must match.

Enforce authorization and rate limits in service methods, not in controllers or routes. Every method that modifies data or performs expensive queries should check permissions and quotas before proceeding.

Design database schemas that map cleanly to Holochain entry types. Use JSON columns for nested structures that will become embedded objects in entries. Avoid normalizations that will complicate migration.

Emit REA events from state-modifying methods. This can be a no-op in prototype but the hooks should be present for future activation.

**For AI Assistants (Claude, Gemini):**

Reference this specification when implementing features. Do not invent new URL patterns or data models. If the spec does not cover a use case, note that as a gap rather than improvising.

When generating code, add comments referencing the relevant section of this spec. For example, "// Implements PathService.getPathStep as defined in section 3.1".

Maintain consistency across multiple implementation sessions. This spec is the source of truth that ensures different features integrate correctly.

If requirements conflict with this spec, ask for clarification about which takes precedence rather than making assumptions.

**For Path Creators:**

Understand that paths add meaning to content. You are not creating resources. You are creating journeys through existing resources.

Write step narratives that explain why this resource matters at this point. Do not just describe what the resource contains. Explain how it fits the learner's progression.

Specify clear learning objectives and completion criteria. Learners should know what they are supposed to achieve at each step and how to know when they are done.

Consider alternative resources for accessibility and learning style preferences. Some learners prefer reading, others prefer watching, others prefer doing. Offer choices when possible.

Think about what attestations learners should earn. Granular attestations at the step level create progression. Path-level attestations recognize journey completion.

### 6.2 Testing Strategies

**Unit Tests:**

Test each service method in isolation with mocked dependencies. Verify that PathService.getPathStep returns the correct structure when given a valid pathId and stepIndex. Verify that it throws the expected error when stepIndex is out of range.

Test that AgentService.completeStep updates the progress record correctly. Verify that it grants attestations when the step specifies attestationGranted. Verify that it respects authorization (cannot complete steps for other agents).

Test renderer canRender logic. Verify that each renderer correctly identifies the content it can handle. Verify that render methods produce expected DOM structures.

**Integration Tests:**

Test the full request cycle from URL to rendered view. Set up test data with a known path and content. Navigate to "/lamad/path:test-path/step:0". Verify that the view displays the step narrative, renders the content using the appropriate renderer, and shows navigation buttons.

Test authentication flows. Navigate to "/lamad/me/paths/following" without authentication. Verify redirect to login. Authenticate and verify the page loads with the agent's paths.

Test authorization. Attempt to access content requiring an attestation without having that attestation. Verify locked state display. Grant the attestation and verify content becomes accessible.

**End-to-End Tests:**

Test complete user journeys from landing page to completion. A new learner arrives at the site, browses featured paths, chooses a path, starts following it, completes several steps, earns an attestation, and reaches their learning frontier.

Test path creation workflows. A path creator searches for resources, drags them into sequence, adds narratives to each step, publishes the path, and sees it appear in the path catalog.

Test forking and remixing. A user finds a path they like but wants to modify for their context. They fork it, edit some step narratives, publish their variant, and share it with others.

**Performance Tests:**

Verify lazy loading works correctly. Navigate to a path with fifty steps. Verify that only step zero content loads initially. Navigate to step one and verify step two and three are prefetched but step four through fifty are not loaded.

Test rate limiting. Make eleven graph exploration queries in rapid succession. Verify that the eleventh is rejected with appropriate error. Wait for rate limit reset and verify queries work again.

Test large path performance. Create a path with one hundred steps. Verify that getPath returns quickly without loading all content. Verify that navigating between steps is responsive.

**Holochain Integration Tests (Future):**

Test that path creation writes correct entry structures. Verify entry types, linked entries, and metadata.

Test that progress tracking stays on agent's private chain. Verify that another agent cannot access this progress without explicit sharing.

Test that attestation validation rules work correctly. Attempt to create an attestation entry without meeting earning criteria. Verify that validation rejects it.

### 6.3 Error Handling Standards

**Client Errors (4xx Status Codes):**

Return 400 Bad Request when pathId or resourceId format is invalid. Include a message explaining what format is expected.

Return 401 Unauthorized when accessing "/lamad/me/" routes without authentication. Include redirect URL to login page.

Return 403 Forbidden when attempting operations without required attestations. Include explanation of which attestation is needed and how to earn it.

Return 404 Not Found when pathId or resourceId does not exist in the system. Include suggestion to search for similar content.

Return 429 Too Many Requests when rate limits are exceeded. Include information about when the limit resets and current quota usage.

**Server Errors (5xx Status Codes):**

Return 500 Internal Server Error for unexpected failures with generic message. Log detailed error information server-side.

Return 503 Service Unavailable if backend services (database, Holochain conductor) are not responding. Include estimated time until service is restored.

**Error Response Format:**

All errors should return a consistent JSON structure that frontend code can parse and display appropriately.

```typescript
interface ErrorResponse {
  error: {
    code: string;        // Machine-readable error code like "RATE_LIMIT_EXCEEDED"
    message: string;     // Human-readable explanation
    details?: any;       // Additional context (which rate limit, how many requests, etc.)
    retryAfter?: number; // Seconds until operation can be retried
    helpUrl?: string;    // Link to documentation explaining this error
  };
}
```

Example error responses:

```json
{
  "error": {
    "code": "ATTESTATION_REQUIRED",
    "message": "This step requires the 'graph-researcher' attestation",
    "details": {
      "requiredAttestation": "graph-researcher",
      "earningPaths": [
        "path:research-fundamentals",
        "path:graph-theory-basics"
      ]
    },
    "helpUrl": "https://docs.lamad.org/attestations"
  }
}
```

```json
{
  "error": {
    "code": "RATE_LIMIT_EXCEEDED",
    "message": "You have exceeded your query quota for this hour",
    "details": {
      "limit": 10,
      "used": 11,
      "resetsAt": "2024-11-24T15:00:00Z"
    },
    "retryAfter": 1800
  }
}
```

### 6.4 Performance Requirements

**Response Time Targets:**

Path step loading (PathService.getPathStep) should complete in under two hundred milliseconds for the 95th percentile. This is the most common operation and must be fast.

Resource direct access (ContentService.getContent) should complete in under one hundred milliseconds for the 95th percentile. These are simple lookups.

Graph exploration queries (ExplorationService.exploreNeighborhood with depth one) should complete in under two seconds for the 95th percentile. Deeper explorations may take longer.

Path search and discovery (PathService.listPaths) should complete in under five hundred milliseconds for the 95th percentile even with filters and pagination.

**Lazy Loading Constraints:**

Never preload content beyond the current step plus two in either direction. This is not negotiable even if bandwidth and memory are plentiful. The fog-of-war principle is architectural, not just optimization.

Never load full path structure (all step content) when displaying path overview. Only load metadata and step titles until user navigates to specific steps.

Never load entire graph when exploring neighborhoods. Only load nodes within specified depth from the focus resource.

Paginate large result sets. Path lists, search results, and agent profiles should return maximum fifty items per page. Provide pagination controls to access additional pages.

**Caching Strategy:**

Cache path structures aggressively. They change infrequently (only when creator publishes new version). Use version field in cache key to invalidate on updates.

Cache content nodes aggressively. They are immutable once created. Use content-addressed identifiers as cache keys. Cache can be indefinite since content cannot change.

Do not cache agent progress. It changes frequently and must always be fresh. Every step completion or affinity update should be immediately visible.

Do not cache authentication status. Session tokens can be revoked. Check authentication on every request to protected resources.

Invalidate caches when new path versions are published. Learners following old versions should continue seeing cached old version. New learners should see cached new version. Cache key includes version identifier.

**Database Query Optimization:**

Index pathId and resourceId columns for fast lookups. These are the primary access patterns.

Index agent progress by (agentId, pathId) composite key for fast progress retrieval.

Index content nodes by tags for efficient search and filtering.

Avoid N+1 query problems. When loading multiple paths, batch-fetch their metadata rather than querying for each individually.

Use database connection pooling to handle concurrent requests efficiently. Do not create new database connection for each request.

---

## Appendix: Migration Roadmap

This section provides a timeline for implementing the specification from prototype to production deployment.

### Prototype Phase (Weeks 1-4)

**Goals:** Validate core concepts with minimal viable implementation. Prove that path-centric navigation works and that content rendering is extensible.

**Implementation Tasks:**

Week 1: Implement LearningPath and ContentNode data models. Set up PostgreSQL schema or use in-memory storage for fast iteration. Create sample data including the default Elohim Protocol path with manifesto as step zero.

Week 2: Implement core routing for "/lamad/path:{id}/step:{index}" and "/lamad/resource:{id}". Build PathService and ContentService with basic implementations. Set up Angular components for step viewer and resource viewer.

Week 3: Implement renderer registry with markdown, video, and HTML5-app renderers. Integrate renderers into view components. Test with varied content types including text documents and embedded simulations.

Week 4: Deploy to single-server development environment. Conduct internal testing with small team. Gather feedback on navigation patterns, content rendering quality, and overall user experience.

**Success Criteria:** Users can navigate through a multi-step path, see step-specific narratives, and view content rendered appropriately. Content can be accessed directly via resource URLs. No crashes or data loss during normal use.

### Beta Phase (Weeks 5-12)

**Goals:** Add user accounts, progress tracking, and basic path creation tools. Validate that the system works with real learners and real content.

**Implementation Tasks:**

Week 5-6: Implement authentication and AgentService. Add user registration and login. Implement "/lamad/me/" routes for personal progress tracking. Store agent progress in database with privacy controls.

Week 7-8: Implement path creation UI. Allow authenticated users to create new paths by searching for resources and sequencing them. Add step narrative editor. Allow publishing paths for others to follow.

Week 9-10: Implement basic attestation system with quiz-pass attestations. Create quiz content type and renderer. Integrate attestation checks into step access control. Test that locked content becomes accessible after earning attestations.

Week 11: Implement graph exploration with depth-one queries. Add "/lamad/explore" route with simple graph visualization. Implement rate limiting for queries.

Week 12: Deploy to staging environment accessible to beta testers. Recruit pilot user group of ten to fifty learners and path creators. Provide support during beta period and collect detailed feedback.

**Success Criteria:** Beta users successfully create accounts, follow paths, track progress, earn attestations, and create new paths. No critical bugs. Performance meets targets under beta load. User feedback is positive enough to warrant continued development.

### Production Phase (Weeks 13-24)

**Goals:** Scale to hundreds of users. Implement full attestation system. Add REA event logging. Optimize performance and add monitoring.

**Implementation Tasks:**

Week 13-14: Implement full attestation system with multiple verification types (automated, peer-reviewed, steward-signed). Add attestation wallet UI showing earned credentials and unlocked capabilities.

Week 15-16: Implement REA event logging service. Instrument service methods to emit economic events. Create analytics views showing value creation flows.

Week 17-18: Performance optimization. Add database indexes, implement caching layer, optimize queries based on profiling data. Set up CDN for static assets.

Week 19-20: Implement comprehensive monitoring and logging. Set up alerts for errors, slow queries, and rate limit violations. Create operational dashboards for system health.

Week 21-22: Security hardening. Conduct security review, implement additional input validation, add CSRF protection, set up rate limiting at API gateway level.

Week 23-24: Deploy to production environment with auto-scaling infrastructure. Migrate beta users to production. Begin onboarding wider audience. Establish support processes.

**Success Criteria:** System handles hundreds of concurrent users without performance degradation. No security vulnerabilities discovered. Monitoring catches issues before users report them. Support team can handle user questions and issues efficiently.

### Holochain Migration (Months 7-12)

**Goals:** Migrate from centralized infrastructure to peer-to-peer Holochain deployment. Achieve data sovereignty for users and content stewards.

**Implementation Tasks:**

Month 7: Design Holochain DNA with entry types matching current data models (LearningPath, ContentNode, AgentProgress, Attestation). Write validation rules for each entry type.

Month 8: Implement bridge layer allowing web app to communicate with both REST API and Holochain conductor. Test dual-source architecture with sample data.

Month 9: Migrate path creation to Holochain. New paths write to DHT instead of database. Existing paths remain in database for stability. Web app can read from both sources transparently.

Month 10: Migrate content metadata to Holochain. Large content files move to IPFS with hashes stored in content entries. Test content rendering from IPFS sources.

Month 11: Migrate agent progress to source chains. Each user runs local conductor or connects to organizational conductor. Progress entries stay private by default.

Month 12: Decommission REST API. Web app talks exclusively to Holochain conductors. Organizations run their own nodes as content stewards. System is fully peer-to-peer.

**Success Criteria:** Users can run local conductors and maintain data sovereignty. Content creators can steward their own content nodes. System continues functioning without centralized servers. Network is resilient to node failures.

---

## Part 4: Bidirectional Trust Model (Content Attestations)

The previous sections describe **Agent Attestations** - credentials earned by learners that unlock access to gated content. This section introduces the symmetric counterpart: **Content Attestations** - trust credentials earned by content that unlock broader reach.

### 4.1 The Symmetry Principle

In a system with "boundaries around freedom of reach," accountability must be symmetric:

```
Agent (Traveler)              Content (Territory)
─────────────────             ─────────────────
Earns attestations            Earns attestations
to ACCESS content             to REACH audiences

private → public              private → commons
visibility earned             reach earned
```

**Why Content Needs Attestations:**

Traditional platforms default to public visibility: create content, it's instantly available to everyone. This creates problems at scale:
- Misinformation spreads before verification
- Harmful content reaches vulnerable audiences
- Quantity overwhelms quality
- No mechanism for community trust signals

Lamad inverts this: content starts **private** and earns **reach** through attestation. Just as an agent earns the attestation "4th grade math mastery" to access advanced content, content earns the attestation "steward-approved" to reach community members.

### 4.2 Content Reach Levels

Content reach mirrors agent visibility, creating symmetric trust boundaries:

| Reach Level | Who Can See | How Earned |
|-------------|-------------|------------|
| `private` | Only author | Default for new content |
| `invited` | Specific agents | Author grants explicit access |
| `local` | Author's connections | Author-verified attestation |
| `community` | Community members | Steward-approved or community-endorsed |
| `federated` | Multiple communities | Peer-reviewed or governance-ratified |
| `commons` | All agents (public) | Governance-ratified + safety-reviewed + license-cleared |

**Progression Example:**

A new video tutorial starts at `private`. The author can share it with specific colleagues (`invited`). After refinement, their network sees it (`local`). A domain steward reviews and approves it for their community (`community`). Other communities adopt it (`federated`). Finally, governance ratifies it as curriculum-canonical for the commons (`commons`).

### 4.3 Content Attestation Types

| Attestation Type | Typical Grantor | Reach Granted |
|------------------|-----------------|---------------|
| `author-verified` | System | `local` |
| `steward-approved` | Domain steward | `community` |
| `community-endorsed` | N community members | `community` |
| `peer-reviewed` | Qualified reviewers | `federated` |
| `governance-ratified` | Governance process | `commons` |
| `curriculum-canonical` | Curriculum authority | `commons` |
| `safety-reviewed` | Safety moderators | `federated` |
| `accuracy-verified` | Domain experts | `federated` |
| `accessibility-checked` | Accessibility reviewers | `community` |
| `license-cleared` | Legal/licensing review | `commons` |

**Stacking Attestations:**

Content can hold multiple attestations. Effective reach is the highest level granted by any active attestation. A video with both `steward-approved` (community) and `peer-reviewed` (federated) has effective reach of `federated`.

### 4.4 URL Patterns for Content Attestations

**Viewing Content Trust Profile:**
```
/lamad/resource:{resourceId}/trust
```
Returns the content's trust profile including effective reach, active attestations, trust score, and any flags.

**Requesting Attestation for Content:**
```
POST /lamad/resource:{resourceId}/attestation/request
```
```json
{
  "attestationType": "steward-approved",
  "justification": "Ready for community review after pilot testing",
  "evidence": {
    "type": "review",
    "description": "Tested with 12 learners, 90% completion rate"
  }
}
```

**Steward Review Queue:**
```
/lamad/me/steward/pending-reviews
```
Shows content waiting for this steward's attestation decision.

**Granting Attestation:**
```
POST /lamad/resource:{resourceId}/attestation/grant
```
```json
{
  "attestationType": "steward-approved",
  "reachGranted": "community",
  "scope": {
    "communities": ["elohim-protocol-learners"]
  },
  "evidence": {
    "type": "review",
    "description": "Reviewed for accuracy and pedagogical value"
  }
}
```

**Revoking Attestation:**
```
POST /lamad/attestation:{attestationId}/revoke
```
```json
{
  "reason": "misinformation",
  "explanation": "Contains factually incorrect claim about X",
  "appealable": true,
  "appealDeadline": "2024-12-31T23:59:59Z"
}
```

**Discovering Content by Reach:**
```
/lamad/explore?reach=community&minTrustScore=0.5
```
Returns content available at community reach level with minimum trust score.

### 4.5 Trust Score Calculation

Trust scores (0.0 - 1.0) aggregate attestation quality and quantity:

```
trustScore = Σ(attestationWeight × grantorWeight) / maxPossibleScore
```

**Attestation Weights:**
- `author-verified`: 0.1
- `steward-approved`: 0.3
- `community-endorsed`: 0.2 (per endorsement, capped)
- `peer-reviewed`: 0.4
- `governance-ratified`: 0.5
- `safety-reviewed`: 0.2

**Grantor Weights:**
The grantor's own attestations influence weight. A steward with "curriculum-architect" attestation has higher weight than a newly-appointed steward.

### 4.6 Content Flags and Disputes

Content can be flagged, which may restrict reach regardless of attestations:

| Flag Type | Effect |
|-----------|--------|
| `disputed` | Blocks progression beyond `local` |
| `outdated` | Warning displayed, no reach restriction |
| `under-review` | Blocks progression beyond current level |
| `appeal-pending` | Reach frozen at pre-revocation level |
| `partial-revocation` | Some attestations revoked, others active |

**Flagging Content:**
```
POST /lamad/resource:{resourceId}/flag
```
```json
{
  "type": "disputed",
  "reason": "Contains disputed claim about historical event",
  "evidence": ["https://source1.example", "https://source2.example"]
}
```

### 4.7 Integration with Agent Attestations

The bidirectional model creates a complete trust ecosystem:

**Agent → Content (Existing):**
- Agent earns `4th-grade-math-mastery`
- This unlocks access to `algebra-fundamentals` path
- Gate: Agent attestations → Content access

**Content → Agent (New):**
- Content earns `governance-ratified`
- This unlocks reach to `commons` audience
- Gate: Content attestations → Audience reach

**Circular Trust:**
- Agent creates content
- Content needs attestation to reach community
- Steward (an agent with `steward` attestation) reviews
- Steward grants `steward-approved`
- Content reaches community
- Community members (agents) can now access
- Some earn attestations through engaging with content
- Those agents may become stewards themselves

### 4.8 Holochain Mapping

**Entry Types:**
- `content_attestation`: The attestation record
- Link: `content_node` → `content_attestation`

**Validation Rules:**
- Only agents with appropriate attestations can grant certain attestation types
- Revocations create new entries (no deletion in DHT)
- Trust profiles computed locally by aggregating linked attestations

**Privacy:**
- Attestation requests may be private (author → steward channel)
- Granted attestations are public (stored on DHT)
- Flags are public with attribution

### 4.9 ContentNode Schema Updates

The ContentNode entity now includes trust fields:

```typescript
interface ContentNode {
  // ... existing fields ...

  // Trust & Reach
  authorId: string;                    // Required - anonymous content cannot earn reach
  reach: ContentReach;                 // Current effective reach level
  trustScore: number;                  // Computed from attestations (0.0-1.0)
  activeAttestationIds: string[];      // IDs of active attestations
  invitedAgentIds?: string[];          // For 'invited' reach
  communityIds?: string[];             // For 'community'/'federated' reach
  flags?: ContentFlag[];               // Active warnings
  trustComputedAt?: string;            // When trust profile last calculated
}
```

---

## Conclusion and Commitment

This specification defines the architecture for Lamad as path-centric learning infrastructure that prioritizes journeys over content, stewardship over consumption, and sovereignty over control.

The URL patterns, data models, and service interfaces are designed to remain stable from prototype through production deployment and Holochain migration. Frontend and backend can evolve independently as long as they respect these contracts.

Developers implementing Lamad features should treat this document as the source of truth. When requirements are unclear or conflicts arise, refer back to these definitions. When gaps are discovered, update this specification rather than making ad-hoc decisions.

This is infrastructure for meaning-making at scale. It will serve learners pursuing technical skills, relationships building trust, organizations developing capacity, and communities bridging divides. The architecture must be worthy of that mission.

Build with care, test thoroughly, and maintain fidelity to the core principles: Territory holds knowledge, Journey creates meaning, and Travelers have sovereignty over their own paths through understanding.
