# Banking Bridge Module

## Purpose

This module is an **isolated translation layer** between legacy banking systems (Plaid)
and the Elohim next-generation economy (REA/Holochain).

## Design Principles

1. **Complete Isolation** - No Holochain dependencies. All data is local IndexedDB.
2. **Translation Layer** - Converts bank transactions → REA EconomicEvents
3. **Ephemeral Staging** - Staged data is temporary; only approved events become network signals
4. **Double-Bookkeeping** - Maintains reconciliation between bank records and economic events

## Data Flow

```
Plaid API → PlaidConnection (local) → ImportBatch (local) → StagedTransaction (local)
                                                                      ↓
                                                            [User Review & Approval]
                                                                      ↓
                                            EconomicEvent (Holochain DHT) ← ONLY network signal
```

## What Lives Here (Local IndexedDB)

| Entity | Purpose | Holochain? |
|--------|---------|------------|
| PlaidConnection | OAuth credentials & account mapping | NO - local only |
| ImportBatch | Batch import tracking | NO - local only |
| StagedTransaction | Pre-approval staging | NO - local only |
| TransactionRule | Personal categorization rules | NO - local only |

## What Does NOT Live Here

| Entity | Purpose | Where |
|--------|---------|-------|
| EconomicEvent | Committed value flows | Holochain DHT |
| EconomicResource | Resource inventories | Holochain DHT |
| Agent | Identity | Holochain DHT |

## The Bridge

When a StagedTransaction is approved, the `EconomicEventBridge` service:
1. Transforms the staged transaction → EconomicEvent
2. Calls Holochain zome to create the event
3. Links the event back to the staged transaction (for reconciliation)
4. Marks the staged transaction as completed

## Security

- Plaid access tokens are encrypted with Web Crypto API (AES-GCM)
- Encryption key derived from user passphrase via PBKDF2
- Tokens never leave the device unencrypted
- Tokens are NOT stored on Holochain DHT
