---
name: Skip spec doc + writing-plans for minor changes
description: For trivial scoped changes (small script, hook tweak, config edit), don't write a spec doc to genesis/docs/superpowers/specs/ and don't transition to writing-plans — the brainstorming convo + direct implementation is enough
type: feedback
originSessionId: 35be1571-6ae6-498c-ab6a-9534107e0330
---
For minor changes (a small script, a few-line hook tweak, a config edit), do not write a spec doc to `genesis/docs/superpowers/specs/` and do not transition to the `writing-plans` skill. Brainstorming conversation in the chat + direct implementation is enough. Mark the spec/plan brainstorming tasks as `deleted` rather than `completed`.

**Why:** User said "this is a minor change, just use your best judgement on the rest of the checks please" and then "even cleanup the written plan, it's not worth cluttering up the plans." Spec docs for trivial changes clutter the specs directory and rot fast — the value of a spec is in coordinating non-obvious work or multi-session efforts, not in documenting a 50-line shell script.

**How to apply:** When the brainstorming skill loads its full process, judge scope first. If the change is one file or one short script with no architectural implications, present the design inline, get verbal approval, implement, test, done. Skip steps 6 (write design doc), 7 (self-review), 8 (user reviews spec), 9 (writing-plans). The brainstorming skill itself acknowledges "The design can be short (a few sentences for truly simple projects)" — extend that latitude to skipping the artifact entirely. For substantive features (new service, new pillar surface, multi-component refactor) keep the full ceremony.
