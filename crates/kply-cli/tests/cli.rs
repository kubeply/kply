//! CLI placeholder behavior tests for Kply.

use clap::CommandFactory;
use kply_cli::cli::Cli;
use kply_cli::cli::Command;
use kply_cli::cli::ConfigCommand;
use kply_test::{EXIT_USAGE, assert_kply_exit_code, kply_cmd};

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
        ["show"],
        "update config command tests when the config command surface changes"
    );

    kply_cmd()
        .args(["config", ConfigCommand::Show.name()])
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
