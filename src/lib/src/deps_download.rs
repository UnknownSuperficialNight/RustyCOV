use std::fs::File;
#[cfg(all(unix, feature = "depend-on-ffmpeg"))]
use std::io;
use std::io::{Read, Write};
#[cfg(all(unix, feature = "depend-on-ffmpeg"))]
use std::path::Path;

use indicatif::{ProgressBar, ProgressStyle};
use thiserror::Error;
use ureq::get;
#[cfg(all(unix, feature = "depend-on-ffmpeg"))]
use xz2::stream::Error as XzError;
#[cfg(all(windows, feature = "depend-on-ffmpeg"))]
use zip::result::ZipError;

#[cfg(unix)]
use crate::helpers::set_executable_permissions;
use crate::helpers::{get_current_dir, is_in_path};

#[derive(Debug, Clone)]
pub struct DependencyPaths {
    #[cfg(feature = "depend-on-ffmpeg")]
    pub ffmpeg: String,
    #[cfg(feature = "depend-on-ffmpeg")]
    pub ffprobe: String,
    pub covit: String,
}

impl DependencyPaths {
    #[cfg(feature = "depend-on-ffmpeg")]
    pub fn ffmpeg(&self) -> &str {
        &self.ffmpeg
    }
    #[cfg(feature = "depend-on-ffmpeg")]
    pub fn ffprobe(&self) -> &str {
        &self.ffprobe
    }
    pub fn covit(&self) -> &str {
        &self.covit
    }
}

#[derive(Error, Debug)]
enum DownloadError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] ureq::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Header parsing error")]
    HeaderParse,
}

#[cfg(all(unix, feature = "depend-on-ffmpeg"))]
#[derive(Error, Debug)]
pub enum ExtractError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("XZ decompression error: {0}")]
    Xz(#[from] XzError),
}

#[cfg(all(windows, feature = "depend-on-ffmpeg"))]
#[derive(Error, Debug)]
pub enum ExtractError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Zip error: {0}")]
    Zip(#[from] ZipError),
}

pub fn download_and_extract_deps() -> Result<DependencyPaths, Box<dyn std::error::Error>> {
    let exe_dir = get_current_dir();
    let bin_dir = exe_dir.join("deps_bin");
    std::fs::create_dir_all(&bin_dir)?;

    // --- Platform/feature-specific constants ---
    #[cfg(all(unix, feature = "depend-on-ffmpeg"))]
    const FFMPEG_ARCHIVE: &str = "ffmpeg.tar.xz";
    #[cfg(all(unix, feature = "depend-on-ffmpeg"))]
    const FFMPEG_URL: &str = "https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz";
    #[cfg(all(unix, feature = "depend-on-ffmpeg"))]
    const FFMPEG_FILES: [&str; 2] = ["ffmpeg", "ffprobe"];

    #[cfg(all(windows, feature = "depend-on-ffmpeg"))]
    const FFMPEG_ARCHIVE: &str = "ffmpeg.zip";
    #[cfg(all(windows, feature = "depend-on-ffmpeg"))]
    const FFMPEG_URL: &str = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";
    #[cfg(all(windows, feature = "depend-on-ffmpeg"))]
    const FFMPEG_FILES: [&str; 2] = ["ffmpeg.exe", "ffprobe.exe"];

    #[cfg(unix)]
    const COVIT_URL: &str = "https://covers.musichoarders.xyz/share/covit-linux-amd64";
    #[cfg(unix)]
    const COVIT_BIN: &str = "covit";

    #[cfg(windows)]
    const COVIT_URL: &str = "https://covers.musichoarders.xyz/share/covit-windows-amd64.exe";
    #[cfg(windows)]
    const COVIT_BIN: &str = "covit.exe";

    // --- Download and extract ffmpeg/ffprobe if needed ---
    #[cfg(feature = "depend-on-ffmpeg")]
    let (ffmpeg_path, ffprobe_path) = {
        let archive_path = bin_dir.join(FFMPEG_ARCHIVE);
        let mut extracted = [None, None];

        // Only download if neither binary is present
        let mut need_download = false;
        for (i, bin) in FFMPEG_FILES.iter().enumerate() {
            let out_path = bin_dir.join(bin);
            if !out_path.exists() && !is_in_path(bin) {
                need_download = true;
            } else {
                extracted[i] = Some(out_path.to_string_lossy().to_string());
            }
        }

        if need_download {
            println!("Downloading ffmpeg archive...");
            download_with_progress(FFMPEG_URL, archive_path.to_str().unwrap())?;

            println!("Extracting ffmpeg/ffprobe...");
            extract_selected_files(&archive_path, &FFMPEG_FILES, &bin_dir)?;

            #[cfg(unix)]
            for bin in &FFMPEG_FILES {
                let out_path = bin_dir.join(bin);
                set_executable_permissions(&out_path)?;
            }
        }

        // After extraction, fill in paths
        for (i, bin) in FFMPEG_FILES.iter().enumerate() {
            let out_path = bin_dir.join(bin);
            if !out_path.exists() && !is_in_path(bin) {
                return Err(format!("Failed to extract or find {}", bin).into());
            }
            extracted[i] = Some(out_path.to_string_lossy().to_string());
        }

        (extracted[0].clone().unwrap(), extracted[1].clone().unwrap())
    };

    // --- Always download covit ---
    let covit_out_path = bin_dir.join(COVIT_BIN);
    if !covit_out_path.exists() && !is_in_path(COVIT_BIN) {
        println!("Downloading covit...");
        download_with_progress(COVIT_URL, covit_out_path.to_str().unwrap())?;
        #[cfg(unix)]
        set_executable_permissions(&covit_out_path)?;
    }

    // --- Build DependencyPaths ---
    let covit = covit_out_path.to_string_lossy().to_string();
    #[cfg(feature = "depend-on-ffmpeg")]
    {
        Ok(DependencyPaths { ffmpeg: ffmpeg_path, ffprobe: ffprobe_path, covit })
    }
    #[cfg(not(feature = "depend-on-ffmpeg"))]
    {
        Ok(DependencyPaths { covit })
    }
}

