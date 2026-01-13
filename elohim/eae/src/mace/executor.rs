//! Executor component - executes actions from decisions.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Semaphore};
use tracing::{debug, error, info, warn};

use crate::config::ExecutorConfig;
use crate::types::{Action, ActionType, Decision, EaeError, ExecutionResult, Result};

/// Handler trait for action execution.
#[async_trait::async_trait]
pub trait ActionHandler: Send + Sync {
    /// Execute an action.
    async fn execute(&self, action: &Action) -> Result<serde_json::Value>;

    /// Action types this handler supports.
    fn supported_types(&self) -> Vec<ActionType>;
}

/// Default handler for logging actions.
pub struct LoggingHandler;

#[async_trait::async_trait]
impl ActionHandler for LoggingHandler {
    async fn execute(&self, action: &Action) -> Result<serde_json::Value> {
        info!(
            action_id = %action.id,
            action_type = ?action.action_type,
            target = %action.target,
            "Logging action executed"
        );
        Ok(serde_json::json!({
            "logged": true,
            "action_id": action.id,
        }))
    }

    fn supported_types(&self) -> Vec<ActionType> {
        vec![ActionType::Log]
    }
}

/// Default handler for notifications.
pub struct NotificationHandler;

#[async_trait::async_trait]
impl ActionHandler for NotificationHandler {
    async fn execute(&self, action: &Action) -> Result<serde_json::Value> {
        let message = action
            .params
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("No message");

        info!(
            action_id = %action.id,
            target = %action.target,
            message = %message,
            "Notification sent"
        );

        Ok(serde_json::json!({
            "notified": true,
            "target": action.target,
            "message": message,
        }))
    }

    fn supported_types(&self) -> Vec<ActionType> {
        vec![ActionType::Notify]
    }
}

/// Executor for running actions.
pub struct Executor {
    /// Configuration
    config: ExecutorConfig,
    /// Registered action handlers
    handlers: Arc<RwLock<HashMap<ActionType, Arc<dyn ActionHandler>>>>,
    /// Concurrency limiter
    semaphore: Arc<Semaphore>,
    /// Execution history
    history: Arc<RwLock<Vec<ExecutionResult>>>,
}

impl Executor {
    /// Create a new executor with default configuration.
    pub fn new() -> Self {
        Self::with_config(ExecutorConfig::default())
    }

