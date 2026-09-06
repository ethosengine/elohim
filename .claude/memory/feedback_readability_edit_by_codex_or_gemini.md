---
name: feedback_readability_edit_by_codex_or_gemini
title: Explainers get a Codex/Gemini readability edit
description: "Operator 2026-09-06: long explainer docs get a blind-reader pass then a readability EDIT by Codex (GPT-6) or Gemini — Claude 5 models 'struggle' at clear concise readability; bites on every teach-from document"
metadata:
  type: feedback
---

**What the operator said (2026-09-06, on the Holons-Are-Spaces explainer):** "make sure you dispatch a blind reader,
maybe even have codex 5.6 take a stab at editing, because we need clear concise readability (some of the claude 5
series models have kind of struggled in this domain, gemini would also be a preferred writer)."

**Why:** the operator reads these documents to learn and to teach others; density and long sentences defeat that.
**How to apply:** for any explainer / guide / teach-from doc: (1) write; (2) context-isolated `blind-reader` pass with
the right profile; (3) a readability EDIT pass by `codex exec` (GPT-6 "astra") or a Gemini CLI if present — shorter
sentences, one idea per paragraph, keep every fact and citation; (4) re-read the diff for fact drift before commit.
See [[feedback_delegate_research_to_opus_sonnet_codex]].
