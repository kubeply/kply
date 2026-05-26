//! Routing adapters for agent and test traffic isolation.

use std::collections::BTreeSet;

use kply_k8s::{GatewayClassSummary, GatewaySummary, HttpRouteSummary};
use serde::Serialize;

/// Successful or missing Gateway API discovery lists.
#[derive(Clone, Copy, Debug)]
pub struct GatewayApiDiscoveryInput<'a> {
    /// GatewayClass list when the API resource was discovered.
    pub gateway_classes: Option<&'a [GatewayClassSummary]>,
    /// Gateway list when the API resource was discovered.
    pub gateways: Option<&'a [GatewaySummary]>,
    /// HTTPRoute list when the API resource was discovered.
    pub http_routes: Option<&'a [HttpRouteSummary]>,
}

/// Detected Gateway API inventory relevant to temporary session routing.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GatewayApiResourceDetection {
    /// Overall Gateway API resource inventory status.
    pub status: GatewayApiResourceStatus,
    /// Whether GatewayClass could be listed through the Gateway API.
    pub gateway_class_api_detected: bool,
    /// Whether Gateway could be listed through the Gateway API.
    pub gateway_api_detected: bool,
    /// Whether HTTPRoute could be listed through the Gateway API.
    pub http_route_api_detected: bool,
    /// Number of discovered GatewayClass resources.
    pub gateway_class_count: usize,
    /// Number of discovered Gateway resources in the target namespace.
    pub gateway_count: usize,
    /// Number of discovered HTTPRoute resources in the target namespace.
    pub http_route_count: usize,
    /// Discovered GatewayClass controller names in deterministic order.
    pub controller_names: Vec<String>,
    /// Discovered GatewayClass names in deterministic order.
    pub gateway_class_names: Vec<String>,
    /// Discovered Gateway resource names in deterministic order.
    pub gateway_names: Vec<String>,
    /// Discovered HTTPRoute resource names in deterministic order.
    pub http_route_names: Vec<String>,
}

/// Gateway API resource inventory readiness for route planning.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GatewayApiResourceStatus {
    /// No Gateway API resources were discovered.
    Unavailable,
    /// Some Gateway API resources were discovered, but core resources are missing.
    Partial,
    /// GatewayClass and Gateway resources were discovered.
    Available,
}

impl GatewayApiResourceStatus {
    /// Return the stable snake_case string form of this status.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unavailable => "unavailable",
            Self::Partial => "partial",
            Self::Available => "available",
        }
    }
}

/// Supported Gateway API route capabilities for a sandbox session.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GatewayRouteCapabilities {
    /// Gateway API resources used to derive these capabilities.
    pub resources: GatewayApiResourceDetection,
    /// Overall capability status for temporary Gateway API routes.
    pub status: GatewayRouteCapabilityStatus,
    /// Whether temporary HTTPRoute objects can be planned.
    pub supports_temporary_http_routes: bool,
    /// Whether header-based sandbox routing can be planned.
    pub supports_header_based_routing: bool,
    /// Whether host-based preview routing can be planned.
    pub supports_host_based_routing: bool,
    /// Gateway resources with HTTP-compatible listeners in deterministic order.
    pub http_compatible_gateway_names: Vec<String>,
    /// Listener protocols observed on discovered Gateway resources.
    pub listener_protocols: Vec<String>,
    /// Limitations preventing or constraining route planning.
    pub limitations: Vec<GatewayRouteCapabilityLimitation>,
}

/// Gateway route capability readiness.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GatewayRouteCapabilityStatus {
    /// Route capabilities cannot currently be used.
    Unsupported,
    /// Some routing inputs are present, but capability is incomplete.
    Partial,
    /// Required inputs for temporary HTTPRoute planning are present.
    Supported,
}

impl GatewayRouteCapabilityStatus {
    /// Return the stable snake_case string form of this status.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unsupported => "unsupported",
            Self::Partial => "partial",
            Self::Supported => "supported",
        }
    }
}

/// Gateway route capability limitation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GatewayRouteCapabilityLimitation {
    /// No Gateway API resources could be discovered.
    GatewayApiUnavailable,
    /// Gateway API discovery is incomplete.
    GatewayApiPartial,
    /// No GatewayClass resources were found.
    MissingGatewayClass,
    /// No Gateway resources were found.
    MissingGateway,
    /// HTTPRoute API discovery did not succeed.
    MissingHttpRouteApi,
    /// No Gateway listener can accept HTTPRoute traffic.
    NoHttpCompatibleListener,
}

