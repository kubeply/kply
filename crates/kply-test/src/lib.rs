//! Shared helpers for future Kply integration tests and fixtures.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use assert_cmd::Command;
use regex::Regex;
use serde_json::Value;
use tempfile::TempDir;

pub use insta;

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
    Regex::new(r#"(?m)(^|[\s"=])/(?:Users|home|private|tmp|var|workspace)/[^\s"]+"#)
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

/// Create a temporary workspace directory for tests.
pub fn temp_workspace() -> TempDir {
    tempfile::Builder::new()
        .prefix("kply-test-")
        .tempdir()
        .expect("temporary workspace should be created")
}

/// Create a directory path inside a temporary workspace.
pub fn temp_workspace_dir(workspace: &TempDir, relative_path: impl AsRef<Path>) -> PathBuf {
    let path = workspace.path().join(relative_path);
    std::fs::create_dir_all(&path).expect("temporary workspace directory should be created");
    path
}

/// Write a UTF-8 fixture file inside a temporary workspace.
pub fn write_temp_file(
    workspace: &TempDir,
    relative_path: impl AsRef<Path>,
    contents: impl AsRef<str>,
) -> PathBuf {
    let path = workspace.path().join(relative_path);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("temporary file parent should be created");
    }

    std::fs::write(&path, contents.as_ref()).expect("temporary file should be written");
    path
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

/// Assert normalized CLI text output against an insta snapshot.
#[macro_export]
macro_rules! assert_cli_text_snapshot {
    ($name:expr, $output:expr $(,)?) => {{
        $crate::__assert_normalized_text_snapshot!($name, $output);
    }};
}

/// Assert CLI JSON output against an insta JSON snapshot.
#[macro_export]
macro_rules! assert_cli_json_snapshot {
    ($name:expr, $output:expr $(,)?) => {{
        $crate::__assert_json_snapshot!($name, $output);
    }};
}

/// Assert generated Kubernetes manifests against an insta snapshot.
#[macro_export]
macro_rules! assert_manifest_snapshot {
    ($name:expr, $manifest:expr $(,)?) => {{
        $crate::__assert_normalized_text_snapshot!($name, $manifest);
    }};
}

/// Assert check report JSON against an insta JSON snapshot.
#[macro_export]
macro_rules! assert_check_report_snapshot {
    ($name:expr, $report:expr $(,)?) => {{
        $crate::__assert_json_snapshot!($name, $report);
    }};
}

/// Assert route plan JSON against an insta JSON snapshot.
#[macro_export]
macro_rules! assert_route_plan_snapshot {
    ($name:expr, $route_plan:expr $(,)?) => {{
        $crate::__assert_json_snapshot!($name, $route_plan);
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __assert_normalized_text_snapshot {
    ($name:expr, $output:expr $(,)?) => {{
        let normalized = $crate::normalize_output($output);
        $crate::insta::assert_snapshot!($name, normalized);
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __assert_json_snapshot {
    ($name:expr, $output:expr $(,)?) => {{
        let value = $crate::parse_json_output($output);
        $crate::insta::assert_json_snapshot!($name, value);
    }};
}

#[cfg(test)]
mod tests {
    use super::{
        fixture_path, fixture_root, kply_stdout, normalize_absolute_paths, normalize_generated_ids,
        normalize_kubernetes_object_names, normalize_output, normalize_timestamps,
        parse_json_output, temp_workspace, temp_workspace_dir, write_temp_file,
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
    fn creates_temporary_workspace_directories_and_files() {
        let workspace = temp_workspace();

        let directory = temp_workspace_dir(&workspace, "config/nested");
        let file = write_temp_file(&workspace, "config/nested/kply.yaml", "name: demo\n");

        assert!(directory.is_dir());
        assert_eq!(
            std::fs::read_to_string(file).expect("temporary file should be readable"),
            "name: demo\n"
        );
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
        assert_eq!(
            normalize_absolute_paths("path=/home/runner/work/kply"),
            "path=<absolute-path>"
        );
        assert_eq!(
            normalize_absolute_paths("path=/workspace/kply"),
            "path=<absolute-path>"
        );
    }

    #[test]
    fn snapshot_helpers_assert_expected_outputs() {
        crate::assert_cli_text_snapshot!(
            "helper_cli_text",
            "created=2026-05-23T22:00:00Z path=/tmp/kply/demo"
        );
        crate::assert_cli_json_snapshot!("helper_cli_json", br#"{"status":"placeholder"}"#);
        crate::assert_manifest_snapshot!(
            "helper_manifest",
            "metadata:\n  name: kply-backend-abcdef12\n"
        );
        crate::assert_check_report_snapshot!(
            "helper_check_report",
            br#"{"checks":[{"name":"health","status":"pass"}]}"#
        );
        crate::assert_route_plan_snapshot!(
            "helper_route_plan",
            br#"{"routes":[{"header":"x-kply-session","target":"sandbox"}]}"#
        );
    }
}
