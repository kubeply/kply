//! Local machine utility checks for CLI commands.

use std::path::{Path, PathBuf};

/// Find the first executable command candidate available on `PATH`.
pub(crate) fn find_command_in_path(command: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).find_map(|directory| {
        command_path_candidates(&directory, command)
            .into_iter()
            .find(|candidate| is_executable_file(candidate))
    })
}

fn is_executable_file(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        std::fs::metadata(path)
            .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
    }

    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

fn command_path_candidates(directory: &Path, command: &str) -> Vec<PathBuf> {
    let mut candidates = vec![directory.join(command)];
    let executable_suffix = std::env::consts::EXE_SUFFIX;
    if !executable_suffix.is_empty() && !command.ends_with(executable_suffix) {
        candidates.push(directory.join(format!("{command}{executable_suffix}")));
    }
    candidates
}
