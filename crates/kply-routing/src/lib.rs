//! Routing adapters for agent and test traffic isolation.

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use kply_core::{
    CLEANUP_SELECTOR_LABELS, KubernetesResourceRef, MetadataEntry, REQUIRED_OWNERSHIP_LABELS,
    RouteSelector, SAFE_APP_LABELS,
};
use kply_k8s::{GatewayClassSummary, GatewaySummary, HttpRouteSummary, IngressSummary};
use serde::Serialize;

const GATEWAY_API_VERSION: &str = "gateway.networking.k8s.io/v1";
const HTTP_ROUTE_KIND: &str = "HTTPRoute";
const GATEWAY_KIND: &str = "Gateway";
const INGRESS_API_VERSION: &str = "networking.k8s.io/v1";
const INGRESS_KIND: &str = "Ingress";
const SERVICE_KIND: &str = "Service";
const NGINX_CANARY_ANNOTATION: &str = "nginx.ingress.kubernetes.io/canary";
const NGINX_CANARY_BY_HEADER_ANNOTATION: &str = "nginx.ingress.kubernetes.io/canary-by-header";
const NGINX_CANARY_BY_HEADER_VALUE_ANNOTATION: &str =
    "nginx.ingress.kubernetes.io/canary-by-header-value";

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

/// Successful or missing discovery inputs for route strategy capability detection.
#[derive(Clone, Copy, Debug)]
pub struct RoutingCapabilityInput<'a> {
    /// Gateway API discovery inputs.
    pub gateway_api: GatewayApiDiscoveryInput<'a>,
    /// Ingress list when the API resource was discovered.
    pub ingresses: Option<&'a [IngressSummary]>,
    /// Whether preview Service fallback checks are explicitly available.
    pub preview_service_enabled: bool,
}

/// Detected routing capabilities across supported and fallback route strategies.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RoutingCapabilityDetection {
    /// Gateway API temporary route capability details.
    pub gateway_api: GatewayRouteCapabilities,
    /// Ingress inventory relevant to future ingress-based route planning.
    pub ingress: IngressRouteDetection,
    /// Whether direct preview Service checks can be used as an explicit fallback.
    pub preview_service_available: bool,
    /// Route strategies with enough detected input to be considered.
    pub candidate_strategies: Vec<RouteStrategy>,
    /// Limitations that explain why candidates are fallback-only or incomplete.
    pub limitations: Vec<RoutingCapabilityLimitation>,
}

/// Detected Ingress inventory relevant to future route planning.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IngressRouteDetection {
    /// Whether the Ingress API could be listed.
    pub ingress_api_detected: bool,
    /// Number of discovered Ingress resources in the target namespace.
    pub ingress_count: usize,
    /// Discovered Ingress resource names in deterministic order.
    pub ingress_names: Vec<String>,
    /// Discovered IngressClass names in deterministic order.
    pub ingress_class_names: Vec<String>,
    /// Hostnames discovered from Ingress rules and TLS blocks.
    pub hostnames: Vec<String>,
    /// Backend Service names referenced by discovered Ingress resources.
    pub backend_service_names: Vec<String>,
}

/// Route strategy detected as usable or worth considering.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteStrategy {
    /// Gateway API HTTPRoute strategy.
    GatewayApi,
    /// Kubernetes Ingress strategy for future controller-specific adapters.
    Ingress,
    /// Direct preview Service strategy for agent-only checks.
    PreviewService,
}

impl RouteStrategy {
    /// Return the stable snake_case string form of this strategy.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GatewayApi => "gateway_api",
            Self::Ingress => "ingress",
            Self::PreviewService => "preview_service",
        }
    }
}

/// Limitation discovered while modeling route strategy capabilities.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingCapabilityLimitation {
    /// Gateway API did not expose enough resources for temporary HTTPRoute routing.
    GatewayApiUnavailable,
    /// Gateway API is present but incomplete for temporary HTTPRoute routing.
    GatewayApiPartial,
    /// Ingress objects were not available in the target namespace.
    IngressUnavailable,
    /// Ingress resources exist, but controller-specific route planning is future work.
    IngressPlanningNotImplemented,
    /// Preview Service checks bypass edge routing behavior.
    PreviewServiceBypassesEdgeRouting,
}

impl RoutingCapabilityLimitation {
    /// Return the stable snake_case string form of this limitation.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GatewayApiUnavailable => "gateway_api_unavailable",
            Self::GatewayApiPartial => "gateway_api_partial",
            Self::IngressUnavailable => "ingress_unavailable",
            Self::IngressPlanningNotImplemented => "ingress_planning_not_implemented",
            Self::PreviewServiceBypassesEdgeRouting => "preview_service_bypasses_edge_routing",
        }
    }
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

/// Input for generating a temporary Gateway API HTTPRoute manifest.
#[derive(Clone, Copy, Debug)]
pub struct GatewayHttpRouteManifestInput<'a> {
    /// Planned HTTPRoute resource to create for the sandbox session.
    pub route: &'a KubernetesResourceRef,
    /// Gateway parent that should receive the temporary route attachment.
    pub parent_gateway: &'a KubernetesResourceRef,
    /// Service backend that should receive matching sandbox traffic.
    pub backend_service: &'a KubernetesResourceRef,
    /// Backend Service port used by the temporary route.
    pub backend_port: u16,
    /// Request selector that isolates sandbox traffic from normal users.
    pub selector: &'a RouteSelector,
    /// Metadata labels to attach to the temporary HTTPRoute.
    pub labels: &'a [MetadataEntry],
    /// Metadata annotations to attach to the temporary HTTPRoute.
    pub annotations: &'a [MetadataEntry],
}

/// Input for planning a temporary ingress-nginx canary Ingress.
#[derive(Clone, Copy, Debug)]
pub struct NginxIngressRoutePlanInput<'a> {
    /// Existing NGINX Ingress whose HTTP rules should be mirrored.
    pub source_ingress: &'a IngressSummary,
    /// Planned canary Ingress resource to create for the sandbox session.
    pub route: &'a KubernetesResourceRef,
    /// Service backend that should receive matching sandbox traffic.
    pub backend_service: &'a KubernetesResourceRef,
    /// Backend Service port used by the temporary route.
    pub backend_port: &'a str,
    /// Request selector that isolates sandbox traffic from normal users.
    pub selector: &'a RouteSelector,
    /// Metadata labels to attach to the temporary Ingress.
    pub labels: &'a [MetadataEntry],
    /// Metadata annotations to attach to the temporary Ingress.
    pub annotations: &'a [MetadataEntry],
}

/// Temporary Gateway API HTTPRoute manifest for a sandbox session.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayHttpRouteManifest {
    /// Gateway API version for the HTTPRoute resource.
    pub api_version: &'static str,
    /// Kubernetes resource kind for this manifest.
    pub kind: &'static str,
    /// Kubernetes metadata for the generated HTTPRoute.
    pub metadata: GatewayHttpRouteMetadata,
    /// Gateway API HTTPRoute specification.
    pub spec: GatewayHttpRouteSpec,
}

/// Temporary ingress-nginx canary Ingress manifest for a sandbox session.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxIngressCanaryManifest {
    /// Kubernetes API version for the Ingress resource.
    pub api_version: &'static str,
    /// Kubernetes resource kind for this manifest.
    pub kind: &'static str,
    /// Kubernetes metadata for the generated canary Ingress.
    pub metadata: NginxIngressCanaryMetadata,
    /// Kubernetes Ingress specification for mirrored canary routing.
    pub spec: NginxIngressCanarySpec,
}

/// Kubernetes metadata for a temporary Gateway API HTTPRoute.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GatewayHttpRouteMetadata {
    /// Namespace that contains the temporary HTTPRoute.
    pub namespace: String,
    /// Name of the temporary HTTPRoute.
    pub name: String,
    /// Labels attached to the temporary HTTPRoute.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub labels: BTreeMap<String, String>,
    /// Annotations attached to the temporary HTTPRoute.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub annotations: BTreeMap<String, String>,
}

/// Kubernetes metadata for a temporary ingress-nginx canary Ingress.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct NginxIngressCanaryMetadata {
    /// Namespace that contains the temporary canary Ingress.
    pub namespace: String,
    /// Name of the temporary canary Ingress.
    pub name: String,
    /// Labels attached to the temporary canary Ingress.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub labels: BTreeMap<String, String>,
    /// Annotations attached to the temporary canary Ingress.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub annotations: BTreeMap<String, String>,
}

/// Gateway API HTTPRoute spec for a sandbox session.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayHttpRouteSpec {
    /// Parent Gateway references that should attach this route.
    pub parent_refs: Vec<GatewayHttpRouteParentRef>,
    /// Hostnames matched by this route when host preview routing is used.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub hostnames: Vec<String>,
    /// Routing rules that forward matching traffic to the sandbox backend.
    pub rules: Vec<GatewayHttpRouteRule>,
}

/// Kubernetes Ingress spec for a temporary ingress-nginx canary route.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxIngressCanarySpec {
    /// IngressClass copied from the source NGINX Ingress.
    pub ingress_class_name: String,
    /// HTTP host/path rules mirrored from the source Ingress.
    pub rules: Vec<NginxIngressCanaryRule>,
}

/// Gateway API parent reference for a temporary HTTPRoute.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayHttpRouteParentRef {
    /// Referenced parent kind.
    pub kind: &'static str,
    /// Referenced parent Gateway name.
    pub name: String,
    /// Referenced parent Gateway namespace when it differs from the route.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
}

