use indicatif::{ProgressBar, ProgressStyle};
use std::fs::File;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
use thiserror::Error;
use ureq::get;

#[cfg(target_os = "linux")]
use xz2::stream::Error as XzError;

#[cfg(target_os = "windows")]
use zip::result::ZipError;

use crate::helpers::get_current_dir;
use crate::helpers::is_in_path;
#[cfg(target_os = "linux")]
use crate::helpers::set_executable_permissions;

#[derive(Error, Debug)]
enum DownloadError {
    #[error("HTTP request failed: {0}")]
    RequestError(#[from] ureq::Error),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Header parsing error")]
    HeaderParseError,
}

#[cfg(target_os = "linux")]
#[derive(Error, Debug)]
pub enum ExtractError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("XZ decompression error: {0}")]
    Xz(#[from] XzError),

    #[error("Unsupported archive format")]
    UnsupportedFormat,
}

#[cfg(target_os = "windows")]
#[derive(ThisError, Debug)]
pub enum ExtractError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Zip error: {0}")]
    Zip(#[from] ZipError),

    #[error("Unsupported archive format")]
    UnsupportedFormat,
}

#[derive(Debug, Clone)]
pub struct DependencyPaths {
    pub ffmpeg: String,
    pub ffprobe: String,
    pub covit: String,
}

pub fn download_and_extract_deps() -> Result<DependencyPaths, Box<dyn std::error::Error>> {
    // Tool name constants
    const FFMPEG: &str = "ffmpeg";
    const FFPROBE: &str = "ffprobe";
    const COVIT: &str = "covit";

    #[cfg(target_os = "linux")]
    const FFMPEG_ARCHIVE: &str = "ffmpeg.tar.xz";
    #[cfg(target_os = "linux")]
    const FFMPEG_URL: &str =
        "https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz";
    #[cfg(target_os = "linux")]
    const FFMPEG_FILES: [&str; 2] = [FFMPEG, FFPROBE];
    #[cfg(target_os = "linux")]
    const COVIT_URL: &str = "https://covers.musichoarders.xyz/share/covit-linux-amd64";
    #[cfg(target_os = "linux")]
    const COVIT_BIN: &str = COVIT;

    #[cfg(target_os = "windows")]
    const FFMPEG_ARCHIVE: &str = "ffmpeg.zip";
    #[cfg(target_os = "windows")]
    const FFMPEG_URL: &str = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";
    #[cfg(target_os = "windows")]
    const FFMPEG_FILES: [&str; 2] = ["ffmpeg.exe", "ffprobe.exe"];
    #[cfg(target_os = "windows")]
    const COVIT_URL: &str = "https://covers.musichoarders.xyz/share/covit-windows-amd64.exe";
    #[cfg(target_os = "windows")]
    const COVIT_BIN: &str = "covit.exe";

    let exe_dir = get_current_dir();
    let bin_dir = exe_dir.join("deps_bin");
    std::fs::create_dir_all(&bin_dir)?;

    // Helper closure for PATH check or download
    let resolve_dep = |name: &str,
                       bin_name: &str,
                       archive_path: &std::path::Path,
                       url: &str,
                       files: &[&str]|
     -> Result<String, Box<dyn std::error::Error>> {
        if is_in_path(name) {
            Ok(name.to_string())
        } else {
            let out_path = bin_dir.join(bin_name);
            if !out_path.exists() {
                download_with_progress(url, archive_path.to_str().unwrap())?;
                extract_selected_files(archive_path, files, &bin_dir)?;
            }
            #[cfg(target_os = "linux")]
            set_executable_permissions(&out_path)?;
            Ok(out_path.to_string_lossy().to_string())
        }
    };

    // ffmpeg and ffprobe (from same archive)
    let archive_path = bin_dir.join(FFMPEG_ARCHIVE);
    let ffmpeg = resolve_dep(
        FFMPEG,
        FFMPEG_FILES[0],
        &archive_path,
        FFMPEG_URL,
        &[FFMPEG_FILES[0]],
    )?;
    let ffprobe = resolve_dep(
        FFPROBE,
        FFMPEG_FILES[1],
        &archive_path,
        FFMPEG_URL,
        &[FFMPEG_FILES[1]],
    )?;

    // covit (standalone)
    let covit = if is_in_path(COVIT) {
        COVIT.to_string()
    } else {
        let covit_path = bin_dir.join(COVIT_BIN);
        if !covit_path.exists() {
            download_with_progress(COVIT_URL, covit_path.to_str().unwrap())?;
            #[cfg(target_os = "linux")]
            set_executable_permissions(&covit_path)?;
        }
        covit_path.to_string_lossy().to_string()
    };

    Ok(DependencyPaths {
        ffmpeg,
        ffprobe,
        covit,
    })
}

#[cfg(target_os = "linux")]
pub fn extract_selected_files(
    archive_path: &std::path::Path,
    files_to_extract: &[&str],
    output_dir: &std::path::Path,
) -> Result<(), ExtractError> {
    use std::fs::{self, File};

    let file = File::open(archive_path)?;
    let decompressor = xz2::read::XzDecoder::new(file);
    let mut archive = tar::Archive::new(decompressor);

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        if let Some(name) = path.file_name().and_then(|n| n.to_str())
            && files_to_extract.contains(&name)
        {
            use crate::helpers::set_executable_permissions;

            let out_path = output_dir.join(name);
            let mut out_file = File::create(&out_path)?;
            std::io::copy(&mut entry, &mut out_file)?;

            // Set executable permissions
            set_executable_permissions(&out_path)?;
        }
    }

    // Remove the uncompressed tar file
    if let Err(err) = fs::remove_file(archive_path) {
        eprintln!("Error deleting file: {err}");
    } else {
        println!("File cleanup successful!");
    }
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn extract_selected_files(
    archive_path: &std::path::Path,
    files_to_extract: &[&str],
    output_dir: &std::path::Path,
) -> Result<(), ExtractError> {
    use std::fs::File;
    use zip::ZipArchive;

    let file = File::open(archive_path)?;
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name();

        if files_to_extract.iter().any(|wanted| name.ends_with(wanted)) {
            let out_path = output_dir.join(
                std::path::Path::new(name)
                    .file_name()
                    .unwrap_or_else(|| std::ffi::OsStr::new(name)),
            );
            let mut out_file = File::create(&out_path)?;
            std::io::copy(&mut entry, &mut out_file)?;
        }
    }

    // Remove the ZIP file
    if let Err(err) = fs::remove_file(archive_path) {
        eprintln!("Error deleting file: {}", err);
    }
    Ok(())
}

/// Downloads a file from a URL with a progress bar and saves it to a local path.
fn download_with_progress(linux_url: &str, tar_xz_path: &str) -> Result<(), DownloadError> {
    let (headers, body) = get(linux_url).call()?.into_parts();

    let total_size = headers
        .headers
        .get("Content-Length")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .ok_or(DownloadError::HeaderParseError)?;

    let mut file = File::create(tar_xz_path)?;
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
                file.write_all(&buffer[..n])?;
                downloaded += n as u64;
                pb.set_position(downloaded);
            }
            Err(e) => return Err(e.into()),
        }
    }

    pb.finish();
    Ok(())
}
