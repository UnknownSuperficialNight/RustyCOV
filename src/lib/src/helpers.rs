use std::env;
use std::io::Read;
use std::path::PathBuf;

use indicatif::{ProgressBar, ProgressStyle};
use ureq::get;

use crate::deps_download::DownloadError;

/// Checks if a command is in the user's PATH (environmental variable).
///
/// This function checks the provided `cmd` against all directories specified in the `PATH`
/// environment variable. It also handles the special case on Windows where `.exe` might be
/// appended to the command name.
///
/// # Arguments
///
/// * `cmd` - The command to check for in the PATH.
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
///
/// This function retrieves the absolute path of the current executable using `env::current_exe()`
/// and then extracts its parent directory.
///
/// # Returns
///
/// The `PathBuf` representing the parent directory of the current executable, or an error if it
/// cannot be determined.
pub fn get_current_dir() -> PathBuf {
    env::current_exe()
        .expect("Failed to get current executable path")
        .parent()
        .expect("Failed to get parent directory")
        .to_path_buf()
}

/// Sets the file permissions to executable (755).
///
/// This function sets the specified `path`'s permissions to 755, making it executable.
///
/// # Arguments
///
/// * `path` - The file path for which to set executable permissions.
#[cfg(unix)]
pub fn set_executable_permissions(path: &std::path::Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms)
}

/// Downloads an image from the given URL and returns the image bytes.
///
/// This function sends an HTTP GET request to the specified `image_url` using `ureq::get`
/// and reads the response body into a vector of bytes. If the HTTP request fails, or if the
/// response status is not 200, or if reading the response body fails, an error is returned.
///
/// # Arguments
///
/// * `image_url` - The URL of the image to download.
pub fn download_image(image_url: &str) -> Result<Vec<u8>, DownloadError> {
    let (headers, body) = get(image_url).call()?.into_parts();

    let total_size = headers
        .headers
        .get("Content-Length")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .ok_or(DownloadError::HeaderParse)?;

    let mut image_data = Vec::with_capacity(total_size as usize);
    let mut reader = body.into_reader();
    let mut buffer = [0; 8192];
    let mut downloaded = 0u64;

    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {binary_bytes_per_sec} {bar:40} {binary_bytes} / {binary_total_bytes}")
            .expect("Failed to create ProgressStyle object")
            .progress_chars("#-"),
    );

    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                image_data.extend_from_slice(&buffer[..n]);
                downloaded += n as u64;
                pb.set_position(downloaded);
            }
            Err(e) => return Err(e.into()),
        }
    }

    pb.finish();

    Ok(image_data)
}