/// Host rule for a temporary ingress-nginx canary route.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct NginxIngressCanaryRule {
    /// Optional host matched by the source Ingress rule.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    /// HTTP paths for the mirrored source rule.
    pub http: NginxIngressCanaryHttpRule,
}

/// HTTP rule body for a temporary ingress-nginx canary route.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct NginxIngressCanaryHttpRule {
    /// HTTP paths mirrored from the source Ingress.
    pub paths: Vec<NginxIngressCanaryPath>,
}

/// HTTP path for a temporary ingress-nginx canary route.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxIngressCanaryPath {
    /// Optional path copied from the source Ingress path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Path type copied from the source Ingress path.
    pub path_type: String,
    /// Sandbox backend that receives matched canary traffic.
    pub backend: NginxIngressCanaryBackend,
}

/// Backend for a temporary ingress-nginx canary route.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct NginxIngressCanaryBackend {
    /// Service backend for matched traffic.
    pub service: NginxIngressCanaryBackendService,
}

/// Service backend for a temporary ingress-nginx canary route.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct NginxIngressCanaryBackendService {
    /// Backend Service name.
    pub name: String,
    /// Backend Service port.
    pub port: NginxIngressCanaryServicePort,
}

/// Service port for a temporary ingress-nginx canary route.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum NginxIngressCanaryServicePort {
    /// Numeric Service port.
    Number {
        /// Service port number.
        number: u16,
    },
    /// Named Service port.
    Name {
        /// Service port name.
        name: String,
    },
}

/// Gateway API HTTPRoute rule for sandbox traffic.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayHttpRouteRule {
    /// Request matches that should select sandbox traffic.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub matches: Vec<GatewayHttpRouteMatch>,
    /// Service backends receiving the matched traffic.
    pub backend_refs: Vec<GatewayHttpRouteBackendRef>,
}

/// Gateway API HTTPRoute match for sandbox traffic.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GatewayHttpRouteMatch {
    /// Header matchers for header-isolated sandbox traffic.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<GatewayHttpRouteHeaderMatch>,
}

/// Gateway API HTTPRoute header matcher.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GatewayHttpRouteHeaderMatch {
    /// Gateway API header match type.
    #[serde(rename = "type")]
    pub match_type: &'static str,
    /// HTTP header name to match.
    pub name: String,
    /// HTTP header value to match exactly.
    pub value: String,
}

/// Gateway API HTTPRoute backend reference.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayHttpRouteBackendRef {
    /// Referenced backend kind.
    pub kind: &'static str,
    /// Referenced backend Service name.
    pub name: String,
    /// Referenced backend Service port.
    pub port: u16,
}

/// Cleanup target for a generated Gateway API HTTPRoute.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayHttpRouteCleanupTarget {
    /// Gateway API version for the HTTPRoute resource.
    pub api_version: &'static str,
    /// Kubernetes resource kind for this cleanup target.
    pub kind: &'static str,
    /// Namespace containing the temporary HTTPRoute.
    pub namespace: String,
    /// Name of the temporary HTTPRoute.
    pub name: String,
    /// [`GatewayRouteCleanupSelector`] that must match before cleanup deletes the route.
    pub selector: GatewayRouteCleanupSelector,
}

/// Kubernetes label selector used to clean up Gateway API routes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GatewayRouteCleanupSelector {
    /// Required ownership labels for route cleanup.
    #[serde(rename = "matchLabels")]
    pub match_labels: BTreeMap<String, String>,
}

/// Error returned while generating a temporary Gateway API HTTPRoute manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GatewayHttpRouteManifestError {
    /// The planned route resource is not an HTTPRoute.
    RouteKind { kind: String },
    /// The parent resource is not a Gateway.
    ParentKind { kind: String },
    /// The backend resource is not a Service.
    BackendKind { kind: String },
    /// The backend Service requires unsupported cross-namespace routing.
    BackendNamespace {
        route_namespace: String,
        backend_namespace: String,
    },
    /// Header-based routing is not supported by the modeled Gateway capabilities.
    HeaderRoutingUnavailable,
    /// The route selector is not a header selector.
    HeaderSelectorRequired { kind: String },
    /// Host-based preview routing is not supported by the modeled Gateway capabilities.
    HostRoutingUnavailable,
    /// The route selector is not a host selector.
    HostSelectorRequired { kind: String },
    /// A required route ownership label is missing.
    MissingOwnershipLabel { key: &'static str },
}

/// Error returned while planning a temporary ingress-nginx canary Ingress.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NginxIngressRoutePlanError {
    /// The planned route resource is not an Ingress.
    RouteKind { kind: String },
    /// The backend resource is not a Service.
    BackendKind { kind: String },
    /// The backend Service requires unsupported cross-namespace routing.
    BackendNamespace {
        route_namespace: String,
        backend_namespace: String,
    },
    /// The source Ingress is not explicitly owned by ingress-nginx.
    UnsupportedIngressClass { class_name: Option<String> },
    /// The route selector is not a header selector.
    HeaderSelectorRequired { kind: String },
    /// The source Ingress does not have HTTP paths to mirror.
    MissingHttpPaths,
    /// The backend Service port is empty or invalid.
    BackendPort { port: String },
    /// A required route ownership label is missing.
    MissingOwnershipLabel { key: &'static str },
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

impl fmt::Display for GatewayHttpRouteManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RouteKind { kind } => {
                write!(formatter, "expected HTTPRoute route resource, found {kind}")
            }
            Self::ParentKind { kind } => {
                write!(formatter, "expected Gateway parent resource, found {kind}")
            }
            Self::BackendKind { kind } => {
                write!(formatter, "expected Service backend resource, found {kind}")
            }
            Self::BackendNamespace {
                route_namespace,
                backend_namespace,
            } => write!(
                formatter,
                "HTTPRoute backend service must be in route namespace {route_namespace}, found {backend_namespace}"
            ),
            Self::HeaderRoutingUnavailable => {
                write!(
                    formatter,
                    "Gateway capabilities do not support header-based routing"
                )
            }
            Self::HeaderSelectorRequired { kind } => {
                write!(formatter, "expected header route selector, found {kind}")
            }
            Self::HostRoutingUnavailable => {
                write!(
                    formatter,
                    "Gateway capabilities do not support host-based preview routing"
                )
            }
            Self::HostSelectorRequired { kind } => {
                write!(formatter, "expected host route selector, found {kind}")
            }
            Self::MissingOwnershipLabel { key } => {
                write!(
                    formatter,
                    "Gateway route is missing ownership label `{key}`"
                )
            }
        }
    }
}

impl Error for GatewayHttpRouteManifestError {}

impl fmt::Display for NginxIngressRoutePlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RouteKind { kind } => {
                write!(formatter, "expected Ingress route resource, found {kind}")
            }
            Self::BackendKind { kind } => {
                write!(formatter, "expected Service backend resource, found {kind}")
            }
            Self::BackendNamespace {
                route_namespace,
                backend_namespace,
            } => write!(
                formatter,
                "Ingress backend service must be in route namespace {route_namespace}, found {backend_namespace}"
            ),
            Self::UnsupportedIngressClass { class_name } => match class_name {
                Some(class_name) => {
                    write!(
                        formatter,
                        "expected ingress-nginx IngressClass, found {class_name}"
                    )
                }
                None => write!(
                    formatter,
                    "expected explicit ingress-nginx IngressClass, found none"
                ),
            },
            Self::HeaderSelectorRequired { kind } => {
                write!(formatter, "expected header route selector, found {kind}")
            }
            Self::MissingHttpPaths => write!(
                formatter,
                "source Ingress has no HTTP paths to mirror for canary routing"
            ),
            Self::BackendPort { port } => {
                write!(formatter, "backend Service port is invalid: {port}")
            }
            Self::MissingOwnershipLabel { key } => {
                write!(
                    formatter,
                    "NGINX canary Ingress is missing ownership label `{key}`"
                )
            }
        }
    }
}

impl Error for NginxIngressRoutePlanError {}

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

/// Detect routing capabilities across Gateway API, Ingress, and explicit fallbacks.
pub fn detect_routing_capabilities(
    input: RoutingCapabilityInput<'_>,
) -> RoutingCapabilityDetection {
    let preview_service_available = detect_preview_service_available(&input);
    let gateway_api = model_gateway_route_capabilities(input.gateway_api);
    let ingress = detect_ingress_routes(input.ingresses);
    let candidate_strategies =
        routing_candidate_strategies(&gateway_api, &ingress, preview_service_available);
    let limitations =
        routing_capability_limitations(&gateway_api, &ingress, preview_service_available);

    RoutingCapabilityDetection {
        gateway_api,
        ingress,
        preview_service_available,
        candidate_strategies,
        limitations,
    }
}

/// Generate a temporary Gateway API HTTPRoute manifest for sandbox traffic.
pub fn generate_gateway_http_route_manifest(
    input: GatewayHttpRouteManifestInput<'_>,
) -> Result<GatewayHttpRouteManifest, GatewayHttpRouteManifestError> {
    validate_gateway_http_route_manifest_input(input)?;

    Ok(GatewayHttpRouteManifest {
        api_version: GATEWAY_API_VERSION,
        kind: HTTP_ROUTE_KIND,
        metadata: GatewayHttpRouteMetadata {
            namespace: input.route.namespace().to_owned(),
            name: input.route.name().to_owned(),
            labels: route_labels(input.labels)?,
            annotations: metadata_entries_to_map(input.annotations),
        },
        spec: GatewayHttpRouteSpec {
            parent_refs: vec![GatewayHttpRouteParentRef {
                kind: GATEWAY_KIND,
                name: input.parent_gateway.name().to_owned(),
                namespace: parent_gateway_namespace(input.route, input.parent_gateway),
            }],
            hostnames: selector_hostnames(input.selector),
            rules: vec![GatewayHttpRouteRule {
                matches: selector_matches(input.selector),
                backend_refs: vec![GatewayHttpRouteBackendRef {
                    kind: SERVICE_KIND,
                    name: input.backend_service.name().to_owned(),
                    port: input.backend_port,
                }],
            }],
        },
    })
}

