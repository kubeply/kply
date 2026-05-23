use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Stable identifier for a Kply session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(Uuid);

impl SessionId {
    /// Create a new random session id.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Kubernetes workload targeted by a session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkloadRef {
    pub namespace: String,
    pub name: String,
}

impl WorkloadRef {
    #[must_use]
    pub fn new(namespace: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            name: name.into(),
        }
    }
}

/// Header used to identify agent/test traffic for a session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteHeader {
    pub name: String,
    pub value: String,
}

impl RouteHeader {
    #[must_use]
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

/// Lifecycle status for a Kply session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SessionStatus {
    Planned,
    Active,
    Verified,
    Blocked,
    CleanedUp,
}

/// Runtime check attached to a session plan or report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckPlan {
    pub name: String,
    pub description: String,
}

impl CheckPlan {
    #[must_use]
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
        }
    }
}

/// Planned safe workspace for an agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionPlan {
    pub id: SessionId,
    pub workload: WorkloadRef,
    pub image: String,
    pub route_header: Option<RouteHeader>,
    pub status: SessionStatus,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    pub checks: Vec<CheckPlan>,
    pub dry_run: bool,
}

impl SessionPlan {
    #[must_use]
    pub fn new(
        workload: WorkloadRef,
        image: impl Into<String>,
        route_header: Option<RouteHeader>,
        checks: Vec<CheckPlan>,
        dry_run: bool,
    ) -> Self {
        Self {
            id: SessionId::new(),
            workload,
            image: image.into(),
            route_header,
            status: SessionStatus::Planned,
            created_at: OffsetDateTime::now_utc(),
            checks,
            dry_run,
        }
    }
}

/// Human-facing summary that remains stable enough for agents to parse.
#[must_use]
pub fn render_human_plan(plan: &SessionPlan) -> String {
    let mut output = String::new();
    output.push_str("Kply session plan\n");
    output.push_str(&format!("  id: {}\n", plan.id));
    output.push_str(&format!("  workload: {}\n", plan.workload.name));
    output.push_str(&format!("  namespace: {}\n", plan.workload.namespace));
    output.push_str(&format!("  image: {}\n", plan.image));
    output.push_str(&format!("  status: {:?}\n", plan.status));
    output.push_str(&format!("  dry_run: {}\n", plan.dry_run));

    if let Some(route_header) = &plan.route_header {
        output.push_str(&format!(
            "  route_header: {}={}\n",
            route_header.name, route_header.value
        ));
    }

    output.push_str("  checks:\n");
    for check in &plan.checks {
        output.push_str(&format!("    - {}: {}\n", check.name, check.description));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::{CheckPlan, RouteHeader, SessionPlan, WorkloadRef, render_human_plan};

    #[test]
    fn human_plan_mentions_route_header() {
        let plan = SessionPlan::new(
            WorkloadRef::new("shop", "backend-api"),
            "ghcr.io/acme/backend:fix",
            Some(RouteHeader::new("x-kply-session", "fix-123")),
            vec![CheckPlan::new("pod-starts", "Sandbox pods should start")],
            true,
        );

        let rendered = render_human_plan(&plan);

        assert!(rendered.contains("route_header: x-kply-session=fix-123"));
    }
}
