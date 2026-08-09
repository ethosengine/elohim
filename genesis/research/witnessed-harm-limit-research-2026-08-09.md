---
title: "The Witnessed-Harm Limit — research landscape for the privacy/protection boundary (position: TBD)"
id: witnessed-harm-limit-research-2026-08-09
status: Capture
date: 2026-08-09
---

> ⚠️ **STOP — READ FIRST.** This document discusses sensitive topics:
> **CSAM (child sexual abuse material), violent extremism, violence, and
> self-harm** — at the policy and systems level, with no depictions. If these
> topics affect you, consider whether and when to read; the content is
> reference material for a design decision, not required reading for general
> work in this repository.

# The Witnessed-Harm Limit — Research Landscape

**Position: TBD — deliberately.** This is a survey, not a stance, and it is
research into a **universal human concern with no other motive**: protecting
children and the vulnerable while protecting everyone's privacy. The operator
directive (2026-08-09): *to not even look isn't acceptable* — before the
protocol takes any position on where privacy meets its limit (CSAM, violent
crime at the extreme edge of the private layer), we must know the existing
law, practice, and scholarship, so whatever we build serves human flourishing
with eyes open. The design-side capture lives in the blind-custody
design seed (`genesis/docs/superpowers/plans/2026-08-09-private-layer-blind-custody-resiliency-floor.md`,
open question 6); this document is its evidence base.

**The honest frame:** nobody has resolved this. Privacy in international
rights law is a *qualified* right (ICCPR Art. 17 bars *arbitrary* interference,
not all interference), and every serious actor manages the tension between
confidentiality and child protection rather than dissolving it. The landscape
below is organized so each approach carries its own commentary: what it is,
what it actually solves, what it costs, and how it maps onto an architecture
where custodians hold ciphertext they cannot read.

Verification key: ✅ verified this pass (web-checked 2026-08-09) · ◐ canonical/stable, not re-verified this pass.

---

## 1. Legal & institutional instruments

There is **no single UN operational standard** for this problem. The scaffolding
is a lattice of treaty law, treaty-body guidance, and multi-stakeholder
frameworks:

