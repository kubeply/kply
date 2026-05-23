//! Test helper placeholders for future Kply integration tests.

use assert_cmd::Command;

/// Return a command handle for the `kply` binary in integration tests.
pub fn kply_cmd() -> Command {
    Command::cargo_bin("kply").expect("kply binary should be built for tests")
}
