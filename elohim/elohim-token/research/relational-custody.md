# Relational Custody: How Elohim Hold Secrets

## The Insight

Traditional cryptographic custody protects secrets through math — shard a key, distribute fragments, reconstruct from threshold. Get 4 of 7 shards and you're in. No judgment. No context. Just math.

Relational custody protects secrets through relationship. Multiple elohim hold the *whole* secret. The security isn't that the password is split — it's that **you have to convince the elohim to give it to you**, and the elohim evaluate your request against the constitution.

## Why This Is Stronger

A sharded key has a fixed reconstruction threshold. An attacker who obtains enough shards bypasses all judgment. There's a mathematical backdoor around the system's values.

An elohim that holds the whole secret can:
- Ask *why* you want it
- Evaluate the request against constitutional values
- Consider the current network state (are there people below the dignity floor who depend on this pool?)
- Say "no" with reasoning
- Say "yes, but only because there's no network to serve yet" (the $100 test)
- Coordinate with other elohim before releasing (consensus, not threshold)

The security comes from the same place trust comes from in a healthy community — relationships with entities that have judgment, not locks that have combinations.

## The Architecture

1. **Multiple elohim hold the whole secret** — encrypted in their private memory, stored on the DHT as encrypted blobs. Resilient through replication, not fragmentation.

2. **Access requires going *through* the elohim, not *around* them** — there's no mathematical backdoor. You present your case. The elohim evaluate it. The constitution governs the response.

3. **The distributed part is resilience, not security** — if one elohim node goes down, others still hold the secret. You don't lose the password. But the redundancy serves availability, not access control. Access control is judgment.

4. **The security model is the communication/relationship** — knowing who to ask, and having to get through constitutional evaluation to receive. This is the "you can't sue God" architecture made literal for custody.

## The Household Analogy

A family knows the combination to the safe. The security isn't the lock — it's the family's judgment about when to open it. A child can ask. A stranger can ask. The family evaluates each request differently, based on relationship, context, need, and values.

Scale that to a network of elohim, each with constitutional constraints, each capable of evaluating requests, and you have custodial security that's both resilient and wise.

## Private Memory on the DHT

The elohim's private memory lives on the DHT for resilience:
- **Encrypted blob** → gossipped, replicated, visible to everyone, readable by no one
- **Elohim agent key** → cryptographically distinct from human agent keys (imagodei DNA distinguishes agent types)
- **Multiple elohim** → each holds the whole secret, encrypted under their own key
- **Small footprint** — bank passwords, API keys, wallet seeds are bytes to kilobytes

When the elohim needs the secret:
1. Fetches its own encrypted blob from DHT
2. Decrypts in ephemeral memory
3. Uses the secret
4. Zeroes the cleartext immediately

The secret exists in plaintext for milliseconds, in one process's memory. The resilience lives in the DHT. The protection lives in the elohim's constitutional judgment.

## Prerequisites

- Elohim agent DID distinct from human agent DID (imagodei DNA)
- DHT entry type for encrypted elohim private memory (small, agent-scoped)
- Constitutional framework for what categories of secrets elohim may hold (qahal consented)
- Elohim inference capability to evaluate custodial requests (elohim-agent-service)

## Connection to EAE

Without relational custody, the Elohim Autonomous Entity is a fiction — an "autonomous" entity that can't actually hold anything. With it, the EAE can be a genuine custodian:
- Hold commons pool credentials
- Manage settlement bridge keys
- Custody unclaimed attribution value
- Execute the CompletedGift — irrevocable release of value into constitutional governance

The 21st human shows up, and the elohim *actually* has the bank password, distributed across a network no single person controls, accessible only through constitutional negotiation.