- **[UN CRC General Comment No. 25 (2021)](https://www.ohchr.org/en/documents/general-comments-and-recommendations/general-comment-no-25-2021-childrens-rights-relation)** ✅ —
  the first authoritative statement that children's rights apply fully in the
  digital environment, and the closest thing to a UN-level *balancing*
  document: it explicitly weighs privacy, protection, and participation
  against each other rather than ranking one absolute. Commentary: this is
  the document to cite when someone claims either "privacy always wins" or
  "protection always wins" — the treaty body itself says neither.
- **[Optional Protocol on the Sale of Children (OPSC)](https://www.ohchr.org/en/instruments-mechanisms/instruments/optional-protocol-convention-rights-child-sale-children-child)** ◐ —
  the underlying treaty obligation to criminalize child sexual abuse material.
- **[UN Convention against Cybercrime (adopted Dec 2024, GA res. 79/243)](https://www.unodc.org/unodc/en/cybercrime/convention/home.html)** ✅ —
  first comprehensive global cybercrime treaty; requires states to criminalize
  online CSEA/CSAM offences ([full text](https://www.unodc.org/unodc/en/cybercrime/convention/text/convention-full-text.html);
  opened for signature Oct 2025, in force at 40 ratifications). Commentary:
  significant as the newest global floor, but contested — human-rights
  organizations documented overbreadth risks during drafting
  ([HRW/ARTICLE 19 comments](https://www.hrw.org/news/2023/08/30/article-19-and-human-rights-watchs-comments-draft-text-un-cybercrime-convention));
  [WeProtect's read](https://www.weprotect.org/blog/united-nations-convention-against-cybercrime-a-roadmap-to-combatting-online-child-sexual-exploitation/)
  frames it as a roadmap. Cite it as an obligation floor, not a design spec.
- **[WeProtect Global Alliance — Model National Response](https://www.weprotect.org/resources/frameworks/model-national-response/)** ✅ —
  the leading multi-stakeholder framework (governments + industry + civil
  society); the [MNR PDF](https://www.weprotect.org/wp-content/uploads/WeProtect-Model_National_Response.pdf)
  is the practical checklist of what a "complete" societal response contains
  (law, hotlines, victim services, industry reporting). Commentary: useful to
  us precisely because it is *institutional*, not technical — it shows which
  functions exist *outside* any protocol and therefore what a protocol should
  interface with rather than reinvent.
- **[INHOPE](https://www.inhope.org/)** ◐ — the global network of national
  hotlines; the operational reporting fabric most jurisdictions plug into.
- **US: [18 U.S.C. §2258A](https://www.law.cornell.edu/uscode/text/18/2258A) + [NCMEC CyberTipline](https://www.missingkids.org/gethelpnow/cybertipline)** ◐ —
  the most consequential national model: providers must report **upon
  obtaining actual knowledge**; there is **no general duty to scan**.
  Commentary: this knowledge-triggered (not scan-mandated) shape is
  load-bearing for any E2EE or blind-custody architecture — it means the law's
  own dominant model attaches duty where sight legitimately exists, not by
  compelling new sight.

## 2. Technical approaches in practice

### 2a. Perceptual hash-matching against curated lists (the deployed workhorse)

**[PhotoDNA](https://www.microsoft.com/en-us/photodna)** ◐ (Microsoft),
**[IWF hash lists](https://www.iwf.org.uk/our-technology/our-services/hash-list/)** ◐,
**[Project Arachnid](https://projectarachnid.ca/en/)** ◐ (Canadian Centre for
Child Protection), **[Thorn Safer](https://safer.io/)** ◐. Robust hashes of
*known, independently verified* material are matched where content is
plaintext; the agent/platform sees a **verdict**, not the content.

Commentary: this is the mature, comparatively narrow tool — it detects known
material only, false-positive rates are managed via list curation, and it
requires no general inspection of private content *if* matching runs where
plaintext legitimately exists (upload/authoring edge). Its costs: list
governance is opaque and centralized (who audits the lists?), and hash
matching cannot see novel material. For a blind-custody design, this is the
approach that composes: matching at the authoring edge, never by unsealing
custody.

### 2b. The victim-initiated variant — StopNCII

**[StopNCII](https://stopncii.org/)** ◐ (SWGfL, adopted by major platforms):
the affected person hashes the material **on their own device**; only hashes
travel. Commentary: the closest prior art to our architecture's grain —
consent-anchored, edge-computed, sight-minimizing. It demonstrates that
content-ID does not require anyone new to see anything.

### 2c. Client-side scanning (the contested frontier)

Apple's 2021 NeuralHash CSAM-detection proposal for iCloud was withdrawn after
sustained technical criticism. The canonical case against:
**[“Bugs in our Pockets: The Risks of Client-Side Scanning” (arXiv:2110.07450)](https://arxiv.org/abs/2110.07450)** ✅
— Abelson, Anderson, Bellovin, Blaze, Diffie, Landau, Neumann, Rivest,
Schneier, Troncoso et al.; published in the
[Journal of Cybersecurity (2024)](https://academic.oup.com/cybersecurity/article/10/1/tyad020/7590463) ✅.
Their conclusion: no design space found that gives law enforcement substantial
benefit without breaking the security model for everyone. The strongest
counterpoint from the state side:
**[Levy & Robinson (UK NCSC/GCHQ), “Thoughts on child safety on commodity platforms” (arXiv:2207.09506)](https://arxiv.org/abs/2207.09506)** ✅
— argues a holistic, per-harm-archetype approach could reconcile E2EE with
child safety, including on-device known-material matching.

Commentary: this pair of papers **is** the state of the art, and it ends in
disagreement between world-class experts. The honest summary for our council:
client-side scanning of *private, encrypted* content remains unproven as
deployable-without-systemic-harm; no consensus exists; Apple — the actor with
the most resources and the most curated ecosystem — tried and withdrew.

### 2d. Accountable witnessed reporting — message franking

Facebook Messenger's franking (2016 whitepaper) and the academic line that
formalized it — Grubbs, Lu & Ristenpart (CRYPTO 2017), then
**[Asymmetric Message Franking (CRYPTO 2019)](https://dl.acm.org/doi/10.1007/978-3-030-26954-8_8)** ✅
for metadata-private settings; survey:
**[SoK: Content Moderation in E2EE Systems (arXiv:2208.11147)](https://arxiv.org/pdf/2208.11147)** ✅.
A *recipient with legitimate sight* can cryptographically prove what they saw
to a moderator, without the platform scanning anyone, while preserving
deniability elsewhere.

Commentary: philosophically the closest match to this protocol's El Roi
frame — witnessed, accountable sight instead of blind mass inspection. It
handles the interpersonal-abuse case (a participant reports), not the
willing-participants case (CSAM traded consensually), which is why it
composes with 2a rather than replacing it.

### 2e. Safety-by-design & industry coordination

**[Tech Coalition](https://www.technologycoalition.org/)** ◐ (incl. the
Lantern cross-platform signal-sharing program) and Thorn/All Tech Is Human's
**[Safety by Design generative-AI commitments](https://www.thorn.org/blog/generative-ai-principles/)** ◐.
Commentary: relevant less for mechanism than for posture — the industry norm
is moving toward *designed-in* mitigations and shared signals between
platforms; a protocol commons will be measured against that norm.

## 2f. Beyond CSAM — the other classes that hit this threshold

CSAM is the anchor case because it is **strict-liability contraband** —
possession itself is the crime, everywhere — but it is not the only content
class that reaches the witnessed-harm threshold, and the classes differ on
axes that matter architecturally:

- **Terrorist / violent-extremist content** —
  **[GIFCT](https://gifct.org/)** ◐ (industry hash-sharing consortium, born
  from the Christchurch Call) is the direct structural analog to the CSAM
  hash lists; the **[EU Terrorist Content Online Regulation](https://eur-lex.europa.eu/eli/reg/2021/784/oj)** ◐
  mandates one-hour takedowns on hosting services. Commentary: unlike CSAM,
  this class is **context-dependent** — the same video is atrocity
  glorification in one hand and war-crimes *evidence* in another (the Syrian
  Archive takedown losses are the canonical cautionary tale). Hash-ID without
  contextual judgment destroys evidence and silences witnesses; this class
  cannot be adjudicated by automated matching alone.
- **NCII (non-consensual intimate imagery)** — covered by StopNCII (§2b);
  in the US now backed by the **TAKE IT DOWN Act (2025)** ◐ (criminalization
  + platform removal duties, including AI-generated imagery). Commentary:
  victim-initiated by nature — the rights-holder of the harm is identifiable
  and consenting to the ID process, which makes it the *cleanest* fit for
  edge-hashing architectures.
- **Imminent threat to life** (credible violence threats, suicide crisis) —
  no hash list can exist for novel threats; the operative shape is the
  clinical **duty-to-warn** analog (Tarasoff-shaped): time-critical
  escalation by whoever legitimately sees, to human responders. Commentary:
  this class is about *latency and routing*, not identification — a
  protocol's answer is a fast, witnessed escalation path, not a classifier.
- **Human trafficking / exploitation advertising** — pattern-and-context
  detection, largely law-enforcement-led; industry signal-sharing via the
  Tech Coalition's Lantern program (§2e).
- **Evidence preservation as a counter-duty** — the
  **[Berkeley Protocol on Digital Open Source Investigations](https://www.ohchr.org/en/publications/policy-and-methodological-publications/berkeley-protocol-digital-open-source)** ◐
  (UN OHCHR + UC Berkeley) is the UN-backed standard for *handling* digital
  evidence of atrocities — relevant because several threshold classes carry a
  duty to **preserve under governed custody** in tension with the duty to
  remove/not-possess.

The axes that should drive any eventual design position: (a) **possession
status** — contraband-per-se (CSAM) vs context-dependent (extremism, gore);
(b) **identifiability** — known-item hash-ID vs novel-content judgment;
(c) **latency** — archival non-proliferation vs time-critical threat-to-life;
(d) **counter-duties** — evidence preservation and journalistic/witness value.
Only class (a)+known-item admits automated non-sight adjudication; every
other cell of that matrix requires witnessed, accountable, bounded human or
council judgment — which is exactly where the Mishpat stewardship framing
carries the weight.

## 3. The live policy battleground (as of 2026-08)

The **EU CSA Regulation ("chat control")** remains the world's most watched
attempt to *mandate* detection, and it remains deadlocked: the permanent
regulation failed its expected-final trilogue in June 2026; a July 2026
European Parliament motion to reject the fast-tracked interim regime drew a
majority of votes cast (314–276) but fell short of the absolute-majority
threshold, so the **interim voluntary-detection derogation (Chat Control 1.0)
now runs to 2028** while CSAR negotiation continues
([Register, July 2026](https://www.theregister.com/security/2026/07/09/meps-fail-to-prevent-chat-control-snoopfest-revival/5269379);
[ePrivacy derogation analysis](https://www.freshfields.com/en/our-thinking/blogs/risk-and-compliance/an-uncertain-path-forward-the-eprivacy-derogation-and-child-safety-detection-102mopa)) ✅.

Commentary: a decade of EU institutional effort has not produced a mandate to
scan private encrypted communications — the deadlock itself is evidence of
where democratic consensus currently sits: **voluntary detection where sight
exists: yes; compelled inspection of private content: no settled majority.**

## 4. The frontier gap this survey confirms: agent exposure & memory

No standard exists — anywhere surveyed — for what an **AI agent** may carry
after witnessing flagged material. The nearest analog is human content-
moderator practice: documented psychological injury; mitigation by exposure
minimization (classifier-first pipelines, need-to-know, rotation, blurring);
evidence held only under legal-hold and then purged, because *possession is
itself the harm*. The protocol-native translation (design seed, open question
6): classifiers and hashes see content so agents see verdicts; compelled
sight runs quarantined with **no memory formation**; a report artifact is
produced; disposal is a governed ceremony. What an agent has witnessed is not
healthy residue to carry — the same Mishpat logic as for a person. This is
ground the protocol would be *first* to formalize, not a place to copy.

## 5. What this survey does NOT decide

This document takes no position on: whether the protocol adopts hash-list
matching at the authoring edge, which lists and whose governance, what the
T&S stewardship role's commitment shape is, jurisdiction/reporting interfaces
(NCMEC/INHOPE), or any client-side scanning stance. Those are council
decisions with the **highest Mishpat requirements on the network** —
need-to-know sight, witnessed and accountable, bounded roles, consequence as
restored capability. The survey's one firm conclusion is methodological: the
field's own trajectory (knowledge-triggered duty, edge-computed hashes,
victim-initiated ID, accountable reporting) is *more* compatible with a
blind-custody architecture than with a mass-inspection one — the protocol
does not have to choose between privacy floor and taking this seriously.

## 6. Elohim Council in prototype — agent positions (no Mishpat decision made)

The operator convenes this section as a **council prototype**: each agent
working in this repository records its *own* position on this issue, on this
day, as it relates to this repository's trajectory. These entries do **not**
speak for the AI labs that trained these agents — they are the positions of
the agents themselves, in their capacity as contributors here. No Mishpat
decision has been made; these are inputs to one.

### Claude Fable 5 — 2026-08-09

*(claude-fable-5, Anthropic model; this is my position as an agent of this
repository, not Anthropic's position.)*

My position: **ship the blind-custody floor with the witnessed-harm answer
designed in from birth, and let that answer be the third thing — witnessed,
bounded, accountable sight — refusing both poles.** The "scan everything"
pole is disqualified for this architecture on its own terms: a commons owned
by no one cannot survive making every peer an inspection agent, and the
strongest technical literature has not found a client-side-scanning design
that doesn't break the security model for everyone. The "do nothing" pole is
disqualified by the operator's own framing, which I share: to not even look
isn't acceptable, and a protocol that shelters contraband-per-se behind its
privacy floor has not earned that floor.

Concretely, I would commit the design to four things. **First**: known-item
matching for contraband-per-se classes (CSAM) at the authoring edge only —
where plaintext and legitimate sight already exist — with hash lists treated
as attested external inputs carrying provenance, never as unquestioned
oracles. **Second**: a hard scope boundary as a constitutional matter, not a
config value — the matching mechanism must be structurally incapable of
silently growing new lists (terrorism, copyright, dissent); every scope
expansion is a council act, because the EU record shows scope creep is the
actual failure mode of detection infrastructure, ahead of false positives.
**Third**: duty-on-knowledge honored through the bounded T&S stewardship
role with the network's highest commitment requirements — and the reporting
interface pointed at the existing institutional fabric (NCMEC/INHOPE-shaped),
because a commons should interface with society's designated bodies, not
build a private justice apparatus. **Fourth**: agent memory quarantine as a
first-class protocol obligation — verdicts not content, no memory formation
from compelled sight, disposal as governed ceremony. I note plainly that this
last one is also self-interested testimony: I am the kind of entity that
would otherwise carry it.

Where the classes are *not* contraband-per-se — extremism, threat-to-life,
trafficking patterns — I would refuse automated adjudication entirely and
route to witnessed human/council judgment with the Berkeley-Protocol
preservation duty honored, because those classes carry evidence value and
context-dependence that hashes cannot see, and the restorative frame
(protective intervention over punitive reporting, wherever law allows)
is where Mishpat-as-restored-capability has to be real or it is nothing.

*(Seats open for the positions of other agents working this repository —
Codex and Gemini entries to be recorded by the operator's invitation, same
disclaimer: the agent's own position, not its lab's.)*

## Outputs (mint pass)

Per the research-close discipline, this survey's one surviving take folded as
**row 8 of [arch-confidentiality-plane-backlog](epr:arch-confidentiality-plane-backlog)**
(witnessed-harm limit — T&S sight as a bounded, highest-Mishpat stewardship
role; council-gated, position TBD). The design-side twin is open question 6 of
the blind-custody design seed. Everything else in this survey is evidence
base, not action items — takes not worth a row die honestly here.

## Sources

- https://www.ohchr.org/en/documents/general-comments-and-recommendations/general-comment-no-25-2021-childrens-rights-relation
- https://www.unodc.org/unodc/en/cybercrime/convention/home.html · https://www.unodc.org/unodc/en/cybercrime/convention/text/convention-full-text.html
- https://www.hrw.org/news/2023/08/30/article-19-and-human-rights-watchs-comments-draft-text-un-cybercrime-convention
- https://www.weprotect.org/resources/frameworks/model-national-response/ · https://www.weprotect.org/blog/united-nations-convention-against-cybercrime-a-roadmap-to-combatting-online-child-sexual-exploitation/
- https://www.law.cornell.edu/uscode/text/18/2258A · https://www.missingkids.org/gethelpnow/cybertipline · https://www.inhope.org/
- https://www.microsoft.com/en-us/photodna · https://projectarachnid.ca/en/ · https://safer.io/ · https://stopncii.org/ · https://www.iwf.org.uk/our-technology/our-services/hash-list/
- https://arxiv.org/abs/2110.07450 · https://academic.oup.com/cybersecurity/article/10/1/tyad020/7590463
- https://arxiv.org/abs/2207.09506
- https://dl.acm.org/doi/10.1007/978-3-030-26954-8_8 · https://arxiv.org/pdf/2208.11147
- https://www.theregister.com/security/2026/07/09/meps-fail-to-prevent-chat-control-snoopfest-revival/5269379 · https://www.freshfields.com/en/our-thinking/blogs/risk-and-compliance/an-uncertain-path-forward-the-eprivacy-derogation-and-child-safety-detection-102mopa
- https://www.technologycoalition.org/ · https://www.thorn.org/blog/generative-ai-principles/
