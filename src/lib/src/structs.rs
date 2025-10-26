use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use walkdir::WalkDir;

use crate::deps_download::DependencyPaths;

/// Supported audio/video file extensions.
#[derive(Debug, PartialEq, Eq)]
pub enum FileFormat {
    Mp3,
    M4a,
    Flac,
    Aac,
    Opus,
    Ogg,
    Wma,
    Wav,
    Aiff,
    Alac,
    Ape,
    Flv,
    Webm,
    Unknown,
}

impl FileFormat {
    /// Return the enum variant that matches the file’s extension (case‑insensitive).
    fn from_path<P: AsRef<Path>>(path: P) -> Self {
        match path
            .as_ref()
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .as_deref()
        {
            Some("mp3") => FileFormat::Mp3,
            Some("m4a") => FileFormat::M4a,
            Some("flac") => FileFormat::Flac,
            Some("aac") => FileFormat::Aac,
            Some("opus") => FileFormat::Opus,
            Some("ogg") => FileFormat::Ogg,
            Some("wma") => FileFormat::Wma,
            Some("wav") => FileFormat::Wav,
            Some("aiff") => FileFormat::Aiff,
            Some("alac") => FileFormat::Alac,
            Some("ape") => FileFormat::Ape,
            Some("flv") => FileFormat::Flv,
            Some("webm") => FileFormat::Webm,
            _ => FileFormat::Unknown,
        }
    }

    /// Helper to know whether the variant is a real format.
    fn is_known(&self) -> bool {
        *self != FileFormat::Unknown
    }
}

/// Holds the list of files (with detected format) for the supplied input.
pub struct RustyCov<'a> {
    /// `None` → no input processed yet; `Some(map)` → files grouped by parent directory.
    pub files: Option<HashMap<PathBuf, Vec<PathBuf>>>,
    pub deps: Option<DependencyPaths>,
    pub cov_address: Option<&'a str>,
}

impl<'a> Default for RustyCov<'a> {
    fn default() -> Self {
        Self { files: None, deps: None, cov_address: Some("https://covers.musichoarders.xyz") }
    }
}

impl<'a> RustyCov<'a> {
    /// Populate `files` from a path that may be a file or a directory.
    /// Only entries whose extension maps to a known `FileFormat` are kept.
    pub fn populate_from_input<S: Into<String>>(&mut self, input: S) {
        let path_str = input.into();
        let path = PathBuf::from(&path_str);

        let mut files_by_dir: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();

        if path.is_dir() {
            // Walk the directory recursively, keeping only known formats.
            for entry in WalkDir::new(&path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let p = entry.path().to_path_buf();
                let fmt = FileFormat::from_path(&p);
                if fmt.is_known() {
                    let parent = p.parent().unwrap_or(Path::new("")).to_path_buf();
                    files_by_dir.entry(parent).or_default().push(p);
                }
            }
        } else if path.is_file() {
            // Single file case – keep it only if it matches a known format.
            let fmt = FileFormat::from_path(&path);
            if fmt.is_known() {
                let parent = path.parent().unwrap_or(Path::new("")).to_path_buf();
                files_by_dir.entry(parent).or_default().push(path);
            }
        } else {
            eprintln!("❌ Path '{}' does not exist.", path_str);
            self.files = None;
            return;
        }

        // If we gathered at least one supported file, store it; otherwise keep None.
        if !files_by_dir.is_empty() {
            self.files = Some(files_by_dir);
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Picked {
    pub big_cover_url: String,
    pub release_info: ReleaseInfo,
    pub cover_info: CoverInfo,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseInfo {
    pub title: String,
    pub artist: String,
    pub date: String,
    pub tracks: Option<u32>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CoverInfo {
    pub format: String,
    pub height: u32,
    pub width: u32,
    pub size: u64,
}
