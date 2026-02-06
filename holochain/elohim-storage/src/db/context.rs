//! App context for multi-tenant database operations
//!
//! All database operations are scoped by app_id to enable multiple apps
//! to store content in the same database without interference.

/// App context passed to all database operations for multi-tenant isolation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AppContext {
    /// Application identifier for scoping database operations
    pub app_id: String,
}

impl AppContext {
    /// Create a new app context with the specified app ID
    pub fn new(app_id: impl Into<String>) -> Self {
        Self {
            app_id: app_id.into(),
        }
    }

    /// Default context for learning content (paths, concepts, quizzes)
    pub fn default_lamad() -> Self {
        Self::new("lamad")
    }

    /// Default context for shared infrastructure (resources, sensemaking)
    pub fn default_elohim() -> Self {
        Self::new("elohim")
    }

    /// Get the app_id as a string reference
    pub fn app_id(&self) -> &str {
        &self.app_id
    }
}

impl Default for AppContext {
    /// Defaults to lamad for backwards compatibility with existing content
    fn default() -> Self {
        Self::default_lamad()
    }
}

impl std::fmt::Display for AppContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AppContext({})", self.app_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_contexts() {
        assert_eq!(AppContext::default_lamad().app_id, "lamad");
        assert_eq!(AppContext::default_elohim().app_id, "elohim");
        assert_eq!(AppContext::default().app_id, "lamad");
    }

    #[test]
    fn test_custom_context() {
        let ctx = AppContext::new("calendar");
        assert_eq!(ctx.app_id, "calendar");
    }
}
