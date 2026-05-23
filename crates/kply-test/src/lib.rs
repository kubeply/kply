//! Shared helpers for future Kply integration tests and fixtures.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use assert_cmd::Command;
use regex::Regex;
use serde_json::Value;

static RFC3339_TIMESTAMP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z\b")
        .expect("timestamp regex should compile")
});
static GENERATED_ID: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b")
        .expect("generated id regex should compile")
});
static KUBERNETES_OBJECT_NAME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\bkply-[a-z0-9]([-a-z0-9]*[a-z0-9])?-[0-9a-f]{8,}\b")
        .expect("Kubernetes object name regex should compile")
});
static ABSOLUTE_PATH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?m)(^|[\s"=])/(?:Users|private|tmp|var)/[^\s"]+"#)
        .expect("absolute path regex should compile")
});

/// Return a command handle for the `kply` binary in integration tests.
pub fn kply_cmd() -> Command {
    Command::cargo_bin("kply").expect("kply binary should be built for tests")
}

/// Return the repository fixture root.
pub fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("kply-test should live under crates/")
        .join("fixtures")
}

/// Resolve a path inside the repository fixture root.
pub fn fixture_path(relative_path: impl AsRef<Path>) -> PathBuf {
    fixture_root().join(relative_path)
}

/// Run `kply` and return UTF-8 stdout for integration tests.
pub fn kply_stdout(args: &[&str]) -> String {
    let output = kply_cmd()
        .args(args)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    String::from_utf8(output).expect("kply stdout should be UTF-8")
}

/// Parse CLI stdout as JSON for stable assertions.
pub fn parse_json_output(output: impl AsRef<[u8]>) -> Value {
    serde_json::from_slice(output.as_ref()).expect("output should be valid JSON")
}

/// Normalize values that commonly make snapshots unstable.
pub fn normalize_output(output: &str) -> String {
    let output = normalize_timestamps(output);
    let output = normalize_generated_ids(&output);
    let output = normalize_kubernetes_object_names(&output);
    normalize_absolute_paths(&output)
}

/// Normalize RFC 3339 UTC timestamps.
pub fn normalize_timestamps(output: &str) -> String {
    RFC3339_TIMESTAMP
        .replace_all(output, "<timestamp>")
        .into_owned()
}

/// Normalize generated UUID-style identifiers.
pub fn normalize_generated_ids(output: &str) -> String {
    GENERATED_ID
        .replace_all(output, "<generated-id>")
        .into_owned()
}

/// Normalize generated Kply Kubernetes object names.
pub fn normalize_kubernetes_object_names(output: &str) -> String {
    KUBERNETES_OBJECT_NAME
        .replace_all(output, "kply-<object-name>")
        .into_owned()
}

/// Normalize absolute local filesystem paths.
pub fn normalize_absolute_paths(output: &str) -> String {
    ABSOLUTE_PATH
        .replace_all(output, "$1<absolute-path>")
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::{
        fixture_path, fixture_root, kply_stdout, normalize_absolute_paths, normalize_generated_ids,
        normalize_kubernetes_object_names, normalize_output, normalize_timestamps,
        parse_json_output,
    };

    #[test]
    fn resolves_fixture_paths_from_repo_root() {
        assert!(fixture_root().ends_with("fixtures"));
        assert!(fixture_path("cli/example").ends_with("fixtures/cli/example"));
    }

    #[test]
    fn captures_kply_stdout() {
        let output = kply_stdout(&["--json"]);
        let value = parse_json_output(output);

        assert_eq!(value["name"], "kply");
    }

    #[test]
    fn parses_json_output() {
        let value = parse_json_output(br#"{"status":"placeholder"}"#);

        assert_eq!(value["status"], "placeholder");
    }

    #[test]
    fn normalizes_unstable_values() {
        let output = normalize_output(
            "/Users/example/project 2026-05-23T22:00:00Z 123e4567-e89b-12d3-a456-426614174000 kply-api-abcdef1234",
        );

        assert_eq!(
            output,
            "<absolute-path> <timestamp> <generated-id> kply-<object-name>"
        );
    }

    #[test]
    fn normalizes_values_individually() {
        assert_eq!(
            normalize_timestamps("created=2026-05-23T22:00:00.123Z"),
            "created=<timestamp>"
        );
        assert_eq!(
            normalize_generated_ids("id=123e4567-e89b-12d3-a456-426614174000"),
            "id=<generated-id>"
        );
        assert_eq!(
            normalize_kubernetes_object_names("name=kply-backend-abcdef12"),
            "name=kply-<object-name>"
        );
        assert_eq!(
            normalize_absolute_paths("path=/tmp/kply/demo"),
            "path=<absolute-path>"
        );
    }
}
