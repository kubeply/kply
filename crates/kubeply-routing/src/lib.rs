use kubeply_core::RouteHeader;

/// Routing backend supported by Kubeply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingBackend {
    GatewayApi,
    NginxIngress,
    FallbackPreview,
}

/// Route plan for agent/test traffic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutePlan {
    pub backend: RoutingBackend,
    pub header: Option<RouteHeader>,
}

impl RoutePlan {
    #[must_use]
    pub fn new(backend: RoutingBackend, header: Option<RouteHeader>) -> Self {
        Self { backend, header }
    }
}
