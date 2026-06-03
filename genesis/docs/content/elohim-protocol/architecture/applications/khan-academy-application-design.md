---
title: Khan Academy — substrate-native learning platform (lamad)
id: khan-academy-application-design
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

A grandmother helps her seven-year-old grandson open the app on her old Android tablet. He sees: a learning path ("Counting and Place Value"), the next lesson queued up with a familiar Khan-shape video, an interactive practice quiz rendered by `<sophia-question>`, and a **mastery skyline** showing every concept he has touched colored by Bloom level (seen / remember / understand / apply / analyze / evaluate / create). She taps a tab and sees her own **lifelong-learner view** — Spanish vocabulary she has been refreshing for two years, a knitting course she is auditing, a "civics for elders" cohort she joined with three neighbors. A teacher in their cohort sees: a class roster, per-learner trajectories, what is stuck, what is mastered, who is ready for peer-teaching.

The app feels like Khan Academy. It is not Khan Academy. The mastery history is the grandson's, content-addressed and portable; the cohort is a Collective EPR not a Google Classroom row; the videos pull-fetch from iroh-blob (or fall back to a YouTube bridge during transition); there is no engagement-optimization recommendation engine deciding what he "should" want to learn next.

## Primitive composition

| What you see | Primitive | Notes |
|---|---|---|
| Course | EPR (`content_type: "course"`) | reach=`commons` for public courses; `community` for closed cohorts |
| Learning path | EPR (`content_type: "path"`) | DAG of lesson EPRs; uses `Links` for prerequisite + succession edges |
| Lesson / module | EPR (`content_type: "lesson"`) | `parent_epr_cid = course_cid`; body markdown, video, or sophia-quiz |
| Video lesson body | EPR (`content_type: "lesson"`) + `media_cid → iroh-blob` | bytes pull-fetched on play |
| Quiz / assessment | EPR (`content_type: "quiz"`, `contentFormat: "sophia-quiz-json"`) | rendered by `<sophia-question>` |
| Discovery / reflection | EPR (`content_type: "discovery-assessment"` / `"reflection"`) | psyche-survey mode for open-ended |
| Lesson view | Observation (`observation_kind: "lamad:content-viewed"`) | dwell_ms, scroll_depth; libp2p only; retention=contextual |
| Quiz interaction | Observation (`observation_kind: "lamad:mastery-check-result"`) | per-question Recognition; graduates to attestation:mastery |
| Learner attempt (graduated) | Event (`action: "attempted-quiz"`) | `parent_epr_cid = learner_account_cid`; `observation_refs` to per-question Observations |
| Lesson completion | Event (`action: "completed-lesson"`) | `parent_epr_cid = learner_account_cid` |
| Reflection authored | Event (`action: "reflected"`) | discovery/reflection-mode response |
| Mastery state per concept | Resource (`resource_classified_as: "mastery"`) | per (learner, concept) tuple; level derived from Event history |
| Bloom level transition | derived from Event history | mass-balance: attempt+evidence Events sum to current Bloom level |
| Cohort / class | Collective EPR (`content_type: "community"`) | references learner Households; teacher Membership |
| Teacher / mentor role | Membership EPR + role-Attestation | `role: "teacher"` granted by cohort-Collective |
| Mastery credential | Attestation (`attestation:mastery`) | issued at apply gate or above per Bloom design |
| Content-quality vote | Attestation (`attestation:content-quality`) | reach-elevation evidence for a lesson |
| Peer review of work | FeedbackSignal (`signal_kind: "peer-review"`, `signal_class: "trust"`) | feeds standing-curve for evaluate/create Bloom levels |
| Help-request "stuck" signal | FeedbackSignal (`signal_kind: "stuck"`, `signal_class: "care"`) | routes to cohort-teacher-elohim |
| Quarterly mastery snapshot | Commitment (`action: "checkpoint"`) | per D.12; collapses 10-year mastery query to recent-quarter |
| Cold-archive of attempts | Commitment (`action: "aggregate-subordinate"`) | per D.12; old per-question Observations subordinate after window closes |
| Stuck recovery / re-learn | Event (`action: "surface"`) | per D.2; pulls a shelved mastery Resource back to active |
| Disengagement / abandon | Event (`action: "dispose"`) | closes a path the learner is done with |
| Credential earned | Attestation (`attestation:mastery`) at create-level | issued by cohort-council or self-graduation when threshold + peer-review met |

Eight primitives, ~12 content-type discriminators, ~5 signal_kinds, no special-casing for learning.

## How one mastery attempt flows

