//! Executor - Action execution engine
//!
//! Manages the action queue and executes actions with proper error handling,
//! logging, and result tracking.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

use super::actions;
use super::models::*;

/// Maximum pending actions in queue
const MAX_QUEUE_SIZE: usize = 100;

/// Action executor with priority queue
pub struct Executor {
    node_id: String,
    /// Pending actions by priority
    queue: Arc<RwLock<VecDeque<Action>>>,
    /// Action history (completed/failed)
    history: Arc<RwLock<VecDeque<Action>>>,
    /// Actions in progress
    in_progress: Arc<RwLock<HashMap<ActionId, Action>>>,
    /// Action handlers
    handlers: ActionHandlers,
    /// Actions executed count
    executed_count: Arc<RwLock<u64>>,
}

/// Trait for action execution
#[async_trait::async_trait]
pub trait ActionHandler: Send + Sync {
    async fn execute(&self, action: &Action) -> ActionResult;
    fn can_handle(&self, kind: &ActionKind) -> bool;
}

/// Collection of action handlers
pub struct ActionHandlers {
    handlers: Vec<Box<dyn ActionHandler>>,
}

impl ActionHandlers {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    pub fn register(&mut self, handler: Box<dyn ActionHandler>) {
        self.handlers.push(handler);
    }

    pub fn find_handler(&self, kind: &ActionKind) -> Option<&dyn ActionHandler> {
        self.handlers
            .iter()
            .find(|h| h.can_handle(kind))
            .map(|h| h.as_ref())
    }
}

impl Default for ActionHandlers {
    fn default() -> Self {
        let mut handlers = Self::new();

        // Register default handlers
        handlers.register(Box::new(actions::ConfigActionHandler));
        handlers.register(Box::new(actions::DebugActionHandler));
        handlers.register(Box::new(actions::CacheActionHandler));
        handlers.register(Box::new(actions::StorageActionHandler));
        handlers.register(Box::new(actions::RecoveryActionHandler));

        handlers
    }
}