impl GatewayRouteCapabilityLimitation {
    /// Return the stable snake_case string form of this limitation.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GatewayApiUnavailable => "gateway_api_unavailable",
            Self::GatewayApiPartial => "gateway_api_partial",
            Self::MissingGatewayClass => "missing_gateway_class",
            Self::MissingGateway => "missing_gateway",
            Self::MissingHttpRouteApi => "missing_http_route_api",
            Self::NoHttpCompatibleListener => "no_http_compatible_listener",
        }
    }
}

impl GatewayApiResourceDetection {
    /// Return true when inventory can support temporary HTTPRoute planning.
    pub const fn supports_temporary_http_routes(&self) -> bool {
        matches!(self.status, GatewayApiResourceStatus::Available)
    }
}

/// Detect Gateway API routing inventory from Kubernetes discovery summaries.
pub fn detect_gateway_api_resources(
    input: GatewayApiDiscoveryInput<'_>,
) -> GatewayApiResourceDetection {
    let gateway_classes = input.gateway_classes.unwrap_or_default();
    let gateways = input.gateways.unwrap_or_default();
    let http_routes = input.http_routes.unwrap_or_default();

    let gateway_class_api_detected = input.gateway_classes.is_some();
    let gateway_api_detected = input.gateways.is_some();
    let http_route_api_detected = input.http_routes.is_some();

    gateway_api_resource_detection(
        gateway_classes,
        gateways,
        http_routes,
        gateway_class_api_detected,
        gateway_api_detected,
        http_route_api_detected,
    )
}

/// Model supported Gateway API route capabilities from discovery summaries.
pub fn model_gateway_route_capabilities(
    input: GatewayApiDiscoveryInput<'_>,
) -> GatewayRouteCapabilities {
    let gateways = input.gateways.unwrap_or_default();
    let resources = detect_gateway_api_resources(input);
    let http_compatible_gateway_names = http_compatible_gateway_names(gateways);
    let listener_protocols = listener_protocols(gateways);
    let supports_temporary_http_routes =
        resources.supports_temporary_http_routes() && !http_compatible_gateway_names.is_empty();
    let limitations = gateway_route_capability_limitations(
        &resources,
        supports_temporary_http_routes,
        http_compatible_gateway_names.is_empty(),
    );
    let status = gateway_route_capability_status(&resources, supports_temporary_http_routes);

    GatewayRouteCapabilities {
        resources,
        status,
        supports_temporary_http_routes,
        supports_header_based_routing: supports_temporary_http_routes,
        supports_host_based_routing: supports_temporary_http_routes,
        http_compatible_gateway_names,
        listener_protocols,
        limitations,
    }
}

