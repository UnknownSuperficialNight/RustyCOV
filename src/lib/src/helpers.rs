use std::env;
use std::fs::File;
use std::io::{Read, Write};
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

#[derive(Debug)]
pub enum DownloadTarget<'a> {
    File(&'a str),
    Memory,
}

/// Downloads a file from the specified URL and saves it to the given output path or returns the
/// bytes.
///
/// This function sends an HTTP GET request to the specified `url` using `ureq::get`
/// and reads the response body into a vector of bytes or writes it to a file.
/// If the HTTP request fails, or if the response status is not 200, or if reading the response body
/// fails, an error is returned.
///
/// # Arguments
///
/// * `url` - URL of the file to download.
/// * `target` - The target where the downloaded file will be saved. If `Memory`, returns the bytes.
pub fn download_with_progress(
    url: &str,
    target: DownloadTarget,
) -> Result<Option<Vec<u8>>, DownloadError> {
    let (headers, body) = get(url).call()?.into_parts();

    let total_size = headers
        .headers
        .get("Content-Length")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok());

    let mut reader = body.into_reader();
    let mut buffer = [0; 8192];

    // Prepare output sink and buffer if needed
    let mut file: Option<File> = None;
    let mut memory_buffer: Option<Vec<u8>> = None;

    match target {
        DownloadTarget::File(path) => {
            file = Some(File::create(path)?);
        }
        DownloadTarget::Memory => {
            memory_buffer = Some(match total_size {
                Some(size) => Vec::with_capacity(size as usize),
                None => Vec::new(),
            });
        }
    }

    // Setup progress bar
    let pb = if let Some(total) = total_size {
        let pb = ProgressBar::new(total);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {binary_bytes_per_sec} {bar:40} {binary_bytes} / {binary_total_bytes}")
                .expect("Failed to create ProgressStyle object")
                .progress_chars("#-"),
        );
        pb
    } else {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {spinner} Received {binary_bytes}")
                .expect("Failed to create ProgressStyle object")
                .progress_chars("#-"),
        );
        pb
    };

    // Read loop
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                if let Some(f) = file.as_mut() {
                    f.write_all(&buffer[..n])?;
                } else if let Some(mem) = memory_buffer.as_mut() {
                    mem.extend_from_slice(&buffer[..n]);
                }
                pb.inc(n as u64);
            }
            Err(e) => return Err(e.into()),
        }
    }

    pb.finish();

    match memory_buffer {
        Some(bytes) => Ok(Some(bytes)),
        None => Ok(None),
    }
}

/// Extracts the first contiguous digit substring from `s`
/// Returns Some((number_value, digit_length)) or None if no digits found.
pub fn extract_first_number(s: &str) -> Option<(usize, usize)> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            let digit_str = &s[start..i];
            if let Ok(num) = digit_str.parse::<usize>() {
                return Some((num, digit_str.len()));
            } else {
                return None;
            }
        } else {
            i += 1;
        }
    }
    None
}