impl Executor {
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            queue: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_QUEUE_SIZE))),
            history: Arc::new(RwLock::new(VecDeque::with_capacity(100))),
            in_progress: Arc::new(RwLock::new(HashMap::new())),
            handlers: ActionHandlers::default(),
            executed_count: Arc::new(RwLock::new(0)),
        }
    }

    /// Queue an action for execution
    pub async fn queue(&self, action: Action) -> Result<ActionId, String> {
        let mut queue = self.queue.write().await;

        if queue.len() >= MAX_QUEUE_SIZE {
            return Err("Action queue is full".to_string());
        }

        // Check if this action requires consensus
        if matches!(action.status, ActionStatus::PendingConsensus) {
            info!(
                action_id = %action.id,
                kind = ?action.kind,
                "Action queued pending consensus"
            );
        } else {
            info!(
                action_id = %action.id,
                kind = ?action.kind,
                "Action queued for execution"
            );
        }

        let id = action.id.clone();
        queue.push_back(action);

        Ok(id)
    }

    /// Get pending action count
    pub async fn pending_count(&self) -> usize {
        self.queue.read().await.len()
    }

    /// Get executed action count
    pub async fn executed_count(&self) -> u64 {
        *self.executed_count.read().await
    }

    /// Get action by ID
    pub async fn get_action(&self, id: &str) -> Option<Action> {
        // Check queue
        {
            let queue = self.queue.read().await;
            if let Some(action) = queue.iter().find(|a| a.id == id) {
                return Some(action.clone());
            }
        }

        // Check in progress
        {
            let in_progress = self.in_progress.read().await;
            if let Some(action) = in_progress.get(id) {
                return Some(action.clone());
            }
        }

        // Check history
        {
            let history = self.history.read().await;
            if let Some(action) = history.iter().find(|a| a.id == id) {
                return Some(action.clone());
            }
        }

        None
    }

    /// Get recent actions from history
    pub async fn get_history(&self, count: usize) -> Vec<Action> {
        let history = self.history.read().await;
        history.iter().rev().take(count).cloned().collect()
    }

    /// Mark an action as approved (after consensus)
    pub async fn approve(&self, id: &str) -> Result<(), String> {
        let mut queue = self.queue.write().await;

        if let Some(action) = queue.iter_mut().find(|a| a.id == id) {
            if matches!(action.status, ActionStatus::PendingConsensus) {
                action.status = ActionStatus::Queued;
                info!(action_id = %id, "Action approved for execution");
                Ok(())
            } else {
                Err(format!("Action {} is not pending consensus", id))
            }
        } else {
            Err(format!("Action {} not found in queue", id))
        }
    }

    /// Reject an action (after consensus failure)
    pub async fn reject(&self, id: &str, reason: &str) -> Result<(), String> {
        let mut queue = self.queue.write().await;

        if let Some(pos) = queue.iter().position(|a| a.id == id) {
            let mut action = queue.remove(pos).unwrap();
            action.status = ActionStatus::Rejected;
            action.result = Some(ActionResult {
                success: false,
                message: format!("Rejected: {}", reason),
                duration_ms: 0,
                details: None,
            });

            let mut history = self.history.write().await;
            history.push_back(action);

            warn!(action_id = %id, reason, "Action rejected");
            Ok(())
        } else {
            Err(format!("Action {} not found in queue", id))
        }
    }

    /// Cancel a pending action
    pub async fn cancel(&self, id: &str) -> Result<(), String> {
        let mut queue = self.queue.write().await;

        if let Some(pos) = queue.iter().position(|a| a.id == id) {
            let mut action = queue.remove(pos).unwrap();
            action.status = ActionStatus::Cancelled;
            action.result = Some(ActionResult {
                success: false,
                message: "Cancelled by operator".to_string(),
                duration_ms: 0,
                details: None,
            });

            let mut history = self.history.write().await;
            history.push_back(action);

            info!(action_id = %id, "Action cancelled");
            Ok(())
        } else {
            Err(format!("Action {} not found in queue", id))
        }
    }

    /// Execute the next queued action
    pub async fn execute_next(&self) -> Option<ActionResult> {
        // Find next executable action
        let action = {
            let mut queue = self.queue.write().await;

            // Find first action that's ready to execute
            let pos = queue
                .iter()
                .position(|a| matches!(a.status, ActionStatus::Queued));

            if let Some(pos) = pos {
                let mut action = queue.remove(pos).unwrap();
                action.status = ActionStatus::InProgress;
                Some(action)
            } else {
                None
            }
        };

        let action = match action {
            Some(a) => a,
            None => return None,
        };

        // Track in progress
        {
            let mut in_progress = self.in_progress.write().await;
            in_progress.insert(action.id.clone(), action.clone());
        }

        info!(
            action_id = %action.id,
            kind = ?action.kind,
            "Executing action"
        );

        // Execute with timing
        let start = Instant::now();
        let result = self.execute_action(&action).await;
        let duration_ms = start.elapsed().as_millis() as u64;

        // Update result with timing
        let result = ActionResult {
            duration_ms,
            ..result
        };

        // Move to history
        {
            let mut in_progress = self.in_progress.write().await;
            in_progress.remove(&action.id);
        }

        {
            let mut history = self.history.write().await;
            let mut completed = action.clone();
            completed.status = if result.success {
                ActionStatus::Completed
            } else {
                ActionStatus::Failed
            };
            completed.result = Some(result.clone());

            history.push_back(completed);

            // Trim history
            while history.len() > 100 {
                history.pop_front();
            }
        }

        // Update count
        {
            let mut count = self.executed_count.write().await;
            *count += 1;
        }

        if result.success {
            info!(
                action_id = %action.id,
                duration_ms,
                "Action completed successfully"
            );
        } else {
            error!(
                action_id = %action.id,
                duration_ms,
                error = %result.message,
                "Action failed"
            );
        }

        Some(result)
    }

    /// Execute an action using the appropriate handler
    async fn execute_action(&self, action: &Action) -> ActionResult {
        // Find handler
        let handler = match self.handlers.find_handler(&action.kind) {
            Some(h) => h,
            None => {
                return ActionResult {
                    success: false,
                    message: format!("No handler for action kind {:?}", action.kind),
                    duration_ms: 0,
                    details: None,
                };
            }
        };

        // Execute
        handler.execute(action).await
    }

    /// Process all queued actions (for batch execution)
    pub async fn process_queue(&self) -> Vec<ActionResult> {
        let mut results = Vec::new();

        loop {
            match self.execute_next().await {
                Some(result) => results.push(result),
                None => break,
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_queue_action() {
        let executor = Executor::new("test-node".to_string());

        let action = Action::new(
            ActionKind::SetLogLevel,
            "Test action",
            serde_json::json!({"level": "debug"}),
        );

        let id = executor.queue(action).await.unwrap();
        assert!(!id.is_empty());
        assert_eq!(executor.pending_count().await, 1);
    }

    #[tokio::test]
    async fn test_execute_action() {
        let executor = Executor::new("test-node".to_string());

        let action = Action::new(
            ActionKind::SetLogLevel,
            "Set log level to debug",
            serde_json::json!({"level": "debug", "module": "elohim_node"}),
        );

        executor.queue(action).await.unwrap();

        let result = executor.execute_next().await;
        assert!(result.is_some());

        assert_eq!(executor.pending_count().await, 0);
        assert_eq!(executor.executed_count().await, 1);
    }
}
