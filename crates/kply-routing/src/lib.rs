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
        detect_gateway_api_resources,
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

    fn gateway_class(name: &str, controller_name: Option<&str>) -> GatewayClassSummary {
        GatewayClassSummary {
            name: name.to_owned(),
            controller_name: controller_name.map(ToOwned::to_owned),
            description: None,
        }
    }

    fn gateway(namespace: &str, name: &str, gateway_class_name: &str) -> GatewaySummary {
        GatewaySummary {
            namespace: namespace.to_owned(),
            name: name.to_owned(),
            gateway_class_name: Some(gateway_class_name.to_owned()),
            listeners: vec![GatewayListenerSummary {
                name: Some("http".to_owned()),
                hostname: None,
                port: Some(80),
                protocol: Some("HTTP".to_owned()),
            }],
        }
    }
}
