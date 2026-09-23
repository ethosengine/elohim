---
name: feedback_harness_witnesses_agents_dont_narrate
title: "The harness witnesses; agents never run their own attribution"
id: feedback-harness-witnesses-agents-dont-narrate
description: "Operator 2026-09-23 — an agent's change is captured by the harness (PostToolUse observer) into eprfs; agents never run import/witness verbs by hand. Friction only at validation."
metadata:
  node_type: memory
  type: feedback
  originSessionId: fa35e94d-33e3-4301-a1ba-263d90ff52da
  modified: 2026-09-23T16:56:02.526Z
---

An agent acting in the workspace should carry no mental load for provenance. When it changes a file,
the harness witnesses that and captures it into eprfs (the PostToolUse observer mints the act);
the agent never runs `epr flow memory import`, a witness verb, or an attribution step to be
recorded. Friction is spent only where validation needs it: a gate, a review, a graduate — the
points where trust is built and accountability exercised.

**Why:** operator 2026-09-23, stopping me before a manual memory import: "reduce the mental load on
any agent acting in the workspace… the harness should witness that and capture that to the files…
that's part of why we've got eprfs." A discipline that depends on agents remembering to record
themselves is one the swarm forgets first; a harness-witnessed one holds by construction.

**How to apply:** when a design needs an act recorded (author, edit, review, claim, fulfilment),
place the writer in the harness (hook/observer), not in the agent's instructions. If a hook is
missing, say so as the gap rather than doing its job by hand. See
[[project_human_participant_actor_plane]] (station 3 is an observer duty),
[[feedback_valueflow_authorship_is_the_process]] (verbs are the designed friction; everything else
frictionless), [[feedback_private_thought_governed_fruit]].
