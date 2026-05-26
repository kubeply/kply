//! Smoke tests for local demo Kubernetes manifests.

use kply_test::fixture_path;
use serde::Deserialize;
use serde_norway::Value;
use std::collections::BTreeMap;

const DEMO_NAMESPACE: &str = "kply-demo";
const PART_OF_LABEL: &str = "app.kubernetes.io/part-of";

/// Demo manifest fixture paths covered by the smoke tests.
const DEMO_MANIFESTS: [&str; 6] = [
    "demo/ecommerce-basic/manifests/namespace.yaml",
    "demo/ecommerce-basic/manifests/catalog.yaml",
    "demo/ecommerce-basic/manifests/frontend.yaml",
    "demo/ecommerce-basic/manifests/backend.yaml",
    "demo/ecommerce-basic/manifests/backend-broken.yaml",
    "demo/ecommerce-basic/manifests/backend-fixed.yaml",
];

/// Ensure the demo manifests still expose the expected object set.
#[test]
fn demo_manifests_parse_into_expected_resources() {
    let resources = demo_resources();
    let identities = resources
        .iter()
        .map(DemoResource::identity)
        .collect::<Vec<_>>();

    assert_eq!(
        identities,
        [
            "demo/ecommerce-basic/manifests/namespace.yaml Namespace/kply-demo",
            "demo/ecommerce-basic/manifests/catalog.yaml Deployment/kply-demo/catalog-api",
            "demo/ecommerce-basic/manifests/catalog.yaml Service/kply-demo/catalog-api",
            "demo/ecommerce-basic/manifests/frontend.yaml Deployment/kply-demo/storefront-web",
            "demo/ecommerce-basic/manifests/frontend.yaml Service/kply-demo/storefront-web",
            "demo/ecommerce-basic/manifests/backend.yaml Deployment/kply-demo/checkout-api",
            "demo/ecommerce-basic/manifests/backend.yaml Service/kply-demo/checkout-api",
            "demo/ecommerce-basic/manifests/backend-broken.yaml Deployment/kply-demo/checkout-api",
            "demo/ecommerce-basic/manifests/backend-broken.yaml Service/kply-demo/checkout-api",
            "demo/ecommerce-basic/manifests/backend-fixed.yaml Deployment/kply-demo/checkout-api",
            "demo/ecommerce-basic/manifests/backend-fixed.yaml Service/kply-demo/checkout-api",
        ],
        "update the demo smoke tests when fixture resources change"
    );
}

/// Ensure every demo resource is isolated to the dedicated namespace boundary.
#[test]
fn demo_resources_stay_in_the_dedicated_namespace() {
    for resource in demo_resources() {
        let labels = string_map_at(&resource.value, &["metadata", "labels"]);

        assert_eq!(
            labels.get(PART_OF_LABEL).map(String::as_str),
            Some(DEMO_NAMESPACE),
            "{} must carry the demo ownership label",
            resource.identity()
        );

        if resource.kind == "Namespace" {
            assert_eq!(
                resource.name, DEMO_NAMESPACE,
                "the demo namespace manifest must define the dedicated namespace"
            );
            assert!(
                resource.namespace.is_none(),
                "namespace resources must not set metadata.namespace"
            );
        } else {
            assert_eq!(
                resource.namespace.as_deref(),
                Some(DEMO_NAMESPACE),
                "{} must stay inside the dedicated demo namespace",
                resource.identity()
            );
        }
    }
}

/// Ensure demo Services route to the pods created by their paired Deployments.
#[test]
fn demo_services_select_matching_deployments() {
    for manifest_path in DEMO_MANIFESTS
        .iter()
        .copied()
        .filter(|path| !path.ends_with("namespace.yaml"))
    {
        let resources = read_manifest_resources(manifest_path);
        let deployment = find_kind(&resources, "Deployment", manifest_path);
        let service = find_kind(&resources, "Service", manifest_path);

        assert_eq!(
            deployment.name, service.name,
            "{manifest_path} service and deployment should share a workload name"
        );

        let deployment_selector =
            string_map_at(&deployment.value, &["spec", "selector", "matchLabels"]);
        let pod_labels = string_map_at(
            &deployment.value,
            &["spec", "template", "metadata", "labels"],
        );
        let service_selector = string_map_at(&service.value, &["spec", "selector"]);

        assert_eq!(
            service_selector, deployment_selector,
            "{manifest_path} service selector must match the deployment selector"
        );

        for (key, value) in deployment_selector {
            assert_eq!(
                pod_labels.get(&key),
                Some(&value),
                "{manifest_path} pod template must include selector label {key}"
            );
        }
    }
}

