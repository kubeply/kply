//! CLI placeholder behavior tests for Kply.

use clap::CommandFactory;
use kply_cli::cli::AppCommand;
use kply_cli::cli::CheckCommand;
use kply_cli::cli::Cli;
use kply_cli::cli::ClusterCommand;
use kply_cli::cli::Command;
use kply_cli::cli::ConfigCommand;
use kply_cli::cli::DemoCommand;
use kply_cli::cli::ReportCommand;
use kply_cli::cli::ReportExportFormat;
use kply_cli::cli::RouteCommand;
use kply_cli::cli::SessionCommand;
use kply_test::{
    EXIT_BLOCKING, EXIT_USAGE, assert_kply_exit_code, kply_cmd, normalize_output, temp_workspace,
    write_fake_kubeconfig, write_temp_file,
};
use std::path::{Path, PathBuf};

const SESSION_PLAN_CONFIG: &str = r#"
version: 1
apps:
  - name: checkout
    namespace: shop
    workload: checkout-api
    service: checkout-http
    default_image: ghcr.io/acme/checkout:next
    route_strategy: header
"#;

const SESSION_PLAN_RISK_CONFIG: &str = r#"
version: 1
apps:
  - name: checkout-db
    namespace: shop
    workload: checkout-postgres
    service: checkout-postgres
    default_image: postgres:16
    route_strategy: preview
"#;

const SESSION_PLAN_STATEFULSET_CONFIG: &str = r#"
version: 1
apps:
  - name: cart
    namespace: shop
    workload: cart-store
    workload_kind: StatefulSet
    service: cart-store
    default_image: ghcr.io/acme/cart:next
    route_strategy: header
"#;

const SESSION_PLAN_NO_IMAGE_CONFIG: &str = r#"
version: 1
apps:
  - name: checkout
    namespace: shop
    workload: checkout-api
    service: checkout-http
    route_strategy: header
"#;

fn with_session_plan_config<T>(run: impl FnOnce(&str) -> T) -> T {
    let workspace = temp_workspace();
    let config_path = write_temp_file(&workspace, "kply.yaml", SESSION_PLAN_CONFIG);
    let config_path = config_path.to_str().expect("config path should be UTF-8");

    run(config_path)
}

fn with_session_plan_risk_config<T>(run: impl FnOnce(&str) -> T) -> T {
    let workspace = temp_workspace();
    let config_path = write_temp_file(&workspace, "kply.yaml", SESSION_PLAN_RISK_CONFIG);
    let config_path = config_path.to_str().expect("config path should be UTF-8");

    run(config_path)
}

fn with_session_plan_statefulset_config<T>(run: impl FnOnce(&str) -> T) -> T {
    let workspace = temp_workspace();
    let config_path = write_temp_file(&workspace, "kply.yaml", SESSION_PLAN_STATEFULSET_CONFIG);
    let config_path = config_path.to_str().expect("config path should be UTF-8");

    run(config_path)
}

fn with_session_plan_no_image_config<T>(run: impl FnOnce(&str) -> T) -> T {
    let workspace = temp_workspace();
    let config_path = write_temp_file(&workspace, "kply.yaml", SESSION_PLAN_NO_IMAGE_CONFIG);
    let config_path = config_path.to_str().expect("config path should be UTF-8");

    run(config_path)
}

fn fake_demo_path(workspace: &Path) -> String {
    let bin_dir = workspace.join("bin");
    std::fs::create_dir_all(&bin_dir).expect("fake PATH bin directory should be created");
    for command in ["kind", "kubectl", "docker"] {
        let path = bin_dir.join(command);
        std::fs::write(&path, "#!/bin/sh\nexit 0\n").expect("fake executable should be written");
        set_fake_executable_permissions(&path);
    }
    bin_dir.to_string_lossy().into_owned()
}

fn fake_kubectl_path(workspace: &Path, exit_code: i32) -> (String, PathBuf) {
    let bin_dir = workspace.join("bin");
    let log_path = workspace.join("kubectl.log");
    std::fs::create_dir_all(&bin_dir).expect("fake PATH bin directory should be created");
    let path = bin_dir.join("kubectl");
    std::fs::write(
        &path,
        format!(
            "#!/bin/sh\nfirst=1\nfor arg in \"$@\"; do\n  if [ \"$first\" -eq 1 ]; then first=0; else printf '\\t' >> \"$KPLY_FAKE_KUBECTL_LOG\"; fi\n  printf '%s' \"$arg\" >> \"$KPLY_FAKE_KUBECTL_LOG\"\ndone\nprintf '\\n' >> \"$KPLY_FAKE_KUBECTL_LOG\"\necho fake kubectl >&2\nexit {exit_code}\n"
        ),
    )
    .expect("fake kubectl should be written");
    set_fake_executable_permissions(&path);

    (bin_dir.to_string_lossy().into_owned(), log_path)
}

#[cfg(unix)]
fn set_fake_executable_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = std::fs::metadata(path)
        .expect("fake executable metadata should be readable")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("fake executable permissions should be set");
}

#[cfg(not(unix))]
fn set_fake_executable_permissions(_path: &Path) {}

#[test]
fn prints_placeholder_text() {
    let output = kply_cmd().assert().success().get_output().stdout.clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("placeholder_text", output);
}