    /// Create with custom configuration.
    pub fn with_config(config: ExecutorConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent));
        let executor = Self {
            config,
            handlers: Arc::new(RwLock::new(HashMap::new())),
            semaphore,
            history: Arc::new(RwLock::new(Vec::new())),
        };

        // Register default handlers
        tokio::spawn({
            let handlers = Arc::clone(&executor.handlers);
            async move {
                let mut h = handlers.write().await;
                h.insert(ActionType::Log, Arc::new(LoggingHandler));
                h.insert(ActionType::Notify, Arc::new(NotificationHandler));
            }
        });

        executor
    }

    /// Create a builder.
    pub fn builder() -> ExecutorBuilder {
        ExecutorBuilder::new()
    }

    /// Register an action handler.
    pub async fn register_handler(&self, handler: Arc<dyn ActionHandler>) {
        let mut handlers = self.handlers.write().await;
        for action_type in handler.supported_types() {
            handlers.insert(action_type, Arc::clone(&handler));
        }
    }

    /// Execute all actions from a decision.
    pub async fn execute(&self, decision: &Decision) -> Vec<ExecutionResult> {
        let mut results = Vec::new();

        for action in &decision.actions {
            let result = self.execute_action(action).await;
            results.push(result);
        }

        // Store in history
        {
            let mut history = self.history.write().await;
            for result in &results {
                history.push(result.clone());
            }

            // Keep history bounded
            while history.len() > 10_000 {
                history.remove(0);
            }
        }

        results
    }

    /// Execute a single action.
    pub async fn execute_action(&self, action: &Action) -> ExecutionResult {
        let start = std::time::Instant::now();

        // Acquire semaphore permit
        let _permit = match self.semaphore.acquire().await {
            Ok(permit) => permit,
            Err(_) => {
                return ExecutionResult {
                    action_id: action.id.clone(),
                    success: false,
                    result: None,
                    error: Some("Failed to acquire execution permit".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                    timestamp: chrono::Utc::now(),
                };
            }
        };

        debug!(
            action_id = %action.id,
            action_type = ?action.action_type,
            "Executing action"
        );

        // Find handler
        let handlers = self.handlers.read().await;
        let handler = handlers.get(&action.action_type);

        let result = if let Some(handler) = handler {
            // Execute with timeout
            let timeout = tokio::time::Duration::from_millis(self.config.action_timeout_ms);
            let handler = Arc::clone(handler);
            let execution_result: std::result::Result<
                std::result::Result<serde_json::Value, crate::types::EaeError>,
                tokio::time::error::Elapsed,
            > = tokio::time::timeout(timeout, handler.execute(action)).await;

            match execution_result {
                Ok(Ok(value)) => ExecutionResult {
                    action_id: action.id.clone(),
                    success: true,
                    result: Some(value),
                    error: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                    timestamp: chrono::Utc::now(),
                },
                Ok(Err(e)) => {
                    error!(action_id = %action.id, error = %e, "Action execution failed");
                    ExecutionResult {
                        action_id: action.id.clone(),
                        success: false,
                        result: None,
                        error: Some(e.to_string()),
                        duration_ms: start.elapsed().as_millis() as u64,
                        timestamp: chrono::Utc::now(),
                    }
                }
                Err(_) => {
                    warn!(action_id = %action.id, "Action execution timed out");
                    ExecutionResult {
                        action_id: action.id.clone(),
                        success: false,
                        result: None,
                        error: Some("Execution timed out".to_string()),
                        duration_ms: start.elapsed().as_millis() as u64,
                        timestamp: chrono::Utc::now(),
                    }
                }
            }
        } else {
            warn!(
                action_id = %action.id,
                action_type = ?action.action_type,
                "No handler registered for action type"
            );
            ExecutionResult {
                action_id: action.id.clone(),
                success: false,
                result: None,
                error: Some(format!(
                    "No handler registered for action type {:?}",
                    action.action_type
                )),
                duration_ms: start.elapsed().as_millis() as u64,
                timestamp: chrono::Utc::now(),
            }
        };

        info!(
            action_id = %action.id,
            success = result.success,
            duration_ms = result.duration_ms,
            "Action execution completed"
        );

        result
    }

    /// Execute with retries.
    pub async fn execute_with_retry(&self, action: &Action) -> ExecutionResult {
        let mut result = self.execute_action(action).await;

        for attempt in 0..self.config.retry_count {
            if result.success {
                break;
            }

            debug!(
                action_id = %action.id,
                attempt = attempt + 1,
                "Retrying action"
            );

            tokio::time::sleep(tokio::time::Duration::from_millis(
                self.config.retry_delay_ms * (attempt as u64 + 1),
            ))
            .await;

            result = self.execute_action(action).await;
        }

        result
    }

    /// Get recent execution results.
    pub async fn recent_results(&self, limit: usize) -> Vec<ExecutionResult> {
        let history = self.history.read().await;
        history.iter().rev().take(limit).cloned().collect()
    }

    /// Get execution statistics.
    pub async fn stats(&self) -> ExecutorStats {
        let history = self.history.read().await;

        let total = history.len();
        let successful = history.iter().filter(|r| r.success).count();
        let failed = total - successful;
        let avg_duration_ms = if total > 0 {
            history.iter().map(|r| r.duration_ms).sum::<u64>() / total as u64
        } else {
            0
        };

        ExecutorStats {
            total_executions: total,
            successful,
            failed,
            avg_duration_ms,
        }
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for executor.
#[derive(Debug, Clone)]
pub struct ExecutorStats {
    /// Total executions
    pub total_executions: usize,
    /// Successful executions
    pub successful: usize,
    /// Failed executions
    pub failed: usize,
    /// Average duration
    pub avg_duration_ms: u64,
}

/// Builder for Executor configuration.
pub struct ExecutorBuilder {
    config: ExecutorConfig,
    handlers: Vec<Arc<dyn ActionHandler>>,
}

impl ExecutorBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            config: ExecutorConfig::default(),
            handlers: Vec::new(),
        }
    }

    /// Set max concurrent executions.
    pub fn max_concurrent(mut self, max: usize) -> Self {
        self.config.max_concurrent = max;
        self
    }

    /// Set action timeout.
    pub fn timeout_ms(mut self, timeout: u64) -> Self {
        self.config.action_timeout_ms = timeout;
        self
    }

    /// Set retry count.
    pub fn retry_count(mut self, count: usize) -> Self {
        self.config.retry_count = count;
        self
    }

    /// Add a handler.
    pub fn with_handler(mut self, handler: Arc<dyn ActionHandler>) -> Self {
        self.handlers.push(handler);
        self
    }

    /// Build the executor.
    pub async fn build(self) -> Executor {
        let executor = Executor::with_config(self.config);
        for handler in self.handlers {
            executor.register_handler(handler).await;
        }
        executor
    }
}

impl Default for ExecutorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_executor_logging() {
        let executor = Executor::new();

        // Give default handlers time to register
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let action = Action::new(ActionType::Log, "test-target");
        let result = executor.execute_action(&action).await;

        assert!(result.success);
    }

    #[tokio::test]
    async fn test_executor_unknown_handler() {
        let executor = Executor::new();

        // Give default handlers time to register
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let action = Action::new(ActionType::FilterContent, "test");
        let result = executor.execute_action(&action).await;

        assert!(!result.success);
        assert!(result.error.is_some());
    }
}