Tommy (age 7, on grandma's tablet) opens a multiplication quiz embedded in a lesson:

```
1. The lesson EPR's body declares contentFormat: "sophia-quiz-json"
   Angular renderer mounts <sophia-question> with the quiz Moment
2. Tommy answers question 1; sophia-element fires onRecognition callback
   → QuizSessionService records a per-question Observation
        observation_kind: "lamad:mastery-check-result"
        subject_cid: quiz_cid
        payload_json: { node_id: concept_cid, score: 1.0, hint_count: 0 }
        reach: agent-private
        libp2p only — no DHT write yet
3. Tommy finishes all 6 questions over ~90 seconds
4. learning-elohim (per D.6 — subscribes to lamad:mastery-check-result)
   evaluates the per-question Observations against the manifest graduation_policy
   (self-threshold; 86400s window — see lamad/manifest/observation-kinds.json)
5. Graduation fires: learning-elohim authors a single Event
        action: "attempted-quiz"
        provider: Tommy's learner_account_cid
        resource: "mastery:multiplication" Resource
        quantity_delta: +1 attempt; score 5/6
        parent_epr_cid: Tommy's account EPR
        observation_refs: [iroh://... × 6 individual Observations]
6. DHT write — ~2 KB Event entry; reach=agent-private; validated by Tommy's neighborhood
7. Mastery Resource updates: the (Tommy, multiplication) tuple climbs from
   understand → apply (the attestation gate per BLOOM-MASTERY-DESIGN.md)
8. learning-elohim authors Attestation(attestation:mastery)
        subject_cid: Tommy's agent EPR
        metadata: { concept_cid, level: "apply", evidence_refs: [the Event] }
        reach: agent-private by default; learner can elevate to community
9. libp2p sync plane delta-syncs the Event + Resource projection
   to grandma's other devices (her phone, her laptop)
10. Local SQL projection update: ContentMasteryView row for (Tommy, multiplication)
    flips mastery_level = "apply", mastery_level_index = 4, freshness_score = 1.0
11. Dashboard auto-refresh: the mastery skyline tile turns from amber to green
    in <150 ms (local SQL, no network)
```

Bidirectionality matters for credentialing. Tommy's apply-level Attestation lives in his agent EPR's reach=`agent-private` shell. When he eventually wants college credit for the cumulative achievement, he elevates relevant Attestations to reach=`commons-attested` via a reach-mutation Event (per D.9); a credentialing collective queries the federated bundle. Cash-out the other direction: he can export the entire Event log + Attestation set as a portable JSON-LD bundle that any compliant learning platform can re-import.

## Storage footprint per learner-household

For a single learner with 10 years of trajectory (typical K-12 grandchild + grandparent lifelong-learning combination, ~3 active learners per household):

| Item | Count | Size | Total |
|---|---|---|---|
| Learner-account EPR (one per learner) | 3 | 5 KB | 15 KB |
| Subscribed Course EPRs (cached for offline) | ~50 | 8 KB | 400 KB |
| Subscribed Lesson EPRs (cached metadata; bodies pull from iroh-blob) | ~2,000 | 4 KB | 8 MB |
| Cohort Collective EPRs | ~5 | 6 KB | 30 KB |
| Lifetime Events (`attempted-quiz`, `completed-lesson`, `reflected`) | 3 learners × 10 yr × ~500/yr = 15k | 500 B | 7.5 MB |
| Mastery Resources (per concept, per learner) | 3 × ~3k concepts = 9k | 800 B | 7 MB |
| Mastery Attestations | ~5k | 1.5 KB | 7.5 MB |
| FeedbackSignals (peer-review, stuck, endorse) | ~2k | 300 B | 600 KB |
| Checkpoint Commitments (quarterly, per active mastery Resource) | ~100 | 2 KB | 200 KB |
| Quiz Observations (recent — pre-graduation cache) | ~5k | 250 B | 1.25 MB |
| Video-lesson `media_cid` references (bytes pulled on demand) | ~500 | — | 0 (refs only) |
| **Total local SQL projection** | | | **~32 MB** |
| Cold-archive (Observations graduated + signals aggregate-subordinated) | — | — | ~15 MB residue in quilt |

**Fits on a phone.** The mastery skyline, path browser, and quiz renderer all read from local SQL. The only network call is iroh-blob pull when a video plays — and even that lands in the local blob-cache (`blob-cache-tiers.service.ts`) for offline rewatching.

## Network bandwidth profile

- New attempt (with 6 per-question Observations + 1 graduated Event): ~3 KB DHT write + ~2 KB libp2p delta-sync to other learner devices ≈ ~25 KB total peer load
- Daily inbound per household: ~30 events × 25 KB ≈ 750 KB/day
- Video pull (~10 min lesson at 480p H.264 segmented): ~80 MB on first watch; **zero** on rewatch from `blob-cache-tiers` (warm → cold demotion per archetype age-curve)
- **Per learner-household, full participation: ~150 MB/month** for everything except the video bytes themselves; with regular video study, the warm video cache dominates and stabilizes around 5–10 GB on disk

## DHT entry impact

- Per learner-year: ~500 graduated Events (already heavily aggregated by `learning-elohim`) + ~50 Attestations + ~30 Mastery Resources updated ≈ 580 DHT entries
- 8B humans × 580 entries/yr × 80-year span = ~4 × 10¹³ entries IF everything went global
- But: nearly all Events are reach=`agent-private`; the DHT entry budget per peer is bounded by *neighborhood interest*, not global volume
- Per-peer DHT entry visibility: the learner's own neighborhood + opted-in cohort scope + commons-reach courses they consume
- Typical learner-household peer holds ~5k DHT entries — well inside the ~3k-per-peer-namespace working budget (multiple DNA namespaces split the load)
- The Bloom-gate's attestation gate matters here: only ~3–5% of attempts cross apply and earn an Attestation that propagates beyond agent-private reach. The mass of practice volume stays personal.

## Why the mastery skyline renders fast

- **Per-concept current level**: `SELECT mastery_level, freshness_score FROM content_mastery_view WHERE human_id = :learner ORDER BY updated_at DESC`. Single-table read against the local projection (`elohim/elohim-storage/src/db/content_mastery.rs`). Indexed by `human_id`; ~3k rows for an active learner; under 50 ms.
- **Path progress**: `SELECT path_id, COUNT(*) FILTER (WHERE mastery_level_index >= 4) / COUNT(*) FROM content_mastery_view JOIN path_node_edges ON ... GROUP BY path_id`. Pre-aggregated by `mastery-stats.service.ts`.
- **10-year mastery history without the snapshot**: 15,000 Events × 500 B = 7.5 MB row-scan; ~400 ms cold. Unacceptable for the skyline.
- **With D.12 checkpoint Commitments**: each Mastery Resource carries quarterly checkpoint snapshots; the SQL collapses to:
  ```sql
  WITH latest_checkpoint AS (
    SELECT (metadata_json::jsonb->'balance_snapshot'->>'level_index')::int AS snap_level,
           (metadata_json::jsonb->>'period_end')::int AS period_end
    FROM commitments
    WHERE action = 'checkpoint' AND subject_cid = :mastery_resource_cid
    ORDER BY period_end DESC LIMIT 1
  )
  SELECT COALESCE(lc.snap_level, 0) +
         COUNT(*) FILTER (WHERE e.action IN ('attempted-quiz', 'completed-lesson')
                          AND e.observed_at > COALESCE(lc.period_end, 0)) AS level_delta
  FROM latest_checkpoint lc
  LEFT JOIN economic_events e
    ON e.parent_epr_cid = :learner_account_cid
   AND e.metadata_json::jsonb @> jsonb_build_object('concept_cid', :concept_cid);
  ```
  ~50 events per quarter per concept = sub-100ms for any single skyline tile.

## Why long branching mastery trajectories don't melt the network

- Every quiz attempt is an Observation (libp2p, no DHT) until graduation; the per-question stream stays off the DHT
- `learning-elohim` graduates per-question Observations into one Event per session — a 6-question quiz collapses 6 Observations → 1 DHT Event
- Mastery Resources are subordinate to the learner-account EPR (via `parent_epr_cid`) — not independently gossiped; queryable only through the account's reach
- 3k concepts × 8B humans = 2.4 × 10¹³ Mastery Resources globally — but each one is reach=`agent-private`; the DHT never has to carry them all
- The only learning-side data that earns commons reach is the *content* (Course/Lesson/Quiz EPRs that pass the reach-elevation gate via `attestation:content-quality` votes) — and there are ~10⁶ of those globally, not 10¹³

## Where agentic intelligence carries the load

- **`learning-elohim`** (per D.6, watches `lamad:content-viewed`, `lamad:mastery-check-result`, `lamad:reflection-authored`): graduates per-question Observations into session-scope Events; updates Mastery Resources; issues Attestations at Bloom transitions. Without it the DHT would be flooded with per-question chatter; with it, a 6-question quiz collapses to 1 Event.
- **`vision-elohim`** (per D.6, watches `lamad:image-captured`): Tommy snaps a photo of his hand-drawn strawberry; vision-elohim authors `attestation:auto-tag` and routes the artifact to the cohort-teacher for evaluate-level peer review. Out-of-app learning becomes legible.
- **`care-stewardship-elohim`** (per D.6, watches `imagodei:care-act`): grandma reading to Tommy at bedtime → care-Event with a `concept_cid` reference → tags the bedtime story as informal-learning evidence on Tommy's `understand`-level Mastery Resource. Kitchen-table learning lands in the mastery skyline without surveillance.
- **`cohort-teacher-elohim`** (new specialization, planned): listens for `signal_kind: "stuck"` (signal_class=`care`) FeedbackSignals; surfaces stuck-learner clusters to the human teacher; drafts re-explanation paths drawing on commons-reach lessons. Never authors learner-facing decisions, only mentor-facing observations.
- **`standing-stewardship-elohim`** (per D.14): re-derives per-signal_class standing so learners reaching evaluate/create Bloom levels accumulate reputation that gates further authoring privileges (contributing derivative lesson EPRs with elevated reach).

Without these agents, every keystroke would be a DHT write or nothing; out-of-app evidence would stay invisible; grandparent–grandchild reading time would not be part of the mastery picture.

## What the cohort and federated views show

The Cohort Collective EPR references member Household EPRs and per-learner Membership EPRs. Teacher dashboards render via federated query:

```
cohort-teacher-elohim issues a federated SELECT across member households:
   For each (household, learner) where learner.consents_to_cohort_visibility = true:
     SELECT mastery_level, last_engagement_at, stuck_signal_count
     FROM content_mastery_view JOIN feedback_signals ...
     WHERE human_id = :learner AND path_id = :cohort_path
  parallel libp2p RPC to each household's local SQL projection
  cohort-hub aggregates the per-learner result rows
  renders the cohort-progress matrix without ever ingesting source Observations
```

The cohort-hub holds **per-learner roll-ups** (current Bloom level per concept, last-engagement timestamp, count of stuck signals) — never the underlying quiz responses, never the dwell-time Observations, never the reflection text. Per-learner privacy is preserved by reach: only the roll-up summary fields are surfaced via the Membership's declared reach=`community`. If a learner revokes cohort membership (or grandma withdraws Tommy from the class), the hub's roll-up references go stale immediately because the hub never held the source data — the consent revocation is structural, not policy-enforced.

Multi-grade and multi-cohort federation works the same way recursively: a district-scale collective EPR references cohort Collective EPRs which reference learner Households. Each tier holds only the aggregate appropriate to its reach. **No tier replicates the layer below.**

## Dissolution in practice

- Tommy graduates from second grade → his K-1 cohort Membership transitions to `closed` via `Event(action="close-membership")`; the apply-level Attestations remain queryable forever; the per-question Observations have long since aggregate-subordinated to cold-archive
- A course steward removes a deprecated lesson → `Event(action="dispose", subject_cid=lesson_cid)`; existing Mastery Resources referencing that lesson keep their level (provenance preserved); the lesson EPR transitions to `closed`; new Mastery Events targeting the closed lesson fail validation
- Grandma abandons her knitting course mid-trajectory → `Event(action="dispose", subject_cid=path_cid, disposition_kind="abandoned")`; the path Resource transitions to `closed`; mastery skyline hides closed paths by default but surfaces them under "what I've moved on from"
- Cohort ends at end of term → `Event(action="dispose", subject_cid=cohort_cid)`; the Collective EPR transitions to `closed`; the teacher's roster Attestations remain queryable for transcript generation

The cradle-to-cradle hook (deferred per the records-lifecycle spec) shows up here as: when a learning path closes, its Mastery Resources retain a `disposition_link` to the successor path (if any) so a learner returning years later can pick up the trajectory without losing history.

## Bridges (legacy interop / cash-out)

- **`bridges/khan/`** (import-only) — Khan Academy's open content corpus imports as Course/Lesson/Quiz EPRs at reach=`commons` with `attestation:content-quality` pre-issued by an import-stewardship-elohim under the Khan Academy collective's signature. Re-export: any substrate-native Lesson EPR re-exports to Khan-compatible JSON-LD.
- **`bridges/youtube/`** (read-only, transitional) — Khan-on-YouTube videos referenced by `media_cid` plus `legacy_provenance: { source: "youtube", video_id: "..." }`. Viewer pulls from YouTube during transition; substrate-native re-host happens organically as Lesson EPRs accumulate iroh-blob bodies.
- **`bridges/perseus/`** (already integrated) — Khan-Perseus assessment widgets render via `<sophia-question>`. Not a wire-shape bridge; a rendering bridge — assessment content is substrate-native JSON; only the widget renderer is forked from Perseus.
- **`bridges/qti/`** (planned) — IMS QTI + Experience API export/import for Canvas/Blackboard/Moodle. Cash-out: full Attestation set + Event log → QTI/LRS for legacy LMS transcript ingestion.
- **`bridges/credly/`** (planned) — at commons-attested credential threshold, emits Credly-shape Open Badges under credentialing-stewardship-elohim signature for legacy resume parsers.

A learner's mastery history is theirs. Any platform respecting the Event + Attestation JSON-LD shape can re-import the trajectory; any substrate-native cohort can ingest a learner's Khan/Coursera/edX past via the import bridges.

## Code anchors

| Surface | Path |
|---|---|
| Lamad pillar Angular services | `app/elohim-app/src/app/lamad/services/` (49 services; the Khan-shape surface) |
| Path / mastery / assessment services | `app/elohim-app/src/app/lamad/services/{path,mastery,assessment,content-mastery,practice}.service.ts` |
| Content / mastery views (Rust) | `elohim/elohim-views/src/lamad.rs` (`ContentView`, `ContentMasteryView`, `ContentAssignmentView`, `ContributorDashboardView`) |
| Mastery storage | `elohim/elohim-storage/src/db/content_mastery.rs`, `src/api/mastery.rs`, `src/services/mastery_depth.rs` |
| Lamad pillar manifest | `elohim/sdk/domains/lamad/manifest.json` (+ `manifest/observation-kinds.json`, `manifest/attestations.json`, `manifest/signals.json`) |
| Bloom mastery design | `app/elohim-app/src/app/lamad/BLOOM-MASTERY-DESIGN.md` |
| Sophia integration | `app/elohim-library/projects/perseus-plugin/` (the Angular wrapper; called sophia-plugin historically) |
| Sophia rendering | `<sophia-question>` web component from `@ethosengine/sophia-element` UMD |
| Quiz session services | `app/elohim-app/src/app/lamad/quiz-engine/`, `services/practice.service.ts` |
| Cohort / community services | `app/elohim-app/src/app/lamad/services/{relationship,resilience,household-resilience}.service.ts` + qahal pillar membership |
| Lesson blob bootstrap | `app/elohim-app/src/app/lamad/services/blob-bootstrap.service.ts`, `blob-cache-tiers.service.ts` |
| Learning-elohim graduation evaluator (planned per D.6) | `elohim/elohim-storage/src/services/graduation_evaluator.rs` (extension) + `app/elohim-app/src/app/elohim/elohim-agents/learning-elohim.service.ts` |
| Khan import bridge (planned) | `bridges/khan/` |
| YouTube bridge (planned) | `bridges/youtube/` |
| QTI export/import (planned) | `bridges/qti/` |
| Doorway HTTP routes | `doorway/doorway-service/src/handlers/lamad/` |

## What this proves about the substrate

A skeptical systems architect should walk away from this archetype able to say:

- The Postgres-like primary surface (EPR + Event + Mastery-Resource projected to local SQL via `content_mastery.rs`) handles 10 years of K-12-through-grandparent learning trajectory across three household learners in ~32 MB — interactive at touch speed, fully offline-capable.
- The Kafka-like event flow (per-question Observations → session-scope graduated Events authored by `learning-elohim`) replaces the per-keystroke logging anti-pattern of legacy LMSs while preserving the per-question evidence as `observation_refs` for audit / re-grading.
- The S3-like asset surface (iroh-blob keyed by `media_cid`, with the `blob-cache-tiers` warm/cold demotion) handles video-lesson bytes at near-zero gossip cost and full offline rewatchability.
- The federated-query pattern (cohort-hub coordinating libp2p RPC across member households) gives the teacher dashboard without ingesting per-learner Observations — privacy by structure, not by ACL.
- The Bloom-gated reach pattern (apply-level Attestations stay agent-private by default; analyze/evaluate/create earn commons reach through peer-review FeedbackSignals feeding the standing-curve) gives credentialing-without-central-issuer: a degree is just an Attestation bundle with sufficient evidence depth, queryable by any consenting verifier.

If those five claims hold for Khan, the substrate's learning surface is real. The seven sibling archetypes in this directory pressure-test the same primitives against different stress profiles (massive blob in Photos, social-graph density in Meta, real-time collab in Drive, marketplace matching in Requests-and-Offers, compute economics in AWS, creator monetization in Patreon, personal-finance flow in Mint/Monarch).
