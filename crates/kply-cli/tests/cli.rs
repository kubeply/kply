//! CLI placeholder behavior tests for Kply.

use clap::CommandFactory;
use kply_cli::cli::AppCommand;
use kply_cli::cli::Cli;
use kply_cli::cli::ClusterCommand;
use kply_cli::cli::Command;
use kply_cli::cli::ConfigCommand;
use kply_test::{
    EXIT_BLOCKING, EXIT_USAGE, assert_kply_exit_code, kply_cmd, normalize_output, temp_workspace,
    write_fake_kubeconfig, write_temp_file,
};

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
fn rejects_app_graph_without_json() {
    let output = assert_kply_exit_code(&["app", "graph", "checkout"], EXIT_USAGE);

    assert!(
        output.stdout.is_empty(),
        "app graph usage errors should not write stdout"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    insta::assert_snapshot!("app_graph_requires_json", stderr);
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
    let stderr = stderr.replace(missing_kubeconfig, "<kubeconfig-path>");
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
    let stderr = stderr.replace(missing_kubeconfig, "<kubeconfig-path>");
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
            "cluster",
            "completion",
            "config",
            "help",
            "report",
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
            "--json",
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