/// Generate a capability-gated header-based Gateway API HTTPRoute manifest.
pub fn generate_gateway_header_http_route_manifest(
    capabilities: &GatewayRouteCapabilities,
    input: GatewayHttpRouteManifestInput<'_>,
) -> Result<GatewayHttpRouteManifest, GatewayHttpRouteManifestError> {
    validate_gateway_header_route_support(capabilities, input.selector)?;
    generate_gateway_http_route_manifest(input)
}

/// Generate a capability-gated host-based Gateway API HTTPRoute manifest.
pub fn generate_gateway_host_http_route_manifest(
    capabilities: &GatewayRouteCapabilities,
    input: GatewayHttpRouteManifestInput<'_>,
) -> Result<GatewayHttpRouteManifest, GatewayHttpRouteManifestError> {
    validate_gateway_host_route_support(capabilities, input.selector)?;
    generate_gateway_http_route_manifest(input)
}

/// Generate a temporary ingress-nginx canary Ingress manifest for sandbox traffic.
///
/// The generated manifest mirrors source Ingress host/path rules and adds
/// ingress-nginx canary-by-header annotations so only matching agent traffic
/// reaches the sandbox backend.
pub fn generate_nginx_ingress_canary_manifest(
    input: NginxIngressRoutePlanInput<'_>,
) -> Result<NginxIngressCanaryManifest, NginxIngressRoutePlanError> {
    validate_nginx_ingress_route_plan_input(input)?;
    let (header_name, header_value) = input.selector.header_parts().ok_or_else(|| {
        NginxIngressRoutePlanError::HeaderSelectorRequired {
            kind: input.selector.kind().to_owned(),
        }
    })?;

    Ok(NginxIngressCanaryManifest {
        api_version: INGRESS_API_VERSION,
        kind: INGRESS_KIND,
        metadata: NginxIngressCanaryMetadata {
            namespace: input.route.namespace().to_owned(),
            name: input.route.name().to_owned(),
            labels: nginx_ingress_labels(input.labels)?,
            annotations: nginx_canary_annotations(input.annotations, header_name, header_value),
        },
        spec: NginxIngressCanarySpec {
            ingress_class_name: input
                .source_ingress
                .ingress_class_name
                .clone()
                .expect("validated nginx ingress class should be present"),
            rules: nginx_canary_rules(
                input.source_ingress,
                input.backend_service,
                input.backend_port,
            )?,
        },
    })
}

/// Generate a [`GatewayHttpRouteCleanupTarget`] for a temporary Gateway API HTTPRoute.
///
/// The cleanup target is derived from a [`KubernetesResourceRef`] and ownership
/// [`MetadataEntry`] labels.
pub fn gateway_http_route_cleanup_target(
    route: &KubernetesResourceRef,
    labels: &[MetadataEntry],
) -> Result<GatewayHttpRouteCleanupTarget, GatewayHttpRouteManifestError> {
    if route.kind() != HTTP_ROUTE_KIND {
        return Err(GatewayHttpRouteManifestError::RouteKind {
            kind: route.kind().to_owned(),
        });
    }

    Ok(GatewayHttpRouteCleanupTarget {
        api_version: GATEWAY_API_VERSION,
        kind: HTTP_ROUTE_KIND,
        namespace: route.namespace().to_owned(),
        name: route.name().to_owned(),
        selector: gateway_route_cleanup_selector(labels)?,
    })
}

/// Generate the minimal [`GatewayRouteCleanupSelector`] required for Gateway API route cleanup.
///
/// The selector is derived from ownership [`MetadataEntry`] labels.
pub fn gateway_route_cleanup_selector(
    labels: &[MetadataEntry],
) -> Result<GatewayRouteCleanupSelector, GatewayHttpRouteManifestError> {
    let labels = metadata_entries_to_map(labels);
    ensure_route_ownership_labels(&labels)?;

    Ok(GatewayRouteCleanupSelector {
        match_labels: cleanup_labels(&labels),
    })
}

/// Validate the typed inputs used to generate a Gateway API HTTPRoute.
fn validate_gateway_http_route_manifest_input(
    input: GatewayHttpRouteManifestInput<'_>,
) -> Result<(), GatewayHttpRouteManifestError> {
    if input.route.kind() != HTTP_ROUTE_KIND {
        return Err(GatewayHttpRouteManifestError::RouteKind {
            kind: input.route.kind().to_owned(),
        });
    }
    if input.parent_gateway.kind() != GATEWAY_KIND {
        return Err(GatewayHttpRouteManifestError::ParentKind {
            kind: input.parent_gateway.kind().to_owned(),
        });
    }
    if input.backend_service.kind() != SERVICE_KIND {
        return Err(GatewayHttpRouteManifestError::BackendKind {
            kind: input.backend_service.kind().to_owned(),
        });
    }
    if input.route.namespace() != input.backend_service.namespace() {
        return Err(GatewayHttpRouteManifestError::BackendNamespace {
            route_namespace: input.route.namespace().to_owned(),
            backend_namespace: input.backend_service.namespace().to_owned(),
        });
    }

    Ok(())
}

/// Validate typed inputs used to plan an ingress-nginx canary Ingress.
fn validate_nginx_ingress_route_plan_input(
    input: NginxIngressRoutePlanInput<'_>,
) -> Result<(), NginxIngressRoutePlanError> {
    if input.route.kind() != INGRESS_KIND {
        return Err(NginxIngressRoutePlanError::RouteKind {
            kind: input.route.kind().to_owned(),
        });
    }
    if input.backend_service.kind() != SERVICE_KIND {
        return Err(NginxIngressRoutePlanError::BackendKind {
            kind: input.backend_service.kind().to_owned(),
        });
    }
    if input.route.namespace() != input.backend_service.namespace() {
        return Err(NginxIngressRoutePlanError::BackendNamespace {
            route_namespace: input.route.namespace().to_owned(),
            backend_namespace: input.backend_service.namespace().to_owned(),
        });
    }
    if !is_nginx_ingress_class(input.source_ingress.ingress_class_name.as_deref()) {
        return Err(NginxIngressRoutePlanError::UnsupportedIngressClass {
            class_name: input.source_ingress.ingress_class_name.clone(),
        });
    }
    if input.selector.header_parts().is_none() {
        return Err(NginxIngressRoutePlanError::HeaderSelectorRequired {
            kind: input.selector.kind().to_owned(),
        });
    }
    if service_port(input.backend_port).is_none() {
        return Err(NginxIngressRoutePlanError::BackendPort {
            port: input.backend_port.to_owned(),
        });
    }
    if !ingress_has_http_paths(input.source_ingress) {
        return Err(NginxIngressRoutePlanError::MissingHttpPaths);
    }

    Ok(())
}

/// Return the safe label set for generated Gateway API routes.
fn route_labels(
    labels: &[MetadataEntry],
) -> Result<BTreeMap<String, String>, GatewayHttpRouteManifestError> {
    let labels = metadata_entries_to_map(labels);
    ensure_route_ownership_labels(&labels)?;

    Ok(labels
        .into_iter()
        .filter(|(key, _)| should_preserve_route_label(key))
        .collect())
}

/// Return the safe label set for generated ingress-nginx canary routes.
fn nginx_ingress_labels(
    labels: &[MetadataEntry],
) -> Result<BTreeMap<String, String>, NginxIngressRoutePlanError> {
    let labels = metadata_entries_to_map(labels);
    ensure_nginx_ingress_ownership_labels(&labels)?;

    Ok(labels
        .into_iter()
        .filter(|(key, _)| should_preserve_route_label(key))
        .collect())
}

/// Ensure all route ownership labels are present.
fn ensure_route_ownership_labels(
    labels: &BTreeMap<String, String>,
) -> Result<(), GatewayHttpRouteManifestError> {
    for key in REQUIRED_OWNERSHIP_LABELS {
        if !labels.contains_key(key) {
            return Err(GatewayHttpRouteManifestError::MissingOwnershipLabel { key });
        }
    }

    Ok(())
}

/// Ensure all NGINX canary Ingress ownership labels are present.
fn ensure_nginx_ingress_ownership_labels(
    labels: &BTreeMap<String, String>,
) -> Result<(), NginxIngressRoutePlanError> {
    for key in REQUIRED_OWNERSHIP_LABELS {
        if !labels.contains_key(key) {
            return Err(NginxIngressRoutePlanError::MissingOwnershipLabel { key });
        }
    }

    Ok(())
}

/// Return true when a label should be preserved on generated Gateway routes.
fn should_preserve_route_label(key: &str) -> bool {
    REQUIRED_OWNERSHIP_LABELS.contains(&key) || SAFE_APP_LABELS.contains(&key)
}

/// Return only ownership labels required for route cleanup selectors.
fn cleanup_labels(labels: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    labels
        .iter()
        .filter(|(key, _)| CLEANUP_SELECTOR_LABELS.contains(&key.as_str()))
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
}

