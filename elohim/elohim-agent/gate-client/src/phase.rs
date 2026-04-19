//! Phase marker for dev-context vs elohim-active execution.
//!
//! All definitions live in `gate-types::phase`; this module re-exports them
//! for backward compatibility with any code importing `gate_client::phase::*`.

pub use gate_types::phase::Phase;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_context_is_dev_context() {
        assert!(Phase::DevContext.is_dev_context());
        assert!(!Phase::DevContext.is_elohim_active());
    }

    #[test]
    fn elohim_active_is_elohim_active() {
        assert!(Phase::ElohimActive.is_elohim_active());
        assert!(!Phase::ElohimActive.is_dev_context());
    }

    #[test]
    fn default_phase_is_dev_context() {
        assert_eq!(Phase::default(), Phase::DevContext);
    }
}
