//! CLI placeholder behavior tests for Kply.

use clap::CommandFactory;
use kply_cli::cli::Cli;
use kply_cli::cli::Command;
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
fn covers_every_top_level_command() {
    let mut command_names = Cli::command()
        .get_subcommands()
        .map(|command| command.get_name().to_owned())
        .collect::<Vec<_>>();
    command_names.sort_unstable();

    assert_eq!(
        command_names,
        ["app", "cluster", "config", "help", "report", "session"],
        "update CLI command tests when the top-level command surface changes"
    );

    kply_cmd().arg("help").assert().success();

    for command in Command::PLACEHOLDER_GROUPS {
        kply_cmd().arg(command.name()).assert().success();
    }
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
        ["help", "json", "no-color", "quiet", "verbose", "version"],
        "update CLI flag tests when the top-level flag surface changes"
    );

    for args in [
        &["--help"][..],
        &["--json"],
        &["--no-color"],
        &["--quiet"],
        &["--verbose"],
        &["--version"],
    ] {
        kply_cmd().args(args).assert().success();
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
