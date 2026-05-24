---
title: Khan Academy — substrate-native learning platform (lamad)
tier: architecture
status: Composition draft (primitives mapped; full walkthrough pending)
created: 2026-05-24
authors: Matthew Dowell + Opus 4.7
pillar coupling: lamad (primary — learning, content, assessment, mastery), elohim (substrate), imagodei (cohort membership), shefa (educator monetization in commons-reach)
realizes:
  - genesis/docs/content/elohim-protocol/value_scanner/epic.md (learning visibility across life-stage archetypes — child, middle_aged, elder, grandparent learning trajectories)
  - genesis/docs/content/elohim-protocol/social_medium/epic.md (educator-as-creator with reach earning)
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md
  - genesis/docs/content/elohim-protocol/architecture/2026-05-11-observation-event-layer-design.md (learning observations graduate to mastery Events)
  - genesis/docs/content/elohim-protocol/architecture/2026-05-08-iroh-libp2p-complementarity.md (video lesson bytes in iroh-blob)
informs:
  - app/elohim-app/src/app/lamad/ (where the Khan-shape learning surface lives)
  - elohim/sdk/domains/lamad/manifest.json (content_types: course, lesson, quiz; action verbs: attempted-quiz, completed-lesson; observation_kinds: content-viewed, mastery-check-result)
  - sophia-element / sophia-plugin (assessment rendering — already substrate-integrated)
defers:
  - Recommendation engine / adaptive-pathway algorithm (application-layer ML, not substrate)
  - Credentialing / degree-equivalent attestation framework (governance design needed)
---

## The grandma test

A learner — child, teen, returning adult — opens the app on whatever device they have. They see: paths of courses, their current cohort, the next lesson, their mastery trajectory across topics, recent assessments. A teacher / mentor sees: cohort dashboards, per-learner trajectories, what's stuck, what's mastered. Khan-shape — but content-addressed, locally rendered, learner-owned mastery history.

## Primitive composition

| What you see | Primitive | Notes |
|---|---|---|
| Course | EPR (`content_type: "course"`) | reach=`commons` for public courses; `community` for closed cohorts |
| Lesson / module | EPR (`content_type: "lesson"`) | `parent_epr_cid = course_cid`; body can be markdown, video, or sophia-quiz |
| Video lesson | EPR (`content_type: "lesson"`) with `media_cid → iroh-blob` | bytes pull-fetched on play |
| Quiz / assessment | Content (`contentFormat: "sophia-quiz-json"`) | rendered by sophia-element |
| Learner attempt | Event (`action: "attempted-quiz"`) | `parent_epr_cid = learner_account_cid`; observation_refs to interaction-Observations |
| Score / answer | derived from Event's payload | mass-balance: attempts → mastery delta |
| Mastery state | Resource (`resource_classified_as: "mastery"`) | per topic; balance derived from Event history of attempts + reflections |
| Reflection (discovery / open-ended) | Event (`action: "reflected"`) | sophia discovery / reflection mode |
| Cohort | Collective EPR (`content_type: "cohort"`) | references learner Households; teacher-Membership |
| Teacher / mentor role | Membership EPR with role-attestation | `role: "teacher"` granted by cohort-Collective |
| Lesson view | Observation (`observation_kind: "lamad:content-viewed"`) | dwell_ms, scroll_depth; libp2p; graduates to summary Event |
| Credential earned | Attestation (`content_type: "attestation:mastery"`) | issued by cohort or self-graduation when threshold met |

## Stress points the substrate handles

- **Long branching mastery trajectories per learner** — every attempt is an Event; balance derived; no special schema per subject
- **Many learners per cohort** — cohort-level aggregates federate through the cohort-hub; per-learner data stays agent-private
- **Sophia-rendered assessments** — already substrate-integrated via sophia-element web component; Recognition callbacks become Events
- **Video lessons at scale** — bytes content-addressed in iroh-blob; popular lessons replicate by demand
- **Reach across grade levels** — courses earn reach via FeedbackSignal + Attestation; commons-reach for public; community-reach for private cohorts

## Scale answer

- Per-learner: ~10k Events over a 10-year learning trajectory × 500 B ≈ 5 MB SQL
- Per-household: a few learners + a few courses tracked ≈ 20 MB SQL projection
- Per-cohort hub: aggregates over its members via federated query; doesn't replicate per-learner data
- Video bytes: in iroh-blob, demand-replicated; long-tail in quilt
- Globally: 8B learners × 5 MB = 40 EB of mastery history — but distributed across peers; per-peer footprint stays small

## Bridges to legacy

- **bridges/youtube/** (read-only) — Khan-Academy videos hosted on YouTube can be referenced by `media_cid` plus a `youtube_url` provenance field; viewer pulls from YouTube during transition; substrate-native re-host happens organically
- **bridges/khan/** (import-only) — Khan Academy's open content corpus imports as Course/Lesson EPRs under a public-commons reach scope
- **Cash-out**: a learner's mastery history exports as REA Event log + Attestation set; portable to any other learning platform that respects the format

## Code anchors

| Surface | Path |
|---|---|
| Lamad pillar Angular services | `app/elohim-app/src/app/lamad/` |
| Content / mastery views | `elohim/elohim-storage/src/views.rs` (`ContentView`, `MasteryView`) |
| Lamad pillar manifest | `elohim/sdk/domains/lamad/manifest.json` |
| Sophia integration | `app/elohim-library/projects/sophia-plugin/` + `sophia-element/` UMD |
| Sophia rendering | `<sophia-question>` web component (Recognition callbacks → Events) |

*Full draft pending — this composition draft establishes the primitive map; the storage footprint table, render-speed walkthrough, and "where agentic intelligence carries the load" sections follow the mint-monarch exemplar shape.*
