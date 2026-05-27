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
use kply_k8s::{GatewayClassSummary, GatewaySummary, HttpRouteSummary};
use serde::Serialize;

const GATEWAY_API_VERSION: &str = "gateway.networking.k8s.io/v1";
const HTTP_ROUTE_KIND: &str = "HTTPRoute";
const GATEWAY_KIND: &str = "Gateway";
const SERVICE_KIND: &str = "Service";

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
    /// Label selector that must match before cleanup deletes the route.
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
                    "HTTPRoute manifest is missing ownership label `{key}`"
                )
            }
        }
    }
}

impl Error for GatewayHttpRouteManifestError {}

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

/// Generate a deterministic cleanup target for a temporary Gateway API HTTPRoute.
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

/// Generate the minimal label selector required for Gateway API route cleanup.
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
        GatewayHttpRouteManifestError, GatewayHttpRouteManifestInput, GatewayRouteCapabilities,
        GatewayRouteCapabilityLimitation, GatewayRouteCapabilityStatus,
        detect_gateway_api_resources, gateway_http_route_cleanup_target,
        gateway_route_cleanup_selector, generate_gateway_header_http_route_manifest,
        generate_gateway_host_http_route_manifest, generate_gateway_http_route_manifest,
        model_gateway_route_capabilities,
    };
    use kply_core::{KubernetesResourceRef, MetadataEntry, RouteSelector};
    use kply_k8s::{GatewayClassSummary, GatewayListenerSummary, GatewaySummary, HttpRouteSummary};
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
            "HTTPRoute manifest is missing ownership label `kply.dev/app`"
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

    /// Build a Kubernetes resource fixture.
    fn resource(namespace: &str, kind: &str, name: &str) -> KubernetesResourceRef {
        KubernetesResourceRef::new(namespace, kind, name).unwrap()
    }
}
