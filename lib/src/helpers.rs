use std::env;
use std::path::PathBuf;

/// Checks if a command is in the user's PATH.
/// On Windows, checks both with and without `.exe` extension.
pub fn is_in_path(cmd: &str) -> bool {
    let paths = match env::var_os("PATH") {
        Some(paths) => env::split_paths(&paths).collect::<Vec<_>>(),
        None => return false,
    };

    #[cfg(windows)]
    let candidates = if cmd.to_lowercase().ends_with(".exe") {
        vec![cmd.to_string()]
    } else {
        vec![cmd.to_string(), format!("{cmd}.exe")]
    };

    #[cfg(not(windows))]
    let candidates = vec![cmd.to_string()];

    for dir in paths {
        for candidate in &candidates {
            let full_path = dir.join(candidate);
            if full_path.exists() && full_path.is_file() {
                return true;
            }
        }
    }
    false
}

/// Returns the directory containing the current executable.
pub fn get_current_dir() -> PathBuf {
    env::current_exe()
        .expect("Failed to get current executable path")
        .parent()
        .expect("Failed to get parent directory")
        .to_path_buf()
}

#[cfg(target_os = "linux")]
pub fn set_executable_permissions(path: &std::path::Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms)
}
