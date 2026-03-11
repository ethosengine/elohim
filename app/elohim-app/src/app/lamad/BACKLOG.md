# Lamad Feature Backlog

Ideas that are valuable but not immediate priorities. These are documented for future implementation.

---

## Expertise Discovery System

**Status**: Backlogged
**Model**: `expertise-discovery.model.ts` (exists but not yet integrated)
**Priority**: Medium
**Depends On**: Content Mastery system being fully implemented

### Overview

The mastery graph naturally reveals expertise. This enables queries like "Who's the best at X?" without gamified reputation scores.

### Key Features

1. **Expert Finding**
   - Query by domain/content area
   - Filter by mastery level, freshness, contribution activity
   - Rank by composite expertise score

2. **Reviewer Matching**
   - Find qualified reviewers for contributions
   - Match based on domain expertise + review history

3. **Mentor Matching**
   - Connect learners with appropriate mentors
   - Consider level advantage, teaching history

4. **Leaderboards**
   - Top experts by domain
   - Rising experts (steep mastery velocity)
   - Most helpful (peer review activity)

5. **Privacy Controls**
   - Discoverability settings (public/network/private)
   - Domain visibility controls
   - Leaderboard opt-in/out

### Value Proposition

- **For Learners**: Find mentors who actually know what they're teaching
- **For Contributors**: Find reviewers who can give meaningful feedback
- **For Stewards**: Identify who should participate in content governance
- **For the Community**: Surface hidden experts, reduce knowledge silos
- **For Elohim**: Route questions to humans who can actually answer them

### Implementation Notes

The `expertise-discovery.model.ts` file contains complete interfaces:
- `ExpertiseQuery`, `ExpertCandidate`, `ExpertLeaderboard`
- `ReviewerMatch`, `MentorMatch`
- `ExpertiseVisibility`, `MentorshipPreferences`
- `MasteryVelocity` for rising expert detection

Service to implement: `ExpertiseDiscoveryService`

---

## Future Ideas

_Add new backlog items below as they come up_

---

*Last updated: 2024-11-29*
