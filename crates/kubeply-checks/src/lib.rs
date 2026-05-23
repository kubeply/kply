use kubeply_core::CheckPlan;

/// Default checks attached to the first safe-session workflow.
#[must_use]
pub fn default_session_checks() -> Vec<CheckPlan> {
    vec![
        CheckPlan::new("pod-starts", "Sandbox pods should start successfully"),
        CheckPlan::new("service-reachable", "Sandbox service should be reachable"),
        CheckPlan::new("logs-clean", "Sandbox logs should not show fatal errors"),
        CheckPlan::new(
            "cleanup-ready",
            "Temporary session resources should be removable",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::default_session_checks;

    #[test]
    fn default_checks_include_cleanup() {
        let checks = default_session_checks();

        assert!(checks.iter().any(|check| check.name == "cleanup-ready"));
    }
}