/// Validate that Gateway capabilities allow a header-based route selector.
fn validate_gateway_header_route_support(
    capabilities: &GatewayRouteCapabilities,
    selector: &RouteSelector,
) -> Result<(), GatewayHttpRouteManifestError> {
    if !capabilities.supports_header_based_routing {
        return Err(GatewayHttpRouteManifestError::HeaderRoutingUnavailable);
    }
    if selector.header_parts().is_none() {
        return Err(GatewayHttpRouteManifestError::HeaderSelectorRequired {
            kind: selector.kind().to_owned(),
        });
    }

    Ok(())
}

/// Validate that Gateway capabilities allow a host-based route selector.
fn validate_gateway_host_route_support(
    capabilities: &GatewayRouteCapabilities,
    selector: &RouteSelector,
) -> Result<(), GatewayHttpRouteManifestError> {
    if !capabilities.supports_host_based_routing {
        return Err(GatewayHttpRouteManifestError::HostRoutingUnavailable);
    }
    if selector.hostname().is_none() {
        return Err(GatewayHttpRouteManifestError::HostSelectorRequired {
            kind: selector.kind().to_owned(),
        });
    }

    Ok(())
}

/// Convert metadata entries into deterministic Kubernetes metadata maps.
fn metadata_entries_to_map(entries: &[MetadataEntry]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|entry| (entry.key().to_owned(), entry.value().to_owned()))
        .collect()
}

/// Return ingress-nginx canary annotations for header-selected sandbox traffic.
fn nginx_canary_annotations(
    annotations: &[MetadataEntry],
    header_name: &str,
    header_value: &str,
) -> BTreeMap<String, String> {
    let mut annotations = metadata_entries_to_map(annotations);
    annotations.insert(NGINX_CANARY_ANNOTATION.to_owned(), "true".to_owned());
    annotations.insert(
        NGINX_CANARY_BY_HEADER_ANNOTATION.to_owned(),
        header_name.to_owned(),
    );
    annotations.insert(
        NGINX_CANARY_BY_HEADER_VALUE_ANNOTATION.to_owned(),
        header_value.to_owned(),
    );
    annotations
}

/// Return mirrored canary HTTP rules from a source Ingress.
fn nginx_canary_rules(
    source_ingress: &IngressSummary,
    backend_service: &KubernetesResourceRef,
    backend_port: &str,
) -> Result<Vec<NginxIngressCanaryRule>, NginxIngressRoutePlanError> {
    let port =
        service_port(backend_port).ok_or_else(|| NginxIngressRoutePlanError::BackendPort {
            port: backend_port.to_owned(),
        })?;
    let rules = source_ingress
        .rules
        .iter()
        .filter_map(|rule| {
            let paths = rule
                .paths
                .iter()
                .filter(|path| path.backend.is_some())
                .map(|path| NginxIngressCanaryPath {
                    path: path.path.clone(),
                    path_type: path
                        .path_type
                        .clone()
                        .unwrap_or_else(|| "ImplementationSpecific".to_owned()),
                    backend: NginxIngressCanaryBackend {
                        service: NginxIngressCanaryBackendService {
                            name: backend_service.name().to_owned(),
                            port: port.clone(),
                        },
                    },
                })
                .collect::<Vec<_>>();

            (!paths.is_empty()).then(|| NginxIngressCanaryRule {
                host: rule.host.clone(),
                http: NginxIngressCanaryHttpRule { paths },
            })
        })
        .collect::<Vec<_>>();

    if rules.is_empty() {
        return Err(NginxIngressRoutePlanError::MissingHttpPaths);
    }

    Ok(rules)
}

/// Return true when the source Ingress has at least one HTTP path backend.
fn ingress_has_http_paths(source_ingress: &IngressSummary) -> bool {
    source_ingress
        .rules
        .iter()
        .any(|rule| rule.paths.iter().any(|path| path.backend.is_some()))
}

/// Return true when the Ingress class clearly belongs to ingress-nginx.
fn is_nginx_ingress_class(class_name: Option<&str>) -> bool {
    class_name.is_some_and(|class_name| {
        class_name.eq_ignore_ascii_case("nginx")
            || class_name.eq_ignore_ascii_case("ingress-nginx")
            || class_name.eq_ignore_ascii_case("nginx-ingress")
    })
}

/// Convert a Service port string into a Kubernetes Ingress backend port.
fn service_port(port: &str) -> Option<NginxIngressCanaryServicePort> {
    if port.is_empty() {
        return None;
    }

    match port.parse::<u16>() {
        Ok(number) if number > 0 => Some(NginxIngressCanaryServicePort::Number { number }),
        Ok(_) => None,
        Err(_) => Some(NginxIngressCanaryServicePort::Name {
            name: port.to_owned(),
        }),
    }
}

/// Return the parent namespace only when the Gateway is cross-namespace.
fn parent_gateway_namespace(
    route: &KubernetesResourceRef,
    parent_gateway: &KubernetesResourceRef,
) -> Option<String> {
    (route.namespace() != parent_gateway.namespace()).then(|| parent_gateway.namespace().to_owned())
}

/// Return hostnames selected by a route selector.
fn selector_hostnames(selector: &RouteSelector) -> Vec<String> {
    selector
        .hostname()
        .map(|hostname| vec![hostname.to_owned()])
        .unwrap_or_default()
}