/// Build a deterministic Gateway API resource detection result.
fn gateway_api_resource_detection(
    gateway_classes: &[GatewayClassSummary],
    gateways: &[GatewaySummary],
    http_routes: &[HttpRouteSummary],
    gateway_class_api_detected: bool,
    gateway_api_detected: bool,
    http_route_api_detected: bool,
) -> GatewayApiResourceDetection {
    let gateway_class_names = gateway_classes
        .iter()
        .map(|gateway_class| gateway_class.name.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let controller_names = gateway_classes
        .iter()
        .filter_map(|gateway_class| gateway_class.controller_name.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let gateway_names = gateways
        .iter()
        .map(|gateway| qualified_resource_name(&gateway.namespace, &gateway.name))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let http_route_names = http_routes
        .iter()
        .map(|route| qualified_resource_name(&route.namespace, &route.name))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let gateway_class_count = gateway_classes.len();
    let gateway_count = gateways.len();

    GatewayApiResourceDetection {
        status: gateway_api_resource_status(
            gateway_class_count,
            gateway_count,
            gateway_class_api_detected,
            gateway_api_detected,
            http_route_api_detected,
        ),
        gateway_class_api_detected,
        gateway_api_detected,
        http_route_api_detected,
        gateway_class_count,
        gateway_count,
        http_route_count: http_routes.len(),
        controller_names,
        gateway_class_names,
        gateway_names,
        http_route_names,
    }
}

/// Determine the Gateway API resource inventory status.
fn gateway_api_resource_status(
    gateway_class_count: usize,
    gateway_count: usize,
    gateway_class_api_detected: bool,
    gateway_api_detected: bool,
    http_route_api_detected: bool,
) -> GatewayApiResourceStatus {
    if !gateway_class_api_detected && !gateway_api_detected && !http_route_api_detected {
        return GatewayApiResourceStatus::Unavailable;
    }

    if gateway_class_api_detected
        && gateway_api_detected
        && http_route_api_detected
        && gateway_class_count > 0
        && gateway_count > 0
    {
        return GatewayApiResourceStatus::Available;
    }

    GatewayApiResourceStatus::Partial
}

/// Determine the Gateway route capability status.
fn gateway_route_capability_status(
    resources: &GatewayApiResourceDetection,
    supports_temporary_http_routes: bool,
) -> GatewayRouteCapabilityStatus {
    if supports_temporary_http_routes {
        return GatewayRouteCapabilityStatus::Supported;
    }

    match resources.status {
        GatewayApiResourceStatus::Unavailable => GatewayRouteCapabilityStatus::Unsupported,
        GatewayApiResourceStatus::Partial | GatewayApiResourceStatus::Available => {
            GatewayRouteCapabilityStatus::Partial
        }
    }
}

/// Build stable route capability limitation codes.
fn gateway_route_capability_limitations(
    resources: &GatewayApiResourceDetection,
    supports_temporary_http_routes: bool,
    has_no_http_compatible_gateways: bool,
) -> Vec<GatewayRouteCapabilityLimitation> {
    if supports_temporary_http_routes {
        return Vec::new();
    }

    let mut limitations = Vec::new();
    match resources.status {
        GatewayApiResourceStatus::Unavailable => {
            limitations.push(GatewayRouteCapabilityLimitation::GatewayApiUnavailable);
        }
        GatewayApiResourceStatus::Partial => {
            limitations.push(GatewayRouteCapabilityLimitation::GatewayApiPartial);
        }
        GatewayApiResourceStatus::Available => {}
    }

    if resources.gateway_class_count == 0 {
        limitations.push(GatewayRouteCapabilityLimitation::MissingGatewayClass);
    }
    if resources.gateway_count == 0 {
        limitations.push(GatewayRouteCapabilityLimitation::MissingGateway);
    }
    if !resources.http_route_api_detected {
        limitations.push(GatewayRouteCapabilityLimitation::MissingHttpRouteApi);
    }
    if resources.gateway_count > 0 && has_no_http_compatible_gateways {
        limitations.push(GatewayRouteCapabilityLimitation::NoHttpCompatibleListener);
    }

    limitations
}

/// Return Gateway names with HTTP-compatible listeners.
fn http_compatible_gateway_names(gateways: &[GatewaySummary]) -> Vec<String> {
    gateways
        .iter()
        .filter(|gateway| {
            gateway
                .listeners
                .iter()
                .any(|listener| is_http_compatible_protocol(listener.protocol.as_deref()))
        })
        .map(|gateway| qualified_resource_name(&gateway.namespace, &gateway.name))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Return all discovered listener protocols in deterministic order.
fn listener_protocols(gateways: &[GatewaySummary]) -> Vec<String> {
    gateways
        .iter()
        .flat_map(|gateway| gateway.listeners.iter())
        .filter_map(|listener| listener.protocol.as_deref())
        .map(|protocol| protocol.to_ascii_uppercase())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Return true when a Gateway listener protocol can attach HTTPRoute traffic.
fn is_http_compatible_protocol(protocol: Option<&str>) -> bool {
    protocol.is_some_and(|protocol| {
        protocol.eq_ignore_ascii_case("HTTP") || protocol.eq_ignore_ascii_case("HTTPS")
    })
}

/// Return a stable namespace-qualified resource name.
fn qualified_resource_name(namespace: &str, name: &str) -> String {
    if namespace.is_empty() {
        return name.to_owned();
    }

    format!("{namespace}/{name}")
}

#[cfg(test)]
mod tests {
    use super::{
        GatewayApiDiscoveryInput, GatewayApiResourceDetection, GatewayApiResourceStatus,
        GatewayRouteCapabilities, GatewayRouteCapabilityLimitation, GatewayRouteCapabilityStatus,
        detect_gateway_api_resources, model_gateway_route_capabilities,
    };
    use kply_k8s::{GatewayClassSummary, GatewayListenerSummary, GatewaySummary, HttpRouteSummary};

    #[test]
    /// Detects missing Gateway API inventory as unavailable.
    fn detects_unavailable_gateway_api_resources() {
        let detection = detect_gateway_api_resources(GatewayApiDiscoveryInput {
            gateway_classes: None,
            gateways: None,
            http_routes: None,
        });

        assert_eq!(
            detection,
            GatewayApiResourceDetection {
                status: GatewayApiResourceStatus::Unavailable,
                gateway_class_api_detected: false,
                gateway_api_detected: false,
                http_route_api_detected: false,
                gateway_class_count: 0,
                gateway_count: 0,
                http_route_count: 0,
                controller_names: Vec::new(),
                gateway_class_names: Vec::new(),
                gateway_names: Vec::new(),
                http_route_names: Vec::new(),
            }
        );
        assert!(!detection.supports_temporary_http_routes());
        assert_eq!(detection.status.as_str(), "unavailable");
    }

    #[test]
    /// Detects incomplete Gateway API inventory as partial.
    fn detects_partial_gateway_api_resources() {
        let gateway_classes = vec![gateway_class("istio", Some("istio.io/gateway-controller"))];

        let detection = detect_gateway_api_resources(GatewayApiDiscoveryInput {
            gateway_classes: Some(&gateway_classes),
            gateways: None,
            http_routes: None,
        });

        assert_eq!(detection.status, GatewayApiResourceStatus::Partial);
        assert!(detection.gateway_class_api_detected);
        assert!(!detection.gateway_api_detected);
        assert!(!detection.http_route_api_detected);
        assert!(!detection.supports_temporary_http_routes());
        assert_eq!(detection.gateway_class_names, ["istio"]);
        assert_eq!(detection.controller_names, ["istio.io/gateway-controller"]);
    }

    #[test]
    /// Detects Gateway inventory without GatewayClass objects as partial.
    fn detects_partial_gateway_api_resources_without_gateway_classes() {
        let gateway_classes: Vec<GatewayClassSummary> = Vec::new();
        let gateways = vec![gateway("shop", "public", "istio")];
        let http_routes: Vec<HttpRouteSummary> = Vec::new();

        let detection = detect_gateway_api_resources(GatewayApiDiscoveryInput {
            gateway_classes: Some(&gateway_classes),
            gateways: Some(&gateways),
            http_routes: Some(&http_routes),
        });

        assert_eq!(detection.status, GatewayApiResourceStatus::Partial);
        assert!(detection.gateway_class_api_detected);
        assert!(detection.gateway_api_detected);
        assert!(detection.http_route_api_detected);
        assert!(!detection.supports_temporary_http_routes());
    }

    #[test]
    /// Detects usable Gateway API inventory as available.
    fn detects_available_gateway_api_resources() {
        let gateway_classes = vec![
            gateway_class("istio", Some("istio.io/gateway-controller")),
            gateway_class(
                "envoy",
                Some("gateway.envoyproxy.io/gatewayclass-controller"),
            ),
            gateway_class("istio", Some("istio.io/gateway-controller")),
        ];
        let gateways = vec![
            gateway("shop", "public", "istio"),
            gateway("platform", "internal", "envoy"),
        ];
        let http_routes: Vec<HttpRouteSummary> = Vec::new();

        let detection = detect_gateway_api_resources(GatewayApiDiscoveryInput {
            gateway_classes: Some(&gateway_classes),
            gateways: Some(&gateways),
            http_routes: Some(&http_routes),
        });

        assert_eq!(detection.status, GatewayApiResourceStatus::Available);
        assert!(detection.gateway_class_api_detected);
        assert!(detection.gateway_api_detected);
        assert!(detection.http_route_api_detected);
        assert!(detection.supports_temporary_http_routes());
        assert_eq!(
            detection.controller_names,
            [
                "gateway.envoyproxy.io/gatewayclass-controller",
                "istio.io/gateway-controller"
            ]
        );
        assert_eq!(detection.gateway_class_names, ["envoy", "istio"]);
        assert_eq!(
            detection.gateway_names,
            ["platform/internal", "shop/public"]
        );
        assert_eq!(detection.http_route_names, Vec::<String>::new());
    }

    #[test]
    /// Models unavailable Gateway API route capabilities.
    fn models_unavailable_gateway_route_capabilities() {
        let capabilities = model_gateway_route_capabilities(GatewayApiDiscoveryInput {
            gateway_classes: None,
            gateways: None,
            http_routes: None,
        });

        assert_eq!(
            capabilities,
            GatewayRouteCapabilities {
                resources: GatewayApiResourceDetection {
                    status: GatewayApiResourceStatus::Unavailable,
                    gateway_class_api_detected: false,
                    gateway_api_detected: false,
                    http_route_api_detected: false,
                    gateway_class_count: 0,
                    gateway_count: 0,
                    http_route_count: 0,
                    controller_names: Vec::new(),
                    gateway_class_names: Vec::new(),
                    gateway_names: Vec::new(),
                    http_route_names: Vec::new(),
                },
                status: GatewayRouteCapabilityStatus::Unsupported,
                supports_temporary_http_routes: false,
                supports_header_based_routing: false,
                supports_host_based_routing: false,
                http_compatible_gateway_names: Vec::new(),
                listener_protocols: Vec::new(),
                limitations: vec![
                    GatewayRouteCapabilityLimitation::GatewayApiUnavailable,
                    GatewayRouteCapabilityLimitation::MissingGatewayClass,
                    GatewayRouteCapabilityLimitation::MissingGateway,
                    GatewayRouteCapabilityLimitation::MissingHttpRouteApi,
                ],
            }
        );
        assert_eq!(capabilities.status.as_str(), "unsupported");
        assert_eq!(
            capabilities.limitations[0].as_str(),
            "gateway_api_unavailable"
        );
    }

    #[test]
    /// Models Gateway API route capabilities without HTTP listeners as partial.
    fn models_partial_gateway_route_capabilities_without_http_listeners() {
        let gateway_classes = vec![gateway_class("mesh", Some("example.com/controller"))];
        let gateways = vec![gateway_with_protocol("shop", "mesh", "mesh", Some("TCP"))];
        let http_routes: Vec<HttpRouteSummary> = Vec::new();

        let capabilities = model_gateway_route_capabilities(GatewayApiDiscoveryInput {
            gateway_classes: Some(&gateway_classes),
            gateways: Some(&gateways),
            http_routes: Some(&http_routes),
        });

        assert_eq!(capabilities.status, GatewayRouteCapabilityStatus::Partial);
        assert!(!capabilities.supports_temporary_http_routes);
        assert!(!capabilities.supports_header_based_routing);
        assert!(!capabilities.supports_host_based_routing);
        assert_eq!(capabilities.listener_protocols, ["TCP"]);
        assert_eq!(
            capabilities.http_compatible_gateway_names,
            Vec::<String>::new()
        );
        assert_eq!(
            capabilities.limitations,
            [GatewayRouteCapabilityLimitation::NoHttpCompatibleListener]
        );
    }

    #[test]
    /// Models supported Gateway API route capabilities.
    fn models_supported_gateway_route_capabilities() {
        let gateway_classes = vec![gateway_class("istio", Some("istio.io/gateway-controller"))];
        let gateways = vec![
            gateway_with_protocol("shop", "public", "istio", Some("HTTP")),
            gateway_with_protocol("shop", "secure", "istio", Some("HTTPS")),
            gateway_with_protocol("shop", "tcp", "istio", Some("TCP")),
        ];
        let http_routes: Vec<HttpRouteSummary> = Vec::new();

        let capabilities = model_gateway_route_capabilities(GatewayApiDiscoveryInput {
            gateway_classes: Some(&gateway_classes),
            gateways: Some(&gateways),
            http_routes: Some(&http_routes),
        });

        assert_eq!(capabilities.status, GatewayRouteCapabilityStatus::Supported);
        assert!(capabilities.supports_temporary_http_routes);
        assert!(capabilities.supports_header_based_routing);
        assert!(capabilities.supports_host_based_routing);
        assert_eq!(capabilities.listener_protocols, ["HTTP", "HTTPS", "TCP"]);
        assert_eq!(
            capabilities.http_compatible_gateway_names,
            ["shop/public", "shop/secure"]
        );
        assert_eq!(capabilities.limitations, Vec::new());
    }

    /// Build a GatewayClass summary fixture.
    fn gateway_class(name: &str, controller_name: Option<&str>) -> GatewayClassSummary {
        GatewayClassSummary {
            name: name.to_owned(),
            controller_name: controller_name.map(ToOwned::to_owned),
            description: None,
        }
    }

    /// Build an HTTP Gateway summary fixture.
    fn gateway(namespace: &str, name: &str, gateway_class_name: &str) -> GatewaySummary {
        gateway_with_protocol(namespace, name, gateway_class_name, Some("HTTP"))
    }

    /// Build a Gateway summary fixture with one listener protocol.
    fn gateway_with_protocol(
        namespace: &str,
        name: &str,
        gateway_class_name: &str,
        protocol: Option<&str>,
    ) -> GatewaySummary {
        GatewaySummary {
            namespace: namespace.to_owned(),
            name: name.to_owned(),
            gateway_class_name: Some(gateway_class_name.to_owned()),
            listeners: vec![GatewayListenerSummary {
                name: Some("http".to_owned()),
                hostname: None,
                port: Some(80),
                protocol: protocol.map(ToOwned::to_owned),
            }],
        }
    }
}
