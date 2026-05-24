//! CLI placeholder behavior tests for Kply.

use kply_cli::cli::Command;
use kply_test::kply_cmd;

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