/// Return HTTPRoute matches selected by a route selector.
fn selector_matches(selector: &RouteSelector) -> Vec<GatewayHttpRouteMatch> {
    selector
        .header_parts()
        .map(|(name, value)| {
            vec![GatewayHttpRouteMatch {
                headers: vec![GatewayHttpRouteHeaderMatch {
                    match_type: "Exact",
                    name: name.to_owned(),
                    value: value.to_owned(),
                }],
            }]
        })
        .unwrap_or_default()
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

/// Detect Ingress inventory from read-only Kubernetes summaries.
fn detect_ingress_routes(ingresses: Option<&[IngressSummary]>) -> IngressRouteDetection {
    let ingress_api_detected = ingresses.is_some();
    let ingresses = ingresses.unwrap_or_default();

    IngressRouteDetection {
        ingress_api_detected,
        ingress_count: ingresses.len(),
        ingress_names: ingress_names(ingresses),
        ingress_class_names: ingress_class_names(ingresses),
        hostnames: ingress_hostnames(ingresses),
        backend_service_names: ingress_backend_service_names(ingresses),
    }
}

/// Return candidate route strategies in deterministic preference order.
fn routing_candidate_strategies(
    gateway_api: &GatewayRouteCapabilities,
    ingress: &IngressRouteDetection,
    preview_service_available: bool,
) -> Vec<RouteStrategy> {
    let mut strategies = Vec::new();

    if gateway_api.supports_temporary_http_routes {
        strategies.push(RouteStrategy::GatewayApi);
    }
    if ingress.ingress_count > 0 {
        strategies.push(RouteStrategy::Ingress);
    }
    if preview_service_available {
        strategies.push(RouteStrategy::PreviewService);
    }

    strategies
}

/// Detect whether direct preview Service fallback checks are explicitly available.
fn detect_preview_service_available(input: &RoutingCapabilityInput<'_>) -> bool {
    input.preview_service_enabled
}

/// Return stable high-level routing capability limitation codes.
fn routing_capability_limitations(
    gateway_api: &GatewayRouteCapabilities,
    ingress: &IngressRouteDetection,
    preview_service_available: bool,
) -> Vec<RoutingCapabilityLimitation> {
    let mut limitations = Vec::new();

    match gateway_api.status {
        GatewayRouteCapabilityStatus::Supported => {}
        GatewayRouteCapabilityStatus::Partial => {
            limitations.push(RoutingCapabilityLimitation::GatewayApiPartial);
        }
        GatewayRouteCapabilityStatus::Unsupported => {
            limitations.push(RoutingCapabilityLimitation::GatewayApiUnavailable);
        }
    }
    if ingress.ingress_count == 0 {
        limitations.push(RoutingCapabilityLimitation::IngressUnavailable);
    } else {
        limitations.push(RoutingCapabilityLimitation::IngressPlanningNotImplemented);
    }
    if preview_service_available {
        limitations.push(RoutingCapabilityLimitation::PreviewServiceBypassesEdgeRouting);
    }

    limitations
}

/// Return namespace-qualified Ingress names in deterministic order.
fn ingress_names(ingresses: &[IngressSummary]) -> Vec<String> {
    ingresses
        .iter()
        .map(|ingress| qualified_resource_name(&ingress.namespace, &ingress.name))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Return discovered IngressClass names in deterministic order.
fn ingress_class_names(ingresses: &[IngressSummary]) -> Vec<String> {
    ingresses
        .iter()
        .filter_map(|ingress| ingress.ingress_class_name.as_deref())
        .map(str::to_owned)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Return discovered Ingress hostnames in deterministic order.
fn ingress_hostnames(ingresses: &[IngressSummary]) -> Vec<String> {
    ingresses
        .iter()
        .flat_map(|ingress| {
            ingress
                .rules
                .iter()
                .filter_map(|rule| rule.host.as_deref())
                .chain(
                    ingress
                        .tls
                        .iter()
                        .flat_map(|tls| tls.hosts.iter().map(String::as_str)),
                )
        })
        .map(str::to_owned)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Return Ingress backend Service names in deterministic order.
fn ingress_backend_service_names(ingresses: &[IngressSummary]) -> Vec<String> {
    ingresses
        .iter()
        .flat_map(|ingress| {
            ingress.default_backend.iter().chain(
                ingress
                    .rules
                    .iter()
                    .flat_map(|rule| rule.paths.iter().filter_map(|path| path.backend.as_ref())),
            )
        })
        .map(|backend| backend.service_name.clone())
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
    use std::{fs, path::Path};

    use super::{
        GatewayApiDiscoveryInput, GatewayApiResourceDetection, GatewayApiResourceStatus,
        GatewayHttpRouteManifestError, GatewayHttpRouteManifestInput, GatewayRouteCapabilities,
        GatewayRouteCapabilityLimitation, GatewayRouteCapabilityStatus, IngressRouteDetection,
        NginxIngressRoutePlanError, NginxIngressRoutePlanInput, RouteStrategy,
        RoutingCapabilityDetection, RoutingCapabilityInput, RoutingCapabilityLimitation,
        detect_gateway_api_resources, detect_routing_capabilities,
        gateway_http_route_cleanup_target, gateway_route_cleanup_selector,
        generate_gateway_header_http_route_manifest, generate_gateway_host_http_route_manifest,
        generate_gateway_http_route_manifest, generate_nginx_ingress_canary_manifest,
        model_gateway_route_capabilities,
    };
    use kply_core::{KubernetesResourceRef, MetadataEntry, RouteSelector};
    use kply_k8s::{
        GatewayClassSummary, GatewayListenerSummary, GatewaySummary, HttpRouteSummary,
        IngressBackendSummary, IngressPathSummary, IngressRuleSummary, IngressSummary,
        IngressTlsSummary, gateway_class_summary, gateway_summary, http_route_summary,
    };
    use kube::{api::ObjectList, core::DynamicObject};
    use serde_json::json;

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

    #[test]
    /// Detects routing capabilities with Gateway API as the preferred candidate.
    fn detects_routing_capabilities_with_supported_gateway_api() {
        let gateway_classes = vec![gateway_class("istio", Some("istio.io/gateway-controller"))];
        let gateways = vec![gateway_with_protocol(
            "shop",
            "public",
            "istio",
            Some("HTTP"),
        )];
        let http_routes: Vec<HttpRouteSummary> = Vec::new();
        let ingresses = vec![ingress("shop", "checkout", Some("nginx"))];

        let detection = detect_routing_capabilities(RoutingCapabilityInput {
            gateway_api: GatewayApiDiscoveryInput {
                gateway_classes: Some(&gateway_classes),
                gateways: Some(&gateways),
                http_routes: Some(&http_routes),
            },
            ingresses: Some(&ingresses),
            preview_service_enabled: true,
        });

        assert_eq!(
            detection,
            RoutingCapabilityDetection {
                gateway_api: model_gateway_route_capabilities(GatewayApiDiscoveryInput {
                    gateway_classes: Some(&gateway_classes),
                    gateways: Some(&gateways),
                    http_routes: Some(&http_routes),
                }),
                ingress: IngressRouteDetection {
                    ingress_api_detected: true,
                    ingress_count: 1,
                    ingress_names: vec!["shop/checkout".to_owned()],
                    ingress_class_names: vec!["nginx".to_owned()],
                    hostnames: vec!["checkout.example.com".to_owned()],
                    backend_service_names: vec!["checkout-http".to_owned()],
                },
                preview_service_available: true,
                candidate_strategies: vec![
                    RouteStrategy::GatewayApi,
                    RouteStrategy::Ingress,
                    RouteStrategy::PreviewService,
                ],
                limitations: vec![
                    RoutingCapabilityLimitation::IngressPlanningNotImplemented,
                    RoutingCapabilityLimitation::PreviewServiceBypassesEdgeRouting,
                ],
            }
        );
        assert_eq!(RouteStrategy::GatewayApi.as_str(), "gateway_api");
        assert_eq!(
            RoutingCapabilityLimitation::PreviewServiceBypassesEdgeRouting.as_str(),
            "preview_service_bypasses_edge_routing"
        );
    }

    #[test]
    /// Detects routing capabilities when only fallback preview Service checks are possible.
    fn detects_routing_capabilities_with_preview_service_fallback() {
        let detection = detect_routing_capabilities(RoutingCapabilityInput {
            gateway_api: GatewayApiDiscoveryInput {
                gateway_classes: None,
                gateways: None,
                http_routes: None,
            },
            ingresses: None,
            preview_service_enabled: true,
        });

        assert_eq!(
            detection.gateway_api.status,
            GatewayRouteCapabilityStatus::Unsupported
        );
        assert_eq!(
            detection.ingress,
            IngressRouteDetection {
                ingress_api_detected: false,
                ingress_count: 0,
                ingress_names: Vec::new(),
                ingress_class_names: Vec::new(),
                hostnames: Vec::new(),
                backend_service_names: Vec::new(),
            }
        );
        assert!(detection.preview_service_available);
        assert_eq!(
            detection.candidate_strategies,
            [RouteStrategy::PreviewService]
        );
        assert_eq!(
            detection.limitations,
            [
                RoutingCapabilityLimitation::GatewayApiUnavailable,
                RoutingCapabilityLimitation::IngressUnavailable,
                RoutingCapabilityLimitation::PreviewServiceBypassesEdgeRouting,
            ]
        );
    }

    #[test]
    /// Does not advertise preview Service fallback unless explicitly enabled.
    fn detects_routing_capabilities_without_preview_service_fallback() {
        let detection = detect_routing_capabilities(RoutingCapabilityInput {
            gateway_api: GatewayApiDiscoveryInput {
                gateway_classes: None,
                gateways: None,
                http_routes: None,
            },
            ingresses: None,
            preview_service_enabled: false,
        });

        assert!(!detection.preview_service_available);
        assert_eq!(detection.candidate_strategies, Vec::<RouteStrategy>::new());
        assert_eq!(
            detection.limitations,
            [
                RoutingCapabilityLimitation::GatewayApiUnavailable,
                RoutingCapabilityLimitation::IngressUnavailable,
            ]
        );
    }

    #[test]
    /// Reports partial Gateway API limitations without labeling Gateway API unavailable.
    fn detects_routing_capabilities_with_partial_gateway_api() {
        let gateway_classes = vec![gateway_class("mesh", Some("example.com/controller"))];
        let gateways = vec![gateway_with_protocol("shop", "mesh", "mesh", Some("TCP"))];
        let http_routes: Vec<HttpRouteSummary> = Vec::new();

        let detection = detect_routing_capabilities(RoutingCapabilityInput {
            gateway_api: GatewayApiDiscoveryInput {
                gateway_classes: Some(&gateway_classes),
                gateways: Some(&gateways),
                http_routes: Some(&http_routes),
            },
            ingresses: Some(&[]),
            preview_service_enabled: false,
        });

        assert_eq!(
            detection.gateway_api.status,
            GatewayRouteCapabilityStatus::Partial
        );
        assert_eq!(detection.candidate_strategies, Vec::<RouteStrategy>::new());
        assert_eq!(
            detection.limitations,
            [
                RoutingCapabilityLimitation::GatewayApiPartial,
                RoutingCapabilityLimitation::IngressUnavailable,
            ]
        );
        assert_eq!(
            RoutingCapabilityLimitation::GatewayApiPartial.as_str(),
            "gateway_api_partial"
        );
    }

    #[test]
    /// Generates an ingress-nginx header canary Ingress manifest.
    fn generates_nginx_ingress_canary_manifest() {
        let source = ingress("shop", "checkout", Some("nginx"));
        let route = resource("shop", "Ingress", "checkout-session-123");
        let service = resource("shop", "Service", "checkout-sandbox");
        let selector = RouteSelector::header("x-kply-session", "session-123").unwrap();
        let labels = ownership_labels();
        let annotations =
            vec![MetadataEntry::new("kply.dev/route-strategy", "nginx-ingress").unwrap()];

        let manifest = generate_nginx_ingress_canary_manifest(NginxIngressRoutePlanInput {
            source_ingress: &source,
            route: &route,
            backend_service: &service,
            backend_port: "8080",
            selector: &selector,
            labels: &labels,
            annotations: &annotations,
        })
        .unwrap();

        assert_eq!(
            serde_json::to_value(manifest).unwrap(),
            json!({
                "apiVersion": "networking.k8s.io/v1",
                "kind": "Ingress",
                "metadata": {
                    "namespace": "shop",
                    "name": "checkout-session-123",
                    "labels": {
                        "kply.dev/app": "checkout",
                        "kply.dev/managed-by": "kply",
                        "kply.dev/session-id": "session-123",
                        "kply.dev/session-name": "checkout-test"
                    },
                    "annotations": {
                        "kply.dev/route-strategy": "nginx-ingress",
                        "nginx.ingress.kubernetes.io/canary": "true",
                        "nginx.ingress.kubernetes.io/canary-by-header": "x-kply-session",
                        "nginx.ingress.kubernetes.io/canary-by-header-value": "session-123"
                    }
                },
                "spec": {
                    "ingressClassName": "nginx",
                    "rules": [
                        {
                            "host": "checkout.example.com",
                            "http": {
                                "paths": [
                                    {
                                        "path": "/",
                                        "pathType": "Prefix",
                                        "backend": {
                                            "service": {
                                                "name": "checkout-sandbox",
                                                "port": {
                                                    "number": 8080
                                                }
                                            }
                                        }
                                    }
                                ]
                            }
                        }
                    ]
                }
            })
        );
    }

    #[test]
    /// Generates an ingress-nginx canary manifest with a named Service port.
    fn generates_nginx_ingress_canary_manifest_with_named_port() {
        let source = ingress("shop", "checkout", Some("ingress-nginx"));
        let route = resource("shop", "Ingress", "checkout-session-123");
        let service = resource("shop", "Service", "checkout-sandbox");
        let selector = RouteSelector::header("x-kply-session", "session-123").unwrap();

        let manifest = generate_nginx_ingress_canary_manifest(NginxIngressRoutePlanInput {
            source_ingress: &source,
            route: &route,
            backend_service: &service,
            backend_port: "http",
            selector: &selector,
            labels: &ownership_labels(),
            annotations: &[],
        })
        .unwrap();

        assert_eq!(
            serde_json::to_value(&manifest.spec.rules[0].http.paths[0].backend.service.port)
                .unwrap(),
            json!({ "name": "http" })
        );
    }

    #[test]
    /// Rejects ingress-nginx canary planning for non-NGINX Ingress classes.
    fn rejects_nginx_ingress_canary_manifest_for_non_nginx_class() {
        let source = ingress("shop", "checkout", Some("alb"));
        let route = resource("shop", "Ingress", "checkout-session-123");
        let service = resource("shop", "Service", "checkout-sandbox");
        let selector = RouteSelector::header("x-kply-session", "session-123").unwrap();

        let error = generate_nginx_ingress_canary_manifest(NginxIngressRoutePlanInput {
            source_ingress: &source,
            route: &route,
            backend_service: &service,
            backend_port: "8080",
            selector: &selector,
            labels: &ownership_labels(),
            annotations: &[],
        })
        .unwrap_err();

        assert_eq!(
            error,
            NginxIngressRoutePlanError::UnsupportedIngressClass {
                class_name: Some("alb".to_owned())
            }
        );
        assert_eq!(
            error.to_string(),
            "expected ingress-nginx IngressClass, found alb"
        );
    }

    #[test]
    /// Rejects ingress-nginx canary planning for non-header selectors.
    fn rejects_nginx_ingress_canary_manifest_for_host_selector() {
        let source = ingress("shop", "checkout", Some("nginx"));
        let route = resource("shop", "Ingress", "checkout-session-123");
        let service = resource("shop", "Service", "checkout-sandbox");
        let selector = RouteSelector::host("checkout-preview.example.com").unwrap();

        let error = generate_nginx_ingress_canary_manifest(NginxIngressRoutePlanInput {
            source_ingress: &source,
            route: &route,
            backend_service: &service,
            backend_port: "8080",
            selector: &selector,
            labels: &ownership_labels(),
            annotations: &[],
        })
        .unwrap_err();

        assert_eq!(
            error,
            NginxIngressRoutePlanError::HeaderSelectorRequired {
                kind: "host".to_owned()
            }
        );
        assert_eq!(
            error.to_string(),
            "expected header route selector, found host"
        );
    }

    #[test]
    /// Rejects ingress-nginx canary planning when there are no HTTP paths to mirror.
    fn rejects_nginx_ingress_canary_manifest_without_http_paths() {
        let source = IngressSummary {
            namespace: "shop".to_owned(),
            name: "checkout".to_owned(),
            ingress_class_name: Some("nginx".to_owned()),
            default_backend: None,
            rules: Vec::new(),
            tls: Vec::new(),
        };
        let route = resource("shop", "Ingress", "checkout-session-123");
        let service = resource("shop", "Service", "checkout-sandbox");
        let selector = RouteSelector::header("x-kply-session", "session-123").unwrap();

        let error = generate_nginx_ingress_canary_manifest(NginxIngressRoutePlanInput {
            source_ingress: &source,
            route: &route,
            backend_service: &service,
            backend_port: "8080",
            selector: &selector,
            labels: &ownership_labels(),
            annotations: &[],
        })
        .unwrap_err();

        assert_eq!(error, NginxIngressRoutePlanError::MissingHttpPaths);
    }

    #[test]
    /// Models supported Gateway API route capabilities from fixture shapes.
    fn models_supported_gateway_api_fixture_shapes() {
        let gateway_classes = read_gateway_fixture("gateway-api-supported", "gatewayclasses.json")
            .items
            .iter()
            .map(gateway_class_summary)
            .collect::<Vec<_>>();
        let gateways = read_gateway_fixture("gateway-api-supported", "gateways.json")
            .items
            .iter()
            .map(gateway_summary)
            .collect::<Vec<_>>();
        let http_routes = read_gateway_fixture("gateway-api-supported", "httproutes.json")
            .items
            .iter()
            .map(http_route_summary)
            .collect::<Vec<_>>();

        let capabilities = model_gateway_route_capabilities(GatewayApiDiscoveryInput {
            gateway_classes: Some(&gateway_classes),
            gateways: Some(&gateways),
            http_routes: Some(&http_routes),
        });

        assert_eq!(capabilities.status, GatewayRouteCapabilityStatus::Supported);
        assert!(capabilities.supports_temporary_http_routes);
        assert!(capabilities.supports_header_based_routing);
        assert!(capabilities.supports_host_based_routing);
        assert_eq!(capabilities.resources.gateway_class_count, 2);
        assert_eq!(capabilities.resources.gateway_count, 2);
        assert_eq!(capabilities.resources.http_route_count, 2);
        assert_eq!(
            capabilities.http_compatible_gateway_names,
            ["platform/shared-gateway", "shop/public-gateway"]
        );
        assert_eq!(capabilities.listener_protocols, ["HTTP", "HTTPS"]);
        assert_eq!(
            capabilities.resources.http_route_names,
            ["shop/checkout-public", "shop/checkout-sandbox-header"]
        );
        assert_eq!(capabilities.limitations, Vec::new());
    }

    #[test]
    /// Models missing GatewayClass fixtures as unsupported for route changes.
    fn models_missing_gateway_class_fixture_shapes() {
        let capabilities = route_capabilities_from_fixture("gateway-api-unsupported-missing-class");

        assert_eq!(capabilities.status, GatewayRouteCapabilityStatus::Partial);
        assert!(!capabilities.supports_temporary_http_routes);
        assert!(!capabilities.supports_header_based_routing);
        assert!(!capabilities.supports_host_based_routing);
        assert_eq!(capabilities.resources.gateway_class_count, 0);
        assert_eq!(capabilities.resources.gateway_count, 1);
        assert_eq!(capabilities.resources.http_route_count, 0);
        assert_eq!(
            capabilities.http_compatible_gateway_names,
            ["shop/public-gateway"]
        );
        assert_eq!(
            capabilities.limitations,
            [
                GatewayRouteCapabilityLimitation::GatewayApiPartial,
                GatewayRouteCapabilityLimitation::MissingGatewayClass
            ]
        );
    }

    #[test]
    /// Models TCP-only Gateway fixtures as unsupported for HTTPRoute changes.
    fn models_tcp_only_gateway_fixture_shapes() {
        let capabilities = route_capabilities_from_fixture("gateway-api-unsupported-tcp-only");

        assert_eq!(capabilities.status, GatewayRouteCapabilityStatus::Partial);
        assert!(!capabilities.supports_temporary_http_routes);
        assert!(!capabilities.supports_header_based_routing);
        assert!(!capabilities.supports_host_based_routing);
        assert_eq!(capabilities.resources.gateway_class_count, 1);
        assert_eq!(capabilities.resources.gateway_count, 1);
        assert_eq!(capabilities.resources.http_route_count, 0);
        assert_eq!(
            capabilities.http_compatible_gateway_names,
            Vec::<String>::new()
        );
        assert_eq!(capabilities.listener_protocols, ["TCP"]);
        assert_eq!(
            capabilities.limitations,
            [GatewayRouteCapabilityLimitation::NoHttpCompatibleListener]
        );
    }

    #[test]
    /// Generates a header-isolated temporary HTTPRoute manifest.
    fn generates_header_based_gateway_http_route_manifest() {
        let route = resource("shop", "HTTPRoute", "checkout-kply");
        let gateway = resource("shop", "Gateway", "public");
        let service = resource("shop", "Service", "checkout-sandbox");
        let selector = RouteSelector::header("x-kply-session", "session-123").unwrap();
        let labels = ownership_labels();
        let annotations =
            vec![MetadataEntry::new("kply.dev/route-strategy", "gateway-api").unwrap()];

        let manifest = generate_gateway_http_route_manifest(GatewayHttpRouteManifestInput {
            route: &route,
            parent_gateway: &gateway,
            backend_service: &service,
            backend_port: 8080,
            selector: &selector,
            labels: &labels,
            annotations: &annotations,
        })
        .unwrap();

        assert_eq!(
            serde_json::to_value(manifest).unwrap(),
            json!({
                "apiVersion": "gateway.networking.k8s.io/v1",
                "kind": "HTTPRoute",
                "metadata": {
                    "namespace": "shop",
                    "name": "checkout-kply",
                    "labels": {
                        "kply.dev/app": "checkout",
                        "kply.dev/managed-by": "kply",
                        "kply.dev/session-id": "session-123",
                        "kply.dev/session-name": "checkout-test"
                    },
                    "annotations": {
                        "kply.dev/route-strategy": "gateway-api"
                    }
                },
                "spec": {
                    "parentRefs": [
                        {
                            "kind": "Gateway",
                            "name": "public"
                        }
                    ],
                    "rules": [
                        {
                            "matches": [
                                {
                                    "headers": [
                                        {
                                            "type": "Exact",
                                            "name": "x-kply-session",
                                            "value": "session-123"
                                        }
                                    ]
                                }
                            ],
                            "backendRefs": [
                                {
                                    "kind": "Service",
                                    "name": "checkout-sandbox",
                                    "port": 8080
                                }
                            ]
                        }
                    ]
                }
            })
        );
    }

    #[test]
    /// Generates header HTTPRoute manifests only when Gateway capabilities allow it.
    fn generates_capability_gated_header_gateway_http_route_manifest() {
        let capabilities = supported_gateway_capabilities();
        let route = resource("shop", "HTTPRoute", "checkout-kply");
        let gateway = resource("shop", "Gateway", "public");
        let service = resource("shop", "Service", "checkout-sandbox");
        let selector = RouteSelector::header("x-kply-session", "session-123").unwrap();
        let labels = ownership_labels();

        let manifest = generate_gateway_header_http_route_manifest(
            &capabilities,
            GatewayHttpRouteManifestInput {
                route: &route,
                parent_gateway: &gateway,
                backend_service: &service,
                backend_port: 8080,
                selector: &selector,
                labels: &labels,
                annotations: &[],
            },
        )
        .unwrap();

        assert_eq!(manifest.kind, "HTTPRoute");
        assert_eq!(
            manifest.spec.rules[0].matches[0].headers[0].name,
            "x-kply-session"
        );
    }

    #[test]
    /// Rejects header HTTPRoute generation when Gateway capabilities are unavailable.
    fn rejects_header_gateway_http_route_manifest_without_capability_support() {
        let capabilities = model_gateway_route_capabilities(GatewayApiDiscoveryInput {
            gateway_classes: None,
            gateways: None,
            http_routes: None,
        });
        let route = resource("shop", "HTTPRoute", "checkout-kply");
        let gateway = resource("shop", "Gateway", "public");
        let service = resource("shop", "Service", "checkout-sandbox");
        let selector = RouteSelector::header("x-kply-session", "session-123").unwrap();

        let error = generate_gateway_header_http_route_manifest(
            &capabilities,
            GatewayHttpRouteManifestInput {
                route: &route,
                parent_gateway: &gateway,
                backend_service: &service,
                backend_port: 8080,
                selector: &selector,
                labels: &[],
                annotations: &[],
            },
        )
        .unwrap_err();

        assert_eq!(
            error,
            GatewayHttpRouteManifestError::HeaderRoutingUnavailable
        );
        assert_eq!(
            error.to_string(),
            "Gateway capabilities do not support header-based routing"
        );
    }

    #[test]
    /// Rejects non-header selectors for header HTTPRoute generation.
    fn rejects_host_selector_for_header_gateway_http_route_manifest() {
        let capabilities = supported_gateway_capabilities();
        let route = resource("shop", "HTTPRoute", "checkout-kply");
        let gateway = resource("shop", "Gateway", "public");
        let service = resource("shop", "Service", "checkout-sandbox");
        let selector = RouteSelector::host("checkout-preview.example.com").unwrap();

        let error = generate_gateway_header_http_route_manifest(
            &capabilities,
            GatewayHttpRouteManifestInput {
                route: &route,
                parent_gateway: &gateway,
                backend_service: &service,
                backend_port: 8080,
                selector: &selector,
                labels: &[],
                annotations: &[],
            },
        )
        .unwrap_err();

        assert_eq!(
            error,
            GatewayHttpRouteManifestError::HeaderSelectorRequired {
                kind: "host".to_owned()
            }
        );
        assert_eq!(
            error.to_string(),
            "expected header route selector, found host"
        );
    }

    #[test]
    /// Generates a host-isolated temporary HTTPRoute manifest.
    fn generates_host_based_gateway_http_route_manifest() {
        let route = resource("shop", "HTTPRoute", "checkout-preview");
        let gateway = resource("platform", "Gateway", "edge");
        let service = resource("shop", "Service", "checkout-sandbox");
        let selector = RouteSelector::host("checkout-preview.example.com").unwrap();
        let labels = ownership_labels();

        let manifest = generate_gateway_http_route_manifest(GatewayHttpRouteManifestInput {
            route: &route,
            parent_gateway: &gateway,
            backend_service: &service,
            backend_port: 80,
            selector: &selector,
            labels: &labels,
            annotations: &[],
        })
        .unwrap();

        assert_eq!(
            serde_json::to_value(manifest).unwrap(),
            json!({
                "apiVersion": "gateway.networking.k8s.io/v1",
                "kind": "HTTPRoute",
                "metadata": {
                    "namespace": "shop",
                    "name": "checkout-preview",
                    "labels": {
                        "kply.dev/app": "checkout",
                        "kply.dev/managed-by": "kply",
                        "kply.dev/session-id": "session-123",
                        "kply.dev/session-name": "checkout-test"
                    }
                },
                "spec": {
                    "parentRefs": [
                        {
                            "kind": "Gateway",
                            "name": "edge",
                            "namespace": "platform"
                        }
                    ],
                    "hostnames": ["checkout-preview.example.com"],
                    "rules": [
                        {
                            "backendRefs": [
                                {
                                    "kind": "Service",
                                    "name": "checkout-sandbox",
                                    "port": 80
                                }
                            ]
                        }
                    ]
                }
            })
        );
    }

    #[test]
    /// Generates host HTTPRoute manifests only when Gateway capabilities allow it.
    fn generates_capability_gated_host_gateway_http_route_manifest() {
        let capabilities = supported_gateway_capabilities();
        let route = resource("shop", "HTTPRoute", "checkout-preview");
        let gateway = resource("platform", "Gateway", "edge");
        let service = resource("shop", "Service", "checkout-sandbox");
        let selector = RouteSelector::host("checkout-preview.example.com").unwrap();
        let labels = ownership_labels();

        let manifest = generate_gateway_host_http_route_manifest(
            &capabilities,
            GatewayHttpRouteManifestInput {
                route: &route,
                parent_gateway: &gateway,
                backend_service: &service,
                backend_port: 80,
                selector: &selector,
                labels: &labels,
                annotations: &[],
            },
        )
        .unwrap();

        assert_eq!(manifest.kind, "HTTPRoute");
        assert_eq!(
            manifest.spec.hostnames,
            ["checkout-preview.example.com".to_owned()]
        );
    }

    #[test]
    /// Rejects host HTTPRoute generation when Gateway capabilities are unavailable.
    fn rejects_host_gateway_http_route_manifest_without_capability_support() {
        let capabilities = model_gateway_route_capabilities(GatewayApiDiscoveryInput {
            gateway_classes: None,
            gateways: None,
            http_routes: None,
        });
        let route = resource("shop", "HTTPRoute", "checkout-preview");
        let gateway = resource("platform", "Gateway", "edge");
        let service = resource("shop", "Service", "checkout-sandbox");
        let selector = RouteSelector::host("checkout-preview.example.com").unwrap();

        let error = generate_gateway_host_http_route_manifest(
            &capabilities,
            GatewayHttpRouteManifestInput {
                route: &route,
                parent_gateway: &gateway,
                backend_service: &service,
                backend_port: 80,
                selector: &selector,
                labels: &[],
                annotations: &[],
            },
        )
        .unwrap_err();

        assert_eq!(error, GatewayHttpRouteManifestError::HostRoutingUnavailable);
        assert_eq!(
            error.to_string(),
            "Gateway capabilities do not support host-based preview routing"
        );
    }

    #[test]
    /// Rejects non-host selectors for host HTTPRoute generation.
    fn rejects_header_selector_for_host_gateway_http_route_manifest() {
        let capabilities = supported_gateway_capabilities();
        let route = resource("shop", "HTTPRoute", "checkout-preview");
        let gateway = resource("platform", "Gateway", "edge");
        let service = resource("shop", "Service", "checkout-sandbox");
        let selector = RouteSelector::header("x-kply-session", "session-123").unwrap();

        let error = generate_gateway_host_http_route_manifest(
            &capabilities,
            GatewayHttpRouteManifestInput {
                route: &route,
                parent_gateway: &gateway,
                backend_service: &service,
                backend_port: 80,
                selector: &selector,
                labels: &[],
                annotations: &[],
            },
        )
        .unwrap_err();

        assert_eq!(
            error,
            GatewayHttpRouteManifestError::HostSelectorRequired {
                kind: "header".to_owned()
            }
        );
        assert_eq!(
            error.to_string(),
            "expected host route selector, found header"
        );
    }

    #[test]
    /// Rejects non-HTTPRoute route resources.
    fn rejects_non_http_route_manifest_resources() {
        let route = resource("shop", "ConfigMap", "checkout-kply");
        let gateway = resource("shop", "Gateway", "public");
        let service = resource("shop", "Service", "checkout-sandbox");
        let selector = RouteSelector::header("x-kply-session", "session-123").unwrap();

        let error = generate_gateway_http_route_manifest(GatewayHttpRouteManifestInput {
            route: &route,
            parent_gateway: &gateway,
            backend_service: &service,
            backend_port: 8080,
            selector: &selector,
            labels: &[],
            annotations: &[],
        })
        .unwrap_err();

        assert_eq!(
            error,
            GatewayHttpRouteManifestError::RouteKind {
                kind: "ConfigMap".to_owned()
            }
        );
        assert_eq!(
            error.to_string(),
            "expected HTTPRoute route resource, found ConfigMap"
        );
    }

    #[test]
    /// Generates a cleanup target for a temporary HTTPRoute.
    fn generates_gateway_http_route_cleanup_target() {
        let route = resource("shop", "HTTPRoute", "checkout-kply");
        let labels = ownership_labels();

        let target = gateway_http_route_cleanup_target(&route, &labels).unwrap();

        assert_eq!(
            serde_json::to_value(target).unwrap(),
            json!({
                "apiVersion": "gateway.networking.k8s.io/v1",
                "kind": "HTTPRoute",
                "namespace": "shop",
                "name": "checkout-kply",
                "selector": {
                    "matchLabels": {
                        "kply.dev/managed-by": "kply",
                        "kply.dev/session-id": "session-123"
                    }
                }
            })
        );
    }

    #[test]
    /// Generates route cleanup selectors from minimal ownership labels.
    fn generates_gateway_route_cleanup_selector() {
        let mut labels = ownership_labels();
        labels.push(MetadataEntry::new_label("pod-template-hash", "abc123").unwrap());
        labels.push(MetadataEntry::new_label("app.kubernetes.io/name", "checkout").unwrap());

        let selector = gateway_route_cleanup_selector(&labels).unwrap();

        assert_eq!(
            serde_json::to_value(selector).unwrap(),
            json!({
                "matchLabels": {
                    "kply.dev/managed-by": "kply",
                    "kply.dev/session-id": "session-123"
                }
            })
        );
    }

    #[test]
    /// Rejects route cleanup targets for non-HTTPRoute resources.
    fn rejects_non_http_route_cleanup_targets() {
        let route = resource("shop", "ConfigMap", "checkout-kply");
        let labels = ownership_labels();

        let error = gateway_http_route_cleanup_target(&route, &labels).unwrap_err();

        assert_eq!(
            error,
            GatewayHttpRouteManifestError::RouteKind {
                kind: "ConfigMap".to_owned()
            }
        );
        assert_eq!(
            error.to_string(),
            "expected HTTPRoute route resource, found ConfigMap"
        );
    }

    #[test]
    /// Rejects route cleanup selectors without complete ownership labels.
    fn rejects_gateway_route_cleanup_selector_without_ownership_labels() {
        let labels = vec![MetadataEntry::new_label("kply.dev/session-id", "session-123").unwrap()];

        let error = gateway_route_cleanup_selector(&labels).unwrap_err();

        assert_eq!(
            error,
            GatewayHttpRouteManifestError::MissingOwnershipLabel {
                key: "kply.dev/app"
            }
        );
    }

    #[test]
    /// Rejects valid route manifests without complete ownership labels.
    fn rejects_gateway_http_route_manifest_without_ownership_labels() {
        let route = resource("shop", "HTTPRoute", "checkout-kply");
        let gateway = resource("shop", "Gateway", "public");
        let service = resource("shop", "Service", "checkout-sandbox");
        let selector = RouteSelector::header("x-kply-session", "session-123").unwrap();
        let labels = vec![MetadataEntry::new_label("kply.dev/session-id", "session-123").unwrap()];

        let error = generate_gateway_http_route_manifest(GatewayHttpRouteManifestInput {
            route: &route,
            parent_gateway: &gateway,
            backend_service: &service,
            backend_port: 8080,
            selector: &selector,
            labels: &labels,
            annotations: &[],
        })
        .unwrap_err();

        assert_eq!(
            error,
            GatewayHttpRouteManifestError::MissingOwnershipLabel {
                key: "kply.dev/app"
            }
        );
        assert_eq!(
            error.to_string(),
            "Gateway route is missing ownership label `kply.dev/app`"
        );
    }

    #[test]
    /// Rejects non-Gateway parent resources.
    fn rejects_non_gateway_parent_resources() {
        let route = resource("shop", "HTTPRoute", "checkout-kply");
        let gateway = resource("shop", "Service", "not-a-gateway");
        let service = resource("shop", "Service", "checkout-sandbox");
        let selector = RouteSelector::header("x-kply-session", "session-123").unwrap();

        let error = generate_gateway_http_route_manifest(GatewayHttpRouteManifestInput {
            route: &route,
            parent_gateway: &gateway,
            backend_service: &service,
            backend_port: 8080,
            selector: &selector,
            labels: &[],
            annotations: &[],
        })
        .unwrap_err();

        assert_eq!(
            error,
            GatewayHttpRouteManifestError::ParentKind {
                kind: "Service".to_owned()
            }
        );
        assert_eq!(
            error.to_string(),
            "expected Gateway parent resource, found Service"
        );
    }

    #[test]
    /// Rejects non-Service backend resources.
    fn rejects_non_service_backend_resources() {
        let route = resource("shop", "HTTPRoute", "checkout-kply");
        let gateway = resource("shop", "Gateway", "public");
        let service = resource("shop", "Deployment", "checkout-sandbox");
        let selector = RouteSelector::header("x-kply-session", "session-123").unwrap();

        let error = generate_gateway_http_route_manifest(GatewayHttpRouteManifestInput {
            route: &route,
            parent_gateway: &gateway,
            backend_service: &service,
            backend_port: 8080,
            selector: &selector,
            labels: &[],
            annotations: &[],
        })
        .unwrap_err();

        assert_eq!(
            error,
            GatewayHttpRouteManifestError::BackendKind {
                kind: "Deployment".to_owned()
            }
        );
        assert_eq!(
            error.to_string(),
            "expected Service backend resource, found Deployment"
        );
    }

    #[test]
    /// Rejects cross-namespace Service backends.
    fn rejects_cross_namespace_backend_services() {
        let route = resource("shop", "HTTPRoute", "checkout-kply");
        let gateway = resource("shop", "Gateway", "public");
        let service = resource("payments", "Service", "checkout-sandbox");
        let selector = RouteSelector::header("x-kply-session", "session-123").unwrap();

        let error = generate_gateway_http_route_manifest(GatewayHttpRouteManifestInput {
            route: &route,
            parent_gateway: &gateway,
            backend_service: &service,
            backend_port: 8080,
            selector: &selector,
            labels: &[],
            annotations: &[],
        })
        .unwrap_err();

        assert_eq!(
            error,
            GatewayHttpRouteManifestError::BackendNamespace {
                route_namespace: "shop".to_owned(),
                backend_namespace: "payments".to_owned()
            }
        );
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

    /// Build complete route ownership labels.
    fn ownership_labels() -> Vec<MetadataEntry> {
        vec![
            MetadataEntry::new_label("kply.dev/app", "checkout").unwrap(),
            MetadataEntry::new_label("kply.dev/managed-by", "kply").unwrap(),
            MetadataEntry::new_label("kply.dev/session-id", "session-123").unwrap(),
            MetadataEntry::new_label("kply.dev/session-name", "checkout-test").unwrap(),
        ]
    }

    /// Build supported Gateway route capabilities.
    fn supported_gateway_capabilities() -> GatewayRouteCapabilities {
        let gateway_classes = vec![gateway_class("istio", Some("istio.io/gateway-controller"))];
        let gateways = vec![gateway("shop", "public", "istio")];
        let http_routes: Vec<HttpRouteSummary> = Vec::new();

        model_gateway_route_capabilities(GatewayApiDiscoveryInput {
            gateway_classes: Some(&gateway_classes),
            gateways: Some(&gateways),
            http_routes: Some(&http_routes),
        })
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

    /// Build an Ingress summary fixture with one host and backend Service.
    fn ingress(namespace: &str, name: &str, ingress_class_name: Option<&str>) -> IngressSummary {
        IngressSummary {
            namespace: namespace.to_owned(),
            name: name.to_owned(),
            ingress_class_name: ingress_class_name.map(ToOwned::to_owned),
            default_backend: None,
            rules: vec![IngressRuleSummary {
                host: Some("checkout.example.com".to_owned()),
                paths: vec![IngressPathSummary {
                    path: Some("/".to_owned()),
                    path_type: Some("Prefix".to_owned()),
                    backend: Some(IngressBackendSummary {
                        service_name: "checkout-http".to_owned(),
                        service_port: "80".to_owned(),
                    }),
                }],
            }],
            tls: vec![IngressTlsSummary {
                hosts: vec!["checkout.example.com".to_owned()],
                secret_name: Some("checkout-tls".to_owned()),
            }],
        }
    }

    /// Read one supported Gateway API fixture list.
    /// Build route capabilities from one Gateway API response fixture shape.
    fn route_capabilities_from_fixture(shape: &str) -> GatewayRouteCapabilities {
        let gateway_classes = read_gateway_fixture(shape, "gatewayclasses.json")
            .items
            .iter()
            .map(gateway_class_summary)
            .collect::<Vec<_>>();
        let gateways = read_gateway_fixture(shape, "gateways.json")
            .items
            .iter()
            .map(gateway_summary)
            .collect::<Vec<_>>();
        let http_routes = read_gateway_fixture(shape, "httproutes.json")
            .items
            .iter()
            .map(http_route_summary)
            .collect::<Vec<_>>();

        model_gateway_route_capabilities(GatewayApiDiscoveryInput {
            gateway_classes: Some(&gateway_classes),
            gateways: Some(&gateways),
            http_routes: Some(&http_routes),
        })
    }

    /// Read one Gateway API fixture list.
    fn read_gateway_fixture(shape: &str, name: &str) -> ObjectList<DynamicObject> {
        let fixture_path =
            kply_test::fixture_path(Path::new("k8s-responses").join(shape).join(name));
        let source = fs::read_to_string(&fixture_path).unwrap_or_else(|error| {
            panic!(
                "Gateway API fixture {} should be readable: {error}",
                fixture_path.display()
            )
        });

        serde_json::from_str(&source).unwrap_or_else(|error| {
            panic!(
                "Gateway API fixture {} should deserialize: {error}",
                fixture_path.display()
            )
        })
    }

    /// Build a Kubernetes resource fixture.
    fn resource(namespace: &str, kind: &str, name: &str) -> KubernetesResourceRef {
        KubernetesResourceRef::new(namespace, kind, name).unwrap()
    }
}