/// Extracts selected files from a tar.xz archive and saves them to the specified output directory.
///
/// # Arguments
///
/// * `archive_path` - Path to the tar.xz archive file.
/// * `files_to_extract` - Slice of filenames to extract from within the archive.
/// * `output_dir` - Directory where the extracted files will be saved.
#[cfg(all(unix, feature = "depend-on-ffmpeg"))]
fn extract_selected_files(archive_path: &Path, files_to_extract: &[&str], output_dir: &Path) -> Result<(), ExtractError> {
    use std::fs::{self, File};
    let file = File::open(archive_path)?;
    let decompressor = xz2::read::XzDecoder::new(file);
    let mut archive = tar::Archive::new(decompressor);

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) &&
            files_to_extract.contains(&name)
        {
            let out_path = output_dir.join(name);
            let mut out_file = File::create(&out_path)?;
            io::copy(&mut entry, &mut out_file)?;
            set_executable_permissions(&out_path)?;
        }
    }
    if let Err(err) = fs::remove_file(archive_path) {
        eprintln!("Error deleting file: {err}");
    }
    Ok(())
}

/// Extracts selected files from a zip archive and saves them to the specified output directory.
///
/// # Arguments
///
/// * `archive_path` - Path to the zip archive file.
/// * `files_to_extract` - Slice of filenames to extract from within the archive.
/// * `output_dir` - Directory where the extracted files will be saved.
#[cfg(all(windows, feature = "depend-on-ffmpeg"))]
fn extract_selected_files(archive_path: &Path, files_to_extract: &[&str], output_dir: &Path) -> Result<(), ExtractError> {
    use std::fs::File;

    use zip::ZipArchive;

    let file = File::open(archive_path)?;
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name();
        if files_to_extract.iter().any(|wanted| name.ends_with(wanted)) {
            let out_path = output_dir.join(Path::new(name).file_name().unwrap_or_else(|| std::ffi::OsStr::new(name)));
            let mut out_file = File::create(&out_path)?;
            std::io::copy(&mut entry, &mut out_file)?;
        }
    }
    if let Err(err) = fs::remove_file(archive_path) {
        eprintln!("Error deleting file: {}", err);
    }
    Ok(())
}

/// Downloads a file from the specified URL and saves it to the given output path.
/// Shows progress during the download.
///
/// # Arguments
///
/// * `url` - URL of the file to download.
/// * `out_path` - Path where the downloaded file will be saved.
fn download_with_progress(url: &str, out_path: &str) -> Result<(), DownloadError> {
    let (headers, body) = get(url).call()?.into_parts();

    let total_size = headers.headers.get("Content-Length").and_then(|v| v.to_str().ok()).and_then(|s| s.parse::<u64>().ok()).ok_or(DownloadError::HeaderParse)?;

    let mut file = File::create(out_path)?;
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