#[derive(Debug)]
struct DemoResource {
    /// Repository fixture path that defined the resource.
    file: &'static str,
    /// Parsed YAML document for structured assertions.
    value: Value,
    /// Kubernetes API version declared by the resource.
    api_version: String,
    /// Kubernetes kind declared by the resource.
    kind: String,
    /// Kubernetes object name.
    name: String,
    /// Kubernetes object namespace when the resource is namespaced.
    namespace: Option<String>,
}

impl DemoResource {
    /// Return the stable identity used in smoke-test failure messages.
    fn identity(&self) -> String {
        match &self.namespace {
            Some(namespace) => {
                format!("{} {}/{}/{}", self.file, self.kind, namespace, self.name)
            }
            None => format!("{} {}/{}", self.file, self.kind, self.name),
        }
    }
}

/// Load every local demo manifest resource in deterministic fixture order.
fn demo_resources() -> Vec<DemoResource> {
    DEMO_MANIFESTS
        .into_iter()
        .flat_map(read_manifest_resources)
        .collect()
}

/// Read one multi-document manifest file into parsed Kubernetes object values.
fn read_manifest_resources(file: &'static str) -> Vec<DemoResource> {
    let path = fixture_path(file);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("reading {} failed: {error}", path.display()));

    let resources = serde_norway::Deserializer::from_str(&source)
        .map(|document| Value::deserialize(document).expect("demo manifest document should parse"))
        .filter(|value| !value.is_null())
        .map(|value| DemoResource {
            api_version: string_at(&value, &["apiVersion"]).to_owned(),
            kind: string_at(&value, &["kind"]).to_owned(),
            name: string_at(&value, &["metadata", "name"]).to_owned(),
            namespace: value
                .get("metadata")
                .and_then(|metadata| metadata.get("namespace"))
                .and_then(Value::as_str)
                .map(str::to_owned),
            value,
            file,
        })
        .collect::<Vec<_>>();

    assert!(
        !resources.is_empty(),
        "{file} should contain at least one Kubernetes object"
    );

    for resource in &resources {
        assert!(
            !resource.api_version.is_empty(),
            "{} must declare apiVersion",
            resource.identity()
        );
    }

    resources
}

/// Find a resource by Kubernetes kind inside one manifest file.
fn find_kind<'a>(
    resources: &'a [DemoResource],
    kind: &str,
    manifest_path: &str,
) -> &'a DemoResource {
    let matches = resources
        .iter()
        .filter(|resource| resource.kind == kind)
        .collect::<Vec<_>>();

    assert_eq!(
        matches.len(),
        1,
        "{manifest_path} should contain exactly one {kind}, found {}",
        matches.len()
    );

    matches[0]
}

/// Read a string field from a parsed YAML value.
fn string_at<'a>(value: &'a Value, path: &[&str]) -> &'a str {
    value_at(value, path)
        .as_str()
        .unwrap_or_else(|| panic!("{} should be a string", path.join(".")))
}

/// Read a string-to-string map field from a parsed YAML value.
fn string_map_at(value: &Value, path: &[&str]) -> BTreeMap<String, String> {
    value_at(value, path)
        .as_mapping()
        .unwrap_or_else(|| panic!("{} should be a map", path.join(".")))
        .iter()
        .map(|(key, value)| {
            let key = key
                .as_str()
                .unwrap_or_else(|| panic!("{} key should be a string", path.join(".")));
            let value = value
                .as_str()
                .unwrap_or_else(|| panic!("{} value should be a string", path.join(".")));

            (key.to_owned(), value.to_owned())
        })
        .collect()
}

/// Traverse a parsed YAML value by path and return the nested value.
fn value_at<'a>(value: &'a Value, path: &[&str]) -> &'a Value {
    path.iter().fold(value, |current, key| {
        current
            .get(*key)
            .unwrap_or_else(|| panic!("missing {}", path.join(".")))
    })
}
