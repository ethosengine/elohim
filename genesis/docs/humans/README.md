# Sample Humans for Lamad Prototype

This directory contains sample human profiles and relationships for demonstrating the Lamad data model and user stories.

## Files

- `humans.json` - All humans and relationships (source of truth)
- `README.md` - This documentation

## Human Graph Concept

Humans are graph objects in their own layer, separate from content:
- **Content Graph**: What you learn about (ContentNodes)
- **Human Graph**: Who you learn with/from, who you relate to (HumanNodes)

Both graphs use the same infrastructure (edges, reach, intimacy) but with different semantics:
- Content has authors and attestations
- Humans have agency, consent requirements, and bidirectional relationships

## Managing Humans

### Add a new human (interactive)
```bash
cd elohim-app
python scripts/add_human.py
```

### Add a human via command line
```bash
python scripts/add_human.py \
  --name "Alice" \
  --id "alice-activist" \
  --bio "Community organizer" \
  --category "community" \
  --affinities "organizing,mutual-aid"
```

### Add a relationship
```bash
python scripts/add_relationship.py \
  --from matthew-manager \
  --to alice-activist \
  --type neighbor \
  --intimacy connection
```

### Import into Lamad data
```bash
python scripts/import_humans.py
```

## Human Categories

| Category | Description | Count |
|----------|-------------|-------|
| core-family | Matthew's household | 4 |
| workplace | EthosEngine team | 1 |
| community | Neighborhood connections | 2 |
| affinity | Faith, education, and interest networks | 4 |
| local-economy | Workers and business owners | 7 |
| newcomer | Immigrants and refugees | 2 |
| visitor | Anonymous explorers | 1 |
| red-team | Adversarial actors for testing | 5 |
| edge-case | Low-engagement and special cases | 1 |

## Relationship Types

Relationships map to governance layers:

| Layer | Relationship Types |
|-------|-------------------|
| Family | spouse, parent, child, sibling, grandparent, grandchild |
| Neighborhood | neighbor, local_friend |
| Workplace | coworker, manager, direct_report, business_partner |
| Affinity | congregation_member, mentor, mentee, learning_partner |
| General | friend, acquaintance, network_connection |

## Intimacy Levels

| Level | Description | Example |
|-------|-------------|---------|
| intimate | Closest relationships | Spouses, parent-child |
| trusted | Deep trust, shared vulnerability | Mentors, close friends, grandparents |
| connection | Regular positive interaction | Neighbors, coworkers, congregation |
| recognition | Know of each other | Acquaintances, customers |

## Red Team Patterns

These humans demonstrate adversarial patterns the network must handle:

- **charlie-challenged**: Misinformation spreader (reach restricted)
- **sam-shipper**: Spam/commercial abuse (high-volume low-quality)
- **renold-regent**: Tribal gatekeeper (exclusionary behavior)
- **dolittle-doctor**: Credentials fraud (unverified claims)
- **tiffany-teen**: Vulnerable minor (no guardian oversight)

## User Stories Enabled

### Positive Patterns
1. **Love Map**: Matthew and Susan generate shared learning paths
2. **Family Learning**: Sammy's content is guardian-gated
3. **Community Discovery**: Nancy introduces Pam to Matthew
4. **Workplace Attestations**: Dan's skills attested by EthosEngine
5. **Economic Coordination**: Frank → Georgina → Susan supply chain
6. **Faith Formation**: Pete's congregation shares paths with intimacy controls
7. **Anonymous Exploration**: Traveler explores before committing to identity

### Adversarial Patterns (Reach Negotiation)
8. **Misinformation Containment**: Charlie's reach restricted after governance
9. **Spam Detection**: Sam flagged for high-volume low-acceptance
10. **Credentials Verification**: Dr. Dolittle's claims challenged
11. **Minor Protection**: Tiffany flagged as unguarded minor
