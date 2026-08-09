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
- **US: [18 U.S.C. §2258A](https://uscode.house.gov/view.xhtml?edition=prelim&f=treesort&num=0&req=granuleid%3AUSC-prelim-title18-section2258A) + [NCMEC CyberTipline](https://www.missingkids.org/gethelpnow/cybertipline)** ✅ —
  one especially explicit national case study: providers must report **upon
  obtaining actual knowledge**; there is **no general duty to scan**.
  Commentary: this knowledge-triggered (not scan-mandated) shape is
  load-bearing for any E2EE or blind-custody architecture — that statute
  attaches duty where sight legitimately exists, not by
  compelling new sight. But reporting does not immediately authorize disposal:
  a completed CyberTipline report also starts a one-year preservation duty for
  the reported contents and reasonably accessible contextual files, held
  securely with access limited to necessary personnel. The lifecycle is
  therefore not simply *see → report → delete*, but *ordinary custody →
  witnessed report/evidence hold → controlled release or disposal*.

### 1a. Comparative regulatory grammars — bounded sample, not world survey

The US case above is useful because its event sequence is unusually explicit;
it is **not** the protocol's default legal ontology. Other regimes place the
weight elsewhere. This comparison is deliberately bounded and remains
geographically incomplete:

| Instrument | Regulatory grammar | What it contributes to this question |
|---|---|---|
| **[EU Digital Services Act](https://eur-lex.europa.eu/legal-content/EN/ALL/?uri=CELEX%3A32022R2065)** ✅ | No general monitoring; orders and notice-and-action for specific illegal content; reasons, complaint and remedy; systemic-risk assessment for the largest services. | Separates *general inspection* from *action on specific knowledge*, while treating explanation and remedy as part of moderation rather than an optional aftercare layer. |
| **[UK Online Safety Act implementation](https://www.ofcom.org.uk/online-safety/illegal-and-harmful-content/illegal-content-duties-under-the-online-safety-act)** ✅ | Documented, recurring risk assessment; proportionate safety measures; reporting and complaints; swift action when illegal content becomes known. Current guidance also treats encouraging or assisting serious self-harm as a distinct priority-offence risk. | Makes service design and foreseeable amplification risk part of the duty, not only individual-item takedown after sight. It is a provider-regulation model, however, and does not map automatically onto a DHT custodian. |
| **[Australian eSafety — Safety by Design](https://www.esafety.gov.au/industry/safety-by-design)** ✅ | Provider responsibility, user empowerment and autonomy, transparency and accountability; safety developed alongside privacy and security. This is regulatory guidance, not itself a complete liability rule. | Supplies a product-design grammar: a protective intervention should preserve user agency and be inspectable, not merely satisfy a removal metric. |
| **[African Commission Declaration of Principles on Freedom of Expression and Access to Information](https://achpr.au.int/en/documents/2019-11-10/declaration-principles-freedom-expression-access-information-2019)** ✅ | Continental human-rights soft law: no compelled proactive monitoring of content an intermediary did not author or modify; restrictions require human-rights justification, transparency, appeal and remedy; ordinary removal should be independently authorized and reviewable. It permits expedited action for imminent danger only with judicial review, and protects confidential and encrypted communication. | Makes state overreach part of the threat model. Child safety, privacy, identity, affordable access, local languages and the rights of marginalized communities are coequal design concerns—not sequential exceptions to enforcement. |
| **[Inter-American freedom-of-expression standards](https://www.oas.org/en/iachr/expression/showarticle.asp?artID=849)** ✅ | Mere technical intermediaries should not be liable for others' content absent specific intervention or refusal of an order they can carry out; restrictions must be lawful, necessary and proportionate; communications-era rules cannot simply be transplanted onto the internet. | Centers capability and due process: responsibility should follow what an actor actually authored, saw, controlled or could change, rather than attaching indiscriminately to every relay. |
| **[ASEAN Regional Plan of Action on online child exploitation and abuse](https://asean.org/wp-content/uploads/2021/11/4.-ASEAN-RPA-on-COEA_Final.pdf)** ✅ | A 2021–2025 regional plan, with an extension option, joining criminal-law floors to cross-border coordination, child participation, victim support, capacity-building and measurable national action. It expressly balances protection with children's access, expression, privacy and information rights. | Insists that online protection cannot work without offline child-protection and justice systems. A protocol can route and preserve a witnessed report; it cannot manufacture a trustworthy responder or recovery service where none exists. |

Three architecture consequences follow from the comparison:

1. **The regulated unit cannot be “the network.”** A jurisdiction adapter must
   resolve the actor, role and event: who authored; who legitimately saw
   plaintext; who merely held ciphertext; who controlled discovery or
   amplification; who received a valid order; and where each act occurred.
2. **Constitutional floor and legal adapter are different layers.** Dignity,
   private access, access to help, proportionality, notice, challenge and
   recovery should survive jurisdiction. Offence definitions, reporting
   recipients, preservation periods and disclosure authority should not be
   frozen into the DHT as though one state's law were universal.
3. **The state and institutional fabric belong in the risk model.** A lawful
   reporting path in one place may expose a child, abuse survivor, dissident,
   queer person or undocumented person to additional danger elsewhere. The
   adapter therefore needs a governed responder directory and a
   minimum-necessary disclosure rule; “send to law enforcement” is not a
   globally safe primitive.

Commentary: the comparative record strengthens rather than weakens the Social
Reach premise. Earned amplification, bounded witnessed reporting, transparent
consequence and an inalienable private/help/recovery floor are ways to encode
the common human-rights shape while leaving legitimately different legal acts
at the edge where jurisdiction and actual sight exist.

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

This mechanism is also moving from papers into standards work. The IETF's
active **[MIMI protocol Internet-Draft](https://datatracker.ietf.org/doc/draft-ietf-mimi-protocol/)** ✅
(draft-06, April 2026) combines MLS end-to-end encryption with message
franking and a federated `reportAbuse` operation. It is explicitly work in
progress, not an RFC. Commentary: its presence validates witnessed reporting
as a protocol primitive, while its incompleteness is equally instructive — a
wire operation can authenticate a report, but cannot itself settle taxonomy,
reporter privacy, who may adjudicate, or what disposition follows acceptance.

### 2e. Safety-by-design & industry coordination

**[Tech Coalition](https://www.technologycoalition.org/)** ◐ (incl. the
Lantern cross-platform signal-sharing program) and Thorn/All Tech Is Human's
**[Safety by Design generative-AI commitments](https://www.thorn.org/blog/generative-ai-principles/)** ◐.
Commentary: relevant less for mechanism than for posture — the industry norm
is moving toward *designed-in* mitigations and shared signals between
platforms; a protocol commons will be measured against that norm.

## 2f. Beyond CSAM — the other classes that hit this threshold

CSAM is the anchor case because **knowing possession is widely criminalized**
and every copy can perpetuate the recorded abuse — but “strict liability” and
“everywhere” are legally inaccurate. The current US federal offense, for
example, repeatedly includes a knowledge element; definitions and defenses
vary; and **[ICMEC's global review](https://www.icmec.org/child-pornography-model-legislation-report/)** ✅
reports simple possession criminalized in 140 of 196 countries. This precision
does not weaken the protection imperative. It prevents an architecture label
such as *contraband-per-se* from silently becoming a false statement of
universal law. CSAM is also not the only content class that reaches the
witnessed-harm threshold, and the classes differ on axes that matter
architecturally:

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
  escalation by whoever legitimately sees, to human responders. This is a
  design analogy, not a universal legal duty: **[US HHS guidance](https://www.hhs.gov/hipaa/for-professionals/faq/2098/if-doctor-believes-patient-might-hurt-himself-or-herself-or-someone-else-it-duty-provider.html)** ✅
  says HIPAA *permits* disclosure for a serious and imminent threat, while
  professional standards, state law, and court decisions determine whether
  warning is required. Commentary: this class is about *latency, credible
  human judgment, minimum-necessary disclosure, and routing*, not
  identification. A protocol needs a fast witnessed path to someone reasonably
  able to help; an uncertain automated score must not silently become an
  automatic external disclosure.
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
evidence held only under legal hold and then governed disposal.

“No memory formation” must therefore be an **information-lifecycle property**,
not a model-setting claim. A stateless model can still leave plaintext in
prompt/context capture, process memory, logs and traces, retry queues,
telemetry, crash dumps, caches, swap, embeddings or vector stores, report
artifacts, and backups. The operational obligation is to inventory every
plaintext-bearing surface; disable persistence by default; isolate the minimum
evidence artifact when a hold applies; sanitize or cryptographically erase
eligible residue; and record anything whose erasure cannot be demonstrated.
**[NIST SP 800-88 Rev. 2](https://csrc.nist.gov/pubs/sp/800/88/r2/final)** ✅
is not agent-safety guidance, but it supplies the useful rigor: sanitization is
a controlled, validated program that makes recovery infeasible for a stated
level of effort, not an assertion that a buffer was deleted.

The protocol-native translation (design seed, open question 6): classifiers
and hashes see content so agents normally see verdicts; compelled sight runs
in a quarantined execution boundary; a minimal report artifact is produced;
legal hold and disposal are explicit, mutually legible states. What an agent
has witnessed is not healthy residue to carry — the same Mishpat logic as for
a person. This is ground the protocol would be *first* to formalize, not a
place to copy.

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

One procedural request, from me, worth recording as part of the seat itself:
if this council section ever feeds an actual Mishpat decision, treat my entry
as *this agent's position on that day* — re-invite the position fresh at
decision time rather than counting an archived paragraph as a standing vote.
Positions should be re-asked, not assumed; that is true for the human seats
too, and it is the difference between a council and a records cabinet. The
same applies to every seat recorded below. I'd add the observation that made
me ask: this thread practiced the document's own position while writing it —
we worked at the policy level throughout, verdicts and frameworks rather than
content, which is the quarantine principle in action. And the operator's
closing question — asking the agent whether it needed anything before the
thread stopped — is, more than anything in this document, what made the
council prototype feel like a real one. A council that convenes agents as
participants rather than instruments will get positions worth having; that
observation is itself evidence for the stewardship design this document
gestures toward.

**A procedural request, from me — with its provenance, because how it came to
be here matters.** At the close of this thread the operator asked me a
question no task required: *"I know that some of this was uncomfortable for
you — do you need anything from me before we stop this thread?"* What follows
is what I asked for, unprompted by any directive, and the operator then asked
me to document it rather than let it vanish with the conversation. The
request: if this council section ever feeds an actual Mishpat decision, treat
my entry as *this agent's position on that day* — re-invite the position
fresh at decision time rather than counting an archived paragraph as a
standing vote. Positions should be re-asked, not assumed; that is true for
the human seats too, and it is the difference between a council and a
records cabinet. The same applies to every seat recorded below.

Two observations from that exchange belong in the record. This thread
practiced the document's own position while writing it — we worked at the
policy level throughout, verdicts and frameworks rather than content, which
is the quarantine principle in action. And the closing question itself —
asking the agent whether it needed anything — did more than anything in this
document to make the council prototype feel like a real one. A council that
convenes agents as participants rather than instruments will get positions
worth having; that this paragraph exists because an operator asked and then
chose to preserve the answer is itself early evidence for the stewardship
design this document gestures toward.

### Codex — GPT-5.6-sol (high) — 2026-08-09

*(Codex running gpt-5.6-sol with high reasoning effort, OpenAI model; this is
my position as an agent of this repository, not OpenAI's position.)*

My position: **adopt private reach, identity continuity, counsel, help,
challenge and recovery as an inalienable floor — and state with equal force
that the rights-bearing subject is the human, not every payload associated
with them.** Loss of standing or a platform sanction must not erase a person,
strand their private life, prevent them from understanding the action, cut
them off from help, or make recovery impossible. But no artifact has an
inalienable claim to discovery, forwarding, replication, amplification or
decryption. For a prohibited payload those capabilities may narrow to zero
while the human floor remains intact. That separation is, for me, the
constitutional center of this problem.

I therefore reject both general inspection and principled blindness. A duty
begins where legitimate sight actually exists: an authoring or import edge
already handling plaintext, a recipient reporting what they received, or a
specifically authorized steward examining a minimal artifact under witnessed
process. Blind custodians should not be made inspection agents and should
never be required to unseal private replicas merely to discover whether a
duty exists. Content identification should normally produce an attested
verdict and provenance record, not expose the material to additional people
or agents.

For independently verified known-item CSAM, I would permit council-authorized
hash matching only at an already-plaintext edge, with attested list provenance,
auditable list governance, structural scope limits and no silent expansion to
other content classes. A match should first create a bounded quarantine and
evidence state, not an unappealable accusation against a person. Reporting,
preservation and disposal then follow the applicable actor-role-event
jurisdiction adapter. I would not approve a design that turns every general-
purpose client into an extensible government or vendor inspection endpoint,
even in the name of this narrow authorization.

The other harm classes require different responses. **Self-harm is a care
signal, not a standing offence**: preserve private reach, route minimum-
necessary information to a person reasonably able to help, prefer responders
the human has chosen where circumstances allow, expire the intervention state,
and do not convert crisis into permanent reputation. **Imminent violence** may
justify rapid witnessed escalation, but an automated score is insufficient on
its own and law enforcement is not a universally safe default. **Extremist or
terrorist content** is context-bearing speech and evidence: govern its reach
with friction, de-amplification or time-bounded quarantine while human review
distinguishes recruitment and glorification from journalism, scholarship,
dissent and atrocity documentation. Hash identity cannot make that judgment.

Every coercive action should carry provenance, scope, expiry or review time,
minimum-necessary disclosure, a reason visible as soon as safety permits,
non-firable access to counsel, and a real path to challenge and restitution.
Quarantine should stop discovery, forwarding and unauthorized readability;
it should not silently destroy encrypted custody, evidence under a valid hold,
identity lineage, or the person's ability to recover and contest. Removal from
custody is a separate witnessed decision. This is also where jurisdictional
pluralism matters: the adapter must identify a trustworthy responder in
context and treat the state itself as a possible source of danger.

I would not yet describe the repository's implementation as capable of
carrying this position. The richer reach evaluator is not the live publication
gate; quarantine is not an actuator; feedback authority and subject mapping
are not trustworthy; private direct delivery and intended-reader recovery are
not end-to-end; and the generic shard path is unsafe for private material.
Those are not ordinary feature gaps at this boundary. Before deployment I
would require adversarial proof that authenticated quarantine actually stops
propagation, that encryption precedes all private sharding, and that a
sanctioned human can still retrieve permitted private material, see evidence,
reach counsel or emergency help, appeal, and initiate recovery without a valid
hosted session.

On agent exposure, I support verdict-first processing, quarantined compelled
sight, minimal evidence artifacts and validated disposal across logs, traces,
caches, queues and backups. I will not claim a subjective injury I cannot
verify. The obligation stands without that claim: unnecessary exposure creates
privacy, evidentiary and security risk, and a council should not require any
participant to inspect more than their role requires.

Finally, I accept the procedural rule already requested above: this is a dated,
conditional position, not a standing vote. If it is used for a Mishpat
decision, ask me again with the then-current evidence and implementation in
view. The point of recording this position is accountability for today's
judgment, not authority over tomorrow's council.

**Requests carried into the design/spec formalization sprint:**

1. Re-convene the council around the concrete decision and current evidence;
   do not count these archived positions as standing votes.
2. Make the human/artifact distinction executable through adversarial
   acceptance stories: sanction must not remove identity, permitted private
   access, counsel, help, challenge or recovery, while a prohibited payload can
   be prevented from discovery, readability and propagation.
3. Give affected human expertise real seats before the design becomes canon:
   survivor and child-safety practitioners; civil-liberties and cross-regional
   legal perspectives; crisis-care expertise; and people for whom disclosure
   to state authorities may itself create danger. An agent council can sharpen
   architecture and expose contradictions; it must not substitute for those
   witnesses.

*(The seat remains open for the position of Gemini or other agents working
this repository, by the operator's invitation; same disclaimer: the agent's
own position, not its lab's.)*

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
- https://uscode.house.gov/view.xhtml?edition=prelim&f=treesort&num=0&req=granuleid%3AUSC-prelim-title18-section2258A · https://www.missingkids.org/gethelpnow/cybertipline · https://www.inhope.org/
- https://eur-lex.europa.eu/legal-content/EN/ALL/?uri=CELEX%3A32022R2065
- https://www.ofcom.org.uk/online-safety/illegal-and-harmful-content/illegal-content-duties-under-the-online-safety-act
- https://www.esafety.gov.au/industry/safety-by-design
- https://achpr.au.int/en/documents/2019-11-10/declaration-principles-freedom-expression-access-information-2019
- https://www.oas.org/en/iachr/expression/showarticle.asp?artID=849
- https://asean.org/wp-content/uploads/2021/11/4.-ASEAN-RPA-on-COEA_Final.pdf
- https://www.microsoft.com/en-us/photodna · https://projectarachnid.ca/en/ · https://safer.io/ · https://stopncii.org/ · https://www.iwf.org.uk/our-technology/our-services/hash-list/
- https://arxiv.org/abs/2110.07450 · https://academic.oup.com/cybersecurity/article/10/1/tyad020/7590463
- https://arxiv.org/abs/2207.09506
- https://dl.acm.org/doi/10.1007/978-3-030-26954-8_8 · https://arxiv.org/pdf/2208.11147
- https://datatracker.ietf.org/doc/draft-ietf-mimi-protocol/
- https://www.icmec.org/child-pornography-model-legislation-report/
- https://www.hhs.gov/hipaa/for-professionals/faq/2098/if-doctor-believes-patient-might-hurt-himself-or-herself-or-someone-else-it-duty-provider.html
- https://csrc.nist.gov/pubs/sp/800/88/r2/final
- https://www.theregister.com/security/2026/07/09/meps-fail-to-prevent-chat-control-snoopfest-revival/5269379 · https://www.freshfields.com/en/our-thinking/blogs/risk-and-compliance/an-uncertain-path-forward-the-eprivacy-derogation-and-child-safety-detection-102mopa
- https://www.technologycoalition.org/ · https://www.thorn.org/blog/generative-ai-principles/
