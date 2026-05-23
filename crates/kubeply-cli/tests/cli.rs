use kubeply_test::{kubeply_cmd, normalized_json};

#[test]
fn prints_version_as_json() {
    let output = kubeply_cmd()
        .args(["--json", "--version"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_json_snapshot!("version_json", normalized_json(&output));
}

#[test]
fn creates_dry_run_session_plan_as_json() {
    let output = kubeply_cmd()
        .args([
            "--json",
            "session",
            "create",
            "backend-api",
            "--namespace",
            "shop",
            "--image",
            "ghcr.io/acme/backend:agent-fix-123",
            "--route-header",
            "x-kubeply-session",
            "--route-value",
            "fix-123",
            "--dry-run",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_json_snapshot!("session_create_json", normalized_json(&output));
}

#[test]
fn rejects_partial_route_header() {
    kubeply_cmd()
        .args([
            "session",
            "create",
            "backend-api",
            "--image",
            "ghcr.io/acme/backend:agent-fix-123",
            "--route-header",
            "x-kubeply-session",
            "--dry-run",
        ])
        .assert()
        .failure();
}
