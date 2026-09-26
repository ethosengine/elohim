---
offerVersion: 1
id: jenkins-edge-pipeline
collective: .epr-meta/collective.json
provider: collective:ethosengine/elohim
receiver: service:jenkins
author: agent:implementer@claude-opus-5-5
recipe:
  id: edge-pipeline
  version: 1
offered:
  - "recipe-projection: edge-pipeline@1, the edge pipeline declared by the network (.claude/epr-meta/recipes.yaml)"
  - "drift: declared recipe compared with each observed edge build, including undisclosed capabilities"
  - "card: elohim-governance.json, the governance legible at the point of contact"
terms:
  optional: true
  exclusive: false
  withdrawable: true
  claims: none
  settlement: none
crossesNetwork: false
disclosure:
  inflows:
    - presence: presence:jenkins-project
      brings: "the Jenkins automation server and its plugins, maintained by the Jenkins open-source project outside this network"
    - presence: presence:ethosengine-cluster
      brings: "compute, storage and power for the CI agents, paid for outside the network by the operator's own funds"
  heldCapabilities:
    - id: scm:github-token
      what: "the ee-bot-pat credential: clones ethosengine/elohim over HTTPS"
    - id: scm:github-webhook
      what: "receives GitHub push webhooks; the orchestrator decides what runs"
    - id: node:containerd-socket
      what: "builds, tags and prunes images through the node's buildkit and containerd (namespace k8s.io)"
    - id: registry:harbor-push
      what: "the harbor-robot-registry credential: pushes images to harbor.ethosengine.com"
    - id: registry:harbor-pull
      what: "the same credential: pulls the hApp and images it deploys"
    - id: deploy:kube-credentials
      what: "the agent's service account: kubectl apply, rollout restart and scale in the alpha, staging and prod namespaces"
    - id: fleet:probe
      what: "reaches the deployed doorways and storage peers to run Dataplane Validation"
    - id: credential-store:jenkins-secrets
      what: "the Jenkins credential store, which holds every secret above and others no stage names"
  externalities:
    positive:
      - "every push to dev is built, deployed to alpha and validated without a human at the keyboard"
      - "the archived sprint reports and build graphs are the evidence this network reads its own health from"
    negative:
      - "a fleet roll restarts the pods it deploys: about 20 minutes of churn and hours of catch-up per roll"
      - "gate time is duplicated: stages re-run checks the developer's own gate already ran"
      - "the credential store concentrates deploy authority in one service no Steward reviews"
---
# Offer: the edge pipeline, given to Jenkins

The collective `collective:ethosengine/elohim` offers `service:jenkins` three gifts: the edge
pipeline's recipe as the network declares it, drift between that recipe and each build Jenkins
runs, and a governance card it may archive beside its builds. Jenkins may use any of them to get
simpler. Refusing them costs nothing, and the network never waits on Jenkins or needs it.

## Terms

- **Optional, non-exclusive and withdrawable.** Jenkins may ignore every gift. Any other consumer
  may be offered the same. The collective withdraws the offer by the process that approves it: a
  Steward's `changes-requested` verdict against the offer's CID.
- **No claims.** The offer creates no claim for either party. Jenkins' observations of its own
  builds are scoped to this offer's CID, so every flow traces to the governance that allowed it,
  and they discharge nothing.
- **No settlement.** Nothing here converts to token or fiat. A settlement is a separate Agreement,
  a governance act under a verdict from two distinct Stewards who are not fixtures.
- **Active only on a distinct Steward's verdict.** The offer starts Proposed. It becomes Active
  when a Steward of the collective who is not its author (`agent:implementer@claude-opus-5-5`, who
  drafted it) records an approving verdict. An approval by the fixture co-steward `human:adam`
  reads `validated-at: bootstrap (fixture co-steward)` on every surface that shows it: the real
  primitive ran at Bootstrap stakes, and the approval is not network validation.
  `crossesNetwork: false` records that the collective owns this consumer. An offer to a
  consumer it does not own needs two distinct Stewards who are not fixtures.

## Two-sided disclosure

What the network gives is only half of the relationship. The frontmatter also records what Jenkins
brings from outside the network:

- **Inflows** name unclaimed presences, never payees. Recognition for outside labour (the Jenkins
  project's software, the operator's hardware) collects under commons stewardship and redirects to
  that entity if it ever attests. `service:jenkins` itself remains provenance only.
- **Held capabilities** are what Jenkins can do that the network did not grant. The recipe's
  `exercises:` binding names the capability each stage uses. `jenkins-bridge drift` reports a
  capability that a stage exercised and this list does not disclose as `undisclosed capability`.
- **Externalities** are the costs and benefits that fall outside the pipeline itself.

Self-report is only a claim. The network holds its declared understanding of the edge beside what
it observes, and the gap between them is the signal.

## How it is minted, approved and read

```bash
jenkins-bridge offer mint                  # proposes the Intent; prints its CID
jenkins-bridge offer status                # Proposed | Active | Withdrawn, with the stakes
epr flow note --on <offer-cid> --kind verdict --verdict approved \
  --reason "<what you approve>" --session <a Steward's registered session>
jenkins-bridge card --out elohim-governance.json
```

Editing this document changes the offer's address. A Steward approves the exact text they read,
so any edit needs a fresh approval.
