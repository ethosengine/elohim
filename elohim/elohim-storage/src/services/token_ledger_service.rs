//! Token ledger service — balance queries and peer-to-peer transfers.
//!
//! Provides the read/write API over the `token_balances` and `token_transfers`
//! tables.  All mutation is via atomic Diesel operations; no distributed
//! consensus required at this layer (projection is agent-scoped Category B).

use diesel::SqliteConnection;
use sha2::{Digest, Sha256};

use crate::db::context::AppContext;
use crate::db::models::NewTokenTransfer;
use crate::db::token_balances::CreditSource;
use crate::db::{token_balances, token_transfers};
use crate::error::StorageError;
use crate::services::responsibility_demand_service::ResponsibilityDemandService;
use crate::views::{TokenBalanceView, TokenTransferView};

pub struct TokenLedgerService;

impl TokenLedgerService {
    /// Fetch the current balance for an agent in a governance layer.
    ///
    /// Returns a zero-balance view when no row exists yet (agent has never
    /// held tokens in this layer) rather than an error.
    pub fn get_balance(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        agent_id: &str,
        governance_layer: &str,
    ) -> Result<TokenBalanceView, StorageError> {
        match token_balances::get_balance(conn, ctx, agent_id, governance_layer)? {
            Some(bal) => Ok(TokenBalanceView::from(bal)),
            None => {
                let now = chrono::Utc::now().to_rfc3339();
                Ok(TokenBalanceView {
                    agent_id: agent_id.to_string(),
                    h_app_id: ctx.h_app_id.clone(),
                    governance_layer: governance_layer.to_string(),
                    balance: 0.0,
                    total_minted: 0.0,
                    total_transferred_in: 0.0,
                    total_transferred_out: 0.0,
                    last_activity_at: now.clone(),
                    created_at: now,
                })
            }
        }
    }

    /// Fetch all balance rows for an agent across every governance layer.
    pub fn get_all_balances(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        agent_id: &str,
    ) -> Result<Vec<TokenBalanceView>, StorageError> {
        let balances = token_balances::get_all_balances_for_agent(conn, ctx, agent_id)?;
        Ok(balances.into_iter().map(TokenBalanceView::from).collect())
    }

    /// Transfer tokens from one agent to another within a governance layer.
    ///
    /// Atomically debits `from_agent` and credits `to_agent`, then records
    /// an immutable `token_transfers` entry.
    pub fn transfer(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        from_agent: &str,
        to_agent: &str,
        amount: f32,
        governance_layer: &str,
        note: Option<&str>,
    ) -> Result<TokenTransferView, StorageError> {
        if amount <= 0.0 {
            return Err(StorageError::InvalidInput(
                "transfer amount must be positive".into(),
            ));
        }
        if from_agent == to_agent {
            return Err(StorageError::InvalidInput("cannot transfer to self".into()));
        }

        // Check responsibility demand curve
        ResponsibilityDemandService::check_transfer_allowed(
            conn,
            ctx,
            from_agent,
            amount,
            governance_layer,
        )?;

        token_balances::debit_balance(conn, ctx, from_agent, governance_layer, amount)?;
        token_balances::credit_balance(
            conn,
            ctx,
            to_agent,
            governance_layer,
            amount,
            CreditSource::TransferIn,
        )?;

        let transfer_id = Self::generate_transfer_id(from_agent, to_agent, amount);

        let new_transfer = NewTokenTransfer {
            id: &transfer_id,
            h_app_id: &ctx.h_app_id,
            from_agent,
            to_agent,
            amount,
            governance_layer,
            note,
            dht_anchor_hash: None,
        };

        let transfer = token_transfers::create_transfer(conn, ctx, new_transfer)?;
        Ok(TokenTransferView::from(transfer))
    }

    /// Fetch all transfers where the agent is either sender or receiver.
    pub fn get_transfers(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        agent_id: &str,
    ) -> Result<Vec<TokenTransferView>, StorageError> {
        let transfers = token_transfers::get_transfers_for_agent(conn, ctx, agent_id)?;
        Ok(transfers.into_iter().map(TokenTransferView::from).collect())
    }

    /// Generate a unique transfer ID from sender, receiver, amount, and wall-clock millis.
    fn generate_transfer_id(from_agent: &str, to_agent: &str, amount: f32) -> String {
        let now = chrono::Utc::now().timestamp_millis();
        let mut hasher = Sha256::new();
        hasher.update(format!("{}{}{}{}", from_agent, to_agent, amount, now).as_bytes());
        let hash = hasher.finalize();
        format!("txfr-{}-{}", now, hex::encode(&hash[..4]))
    }
}