#[test]
fn prints_placeholder_json() {
    let output = kply_cmd()
        .arg("--json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!("placeholder_json", value);
}

#[test]
fn prints_version_text() {
    let output = kply_cmd()
        .arg("--version")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("version_text", output);
}

#[test]
fn prints_version_json() {
    let output = kply_cmd()
        .args(["--version", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let output = normalize_output(&output);
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!("version_json", value);
}

#[test]
fn prints_help_flag() {
    let output = kply_cmd()
        .arg("--help")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("help_flag", output);
}

#[test]
fn prints_help_command() {
    let output = kply_cmd()
        .arg("help")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("help_command", output);
}

#[test]
fn prints_command_group_placeholders() {
    for command in Command::PLACEHOLDER_GROUPS {
        let command = command.name();
        let output = kply_cmd()
            .arg(command)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let output = String::from_utf8(output).expect("stdout should be UTF-8");
        insta::assert_snapshot!(format!("command_group_{command}"), output);
    }
}

#[test]
fn prints_command_group_json_placeholders() {
    for command in Command::PLACEHOLDER_GROUPS {
        let command = command.name();
        let output = kply_cmd()
            .args([command, "--json"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let output = String::from_utf8(output).expect("stdout should be UTF-8");
        let value: serde_json::Value =
            serde_json::from_str(&output).expect("stdout should be JSON");
        insta::assert_json_snapshot!(format!("command_group_{command}_json"), value);
    }
}

#[test]
fn prints_demo_doctor_text() {
    let workspace = temp_workspace();
    let output = kply_cmd()
        .env("PATH", fake_demo_path(workspace.path()))
        .args(["demo", "doctor"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("demo_doctor_text", normalize_output(&output));
}

#[test]
fn requires_explicit_demo_subcommand() {
    let output = assert_kply_exit_code(&["demo"], EXIT_USAGE);

    assert!(
        output.stdout.is_empty(),
        "usage errors should not write stdout"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    insta::assert_snapshot!("demo_requires_explicit_subcommand", stderr);
}

#[test]
fn requires_explicit_demo_subcommand_json() {
    let output = assert_kply_exit_code(&["--json", "demo"], EXIT_USAGE);

    assert!(
        output.stdout.is_empty(),
        "JSON usage errors should not write stdout"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!("demo_requires_explicit_subcommand_json", value);
}

#[test]
fn prints_demo_doctor_json() {
    let workspace = temp_workspace();
    let output = kply_cmd()
        .env("PATH", fake_demo_path(workspace.path()))
        .args(["demo", "doctor", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let output = normalize_output(&output);
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!("demo_doctor_json", value);
}

#[test]
fn reports_demo_doctor_missing_tools() {
    let workspace = temp_workspace();
    let empty_path = workspace.path().join("empty-bin");
    std::fs::create_dir_all(&empty_path).expect("empty PATH directory should be created");
    let output = kply_cmd()
        .env("PATH", &empty_path)
        .args(["demo", "doctor"])
        .assert()
        .code(EXIT_BLOCKING)
        .get_output()
        .clone();

    assert!(
        output.stderr.is_empty(),
        "doctor blocking results should write to stdout, not stderr"
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    insta::assert_snapshot!("demo_doctor_missing_tools", normalize_output(&stdout));
}

#[test]
fn prints_demo_install_text() {
    let workspace = temp_workspace();
    let (path, log_path) = fake_kubectl_path(workspace.path(), 0);
    let output = kply_cmd()
        .env("PATH", path)
        .env("KPLY_FAKE_KUBECTL_LOG", &log_path)
        .args(["demo", "install"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("demo_install_text", normalize_output(&output));

    let log = std::fs::read_to_string(log_path).expect("fake kubectl log should be readable");
    insta::assert_snapshot!("demo_install_kubectl_sequence", normalize_output(&log));
}

#[test]
fn prints_demo_install_json() {
    let workspace = temp_workspace();
    let (path, log_path) = fake_kubectl_path(workspace.path(), 0);
    let output = kply_cmd()
        .env("PATH", path)
        .env("KPLY_FAKE_KUBECTL_LOG", &log_path)
        .args(["demo", "install", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!("demo_install_json", value);
}

#[test]
fn reports_demo_install_kubectl_failure() {
    let workspace = temp_workspace();
    let (path, log_path) = fake_kubectl_path(workspace.path(), 7);
    let output = kply_cmd()
        .env("PATH", path)
        .env("KPLY_FAKE_KUBECTL_LOG", &log_path)
        .args(["demo", "install"])
        .assert()
        .code(EXIT_BLOCKING)
        .get_output()
        .stderr
        .clone();

    let output = String::from_utf8(output).expect("stderr should be UTF-8");
    insta::assert_snapshot!("demo_install_kubectl_failure", normalize_output(&output));
}

#[test]
fn reports_demo_install_kubectl_failure_json() {
    let workspace = temp_workspace();
    let (path, log_path) = fake_kubectl_path(workspace.path(), 7);
    let output = kply_cmd()
        .env("PATH", path)
        .env("KPLY_FAKE_KUBECTL_LOG", &log_path)
        .args(["demo", "install", "--json"])
        .assert()
        .code(EXIT_BLOCKING)
        .get_output()
        .stderr
        .clone();

    let output = String::from_utf8(output).expect("stderr should be UTF-8");
    let output = normalize_output(&output);
    let value: serde_json::Value = serde_json::from_str(&output).expect("stderr should be JSON");
    insta::assert_json_snapshot!("demo_install_kubectl_failure_json", value);
}

#[test]
fn reports_demo_install_missing_kubectl() {
    let output = kply_cmd()
        .env("PATH", "")
        .args(["demo", "install"])
        .assert()
        .code(EXIT_BLOCKING)
        .get_output()
        .stderr
        .clone();

    let output = String::from_utf8(output).expect("stderr should be UTF-8");
    insta::assert_snapshot!("demo_install_missing_kubectl", normalize_output(&output));
}

#[test]
fn reports_demo_install_missing_kubectl_json() {
    let output = kply_cmd()
        .env("PATH", "")
        .args(["demo", "install", "--json"])
        .assert()
        .code(EXIT_BLOCKING)
        .get_output()
        .stderr
        .clone();

    let output = String::from_utf8(output).expect("stderr should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stderr should be JSON");
    insta::assert_json_snapshot!("demo_install_missing_kubectl_json", value);
}

#[test]
fn prints_demo_reset_text() {
    let workspace = temp_workspace();
    let (path, log_path) = fake_kubectl_path(workspace.path(), 0);
    let output = kply_cmd()
        .env("PATH", path)
        .env("KPLY_FAKE_KUBECTL_LOG", &log_path)
        .args(["demo", "reset"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("demo_reset_text", normalize_output(&output));

    let log = std::fs::read_to_string(log_path).expect("fake kubectl log should be readable");
    insta::assert_snapshot!("demo_reset_kubectl_sequence", normalize_output(&log));
}

#[test]
fn prints_demo_reset_json() {
    let workspace = temp_workspace();
    let (path, log_path) = fake_kubectl_path(workspace.path(), 0);
    let output = kply_cmd()
        .env("PATH", path)
        .env("KPLY_FAKE_KUBECTL_LOG", &log_path)
        .args(["demo", "reset", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!("demo_reset_json", value);
}

#[test]
fn reports_demo_reset_kubectl_failure() {
    let workspace = temp_workspace();
    let (path, log_path) = fake_kubectl_path(workspace.path(), 7);
    let output = kply_cmd()
        .env("PATH", path)
        .env("KPLY_FAKE_KUBECTL_LOG", &log_path)
        .args(["demo", "reset"])
        .assert()
        .code(EXIT_BLOCKING)
        .get_output()
        .stderr
        .clone();

    let output = String::from_utf8(output).expect("stderr should be UTF-8");
    insta::assert_snapshot!("demo_reset_kubectl_failure", normalize_output(&output));
}

#[test]
fn reports_demo_reset_missing_kubectl() {
    let output = kply_cmd()
        .env("PATH", "")
        .args(["demo", "reset"])
        .assert()
        .code(EXIT_BLOCKING)
        .get_output()
        .stderr
        .clone();

    let output = String::from_utf8(output).expect("stderr should be UTF-8");
    insta::assert_snapshot!("demo_reset_missing_kubectl", normalize_output(&output));
}

#[test]
fn reports_demo_reset_missing_kubectl_json() {
    let output = kply_cmd()
        .env("PATH", "")
        .args(["demo", "reset", "--json"])
        .assert()
        .code(EXIT_BLOCKING)
        .get_output()
        .stderr
        .clone();

    let output = String::from_utf8(output).expect("stderr should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stderr should be JSON");
    insta::assert_json_snapshot!("demo_reset_missing_kubectl_json", value);
}

#[test]
fn prints_demo_teardown_text() {
    let workspace = temp_workspace();
    let (path, log_path) = fake_kubectl_path(workspace.path(), 0);
    let output = kply_cmd()
        .env("PATH", path)
        .env("KPLY_FAKE_KUBECTL_LOG", &log_path)
        .args(["demo", "teardown"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("demo_teardown_text", normalize_output(&output));

    let log = std::fs::read_to_string(log_path).expect("fake kubectl log should be readable");
    insta::assert_snapshot!("demo_teardown_kubectl_sequence", normalize_output(&log));
}

#[test]
fn prints_demo_teardown_json() {
    let workspace = temp_workspace();
    let (path, log_path) = fake_kubectl_path(workspace.path(), 0);
    let output = kply_cmd()
        .env("PATH", path)
        .env("KPLY_FAKE_KUBECTL_LOG", &log_path)
        .args(["demo", "teardown", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!("demo_teardown_json", value);
}

#[test]
fn reports_demo_teardown_kubectl_failure() {
    let workspace = temp_workspace();
    let (path, log_path) = fake_kubectl_path(workspace.path(), 7);
    let output = kply_cmd()
        .env("PATH", path)
        .env("KPLY_FAKE_KUBECTL_LOG", &log_path)
        .args(["demo", "teardown"])
        .assert()
        .code(EXIT_BLOCKING)
        .get_output()
        .stderr
        .clone();

    let output = String::from_utf8(output).expect("stderr should be UTF-8");
    insta::assert_snapshot!("demo_teardown_kubectl_failure", normalize_output(&output));
}

#[test]
fn reports_demo_teardown_missing_kubectl() {
    let output = kply_cmd()
        .env("PATH", "")
        .args(["demo", "teardown"])
        .assert()
        .code(EXIT_BLOCKING)
        .get_output()
        .stderr
        .clone();

    let output = String::from_utf8(output).expect("stderr should be UTF-8");
    insta::assert_snapshot!("demo_teardown_missing_kubectl", normalize_output(&output));
}

#[test]
fn reports_demo_teardown_missing_kubectl_json() {
    let output = kply_cmd()
        .env("PATH", "")
        .args(["demo", "teardown", "--json"])
        .assert()
        .code(EXIT_BLOCKING)
        .get_output()
        .stderr
        .clone();

    let output = String::from_utf8(output).expect("stderr should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stderr should be JSON");
    insta::assert_json_snapshot!("demo_teardown_missing_kubectl_json", value);
}

#[test]
fn prints_session_plan_placeholder_text() {
    let output = with_session_plan_config(|config_path| {
        kply_cmd()
            .args(["--config", config_path, "session", "plan", "checkout"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    });

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("session_plan_placeholder_text", output);
}

#[test]
fn prints_session_plan_placeholder_json() {
    let output = with_session_plan_config(|config_path| {
        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                "plan",
                "checkout",
                "--json",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    });

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!("session_plan_placeholder_json", value);
}

#[test]
fn prints_session_plan_placeholder_text_with_image() {
    let output = with_session_plan_config(|config_path| {
        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                "plan",
                "checkout",
                "--image",
                "ghcr.io/kubeply/checkout:v2",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    });

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("session_plan_placeholder_text_with_image", output);
}

#[test]
fn prints_session_plan_placeholder_json_with_image() {
    let output = with_session_plan_config(|config_path| {
        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                "plan",
                "checkout",
                "--image",
                "ghcr.io/kubeply/checkout:v2",
                "--json",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    });

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!("session_plan_placeholder_json_with_image", value);
}

#[test]
fn prints_session_plan_placeholder_text_with_namespace() {
    let output = with_session_plan_config(|config_path| {
        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                "plan",
                "checkout",
                "--namespace",
                "staging",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    });

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("session_plan_placeholder_text_with_namespace", output);
}

#[test]
fn prints_session_plan_placeholder_json_with_namespace() {
    let output = with_session_plan_config(|config_path| {
        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                "plan",
                "checkout",
                "--namespace",
                "staging",
                "--json",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    });

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!("session_plan_placeholder_json_with_namespace", value);
}

#[test]
fn prints_session_plan_placeholder_text_with_ttl() {
    let output = with_session_plan_config(|config_path| {
        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                "plan",
                "checkout",
                "--ttl",
                "30m",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    });

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("session_plan_placeholder_text_with_ttl", output);
}

#[test]
fn prints_session_plan_placeholder_json_with_ttl() {
    let output = with_session_plan_config(|config_path| {
        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                "plan",
                "checkout",
                "--ttl",
                "30m",
                "--json",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    });

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!("session_plan_placeholder_json_with_ttl", value);
}

#[test]
fn prints_session_plan_placeholder_text_with_route_strategy() {
    let output = with_session_plan_config(|config_path| {
        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                "plan",
                "checkout",
                "--route-strategy",
                "host",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    });

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("session_plan_placeholder_text_with_route_strategy", output);
}

#[test]
fn prints_session_plan_placeholder_json_with_route_strategy() {
    let output = with_session_plan_config(|config_path| {
        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                "plan",
                "checkout",
                "--route-strategy",
                "host",
                "--json",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    });

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!("session_plan_placeholder_json_with_route_strategy", value);
}

#[test]
fn prints_session_plan_placeholder_json_with_auto_route_strategy() {
    let output = with_session_plan_config(|config_path| {
        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                "plan",
                "checkout",
                "--route-strategy",
                "auto",
                "--json",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    });

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!(
        "session_plan_placeholder_json_with_auto_route_strategy",
        value
    );
}

#[test]
fn prints_session_plan_placeholder_json_with_none_route_strategy() {
    let output = with_session_plan_config(|config_path| {
        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                "plan",
                "checkout",
                "--route-strategy",
                "none",
                "--json",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    });

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!(
        "session_plan_placeholder_json_with_none_route_strategy",
        value
    );
}

#[test]
fn prints_session_plan_placeholder_json_with_preview_service_route_strategy() {
    let output = with_session_plan_config(|config_path| {
        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                "plan",
                "checkout",
                "--route-strategy",
                "preview-service",
                "--json",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    });

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!(
        "session_plan_placeholder_json_with_preview_service_route_strategy",
        value
    );
}

#[test]
fn prints_session_plan_placeholder_text_with_image_and_namespace() {
    let output = with_session_plan_config(|config_path| {
        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                "plan",
                "checkout",
                "--image",
                "ghcr.io/kubeply/checkout:v2",
                "--namespace",
                "staging",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    });

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!(
        "session_plan_placeholder_text_with_image_and_namespace",
        output
    );
}

#[test]
fn prints_session_plan_placeholder_json_with_image_and_namespace() {
    let output = with_session_plan_config(|config_path| {
        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                "plan",
                "checkout",
                "--image",
                "ghcr.io/kubeply/checkout:v2",
                "--namespace",
                "staging",
                "--json",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    });

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!(
        "session_plan_placeholder_json_with_image_and_namespace",
        value
    );
}

#[test]
fn prints_session_plan_placeholder_text_with_all_overrides() {
    let output = with_session_plan_config(|config_path| {
        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                "plan",
                "checkout",
                "--image",
                "ghcr.io/kubeply/checkout:v2",
                "--namespace",
                "staging",
                "--ttl",
                "30m",
                "--route-strategy",
                "header",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    });

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("session_plan_placeholder_text_with_all_overrides", output);
}

#[test]
fn prints_session_plan_placeholder_text_with_warnings_and_risks() {
    let output = with_session_plan_risk_config(|config_path| {
        kply_cmd()
            .args(["--config", config_path, "session", "plan", "checkout-db"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    });

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!(
        "session_plan_placeholder_text_with_warnings_and_risks",
        output
    );
}

#[test]
fn prints_session_plan_placeholder_json_with_warnings_and_risks() {
    let output = with_session_plan_risk_config(|config_path| {
        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                "plan",
                "checkout-db",
                "--json",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    });

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!(
        "session_plan_placeholder_json_with_warnings_and_risks",
        value
    );
}

#[test]
fn prints_session_plan_placeholder_json_with_all_overrides() {
    let output = with_session_plan_config(|config_path| {
        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                "plan",
                "checkout",
                "--image",
                "ghcr.io/kubeply/checkout:v2",
                "--namespace",
                "staging",
                "--ttl",
                "30m",
                "--route-strategy",
                "header",
                "--json",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    });

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!("session_plan_placeholder_json_with_all_overrides", value);
}

#[test]
fn prints_session_create_text() {
    let output = with_session_plan_config(|config_path| {
        kply_cmd()
            .args(["--config", config_path, "session", "create", "checkout"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    });

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("session_create_text", output);
}

#[test]
fn prints_session_create_json() {
    let output = with_session_plan_config(|config_path| {
        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                "create",
                "checkout",
                "--json",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    });

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!("session_create_json", value);
}

#[test]
fn rejects_session_create_apply_without_kubeconfig() {
    let workspace = temp_workspace();
    let missing_kubeconfig_path = workspace.path().join("missing").join("kubeconfig.yaml");
    let missing_kubeconfig = missing_kubeconfig_path
        .to_str()
        .expect("missing kubeconfig path should be UTF-8");

    let output = with_session_plan_config(|config_path| {
        kply_cmd()
            .env("KUBECONFIG", missing_kubeconfig)
            .args([
                "--config",
                config_path,
                "session",
                "create",
                "checkout",
                "--apply",
            ])
            .assert()
            .code(EXIT_USAGE)
            .get_output()
            .clone()
    });

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(
        !stderr.contains(missing_kubeconfig),
        "mutation errors should not leak the configured kubeconfig path"
    );
    insta::assert_snapshot!(
        "session_create_apply_missing_kubeconfig",
        normalize_output(&stderr)
    );
}

#[test]
fn rejects_session_create_apply_without_kubeconfig_json() {
    let workspace = temp_workspace();
    let missing_kubeconfig_path = workspace.path().join("missing").join("kubeconfig.yaml");
    let missing_kubeconfig = missing_kubeconfig_path
        .to_str()
        .expect("missing kubeconfig path should be UTF-8");

    let output = with_session_plan_config(|config_path| {
        kply_cmd()
            .env("KUBECONFIG", missing_kubeconfig)
            .args([
                "--config",
                config_path,
                "session",
                "create",
                "checkout",
                "--apply",
                "--json",
            ])
            .assert()
            .code(EXIT_USAGE)
            .get_output()
            .clone()
    });

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(
        !stderr.contains(missing_kubeconfig),
        "mutation JSON errors should not leak the configured kubeconfig path"
    );
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!("session_create_apply_missing_kubeconfig_json", value);
}

#[test]
fn rejects_session_list_without_kubeconfig() {
    let workspace = temp_workspace();
    let missing_kubeconfig_path = workspace.path().join("missing").join("kubeconfig.yaml");
    let missing_kubeconfig = missing_kubeconfig_path
        .to_str()
        .expect("missing kubeconfig path should be UTF-8");

    let output = kply_cmd()
        .env("KUBECONFIG", missing_kubeconfig)
        .args(["session", "list", "--namespace", "shop"])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(
        !stderr.contains(missing_kubeconfig),
        "session list errors should not leak the configured kubeconfig path"
    );
    insta::assert_snapshot!("session_list_missing_kubeconfig", normalize_output(&stderr));
}

#[test]
fn rejects_session_list_without_kubeconfig_json() {
    let workspace = temp_workspace();
    let missing_kubeconfig_path = workspace.path().join("missing").join("kubeconfig.yaml");
    let missing_kubeconfig = missing_kubeconfig_path
        .to_str()
        .expect("missing kubeconfig path should be UTF-8");

    let output = kply_cmd()
        .env("KUBECONFIG", missing_kubeconfig)
        .args(["session", "list", "--namespace", "shop", "--json"])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(
        !stderr.contains(missing_kubeconfig),
        "session list JSON errors should not leak the configured kubeconfig path"
    );
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!("session_list_missing_kubeconfig_json", value);
}

#[test]
fn rejects_session_status_without_kubeconfig() {
    let workspace = temp_workspace();
    let missing_kubeconfig_path = workspace.path().join("missing").join("kubeconfig.yaml");
    let missing_kubeconfig = missing_kubeconfig_path
        .to_str()
        .expect("missing kubeconfig path should be UTF-8");

    let output = kply_cmd()
        .env("KUBECONFIG", missing_kubeconfig)
        .args(["session", "status", "checkout-plan", "--namespace", "shop"])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(
        !stderr.contains(missing_kubeconfig),
        "session status errors should not leak the configured kubeconfig path"
    );
    insta::assert_snapshot!(
        "session_status_missing_kubeconfig",
        normalize_output(&stderr)
    );
}

#[test]
fn rejects_session_status_without_kubeconfig_json() {
    let workspace = temp_workspace();
    let missing_kubeconfig_path = workspace.path().join("missing").join("kubeconfig.yaml");
    let missing_kubeconfig = missing_kubeconfig_path
        .to_str()
        .expect("missing kubeconfig path should be UTF-8");

    let output = kply_cmd()
        .env("KUBECONFIG", missing_kubeconfig)
        .args([
            "session",
            "status",
            "checkout-plan",
            "--namespace",
            "shop",
            "--json",
        ])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(
        !stderr.contains(missing_kubeconfig),
        "session status JSON errors should not leak the configured kubeconfig path"
    );
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!("session_status_missing_kubeconfig_json", value);
}

#[test]
fn rejects_report_show_invalid_session() {
    let output = kply_cmd()
        .args(["report", "show", "Checkout", "--namespace", "shop"])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    insta::assert_snapshot!("report_show_invalid_session", normalize_output(&stderr));
}

#[test]
fn rejects_report_show_invalid_session_json() {
    let output = kply_cmd()
        .args([
            "report",
            "show",
            "Checkout",
            "--namespace",
            "shop",
            "--json",
        ])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!("report_show_invalid_session_json", value);
}

#[test]
fn rejects_report_show_without_kubeconfig() {
    let workspace = temp_workspace();
    let missing_kubeconfig_path = workspace.path().join("missing").join("kubeconfig.yaml");
    let missing_kubeconfig = missing_kubeconfig_path
        .to_str()
        .expect("missing kubeconfig path should be UTF-8");

    let output = kply_cmd()
        .env("KUBECONFIG", missing_kubeconfig)
        .args(["report", "show", "checkout-plan", "--namespace", "shop"])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(
        !stderr.contains(missing_kubeconfig),
        "report show errors should not leak the configured kubeconfig path"
    );
    insta::assert_snapshot!("report_show_missing_kubeconfig", normalize_output(&stderr));
}

#[test]
fn rejects_report_show_without_kubeconfig_json() {
    let workspace = temp_workspace();
    let missing_kubeconfig_path = workspace.path().join("missing").join("kubeconfig.yaml");
    let missing_kubeconfig = missing_kubeconfig_path
        .to_str()
        .expect("missing kubeconfig path should be UTF-8");

    let output = kply_cmd()
        .env("KUBECONFIG", missing_kubeconfig)
        .args([
            "report",
            "show",
            "checkout-plan",
            "--namespace",
            "shop",
            "--json",
        ])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(
        !stderr.contains(missing_kubeconfig),
        "report show JSON errors should not leak the configured kubeconfig path"
    );
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!("report_show_missing_kubeconfig_json", value);
}

#[test]
fn rejects_report_export_invalid_session_json() {
    let output = kply_cmd()
        .args([
            "report",
            "export",
            "Checkout",
            "--namespace",
            "shop",
            "--format",
            "json",
        ])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!("report_export_invalid_session_json", value);
}

#[test]
fn rejects_report_export_without_kubeconfig_json() {
    let workspace = temp_workspace();
    let missing_kubeconfig_path = workspace.path().join("missing").join("kubeconfig.yaml");
    let missing_kubeconfig = missing_kubeconfig_path
        .to_str()
        .expect("missing kubeconfig path should be UTF-8");

    let output = kply_cmd()
        .env("KUBECONFIG", missing_kubeconfig)
        .args([
            "report",
            "export",
            "checkout-plan",
            "--namespace",
            "shop",
            "--format",
            "json",
        ])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(
        !stderr.contains(missing_kubeconfig),
        "report export JSON errors should not leak the configured kubeconfig path"
    );
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!("report_export_missing_kubeconfig_json", value);
}

#[test]
fn rejects_report_export_without_kubeconfig_default_namespace_json() {
    let workspace = temp_workspace();
    let missing_kubeconfig_path = workspace.path().join("missing").join("kubeconfig.yaml");
    let missing_kubeconfig = missing_kubeconfig_path
        .to_str()
        .expect("missing kubeconfig path should be UTF-8");

    let output = kply_cmd()
        .env("KUBECONFIG", missing_kubeconfig)
        .args(["report", "export", "checkout-plan", "--format", "json"])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(
        !stderr.contains(missing_kubeconfig),
        "report export JSON errors should not leak the configured kubeconfig path"
    );
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!(
        "report_export_default_namespace_missing_kubeconfig_json",
        value
    );
}

#[test]
fn rejects_check_run_without_kubeconfig() {
    let workspace = temp_workspace();
    let missing_kubeconfig_path = workspace.path().join("missing").join("kubeconfig.yaml");
    let missing_kubeconfig = missing_kubeconfig_path
        .to_str()
        .expect("missing kubeconfig path should be UTF-8");

    let output = kply_cmd()
        .env("KUBECONFIG", missing_kubeconfig)
        .args(["check", "run", "checkout-plan", "--namespace", "shop"])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(
        !stderr.contains(missing_kubeconfig),
        "check run errors should not leak the configured kubeconfig path"
    );
    insta::assert_snapshot!("check_run_missing_kubeconfig", normalize_output(&stderr));
}

#[test]
fn rejects_check_run_without_kubeconfig_json() {
    let workspace = temp_workspace();
    let missing_kubeconfig_path = workspace.path().join("missing").join("kubeconfig.yaml");
    let missing_kubeconfig = missing_kubeconfig_path
        .to_str()
        .expect("missing kubeconfig path should be UTF-8");

    let output = kply_cmd()
        .env("KUBECONFIG", missing_kubeconfig)
        .args([
            "check",
            "run",
            "checkout-plan",
            "--namespace",
            "shop",
            "--json",
        ])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(
        !stderr.contains(missing_kubeconfig),
        "check run JSON errors should not leak the configured kubeconfig path"
    );
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!("check_run_missing_kubeconfig_json", value);
}

#[test]
fn rejects_session_cleanup_apply_without_kubeconfig() {
    let workspace = temp_workspace();
    let missing_kubeconfig_path = workspace.path().join("missing").join("kubeconfig.yaml");
    let missing_kubeconfig = missing_kubeconfig_path
        .to_str()
        .expect("missing kubeconfig path should be UTF-8");

    let output = kply_cmd()
        .env("KUBECONFIG", missing_kubeconfig)
        .args([
            "session",
            "cleanup",
            "checkout-plan",
            "--namespace",
            "shop",
            "--apply",
        ])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(
        !stderr.contains(missing_kubeconfig),
        "cleanup mutation errors should not leak the configured kubeconfig path"
    );
    insta::assert_snapshot!(
        "session_cleanup_apply_missing_kubeconfig",
        normalize_output(&stderr)
    );
}

#[test]
fn rejects_session_cleanup_apply_without_kubeconfig_json() {
    let workspace = temp_workspace();
    let missing_kubeconfig_path = workspace.path().join("missing").join("kubeconfig.yaml");
    let missing_kubeconfig = missing_kubeconfig_path
        .to_str()
        .expect("missing kubeconfig path should be UTF-8");

    let output = kply_cmd()
        .env("KUBECONFIG", missing_kubeconfig)
        .args([
            "session",
            "cleanup",
            "checkout-plan",
            "--namespace",
            "shop",
            "--apply",
            "--json",
        ])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(
        !stderr.contains(missing_kubeconfig),
        "cleanup mutation JSON errors should not leak the configured kubeconfig path"
    );
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!("session_cleanup_apply_missing_kubeconfig_json", value);
}

#[test]
fn rejects_session_cleanup_apply_default_namespace_without_kubeconfig_json() {
    let workspace = temp_workspace();
    let missing_kubeconfig_path = workspace.path().join("missing").join("kubeconfig.yaml");
    let missing_kubeconfig = missing_kubeconfig_path
        .to_str()
        .expect("missing kubeconfig path should be UTF-8");

    let output = kply_cmd()
        .env("KUBECONFIG", missing_kubeconfig)
        .args(["session", "cleanup", "checkout-plan", "--apply", "--json"])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(
        !stderr.contains(missing_kubeconfig),
        "cleanup mutation JSON errors should not leak the configured kubeconfig path"
    );
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!(
        "session_cleanup_apply_default_namespace_missing_kubeconfig_json",
        value
    );
}

#[test]
fn rejects_session_cleanup_apply_default_namespace_without_kubeconfig() {
    let workspace = temp_workspace();
    let missing_kubeconfig_path = workspace.path().join("missing").join("kubeconfig.yaml");
    let missing_kubeconfig = missing_kubeconfig_path
        .to_str()
        .expect("missing kubeconfig path should be UTF-8");

    let output = kply_cmd()
        .env("KUBECONFIG", missing_kubeconfig)
        .args(["session", "cleanup", "checkout-plan", "--apply"])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(
        !stderr.contains(missing_kubeconfig),
        "cleanup mutation errors should not leak the configured kubeconfig path"
    );
    insta::assert_snapshot!(
        "session_cleanup_apply_default_namespace_missing_kubeconfig",
        normalize_output(&stderr)
    );
}

#[test]
fn rejects_session_cleanup_dry_run_without_kubeconfig() {
    let workspace = temp_workspace();
    let missing_kubeconfig_path = workspace.path().join("missing").join("kubeconfig.yaml");
    let missing_kubeconfig = missing_kubeconfig_path
        .to_str()
        .expect("missing kubeconfig path should be UTF-8");

    let output = kply_cmd()
        .env("KUBECONFIG", missing_kubeconfig)
        .args([
            "session",
            "cleanup",
            "checkout-plan",
            "--namespace",
            "shop",
            "--dry-run",
        ])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(
        !stderr.contains(missing_kubeconfig),
        "cleanup dry-run errors should not leak the configured kubeconfig path"
    );
    insta::assert_snapshot!(
        "session_cleanup_dry_run_missing_kubeconfig",
        normalize_output(&stderr)
    );
}

#[test]
fn rejects_session_cleanup_dry_run_without_kubeconfig_json() {
    let workspace = temp_workspace();
    let missing_kubeconfig_path = workspace.path().join("missing").join("kubeconfig.yaml");
    let missing_kubeconfig = missing_kubeconfig_path
        .to_str()
        .expect("missing kubeconfig path should be UTF-8");

    let output = kply_cmd()
        .env("KUBECONFIG", missing_kubeconfig)
        .args([
            "session",
            "cleanup",
            "checkout-plan",
            "--namespace",
            "shop",
            "--dry-run",
            "--json",
        ])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(
        !stderr.contains(missing_kubeconfig),
        "cleanup dry-run JSON errors should not leak the configured kubeconfig path"
    );
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!("session_cleanup_dry_run_missing_kubeconfig_json", value);
}

#[test]
fn rejects_invalid_session_status_id() {
    let output = kply_cmd()
        .args(["session", "status", "Checkout_Plan"])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    insta::assert_snapshot!("session_status_invalid_id", normalize_output(&stderr));
}

#[test]
fn rejects_invalid_session_status_id_json() {
    let output = kply_cmd()
        .args(["session", "status", "Checkout_Plan", "--json"])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!("session_status_invalid_id_json", value);
}

#[test]
fn rejects_invalid_check_run_session_id() {
    let output = kply_cmd()
        .args(["check", "run", "Checkout_Plan"])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    insta::assert_snapshot!("check_run_invalid_session_id", normalize_output(&stderr));
}

#[test]
fn rejects_invalid_check_run_session_id_json() {
    let output = kply_cmd()
        .args(["check", "run", "Checkout_Plan", "--json"])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!("check_run_invalid_session_id_json", value);
}

#[test]
fn prints_session_cleanup_text() {
    let output = kply_cmd()
        .args(["session", "cleanup", "checkout-plan"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("session_cleanup_text", output);
}

#[test]
fn prints_session_cleanup_json() {
    let output = kply_cmd()
        .args(["session", "cleanup", "checkout-plan", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!("session_cleanup_json", value);
}

#[test]
fn rejects_session_cleanup_namespace_without_apply() {
    let output = kply_cmd()
        .args(["session", "cleanup", "checkout-plan", "--namespace", "shop"])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    insta::assert_snapshot!(
        "session_cleanup_namespace_without_apply",
        normalize_output(&stderr)
    );
}

#[test]
fn rejects_session_cleanup_namespace_without_apply_json() {
    let output = kply_cmd()
        .args([
            "session",
            "cleanup",
            "checkout-plan",
            "--namespace",
            "shop",
            "--json",
        ])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!("session_cleanup_namespace_without_apply_json", value);
}

#[test]
fn rejects_session_cleanup_apply_with_dry_run() {
    let output = kply_cmd()
        .args([
            "session",
            "cleanup",
            "checkout-plan",
            "--apply",
            "--dry-run",
        ])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    insta::assert_snapshot!(
        "session_cleanup_apply_with_dry_run",
        normalize_output(&stderr)
    );
}

#[test]
fn rejects_session_cleanup_apply_with_dry_run_json() {
    let output = kply_cmd()
        .args([
            "session",
            "cleanup",
            "checkout-plan",
            "--apply",
            "--dry-run",
            "--json",
        ])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!("session_cleanup_apply_with_dry_run_json", value);
}

#[test]
fn rejects_invalid_session_cleanup_id() {
    let output = kply_cmd()
        .args(["session", "cleanup", "Checkout_Plan"])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    insta::assert_snapshot!("session_cleanup_invalid_id", normalize_output(&stderr));
}

#[test]
fn rejects_invalid_session_cleanup_id_json() {
    let output = kply_cmd()
        .args(["session", "cleanup", "Checkout_Plan", "--json"])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!("session_cleanup_invalid_id_json", value);
}

#[test]
fn prints_route_plan_text() {
    let output = kply_cmd()
        .args(["route", "plan", "checkout-plan", "--namespace", "shop"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("route_plan_text", output);
}

#[test]
fn prints_route_plan_json() {
    let output = kply_cmd()
        .args([
            "route",
            "plan",
            "checkout-plan",
            "--namespace",
            "shop",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!("route_plan_json", value);
}

#[test]
fn prints_route_plan_without_namespace_json() {
    let output = kply_cmd()
        .args(["route", "plan", "checkout-plan", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!("route_plan_without_namespace_json", value);
}

#[test]
fn suppresses_route_plan_text_when_quiet() {
    kply_cmd()
        .args(["route", "plan", "checkout-plan", "--quiet"])
        .assert()
        .success()
        .stdout("");
}

#[test]
fn rejects_invalid_route_plan_session_id() {
    let output = assert_kply_exit_code(&["route", "plan", "Checkout_Plan"], EXIT_USAGE);

    assert!(
        output.stdout.is_empty(),
        "invalid route plan session id should not write stdout"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    insta::assert_snapshot!("route_plan_invalid_session_id", normalize_output(&stderr));
}

#[test]
fn rejects_invalid_route_plan_session_id_json() {
    let output = assert_kply_exit_code(&["route", "plan", "Checkout_Plan", "--json"], EXIT_USAGE);

    assert!(
        output.stdout.is_empty(),
        "invalid route plan session id JSON should not write stdout"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!("route_plan_invalid_session_id_json", value);
}

#[test]
fn prints_route_apply_text() {
    let output = kply_cmd()
        .args([
            "route",
            "apply",
            "checkout-plan",
            "--namespace",
            "shop",
            "--confirm-route-mutation",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("route_apply_text", output);
}

#[test]
fn prints_route_apply_json() {
    let output = kply_cmd()
        .args([
            "route",
            "apply",
            "checkout-plan",
            "--namespace",
            "shop",
            "--confirm-route-mutation",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!("route_apply_json", value);
}

#[test]
fn suppresses_route_apply_text_when_quiet() {
    kply_cmd()
        .args([
            "route",
            "apply",
            "checkout-plan",
            "--quiet",
            "--confirm-route-mutation",
        ])
        .assert()
        .success()
        .stdout("");
}

#[test]
fn rejects_route_apply_without_confirmation() {
    let output = assert_kply_exit_code(&["route", "apply", "checkout-plan"], EXIT_USAGE);

    assert!(
        output.stdout.is_empty(),
        "unconfirmed route apply should not write stdout"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    insta::assert_snapshot!(
        "route_apply_missing_confirmation",
        normalize_output(&stderr)
    );
}

#[test]
fn rejects_route_apply_without_confirmation_json() {
    let output = assert_kply_exit_code(&["route", "apply", "checkout-plan", "--json"], EXIT_USAGE);

    assert!(
        output.stdout.is_empty(),
        "unconfirmed route apply JSON should not write stdout"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!("route_apply_missing_confirmation_json", value);
}

#[test]
fn rejects_invalid_route_apply_session_id() {
    let output = assert_kply_exit_code(&["route", "apply", "Checkout_Plan"], EXIT_USAGE);

    assert!(
        output.stdout.is_empty(),
        "invalid route apply session id should not write stdout"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    insta::assert_snapshot!("route_apply_invalid_session_id", normalize_output(&stderr));
}

#[test]
fn rejects_invalid_route_apply_session_id_json() {
    let output = assert_kply_exit_code(&["route", "apply", "Checkout_Plan", "--json"], EXIT_USAGE);

    assert!(
        output.stdout.is_empty(),
        "invalid route apply session id JSON should not write stdout"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!("route_apply_invalid_session_id_json", value);
}

#[test]
fn prints_route_cleanup_text() {
    let output = kply_cmd()
        .args(["route", "cleanup", "checkout-plan", "--namespace", "shop"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("route_cleanup_text", output);
}

#[test]
fn prints_route_cleanup_json() {
    let output = kply_cmd()
        .args([
            "route",
            "cleanup",
            "checkout-plan",
            "--namespace",
            "shop",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!("route_cleanup_json", value);
}

#[test]
fn prints_route_cleanup_without_namespace_json() {
    let output = kply_cmd()
        .args(["route", "cleanup", "checkout-plan", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!("route_cleanup_without_namespace_json", value);
}

#[test]
fn suppresses_route_cleanup_text_when_quiet() {
    kply_cmd()
        .args(["route", "cleanup", "checkout-plan", "--quiet"])
        .assert()
        .success()
        .stdout("");
}

#[test]
fn rejects_invalid_route_cleanup_session_id() {
    let output = assert_kply_exit_code(&["route", "cleanup", "Checkout_Plan"], EXIT_USAGE);

    assert!(
        output.stdout.is_empty(),
        "invalid route cleanup session id should not write stdout"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    insta::assert_snapshot!(
        "route_cleanup_invalid_session_id",
        normalize_output(&stderr)
    );
}

#[test]
fn rejects_invalid_route_cleanup_session_id_json() {
    let output =
        assert_kply_exit_code(&["route", "cleanup", "Checkout_Plan", "--json"], EXIT_USAGE);

    assert!(
        output.stdout.is_empty(),
        "invalid route cleanup session id JSON should not write stdout"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!("route_cleanup_invalid_session_id_json", value);
}

#[test]
fn prints_session_manifests_text() {
    let output = with_session_plan_config(|config_path| {
        kply_cmd()
            .args(["--config", config_path, "session", "manifests", "checkout"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    });

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("session_manifests_text", output);
}

#[test]
fn prints_session_manifests_json() {
    let output = with_session_plan_config(|config_path| {
        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                "manifests",
                "checkout",
                "--json",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    });

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!("session_manifests_json", value);
}

#[test]
fn prints_session_manifests_yaml() {
    let output = with_session_plan_config(|config_path| {
        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                "manifests",
                "checkout",
                "--yaml",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    });

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("session_manifests_yaml", output);
}

#[test]
fn prints_session_manifests_text_without_route_selector() {
    let output = with_session_plan_config(|config_path| {
        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                "manifests",
                "checkout",
                "--route-strategy",
                "preview",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    });

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("session_manifests_text_without_route_selector", output);
}

#[test]
fn prints_session_manifests_json_without_route_selector() {
    let output = with_session_plan_config(|config_path| {
        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                "manifests",
                "checkout",
                "--route-strategy",
                "preview",
                "--json",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    });

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!("session_manifests_json_without_route_selector", value);
}

#[test]
fn prints_session_manifests_yaml_without_route_selector() {
    let output = with_session_plan_config(|config_path| {
        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                "manifests",
                "checkout",
                "--route-strategy",
                "preview",
                "--yaml",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    });

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("session_manifests_yaml_without_route_selector", output);
}

#[test]
fn prints_session_manifests_text_for_statefulset_workload() {
    let output = with_session_plan_statefulset_config(|config_path| {
        kply_cmd()
            .args(["--config", config_path, "session", "manifests", "cart"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    });

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("session_manifests_text_for_statefulset_workload", output);
}

#[test]
fn suppresses_session_plan_placeholder_text_when_quiet() {
    with_session_plan_config(|config_path| {
        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                SessionCommand::Create {
                    app: String::new(),
                    apply: false,
                    image: None,
                    namespace: None,
                    time_to_live: None,
                    route_strategy: None,
                }
                .name(),
                "checkout",
            ])
            .assert()
            .success();
        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                "plan",
                "checkout",
                "--quiet",
            ])
            .assert()
            .success()
            .stdout("");

        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                "plan",
                "checkout",
                "--image",
                "ghcr.io/kubeply/checkout:v2",
                "--quiet",
            ])
            .assert()
            .success()
            .stdout("");

        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                "plan",
                "checkout",
                "--image",
                "ghcr.io/kubeply/checkout:v2",
                "--namespace",
                "staging",
                "--ttl",
                "30m",
                "--route-strategy",
                "header",
                "--quiet",
            ])
            .assert()
            .success()
            .stdout("");
    });
}

#[test]
fn suppresses_session_create_text_when_quiet() {
    with_session_plan_config(|config_path| {
        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                "create",
                "checkout",
                "--quiet",
            ])
            .assert()
            .success()
            .stdout("");
    });
}

#[test]
fn suppresses_session_manifests_text_when_quiet() {
    with_session_plan_config(|config_path| {
        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                "manifests",
                "checkout",
                "--quiet",
            ])
            .assert()
            .success()
            .stdout("");
    });
}

#[test]
fn keeps_session_manifests_yaml_when_quiet() {
    with_session_plan_config(|config_path| {
        let output = kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                "manifests",
                "checkout",
                "--yaml",
                "--quiet",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let output = String::from_utf8(output).expect("stdout should be UTF-8");
        assert!(output.contains("---\napiVersion: apps/v1\n"));
    });
}

#[test]
fn rejects_session_manifests_yaml_with_json() {
    let output = assert_kply_exit_code(
        &["session", "manifests", "checkout", "--yaml", "--json"],
        EXIT_USAGE,
    );

    assert!(
        output.stdout.is_empty(),
        "usage errors should not write stdout"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!("session_manifests_yaml_with_json", value);
}

#[test]
fn rejects_session_plan_missing_image() {
    let output = with_session_plan_no_image_config(|config_path| {
        assert_kply_exit_code(
            &["--config", config_path, "session", "plan", "checkout"],
            EXIT_USAGE,
        )
    });

    assert!(
        output.stdout.is_empty(),
        "missing image errors should not write stdout"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    insta::assert_snapshot!("session_plan_missing_image", stderr);
}

#[test]
fn rejects_session_plan_missing_app_json() {
    let output = with_session_plan_config(|config_path| {
        assert_kply_exit_code(
            &[
                "--json",
                "--config",
                config_path,
                "session",
                "plan",
                "missing",
            ],
            EXIT_USAGE,
        )
    });

    assert!(
        output.stdout.is_empty(),
        "missing app errors should not write stdout"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!("session_plan_missing_app_json", value);
}

#[test]
fn rejects_session_plan_invalid_ttl_json() {
    let output = with_session_plan_config(|config_path| {
        assert_kply_exit_code(
            &[
                "--json",
                "--config",
                config_path,
                "session",
                "plan",
                "checkout",
                "--ttl",
                "forever",
            ],
            EXIT_USAGE,
        )
    });

    assert!(
        output.stdout.is_empty(),
        "invalid ttl errors should not write stdout"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!("session_plan_invalid_ttl_json", value);
}

#[test]
fn prints_config_show_text() {
    let output = kply_cmd()
        .args(["config", "show"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("config_show_text", output);
}

#[test]
fn prints_config_show_json() {
    let output = kply_cmd()
        .args(["config", "show", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!("config_show_json", value);
}

#[test]
fn suppresses_config_show_text_when_quiet() {
    kply_cmd()
        .args(["config", "show", "--quiet"])
        .assert()
        .success()
        .stdout("");
}

#[test]
fn rejects_unreadable_config_show_as_config_error() {
    let workspace = temp_workspace();
    let missing_config_path = workspace.path().join("missing").join("kply.yaml");
    let missing_config = missing_config_path
        .to_str()
        .expect("missing config path should be UTF-8");

    let output = assert_kply_exit_code(&["--config", missing_config, "config", "show"], EXIT_USAGE);

    assert!(
        output.stdout.is_empty(),
        "config load errors should not write stdout"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    let stderr = stderr.replace(missing_config, "<config-path>");
    insta::assert_snapshot!("config_show_load_error", normalize_output(&stderr));
}

#[test]
fn prints_config_validate_text() {
    let output = kply_cmd()
        .args(["config", "validate"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("config_validate_text", output);
}

#[test]
fn prints_config_validate_json() {
    let output = kply_cmd()
        .args(["config", "validate", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!("config_validate_json", value);
}

#[test]
fn rejects_unparseable_config_validate_as_json_config_error() {
    let workspace = temp_workspace();
    let config_path = write_temp_file(&workspace, "kply.yaml", "version: [");

    let output = assert_kply_exit_code(
        &[
            "--json",
            "--config",
            config_path.to_str().expect("config path should be UTF-8"),
            "config",
            "validate",
        ],
        EXIT_USAGE,
    );

    assert!(
        output.stdout.is_empty(),
        "config load errors should not write stdout"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    assert_eq!(value["error"]["code"], "config");
    assert_eq!(
        value["error"]["exit_code"],
        serde_json::Value::Number(EXIT_USAGE.into())
    );
    assert!(
        value["error"]["message"]
            .as_str()
            .expect("message should be a string")
            .contains("failed to parse config file"),
        "config load JSON error should describe parse failure"
    );
}

#[test]
fn rejects_invalid_config_validate_text() {
    let workspace = temp_workspace();
    let config_path = write_temp_file(
        &workspace,
        "kply.yaml",
        r#"
version: 1
apps:
  - name: ""
    namespace: shop
    workload: checkout-api
    service: checkout-http
    route_strategy: header
"#,
    );

    let output = assert_kply_exit_code(
        &[
            "--config",
            config_path.to_str().expect("config path should be UTF-8"),
            "config",
            "validate",
        ],
        EXIT_BLOCKING,
    );

    assert!(
        output.stdout.is_empty(),
        "invalid config text output should not write stdout"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    insta::assert_snapshot!("config_validate_invalid_text", stderr);
}

#[test]
fn rejects_invalid_config_validate_json() {
    let workspace = temp_workspace();
    let config_path = write_temp_file(
        &workspace,
        "kply.yaml",
        r#"
version: 1
apps:
  - name: checkout
    namespace: ""
    workload: checkout-api
    service: checkout-http
    route_strategy: header
"#,
    );

    let output = assert_kply_exit_code(
        &[
            "--json",
            "--config",
            config_path.to_str().expect("config path should be UTF-8"),
            "config",
            "validate",
        ],
        EXIT_BLOCKING,
    );

    assert!(
        output.stdout.is_empty(),
        "invalid config JSON output should not write stdout"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!("config_validate_invalid_json", value);
}

#[test]
fn rejects_invalid_config_validate_json_when_quiet() {
    let workspace = temp_workspace();
    let config_path = write_temp_file(
        &workspace,
        "kply.yaml",
        r#"
version: 1
apps:
  - name: checkout
    namespace: ""
    workload: checkout-api
    service: checkout-http
    route_strategy: header
"#,
    );

    let output = assert_kply_exit_code(
        &[
            "--json",
            "--quiet",
            "--config",
            config_path.to_str().expect("config path should be UTF-8"),
            "config",
            "validate",
        ],
        EXIT_BLOCKING,
    );

    assert!(
        output.stdout.is_empty(),
        "invalid quiet JSON output should not write stdout"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!("config_validate_invalid_json_quiet", value);
}

#[test]
fn rejects_invalid_config_validate_quiet() {
    let workspace = temp_workspace();
    let config_path = write_temp_file(
        &workspace,
        "kply.yaml",
        r#"
version: 1
apps:
  - name: checkout
    namespace: shop
    workload: ""
    service: checkout-http
    route_strategy: header
"#,
    );

    let output = assert_kply_exit_code(
        &[
            "--quiet",
            "--config",
            config_path.to_str().expect("config path should be UTF-8"),
            "config",
            "validate",
        ],
        EXIT_BLOCKING,
    );

    assert!(
        output.stdout.is_empty(),
        "quiet output should not write stdout"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    insta::assert_snapshot!("config_validate_invalid_quiet", stderr);
}

#[test]
fn suppresses_config_validate_text_when_quiet() {
    kply_cmd()
        .args(["config", "validate", "--quiet"])
        .assert()
        .success()
        .stdout("");
}

#[test]
fn prints_app_list_empty_text() {
    let output = kply_cmd()
        .args(["app", "list"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("app_list_empty_text", output);
}

#[test]
fn prints_app_list_empty_json() {
    let output = kply_cmd()
        .args(["app", "list", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!("app_list_empty_json", value);
}

#[test]
fn prints_app_list_configured_text() {
    let workspace = temp_workspace();
    let config_path = write_temp_file(
        &workspace,
        "kply.yaml",
        r#"
version: 1
apps:
  - name: checkout
    namespace: shop
    workload: checkout-api
    service: checkout-http
    default_image: ghcr.io/acme/checkout:next
    route_strategy: header
  - name: catalog
    namespace: shop
    workload: catalog-api
    service: catalog-http
    route_strategy: preview
"#,
    );

    let output = kply_cmd()
        .args([
            "--config",
            config_path.to_str().expect("config path should be UTF-8"),
            "app",
            "list",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("app_list_configured_text", output);
}

#[test]
fn prints_app_list_configured_json() {
    let workspace = temp_workspace();
    let config_path = write_temp_file(
        &workspace,
        "kply.yaml",
        r#"
version: 1
apps:
  - name: checkout
    namespace: shop
    workload: checkout-api
    service: checkout-http
    default_image: ghcr.io/acme/checkout:next
    route_strategy: header
"#,
    );

    let output = kply_cmd()
        .args([
            "--config",
            config_path.to_str().expect("config path should be UTF-8"),
            "app",
            "list",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!("app_list_configured_json", value);
}

#[test]
fn suppresses_app_list_text_when_quiet() {
    kply_cmd()
        .args(["app", "list", "--quiet"])
        .assert()
        .success()
        .stdout("");
}

#[test]
fn rejects_invalid_app_list_config() {
    let workspace = temp_workspace();
    let config_path = write_temp_file(
        &workspace,
        "kply.yaml",
        r#"
version: 1
apps:
  - name: checkout
    namespace: ""
    workload: checkout-api
    service: checkout-http
    route_strategy: header
"#,
    );

    let output = assert_kply_exit_code(
        &[
            "--config",
            config_path.to_str().expect("config path should be UTF-8"),
            "app",
            "list",
        ],
        EXIT_BLOCKING,
    );

    assert!(
        output.stdout.is_empty(),
        "invalid app list config should not write stdout"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    insta::assert_snapshot!("app_list_invalid_config", stderr);
}

#[test]
fn rejects_invalid_app_list_config_json() {
    let workspace = temp_workspace();
    let config_path = write_temp_file(
        &workspace,
        "kply.yaml",
        r#"
version: 1
apps:
  - name: checkout
    namespace: ""
    workload: checkout-api
    service: checkout-http
    route_strategy: header
"#,
    );

    let output = assert_kply_exit_code(
        &[
            "--json",
            "--config",
            config_path.to_str().expect("config path should be UTF-8"),
            "app",
            "list",
        ],
        EXIT_BLOCKING,
    );

    assert!(
        output.stdout.is_empty(),
        "invalid app list JSON config should not write stdout"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!("app_list_invalid_config_json", value);
}

#[test]
fn prints_app_inspect_text() {
    let workspace = temp_workspace();
    let config_path = write_temp_file(
        &workspace,
        "kply.yaml",
        r#"
version: 1
apps:
  - name: checkout
    namespace: shop
    workload: checkout-api
    service: checkout-http
    default_image: ghcr.io/acme/checkout:next
    route_strategy: header
"#,
    );

    let output = kply_cmd()
        .args([
            "--config",
            config_path.to_str().expect("config path should be UTF-8"),
            "app",
            "inspect",
            "checkout",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("app_inspect_text", output);
}

#[test]
fn prints_app_inspect_json() {
    let workspace = temp_workspace();
    let config_path = write_temp_file(
        &workspace,
        "kply.yaml",
        r#"
version: 1
apps:
  - name: catalog
    namespace: shop
    workload: catalog-api
    service: catalog-http
    route_strategy: preview
"#,
    );

    let output = kply_cmd()
        .args([
            "--json",
            "--config",
            config_path.to_str().expect("config path should be UTF-8"),
            "app",
            "inspect",
            "catalog",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!("app_inspect_json", value);
}

#[test]
fn prints_app_graph_json() {
    let workspace = temp_workspace();
    let config_path = write_temp_file(
        &workspace,
        "kply.yaml",
        r#"
version: 1
apps:
  - name: checkout
    namespace: shop
    workload: checkout-api
    workload_kind: StatefulSet
    service: checkout-http
    route_strategy: header
"#,
    );

    let output = kply_cmd()
        .args([
            "--json",
            "--config",
            config_path.to_str().expect("config path should be UTF-8"),
            "app",
            "graph",
            "checkout",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!("app_graph_json", value);
}

#[test]
fn prints_app_graph_text() {
    let workspace = temp_workspace();
    let config_path = write_temp_file(
        &workspace,
        "kply.yaml",
        r#"
version: 1
apps:
  - name: checkout
    namespace: shop
    workload: checkout-api
    workload_kind: StatefulSet
    service: checkout-http
    route_strategy: header
"#,
    );

    let output = kply_cmd()
        .args([
            "--config",
            config_path.to_str().expect("config path should be UTF-8"),
            "app",
            "graph",
            "checkout",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("app_graph_text", output);
}

#[test]
fn suppresses_app_graph_text_when_quiet() {
    let workspace = temp_workspace();
    let config_path = write_temp_file(
        &workspace,
        "kply.yaml",
        r#"
version: 1
apps:
  - name: checkout
    namespace: shop
    workload: checkout-api
    service: checkout-http
    route_strategy: header
"#,
    );

    kply_cmd()
        .args([
            "--config",
            config_path.to_str().expect("config path should be UTF-8"),
            "app",
            "graph",
            "checkout",
            "--quiet",
        ])
        .assert()
        .success()
        .stdout("");
}

#[test]
fn suppresses_app_inspect_text_when_quiet() {
    let workspace = temp_workspace();
    let config_path = write_temp_file(
        &workspace,
        "kply.yaml",
        r#"
version: 1
apps:
  - name: checkout
    namespace: shop
    workload: checkout-api
    service: checkout-http
    route_strategy: header
"#,
    );

    kply_cmd()
        .args([
            "--config",
            config_path.to_str().expect("config path should be UTF-8"),
            "app",
            "inspect",
            "checkout",
            "--quiet",
        ])
        .assert()
        .success()
        .stdout("");
}

#[test]
fn rejects_missing_app_inspect_target() {
    let workspace = temp_workspace();
    let config_path = write_temp_file(
        &workspace,
        "kply.yaml",
        r#"
version: 1
apps:
  - name: checkout
    namespace: shop
    workload: checkout-api
    service: checkout-http
    route_strategy: header
"#,
    );

    let output = assert_kply_exit_code(
        &[
            "--config",
            config_path.to_str().expect("config path should be UTF-8"),
            "app",
            "inspect",
            "catalog",
        ],
        EXIT_USAGE,
    );

    assert!(
        output.stdout.is_empty(),
        "missing app inspect target should not write stdout"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    insta::assert_snapshot!("app_inspect_missing_app", stderr);
}

#[test]
fn rejects_missing_app_inspect_target_json() {
    let output = assert_kply_exit_code(&["--json", "app", "inspect", "catalog"], EXIT_USAGE);

    assert!(
        output.stdout.is_empty(),
        "missing app inspect JSON target should not write stdout"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!("app_inspect_missing_app_json", value);
}

#[test]
fn prints_cluster_info_text() {
    let workspace = temp_workspace();
    let kubeconfig_path = write_fake_kubeconfig(&workspace);
    let output = kply_cmd()
        .env("KUBECONFIG", kubeconfig_path)
        .args(["cluster", "info"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("cluster_info_text", output);
}

#[test]
fn prints_cluster_info_json() {
    let workspace = temp_workspace();
    let kubeconfig_path = write_fake_kubeconfig(&workspace);
    let output = kply_cmd()
        .env("KUBECONFIG", kubeconfig_path)
        .args(["cluster", "info", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!("cluster_info_json", value);
}

#[test]
fn suppresses_cluster_info_text_when_quiet() {
    let workspace = temp_workspace();
    let kubeconfig_path = write_fake_kubeconfig(&workspace);

    kply_cmd()
        .env("KUBECONFIG", kubeconfig_path)
        .args(["cluster", "info", "--quiet"])
        .assert()
        .success()
        .stdout("");
}

#[test]
fn rejects_unreadable_cluster_info_kubeconfig() {
    let workspace = temp_workspace();
    let missing_kubeconfig_path = workspace.path().join("missing").join("kubeconfig.yaml");
    let missing_kubeconfig = missing_kubeconfig_path
        .to_str()
        .expect("missing kubeconfig path should be UTF-8");

    let output = kply_cmd()
        .env("KUBECONFIG", missing_kubeconfig)
        .args(["cluster", "info"])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(
        output.stdout.is_empty(),
        "kubeconfig errors should not write stdout"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(
        !stderr.contains(missing_kubeconfig),
        "cluster info errors should not leak the configured kubeconfig path"
    );
    insta::assert_snapshot!("cluster_info_kubeconfig_error", normalize_output(&stderr));
}

#[test]
fn rejects_unreadable_cluster_info_kubeconfig_as_json() {
    let workspace = temp_workspace();
    let missing_kubeconfig_path = workspace.path().join("missing").join("kubeconfig.yaml");
    let missing_kubeconfig = missing_kubeconfig_path
        .to_str()
        .expect("missing kubeconfig path should be UTF-8");

    let output = kply_cmd()
        .env("KUBECONFIG", missing_kubeconfig)
        .args(["cluster", "info", "--json"])
        .assert()
        .code(EXIT_USAGE)
        .get_output()
        .clone();

    assert!(
        output.stdout.is_empty(),
        "kubeconfig JSON errors should not write stdout"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(
        !stderr.contains(missing_kubeconfig),
        "cluster info JSON errors should not leak the configured kubeconfig path"
    );
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!("cluster_info_kubeconfig_json_error", value);
}

#[test]
fn covers_every_top_level_command() {
    let mut command_names = Cli::command()
        .get_subcommands()
        .map(|command| command.get_name().to_owned())
        .collect::<Vec<_>>();
    command_names.sort_unstable();

    assert_eq!(
        command_names,
        [
            "app",
            "check",
            "cluster",
            "completion",
            "config",
            "demo",
            "help",
            "report",
            "route",
            "session"
        ],
        "update CLI command tests when the top-level command surface changes"
    );

    kply_cmd().arg("help").assert().success();

    for command in Command::PLACEHOLDER_GROUPS {
        kply_cmd().arg(command.name()).assert().success();
    }
}

#[test]
fn covers_every_check_command() {
    let mut command_names = Cli::command()
        .find_subcommand("check")
        .expect("check command")
        .get_subcommands()
        .map(|command| command.get_name().to_owned())
        .collect::<Vec<_>>();
    command_names.sort_unstable();

    assert_eq!(
        command_names,
        ["run"],
        "update check command tests when the check command surface changes"
    );

    kply_cmd()
        .args([
            "check",
            CheckCommand::Run {
                session: String::new(),
                namespace: Some(String::new()),
            }
            .name(),
            "checkout-plan",
            "--namespace",
            "shop",
        ])
        .assert()
        .code(EXIT_USAGE);
}

#[test]
fn covers_every_route_command() {
    let mut command_names = Cli::command()
        .find_subcommand("route")
        .expect("route command")
        .get_subcommands()
        .map(|command| command.get_name().to_owned())
        .collect::<Vec<_>>();
    command_names.sort_unstable();

    assert_eq!(
        command_names,
        ["apply", "cleanup", "plan"],
        "update route command tests when the route command surface changes"
    );

    kply_cmd()
        .args([
            "route",
            RouteCommand::Apply {
                session: String::new(),
                namespace: None,
                confirm_route_mutation: true,
            }
            .name(),
            "checkout-plan",
            "--confirm-route-mutation",
        ])
        .assert()
        .success();
    kply_cmd()
        .args([
            "route",
            RouteCommand::Cleanup {
                session: String::new(),
                namespace: None,
            }
            .name(),
            "checkout-plan",
        ])
        .assert()
        .success();
    kply_cmd()
        .args([
            "route",
            RouteCommand::Plan {
                session: String::new(),
                namespace: None,
            }
            .name(),
            "checkout-plan",
        ])
        .assert()
        .success();
}

#[test]
fn covers_every_report_command() {
    let mut command_names = Cli::command()
        .find_subcommand("report")
        .expect("report command")
        .get_subcommands()
        .map(|command| command.get_name().to_owned())
        .collect::<Vec<_>>();
    command_names.sort_unstable();

    assert_eq!(
        command_names,
        ["export", "show"],
        "update report command tests when the report command surface changes"
    );

    kply_cmd()
        .args([
            "report",
            ReportCommand::Show {
                session: String::new(),
                namespace: None,
            }
            .name(),
            "checkout-plan",
            "--namespace",
            "shop",
        ])
        .assert()
        .code(EXIT_USAGE);

    kply_cmd()
        .args([
            "report",
            ReportCommand::Export {
                session: String::new(),
                namespace: None,
                format: ReportExportFormat::Json,
            }
            .name(),
            "checkout-plan",
            "--namespace",
            "shop",
            "--format",
            "json",
        ])
        .assert()
        .code(EXIT_USAGE);

    kply_cmd()
        .args([
            "report",
            ReportCommand::Export {
                session: String::new(),
                namespace: None,
                format: ReportExportFormat::Markdown,
            }
            .name(),
            "checkout-plan",
            "--namespace",
            "shop",
            "--format",
            "markdown",
        ])
        .assert()
        .code(EXIT_USAGE);
}

#[test]
fn covers_every_config_command() {
    let mut command_names = Cli::command()
        .find_subcommand("config")
        .expect("config command")
        .get_subcommands()
        .map(|command| command.get_name().to_owned())
        .collect::<Vec<_>>();
    command_names.sort_unstable();

    assert_eq!(
        command_names,
        ["show", "validate"],
        "update config command tests when the config command surface changes"
    );

    kply_cmd()
        .args(["config", ConfigCommand::Show.name()])
        .assert()
        .success();
    kply_cmd()
        .args(["config", ConfigCommand::Validate.name()])
        .assert()
        .success();
}

#[test]
fn covers_every_session_command() {
    let mut command_names = Cli::command()
        .find_subcommand("session")
        .expect("session command")
        .get_subcommands()
        .map(|command| command.get_name().to_owned())
        .collect::<Vec<_>>();
    command_names.sort_unstable();

    assert_eq!(
        command_names,
        ["cleanup", "create", "list", "manifests", "plan", "status"],
        "update session command tests when the session command surface changes"
    );

    kply_cmd()
        .args([
            "session",
            SessionCommand::Cleanup {
                session: String::new(),
                apply: false,
                dry_run: false,
                namespace: None,
            }
            .name(),
            "checkout-plan",
        ])
        .assert()
        .success();

    with_session_plan_config(|config_path| {
        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                SessionCommand::Plan {
                    app: String::new(),
                    image: None,
                    namespace: None,
                    time_to_live: None,
                    route_strategy: None,
                }
                .name(),
                "checkout",
            ])
            .assert()
            .success();
        kply_cmd()
            .args([
                "--config",
                config_path,
                "session",
                SessionCommand::Manifests {
                    app: String::new(),
                    yaml: false,
                    image: None,
                    namespace: None,
                    time_to_live: None,
                    route_strategy: None,
                }
                .name(),
                "checkout",
            ])
            .assert()
            .success();
    });
}

#[test]
fn covers_every_app_command() {
    let mut command_names = Cli::command()
        .find_subcommand("app")
        .expect("app command")
        .get_subcommands()
        .map(|command| command.get_name().to_owned())
        .collect::<Vec<_>>();
    command_names.sort_unstable();

    assert_eq!(
        command_names,
        ["graph", "inspect", "list"],
        "update app command tests when the app command surface changes"
    );

    kply_cmd()
        .args(["app", AppCommand::List.name()])
        .assert()
        .success();
    kply_cmd()
        .args([
            "app",
            AppCommand::Inspect { app: String::new() }.name(),
            "missing",
        ])
        .assert()
        .code(EXIT_USAGE);
    kply_cmd()
        .args([
            "app",
            AppCommand::Graph { app: String::new() }.name(),
            "missing",
        ])
        .assert()
        .code(EXIT_USAGE);
}

#[test]
fn covers_every_cluster_command() {
    let mut command_names = Cli::command()
        .find_subcommand("cluster")
        .expect("cluster command")
        .get_subcommands()
        .map(|command| command.get_name().to_owned())
        .collect::<Vec<_>>();
    command_names.sort_unstable();

    assert_eq!(
        command_names,
        ["info"],
        "update cluster command tests when the cluster command surface changes"
    );

    let workspace = temp_workspace();
    let kubeconfig_path = write_fake_kubeconfig(&workspace);
    kply_cmd()
        .env("KUBECONFIG", kubeconfig_path)
        .args(["cluster", ClusterCommand::Info.name()])
        .assert()
        .success();
}

#[test]
fn covers_every_demo_command() {
    let mut command_names = Cli::command()
        .find_subcommand("demo")
        .expect("demo command")
        .get_subcommands()
        .map(|command| command.get_name().to_owned())
        .collect::<Vec<_>>();
    command_names.sort_unstable();

    assert_eq!(
        command_names,
        ["doctor", "install", "reset", "teardown"],
        "update demo command tests when the demo command surface changes"
    );

    let workspace = temp_workspace();
    kply_cmd()
        .env("PATH", fake_demo_path(workspace.path()))
        .args(["demo", DemoCommand::Doctor.name()])
        .assert()
        .success();

    let workspace = temp_workspace();
    let (path, log_path) = fake_kubectl_path(workspace.path(), 0);
    kply_cmd()
        .env("PATH", path)
        .env("KPLY_FAKE_KUBECTL_LOG", log_path)
        .args(["demo", DemoCommand::Install.name()])
        .assert()
        .success();

    let workspace = temp_workspace();
    let (path, log_path) = fake_kubectl_path(workspace.path(), 0);
    kply_cmd()
        .env("PATH", path)
        .env("KPLY_FAKE_KUBECTL_LOG", log_path)
        .args(["demo", DemoCommand::Reset.name()])
        .assert()
        .success();

    let workspace = temp_workspace();
    let (path, log_path) = fake_kubectl_path(workspace.path(), 0);
    kply_cmd()
        .env("PATH", path)
        .env("KPLY_FAKE_KUBECTL_LOG", log_path)
        .args(["demo", DemoCommand::Teardown.name()])
        .assert()
        .success();
}

#[test]
fn covers_every_top_level_flag() {
    let mut flag_names = Cli::command()
        .get_arguments()
        .filter_map(|argument| argument.get_long())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    flag_names.push("help".to_owned());
    flag_names.sort_unstable();

    assert_eq!(
        flag_names,
        [
            "config",
            "help",
            "json",
            "no-color",
            "no-config",
            "quiet",
            "verbose",
            "version"
        ],
        "update CLI flag tests when the top-level flag surface changes"
    );

    for flag_name in &flag_names {
        let mut command = kply_cmd();
        command.arg(format!("--{flag_name}"));
        if flag_name == "config" {
            command.arg("kply.yaml");
        }
        command.assert().success();
    }
}

#[test]
fn rejects_hidden_aliases_until_command_surface_is_stable() {
    let cli_command = Cli::command();

    for command in cli_command.get_subcommands() {
        let aliases = command.get_all_aliases().collect::<Vec<_>>();

        assert!(
            aliases.is_empty(),
            "top-level command `{}` should not define aliases yet",
            command.get_name()
        );
    }

    for argument in cli_command.get_arguments() {
        let long_aliases = argument.get_all_aliases().unwrap_or_default();
        let short_aliases = argument.get_all_short_aliases().unwrap_or_default();

        assert!(
            long_aliases.is_empty() && short_aliases.is_empty(),
            "top-level flag `--{}` should not define aliases yet",
            argument.get_long().unwrap_or(argument.get_id().as_str())
        );
    }
}

#[test]
fn suppresses_placeholder_text_when_quiet() {
    kply_cmd().arg("--quiet").assert().success().stdout("");
}

#[test]
fn suppresses_command_group_text_when_quiet() {
    for command in Command::PLACEHOLDER_GROUPS {
        kply_cmd()
            .args([command.name(), "--quiet"])
            .assert()
            .success()
            .stdout("");
    }
}

#[test]
fn keeps_requested_outputs_when_quiet() {
    kply_cmd()
        .args(["--version", "--quiet"])
        .assert()
        .success()
        .stdout(format!("kply {}\n", env!("CARGO_PKG_VERSION")));

    let output = kply_cmd()
        .args(["--json", "--quiet"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&output).expect("stdout should be JSON");
    insta::assert_json_snapshot!("quiet_json", value);
}

#[test]
fn prints_verbose_trace_to_stderr() {
    let output = kply_cmd()
        .arg("--verbose")
        .assert()
        .success()
        .get_output()
        .stderr
        .clone();

    let output = String::from_utf8(output).expect("stderr should be UTF-8");
    insta::assert_snapshot!("verbose_stderr", output);
}

#[test]
fn keeps_json_stdout_clean_when_verbose() {
    let assert = kply_cmd().args(["--json", "--verbose"]).assert().success();
    let output = assert.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).expect("stdout should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout should be JSON");
    insta::assert_json_snapshot!("verbose_json_stdout", value);

    let stderr = String::from_utf8(output.stderr.clone()).expect("stderr should be UTF-8");
    insta::assert_snapshot!("verbose_json_stderr", stderr);
}

#[test]
fn accepts_no_color_for_deterministic_output() {
    let output = kply_cmd()
        .arg("--no-color")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("no_color_placeholder_text", output);
}

#[test]
fn includes_no_color_in_verbose_trace() {
    let output = kply_cmd()
        .args(["--verbose", "--no-color"])
        .assert()
        .success()
        .get_output()
        .stderr
        .clone();

    let output = String::from_utf8(output).expect("stderr should be UTF-8");
    insta::assert_snapshot!("verbose_no_color_stderr", output);
}

#[test]
fn accepts_explicit_config_path() {
    kply_cmd()
        .args(["--config", "kply.yaml"])
        .assert()
        .success();
}

#[test]
fn includes_explicit_config_path_in_verbose_trace() {
    let output = kply_cmd()
        .args(["--verbose", "--config", "kply.yaml"])
        .assert()
        .success()
        .get_output()
        .stderr
        .clone();

    let output = String::from_utf8(output).expect("stderr should be UTF-8");
    insta::assert_snapshot!("verbose_config_stderr", output);
}

#[test]
fn accepts_no_config_flag() {
    kply_cmd().arg("--no-config").assert().success();
}

#[test]
fn includes_no_config_in_verbose_trace() {
    let output = kply_cmd()
        .args(["--verbose", "--no-config"])
        .assert()
        .success()
        .get_output()
        .stderr
        .clone();

    let output = String::from_utf8(output).expect("stderr should be UTF-8");
    insta::assert_snapshot!("verbose_no_config_stderr", output);
}

#[test]
fn rejects_config_path_with_no_config_as_usage_error() {
    let output = assert_kply_exit_code(&["--config", "kply.yaml", "--no-config"], EXIT_USAGE);

    assert!(
        output.stdout.is_empty(),
        "usage errors should not write stdout"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    insta::assert_snapshot!("config_path_with_no_config_usage_error", stderr);
}

#[test]
fn rejects_config_without_path_as_usage_error() {
    let output = assert_kply_exit_code(&["--config"], EXIT_USAGE);

    assert!(
        output.stdout.is_empty(),
        "usage errors should not write stdout"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    insta::assert_snapshot!("missing_config_path_usage_error", stderr);
}

#[test]
fn renders_unknown_flag_as_usage_error() {
    let output = assert_kply_exit_code(&["--bad-flag"], EXIT_USAGE);

    assert!(
        output.stdout.is_empty(),
        "usage errors should not write stdout"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    insta::assert_snapshot!("unknown_flag_usage_error", stderr);
}

#[test]
fn renders_unknown_command_as_usage_error() {
    let output = assert_kply_exit_code(&["unknown"], EXIT_USAGE);

    assert!(
        output.stdout.is_empty(),
        "usage errors should not write stdout"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    insta::assert_snapshot!("unknown_command_usage_error", stderr);
}

#[test]
fn renders_unknown_flag_as_json_usage_error() {
    let output = assert_kply_exit_code(&["--json", "--bad-flag"], EXIT_USAGE);

    assert!(
        output.stdout.is_empty(),
        "JSON usage errors should not write stdout"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!("unknown_flag_json_usage_error", value);
}

#[test]
fn renders_unknown_command_as_json_usage_error() {
    let output = assert_kply_exit_code(&["unknown", "--json"], EXIT_USAGE);

    assert!(
        output.stdout.is_empty(),
        "JSON usage errors should not write stdout"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr should be JSON");
    insta::assert_json_snapshot!("unknown_command_json_usage_error", value);
}
