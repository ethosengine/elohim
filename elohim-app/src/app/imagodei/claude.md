# Imago Dei Pillar - Identity

Human identity layer aligned with "Image of God" framework.

## Philosophy

Four dimensions of human identity:
- **imagodei-core**: Stable identity center (who am I?)
- **imagodei-experience**: Learning and transformation
- **imagodei-gifts**: Developed capabilities/attestations
- **imagodei-synthesis**: Growth and meaning-making

## Models

| Model | Purpose |
|-------|---------|
| `session-human.model.ts` | SessionHuman, SessionStats, upgrade prompts |
| `profile.model.ts` | HumanProfile, JourneyStats, TimelineEvent |
| `attestations.model.ts` | Agent attestations (credentials earned BY humans) |

## Services

| Service | Purpose |
|---------|---------|
| `SessionHumanService` | Temporary localStorage identity for MVP |
| `ProfileService` | Human-centered profile aggregation |

## Session Human Architecture

Zero-friction entry with Holochain upgrade path:

```
1. Human explores as visitor (localStorage session)
2. Meaningful moments trigger upgrade prompts
3. Human installs Holochain app
4. prepareMigration() packages session data
5. Data imports to agent's source chain
6. clearAfterMigration() removes localStorage
```

### Access Levels

| Level | Identity | Can Access |
|-------|----------|------------|
| `visitor` | Session (localStorage) | Open content |
| `member` | Holochain AgentPubKey | Gated content |
| `attested` | Member + attestations | Protected content |

## ProfileService API

```typescript
// Core identity (imagodei-core)
getProfile(): Observable<HumanProfile>;

// Learning journey (imagodei-experience)
getTimeline(): Observable<TimelineEvent[]>;
getCurrentFocus(): Observable<CurrentFocus[]>;

// Capabilities (imagodei-gifts)
getDevelopedCapabilities(): Observable<DevelopedCapability[]>;

// Meaning-making (imagodei-synthesis)
getTopEngagedContent(): Observable<ContentEngagement[]>;
getAllNotes(): Observable<NoteWithContext[]>;
```

## Terminology

- Use "human" not "user"
- Use "journey" not "consumption"
- Use "meaningful encounters" not "views"
